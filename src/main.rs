//! Subcommands and terminal setup. See docs/cli.md.

use std::ffi::OsStr;
use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result};
use chrono::{Local, Utc};
use clap::{Parser, Subcommand};
use crossterm::event::{self, Event};

use ratodo::model::{Lookup, Priority, State};
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
    let paths = lists(cli.file)?;
    let derived = Derived::real()?;

    match cli.command {
        Some(Command::Add { text }) => add(&paths, &derived, &text.join(" "))?,
        Some(Command::Done { text }) => {
            return done(&paths, &derived, &text.join(" "), Local::now().date_naive());
        }
        Some(Command::List(args)) => list(&paths, &args)?,
        Some(Command::Sync) => sync(&paths, &derived.ics, true)?,
        Some(Command::Theme { what }) => theme_command(what, cli.theme.as_deref())?,
        Some(Command::Status { json }) => return status(&paths, json),
        // `ratodo | wc -l` and `ratodo > out.txt` still have to mean something,
        // and a TUI down a pipe means nothing at all.
        None if std::io::stdout().is_terminal() => {
            return tui(&paths, derived, cli.theme.as_deref());
        }
        None => list(&paths, &ListArgs::default())?,
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

/// Which lists are open, in the order they are read.
///
/// `--file` and `$RATODO_FILE` each name exactly one, and that is what keeps
/// every existing setup and every script working. With neither of them, **every
/// `*.md` in the config directory is a list** — `work.md`, `2026.md`,
/// `personal.md` are one agenda, and a new list is a new file and no
/// configuration at all. See docs/cli.md#several-lists.
///
/// Never empty: with nothing there yet it is the default path, which is also the
/// file the empty screen names.
fn lists(flag: Option<PathBuf>) -> Result<Vec<PathBuf>> {
    if let Some(path) = flag.or_else(env_path) {
        return Ok(vec![path]);
    }

    let dir = dirs()?.config_dir().to_path_buf();
    let mut found: Vec<PathBuf> = std::fs::read_dir(&dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension() == Some(OsStr::new("md")) && path.is_file())
        .collect();
    // A directory hands its entries over in whatever order it likes, and a list
    // that reorders itself between two runs is not a list.
    found.sort();

    if found.is_empty() {
        found.push(default_path()?);
    }
    Ok(found)
}

/// Where a captured task lands: `todo.md` when it is one of the lists, the first
/// of them otherwise. A fixed answer on purpose — `a` must not mean a different
/// file depending on what the cursor happens to be over. `--file` picks another.
///
/// `lists` never returns an empty vector, which is what makes the fallback safe.
fn capture_target(paths: &[PathBuf]) -> &Path {
    paths
        .iter()
        .find(|path| path.file_name() == Some(OsStr::new("todo.md")))
        .unwrap_or(&paths[0])
}

/// Which open list a `$work` is addressing, if any of them is.
///
/// `None` is refused by both callers rather than being turned into a new file:
/// a capture that invents a list in the config directory is a surprise, and a
/// new list is `touch` — docs/decisions.md#a-capture-always-goes-to-todomd--work-picks-the-list-2026-08-11.
fn addressed_list<'a>(name: &str, paths: &'a [PathBuf]) -> Option<&'a Path> {
    paths.iter().map(PathBuf::as_path).find(|path| {
        path.file_name()
            .is_some_and(|file| capture::names_list(name, &file.to_string_lossy()))
    })
}

/// The file this capture is for: the one it named, or the default target.
fn capture_into<'a>(text: &str, paths: &'a [PathBuf]) -> Result<&'a Path, String> {
    match capture::list_of(text) {
        Some(name) => addressed_list(name, paths).ok_or_else(|| format!("no list {name}.md")),
        None => Ok(capture_target(paths)),
    }
}

/// The open lists by file name, for the preview to resolve a `$` against.
fn list_names(paths: &[PathBuf]) -> Vec<String> {
    paths
        .iter()
        .filter_map(|path| Some(path.file_name()?.to_string_lossy().into_owned()))
        .collect()
}

/// One open list: the file, what is in it, and enough to tell our own writes
/// apart from somebody else's.
struct Open {
    path: PathBuf,
    doc: ratodo::model::Doc,
    mtime: Option<std::time::SystemTime>,
    /// The exact bytes the file had when we last read or wrote it. Every save
    /// wakes the watcher, and re-reading our own write would throw away the
    /// in-place update that keeps a ticked task from jumping.
    ///
    /// It is a render rather than the text that was read, because for an
    /// untouched document those are the same string — that is the round-trip
    /// guarantee, tested in tests/fidelity.rs.
    seen: String,
}

/// Reads every list.
///
/// The file name is stamped on to each task **only when there is more than
/// one**, and that single condition is the whole of the difference: a
/// single-file setup produces exactly the tasks it always did, with the
/// identities and so the calendar UIDs it always had.
fn open_all(paths: &[PathBuf]) -> Result<Vec<Open>> {
    let several = paths.len() > 1;
    paths
        .iter()
        .map(|path| {
            let loaded = write::load(path)?;
            let mut doc = loaded.doc;
            if several {
                let name = path.file_name().unwrap_or(path.as_os_str());
                let name = name.to_string_lossy().into_owned();
                for task in doc.tasks_mut() {
                    task.file = Some(name.clone());
                }
            }
            Ok(Open {
                seen: write::render(&doc),
                path: path.clone(),
                doc,
                mtime: loaded.mtime,
            })
        })
        .collect()
}

/// Every task on every list, in file order. A todo list is small; `agenda` wants
/// a slice, and this is not worth a lifetime.
fn all_tasks(open: &[Open]) -> Vec<ratodo::model::Task> {
    open.iter().flat_map(|o| o.doc.tasks().cloned()).collect()
}

/// Where the two derived files go, resolved **once** and then carried.
///
/// Not looked up again deeper in, for the same reason `write::save` takes its
/// backup directory as a parameter rather than finding one: a function that
/// reads the environment writes wherever the environment happens to point, and
/// the callers furthest from `main` are the tests. Until this existed, running
/// `cargo test` regenerated the developer's own `~/.local/share/ratodo/todo.ics`
/// from a fixture and filled `~/.local/state/ratodo` with a backup per case —
/// twenty-two megabytes of them, on the machine this was found on.
#[derive(Debug, Clone)]
struct Derived {
    backups: PathBuf,
    ics: PathBuf,
}

impl Derived {
    /// The real ones, from the environment. The only place that reads it.
    fn real() -> Result<Self> {
        Ok(Derived {
            backups: backup_dir()?,
            ics: ics_path()?,
        })
    }
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
fn sync(paths: &[PathBuf], to: &Path, loud: bool) -> Result<()> {
    let tasks = all_tasks(&open_all(paths)?);

    if let Some(parent) = to.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(to, ics::calendar(&tasks, Utc::now()))?;

    if loud {
        let exported = tasks.iter().filter(|t| t.open() && t.due.is_some()).count();
        writeln!(
            std::io::stdout(),
            "wrote {} dated task{} to {}",
            exported,
            if exported == 1 { "" } else { "s" },
            to.display()
        )?;
    }
    Ok(())
}

fn add(paths: &[PathBuf], derived: &Derived, input: &str) -> Result<()> {
    let today = Local::now().date_naive();
    let path = capture_into(input, paths)
        .map_err(|why| anyhow::anyhow!("{why} — the lists are {}", list_names(paths).join(", ")))?;
    let task = capture::capture(input, today);
    let summary = text::added(&task, today);

    let loaded = write::load(path)?;
    let mut doc = loaded.doc;
    doc.push_task(task);
    write::save(path, &doc, loaded.mtime, &derived.backups)?;

    writeln!(std::io::stdout(), "{summary}")?;
    quietly_sync(paths, &derived.ics);
    Ok(())
}

/// The capture already succeeded and said so. Whatever the calendar export
/// makes of it, the user's list is written and this is not the moment to make
/// noise about a derived file.
fn quietly_sync(paths: &[PathBuf], to: &Path) {
    let _ = sync(paths, to, false);
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
fn watch(paths: &[PathBuf], tx: std::sync::mpsc::Sender<()>) -> Option<notify::RecommendedWatcher> {
    use notify::Watcher;

    let names: Vec<std::ffi::OsString> = paths
        .iter()
        .filter_map(|p| p.file_name().map(std::ffi::OsStr::to_os_string))
        .collect();
    let mut watcher = notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
        if event
            .iter()
            .any(|e| names.iter().any(|name| touches(e, name)))
        {
            let _ = tx.send(());
        }
    })
    .ok()?;

    // Every directory holding a list, deduplicated: `--file` can point somewhere
    // else entirely, and watching one directory twice is two events per save.
    //
    // A directory that is not there yet cannot be watched, and that is not worth
    // refusing to open the list over.
    let mut dirs: Vec<&Path> = paths.iter().filter_map(|p| p.parent()).collect();
    dirs.sort();
    dirs.dedup();
    for dir in dirs {
        let _ = watcher.watch(dir, notify::RecursiveMode::NonRecursive);
    }
    Some(watcher)
}

fn tui(paths: &[PathBuf], derived: Derived, theme_flag: Option<&str>) -> Result<ExitCode> {
    // The capture target, because it is the file the empty screen is teaching
    // you to put your first task in.
    let shown_path = capture_target(paths).display().to_string();
    // What a `$` in the input box has to resolve against, so the preview can
    // answer it before `⏎` does.
    let lists = list_names(paths);
    let render = ui::Render {
        colours: active_theme(theme_flag),
        glyphs: ui::Glyphs::for_locale(locale().as_deref()),
        today: Local::now().date_naive(),
        path: &shown_path,
        lists: &lists,
    };
    let mut live = Live::read(paths, render.today, derived)?;

    let (tx, rx) = std::sync::mpsc::channel();
    // Kept alive for as long as the TUI runs: dropping it stops the watch.
    let _watcher = watch(paths, tx);

    // `try_init` installs a panic hook that turns raw mode off and leaves the
    // alternate screen before chaining to the default one, and `Terminal`'s own
    // Drop puts the cursor back. That is the whole of invariant 5, and it is the
    // library's, so it cannot drift out of step with the setup it undoes.
    let mut terminal = ratatui::try_init()?;
    let result = run(&mut terminal, &mut live, paths, &rx, render);
    ratatui::restore();
    result
}

/// The open lists, what the screen is showing of them, and one level of undo.
///
/// Every file is read into its own `Open`, and every change goes back to the
/// file its task came from: a write never touches a list the user was not
/// editing — docs/cli.md#several-lists.
struct Live {
    files: Vec<Open>,
    /// The document as it was before the last change, which file it belongs to,
    /// and what to call it.
    ///
    /// One level, which is what docs/tui.md promises: "undoes the last change in
    /// this session". A whole `Doc` per change is a few kilobytes and buys an
    /// undo that cannot be subtly wrong about what it is putting back.
    undo: Option<(usize, ratodo::model::Doc, String)>,
    screen: ui::Screen,
    counts: agenda::Counts,
    /// Resolved by the caller, so nothing under here can reach past it.
    derived: Derived,
}

impl Live {
    fn read(paths: &[PathBuf], today: chrono::NaiveDate, derived: Derived) -> Result<Self> {
        let files = open_all(paths)?;
        let tasks = all_tasks(&files);
        Ok(Live {
            counts: agenda::Counts::of(&tasks, today),
            screen: ui::Screen::new(ui::rows(&agenda::agenda(&tasks, today))),
            files,
            undo: None,
            derived,
        })
    }

    fn paths(&self) -> Vec<PathBuf> {
        self.files.iter().map(|f| f.path.clone()).collect()
    }

    /// What goes on a task captured into list `which`: the file name while
    /// several are open, and nothing at all while one is.
    fn stamp(&self, which: usize) -> Option<String> {
        (self.files.len() > 1)
            .then(|| self.files[which].path.file_name())
            .flatten()
            .map(|name| name.to_string_lossy().into_owned())
    }

    /// Which list a task on screen came from. `None` is the single-file case,
    /// where the answer is the only list there is.
    fn which(&self, file: Option<&str>) -> usize {
        file.and_then(|name| {
            self.files
                .iter()
                .position(|f| f.path.file_name().is_some_and(|had| had == name))
        })
        .unwrap_or(0)
    }

    fn reload(&mut self, paths: &[PathBuf], today: chrono::NaiveDate) -> Result<()> {
        self.files = open_all(paths)?;
        self.rebuild(today);
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
    ///
    /// It opens the file under the cursor, which with several lists is the only
    /// answer that is not a guess.
    fn edit(
        &mut self,
        terminal: &mut ratatui::DefaultTerminal,
        paths: &[PathBuf],
        today: chrono::NaiveDate,
    ) -> Result<ui::Notice> {
        let Some(command) = editor() else {
            return Ok(ui::Notice::Warned("no $EDITOR or $VISUAL set".to_string()));
        };
        let path = match self.selected() {
            Some((which, _)) => self.files[which].path.clone(),
            None => capture_target(paths).to_path_buf(),
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
            .arg(&path)
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
        self.reload(paths, today)?;
        quietly_sync(paths, &self.derived.ics);

        Ok(match outcome {
            Ok(_) => ui::Notice::Said("re-read after $EDITOR".to_string()),
            Err(e) => ui::Notice::Warned(format!("{}: {e}", command[0])),
        })
    }

    /// Rebuilds the rows and the counts from the documents already in memory.
    /// `reload` is this plus a read; a change made here does not need the read.
    fn rebuild(&mut self, today: chrono::NaiveDate) {
        let tasks = all_tasks(&self.files);
        self.counts = agenda::Counts::of(&tasks, today);
        self.screen
            .replace(ui::rows(&agenda::agenda(&tasks, today)));
    }

    /// Writes list `which`, and on a refusal puts its document back.
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
        which: usize,
        before: ratodo::model::Doc,
        undo: Option<String>,
    ) -> Result<Option<ui::Notice>> {
        let file = &mut self.files[which];
        match write::save(&file.path, &file.doc, file.mtime, &self.derived.backups) {
            Ok(()) => {
                file.seen = write::render(&file.doc);
                file.mtime = std::fs::metadata(&file.path)
                    .and_then(|m| m.modified())
                    .ok();
                self.undo = undo.map(|what| (which, before, what));
                quietly_sync(&self.paths(), &self.derived.ics);
                Ok(None)
            }
            // The file still says otherwise, and a screen that disagrees with
            // disk is how the *next* write loses data.
            Err(e) if e.downcast_ref::<write::Conflict>().is_some() => {
                file.doc = before;
                Ok(Some(ui::Notice::Warned(
                    "changed on disk - nothing was written.  r reload".to_string(),
                )))
            }
            Err(e) => {
                file.doc = before;
                Err(e)
            }
        }
    }

    /// The list and the raw line of the selected task, which is how a row on
    /// screen is found again in the document it came from.
    fn selected(&self) -> Option<(usize, String)> {
        let task = self.screen.task()?;
        Some((self.which(task.file.as_deref()), task.raw.clone()))
    }

    /// `spc` — done, or open again.
    fn toggle(&mut self, today: chrono::NaiveDate) -> Result<ui::Notice> {
        self.restate(today, |s| match s {
            State::Done => State::Open,
            _ => State::Done,
        })
    }

    /// `X` — decided against, or wanted again. The same key both ways, like
    /// `spc`: a state you can enter by accident needs a way straight back out.
    fn cancel(&mut self, today: chrono::NaiveDate) -> Result<ui::Notice> {
        self.restate(today, |s| match s {
            State::Cancelled => State::Open,
            _ => State::Cancelled,
        })
    }

    /// Moves the selected task between the three states, in its document and on
    /// the screen, and writes.
    ///
    /// The row is updated in place rather than regrouped: a task marked done
    /// stays where it is until the next reload. Nothing else on screen moves.
    fn restate(
        &mut self,
        today: chrono::NaiveDate,
        want: impl Fn(State) -> State,
    ) -> Result<ui::Notice> {
        let Some((which, raw)) = self.selected() else {
            return Ok(ui::Notice::Said("nothing to tick here".to_string()));
        };
        let before = self.files[which].doc.clone();
        let Some(task) = self.files[which].doc.tasks_mut().find(|t| t.raw == raw) else {
            return Ok(ui::Notice::Said("nothing to tick here".to_string()));
        };

        let to = want(task.state);
        task.set_state(to, today);
        let updated = task.clone();
        let title = text::plain(&updated.title);

        // Two strings, because they end up in different sentences: one is the
        // result line, the other is what `undo` reports putting back.
        let (doing, done) = match to {
            State::Done => ("ticking", "done"),
            State::Open => ("reopening", "reopened"),
            State::Cancelled => ("cancelling", "cancelled"),
        };

        if let Some(refused) = self.write_back(which, before, Some(format!("{doing} {title}")))? {
            return Ok(refused);
        }

        self.screen.update_selected(updated);
        let tasks = all_tasks(&self.files);
        self.counts = agenda::Counts::of(&tasks, today);
        Ok(ui::Notice::Said(format!("{done}: {title}    u undo")))
    }

    /// Deletes the selected task immediately, with no confirmation.
    ///
    /// A prompt taxes every delete to protect against the rare wrong one; `u`
    /// inverts that, so deleting costs one key and the mistake costs one more.
    /// See docs/tui.md#deleting--no-confirmation-dialog.
    fn delete(&mut self, today: chrono::NaiveDate) -> Result<ui::Notice> {
        let Some((which, raw)) = self.selected() else {
            return Ok(ui::Notice::Said("nothing to delete here".to_string()));
        };
        let Some(at) = self.files[which]
            .doc
            .lines
            .iter()
            .position(|l| matches!(&l.item, ratodo::model::Item::Task(t) if t.raw == raw))
        else {
            return Ok(ui::Notice::Said("nothing to delete here".to_string()));
        };

        let before = self.files[which].doc.clone();
        let Some(gone) = self.files[which].doc.remove_task(at) else {
            return Ok(ui::Notice::Said("nothing to delete here".to_string()));
        };
        let title = text::plain(&gone.title);

        if let Some(refused) =
            self.write_back(which, before, Some(format!("deleting \"{title}\"")))?
        {
            return Ok(refused);
        }

        self.rebuild(today);
        Ok(ui::Notice::Said(format!("deleted \"{title}\"    u undo")))
    }

    /// Saves what was typed on the input line: a new task, or the selected one
    /// rewritten.
    ///
    /// A new task goes to the capture target; an edit goes back to the file the
    /// task it is rewriting came from.
    ///
    /// An edit replaces the line's body and keeps its prefix byte for byte, so
    /// the indentation and the bullet the user chose survive being retyped —
    /// docs/architecture.md#round-trip-fidelity.
    fn save_typed(
        &mut self,
        paths: &[PathBuf],
        today: chrono::NaiveDate,
        input: &ui::Input,
    ) -> Result<ui::Notice> {
        let typed = input.text.trim();
        if typed.is_empty() {
            return Ok(ui::Notice::Said("nothing typed".to_string()));
        }

        // Refused before anything is opened or cloned: `p` is the one field
        // whose contents can be simply wrong. "3x" is not a length of time, and
        // "2222" is `22` typed twice — six years, which the file used to take
        // without a word. docs/tui.md#putting-a-date-off--p.
        let moved_to = match &input.purpose {
            ui::Purpose::Postpone(_) => match capture::later(typed, today) {
                Some(date) => Some(date),
                None => {
                    return Ok(ui::Notice::Warned(
                        "try 2, 3d, 1w, fri - a year at most, or write the date".to_string(),
                    ));
                }
            },
            _ => None,
        };

        // `$` addresses a capture. An edit writes back to the file its task came
        // from, and moving a line between two lists is two writes against two
        // mtimes — not this key. Refused out loud rather than swallowed, because
        // `capture` drops the word and silence would eat the user's text.
        if let ui::Purpose::Edit(_) = input.purpose
            && let Some(name) = capture::list_of(typed)
        {
            return Ok(ui::Notice::Warned(format!(
                "${name} moves nothing - $ picks the list for a new task"
            )));
        }

        let mut fields = capture::capture(typed, today);
        let title = text::plain(&fields.title);

        // A line that is all sigils and no words is a date, not a task — and
        // since `a` now opens with one in it, saving it straight back is a
        // keystroke away rather than a typo. `p` is exempt: what it was given is
        // a length of time and the title it keeps is the task's own.
        if moved_to.is_none() && title.trim().is_empty() {
            return Ok(ui::Notice::Said("nothing typed".to_string()));
        }

        let which = match input.purpose.raw() {
            Some(raw) => self
                .files
                .iter()
                .position(|f| f.doc.tasks().any(|t| t.raw == raw)),
            None => {
                let target = match capture_into(typed, paths) {
                    Ok(path) => path,
                    Err(why) => return Ok(ui::Notice::Warned(format!("{why} to capture into"))),
                };
                self.files.iter().position(|f| f.path == target)
            }
        };
        let Some(which) = which else {
            return Ok(ui::Notice::Warned(
                "that task is not there any more.  esc, then look again".to_string(),
            ));
        };
        let before = self.files[which].doc.clone();

        let (what, title) = match input.purpose.raw() {
            Some(raw) => {
                let Some(task) = self.files[which].doc.tasks_mut().find(|t| t.raw == raw) else {
                    return Ok(ui::Notice::Warned(
                        "that task is not there any more.  esc, then look again".to_string(),
                    ));
                };
                match moved_to {
                    Some(date) => {
                        // The task's own title: `p` was given a length of time,
                        // so the typed line has no title to report.
                        let title = text::plain(&task.title);
                        task.postpone(date);
                        ("put off", title)
                    }
                    None => {
                        // Against the **untrimmed** field, both times. `body`
                        // keeps whatever the line had after its last word, so a
                        // trimmed comparison calls an untouched line changed
                        // and a trimmed write then eats the spaces to prove it
                        // — docs/architecture.md#round-trip-fidelity.
                        if task.body() == input.text {
                            return Ok(ui::Notice::Said("unchanged".to_string()));
                        }
                        task.retype(&input.text, fields);
                        ("edited", title)
                    }
                }
            }
            None => {
                // Stamped like the tasks around it, or the new one would group
                // under a heading of its own until the next reload moved it.
                fields.file = self.stamp(which);
                self.files[which].doc.push_task(fields);
                ("added", title)
            }
        };

        if self
            .write_back(which, before, Some(format!("{what}: {title}")))?
            .is_some()
        {
            // Refused. Re-read here rather than leaving it to `r`, because the
            // caller is going to hand the sentence straight back to the field
            // and the next `⏎` has to go against the file as it now is.
            // Nothing is merged and nothing is overwritten — docs/tui.md#write-conflict.
            self.reload(paths, today)?;
            return Ok(ui::Notice::Warned(
                "changed on disk - re-read, then save again".to_string(),
            ));
        }

        self.rebuild(today);
        Ok(ui::Notice::Said(format!("{what}: {title}    u undo")))
    }

    /// Puts a document back the way it was before the last change.
    ///
    /// One level, and the whole document rather than an inverse operation: an
    /// undo that reconstructs what a change did is an undo that can be subtly
    /// wrong about it, and this one cannot.
    fn undo(&mut self, today: chrono::NaiveDate) -> Result<ui::Notice> {
        // Cloned rather than taken: if the write is refused the slot has to
        // still be there, or a refusal that changed nothing has cost the user
        // their undo.
        let Some((which, restored, what)) = self.undo.clone() else {
            return Ok(ui::Notice::Said("nothing to undo".to_string()));
        };

        let current = std::mem::replace(&mut self.files[which].doc, restored);
        if let Some(refused) = self.write_back(which, current, None)? {
            return Ok(refused);
        }

        self.rebuild(today);
        Ok(ui::Notice::Said(format!("undone: {what}")))
    }

    /// Whether the watcher's news is ours to act on.
    ///
    /// `changed` comes first and the `&&` is load-bearing: `stale` reads every
    /// list off disk, and the loop must not do that twice a second for nothing.
    fn wants_reload(&self, changed: bool) -> bool {
        changed && self.stale()
    }

    /// Whether any list on disk says something other than what we last read or
    /// wrote there. Our own save wakes the watcher too, and re-reading it would
    /// throw away the in-place update that keeps a ticked task from jumping.
    fn stale(&self) -> bool {
        self.files
            .iter()
            .any(|file| std::fs::read_to_string(&file.path).is_ok_and(|text| text != file.seen))
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
    paths: &[PathBuf],
    rx: &std::sync::mpsc::Receiver<()>,
    render: ui::Render<'_>,
) -> Result<ExitCode> {
    let today = render.today;
    let mut notice = ui::Notice::Hints;
    let mut helping = false;
    // `None` while the list is up. The period is remembered across a close, so
    // `s` twice does not put you back on the week you had just left.
    let mut showing: Option<agenda::Period> = None;
    let mut period = agenda::Period::default();
    // The other mode. `Some` for exactly as long as a line is being typed, and
    // only ever put there by `a`, `o`, `y`, `p` or `⏎`.
    let mut input: Option<ui::Input> = None;
    // The same mode with a screen round it. `a` opens this one wherever there is
    // room; `o`, `y` and `p` never do — docs/decisions.md.
    let mut form: Option<ui::Form> = None;
    // Drawing is still driven by events, not by the timer: a wake-up that finds
    // nothing to do draws nothing.
    let mut redraw = true;

    loop {
        if redraw {
            // Computed at draw time rather than held: every number in it is
            // derived from the file, so a stored copy is one more thing that can
            // be stale — docs/redesign.md, nothing the tool knows that the file
            // does not.
            let stats = showing.map(|period| agenda::stats(&all_tasks(&live.files), today, period));
            let view = match (&stats, helping) {
                (Some(stats), _) => ui::View::Stats(stats, period),
                (None, true) => ui::View::Help,
                (None, false) => ui::View::List,
            };
            terminal.draw(|frame| {
                ui::draw(
                    frame,
                    &mut live.screen,
                    live.counts,
                    render,
                    &notice,
                    view,
                    match (&form, &input) {
                        (Some(form), _) => ui::Open::Form(form),
                        (None, Some(input)) => ui::Open::Box(input),
                        (None, None) => ui::Open::Nothing,
                    },
                )
            })?;
            redraw = false;
        }

        if event::poll(IDLE)? {
            // A resize or a paste falls through to the redraw and nothing else.
            redraw = true;
            if let Event::Key(key) = event::read()? {
                // While the input is open the small keymap has the keyboard, and
                // that is what makes "nothing else can open it" true: an `a` in
                // there is a letter. `ctrl-c` cancels and never quits.
                // The form has its own keymap and keeps it to itself: every key
                // it answers is handled inside it, and the loop only hears the
                // two answers it has to act on.
                if let Some(open) = form.as_mut() {
                    match open.press(key) {
                        ui::Typed::Cancel => {
                            form = None;
                            notice = ui::Notice::Hints;
                        }
                        ui::Typed::Save => {
                            let typed = form.take().expect("the form was open a line ago");
                            notice = live.save_typed(paths, today, &typed.saving())?;
                            // A refusal keeps the form, and the sentence in it.
                            if matches!(notice, ui::Notice::Warned(_)) {
                                form = Some(typed);
                            }
                        }
                        _ => {}
                    }
                } else if let Some(typing) = input.as_mut() {
                    match ui::typing(key) {
                        ui::Typed::Char(c) => typing.insert(c),
                        ui::Typed::Back => typing.back(),
                        ui::Typed::Delete => typing.delete(),
                        ui::Typed::Left => typing.left(),
                        ui::Typed::Right => typing.right(),
                        ui::Typed::Home => typing.home(),
                        ui::Typed::End => typing.end(),
                        ui::Typed::Field => typing.toggle_field(today),
                        ui::Typed::Step(by) => typing.step(by),
                        // One `esc` per thing that is open: the field first, and
                        // the box only once there is no field over it.
                        ui::Typed::Cancel => {
                            if !typing.close_field() {
                                input = None;
                                notice = ui::Notice::Hints;
                            }
                        }
                        // And the same for `⏎`: it puts the date in the line
                        // before it ever saves one.
                        ui::Typed::Save if typing.apply_field() => {}
                        ui::Typed::Save => {
                            let typed = input.take().expect("the input was open a line ago");
                            notice = live.save_typed(paths, today, &typed)?;
                            // A refusal keeps the field, and the sentence in it.
                            // Losing it is the one thing docs/tui.md promises
                            // will not happen.
                            if matches!(notice, ui::Notice::Warned(_)) {
                                input = Some(typed);
                            }
                        }
                        ui::Typed::Ignore => {}
                    }
                } else {
                    // What the key means lives in `ui`, where it can be tested;
                    // this loop only knows how to read one and how to obey.
                    let what = ui::action(key);
                    // The stats screen replaces the list, so the list's keys are
                    // not on the screen and must not act behind it. `spc`
                    // ticking a task nobody can see is the failure this stops;
                    // the four that still mean something are the ones the screen
                    // itself names — docs/tui.md#stats.
                    let blocked = showing.is_some()
                        && !matches!(
                            what,
                            ui::Action::Quit
                                | ui::Action::Stats
                                | ui::Action::Over(_)
                                | ui::Action::Close
                                | ui::Action::Reload
                        );
                    match if blocked { ui::Action::Ignore } else { what } {
                        ui::Action::Quit => return Ok(ExitCode::SUCCESS),
                        ui::Action::Move(n) => {
                            live.screen.move_by(n);
                            notice = ui::Notice::Hints;
                        }
                        ui::Action::Top => live.screen.top(),
                        ui::Action::Bottom => live.screen.bottom(),
                        ui::Action::Toggle => notice = live.toggle(today)?,
                        ui::Action::Cancel => notice = live.cancel(today)?,
                        ui::Action::Delete => notice = live.delete(today)?,
                        ui::Action::Undo => notice = live.undo(today)?,
                        // The overlay comes down with it: a box over the list
                        // while a line is being typed covers the thing the line
                        // is about.
                        // `a` opens the form and `o` opens the box. Both are
                        // doors into the same file and both were already bound
                        // to the same one, so giving each a behaviour costs a
                        // key nobody has to learn: the vim hand that reaches for
                        // `o` is the one that wanted the fast path anyway —
                        // docs/tui.md#adding.
                        ui::Action::Add => {
                            let pane = terminal.size()?;
                            // One row is the bottom line, and the form is drawn
                            // in what is left.
                            let area = ratatui::layout::Rect::new(
                                0,
                                0,
                                pane.width,
                                pane.height.saturating_sub(1),
                            );
                            match ui::Form::fits(area) {
                                true => form = Some(ui::Form::adding(today, render.lists)),
                                false => input = Some(ui::Input::adding(today)),
                            }
                            helping = false;
                        }
                        ui::Action::Quick => {
                            input = Some(ui::Input::adding(today));
                            helping = false;
                        }
                        // The same form, prefilled, and the same fallback: a
                        // pane too small for it gets the box, which is what `⏎`
                        // has always opened.
                        ui::Action::Change => match live.screen.task() {
                            Some(task) => {
                                let pane = terminal.size()?;
                                let area = ratatui::layout::Rect::new(
                                    0,
                                    0,
                                    pane.width,
                                    pane.height.saturating_sub(1),
                                );
                                match ui::Form::fits(area) {
                                    true => {
                                        form = Some(ui::Form::editing(task, today, render.lists))
                                    }
                                    false => input = Some(ui::Input::editing(task)),
                                }
                                helping = false;
                            }
                            None => notice = ui::Notice::Said("nothing to edit here".to_string()),
                        },
                        ui::Action::Duplicate => match live.screen.task() {
                            Some(task) => {
                                input = Some(ui::Input::duplicating(task, today));
                                helping = false;
                            }
                            None => notice = ui::Notice::Said("nothing to copy here".to_string()),
                        },
                        ui::Action::Postpone => match live.screen.task() {
                            Some(task) => {
                                input = Some(ui::Input::postponing(task));
                                helping = false;
                            }
                            None => {
                                notice = ui::Notice::Said("nothing to put off here".to_string())
                            }
                        },
                        ui::Action::Fold(want) => {
                            if let Some(complaint) = live.screen.fold(want) {
                                notice = ui::Notice::Said(complaint.to_string());
                            }
                        }
                        ui::Action::Edit => {
                            notice = live.edit(terminal, paths, today)?;
                        }
                        ui::Action::Reload => {
                            live.reload(paths, today)?;
                            notice = ui::Notice::Said("reloaded".to_string());
                        }
                        ui::Action::Help => helping = !helping,
                        ui::Action::Stats => {
                            showing = match showing {
                                Some(_) => None,
                                None => Some(period),
                            };
                            helping = false;
                        }
                        // Only while the screen they belong to is up. On the
                        // list they are three keys bound to nothing, which is
                        // what they were before this screen existed.
                        ui::Action::Over(want) => {
                            if showing.is_some() {
                                period = want;
                                showing = Some(want);
                            }
                        }
                        // `esc` puts the overlay down and otherwise does nothing
                        // at all. It must never quit.
                        ui::Action::Close => {
                            helping = false;
                            showing = None;
                        }
                        ui::Action::Say(what) => notice = ui::Notice::Said(what.to_string()),
                        ui::Action::Ignore => {}
                    }
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

        if live.wants_reload(changed) {
            live.reload(paths, today)?;
            redraw = true;
        }
    }
}

/// Exit 2 — "asked, could not answer" — for both no match and too many, and in
/// neither case is a file opened for writing.
///
/// Across every open list, because the whole point of several of them is that
/// they are one list to the person using them. Two files holding the same title
/// is the ambiguous case, not a race between them.
fn done(
    paths: &[PathBuf],
    derived: &Derived,
    input: &str,
    today: chrono::NaiveDate,
) -> Result<ExitCode> {
    let mut open = open_all(paths)?;

    let mut hits: Vec<(usize, usize)> = Vec::new();
    let mut candidates: Vec<String> = Vec::new();
    let mut already: Option<String> = None;

    for (which, list) in open.iter().enumerate() {
        match list.doc.find_open(input) {
            Lookup::One(at) => hits.push((which, at)),
            Lookup::Several(mut found) => candidates.append(&mut found),
            Lookup::AlreadyDone(title) => already = Some(title),
            Lookup::None => {}
        }
    }

    // A single hit anywhere, and nothing else close to it, is the only case that
    // writes. The titles of the other hits join the candidate list, or a task
    // that lost by being in the second file would never be shown to the user.
    if hits.len() + candidates.len() > 1 {
        let mut all = candidates;
        for (which, at) in hits {
            if let Some(task) = open[which].doc.task_at(at) {
                all.push(task.title.clone());
            }
        }
        all.sort();
        eprintln!("{}", text::ambiguous(input, &all));
        return Ok(ExitCode::from(2));
    }

    let Some((which, at)) = hits.pop() else {
        if let Some(title) = already {
            eprintln!("already done: {}", text::plain(&title));
            return Ok(ExitCode::SUCCESS);
        }
        eprintln!("no open task matches '{}'", text::plain(input));
        return Ok(ExitCode::from(2));
    };

    let list = &mut open[which];
    let task = list
        .doc
        .task_at_mut(at)
        .context("the matched line stopped being a task")?;
    task.set_state(State::Done, today);
    let summary = text::marked_done(task);

    write::save(&list.path, &list.doc, list.mtime, &derived.backups)?;
    writeln!(std::io::stdout(), "{summary}")?;
    quietly_sync(paths, &derived.ics);
    Ok(ExitCode::SUCCESS)
}

fn list(paths: &[PathBuf], args: &ListArgs) -> Result<()> {
    let open = open_all(paths)?;
    let today = Local::now().date_naive();
    let filter = agenda::Filter {
        tags: &args.tag,
        prio: args.prio,
    };

    let all = all_tasks(&open);
    let tasks: Vec<_> = all.iter().filter(|t| filter.matches(t)).cloned().collect();
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
        if all.is_empty() {
            eprintln!("nothing here yet — try: ratodo add 'buy milk @tomorrow #home'");
            eprintln!("file: {}", capture_target(paths).display());
        } else {
            eprintln!("no task matches that filter");
        }
        return Ok(());
    }

    for group in &groups {
        if let Some(heading) = group.kind.heading() {
            writeln!(out, "\n{}", text::plain(&heading))?;
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
fn status(paths: &[PathBuf], json: bool) -> Result<ExitCode> {
    let today = Local::now().date_naive();
    let tasks = all_tasks(&open_all(paths)?);
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

        let live = Live::read(std::slice::from_ref(&path), a_day(), derived(&dir)).unwrap();
        (path, live)
    }

    fn a_day() -> chrono::NaiveDate {
        chrono::NaiveDate::from_ymd_opt(2026, 8, 11).unwrap()
    }

    /// The derived files, inside the case's own directory.
    ///
    /// Never `Derived::real()`: these tests drive `write_back` in-process, so
    /// the backup and the calendar go wherever this says. Pointed at the real
    /// ones it rewrote the developer's own `todo.ics` from a fixture and left a
    /// `.bak` per case in `~/.local/state/ratodo`, which is how it was found.
    fn derived(dir: &std::path::Path) -> Derived {
        Derived {
            backups: dir.join("state"),
            ics: dir.join("data").join("todo.ics"),
        }
    }

    /// Both derived files land where the caller pointed them, and the caller is
    /// `main`.
    ///
    /// This is the guard against a whole class of bug rather than one instance
    /// of it. `write_back` used to call `backup_dir()` and resolve the calendar
    /// path itself, both of which read the environment — so every in-process
    /// test wrote into the developer's own `~/.local`, regenerating their real
    /// `todo.ics` from a fixture and leaving a `.bak` per case behind. Pinning
    /// that the paths come from the `Derived` handed in is what makes that
    /// impossible instead of merely fixed.
    #[test]
    fn the_derived_files_land_where_the_caller_pointed_them() {
        let (path, mut live) = open("derived", "- [ ] first @2026-08-20\n");
        let dir = path.parent().expect("a directory").to_path_buf();
        let want = derived(&dir);

        live.toggle(a_day()).unwrap();

        assert!(
            std::fs::read_to_string(&want.ics)
                .expect("no calendar under the given path")
                .contains("BEGIN:VCALENDAR"),
            "the calendar did not go where it was told"
        );
        let backups: Vec<PathBuf> = std::fs::read_dir(&want.backups)
            .expect("no backup directory under the given path")
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .collect();
        assert_eq!(backups.len(), 1, "expected one backup, got {backups:?}");
        assert_eq!(
            std::fs::read_to_string(&backups[0]).unwrap(),
            "- [ ] first @2026-08-20\n",
            "the backup does not hold the pre-write list"
        );
    }

    /// `spc` and `X` are each a way in **and** the way back out, which is what
    /// makes a state you can enter by accident survivable. Pinned in the file
    /// rather than in the enum: the tick has a stamp to take back off with it,
    /// and cancelling must not leave one behind.
    #[test]
    fn ticking_and_cancelling_both_go_both_ways() {
        let before = "## Work\n- [ ] first @2026-08-20\n- [ ] second\n";

        let (path, mut live) = open("toggle-back", before);
        live.toggle(a_day()).unwrap();
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "## Work\n- [x] first @2026-08-20 ✓2026-08-11\n- [ ] second\n"
        );
        live.toggle(a_day()).unwrap();
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            before,
            "unticking left the stamp behind"
        );

        let (path, mut live) = open("cancel-back", before);
        live.cancel(a_day()).unwrap();
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "## Work\n- [-] first @2026-08-20\n- [ ] second\n",
            "cancelling is not finishing, so it does not stamp"
        );
        live.cancel(a_day()).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), before);

        // And the two are not the same door: `d` on a ticked task cancels it
        // rather than reopening it, and takes the stamp with it.
        let (path, mut live) = open("tick-then-cancel", before);
        live.toggle(a_day()).unwrap();
        live.cancel(a_day()).unwrap();
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "## Work\n- [-] first @2026-08-20\n- [ ] second\n"
        );
    }

    /// `p` saves through the same path as an edit but means something else: move
    /// the date, keep the time, and leave everything the parser did not
    /// understand exactly where it is.
    #[test]
    fn putting_a_date_off_moves_the_date_and_nothing_else() {
        let raw = "- [ ] first @2026-08-20 16:00 #ops !high";
        let before = format!("## Work\n{raw}\n> a note\n");
        let (path, mut live) = open("postpone", &before);

        let input = ui::Input::new("3d".to_string(), ui::Purpose::Postpone(raw.to_string()));
        let notice = live
            .save_typed(std::slice::from_ref(&path), a_day(), &input)
            .unwrap();
        assert!(
            matches!(&notice, ui::Notice::Said(s) if s.contains("put off: first")),
            "{notice:?}"
        );

        let moved = "## Work\n- [ ] first @2026-08-14 16:00 #ops !high\n> a note\n";
        assert_eq!(std::fs::read_to_string(&path).unwrap(), moved);

        // A bare number is days, and it is the answer the box was asking for.
        let now = "- [ ] first @2026-08-14 16:00 #ops !high";
        let input = ui::Input::new("1".to_string(), ui::Purpose::Postpone(now.to_string()));
        live.save_typed(std::slice::from_ref(&path), a_day(), &input)
            .unwrap();
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "## Work\n- [ ] first @2026-08-12 16:00 #ops !high\n> a note\n"
        );

        // What is not a length of time is refused before the file is opened —
        // and so is one past the horizon, because `2222` is `22` typed twice
        // and six years is not the question the box asked.
        let then = std::fs::read_to_string(&path).unwrap();
        for typed in ["3x", "2222", "2222d", "222w"] {
            let bad = ui::Input::new(
                typed.to_string(),
                ui::Purpose::Postpone("- [ ] first @2026-08-12 16:00 #ops !high".to_string()),
            );
            let notice = live
                .save_typed(std::slice::from_ref(&path), a_day(), &bad)
                .unwrap();
            assert!(
                matches!(&notice, ui::Notice::Warned(text) if text.contains("a year at most")),
                "{typed}: {notice:?}"
            );
            assert_eq!(
                std::fs::read_to_string(&path).unwrap(),
                then,
                "{typed}: a refusal wrote something"
            );
        }

        // The way past the horizon is to write the date, which is why the
        // refusal names it.
        let far = ui::Input::new(
            "2032-09-10".to_string(),
            ui::Purpose::Postpone("- [ ] first @2026-08-12 16:00 #ops !high".to_string()),
        );
        live.save_typed(std::slice::from_ref(&path), a_day(), &far)
            .unwrap();
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "## Work\n- [ ] first @2032-09-10 16:00 #ops !high\n> a note\n"
        );
    }

    /// Deleting takes one line out and leaves everything else exactly as it was,
    /// including the things ratodo does not understand.
    #[test]
    fn delete_removes_the_selected_line_and_nothing_else() {
        let before = "# My list\n\n## Work\n- [ ] first\n- [ ] second\n\n> a note\n";
        let (path, mut live) = open("delete", before);

        let notice = live.delete(a_day()).unwrap();
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
    ///
    /// Unix-only because the failure is forced with a read-only directory mode,
    /// which Windows has no equivalent for — and without the gate the whole test
    /// binary refuses to compile there, so the suite cannot run at all.
    #[test]
    #[cfg(unix)]
    fn a_write_that_fails_for_any_other_reason_is_not_reported_as_a_conflict() {
        use std::os::unix::fs::PermissionsExt;

        let (path, mut live) = open("readonly", "- [ ] first\n");
        let dir = path.parent().unwrap().to_path_buf();

        // The temp file the atomic write needs cannot be created in here.
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o555)).unwrap();
        let outcome = live.toggle(a_day());
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

        live.delete(a_day()).unwrap();
        let notice = live.undo(a_day()).unwrap();

        assert!(
            matches!(&notice, ui::Notice::Said(s) if s.contains("undone")),
            "{notice:?}"
        );
        assert_eq!(std::fs::read_to_string(&path).unwrap(), before);

        // One level, and it is spent.
        assert!(
            matches!(live.undo(a_day()).unwrap(), ui::Notice::Said(s) if s.contains("nothing to undo"))
        );

        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn undo_takes_back_a_toggle_too() {
        let before = "- [ ] first\n";
        let (path, mut live) = open("undo-toggle", before);

        live.toggle(a_day()).unwrap();
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "- [x] first ✓2026-08-11\n"
        );

        live.undo(a_day()).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), before);

        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    fn typed(text: &str, editing: Option<&str>) -> ui::Input {
        let purpose = match editing {
            Some(raw) => ui::Purpose::Edit(raw.to_string()),
            None => ui::Purpose::Add,
        };
        ui::Input::new(text.to_string(), purpose)
    }

    /// `a` then a sentence then `⏎`. It lands where `ratodo add` puts it — after
    /// the last task, not at the end of the file — and `u` takes it back.
    #[test]
    fn the_input_adds_a_task_and_undo_takes_it_away_again() {
        let before = "## Work\n- [ ] first\n\n> a note\n";
        let (path, mut live) = open("input-add", before);

        let notice = live
            .save_typed(
                std::slice::from_ref(&path),
                a_day(),
                &typed("call the bank @tomorrow #work", None),
            )
            .unwrap();
        assert!(
            matches!(&notice, ui::Notice::Said(s) if s.contains("added: call the bank") && s.contains("u undo")),
            "{notice:?}"
        );
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "## Work\n- [ ] first\n- [ ] call the bank @2026-08-12 #work\n\n> a note\n"
        );
        assert_eq!(live.counts.open, 2, "the screen did not catch up");

        live.undo(a_day()).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), before);

        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    /// `⏎` on a task rewrites that line and leaves every other byte in the file
    /// alone — including the shape of the line it rewrote.
    #[test]
    fn the_input_edits_one_line_and_keeps_its_prefix() {
        let before = "# List\n  * [x] wash up @2026-08-01\n- [ ] second\n";
        let (path, mut live) = open("input-edit", before);

        let notice = live
            .save_typed(
                std::slice::from_ref(&path),
                a_day(),
                &typed(
                    "wash up properly !high",
                    Some("  * [x] wash up @2026-08-01"),
                ),
            )
            .unwrap();
        assert!(
            matches!(&notice, ui::Notice::Said(s) if s.contains("edited: wash up properly")),
            "{notice:?}"
        );
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "# List\n  * [x] wash up properly !high\n- [ ] second\n"
        );

        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    /// Two ways of asking for nothing, and neither of them touches the file or
    /// spends the undo: an empty line, and an edit that changed nothing.
    #[test]
    fn an_input_that_says_nothing_writes_nothing() {
        let before = "- [ ] first\n";
        let (path, mut live) = open("input-nothing", before);

        for input in [typed("   ", None), typed("first", Some("- [ ] first"))] {
            let notice = live
                .save_typed(std::slice::from_ref(&path), a_day(), &input)
                .unwrap();
            assert!(matches!(&notice, ui::Notice::Said(_)), "{notice:?}");
            assert_eq!(std::fs::read_to_string(&path).unwrap(), before);
            assert!(
                live.undo.is_none(),
                "a write that did not happen left an undo"
            );
        }

        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    /// Somebody else wrote first. Nothing is merged and nothing is overwritten,
    /// but the file is re-read on the spot, because the caller is about to hand
    /// the sentence back to the field and the next `⏎` has to have a chance of
    /// working.
    #[test]
    fn a_refused_capture_re_reads_so_the_second_try_goes_through() {
        let (path, mut live) = open("input-conflict", "- [ ] first\n");

        std::thread::sleep(std::time::Duration::from_millis(20));
        std::fs::write(&path, "- [ ] first\n- [ ] theirs\n").unwrap();

        let input = typed("mine", None);
        let refused = live
            .save_typed(std::slice::from_ref(&path), a_day(), &input)
            .unwrap();
        assert!(
            matches!(&refused, ui::Notice::Warned(w) if w.contains("changed on disk")),
            "{refused:?}"
        );
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "- [ ] first\n- [ ] theirs\n",
            "the refused write went through anyway"
        );

        // The sentence is still the caller's to keep, and now it lands — on top
        // of the other writer's line, not instead of it.
        live.save_typed(std::slice::from_ref(&path), a_day(), &input)
            .unwrap();
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "- [ ] first\n- [ ] theirs\n- [ ] mine\n"
        );

        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    /// The line being edited went away while it was being typed — a `git pull`,
    /// or the other pane. Saying so beats writing the sentence somewhere it does
    /// not belong.
    #[test]
    fn editing_a_line_that_is_no_longer_there_writes_nothing() {
        let (path, mut live) = open("input-vanished", "- [ ] first\n");

        let notice = live
            .save_typed(
                std::slice::from_ref(&path),
                a_day(),
                &typed("whatever", Some("- [ ] gone")),
            )
            .unwrap();
        assert!(
            matches!(&notice, ui::Notice::Warned(w) if w.contains("not there any more")),
            "{notice:?}"
        );
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "- [ ] first\n");

        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    /// A refusal changes nothing — which has to include the undo slot. Losing it
    /// to a write that did not happen would be the worst of both.
    #[test]
    fn a_refused_undo_keeps_the_undo() {
        let (path, mut live) = open("undo-conflict", "- [ ] first\n- [ ] second\n");
        live.delete(a_day()).unwrap();

        // Somebody else gets there first.
        std::thread::sleep(std::time::Duration::from_millis(20));
        std::fs::write(&path, "- [ ] theirs\n").unwrap();

        let refused = live.undo(a_day()).unwrap();
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
            matches!(live.delete(a_day()).unwrap(), ui::Notice::Said(s) if s.contains("nothing to delete"))
        );
        assert!(
            matches!(live.undo(a_day()).unwrap(), ui::Notice::Said(s) if s.contains("nothing to undo"))
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
        let mut live = Live::read(std::slice::from_ref(&path), today, derived(&dir)).unwrap();

        // Somebody else gets there first. The sleep is for filesystems whose
        // mtime resolution is coarser than two writes in a row.
        std::thread::sleep(std::time::Duration::from_millis(20));
        std::fs::write(&path, "- [ ] mine\n- [ ] theirs\n").unwrap();

        let notice = live.toggle(today).unwrap();
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
            live.files[0].doc.tasks().all(|t| !t.done()),
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

    /// A scratch directory holding several lists, and a `Live` open on all of
    /// them in the order `lists` would hand them over.
    fn open_several(tag: &str, files: &[(&str, &str)]) -> (PathBuf, Vec<PathBuf>, Live) {
        let dir = std::env::temp_dir().join(format!("ratodo-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let mut paths: Vec<PathBuf> = files
            .iter()
            .map(|(name, text)| {
                let path = dir.join(name);
                std::fs::write(&path, text).unwrap();
                path
            })
            .collect();
        paths.sort();

        let live = Live::read(&paths, a_day(), derived(&dir)).unwrap();
        (dir, paths, live)
    }

    /// The point of the feature: files kept apart on disk, one agenda on screen,
    /// and a change that goes back to the file it came from and nowhere else.
    #[test]
    fn several_lists_are_one_agenda_and_a_write_touches_one_file() {
        let work = "## Sprint\n- [ ] deploy the thing @2026-08-09\n- [ ] write the notes\n";
        let home = "## House\n- [ ] fix the tap\n";
        let (dir, paths, mut live) =
            open_several("several", &[("work.md", work), ("home.md", home)]);

        // Both files are on screen, and each undated heading says which file it
        // came out of.
        let tasks = all_tasks(&live.files);
        let titles: Vec<String> = ui::rows(&agenda::agenda(&tasks, a_day()))
            .iter()
            .filter_map(|row| match row {
                ui::Row::Header { title, .. } => Some(title.clone()),
                _ => None,
            })
            .collect();
        assert!(
            titles.contains(&"## Sprint (work.md)".to_string()),
            "{titles:?}"
        );
        assert!(
            titles.contains(&"## House (home.md)".to_string()),
            "{titles:?}"
        );
        assert_eq!(live.counts.open, 3);

        // The cursor starts on the overdue task, which lives in work.md.
        live.toggle(a_day()).unwrap();
        assert_eq!(
            std::fs::read_to_string(paths.iter().find(|p| p.ends_with("work.md")).unwrap())
                .unwrap(),
            "## Sprint\n- [x] deploy the thing @2026-08-09 ✓2026-08-11\n- [ ] write the notes\n"
        );
        assert_eq!(
            std::fs::read_to_string(paths.iter().find(|p| p.ends_with("home.md")).unwrap())
                .unwrap(),
            home,
            "the other list was rewritten by a change that had nothing to do with it"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Two files can hold the same heading and the same title, and they are not
    /// the same task: the groups stay apart and so do the identities.
    #[test]
    fn the_same_heading_in_two_files_is_two_groups() {
        let (dir, _, live) = open_several(
            "same-heading",
            &[
                ("a.md", "## Work\n- [ ] review\n"),
                ("b.md", "## Work\n- [ ] review\n"),
            ],
        );

        let tasks = all_tasks(&live.files);
        let headers: Vec<String> = ui::rows(&agenda::agenda(&tasks, a_day()))
            .iter()
            .filter_map(|row| match row {
                ui::Row::Header { title, .. } => Some(title.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(headers, ["## Work (a.md)", "## Work (b.md)"], "{headers:?}");

        let ids: Vec<String> = tasks.iter().map(|t| t.identity()).collect();
        assert_ne!(ids[0], ids[1], "two files, one identity: {ids:?}");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// One list is stamped with nothing at all, so every identity — and so every
    /// calendar UID — is the one it was before several files were a thing.
    #[test]
    fn one_list_is_stamped_with_nothing() {
        let (path, live) = open("one-list", "## Work\n- [ ] review\n");
        let task = all_tasks(&live.files).remove(0);

        assert_eq!(task.file, None);
        assert_eq!(task.identity(), format!("Work\u{1}review"));

        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    /// A line of sigils and no words is a date, not a task. `a` opens with
    /// `@today` already in it, so saving one straight back is a keystroke rather
    /// than a typo — and a titleless line in the file is what it used to write.
    #[test]
    fn a_capture_with_nothing_but_fields_in_it_is_refused() {
        let before = "- [ ] mine\n";
        let (path, mut live) = open("titleless", before);

        for text in ["@2026-08-10", "@thu #home !high", "  "] {
            let notice = live
                .save_typed(std::slice::from_ref(&path), a_day(), &typed(text, None))
                .unwrap();
            assert!(
                matches!(&notice, ui::Notice::Said(s) if s == "nothing typed"),
                "{text:?} gave {notice:?}"
            );
            assert_eq!(std::fs::read_to_string(&path).unwrap(), before, "{text:?}");
        }

        // The date is not the problem — a title beside it is all it wanted.
        live.save_typed(
            std::slice::from_ref(&path),
            a_day(),
            &typed("@2026-08-10 buy milk", None),
        )
        .unwrap();
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "- [ ] mine\n- [ ] buy milk @2026-08-10\n"
        );
    }

    /// `a` means the same file wherever the cursor is. The task the cursor was
    /// over is in work.md; the capture still lands in todo.md.
    #[test]
    fn a_capture_lands_in_todo_md_whatever_the_cursor_is_over() {
        let (dir, paths, mut live) = open_several(
            "capture",
            &[("todo.md", "- [ ] mine\n"), ("work.md", "- [ ] theirs\n")],
        );

        live.save_typed(&paths, a_day(), &typed("buy milk", None))
            .unwrap();

        assert_eq!(
            std::fs::read_to_string(dir.join("todo.md")).unwrap(),
            "- [ ] mine\n- [ ] buy milk\n"
        );
        assert_eq!(
            std::fs::read_to_string(dir.join("work.md")).unwrap(),
            "- [ ] theirs\n"
        );
        // And it is stamped like the tasks around it, or it would sit under a
        // heading of its own until the next reload moved it.
        let added = all_tasks(&live.files)
            .into_iter()
            .find(|t| t.title == "buy milk")
            .expect("the captured task is not on screen");
        assert_eq!(added.file.as_deref(), Some("todo.md"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// `$work` sends this one capture to `work.md` — the default target is not
    /// touched, and the word is not written to either file.
    #[test]
    fn a_dollar_sends_the_capture_to_the_list_it_names() {
        let (dir, paths, mut live) = open_several(
            "capture-dollar",
            &[("todo.md", "- [ ] mine\n"), ("work.md", "- [ ] theirs\n")],
        );

        live.save_typed(&paths, a_day(), &typed("buy milk $work", None))
            .unwrap();

        assert_eq!(
            std::fs::read_to_string(dir.join("work.md")).unwrap(),
            "- [ ] theirs\n- [ ] buy milk\n"
        );
        assert_eq!(
            std::fs::read_to_string(dir.join("todo.md")).unwrap(),
            "- [ ] mine\n"
        );
        let added = all_tasks(&live.files)
            .into_iter()
            .find(|t| t.title == "buy milk")
            .expect("the captured task is not on screen");
        assert_eq!(added.file.as_deref(), Some("work.md"));

        // And with the extension typed out, which is the same list.
        live.save_typed(&paths, a_day(), &typed("buy bread $work.md", None))
            .unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.join("work.md")).unwrap(),
            "- [ ] theirs\n- [ ] buy milk\n- [ ] buy bread\n"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A `$` naming no list is refused before anything is written. It does not
    /// create the file, and it does not quietly fall back to `todo.md` either —
    /// falling back would put the task somewhere the user did not ask for.
    #[test]
    fn a_dollar_naming_no_list_writes_nothing() {
        let (dir, paths, mut live) = open_several(
            "capture-nolist",
            &[("todo.md", "- [ ] mine\n"), ("work.md", "- [ ] theirs\n")],
        );

        let notice = live
            .save_typed(&paths, a_day(), &typed("buy milk $wrok", None))
            .unwrap();
        assert!(
            matches!(&notice, ui::Notice::Warned(text) if text.contains("wrok.md")),
            "{notice:?}"
        );

        assert_eq!(
            std::fs::read_to_string(dir.join("todo.md")).unwrap(),
            "- [ ] mine\n"
        );
        assert_eq!(
            std::fs::read_to_string(dir.join("work.md")).unwrap(),
            "- [ ] theirs\n"
        );
        assert!(!dir.join("wrok.md").exists(), "a capture invented a list");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// `$` addresses a capture. On an edit it is refused rather than swallowed:
    /// `capture` drops the word, so a silent save would eat the user's text.
    #[test]
    fn a_dollar_in_an_edit_is_refused() {
        let (dir, paths, mut live) = open_several(
            "edit-dollar",
            &[("todo.md", "- [ ] mine\n"), ("work.md", "- [ ] theirs\n")],
        );

        let notice = live
            .save_typed(&paths, a_day(), &typed("mine $work", Some("- [ ] mine")))
            .unwrap();
        assert!(
            matches!(&notice, ui::Notice::Warned(text) if text.contains("$work")),
            "{notice:?}"
        );

        assert_eq!(
            std::fs::read_to_string(dir.join("todo.md")).unwrap(),
            "- [ ] mine\n"
        );
        assert_eq!(
            std::fs::read_to_string(dir.join("work.md")).unwrap(),
            "- [ ] theirs\n"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// With no `todo.md` among them the capture goes to the first list rather
    /// than nowhere.
    #[test]
    fn without_a_todo_md_the_capture_target_is_the_first_list() {
        let dir = std::env::temp_dir().join(format!("ratodo-target-{}", std::process::id()));
        let paths = [dir.join("a.md"), dir.join("b.md")];
        assert_eq!(capture_target(&paths), paths[0]);

        let with_todo = [dir.join("a.md"), dir.join("todo.md")];
        assert_eq!(capture_target(&with_todo), with_todo[1]);
    }

    /// `done` reads every list, so a title in the second file is found — and a
    /// title in *both* is ambiguous rather than a race between them.
    #[test]
    fn done_matches_across_every_list() {
        let (dir, paths, _) = open_several(
            "done-across",
            &[
                ("work.md", "- [ ] fix the tap\n- [ ] ship the tag\n"),
                ("home.md", "- [ ] fix the tap\n"),
            ],
        );
        let before: Vec<String> = paths
            .iter()
            .map(|p| std::fs::read_to_string(p).unwrap())
            .collect();

        // In both files: nothing is written and it says so with exit 2.
        let code = done(&paths, &derived(&dir), "fix the tap", a_day()).unwrap();
        assert_eq!(format!("{code:?}"), format!("{:?}", ExitCode::from(2)));
        for (path, was) in paths.iter().zip(&before) {
            assert_eq!(&std::fs::read_to_string(path).unwrap(), was, "{path:?}");
        }

        // In one file only: ticked there, and the other list is untouched.
        done(&paths, &derived(&dir), "ship the tag", a_day()).unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.join("work.md")).unwrap(),
            "- [ ] fix the tap\n- [x] ship the tag ✓2026-08-11\n"
        );
        assert_eq!(
            std::fs::read_to_string(dir.join("home.md")).unwrap(),
            "- [ ] fix the tap\n"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The three answers the rest of the multi-file behaviour is built on, each
    /// of which a mutation could quietly invert: which files a change syncs, when
    /// a task is stamped with its file, and whether the screen is out of date.
    #[test]
    fn what_live_knows_about_its_files() {
        let (dir, paths, mut live) = open_several(
            "live-knows",
            &[("todo.md", "- [ ] mine\n"), ("work.md", "- [ ] theirs\n")],
        );

        // Every list, or the calendar would be rebuilt from half of them.
        assert_eq!(live.paths(), paths);

        // Two files, so a capture is stamped; one file, and it is not — that
        // boundary is the whole of what keeps single-file identities intact.
        assert_eq!(live.stamp(0).as_deref(), Some("todo.md"));
        let (one, single) = open("live-knows-one", "- [ ] mine\n");
        assert_eq!(single.stamp(0), None);
        let _ = std::fs::remove_dir_all(one.parent().unwrap());

        // Nothing has moved: the files say what we last read.
        assert!(!live.stale());

        // Our own write does not count as a change, or the reload it triggers
        // would throw away the in-place update and let the ticked task jump.
        live.toggle(a_day()).unwrap();
        assert!(!live.stale(), "our own save read as somebody else's");

        // Somebody else's does.
        std::fs::write(dir.join("work.md"), "- [ ] theirs\n- [ ] and mine\n").unwrap();
        assert!(live.stale(), "an outside edit went unnoticed");

        // And the loop only asks the disk when the watcher said something: the
        // check reads every list, and doing that on every idle wake-up is the
        // "no fixed FPS" rule broken by the back door.
        assert!(live.wants_reload(true));
        assert!(!live.wants_reload(false), "the disk was read for nothing");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Every `*.md` in the config directory is a list, sorted, and nothing else
    /// in there is. `--file` and `$RATODO_FILE` still name exactly one.
    #[test]
    fn which_files_count_as_lists() {
        let dir = std::env::temp_dir().join(format!("ratodo-lists-{}", std::process::id()));
        let config = dir.join("ratodo");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&config).unwrap();
        for name in ["work.md", "2026.md", "theme.conf", "notes.txt"] {
            std::fs::write(config.join(name), "").unwrap();
        }
        std::fs::create_dir_all(config.join("archive.md")).unwrap();

        let was = std::env::var_os("XDG_CONFIG_HOME");
        // SAFETY: single-threaded within this test, and put back before it ends.
        unsafe { std::env::set_var("XDG_CONFIG_HOME", &dir) };

        let found = lists(None).unwrap();
        let names: Vec<String> = found
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, ["2026.md", "work.md"], "sorted, and only the lists");

        // A named file is the only file, whatever else is in the directory.
        let one = lists(Some(PathBuf::from("/tmp/elsewhere.md"))).unwrap();
        assert_eq!(one, [PathBuf::from("/tmp/elsewhere.md")]);

        // An empty directory still answers with the default path, which is the
        // file the empty screen teaches you to put your first task in.
        std::fs::remove_dir_all(&config).unwrap();
        std::fs::create_dir_all(&config).unwrap();
        assert_eq!(lists(None).unwrap(), [config.join("todo.md")]);

        // SAFETY: as above.
        unsafe {
            match was {
                Some(value) => std::env::set_var("XDG_CONFIG_HOME", value),
                None => std::env::remove_var("XDG_CONFIG_HOME"),
            }
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
