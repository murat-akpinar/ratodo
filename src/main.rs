//! Subcommands and terminal setup. See docs/cli.md.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::Local;
use clap::{Parser, Subcommand};

use ratodo::text;
use ratodo::{capture, write};

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
    /// Print the list
    List,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let path = match cli.file {
        Some(p) => p,
        None => env_path().map_or_else(default_path, Ok)?,
    };

    match cli.command {
        Some(Command::Add { text }) => add(&path, &text.join(" ")),
        // The TUI arrives in step 4; until then the bare command lists.
        Some(Command::List) | None => list(&path),
    }
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

fn list(path: &Path) -> Result<()> {
    let doc = write::load(path)?.doc;
    let today = Local::now().date_naive();

    // stderr, not stdout: `ratodo list | wc -l` has to count tasks and nothing else.
    if doc.task_count() == 0 {
        eprintln!("nothing here yet — try: ratodo add 'buy milk @tomorrow #home'");
        eprintln!("file: {}", path.display());
        return Ok(());
    }

    // Starting at None means a file with no headings at all prints no heading,
    // rather than a "(no section)" nobody asked for.
    let mut section = None;
    for task in doc.tasks() {
        if task.section != section {
            section = task.section.clone();
            let name = section.as_deref().unwrap_or("(no section)");
            println!("\n{}", text::plain(name));
        }
        println!("{}", text::list_line(task, today));
    }

    let open = doc.tasks().filter(|t| !t.done).count();
    let overdue = doc.tasks().filter(|t| t.is_overdue(today)).count();
    println!("\n{open} open · {overdue} overdue");

    Ok(())
}
