//! The two tests that matter. docs/testing.md.

use std::fs;
use std::path::PathBuf;

use ratodo::model::Item;
use ratodo::{parse::parse, write::render};

const FIXTURES: &[&str] = &[
    "simple.md",
    "gnarly.md",
    "crlf.md",
    "no-final-newline.md",
    "empty.md",
];

fn fixture(name: &str) -> String {
    let path: PathBuf = [env!("CARGO_MANIFEST_DIR"), "tests", "fixtures", name]
        .iter()
        .collect();
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()))
}

#[test]
fn rendering_an_untouched_file_returns_it_byte_for_byte() {
    for name in FIXTURES {
        let original = fixture(name);
        assert_eq!(render(&parse(&original)), original, "fixture {name}");
    }
}

#[test]
fn round_trip() {
    for name in FIXTURES {
        let original = fixture(name);
        let once = parse(&original);
        let twice = parse(&render(&once));
        assert_eq!(once, twice, "fixture {name}");
    }
}

#[test]
fn completing_one_task_leaves_every_other_line_byte_for_byte() {
    for name in FIXTURES {
        let original = fixture(name);
        let task_count = parse(&original).task_count();

        for nth in 0..task_count {
            let mut doc = parse(&original);
            let task = doc.tasks_mut().nth(nth).expect("task exists");
            let was_done = task.done;
            task.set_done(!was_done);

            let after = render(&doc);
            let before_lines: Vec<&str> = original.split_inclusive('\n').collect();
            let after_lines: Vec<&str> = after.split_inclusive('\n').collect();

            assert_eq!(
                before_lines.len(),
                after_lines.len(),
                "{name}: toggling task {nth} changed the line count"
            );

            let mut changed = 0;
            for (a, b) in before_lines.iter().zip(&after_lines) {
                if a != b {
                    changed += 1;
                    assert_eq!(
                        a.len(),
                        b.len(),
                        "{name}: toggling task {nth} resized a line"
                    );
                    let diff = a.chars().zip(b.chars()).filter(|(x, y)| x != y).count();
                    assert_eq!(diff, 1, "{name}: more than one character changed");
                }
            }
            assert_eq!(
                changed, 1,
                "{name}: toggling task {nth} touched {changed} lines"
            );
        }
    }
}

#[test]
fn gnarly_parses_the_way_the_docs_say_it_should() {
    let doc = parse(&fixture("gnarly.md"));
    let tasks: Vec<_> = doc.tasks().collect();

    let titles: Vec<&str> = tasks.iter().map(|t| t.title.as_str()).collect();
    assert!(titles.contains(&"a star-bulleted item"));
    assert!(titles.contains(&"a plus-bulleted item"));
    assert!(titles.contains(&"an indented subtask"));
    assert!(titles.contains(&"tab-indented task"));
    assert!(titles.contains(&"three-space title"));

    for title in &titles {
        assert!(
            !title.starts_with("not a task"),
            "parsed a non-task: {title}"
        );
    }

    let invalid = tasks
        .iter()
        .find(|t| t.title.starts_with("invalid date"))
        .expect("the invalid-date task");
    assert!(invalid.due.is_none());
    assert_eq!(invalid.title, "invalid date @2026-13-45");

    let three = tasks
        .iter()
        .find(|t| t.title == "three tags")
        .expect("the three-tag task");
    assert_eq!(three.tags, vec!["a", "b", "c"]);
    assert!(three.due.is_some());

    let unicode = tasks
        .iter()
        .find(|t| t.title.starts_with("non-ASCII"))
        .expect("the non-ASCII task");
    assert!(unicode.title.contains('🚀'));
    assert!(unicode.title.contains("şğüöçİI"));

    assert_eq!(
        tasks
            .iter()
            .filter(|t| t.section.as_deref() == Some("Personal"))
            .count(),
        3
    );
}

#[test]
fn everything_that_is_not_a_task_is_preserved_as_text() {
    let original = fixture("gnarly.md");
    let doc = parse(&original);

    let text_lines: Vec<&String> = doc
        .lines
        .iter()
        .filter_map(|l| match &l.item {
            Item::Text(s) => Some(s),
            Item::Task(_) => None,
        })
        .collect();

    for expected in [
        "# My list",
        "> A quoted line. Do not touch.",
        "| a | table |",
        "---",
        "-[ ] not a task, no space after the bullet",
        "",
    ] {
        assert!(
            text_lines.iter().any(|l| l.as_str() == expected),
            "missing preserved line: {expected:?}"
        );
    }
}

#[test]
fn the_simple_fixture_matches_the_documented_example() {
    let docs_example: PathBuf = [env!("CARGO_MANIFEST_DIR"), "docs", "examples", "todo.md"]
        .iter()
        .collect();
    assert_eq!(
        fs::read_to_string(docs_example).expect("docs/examples/todo.md"),
        fixture("simple.md"),
        "tests/fixtures/simple.md drifted from docs/examples/todo.md"
    );
}
