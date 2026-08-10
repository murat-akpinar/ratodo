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

fn run(args: &[&str]) -> Output {
    Command::new(BIN)
        .args(args)
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

    let after = fs::read_to_string(&path).unwrap();
    assert_eq!(after, format!("{original}- [ ] second\n"));
    assert_eq!(
        fs::read_to_string(path.with_file_name("todo.md.bak")).unwrap(),
        original
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
        "## Work\n- [ ] first @2026-12-01 #ops !high\n- [x] second\n\n## Home\n- [ ] third\n",
    )
    .unwrap();

    let out = stdout_of(&path, &["list"]);

    assert!(out.contains("\nWork\n"), "{out}");
    assert!(out.contains("\nHome\n"), "{out}");
    assert!(out.contains("[ ] first  2026-12-01  #ops  !high"), "{out}");
    assert!(out.contains("[x] second"), "{out}");
    assert!(out.contains("[ ] third"), "{out}");
    assert!(out.contains("2 open · "), "{out}");
    assert!(out.trim_end().ends_with("overdue"), "{out}");
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

#[test]
fn an_empty_list_says_where_the_file_is() {
    let dir = TempDir::new("empty");
    let path = dir.file("todo.md");

    let out = stdout_of(&path, &["list"]);
    assert!(out.contains("nothing here yet"), "{out}");
    assert!(out.contains(path.to_str().unwrap()), "{out}");
    assert!(!path.exists(), "listing an absent file must not create it");
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
