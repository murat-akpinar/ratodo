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
use ratodo::{agenda, capture, ics, theme, ui, write};

#[derive(Parser)]
#[command(name = "ratodo", version, about, long_about = None)]
struct Cli {
    /// Use a different list instead of ~/.config/ratodo/todo.md
    #[arg(long, short, global = true, value_name = "PATH")]
    file: Option<PathBuf>,

    /// Run once with a built-in theme, ignoring theme.conf
    #[arg(long, global = true, value_name = "NAME")]
    theme: Option<String>,

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
    /// Look at the colours
    Theme {
        #[command(subcommand)]
        what: ThemeCommand,
    },
    /// Print the counts on one line, for a status bar
    Status {
        /// waybar's format: {"text":…,"tooltip":…,"class":…}
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
enum ThemeCommand {
    /// List the built-in themes
    List,
    /// Print the active theme as a valid theme.conf
    Dump,
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
        Some(Command::Theme { what }) => theme_command(what, cli.theme.as_deref())?,
        Some(Command::Status { json }) => return status(&path, json),
        // `ratodo | wc -l` and `ratodo > out.txt` still have to mean something,
        // and a TUI down a pipe means nothing at all.
        None if std::io::stdout().is_terminal() => return tui(&path, cli.theme.as_deref()),
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

/// `$VISUAL` then `$EDITOR`, split on whitespace so that `EDITOR="nvim -u NONE"`
/// works. That does mean an editor whose *path* contains a space cannot be found
/// — the usual trade, and the one every other tool makes.
///
/// No fallback to `vi`: guessing at a program that may not be installed produces
/// a worse message than saying which variable to set.
fn editor() -> Option<Vec<String>> {
    let raw = ["VISUAL", "EDITOR"]
        .iter()
        .find_map(|name| std::env::var(name).ok().filter(|v| !v.trim().is_empty()))?;

    let words: Vec<String> = raw.split_whitespace().map(str::to_string).collect();
    (!words.is_empty()).then_some(words)
}

/// The first locale variable that is set, in the order the C library reads them.
fn locale() -> Option<String> {
    ["LC_ALL", "LC_CTYPE", "LANG"]
        .iter()
        .find_map(|name| std::env::var(name).ok().filter(|v| !v.is_empty()))
}

/// Reads `theme.conf`, applies `--theme` and `NO_COLOR` over it, and complains
/// on stderr about anything wrong.
///
/// **Nothing in here can stop the program.** A theme file that cannot be read at
/// all is the same as not having one — invariant 8, and the reason this returns
/// a `Theme` rather than a `Result`.
fn active_theme(flag: Option<&str>) -> theme::Theme {
    let config = dirs()
        .ok()
        .map(|d| d.config_dir().join("theme.conf"))
        .and_then(|p| std::fs::read_to_string(p).ok());

    // Any non-empty value counts, which is what the NO_COLOR convention says.
    let no_colour = std::env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty());
    let parsed = theme::resolve(config.as_deref(), flag, no_colour);

    for warning in &parsed.warnings {
        eprintln!("theme.conf: {warning}");
    }
    parsed.theme
}

fn theme_command(what: ThemeCommand, flag: Option<&str>) -> Result<()> {
    let mut out = std::io::stdout().lock();
    match what {
        ThemeCommand::List => {
            for (name, _) in theme::BUILT_IN {
                let note = if name == "catppuccin-mocha" {
                    "  (default)"
                } else {
                    ""
                };
                writeln!(out, "{name}{note}")?;
            }
        }
        // `ratodo theme dump > ~/.config/ratodo/theme.conf` is the documented
        // use, so this has to be the file, and nothing else on stdout.
        ThemeCommand::Dump => write!(out, "{}", active_theme(flag).dump())?,
    }
    Ok(())
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
///
/// The channel carries nothing but "it changed": the loop reads keys itself and
/// this is the only other thing that can happen.
fn watch(path: &Path, tx: std::sync::mpsc::Sender<()>) -> Option<notify::RecommendedWatcher> {
    use notify::Watcher;

    let dir = path.parent()?;
    let name = path.file_name()?.to_os_string();
    let mut watcher = notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
        if event.iter().any(|e| touches(e, &name)) {
            let _ = tx.send(());
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

fn tui(path: &Path, theme_flag: Option<&str>) -> Result<ExitCode> {
    let shown_path = path.display().to_string();
    let render = ui::Render {
        colours: active_theme(theme_flag),
        glyphs: ui::Glyphs::for_locale(locale().as_deref()),
        today: Local::now().date_naive(),
        path: &shown_path,
    };
    let mut live = Live::read(path, render.today)?;

    let (tx, rx) = std::sync::mpsc::channel();
    // Kept alive for as long as the TUI runs: dropping it stops the watch.
    let _watcher = watch(path, tx);

    // `try_init` installs a panic hook that turns raw mode off and leaves the
    // alternate screen before chaining to the default one, and `Terminal`'s own
    // Drop puts the cursor back. That is the whole of invariant 5, and it is the
    // library's, so it cannot drift out of step with the setup it undoes.
    let mut terminal = ratatui::try_init()?;
    let result = run(&mut terminal, &mut live, path, &rx, render);
    ratatui::restore();
    result
}

/// The open list: the document, what the screen is showing of it, and enough to
/// tell our own writes apart from somebody else's.
struct Live {
    doc: ratodo::model::Doc,
    mtime: Option<std::time::SystemTime>,
    /// The exact bytes we last put on disk. Every save wakes the watcher, and
    /// re-reading our own write would undo the in-place update that keeps a
    /// ticked task from jumping.
    written: Option<String>,
    /// The document as it was before the last change, and what to call it.
    ///
    /// One level, which is what docs/tui.md promises: "undoes the last change in
    /// this session". A whole `Doc` per change is a few kilobytes and buys an
    /// undo that cannot be subtly wrong about what it is putting back.
    undo: Option<(ratodo::model::Doc, String)>,
    screen: ui::Screen,
    counts: agenda::Counts,
}

impl Live {
    fn read(path: &Path, today: chrono::NaiveDate) -> Result<Self> {
        let loaded = write::load(path)?;
        let tasks: Vec<_> = loaded.doc.tasks().cloned().collect();
        Ok(Live {
            counts: agenda::Counts::of(&tasks, today),
            screen: ui::Screen::new(ui::rows(&agenda::agenda(&tasks, today))),
            doc: loaded.doc,
            mtime: loaded.mtime,
            written: None,
            undo: None,
        })
    }

    fn reload(&mut self, path: &Path, today: chrono::NaiveDate) -> Result<()> {
        let loaded = write::load(path)?;
        let tasks: Vec<_> = loaded.doc.tasks().cloned().collect();
        self.counts = agenda::Counts::of(&tasks, today);
        self.screen
            .replace(ui::rows(&agenda::agenda(&tasks, today)));
        self.doc = loaded.doc;
        self.mtime = loaded.mtime;
        self.written = None;
        Ok(())
    }

    /// Hands the terminal to `$EDITOR` and takes it back.
    ///
    /// The escape hatch: whatever the tool cannot do, the file can — and the
    /// file is a Markdown file the user already knows how to edit. See
    /// docs/product.md#product-decisions.
    ///
    /// This is what the event loop is shaped around. A thread parked in
    /// `crossterm::event::read` would be reading the same terminal as vim, and
    /// would eat its keystrokes; `poll` in one thread has nobody to compete
    /// with. See docs/decisions.md#reversed.
    fn edit(
        &mut self,
        terminal: &mut ratatui::DefaultTerminal,
        path: &Path,
        today: chrono::NaiveDate,
    ) -> Result<ui::Notice> {
        let Some(command) = editor() else {
            return Ok(ui::Notice::Warned("no $EDITOR or $VISUAL set".to_string()));
        };

        // The screen is handed back and taken again around the same `Terminal`,
        // rather than torn down and rebuilt with `try_init`. Rebuilding chains a
        // second panic hook onto the first every time, and asks the terminal for
        // its cursor position — a question some terminals never answer, which
        // turns `e` into a hang. Suspending is both smaller and safer.
        crossterm::terminal::disable_raw_mode()?;
        crossterm::execute!(std::io::stdout(), crossterm::terminal::LeaveAlternateScreen)?;

        let outcome = std::process::Command::new(&command[0])
            .args(&command[1..])
            .arg(path)
            .status();

        crossterm::execute!(
            std::io::stdout(),
            crossterm::terminal::EnterAlternateScreen,
            crossterm::terminal::Clear(crossterm::terminal::ClearType::All),
        )?;
        crossterm::terminal::enable_raw_mode()?;

        // The backend still believes the screen holds the last frame it drew, so
        // the next draw would send a diff against something that is no longer
        // there. `resize` to the size it already is throws that memory away —
        // and, unlike `Terminal::clear`, it does not ask the terminal where its
        // cursor is. That is a question some terminals never answer, and waiting
        // for the reply is `e` hanging.
        let size = terminal.size()?;
        terminal.resize(ratatui::layout::Rect::new(0, 0, size.width, size.height))?;

        // Read it back whatever the editor said: somebody who saves and then
        // exits non-zero still saved.
        self.reload(path, today)?;
        quietly_sync(path);

        Ok(match outcome {
            Ok(_) => ui::Notice::Said("re-read after $EDITOR".to_string()),
            Err(e) => ui::Notice::Warned(format!("{}: {e}", command[0])),
        })
    }

    /// Rebuilds the rows and the counts from the document already in memory.
    /// `reload` is this plus a read; a change made here does not need the read.
    fn rebuild(&mut self, today: chrono::NaiveDate) {
        let tasks: Vec<_> = self.doc.tasks().cloned().collect();
        self.counts = agenda::Counts::of(&tasks, today);
        self.screen
            .replace(ui::rows(&agenda::agenda(&tasks, today)));
    }

    /// Writes what is in memory, and on a refusal puts the document back.
    ///
    /// `Some(notice)` means the write was refused and the caller should show it
    /// and change nothing else. `None` means it went through. Every change goes
    /// through here so that "warn, do not merge" is written once — three copies
    /// of it would be three chances to get the recovery subtly wrong.
    ///
    /// `undo` is what to call this change if it is later taken back; `None`
    /// clears the slot, which is what an undo itself does.
    fn write_back(
        &mut self,
        path: &Path,
        before: ratodo::model::Doc,
        undo: Option<String>,
    ) -> Result<Option<ui::Notice>> {
        match write::save(path, &self.doc, self.mtime, &backup_dir()?) {
            Ok(()) => {
                self.written = Some(write::render(&self.doc));
                self.mtime = std::fs::metadata(path).and_then(|m| m.modified()).ok();
                self.undo = undo.map(|what| (before, what));
                quietly_sync(path);
                Ok(None)
            }
            // The file still says otherwise, and a screen that disagrees with
            // disk is how the *next* write loses data.
            Err(e) if e.downcast_ref::<write::Conflict>().is_some() => {
                self.doc = before;
                Ok(Some(ui::Notice::Warned(
                    "changed on disk — nothing was written.  r reload".to_string(),
                )))
            }
            Err(e) => {
                self.doc = before;
                Err(e)
            }
        }
    }

    /// The raw line of the selected task, which is how a row on screen is found
    /// again in the document.
    fn selected_raw(&self) -> Option<String> {
        self.screen.task().map(|t| t.raw.clone())
    }

    /// Ticks the selected task, in the document and on the screen, and writes.
    ///
    /// The row is updated in place rather than regrouped: a task marked done
    /// stays where it is until the next reload. Nothing else on screen moves.
    fn toggle(&mut self, path: &Path, today: chrono::NaiveDate) -> Result<ui::Notice> {
        let Some(raw) = self.selected_raw() else {
            return Ok(ui::Notice::Said("nothing to tick here".to_string()));
        };
        let before = self.doc.clone();
        let Some(task) = self.doc.tasks_mut().find(|t| t.raw == raw) else {
            return Ok(ui::Notice::Said("nothing to tick here".to_string()));
        };

        let done = !task.done;
        task.set_done(done);
        let updated = task.clone();
        let title = text::plain(&updated.title);

        // Two strings, because they end up in different sentences: one is the
        // result line, the other is what `undo` reports putting back.
        let undo_label = format!("{} {title}", if done { "ticking" } else { "unticking" });
        let said = format!("{}: {title}", if done { "done" } else { "reopened" });

        if let Some(refused) = self.write_back(path, before, Some(undo_label))? {
            return Ok(refused);
        }

        self.screen.update_selected(updated);
        let tasks: Vec<_> = self.doc.tasks().cloned().collect();
        self.counts = agenda::Counts::of(&tasks, today);
        Ok(ui::Notice::Said(format!("{said}    u undo")))
    }

    /// Deletes the selected task immediately, with no confirmation.
    ///
    /// A prompt taxes every delete to protect against the rare wrong one; `u`
    /// inverts that, so deleting costs one key and the mistake costs one more.
    /// See docs/tui.md#deleting--no-confirmation-dialog.
    fn delete(&mut self, path: &Path, today: chrono::NaiveDate) -> Result<ui::Notice> {
        let Some(raw) = self.selected_raw() else {
            return Ok(ui::Notice::Said("nothing to delete here".to_string()));
        };
        let Some(at) = self
            .doc
            .lines
            .iter()
            .position(|l| matches!(&l.item, ratodo::model::Item::Task(t) if t.raw == raw))
        else {
            return Ok(ui::Notice::Said("nothing to delete here".to_string()));
        };

        let before = self.doc.clone();
        let Some(gone) = self.doc.remove_task(at) else {
            return Ok(ui::Notice::Said("nothing to delete here".to_string()));
        };
        let title = text::plain(&gone.title);

        if let Some(refused) =
            self.write_back(path, before, Some(format!("deleting \"{title}\"")))?
        {
            return Ok(refused);
        }

        self.rebuild(today);
        Ok(ui::Notice::Said(format!("deleted \"{title}\"    u undo")))
    }

    /// Puts the document back the way it was before the last change.
    ///
    /// One level, and the whole document rather than an inverse operation: an
    /// undo that reconstructs what a change did is an undo that can be subtly
    /// wrong about it, and this one cannot.
    fn undo(&mut self, path: &Path, today: chrono::NaiveDate) -> Result<ui::Notice> {
        // Cloned rather than taken: if the write is refused the slot has to
        // still be there, or a refusal that changed nothing has cost the user
        // their undo.
        let Some((restored, what)) = self.undo.clone() else {
            return Ok(ui::Notice::Said("nothing to undo".to_string()));
        };

        let current = std::mem::replace(&mut self.doc, restored);
        if let Some(refused) = self.write_back(path, current, None)? {
            return Ok(refused);
        }

        self.rebuild(today);
        Ok(ui::Notice::Said(format!("undone: {what}")))
    }
}

/// How long the loop is willing to sit in `poll` before looking at the channel.
///
/// It bounds one thing only: how stale an outside edit can be before the screen
/// catches up. A keystroke returns from `poll` immediately whatever this is, so
/// nothing a user does waits on it. Half a second of latency on a `git pull` is
/// invisible; two wake-ups a second is the price, and it is what buys `e`.
/// See docs/architecture.md#the-event-loop.
const IDLE: std::time::Duration = std::time::Duration::from_millis(500);

fn run(
    terminal: &mut ratatui::DefaultTerminal,
    live: &mut Live,
    path: &Path,
    rx: &std::sync::mpsc::Receiver<()>,
    render: ui::Render<'_>,
) -> Result<ExitCode> {
    let today = render.today;
    let mut notice = ui::Notice::Hints;
    let mut helping = false;
    // Drawing is still driven by events, not by the timer: a wake-up that finds
    // nothing to do draws nothing.
    let mut redraw = true;

    loop {
        if redraw {
            terminal.draw(|frame| {
                ui::draw(
                    frame,
                    &mut live.screen,
                    live.counts,
                    render,
                    &notice,
                    helping,
                )
            })?;
            redraw = false;
        }

        if event::poll(IDLE)? {
            // A resize or a paste falls through to the redraw and nothing else.
            redraw = true;
            if let Event::Key(key) = event::read()? {
                // What the key means lives in `ui`, where it can be tested; this
                // loop only knows how to read one and how to obey.
                match ui::action(key) {
                    ui::Action::Quit => return Ok(ExitCode::SUCCESS),
                    ui::Action::Move(n) => {
                        live.screen.move_by(n);
                        notice = ui::Notice::Hints;
                    }
                    ui::Action::Top => live.screen.top(),
                    ui::Action::Bottom => live.screen.bottom(),
                    ui::Action::Toggle => notice = live.toggle(path, today)?,
                    ui::Action::Delete => notice = live.delete(path, today)?,
                    ui::Action::Undo => notice = live.undo(path, today)?,
                    ui::Action::Fold(want) => {
                        if let Some(complaint) = live.screen.fold(want) {
                            notice = ui::Notice::Said(complaint.to_string());
                        }
                    }
                    ui::Action::Edit => {
                        notice = live.edit(terminal, path, today)?;
                    }
                    ui::Action::Reload => {
                        live.reload(path, today)?;
                        notice = ui::Notice::Said("reloaded".to_string());
                    }
                    ui::Action::Help => helping = !helping,
                    // `esc` puts the overlay down and otherwise does nothing at
                    // all. It must never quit.
                    ui::Action::Close => helping = false,
                    ui::Action::Say(what) => notice = ui::Notice::Said(what.to_string()),
                    ui::Action::Ignore => {}
                }
            }
        }

        // One save arrives as several events; take the lot in one go. Drained
        // with a loop rather than `count() > 0`, which a mutation turns into
        // `>= 0` — always true, so the file gets read twice a second forever
        // and no test can see the difference.
        let mut changed = false;
        for () in rx.try_iter() {
            changed = true;
        }

        if changed {
            // Our own save wakes the watcher too. Re-reading it would throw away
            // the in-place update and let the ticked task jump.
            let on_disk = std::fs::read_to_string(path).ok();
            if on_disk.is_none() || on_disk != live.written {
                live.reload(path, today)?;
                redraw = true;
            }
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

    /// `$VISUAL` wins, `$EDITOR` follows, and neither set is `None` rather than
    /// a guess at `vi` that may not be installed.
    ///
    /// Reads the real environment, so it sets and restores what it looks at.
    /// `#[serial]` would be the tidy answer and would be an eighth dependency.
    #[test]
    fn which_editor_and_how_it_is_split() {
        let restore: Vec<(&str, Option<String>)> = ["VISUAL", "EDITOR"]
            .iter()
            .map(|name| (*name, std::env::var(name).ok()))
            .collect();

        // SAFETY: single-threaded test, and every variable it touches is put
        // back before it returns.
        unsafe {
            std::env::remove_var("VISUAL");
            std::env::remove_var("EDITOR");
            assert_eq!(editor(), None, "nothing set is not a guess at vi");

            std::env::set_var("EDITOR", "nvim");
            assert_eq!(editor(), Some(vec!["nvim".to_string()]));

            std::env::set_var("VISUAL", "code -w");
            assert_eq!(
                editor(),
                Some(vec!["code".to_string(), "-w".to_string()]),
                "VISUAL outranks EDITOR, and its arguments come with it"
            );

            std::env::set_var("VISUAL", "   ");
            assert_eq!(
                editor(),
                Some(vec!["nvim".to_string()]),
                "a blank VISUAL is not a choice"
            );

            for (name, was) in restore {
                match was {
                    Some(value) => std::env::set_var(name, value),
                    None => std::env::remove_var(name),
                }
            }
        }
    }

    /// A scratch list on disk, and a `Live` open on it.
    fn open(tag: &str, text: &str) -> (PathBuf, Live) {
        let dir = std::env::temp_dir().join(format!("ratodo-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("todo.md");
        std::fs::write(&path, text).unwrap();

        let live = Live::read(&path, a_day()).unwrap();
        (path, live)
    }

    fn a_day() -> chrono::NaiveDate {
        chrono::NaiveDate::from_ymd_opt(2026, 8, 11).unwrap()
    }

    /// Deleting takes one line out and leaves everything else exactly as it was,
    /// including the things ratodo does not understand.
    #[test]
    fn delete_removes_the_selected_line_and_nothing_else() {
        let before = "# My list\n\n## Work\n- [ ] first\n- [ ] second\n\n> a note\n";
        let (path, mut live) = open("delete", before);

        let notice = live.delete(&path, a_day()).unwrap();
        assert!(
            matches!(&notice, ui::Notice::Said(s) if s.contains("deleted \"first\"") && s.contains("u undo")),
            "{notice:?}"
        );
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "# My list\n\n## Work\n- [ ] second\n\n> a note\n"
        );
        // And the screen agrees. Writing the file without rebuilding the rows
        // leaves a task on screen that is not in the document any more, and the
        // next thing done to it writes against a line that has gone.
        assert_eq!(
            live.screen.task().map(|t| t.title.clone()),
            Some("second".into())
        );
        assert_eq!(live.counts.open, 1);

        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    /// Only a refused write becomes a warning. Everything else is a real
    /// failure, and swallowing it as "changed on disk" would send the user to
    /// press `r` at a problem that reloading cannot fix.
    #[test]
    fn a_write_that_fails_for_any_other_reason_is_not_reported_as_a_conflict() {
        use std::os::unix::fs::PermissionsExt;

        let (path, mut live) = open("readonly", "- [ ] first\n");
        let dir = path.parent().unwrap().to_path_buf();

        // The temp file the atomic write needs cannot be created in here.
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o555)).unwrap();
        let outcome = live.toggle(&path, a_day());
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).unwrap();

        let error = outcome.expect_err("a permission failure came back as a notice");
        assert!(
            error.downcast_ref::<write::Conflict>().is_none(),
            "a permission failure was dressed up as a conflict: {error:#}"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The trade docs/tui.md makes: no confirmation prompt, because `u` is
    /// cheaper than taxing every delete to catch the rare wrong one.
    #[test]
    fn undo_puts_a_deleted_task_back_where_it_was() {
        let before = "## Work\n- [ ] first\n- [ ] second\n";
        let (path, mut live) = open("undo", before);

        live.delete(&path, a_day()).unwrap();
        let notice = live.undo(&path, a_day()).unwrap();

        assert!(
            matches!(&notice, ui::Notice::Said(s) if s.contains("undone")),
            "{notice:?}"
        );
        assert_eq!(std::fs::read_to_string(&path).unwrap(), before);

        // One level, and it is spent.
        assert!(
            matches!(live.undo(&path, a_day()).unwrap(), ui::Notice::Said(s) if s.contains("nothing to undo"))
        );

        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn undo_takes_back_a_toggle_too() {
        let before = "- [ ] first\n";
        let (path, mut live) = open("undo-toggle", before);

        live.toggle(&path, a_day()).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "- [x] first\n");

        live.undo(&path, a_day()).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), before);

        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    /// A refusal changes nothing — which has to include the undo slot. Losing it
    /// to a write that did not happen would be the worst of both.
    #[test]
    fn a_refused_undo_keeps_the_undo() {
        let (path, mut live) = open("undo-conflict", "- [ ] first\n- [ ] second\n");
        live.delete(&path, a_day()).unwrap();

        // Somebody else gets there first.
        std::thread::sleep(std::time::Duration::from_millis(20));
        std::fs::write(&path, "- [ ] theirs\n").unwrap();

        let refused = live.undo(&path, a_day()).unwrap();
        assert!(matches!(&refused, ui::Notice::Warned(w) if w.contains("changed on disk")));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "- [ ] theirs\n");
        assert!(
            live.undo.is_some(),
            "the undo was spent on a write that never happened"
        );

        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn there_is_nothing_to_delete_or_undo_in_an_empty_list() {
        let (path, mut live) = open("empty-actions", "> just a note\n");

        assert!(
            matches!(live.delete(&path, a_day()).unwrap(), ui::Notice::Said(s) if s.contains("nothing to delete"))
        );
        assert!(
            matches!(live.undo(&path, a_day()).unwrap(), ui::Notice::Said(s) if s.contains("nothing to undo"))
        );
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "> just a note\n");

        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    /// The one case where the tool must interrupt, because the alternative is
    /// losing somebody's work. Deterministic here in a way it cannot be through
    /// a terminal: the file is changed behind `Live`'s back, with no reload in
    /// between, which is exactly the state the mtime check exists to catch.
    #[test]
    fn a_write_over_somebody_elses_edit_is_refused_and_says_so() {
        let dir = std::env::temp_dir().join(format!("ratodo-conflict-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("todo.md");

        let today = chrono::NaiveDate::from_ymd_opt(2026, 8, 11).unwrap();
        std::fs::write(&path, "- [ ] mine\n").unwrap();
        let mut live = Live::read(&path, today).unwrap();

        // Somebody else gets there first. The sleep is for filesystems whose
        // mtime resolution is coarser than two writes in a row.
        std::thread::sleep(std::time::Duration::from_millis(20));
        std::fs::write(&path, "- [ ] mine\n- [ ] theirs\n").unwrap();

        let notice = live.toggle(&path, today).unwrap();
        assert!(
            matches!(&notice, ui::Notice::Warned(w) if w.contains("changed on disk")),
            "{notice:?}"
        );
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "- [ ] mine\n- [ ] theirs\n",
            "the refused write went through anyway"
        );
        // And the in-memory document went back, so the screen does not sit there
        // claiming a task is done that the file says is not.
        assert!(
            live.doc.tasks().all(|t| !t.done),
            "the model kept a change the file refused"
        );

        let _ = std::fs::remove_dir_all(&dir);
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
