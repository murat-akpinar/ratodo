## [unreleased]

### 🚀 Features

- *(parse)* Read and write todo.md without disturbing it ([24b15aa](https://github.com/murat-akpinar/ratodo/commit/24b15aab8b996a43926ae106a09386bc0e06c450))
The pure core of the product, with no terminal involved: parse, write, capture, and the two tests that the whole design rests on.
- *(agenda)* Group the list by date ([c5af3b5](https://github.com/murat-akpinar/ratodo/commit/c5af3b5069619e86ee23c20f655fb43081e758ab))
`agenda(&[Task], today) -> Vec<Group>` with `today` as a parameter, so the whole of the product's real logic is testable without a clock or a terminal. `list` prints those groups instead of walking the file's sections.
- *(cli)* Filter the list and print it for scripts ([18f7512](https://github.com/murat-akpinar/ratodo/commit/18f7512e28f6715ddde6c64786f4432a3c9ee7a5))
`list --tag` (repeatable, or) and `--prio` reach the part of the file the agenda has nothing to say about, which for a developer's list is most of it. `--porcelain` is the stable surface underneath `ratodo done "$(ratodo list --porcelain | fzf | cut -f3)"`.
- *(cli)* Add status for the bar ([5a7e3e9](https://github.com/murat-akpinar/ratodo/commit/5a7e3e9e317c33f11818a63444447f57b68816d3))
`ratodo status` on one line, `--json` in the shape waybar and eww read, and exit 1 when something is overdue so `ratodo status || notify-send "$(ratodo status)"` needs no extra flag. `main` returns `ExitCode` rather than calling `process::exit`, so nothing skips a destructor to carry a number out.
- *(cli)* Mark a task done from the command line ([3c0f8b4](https://github.com/murat-akpinar/ratodo/commit/3c0f8b49455f91d508c0c1de4403ce58e1c4c067))
`ratodo done '<text>'`: case-insensitive substring over the **open** tasks, a unique match required. One match ticks one byte; several print the candidates and exit 2 without the file ever being opened for writing; none exits 2 and says so.
- *(ui)* Draw the list in a terminal ([b044b69](https://github.com/murat-akpinar/ratodo/commit/b044b69de720de28286e6d01fcd54ec1ef916ea2))
The dumb version from todo.md step 4: the agenda flattened into rows, a border, the counts, `j k` and the arrows, `g G`, `q` and ctrl-c. The design in docs/tui.md arrives in step 6; this is the commit that proves the loop runs and gives it a way to be tested.
- *(ui)* Follow the file while the screen is open ([b4db638](https://github.com/murat-akpinar/ratodo/commit/b4db638d2c2c5fc3fe1769223f5f4be06212cf49))
vim, `git pull` or `ratodo add` in another pane now reaches the open list on its own, which is the promise in docs/architecture.md#concurrent-editing.
- *(ics)* Export dated tasks as VTODO ([3d3bd22](https://github.com/murat-akpinar/ratodo/commit/3d3bd22204481ef87e676e452e03d0966a193b09))
`ratodo sync` writes `~/.local/share/ratodo/todo.ics`, and every capture rewrites it so the calendar is never a version behind. Open, dated tasks only; ~90 lines of string formatting and no eighth dependency.
- *(theme)* Eleven colour roles, six built-ins and a theme.conf ([73ac1aa](https://github.com/murat-akpinar/ratodo/commit/73ac1aa4a5363b98799b3e9c1341d9966f152c42))
`theme.rs` holds the `Theme` struct, the built-in tables, the parser and the resolver, in one file and with no new dependency — the parser is `split_once` and a hex decode. `--theme`, `ratodo theme list` and `ratodo theme dump` are wired up, and the colours actually reach the screen.
- *(ui)* Apply the design to the list ([f556500](https://github.com/murat-akpinar/ratodo/commit/f5565003d22ef5fc4abbd1cbfefc3e60788de0dd))
The screens in docs/tui.md, drawn: `○ ✓ !` marks, a `▌` selection, group headers with a rule out to the right edge, and a right-aligned column carrying the date, then the priority, then the tags.

### 🐛 Bug Fixes

- *(write)* Keep symlinks, file modes and the terminal safe ([cacac46](https://github.com/murat-akpinar/ratodo/commit/cacac4633669ff32ac492c449950f082162eac30))
A review of the write path turned up two defects that would have bitten exactly the audience this tool is for, plus two smaller ones.
- *(write)* Keep the backup out of the user's dotfiles, and the capture inside their sections ([83fca6d](https://github.com/murat-akpinar/ratodo/commit/83fca6d6e565a30b828ba91c7b979e18963e8c77))
Four defects that two audience design reviews turned up in code that was already pushed. Each one is small; each one would have been noticed by exactly the person this tool is for.
- *(cli)* Stop panicking when the reader closes the pipe ([6b8219c](https://github.com/murat-akpinar/ratodo/commit/6b8219c5d7f6429502d6cbea865e4ba8299906a6))
`ratodo list | head -3` closed the pipe half way through and `println!` turned the next write into a panic — a backtrace and exit 101 for a command that did nothing wrong, and `| head` is the first thing this audience types. Found by running the binary, not by a test.

### 📚 Documentation

- Restructure the design record into a docs/ directory ([81aba9d](https://github.com/murat-akpinar/ratodo/commit/81aba9d1ee5bf9700ed26ebbbfd334abe2c3ad40))
Split the single 25 KB decision record into eleven focused documents under docs/, one question per file, and translate everything to English.
- *(ui)* Design every screen and settle the keymap ([4e6bef8](https://github.com/murat-akpinar/ratodo/commit/4e6bef82ab8479a27072dbb1ae284bf76a89503a))
The design record had one screen sketch and a seven-key table. Neither covered what the tool actually has to do while someone works next to it.
- Record what two audience design reviews found ([96d0a2c](https://github.com/murat-akpinar/ratodo/commit/96d0a2cdb9d64dcfe5a665061cced58ecc226454))
Two reviews were run against the design, one from each profile this tool claims to be for: a tiling-WM ricer and a terminal-bound developer. They ran separately and reached the same objection, which is why it is worth recording rather than arguing with.
- Point the readme at the command line that now exists ([f5e63b3](https://github.com/murat-akpinar/ratodo/commit/f5e63b36cdbc54e4547227e676370b79bc7704fc))

### 🧪 Testing

- Close the holes mutation testing found ([2809f2a](https://github.com/murat-akpinar/ratodo/commit/2809f2ac2e8e76a52be20e8885a5ba4e5bd87a3f))
The suite was green and partly decorative. `cargo mutants` breaks the source one edit at a time and checks that something goes red; on the first run 34 of 180 mutants survived, meaning 34 ways to break ratodo that no test objected to. It is now 0.

### ⚙️ Miscellaneous Tasks

- Add agent working rules and a Rust gitignore ([6449064](https://github.com/murat-akpinar/ratodo/commit/644906428c4fa86dd1c0c64352274950cff90de5))
CLAUDE.md carries the workflow (write, self-review, test, commit, changelog, push), the conventional-commit format cliff.toml requires, and the nine hard invariants that must not be broken without an explicit decision — round-trip fidelity first among them.
- Keep changelog commits out of the changelog ([41aabd3](https://github.com/murat-akpinar/ratodo/commit/41aabd378b79b68a147e6c8b4b293d20877937d3))
