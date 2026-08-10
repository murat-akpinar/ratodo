//! Subcommands and terminal setup. See docs/cli.md.

use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result};
use chrono::{Local, Utc};
use clap::{Parser, Subcommand};
use crossterm::event::{self, Event};

use ratodo::model::{Lookup, Priority};
use ratodo::text;
use ratodo::{agenda, capture, ics, ui, write};

#[derive(Parser)]
#[command(name = "ratodo", version, about, long_about = None)]
struct Cli {
    /// Use a different list instead of ~/.config/ratodo/todo.md
    #[arg(long, short, global = true, value_name = "PATH")]
    file: Option<PathBuf>,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Capture a task and exit
    Add {
        /// Free text: "pay the invoice @tomorrow #home !high"
        #[arg(required = true, trailing_var_arg = true)]
        text: Vec<String>,
    },
    /// Mark the one task matching this text as done
    Done {
        /// Case-insensitive substring of the task's title. It has to match one
        #[arg(required = true, trailing_var_arg = true)]
        text: Vec<String>,
    },
    /// Print the list
    List(ListArgs),
    /// Regenerate todo.ics by hand. Every write does it anyway
    Sync,
    /// Print the counts on one line, for a status bar
    Status {
        /// waybar's format: {"text":…,"tooltip":…,"class":…}
        #[arg(long)]
        json: bool,
    },
}

#[derive(clap::Args, Default)]
struct ListArgs {
    /// Only tasks carrying this tag. Repeatable, and repeats mean or
    #[arg(long, value_name = "NAME")]
    tag: Vec<String>,

    /// Only tasks at this priority: high, med or low
    #[arg(long, value_name = "LEVEL", value_parser = priority)]
    prio: Option<Priority>,

    /// Tab-separated output for scripts: no headings, no summary, no colour
    #[arg(long)]
    porcelain: bool,
}

fn priority(name: &str) -> Result<Priority, String> {
    Priority::from_name(name).ok_or_else(|| "expected high, med or low".to_string())
}

fn main() -> Result<ExitCode> {
    match dispatch() {
        // `ratodo list | head -3` closes the pipe under us, and Rust turns the
        // write that follows into a panic. Piping into `head` is an ordinary
        // thing to type; a backtrace is not an ordinary thing to get back.
        Err(e) if is_broken_pipe(&e) => Ok(ExitCode::SUCCESS),
        other => other,
    }
}

fn is_broken_pipe(error: &anyhow::Error) -> bool {
    error
        .chain()
        .filter_map(|e| e.downcast_ref::<std::io::Error>())
        .any(|e| e.kind() == std::io::ErrorKind::BrokenPipe)
}

fn dispatch() -> Result<ExitCode> {
    let cli = Cli::parse();
    let path = match cli.file {
        Some(p) => p,
        None => env_path().map_or_else(default_path, Ok)?,
    };

    match cli.command {
        Some(Command::Add { text }) => add(&path, &text.join(" "))?,
        Some(Command::Done { text }) => return done(&path, &text.join(" ")),
        Some(Command::List(args)) => list(&path, &args)?,
        Some(Command::Sync) => sync(&path, true)?,
        Some(Command::Status { json }) => return status(&path, json),
        // `ratodo | wc -l` and `ratodo > out.txt` still have to mean something,
        // and a TUI down a pipe means nothing at all.
        None if std::io::stdout().is_terminal() => return tui(&path),
        None => list(&path, &ListArgs::default())?,
    }
    Ok(ExitCode::SUCCESS)
}

fn dirs() -> Result<directories::ProjectDirs> {
    directories::ProjectDirs::from("", "", "ratodo")
        .context("could not work out where ~/.config is")
}

/// Below `--file`, above the default: `direnv` can then give a repository its
/// own list without an alias per checkout.
fn env_path() -> Option<PathBuf> {
    let raw = std::env::var_os("RATODO_FILE")?;
    (!raw.is_empty()).then(|| PathBuf::from(raw))
}

fn default_path() -> Result<PathBuf> {
    Ok(dirs()?.config_dir().join("todo.md"))
}

/// Derived, so it never lands in the user's dotfiles. `state_dir` is `None` off
/// Linux, where the cache directory is the closest equivalent.
fn backup_dir() -> Result<PathBuf> {
    let dirs = dirs()?;
    Ok(dirs
        .state_dir()
        .unwrap_or_else(|| dirs.cache_dir())
        .to_path_buf())
}

/// Derived, regenerated, pointless to back up — so `$XDG_DATA_HOME`, unlike the
/// list itself. See docs/format.md.
fn ics_path() -> Result<PathBuf> {
    Ok(dirs()?.data_dir().join("todo.ics"))
}

/// Rewrites `todo.ics` from the list. `loud` is for `ratodo sync`, which was
/// asked to do this; after a capture it happens quietly.
///
/// A failure here never fails the command that triggered it. The `.ics` is a
/// convenience and the task is already safely in the file — refusing to capture
/// because a calendar export went wrong would be the tail wagging the dog.
fn sync(path: &Path, loud: bool) -> Result<()> {
    let doc = write::load(path)?.doc;
    let tasks: Vec<_> = doc.tasks().cloned().collect();
    let out = ics_path()?;

    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&out, ics::calendar(&tasks, Utc::now()))?;

    if loud {
        let exported = tasks.iter().filter(|t| !t.done && t.due.is_some()).count();
        writeln!(
            std::io::stdout(),
            "wrote {} dated task{} to {}",
            exported,
            if exported == 1 { "" } else { "s" },
            out.display()
        )?;
    }
    Ok(())
}

fn add(path: &Path, input: &str) -> Result<()> {
    let today = Local::now().date_naive();
    let task = capture::capture(input, today);
    let summary = text::added(&task, today);

    let loaded = write::load(path)?;
    let mut doc = loaded.doc;
    doc.push_task(task);
    write::save(path, &doc, loaded.mtime, &backup_dir()?)?;

    writeln!(std::io::stdout(), "{summary}")?;
    quietly_sync(path);
    Ok(())
}

/// The capture already succeeded and said so. Whatever the calendar export
/// makes of it, the user's list is written and this is not the moment to make
/// noise about a derived file.
fn quietly_sync(path: &Path) {
    let _ = sync(path, false);
}

/// Everything the screen needs, read fresh. Called again on every outside
/// change, which is why it takes a path rather than a document.
fn snapshot(path: &Path, today: chrono::NaiveDate) -> Result<(Vec<ui::Row>, agenda::Counts)> {
    let doc = write::load(path)?.doc;
    let tasks: Vec<_> = doc.tasks().cloned().collect();
    let counts = agenda::Counts::of(&tasks, today);
    Ok((ui::rows(&agenda::agenda(&tasks, today)), counts))
}

enum Msg {
    Input(Event),
    /// The list changed underneath us — vim, `git pull`, `ratodo add` next door.
    Reload,
    /// stdin ended. Without this the loop would wait for a key nobody can send.
    InputGone,
}

/// Whether a directory event is about our list. Watching the directory means
/// hearing about everything in it — `theme.conf` being saved, the temp file our
/// own writer makes on the way to a rename — and re-reading on all of that would
/// be a redraw every time anything in `~/.config/ratodo` moved.
fn touches(event: &notify::Event, name: &std::ffi::OsStr) -> bool {
    event.paths.iter().any(|p| p.file_name() == Some(name))
}

/// Watches the **directory**, not the file. Every safe writer — ours included —
/// replaces a file by writing a temp one and renaming it over the top, and an
/// inotify watch on the old inode goes quiet at exactly that moment.
///
/// The watcher has to stay alive to keep watching, so it is returned.
fn watch(path: &Path, tx: std::sync::mpsc::Sender<Msg>) -> Option<notify::RecommendedWatcher> {
    use notify::Watcher;

    let dir = path.parent()?;
    let name = path.file_name()?.to_os_string();
    let mut watcher = notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
        if event.iter().any(|e| touches(e, &name)) {
            let _ = tx.send(Msg::Reload);
        }
    })
    .ok()?;

    // A directory that is not there yet cannot be watched, and that is not worth
    // refusing to open the list over.
    watcher
        .watch(dir, notify::RecursiveMode::NonRecursive)
        .ok()?;
    Some(watcher)
}

fn tui(path: &Path) -> Result<ExitCode> {
    let today = Local::now().date_naive();
    let (rows, counts) = snapshot(path, today)?;
    let mut screen = ui::Screen::new(rows);

    let (tx, rx) = std::sync::mpsc::channel();
    let _watcher = watch(path, tx.clone());

    // A thread parked in `read` and a `recv` parked here: both sources wake the
    // loop the instant they have something, and neither one polls. That is the
    // whole of "no fixed FPS" — see docs/architecture.md#the-event-loop.
    std::thread::spawn(move || {
        loop {
            match event::read() {
                Ok(event) => {
                    if tx.send(Msg::Input(event)).is_err() {
                        return;
                    }
                }
                Err(_) => {
                    let _ = tx.send(Msg::InputGone);
                    return;
                }
            }
        }
    });

    // `try_init` installs a panic hook that turns raw mode off and leaves the
    // alternate screen before chaining to the default one, and `Terminal`'s own
    // Drop puts the cursor back. That is the whole of invariant 5, and it is the
    // library's, so it cannot drift out of step with the setup it undoes.
    let mut terminal = ratatui::try_init()?;
    let result = run(&mut terminal, &mut screen, counts, today, path, &rx);
    ratatui::restore();
    result
}

fn run(
    terminal: &mut ratatui::DefaultTerminal,
    screen: &mut ui::Screen,
    mut counts: agenda::Counts,
    today: chrono::NaiveDate,
    path: &Path,
    rx: &std::sync::mpsc::Receiver<Msg>,
) -> Result<ExitCode> {
    loop {
        terminal.draw(|frame| ui::draw(frame, screen, counts, today))?;

        match rx.recv().context("both event sources went away")? {
            Msg::InputGone => return Ok(ExitCode::SUCCESS),
            Msg::Reload => {
                // One save can arrive as several events. Re-reading a small file
                // twice costs nothing and the second draw emits no cells, so
                // there is nothing here worth a debounce that could swallow a key.
                let (rows, fresh) = snapshot(path, today)?;
                screen.replace(rows);
                counts = fresh;
            }
            // Anything else — a resize, say — falls through to the redraw above.
            Msg::Input(Event::Key(key)) => {
                // What the key means lives in `ui`, where it can be tested; this
                // loop only knows how to read one and how to obey.
                match ui::action(key) {
                    ui::Action::Quit => return Ok(ExitCode::SUCCESS),
                    ui::Action::Move(n) => screen.move_by(n),
                    ui::Action::Top => screen.top(),
                    ui::Action::Bottom => screen.bottom(),
                    ui::Action::Ignore => {}
                }
            }
            Msg::Input(_) => {}
        }
    }
}

/// Exit 2 — "asked, could not answer" — for both no match and too many, and in
/// neither case is the file opened for writing.
fn done(path: &Path, input: &str) -> Result<ExitCode> {
    let loaded = write::load(path)?;
    let mut doc = loaded.doc;

    let at = match doc.find_open(input) {
        Lookup::One(at) => at,
        Lookup::AlreadyDone(title) => {
            eprintln!("already done: {}", text::plain(&title));
            return Ok(ExitCode::SUCCESS);
        }
        Lookup::Several(candidates) => {
            eprintln!("{}", text::ambiguous(input, &candidates));
            return Ok(ExitCode::from(2));
        }
        Lookup::None => {
            eprintln!("no open task matches '{}'", text::plain(input));
            return Ok(ExitCode::from(2));
        }
    };

    let task = doc
        .task_at_mut(at)
        .context("the matched line stopped being a task")?;
    task.set_done(true);
    let summary = text::marked_done(task);

    write::save(path, &doc, loaded.mtime, &backup_dir()?)?;
    writeln!(std::io::stdout(), "{summary}")?;
    quietly_sync(path);
    Ok(ExitCode::SUCCESS)
}

fn list(path: &Path, args: &ListArgs) -> Result<()> {
    let doc = write::load(path)?.doc;
    let today = Local::now().date_naive();
    let filter = agenda::Filter {
        tags: &args.tag,
        prio: args.prio,
    };

    // `agenda` wants a slice and the tasks live scattered through `doc.lines`,
    // so they are copied out. A todo list is small; this is not worth a lifetime.
    let tasks: Vec<_> = doc.tasks().filter(|t| filter.matches(t)).cloned().collect();
    let groups = agenda::agenda(&tasks, today);

    // Locked once. Every write can fail — the reader may be a `head` that has
    // already seen enough — and `main` turns that into a quiet exit.
    let mut out = std::io::stdout().lock();

    if args.porcelain {
        // Nothing on stderr either: a machine is reading, and an empty result is
        // already the answer.
        for task in groups.iter().flat_map(|g| &g.tasks) {
            writeln!(out, "{}", text::porcelain_line(task))?;
        }
        return Ok(());
    }

    // stderr, not stdout: `ratodo list | wc -l` has to count tasks and nothing else.
    if tasks.is_empty() {
        if doc.task_count() == 0 {
            eprintln!("nothing here yet — try: ratodo add 'buy milk @tomorrow #home'");
            eprintln!("file: {}", path.display());
        } else {
            eprintln!("no task matches that filter");
        }
        return Ok(());
    }

    for group in &groups {
        if let Some(title) = group.kind.title() {
            writeln!(out, "\n{}", text::plain(title))?;
        }
        for task in &group.tasks {
            writeln!(out, "{}", text::list_line(task, today))?;
        }
    }

    // Counted over what was shown, not over the file: a summary that disagrees
    // with the list above it is worse than no summary.
    writeln!(
        out,
        "\n{}",
        text::status_line(agenda::Counts::of(&tasks, today))
    )?;

    Ok(())
}

/// Exits non-zero when something is overdue, which is the whole reason
/// `ratodo status || notify-send "$(ratodo status)"` needs no extra flag.
fn status(path: &Path, json: bool) -> Result<ExitCode> {
    let doc = write::load(path)?.doc;
    let today = Local::now().date_naive();
    let tasks: Vec<_> = doc.tasks().cloned().collect();
    let counts = agenda::Counts::of(&tasks, today);

    writeln!(
        std::io::stdout(),
        "{}",
        if json {
            text::status_json(counts)
        } else {
            text::status_line(counts)
        }
    )?;

    Ok(if counts.overdue > 0 {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsStr;

    fn event(path: &str) -> notify::Event {
        notify::Event::new(notify::EventKind::Any).add_path(PathBuf::from(path))
    }

    /// Both halves. Reacting to the wrong file is a redraw whenever anything in
    /// `~/.config/ratodo` moves; missing our own is the whole feature not working.
    #[test]
    fn only_our_own_list_counts_as_a_change() {
        let name = OsStr::new("todo.md");

        assert!(touches(&event("/home/you/.config/ratodo/todo.md"), name));
        assert!(!touches(
            &event("/home/you/.config/ratodo/theme.conf"),
            name
        ));
        assert!(
            !touches(&event("/home/you/.config/ratodo/todo.md.tmp-1234"), name),
            "our own writer's temp file must not look like the list"
        );
        assert!(!touches(&notify::Event::new(notify::EventKind::Any), name));
    }

    /// A rename reports both ends. Reading only the first would miss every
    /// `mv new todo.md`, which is how vim and our own writer save.
    #[test]
    fn a_rename_is_noticed_from_whichever_end_names_our_list() {
        let name = OsStr::new("todo.md");
        let rename = notify::Event::new(notify::EventKind::Any)
            .add_path(PathBuf::from("/home/you/.config/ratodo/todo.md.new"))
            .add_path(PathBuf::from("/home/you/.config/ratodo/todo.md"));
        assert!(touches(&rename, name));
    }
}
