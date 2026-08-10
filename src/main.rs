//! Subcommands and terminal setup. See docs/cli.md.

use std::path::PathBuf;

use anyhow::{Context, Result};
use chrono::{Local, NaiveDate};
use clap::{Parser, Subcommand};

use ratodo::capture;
use ratodo::model::{Due, Task};
use ratodo::write;

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
        None => default_path()?,
    };

    match cli.command {
        Some(Command::Add { text }) => add(&path, &text.join(" ")),
        // The TUI arrives in step 4; until then the bare command lists.
        Some(Command::List) | None => list(&path),
    }
}

fn default_path() -> Result<PathBuf> {
    let dirs = directories::ProjectDirs::from("", "", "ratodo")
        .context("could not work out where ~/.config is")?;
    Ok(dirs.config_dir().join("todo.md"))
}

fn add(path: &std::path::Path, text: &str) -> Result<()> {
    let today = Local::now().date_naive();
    let task = capture::capture(text, today);
    let summary = describe(&task, today);

    let loaded = write::load(path)?;
    let mut doc = loaded.doc;
    doc.push_task(task);
    write::save(path, &doc, loaded.mtime)?;

    println!("{summary}");
    Ok(())
}

fn list(path: &std::path::Path) -> Result<()> {
    let doc = write::load(path)?.doc;
    let today = Local::now().date_naive();

    let mut section = None;
    let mut open = 0;
    let mut overdue = 0;

    for task in doc.tasks() {
        if task.section != section {
            section = task.section.clone();
            println!("\n{}", section.as_deref().unwrap_or("(no section)"));
        }

        let late = !task.done && task.due.is_some_and(|d| d.date < today);
        let mark = if task.done {
            "[x]"
        } else if late {
            "[!]"
        } else {
            "[ ]"
        };

        if !task.done {
            open += 1;
        }
        if late {
            overdue += 1;
        }

        let mut line = format!("  {mark} {}", task.title);
        if let Some(due) = task.due {
            line.push_str(&format!(
                "  {}",
                due.to_file_string().trim_start_matches('@')
            ));
        }
        for tag in &task.tags {
            line.push_str(&format!("  #{tag}"));
        }
        if let Some(p) = task.priority {
            line.push_str(&format!("  {}", p.as_str()));
        }
        println!("{line}");
    }

    if doc.task_count() == 0 {
        println!("nothing here yet — try: ratodo add \"buy milk @tomorrow #home\"");
        println!("file: {}", path.display());
    } else {
        println!("\n{open} open · {overdue} overdue");
    }

    Ok(())
}

fn describe(task: &Task, today: NaiveDate) -> String {
    let mut parts = vec![format!("added: {}", task.title)];
    if let Some(due) = task.due {
        parts.push(format!("due {}", relative(due, today)));
    }
    for tag in &task.tags {
        parts.push(format!("#{tag}"));
    }
    if let Some(p) = task.priority {
        parts.push(p.as_str().to_string());
    }
    parts.join("  ·  ")
}

fn relative(due: Due, today: NaiveDate) -> String {
    let days = (due.date - today).num_days();
    let when = match days {
        0 => "today".to_string(),
        1 => "tomorrow".to_string(),
        2..=6 => due.date.format("%A").to_string(),
        _ => due.date.format("%Y-%m-%d").to_string(),
    };
    let stamp = match due.time {
        Some(t) => format!("{} {}", due.date.format("%Y-%m-%d"), t.format("%H:%M")),
        None => due.date.format("%Y-%m-%d").to_string(),
    };
    if (0..=6).contains(&days) {
        format!("{when} ({stamp})")
    } else {
        stamp
    }
}
