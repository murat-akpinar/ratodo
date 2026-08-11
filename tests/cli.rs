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
/// never write into the developer's own `~/.local`.
///
/// **All four, and `XDG_DATA_HOME` is not optional.** Every write regenerates
/// the calendar, so a command that runs without it rewrites the real
/// `~/.local/share/ratodo/todo.ics` — with the fixture's two tasks, or with
/// none. This was found by running `cargo test` and watching a live list's
/// calendar file go to 67 bytes.
fn run(args: &[&str]) -> Output {
    xdg(Command::new(BIN).args(args))
        .output()
        .expect("running the binary")
}

/// The scratch XDG environment, on whatever command is about to run — including
/// the `script`-wrapped ones, where the binary is a word inside a shell string
/// and inherits the environment all the same.
fn xdg(command: &mut Command) -> &mut Command {
    let scratch = std::env::temp_dir().join(format!("ratodo-cli-xdg-{}", std::process::id()));
    command
        .env("XDG_STATE_HOME", scratch.join("state"))
        .env("XDG_CACHE_HOME", scratch.join("cache"))
        .env("XDG_DATA_HOME", scratch.join("data"))
        .env("XDG_CONFIG_HOME", scratch.join("config"))
}

/// Today, as the tick stamps it. These tests run against the real clock, so the
/// expected line has to be built the same way the binary builds it.
fn stamp() -> String {
    format!(" ✓{}", chrono::Local::now().date_naive().format("%Y-%m-%d"))
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

    let out = xdg(&mut Command::new(BIN))
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

/// A list to filter, with nothing dated in it: the groups are then the file's
/// own headings whatever day the suite runs on.
fn filterable(dir: &TempDir) -> PathBuf {
    let path = dir.file("todo.md");
    fs::write(
        &path,
        "## Work\n- [ ] deploy #ops !high\n- [ ] invoice #admin\n\n\
         ## Home\n- [ ] plumber #home !low\n- [ ] bins #home #ops\n",
    )
    .unwrap();
    path
}

#[test]
fn tag_and_priority_narrow_the_list() {
    let dir = TempDir::new("filter");
    let path = filterable(&dir);

    let one = stdout_of(&path, &["list", "--tag", "ops"]);
    assert!(one.contains("deploy") && one.contains("bins"), "{one}");
    assert!(
        !one.contains("invoice") && !one.contains("plumber"),
        "{one}"
    );
    assert!(
        one.contains("2 open · "),
        "the summary counts what it showed"
    );

    let two = stdout_of(&path, &["list", "--tag", "admin", "--tag", "home"]);
    assert!(two.contains("invoice") && two.contains("plumber"), "{two}");
    assert!(
        !two.contains("deploy"),
        "repeated tags should widen, not narrow"
    );

    let high = stdout_of(&path, &["list", "--prio", "high"]);
    assert!(
        high.contains("deploy") && !high.contains("plumber"),
        "{high}"
    );

    let both = stdout_of(&path, &["list", "--tag", "home", "--prio", "low"]);
    assert!(both.contains("plumber") && !both.contains("bins"), "{both}");
}

#[test]
fn a_filter_that_matches_nothing_says_so_on_stderr_and_succeeds() {
    let dir = TempDir::new("nomatch");
    let path = filterable(&dir);

    let out = run(&["--file", path.to_str().unwrap(), "list", "--tag", "nope"]);
    assert!(out.status.success(), "an empty result is still an answer");
    assert!(out.stdout.is_empty(), "{:?}", out.stdout);
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("no task matches"),
        "{:?}",
        out.stderr
    );
}

#[test]
fn an_unknown_priority_is_rejected_before_anything_is_read() {
    let dir = TempDir::new("badprio");
    let path = dir.file("todo.md");

    let out = run(&["--file", path.to_str().unwrap(), "list", "--prio", "urgent"]);
    assert!(!out.status.success());
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("high, med or low"),
        "{:?}",
        out.stderr
    );
}

/// The contract behind `ratodo done "$(ratodo list --porcelain | fzf | cut -f3)"`:
/// every line has the same five fields whatever the task is missing.
#[test]
fn porcelain_is_five_tab_separated_fields_and_nothing_else() {
    let dir = TempDir::new("porcelain");
    let path = dir.file("todo.md");
    fs::write(
        &path,
        "## Work\n- [ ] deploy @2099-01-01 #ops !high\n- [x] bare\n",
    )
    .unwrap();

    let out = stdout_of(&path, &["list", "--porcelain"]);
    assert_eq!(
        out, "open\t2099-01-01\tdeploy\tops\thigh\ndone\t\tbare\t\t\n",
        "{out:?}"
    );

    let titles: Vec<&str> = out.lines().map(|l| l.split('\t').nth(2).unwrap()).collect();
    assert_eq!(
        titles,
        ["deploy", "bare"],
        "cut -f3 is the documented column"
    );
}

#[test]
fn porcelain_stays_silent_when_there_is_nothing_to_say() {
    let dir = TempDir::new("porcelain-empty");
    let path = dir.file("todo.md");

    let out = run(&["--file", path.to_str().unwrap(), "list", "--porcelain"]);
    assert!(out.status.success());
    assert!(out.stdout.is_empty(), "{:?}", out.stdout);
    assert!(
        out.stderr.is_empty(),
        "a machine is reading; the hint is noise: {:?}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn porcelain_honours_the_filters_too() {
    let dir = TempDir::new("porcelain-filter");
    let path = filterable(&dir);

    let out = stdout_of(&path, &["list", "--porcelain", "--tag", "ops"]);
    assert_eq!(out.lines().count(), 2, "{out:?}");
    assert!(out.lines().all(|l| l.split('\t').count() == 5), "{out:?}");
}

/// `ratodo list | head -3` is an ordinary thing to type. Rust's `println!`
/// panics when the reader goes away, so without a deliberate answer the user
/// gets a backtrace and exit 101 for doing nothing wrong.
#[test]
fn a_reader_that_stops_early_is_not_an_error() {
    let dir = TempDir::new("pipe");
    let path = dir.file("todo.md");
    let many: String = (0..500)
        .map(|i| format!("- [ ] task number {i}\n"))
        .collect();
    fs::write(&path, many).unwrap();

    for args in [vec!["list"], vec!["list", "--porcelain"], vec!["status"]] {
        let mut full = vec!["--file", path.to_str().unwrap()];
        full.extend_from_slice(&args);

        let mut child = xdg(&mut Command::new(BIN))
            .args(&full)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("spawning");
        // Closing the pipe unread is what `head` does once it has enough.
        drop(child.stdout.take());
        let out = child.wait_with_output().expect("waiting");

        assert!(
            out.status.success(),
            "{args:?} exited {:?}: {}",
            out.status.code(),
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(
            !String::from_utf8_lossy(&out.stderr).contains("panicked"),
            "{args:?} panicked: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
}

#[test]
fn done_ticks_the_one_match_and_changes_exactly_that_byte() {
    let dir = TempDir::new("done");
    let path = dir.file("todo.md");
    let before = "# My list\n\n## Work\n- [ ] pay the invoice @2026-08-12 #ops\n- [ ] call the bank\n\n> a note\n";
    fs::write(&path, before).unwrap();

    let out = stdout_of(&path, &["done", "invoice"]);
    assert_eq!(out, "done: pay the invoice\n");

    let after = fs::read_to_string(&path).unwrap();
    assert_eq!(
        after,
        before.replace(
            "- [ ] pay the invoice @2026-08-12 #ops",
            &format!("- [x] pay the invoice @2026-08-12 #ops{}", stamp())
        ),
        "something other than the one checkbox and its stamp moved"
    );
}

/// The trust break the whole round-trip guarantee exists to prevent: on an
/// ambiguous match the file must be byte-identical afterwards, and the exit code
/// must let a script notice.
#[test]
fn an_ambiguous_done_writes_nothing_at_all() {
    let dir = TempDir::new("ambiguous");
    let path = dir.file("todo.md");
    let before = "- [ ] write the report\n- [ ] send the report\n";
    fs::write(&path, before).unwrap();

    let out = run(&["--file", path.to_str().unwrap(), "done", "report"]);
    assert_eq!(out.status.code(), Some(2), "ambiguity is exit 2");
    assert!(out.stdout.is_empty(), "{:?}", out.stdout);
    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        before,
        "the file changed"
    );

    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("write the report"), "{err}");
    assert!(err.contains("send the report"), "{err}");
    assert!(err.contains("nothing was changed"), "{err}");

    let beside: Vec<String> = fs::read_dir(&dir.0)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(beside, ["todo.md"], "a refused write still took a backup");
}

#[test]
fn done_with_no_match_is_exit_2_and_says_so() {
    let dir = TempDir::new("nomatch-done");
    let path = dir.file("todo.md");
    fs::write(&path, "- [ ] pay the invoice\n").unwrap();

    let out = run(&["--file", path.to_str().unwrap(), "done", "nonsense"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("no open task matches 'nonsense'"),
        "{:?}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// A cancelled task is out of every count and out of `done`'s reach — it is off
/// the list, which is the whole point of it not simply being deleted.
#[test]
fn a_cancelled_task_is_neither_open_nor_overdue_nor_matchable() {
    let dir = TempDir::new("cancelled");
    let path = dir.file("todo.md");
    fs::write(
        &path,
        "- [ ] still wanted @2026-01-01\n- [-] decided against @2026-01-01 #ops\n",
    )
    .unwrap();

    // One open, one overdue — the cancelled one is in neither, though its date
    // is just as far in the past. Not `stdout_of`: `status` exits non-zero when
    // something is overdue, which is the documented behaviour.
    let status = run(&["--file", path.to_str().unwrap(), "status"]);
    assert_eq!(
        String::from_utf8_lossy(&status.stdout),
        "1 open · 1 overdue\n"
    );

    // It is on screen, with its own state word for a script to branch on.
    let porcelain = stdout_of(&path, &["list", "--porcelain"]);
    assert!(
        porcelain.contains("cancelled\t2026-01-01\tdecided against\tops\t"),
        "{porcelain:?}"
    );
    assert!(stdout_of(&path, &["list"]).contains("[-] decided against"));

    // And `done` will not tick it: there is nothing left to do to it.
    let out = run(&["--file", path.to_str().unwrap(), "done", "decided"]);
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("already done: decided against"),
        "{:?}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        "- [ ] still wanted @2026-01-01\n- [-] decided against @2026-01-01 #ops\n",
        "nothing was written"
    );
}

/// The `.ics` is for work still to do. A cancelled task is not that, and neither
/// is a finished one.
#[test]
fn only_open_tasks_reach_the_calendar() {
    let dir = TempDir::new("ics-states");
    let path = dir.file("todo.md");
    fs::write(
        &path,
        "- [ ] open @2026-09-01\n- [x] finished @2026-09-02\n- [-] cancelled @2026-09-03\n",
    )
    .unwrap();

    let data = dir.file("data");
    let out = xdg(&mut Command::new(BIN))
        .args(["--file", path.to_str().unwrap(), "sync"])
        .env("XDG_DATA_HOME", &data)
        .env("XDG_STATE_HOME", dir.file("state"))
        .output()
        .expect("running the binary");
    assert!(
        out.status.success(),
        "{:?}",
        String::from_utf8_lossy(&out.stderr)
    );

    let ics = fs::read_to_string(data.join("ratodo").join("todo.ics")).expect("the calendar file");

    assert_eq!(ics.matches("BEGIN:VTODO").count(), 1, "{ics}");
    assert!(ics.contains("open"), "{ics}");
    assert!(!ics.contains("cancelled"), "{ics}");
    assert!(!ics.contains("finished"), "{ics}");
}

/// "no task matches" would be a lie the user cannot act on, and running it twice
/// is the ordinary way to find out.
#[test]
fn done_twice_says_it_is_already_done_and_succeeds() {
    let dir = TempDir::new("twice");
    let path = dir.file("todo.md");
    fs::write(&path, "- [ ] pay the invoice\n").unwrap();

    stdout_of(&path, &["done", "invoice"]);
    let again = run(&["--file", path.to_str().unwrap(), "done", "invoice"]);

    assert!(again.status.success(), "the desired state already holds");
    assert!(
        String::from_utf8_lossy(&again.stderr).contains("already done: pay the invoice"),
        "{:?}",
        String::from_utf8_lossy(&again.stderr)
    );
    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        format!("- [x] pay the invoice{}\n", stamp()),
        "the second `done` changed nothing, stamp included"
    );
}

/// `ratodo status || notify-send "$(ratodo status)"` is the documented use, and
/// it only works if the exit code carries the overdue flag.
#[test]
fn status_exits_non_zero_only_when_something_is_overdue() {
    let dir = TempDir::new("status");
    let path = dir.file("todo.md");

    fs::write(&path, "- [ ] someday @2099-01-01\n- [x] finished\n").unwrap();
    let quiet = run(&["--file", path.to_str().unwrap(), "status"]);
    assert!(quiet.status.success(), "nothing is late here");
    assert_eq!(
        String::from_utf8_lossy(&quiet.stdout),
        "1 open · 0 overdue\n"
    );

    fs::write(
        &path,
        "- [ ] someday @2099-01-01\n- [ ] ancient @2020-01-01\n",
    )
    .unwrap();
    let late = run(&["--file", path.to_str().unwrap(), "status"]);
    assert_eq!(late.status.code(), Some(1), "an overdue task must exit 1");
    assert_eq!(
        String::from_utf8_lossy(&late.stdout),
        "2 open · 1 overdue\n"
    );
}

/// A missing file is a quiet zero, not an error: the bar runs this every sixty
/// seconds from the moment it starts, which may be before the list exists.
#[test]
fn status_on_a_list_that_is_not_there_yet() {
    let dir = TempDir::new("status-empty");
    let path = dir.file("todo.md");

    let out = run(&["--file", path.to_str().unwrap(), "status"]);
    assert!(out.status.success());
    assert_eq!(String::from_utf8_lossy(&out.stdout), "0 open · 0 overdue\n");
    assert!(!path.exists(), "status must not create the list");
}

#[test]
fn status_json_is_one_line_a_bar_can_parse() {
    let dir = TempDir::new("status-json");
    let path = dir.file("todo.md");
    fs::write(
        &path,
        "- [ ] someday @2099-01-01\n- [ ] ancient @2020-01-01\n",
    )
    .unwrap();

    let out = run(&["--file", path.to_str().unwrap(), "status", "--json"]);
    let text = String::from_utf8_lossy(&out.stdout);
    assert_eq!(text.lines().count(), 1, "{text}");
    assert_eq!(
        text.trim_end(),
        r#"{"text":"2 ○ 1!","tooltip":"2 open, 1 overdue","class":"overdue"}"#
    );
    assert_eq!(out.status.code(), Some(1), "--json keeps the exit code");
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
        let out = xdg(&mut Command::new(BIN))
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
    let out = xdg(&mut Command::new(BIN))
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
    // The derived `todo.ics` does live under XDG_DATA_HOME, and should. What must
    // never be there is the list itself — that is the deviation this test is about.
    assert!(
        !dir.file("data").join("ratodo").join("todo.md").exists(),
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

/// Not every failure is a broken pipe. If the answer to "could not read the
/// list" were also a quiet exit 0, a cron job would report success forever.
#[test]
fn a_list_that_cannot_be_read_fails_loudly() {
    let dir = TempDir::new("unreadable");
    let out = run(&["--file", dir.0.to_str().unwrap(), "list"]);

    assert!(!out.status.success(), "a directory is not a list");
    assert!(
        !String::from_utf8_lossy(&out.stderr).is_empty(),
        "it failed without saying why"
    );
}

/// The one branch that only exists on a terminal, so it takes a terminal to
/// test: `script` lends us a pty. What is asserted is the alternate screen being
/// entered **and left** — the second half is invariant 5, and a TUI that forgets
/// it hands back a wrecked shell.
#[cfg(target_os = "linux")]
#[test]
fn the_bare_command_opens_a_screen_on_a_terminal_and_gives_it_back() {
    let dir = TempDir::new("pty");
    let path = dir.file("todo.md");
    fs::write(&path, "- [ ] pay the invoice\n").unwrap();

    // `timeout` so a TUI that stops answering `q` fails the suite instead of
    // hanging it.
    let out = xdg(&mut Command::new("timeout"))
        .args([
            "10",
            "script",
            "-qec",
            &format!("stty rows 20 cols 60; {BIN} --file {}", path.display()),
            "/dev/null",
        ])
        .env("XDG_STATE_HOME", dir.file("state"))
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child.stdin.take().expect("stdin").write_all(b"q")?;
            child.wait_with_output()
        })
        .expect("script(1) is needed for this test — it is in util-linux");

    assert!(out.status.success(), "the TUI did not exit cleanly");
    let screen = String::from_utf8_lossy(&out.stdout);
    assert!(
        screen.contains("\x1b[?1049h"),
        "no alternate screen was entered, so no TUI opened: {screen:?}"
    );
    assert!(
        screen.contains("\x1b[?1049l"),
        "the alternate screen was never left: {screen:?}"
    );
    // One word, not the phrase: ratatui writes each run of cells with a cursor
    // move between them, so "pay the invoice" never appears contiguously.
    assert!(
        screen.contains("invoice"),
        "the list never reached the screen: {screen:?}"
    );
}

/// Replays what a terminal would have done with this byte stream and returns
/// the screen it would be showing.
///
/// Searching the raw stream instead does not work for anything that gets undone:
/// ratatui redraws only the cells that changed, so a closed overlay is still in
/// the bytes from when it was open. This is the difference between "was ever
/// drawn" and "is on screen".
#[cfg(target_os = "linux")]
fn replay(stream: &str, width: usize, height: usize) -> Vec<String> {
    let mut grid = vec![vec![' '; width]; height];
    let (mut row, mut col) = (0usize, 0usize);
    let mut chars = stream.chars().peekable();

    while let Some(c) = chars.next() {
        if c != '\x1b' {
            match c {
                '\r' => col = 0,
                '\n' => row += 1,
                _ => {
                    if row < height && col < width {
                        grid[row][col] = c;
                    }
                    col += 1;
                }
            }
            continue;
        }

        if chars.peek() != Some(&'[') {
            chars.next();
            continue;
        }
        chars.next();

        let mut params = String::new();
        let mut final_byte = ' ';
        for c in chars.by_ref() {
            if c.is_ascii_alphabetic() {
                final_byte = c;
                break;
            }
            params.push(c);
        }

        match final_byte {
            'H' => {
                let mut parts = params.split(';');
                row = parts.next().and_then(|p| p.parse().ok()).unwrap_or(1) - 1;
                col = parts.next().and_then(|p| p.parse().ok()).unwrap_or(1) - 1;
            }
            'J' => grid = vec![vec![' '; width]; height],
            _ => {}
        }
    }

    grid.into_iter().map(|r| r.into_iter().collect()).collect()
}

/// `e` hands the terminal over and takes it back. This is the whole reason the
/// event loop polls instead of parking a thread in `read`: a second reader would
/// be eating the editor's keystrokes — see docs/decisions.md#reversed.
///
/// The fake editor writes to the screen as well as to the file, so the test can
/// see that it really had the terminal rather than merely being spawned.
#[cfg(target_os = "linux")]
#[test]
fn the_editor_key_hands_the_terminal_over_and_takes_it_back() {
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;

    let dir = TempDir::new("editor");
    let path = dir.file("todo.md");
    fs::write(&path, "- [ ] pay the invoice\n").unwrap();

    let editor = dir.file("fake-editor");
    fs::write(
        &editor,
        "#!/bin/sh\nprintf 'THE EDITOR HAD THE SCREEN'\nprintf -- '- [ ] written in the editor\\n' >> \"$1\"\n",
    )
    .unwrap();
    fs::set_permissions(&editor, fs::Permissions::from_mode(0o755)).unwrap();

    let mut child = xdg(&mut Command::new("timeout"))
        .args([
            "20",
            "script",
            "-qec",
            &format!("stty rows 12 cols 56; {BIN} --file {}", path.display()),
            "/dev/null",
        ])
        .env("EDITOR", &editor)
        .env_remove("VISUAL")
        .env("XDG_STATE_HOME", dir.file("state"))
        .env("XDG_DATA_HOME", dir.file("data"))
        .env("XDG_CONFIG_HOME", dir.file("config"))
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("script(1) is needed for this test — it is in util-linux");

    let mut stdin = child.stdin.take().expect("stdin");
    stdin.write_all(b"e").unwrap();
    std::thread::sleep(std::time::Duration::from_millis(600));
    stdin.write_all(b"q").unwrap();
    drop(stdin);

    let out = child.wait_with_output().expect("waiting");
    assert!(
        out.status.success(),
        "ratodo did not survive the round trip (exit {:?})",
        out.status.code()
    );

    let raw = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(
        raw.contains("THE EDITOR HAD THE SCREEN"),
        "the editor never got the terminal: {raw:?}"
    );
    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        "- [ ] pay the invoice\n- [ ] written in the editor\n"
    );

    // And the list that came back has what the editor wrote in it. The whole
    // screen is repainted, so this is the final state and not a leftover frame.
    let screen = replay(&raw, 56, 12).join("\n");
    assert!(
        screen.contains("written in the editor"),
        "the screen did not pick the change up: {screen}"
    );
    assert!(screen.contains("$EDITOR"), "{screen}");
}

/// `?` is drawn well and tested well one level down, and until this existed
/// nothing checked that the key was wired to it at all — a mutant turning the
/// toggle into a no-op went unnoticed.
#[cfg(target_os = "linux")]
#[test]
fn the_help_key_opens_the_overlay_and_esc_puts_it_away() {
    use std::io::Write;

    let dir = TempDir::new("help");
    let path = dir.file("todo.md");
    fs::write(&path, "- [ ] pay the invoice\n").unwrap();

    let after = |keys: &[u8]| {
        let mut child = xdg(&mut Command::new("timeout"))
            .args([
                "20",
                "script",
                "-qec",
                &format!("stty rows 18 cols 60; {BIN} --file {}", path.display()),
                "/dev/null",
            ])
            .env("XDG_STATE_HOME", dir.file("state"))
            .env("XDG_CONFIG_HOME", dir.file("config"))
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .expect("script(1) is needed for this test — it is in util-linux");

        let mut stdin = child.stdin.take().expect("stdin");
        for key in keys {
            stdin.write_all(&[*key]).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(120));
        }
        stdin.write_all(b"q").unwrap();
        drop(stdin);
        let raw = String::from_utf8_lossy(&child.wait_with_output().expect("waiting").stdout)
            .into_owned();
        replay(&raw, 60, 18).join("\n")
    };

    assert!(
        !after(b"").contains("toggle done"),
        "the overlay was up without being asked for"
    );
    assert!(
        after(b"?").contains("toggle done"),
        "? did not open the overlay"
    );
    // `esc` closes it — and, as ever, does not quit.
    assert!(
        !after(b"?\x1b").contains("toggle done"),
        "esc did not put the overlay away"
    );
    assert!(
        !after(b"??").contains("toggle done"),
        "a second ? did not put the overlay away"
    );
}

/// The second mode, through a terminal — the only place the whole of it is
/// real: `a` opens the input, the preview resolves the shorthand while it is
/// still being typed, `⏎` writes the file, and `ctrl-c` costs a sentence rather
/// than the session.
#[cfg(target_os = "linux")]
#[test]
fn the_input_mode_captures_a_task_and_ctrl_c_only_cancels_it() {
    use std::io::Write;

    let dir = TempDir::new("input");
    let path = dir.file("todo.md");

    let run = |keys: &[u8]| {
        fs::write(&path, "- [ ] pay the invoice\n").unwrap();
        let mut child = xdg(&mut Command::new("timeout"))
            .args([
                "20",
                "script",
                "-qec",
                &format!("stty rows 16 cols 64; {BIN} --file {}", path.display()),
                "/dev/null",
            ])
            .env("LC_ALL", "C.UTF-8")
            .env("LANG", "C.UTF-8")
            .env("XDG_STATE_HOME", dir.file("state"))
            .env("XDG_DATA_HOME", dir.file("data"))
            .env("XDG_CONFIG_HOME", dir.file("config"))
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .expect("script(1) is needed for this test — it is in util-linux");

        // Every sequence ends with its own way out: `q` is a letter while the
        // input is open, so there is no key that always quits.
        let mut stdin = child.stdin.take().expect("stdin");
        for key in keys {
            stdin.write_all(&[*key]).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(60));
        }
        drop(stdin);

        let out = child.wait_with_output().expect("waiting");
        assert!(out.status.success(), "ratodo did not exit cleanly");
        let raw = String::from_utf8_lossy(&out.stdout).into_owned();
        (
            raw.clone(),
            replay(&raw, 64, 16).join("\n"),
            fs::read_to_string(&path).unwrap(),
        )
    };

    // Mid-sentence: `a` opened a field and `@tomorrow` resolved to a date under
    // it, while the file was still untouched.
    //
    // Read from the stream, not the replayed screen. `esc` is the only way back
    // out of the input and it repaints the two rows in question, so "was drawn"
    // is the honest question here — and each keystroke redraws one cell, so what
    // is contiguous in the stream is what a frame painted in one go.
    let (raw, _, file) = run(b"amilk @tomorrow #home\x1bq");
    // The label and the caret are two spans and no longer contiguous in the
    // stream: the label is bold, so the reset sits between them.
    assert!(raw.contains(" ADD"), "no input field opened: {raw:?}");
    assert!(raw.contains("▏"), "no input field opened: {raw:?}");
    assert!(
        raw.contains("due tomorrow ("),
        "the preview never resolved the shorthand: {raw:?}"
    );
    // The way out is on the bottom line under the box, painted over the hint bar
    // that was already there — so the stream repaints only the cells that differ
    // and even a whole word is not reliably contiguous in it: whichever letters
    // happen to match the bar underneath are simply left alone. What the line
    // says, exactly, is pinned by `the_input_screen_exactly` in `ui.rs`, where a
    // buffer can be read directly. Here the question is only whether it was
    // drawn at all.
    assert!(raw.contains("save"), "no way out was drawn: {raw:?}");
    assert!(raw.contains("canc"), "no way out was drawn: {raw:?}");
    assert_eq!(
        file, "- [ ] pay the invoice\n",
        "nothing is written until ⏎"
    );

    // `⏎` saves, and what lands in the file is the resolved date, not `@tomorrow`.
    let (_, screen, file) = run(b"amilk @tomorrow #home\rq");
    let added = file
        .lines()
        .nth(1)
        .unwrap_or_else(|| panic!("nothing was added: {file:?}"));
    assert!(
        added.starts_with("- [ ] milk @20") && added.ends_with(" #home"),
        "{added:?}"
    );
    assert!(screen.contains("added: milk"), "{screen}");
    assert!(screen.contains("u undo"), "{screen}");

    // `ctrl-c` in the input cancels it. The `X` afterwards is what proves the
    // session is still there and back in the list: if ctrl-c had quit, nothing
    // would have been deleted.
    let (_, _, file) = run(b"amilk @tomorrow\x03Xq");
    assert_eq!(
        file, "",
        "ctrl-c took the session down instead of the sentence, or wrote the line"
    );
}

/// The locale reaches the screen. Asserted through a terminal because the
/// wiring between `$LC_ALL` and the glyphs is the part a unit test cannot see.
#[cfg(target_os = "linux")]
#[test]
fn the_locale_picks_the_glyphs_the_screen_is_drawn_with() {
    use std::io::Write;

    let dir = TempDir::new("glyphs");
    let path = dir.file("todo.md");
    fs::write(&path, "- [ ] pay the invoice\n").unwrap();

    let screen_under = |locale: &str| {
        let mut child = xdg(&mut Command::new("timeout"))
            .args([
                "20",
                "script",
                "-qec",
                &format!("stty rows 12 cols 50; {BIN} --file {}", path.display()),
                "/dev/null",
            ])
            .env("LC_ALL", locale)
            .env("LANG", locale)
            .env("XDG_STATE_HOME", dir.file("state"))
            .env("XDG_CONFIG_HOME", dir.file("config"))
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .expect("script(1) is needed for this test — it is in util-linux");
        child.stdin.take().expect("stdin").write_all(b"q").unwrap();
        let out = child.wait_with_output().expect("waiting");
        String::from_utf8_lossy(&out.stdout).into_owned()
    };

    let utf8 = screen_under("en_US.UTF-8");
    assert!(
        utf8.contains('○'),
        "no unicode mark in a UTF-8 locale: {utf8:?}"
    );
    assert!(utf8.contains('▌'), "{utf8:?}");

    let plain = screen_under("C");
    assert!(
        plain.contains("[ ]"),
        "no ascii mark in the C locale: {plain:?}"
    );
    assert!(!plain.contains('○'), "a unicode mark survived: {plain:?}");
}

/// Round-trip fidelity through the TUI, which is where it is easiest to lose:
/// `spc` on a task has to change the one checkbox byte and nothing else in a
/// file full of things ratodo does not understand.
#[cfg(target_os = "linux")]
#[test]
fn ticking_a_task_on_the_screen_changes_one_byte_of_the_file() {
    use std::io::Write;

    let dir = TempDir::new("toggle");
    let path = dir.file("todo.md");
    let before = "# My list\n\n## Work\n- [ ]   oddly   spaced  @2026-08-09 #ops\n\
                  - [ ] second\n\n| a | table |\n|---|---|\n\n> a note\n";
    fs::write(&path, before).unwrap();

    let mut child = xdg(&mut Command::new("timeout"))
        .args([
            "20",
            "script",
            "-qec",
            &format!("stty rows 14 cols 60; {BIN} --file {}", path.display()),
            "/dev/null",
        ])
        .env("XDG_STATE_HOME", dir.file("state"))
        .env("XDG_DATA_HOME", dir.file("data"))
        .env("XDG_CONFIG_HOME", dir.file("config"))
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("script(1) is needed for this test — it is in util-linux");

    std::thread::sleep(std::time::Duration::from_millis(500));
    let mut stdin = child.stdin.take().expect("stdin");
    stdin.write_all(b" ").unwrap();
    std::thread::sleep(std::time::Duration::from_millis(500));
    stdin.write_all(b"q").unwrap();
    drop(stdin);

    let out = child.wait_with_output().expect("waiting");
    assert!(out.status.success());

    let after = fs::read_to_string(&path).unwrap();
    assert_eq!(
        after,
        before.replacen(
            "- [ ]   oddly   spaced  @2026-08-09 #ops",
            &format!("- [x]   oddly   spaced  @2026-08-09 #ops{}", stamp()),
            1
        ),
        "something other than the one checkbox and its stamp moved"
    );

    // And the screen said so, in place — the ticked row is still the first one
    // under OVERDUE rather than having jumped to the end of its group.
    let screen = String::from_utf8_lossy(&out.stdout);
    assert!(screen.contains("done:"), "{screen:?}");
}

/// The promise in docs/architecture.md#concurrent-editing: an edit from vim,
/// `git pull` or `ratodo add` next door reaches the open screen on its own.
/// Timing, so it needs a real process and real waiting.
#[cfg(target_os = "linux")]
#[test]
fn an_edit_from_outside_reaches_the_open_screen() {
    use std::io::Write;

    let dir = TempDir::new("watch");
    let path = dir.file("todo.md");
    fs::write(&path, "- [ ] the original task\n").unwrap();

    let mut child = xdg(&mut Command::new("timeout"))
        .args([
            "20",
            "script",
            "-qec",
            &format!("stty rows 15 cols 50; {BIN} --file {}", path.display()),
            "/dev/null",
        ])
        .env("XDG_STATE_HOME", dir.file("state"))
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("script(1) is needed for this test — it is in util-linux");

    std::thread::sleep(std::time::Duration::from_millis(600));

    // Written the way every safe editor writes: a new file renamed over the top.
    // A watch on the old inode would go silent right here.
    let swap = dir.file("todo.md.new");
    fs::write(
        &swap,
        "- [ ] the original task\n- [ ] arrived from outside\n",
    )
    .unwrap();
    fs::rename(&swap, &path).unwrap();

    std::thread::sleep(std::time::Duration::from_millis(600));
    let mut stdin = child.stdin.take().expect("stdin");
    stdin.write_all(b"q").unwrap();
    drop(stdin);

    let out = child.wait_with_output().expect("waiting");
    let screen = String::from_utf8_lossy(&out.stdout);
    assert!(
        screen.contains("arrived"),
        "the screen never picked up the change: {screen:?}"
    );
}

/// The `.ics` is derived, so it goes under `$XDG_DATA_HOME` and never next to
/// the list. Capturing regenerates it without being asked.
#[test]
fn the_calendar_is_written_beside_no_one_and_kept_up_to_date() {
    let dir = TempDir::new("ics");
    let path = dir.file("todo.md");
    let data = dir.file("data");
    // Two open dated tasks against one completed and one undated, so that every
    // way of getting the filter wrong lands on a different number. One of each
    // would let "count the done ones instead" produce the right answer.
    fs::write(
        &path,
        "- [ ] pay the invoice @2026-08-12\n- [ ] call the bank @2026-08-13\n\
         - [x] already done @2026-08-01\n- [ ] someday\n",
    )
    .unwrap();

    let with_data = |args: &[&str]| {
        let mut full = vec!["--file", path.to_str().unwrap()];
        full.extend_from_slice(args);
        let out = xdg(&mut Command::new(BIN))
            .args(&full)
            .env("XDG_DATA_HOME", &data)
            .env("XDG_STATE_HOME", dir.file("state"))
            .output()
            .expect("running the binary");
        assert!(
            out.status.success(),
            "{args:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).into_owned()
    };

    let said = with_data(&["sync"]);
    assert_eq!(
        said,
        {
            let mut expected = String::from("wrote 2 dated tasks to ");
            expected.push_str(&data.join("ratodo").join("todo.ics").display().to_string());
            expected.push('\n');
            expected
        },
        "the count includes tasks that were not exported"
    );

    let ics = data.join("ratodo").join("todo.ics");
    let text = fs::read_to_string(&ics).unwrap();
    assert!(text.contains("SUMMARY:pay the invoice"), "{text}");
    assert!(
        !dir.file("todo.ics").exists(),
        "the .ics landed by the list"
    );

    // A capture regenerates it, so the calendar is never a version behind.
    with_data(&["add", "renew the domain @2026-09-01"]);
    let text = fs::read_to_string(&ics).unwrap();
    assert!(text.contains("SUMMARY:renew the domain"), "{text}");
    assert_eq!(text.matches("BEGIN:VTODO").count(), 3, "{text}");
}

/// Runs with `$XDG_CONFIG_HOME` pointed somewhere a `theme.conf` can be planted.
fn with_config(dir: &TempDir, args: &[&str]) -> Output {
    xdg(&mut Command::new(BIN))
        .args(args)
        .env("XDG_CONFIG_HOME", dir.file("config"))
        .env("XDG_STATE_HOME", dir.file("state"))
        .env_remove("NO_COLOR")
        .output()
        .expect("running the binary")
}

fn plant_theme(dir: &TempDir, text: &str) {
    let config = dir.file("config").join("ratodo");
    fs::create_dir_all(&config).unwrap();
    fs::write(config.join("theme.conf"), text).unwrap();
}

#[test]
fn theme_list_names_the_built_ins_and_marks_the_default() {
    let dir = TempDir::new("theme-list");
    let out = with_config(&dir, &["theme", "list"]);
    assert!(out.status.success());

    let text = String::from_utf8_lossy(&out.stdout);
    for name in [
        "catppuccin-mocha",
        "catppuccin-latte",
        "gruvbox-dark",
        "nord",
        "dracula",
        "terminal",
    ] {
        assert!(text.contains(name), "{name} missing from {text}");
    }
    assert!(text.contains("catppuccin-mocha  (default)"), "{text}");
}

/// `ratodo theme dump > theme.conf` is the documented way to start a theme
/// file, so the output has to be a file ratodo reads back without complaint.
#[test]
fn a_dumped_theme_is_a_theme_file_ratodo_accepts() {
    let dir = TempDir::new("theme-dump");
    let dumped = with_config(&dir, &["theme", "dump"]);
    assert!(dumped.status.success());

    let text = String::from_utf8_lossy(&dumped.stdout).into_owned();
    assert!(text.contains("background = none"), "{text}");
    assert!(text.contains("accent     = #cba6f7"), "{text}");

    plant_theme(&dir, &text);
    let again = with_config(&dir, &["theme", "dump"]);
    assert!(
        again.stderr.is_empty(),
        "ratodo complained about its own dump: {}",
        String::from_utf8_lossy(&again.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&again.stdout), text);
}

#[test]
fn the_theme_flag_overrides_the_file() {
    let dir = TempDir::new("theme-flag");
    plant_theme(&dir, "accent = #ff0000\n");

    let from_file = with_config(&dir, &["theme", "dump"]);
    assert!(
        String::from_utf8_lossy(&from_file.stdout).contains("accent     = #ff0000"),
        "{}",
        String::from_utf8_lossy(&from_file.stdout)
    );

    let flagged = with_config(&dir, &["--theme", "terminal", "theme", "dump"]);
    let text = String::from_utf8_lossy(&flagged.stdout);
    assert!(!text.contains("#ff0000"), "--theme did not win: {text}");
    assert!(text.contains("accent     = magenta"), "{text}");
}

/// Invariant 8, end to end: whatever is in that file, the program still runs.
#[test]
fn a_broken_theme_file_warns_and_never_stops_anything() {
    let dir = TempDir::new("theme-broken");
    let path = dir.file("todo.md");
    fs::write(&path, "- [ ] still works\n").unwrap();
    plant_theme(
        &dir,
        "theme = nonsense\naccent = puce\nwibble = #ff0000\nnot a pair at all\n",
    );

    let out = with_config(&dir, &["--file", path.to_str().unwrap(), "theme", "dump"]);
    assert!(out.status.success(), "a bad theme file stopped the program");

    let complaints = String::from_utf8_lossy(&out.stderr);
    assert_eq!(complaints.lines().count(), 4, "{complaints}");
    assert!(complaints.contains("theme.conf:"), "{complaints}");

    // stdout is still a usable theme file — the warnings went to the other stream.
    let dumped = String::from_utf8_lossy(&out.stdout);
    assert!(dumped.contains("accent     = #cba6f7"), "{dumped}");
}

#[test]
fn no_color_flattens_every_role() {
    let dir = TempDir::new("no-color");
    let out = xdg(&mut Command::new(BIN))
        .args(["theme", "dump"])
        .env("XDG_CONFIG_HOME", dir.file("config"))
        .env("NO_COLOR", "1")
        .output()
        .expect("running the binary");

    let text = String::from_utf8_lossy(&out.stdout);
    let colours: Vec<&str> = text
        .lines()
        .filter_map(|l| l.split_once('='))
        .map(|(_, v)| v.trim())
        .collect();
    assert_eq!(colours.len(), 12, "{text}");
    assert!(colours.iter().all(|c| *c == "none"), "{text}");
}

/// The convention is any non-empty value. An empty `NO_COLOR=` is not a request.
#[test]
fn an_empty_no_color_is_not_a_request_for_no_colour() {
    let dir = TempDir::new("no-color-empty");
    let out = xdg(&mut Command::new(BIN))
        .args(["theme", "dump"])
        .env("XDG_CONFIG_HOME", dir.file("config"))
        .env("NO_COLOR", "")
        .output()
        .expect("running the binary");

    assert!(
        String::from_utf8_lossy(&out.stdout).contains("#cba6f7"),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
}

/// Hand-written completions rot: a seventh subcommand lands in `clap` and
/// nobody remembers the three files in `completions/`. This asks the binary
/// what it answers to and checks each shell was told.
#[test]
fn every_subcommand_and_flag_reaches_all_three_shells() {
    let help = String::from_utf8_lossy(&run(&["--help"]).stdout).into_owned();

    // clap prints the subcommands one per line, indented, under "Commands:".
    let commands: Vec<String> = help
        .lines()
        .skip_while(|l| !l.starts_with("Commands:"))
        .skip(1)
        .take_while(|l| l.starts_with("  ") && !l.trim().is_empty())
        .filter_map(|l| l.split_whitespace().next())
        .filter(|name| *name != "help")
        .map(str::to_string)
        .collect();

    assert!(
        commands.len() >= 6,
        "did not find the subcommands in --help: {commands:?}"
    );

    let list_flags = String::from_utf8_lossy(&run(&["list", "--help"]).stdout).into_owned();
    let flags: Vec<String> = list_flags
        .split_whitespace()
        .filter(|w| w.starts_with("--") && w.len() > 2)
        .map(|w| w.trim_end_matches(',').to_string())
        .collect();

    for shell in ["bash", "zsh", "fish"] {
        let script = fs::read_to_string(format!(
            "{}/completions/ratodo.{shell}",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap_or_else(|e| panic!("completions/ratodo.{shell}: {e}"));

        for command in &commands {
            assert!(
                script.contains(command.as_str()),
                "{shell} does not know about `{command}`"
            );
        }
        for flag in &flags {
            // fish spells a long option `-l name`, not `--name`.
            let spelling = match shell {
                "fish" => format!("-l {}", flag.trim_start_matches('-')),
                _ => flag.clone(),
            };
            assert!(
                script.contains(&spelling),
                "{shell} does not know about `{flag}` (looked for {spelling:?})"
            );
        }
    }
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
