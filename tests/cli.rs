//! The binary, driven the way a user drives it. docs/cli.md.
//!
//! Deliberately free of anything that depends on today's date: a test that
//! passes this week and fails next week is just another way for a suite to
//! mislead you. Date phrasing is tested in `text.rs`, where today is injected.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const BIN: &str = env!("CARGO_BIN_EXE_ratodo");

struct TempDir(PathBuf);

impl TempDir {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("ratodo-cli-{}-{tag}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        TempDir(dir)
    }

    fn file(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// Every invocation gets its XDG directories pointed at /tmp, so a test run can
/// never write a backup into the real `~/.local/state`.
fn run(args: &[&str]) -> Output {
    let scratch = std::env::temp_dir().join(format!("ratodo-cli-xdg-{}", std::process::id()));
    Command::new(BIN)
        .args(args)
        .env("XDG_STATE_HOME", scratch.join("state"))
        .env("XDG_CACHE_HOME", scratch.join("cache"))
        .output()
        .expect("running the binary")
}

fn stdout_of(path: &Path, args: &[&str]) -> String {
    let mut full = vec!["--file", path.to_str().unwrap()];
    full.extend_from_slice(args);
    let out = run(&full);
    assert!(
        out.status.success(),
        "{args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).expect("stdout is utf-8")
}

#[test]
fn add_creates_the_file_and_reports_one_line() {
    let dir = TempDir::new("add");
    let path = dir.file("todo.md");

    let out = stdout_of(&path, &["add", "buy milk"]);
    assert_eq!(out, "added: buy milk\n");
    assert_eq!(fs::read_to_string(&path).unwrap(), "- [ ] buy milk\n");
}

#[test]
fn add_appends_without_touching_what_is_already_there() {
    let dir = TempDir::new("append");
    let path = dir.file("todo.md");
    let original = "# My list\n\n## Work\n- [ ] existing @2026-08-12 #ops\n\n> a note\n";
    fs::write(&path, original).unwrap();

    stdout_of(&path, &["add", "second"]);

    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        "# My list\n\n## Work\n- [ ] existing @2026-08-12 #ops\n- [ ] second\n\n> a note\n",
        "the task went below the note instead of into the section"
    );
}

/// The list is normally symlinked into a dotfiles repo, so anything we drop next
/// to it shows up in `git status` after every capture. Both halves matter: the
/// backup must be gone from here **and** present over there — checking only the
/// first would pass just as well if the backup stopped being written at all.
#[test]
fn the_backup_lands_in_the_state_directory_and_nowhere_near_the_list() {
    let dir = TempDir::new("clean");
    let path = dir.file("todo.md");
    let state = dir.file("state");
    fs::write(&path, "- [ ] existing\n").unwrap();

    let out = Command::new(BIN)
        .args(["--file", path.to_str().unwrap(), "add", "second"])
        .env("XDG_STATE_HOME", &state)
        .output()
        .expect("running the binary");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let mut beside: Vec<String> = fs::read_dir(&dir.0)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    beside.sort();
    assert_eq!(beside, ["state", "todo.md"], "capture left files behind");

    let backups: Vec<PathBuf> = fs::read_dir(state.join("ratodo"))
        .expect("no backup directory was created")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .collect();
    assert_eq!(backups.len(), 1, "expected one backup, got {backups:?}");
    assert_eq!(
        fs::read_to_string(&backups[0]).unwrap(),
        "- [ ] existing\n",
        "the backup does not hold the pre-write list"
    );
}

#[test]
fn an_iso_date_reaches_the_file_unchanged() {
    let dir = TempDir::new("iso");
    let path = dir.file("todo.md");

    stdout_of(&path, &["add", "renew the domain @2026-12-01 #admin !low"]);
    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        "- [ ] renew the domain @2026-12-01 #admin !low\n"
    );
}

#[test]
fn shorthand_is_resolved_before_it_is_written() {
    let dir = TempDir::new("shorthand");
    let path = dir.file("todo.md");

    stdout_of(&path, &["add", "call the bank @tomorrow"]);

    let line = fs::read_to_string(&path).unwrap();
    assert!(
        !line.contains("@tomorrow"),
        "shorthand reached the file: {line}"
    );
    let date = line
        .split('@')
        .nth(1)
        .expect("a date was written")
        .trim_end();
    assert_eq!(date.len(), 10, "expected an ISO date, got {date:?}");
    assert!(date.chars().enumerate().all(|(i, c)| match i {
        4 | 7 => c == '-',
        _ => c.is_ascii_digit(),
    }));
}

#[test]
fn list_shows_sections_titles_and_a_summary() {
    let dir = TempDir::new("list");
    let path = dir.file("todo.md");
    fs::write(
        &path,
        "## Work\n- [ ] first #ops !high\n- [x] second\n\n## Home\n- [ ] third\n",
    )
    .unwrap();

    let out = stdout_of(&path, &["list"]);

    assert!(out.contains("\nWork\n"), "{out}");
    assert!(out.contains("\nHome\n"), "{out}");
    assert!(out.contains("[ ] first  #ops  !high"), "{out}");
    assert!(out.contains("[x] second"), "{out}");
    assert!(out.contains("[ ] third"), "{out}");
    assert!(out.contains("2 open · "), "{out}");
    assert!(out.trim_end().ends_with("overdue"), "{out}");
}

/// Only the two groups whose answer cannot change with the calendar are asserted
/// here; TODAY and THIS WEEK need `today` injected and are covered in
/// `agenda.rs`, where it is.
#[test]
fn dated_tasks_are_grouped_ahead_of_the_undated_ones() {
    let dir = TempDir::new("groups");
    let path = dir.file("todo.md");
    fs::write(
        &path,
        "## Work\n- [ ] someday @2099-01-01\n- [ ] no date\n- [ ] ancient @2020-01-01\n",
    )
    .unwrap();

    let out = stdout_of(&path, &["list"]);
    let headings: Vec<&str> = out
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with(' ') && !l.contains('·'))
        .collect();
    assert_eq!(headings, ["OVERDUE", "LATER", "Work"], "{out}");
    assert!(out.contains("1 overdue"), "{out}");
}

#[test]
fn a_file_with_no_headings_gets_no_headings() {
    let dir = TempDir::new("nosection");
    let path = dir.file("todo.md");
    fs::write(&path, "- [ ] one\n- [ ] two\n").unwrap();

    let out = stdout_of(&path, &["list"]);
    assert!(!out.contains("(no section)"), "{out}");
    assert!(out.contains("[ ] one"), "{out}");
}

#[test]
fn the_bare_command_lists() {
    let dir = TempDir::new("bare");
    let path = dir.file("todo.md");
    fs::write(&path, "- [ ] only one\n").unwrap();

    assert_eq!(stdout_of(&path, &[]), stdout_of(&path, &["list"]));
}

/// The message is help, not data: it goes to stderr so that `ratodo list | wc -l`
/// counts tasks and nothing else.
#[test]
fn an_empty_list_says_where_the_file_is_on_stderr() {
    let dir = TempDir::new("empty");
    let path = dir.file("todo.md");

    let out = run(&["--file", path.to_str().unwrap(), "list"]);
    assert!(out.status.success());
    assert!(
        out.stdout.is_empty(),
        "stdout was not empty: {:?}",
        out.stdout
    );

    let err = String::from_utf8(out.stderr).unwrap();
    assert!(err.contains("nothing here yet"), "{err}");
    assert!(err.contains(path.to_str().unwrap()), "{err}");
    assert!(!path.exists(), "listing an absent file must not create it");
}

#[test]
fn ratodo_file_is_used_when_there_is_no_flag() {
    let dir = TempDir::new("envvar");
    let listed = dir.file("from-the-env.md");
    let flagged = dir.file("from-the-flag.md");
    let scratch = std::env::temp_dir().join(format!("ratodo-cli-xdg-{}", std::process::id()));

    let with_env = |args: &[&str]| {
        let out = Command::new(BIN)
            .args(args)
            .env("RATODO_FILE", &listed)
            .env("XDG_CONFIG_HOME", dir.file("config"))
            .env("XDG_STATE_HOME", scratch.join("state"))
            .output()
            .expect("running the binary");
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
    };

    with_env(&["add", "from the environment"]);
    assert_eq!(
        fs::read_to_string(&listed).unwrap(),
        "- [ ] from the environment\n"
    );
    assert!(
        !dir.file("config").exists(),
        "the default path was used despite RATODO_FILE"
    );

    // --file still wins.
    with_env(&["--file", flagged.to_str().unwrap(), "add", "from the flag"]);
    assert_eq!(
        fs::read_to_string(&flagged).unwrap(),
        "- [ ] from the flag\n"
    );
    assert_eq!(
        fs::read_to_string(&listed).unwrap(),
        "- [ ] from the environment\n",
        "--file did not override RATODO_FILE"
    );
}

#[test]
fn control_characters_from_the_file_do_not_reach_the_terminal() {
    let dir = TempDir::new("escape");
    let path = dir.file("todo.md");
    fs::write(&path, "- [ ] innocent\x1b[2J title\n").unwrap();

    let out = stdout_of(&path, &["list"]);
    assert!(!out.contains('\x1b'), "an escape sequence was printed raw");
    assert!(out.contains('\u{fffd}'), "{out}");
}

/// The XDG deviation from docs/format.md, asserted rather than assumed: the
/// list lives under the *config* directory, because that is what ends up in
/// somebody's dotfiles.
#[test]
fn the_default_path_is_the_config_directory() {
    let dir = TempDir::new("xdg");
    let out = Command::new(BIN)
        .args(["add", "from the default path"])
        .env("XDG_CONFIG_HOME", &dir.0)
        .env("XDG_DATA_HOME", dir.file("data"))
        .output()
        .expect("running the binary");

    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let expected = dir.0.join("ratodo").join("todo.md");
    assert_eq!(
        fs::read_to_string(&expected).unwrap(),
        "- [ ] from the default path\n",
        "nothing was written to {}",
        expected.display()
    );
    assert!(
        !dir.file("data").exists(),
        "the list must not go under XDG_DATA_HOME"
    );
}

#[test]
fn add_with_no_text_is_an_error() {
    let dir = TempDir::new("noargs");
    let path = dir.file("todo.md");
    let out = run(&["--file", path.to_str().unwrap(), "add"]);

    assert!(!out.status.success(), "empty add should fail");
    assert!(!path.exists(), "a failed add must not create the file");
}

#[test]
fn unknown_arguments_fail_loudly() {
    let out = run(&["--nonsense"]);
    assert!(!out.status.success());
    assert!(!String::from_utf8_lossy(&out.stderr).is_empty());
}

#[test]
fn version_and_help_work() {
    let version = run(&["--version"]);
    assert!(version.status.success());
    assert!(String::from_utf8_lossy(&version.stdout).contains("ratodo"));

    let help = run(&["--help"]);
    assert!(help.status.success());
    let text = String::from_utf8_lossy(&help.stdout);
    assert!(text.contains("--file"), "{text}");
    assert!(text.contains("add"), "{text}");
}
