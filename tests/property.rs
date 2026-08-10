//! Generated documents, checked against the invariants. docs/testing.md.
//!
//! The fixtures cover the cases we thought of. This covers the ones we did not:
//! a few thousand deliberately awkward documents, built from a fixed seed so a
//! failure is reproducible from the number printed in the message.

use ratodo::{parse::parse, write::render};

const RUNS: u64 = 4000;

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }

    fn pick<'a>(&mut self, xs: &'a [&'a str]) -> &'a str {
        xs[self.below(xs.len())]
    }

    fn chance(&mut self, percent: usize) -> bool {
        self.below(100) < percent
    }
}

const INDENTS: &[&str] = &["", " ", "  ", "\t", "   ", "\t  "];
const BULLETS: &[&str] = &["-", "*", "+"];
const BOXES: &[&str] = &["[ ]", "[x]", "[X]"];
const WORDS: &[&str] = &[
    "pay",
    "the",
    "invoice",
    "fatura",
    "öde",
    "şğüöçİI",
    "🚀",
    "deploy",
    "rotate",
    "keys",
    "a",
];
const META: &[&str] = &[
    "@2026-08-12",
    "@2026-08-12 16:00",
    "@2026-13-45",
    "@2026-02-30",
    "@",
    "@notadate",
    "#ops",
    "#a",
    "#",
    "#şey",
    "!high",
    "!med",
    "!low",
    "!nope",
    "!",
    "16:00",
];
/// Every one of these must **not** parse as a task. Several are one character
/// away from being one, which is the point.
const NON_TASKS: &[&str] = &[
    "# My list",
    "## Work",
    "### deeper",
    "#ops",
    "",
    "   ",
    "just a paragraph",
    "> a quote",
    "| a | table |",
    "|---|---|",
    "---",
    "-[ ] no space after the bullet",
    "- [] empty brackets",
    "- [?] not a checkbox",
    "- [ok] not a checkbox either",
    "-- [ ] two bullets",
    "- [ ]x no space after the box",
    "- [x]done, no space either",
    "- [",
    "- [ ",
    "- ",
    "-",
    "\t",
    "> ",
];

fn task_line(rng: &mut Rng) -> String {
    let mut line = String::new();
    line.push_str(rng.pick(INDENTS));
    line.push_str(rng.pick(BULLETS));
    for _ in 0..=rng.below(3) {
        line.push(' ');
    }
    line.push_str(rng.pick(BOXES));

    if rng.chance(90) {
        for _ in 0..=rng.below(3) {
            line.push(' ');
        }
        let parts = rng.below(6);
        for _ in 0..parts {
            if rng.chance(60) {
                line.push_str(rng.pick(WORDS));
            } else {
                line.push_str(rng.pick(META));
            }
            for _ in 0..=rng.below(2) {
                line.push(' ');
            }
        }
        if rng.chance(20) {
            line.push_str("   ");
        }
    }
    line.trim_end_matches('\n').to_string()
}

/// A generated document, and the 1-based line numbers that are tasks.
///
/// Keeping the answer alongside the input is what turns this from a fidelity
/// test into a recognition test: a parser that recognised nothing at all would
/// satisfy every byte-level invariant below and be completely broken.
struct Generated {
    text: String,
    task_lines: Vec<usize>,
}

fn document(seed: u64) -> Generated {
    let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
    let count = rng.below(25);
    let mut text = String::new();
    let mut task_lines = Vec::new();

    for i in 0..count {
        if rng.chance(55) {
            text.push_str(&task_line(&mut rng));
            task_lines.push(i + 1);
        } else {
            text.push_str(rng.pick(NON_TASKS));
        }
        let last = i + 1 == count;
        if last && rng.chance(25) {
            break; // no final newline
        }
        text.push_str(if rng.chance(20) { "\r\n" } else { "\n" });
    }

    Generated { text, task_lines }
}

/// Where a failure gets reported from, so the seed is always in the message.
fn check(seed: u64, generated: &Generated) {
    let doc_text = generated.text.as_str();
    let doc = parse(doc_text);

    let found: Vec<usize> = doc.tasks().map(|t| t.line_no).collect();
    assert_eq!(
        found, generated.task_lines,
        "seed {seed}: the parser did not find exactly the generated tasks\n{doc_text:?}"
    );

    assert_eq!(
        render(&doc),
        doc_text,
        "seed {seed}: rendering changed an untouched document\n{doc_text:?}"
    );

    assert_eq!(
        parse(&render(&doc)),
        doc,
        "seed {seed}: parse is not stable across a round trip\n{doc_text:?}"
    );

    for task in doc.tasks() {
        assert!(
            doc_text.contains(&task.raw),
            "seed {seed}: a task's raw line is not in the source\n{:?}",
            task.raw
        );
    }

    let task_count = doc.task_count();
    for nth in 0..task_count {
        let mut mutated = parse(doc_text);
        let task = mutated.tasks_mut().nth(nth).expect("task exists");
        task.set_done(!task.done);
        let after = render(&mutated);

        let before_lines: Vec<&str> = doc_text.split_inclusive('\n').collect();
        let after_lines: Vec<&str> = after.split_inclusive('\n').collect();
        assert_eq!(
            before_lines.len(),
            after_lines.len(),
            "seed {seed}: toggling task {nth} changed the line count\n{doc_text:?}"
        );

        let mut differing = 0;
        for (a, b) in before_lines.iter().zip(&after_lines) {
            if a == b {
                continue;
            }
            differing += 1;
            assert_eq!(
                a.len(),
                b.len(),
                "seed {seed}: toggling task {nth} resized a line\n{a:?}\n{b:?}"
            );
            let changed: Vec<usize> = a
                .char_indices()
                .zip(b.chars())
                .filter(|((_, x), y)| x != y)
                .map(|((i, _), _)| i)
                .collect();
            assert_eq!(
                changed.len(),
                1,
                "seed {seed}: toggling task {nth} changed {} characters\n{a:?}\n{b:?}",
                changed.len()
            );
        }
        assert_eq!(
            differing, 1,
            "seed {seed}: toggling task {nth} touched {differing} lines\n{doc_text:?}"
        );
    }
}

#[test]
fn generated_documents_hold_every_invariant() {
    for seed in 0..RUNS {
        check(seed, &document(seed));
    }
}

/// The line each task reports must be the line it actually came from, or every
/// error message and every future edit points at the wrong place.
#[test]
fn line_numbers_are_one_based_and_correct() {
    for seed in 0..200 {
        let generated = document(seed);
        let lines: Vec<&str> = generated.text.split_inclusive('\n').collect();
        for task in parse(&generated.text).tasks() {
            let source = lines[task.line_no - 1].trim_end_matches(['\n', '\r']);
            assert_eq!(
                source, task.raw,
                "seed {seed}: task claims line {} but that line is different",
                task.line_no
            );
        }
    }
}

/// The way a property test lies to you is by never generating the interesting
/// case: 4000 documents that all happen to be empty would pass everything above
/// and prove nothing. This asserts the corpus actually contains each trap.
#[test]
fn the_generator_produces_what_we_think_it_does() {
    let corpus: Vec<String> = (0..RUNS).map(|s| document(s).text).collect();
    let all = corpus.join("");

    let total_tasks: usize = corpus.iter().map(|d| parse(d).task_count()).sum();
    let with_tasks = corpus.iter().filter(|d| parse(d).task_count() > 0).count();
    let non_empty = corpus.iter().filter(|d| !d.is_empty()).count();

    assert!(
        total_tasks > 10_000,
        "only {total_tasks} tasks generated — the invariants are barely exercised"
    );
    assert!(
        with_tasks * 2 > corpus.len(),
        "only {with_tasks}/{} documents contain a task",
        corpus.len()
    );
    assert!(non_empty * 10 > corpus.len() * 9);

    for (what, needle) in [
        ("CRLF endings", "\r\n"),
        ("tab indentation", "\t-"),
        ("capital X boxes", "[X]"),
        ("done boxes", "[x]"),
        ("star bullets", "* ["),
        ("plus bullets", "+ ["),
        ("invalid dates", "@2026-13-45"),
        ("bare at signs", " @ "),
        ("bare hashes", " # "),
        ("times", "16:00"),
        ("emoji", "🚀"),
        ("non-ASCII", "şğüöçİI"),
        ("tables", "| a | table |"),
        ("quotes", "> a quote"),
        ("headings", "## Work"),
        ("horizontal rules", "---"),
        ("near-miss task lines", "- [?]"),
        ("boxes with nothing after them", "- [ ]x"),
        ("truncated boxes", "- ["),
    ] {
        assert!(all.contains(needle), "the corpus never generated {what}");
    }

    // A box immediately at the end of the line is the case that separates
    // `i + 2` from `i * 2` in the bounds check, and it is easy to never emit.
    let bare_boxes = corpus
        .iter()
        .flat_map(|d| d.split_inclusive('\n'))
        .filter(|l| {
            let t = l.trim_end_matches(['\n', '\r']);
            t.ends_with("[ ]") || t.ends_with("[x]") || t.ends_with("[X]")
        })
        .count();
    assert!(
        bare_boxes > 50,
        "only {bare_boxes} lines ended right after the box"
    );

    let missing_final_newline = corpus
        .iter()
        .filter(|d| !d.is_empty() && !d.ends_with('\n'))
        .count();
    assert!(
        missing_final_newline > 100,
        "only {missing_final_newline} documents lacked a final newline"
    );
}

/// The checker must reject a document that has been damaged, or the assertions
/// above are decoration. This proves the comparison has teeth without touching
/// the real parser.
#[test]
fn the_checker_would_notice_damage() {
    let original = "## Work\n- [ ] a @2026-08-12 #ops\n> quote\n";

    for damaged in [
        "## Work\n- [ ] a @2026-08-12 #ops\n", // a line went missing
        "## Work\n- [ ] a @2026-08-12 #ops\n> quote", // final newline dropped
        "## Work\n- [ ] a  @2026-08-12 #ops\n> quote\n", // whitespace altered
        "## Work\n- [ ] a @2026-08-12 #ops\r\n> quote\n", // ending rewritten
    ] {
        assert_ne!(
            render(&parse(original)),
            damaged,
            "the fidelity comparison would have accepted {damaged:?}"
        );
    }
}
