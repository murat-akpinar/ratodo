//! Subcommands and terminal setup. See docs/cli.md.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result};
use chrono::Local;
use clap::{Parser, Subcommand};

use ratodo::model::{Lookup, Priority};
use ratodo::text;
use ratodo::{agenda, capture, write};

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
    let cli = Cli::parse();
    let path = match cli.file {
        Some(p) => p,
        None => env_path().map_or_else(default_path, Ok)?,
    };

    match cli.command {
        Some(Command::Add { text }) => add(&path, &text.join(" "))?,
        Some(Command::Done { text }) => return done(&path, &text.join(" ")),
        Some(Command::List(args)) => list(&path, &args)?,
        Some(Command::Status { json }) => return status(&path, json),
        // The TUI arrives in step 4; until then the bare command lists.
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

fn add(path: &Path, input: &str) -> Result<()> {
    let today = Local::now().date_naive();
    let task = capture::capture(input, today);
    let summary = text::added(&task, today);

    let loaded = write::load(path)?;
    let mut doc = loaded.doc;
    doc.push_task(task);
    write::save(path, &doc, loaded.mtime, &backup_dir()?)?;

    println!("{summary}");
    Ok(())
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
    println!("{summary}");
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

    if args.porcelain {
        // Nothing on stderr either: a machine is reading, and an empty result is
        // already the answer.
        for task in groups.iter().flat_map(|g| &g.tasks) {
            println!("{}", text::porcelain_line(task));
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
            println!("\n{}", text::plain(title));
        }
        for task in &group.tasks {
            println!("{}", text::list_line(task, today));
        }
    }

    // Counted over what was shown, not over the file: a summary that disagrees
    // with the list above it is worse than no summary.
    println!("\n{}", text::status_line(agenda::Counts::of(&tasks, today)));

    Ok(())
}

/// Exits non-zero when something is overdue, which is the whole reason
/// `ratodo status || notify-send "$(ratodo status)"` needs no extra flag.
fn status(path: &Path, json: bool) -> Result<ExitCode> {
    let doc = write::load(path)?.doc;
    let today = Local::now().date_naive();
    let tasks: Vec<_> = doc.tasks().cloned().collect();
    let counts = agenda::Counts::of(&tasks, today);

    println!(
        "{}",
        if json {
            text::status_json(counts)
        } else {
            text::status_line(counts)
        }
    );

    Ok(if counts.overdue > 0 {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    })
}
