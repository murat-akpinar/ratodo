## [unreleased]

### 🚀 Features

- *(ui)* [**breaking**] Swap the delete and cancel keys ([e1e5b41](https://github.com/murat-akpinar/ratodo/commit/e1e5b41b9b829b63dffeef2ab58a160c94d129c3))
`d` cancels and `X` deletes, the other way round from v0.2.0. The shift was on the wrong key: cancelling is reversible — `d` again takes it back and the row stays in the file as `- [-]` — while deleting takes a line out of the user's file behind one level of undo that a `q` spends. The key that costs the most is the one that should cost a shift, and `d` sitting a row from `j` and `k` made the cheap key the destructive one.
- *(ui)* Say so when an @ can never be a date ([fe648c4](https://github.com/murat-akpinar/ratodo/commit/fe648c41e49faa4dd7e517084256198963dc43be))
`@2026-13-45` resolves to nothing, so the word falls back to being part of the title. The fallback stays — a word we did not understand belongs to the user — but it was silent, and the preview went quiet in exactly the moment it should speak.
- *(ui)* Copy the selected task with y ([4e0e6fd](https://github.com/murat-akpinar/ratodo/commit/4e0e6fd709e617eb06193c9fb0a25a85f17d707a))
A task that is nearly one already on the list had no way in but `a` and the whole line again. `y` opens the input box pre-filled with the task under the cursor, as a new one: edit it, `⏎` saves a second task, `esc` writes nothing.

### 📚 Documentation

- Put what is left at the top of todo.md ([bb63dcb](https://github.com/murat-akpinar/ratodo/commit/bb63dcb23567cdef37025ca82ff00ab96055ab78))
- Record the swapped keys and the date-entry complaint ([853092b](https://github.com/murat-akpinar/ratodo/commit/853092b87394b2f002a280c054d12777d6e4ce82))
todo.md and notes.md still had `d` deleting. The build record keeps what shipped, with the swap noted where it happened rather than rewritten out.
## [0.2.0] - 2026-08-11

### 🚀 Features

- *(cli)* [**breaking**] Read every list in the config directory as one agenda ([c08c96b](https://github.com/murat-akpinar/ratodo/commit/c08c96ba3bdfc7019fd541d03261238f9bfe2a2c))
Somebody who keeps work.md, personal.md and 2026.md apart on disk still wants one screen. Every `*.md` in the config directory is now a list; `--file` and `$RATODO_FILE` still name exactly one.
- *(format)* [**breaking**] A third state, a completion date, and p to put one off ([fd38a23](https://github.com/murat-akpinar/ratodo/commit/fd38a23309f65f02796e4a00ced507094e76c50e))
Four things the list could not say, and one it said too quietly.
- *(ui)* A cancelled row is red ([2433653](https://github.com/murat-akpinar/ratodo/commit/2433653b6ac006b6091a8c28d75806e2026ff188))
Grey said *finished*, and a cancelled task is the opposite of finished — it is the one that will not be. Three states wanted three colours and the alternatives were both worse: a twelfth theme role for one row, or leaving cancelled looking like something that had been dealt with.

### 🐛 Bug Fixes

- *(ui)* Colour the parse preview field by field ([7173a34](https://github.com/murat-akpinar/ratodo/commit/7173a34619999c5d6d1d439a77d10ef37c9412a4))
The row under the input was one accent-coloured string, which said the parser had understood all of it equally: the resolved date and the tag came out the same colour in the one row whose job is telling them apart.
- *(cli)* Stop the test suite writing into the real ~/.local ([0a20020](https://github.com/murat-akpinar/ratodo/commit/0a200204e0be912f6964bef5907a634b780656ba))
`write_back` called `backup_dir()` and resolved the calendar path itself, both of which read the environment. The callers furthest from `main` are the tests, so every in-process case wrote into the developer's own directories: it regenerated their real `~/.local/share/ratodo/todo.ics` from a fixture and left a `.bak` per case in `~/.local/state/ratodo` — twenty-two megabytes of them on the machine this was found on. `tests/cli.rs` had the same hole from the other side, setting `XDG_STATE_HOME` and `XDG_CACHE_HOME` and forgetting `XDG_DATA_HOME`, under a comment claiming every XDG directory was covered.

### 📚 Documentation

- The three states, the stamp, p, and two reversals ([3e57ca4](https://github.com/murat-akpinar/ratodo/commit/3e57ca4a7d5caae490d34fd3b0a0624e8c44546b))
format.md gains the state and stamp rows plus the two sections behind them; tui.md gains the `p` box, the finished-row colour and date, and an adaptive hint bar in place of a fixed six. decisions.md records three settled decisions and three reversals — grey to green, `x` staying unbound while `X` takes the job, and `:` `/` leaving the overlay to make room.
- *(calendar)* Khal does not show these, todoman does ([26858dd](https://github.com/murat-akpinar/ratodo/commit/26858dd7794d3bb71c301db93409986b26cee0de))
The `.ics` had been verified as *parseable* — by Python's icalendar, a different implementation of the same RFC — and never as *displayed*. todo.md had that open as "the one that catches a client quietly ignoring VTODO", and it caught one on the first try.
- *(calendar)* Todoman verified against a packaged install ([9709e4d](https://github.com/murat-akpinar/ratodo/commit/9709e4dc3e6c995d7efeddc435e2ffdb7681d49b))
- *(roadmap)* Why --as-events is worth building, with the evidence ([838c073](https://github.com/murat-akpinar/ratodo/commit/838c07326a49ad203d45c8f47262f354042ea2ee))
It was a one-line entry reading "for calendar clients that ignore VTODO", which is a guess until somebody checks. Somebody checked: khal shows none of a generated todo.ics, and a hand-written VEVENT dropped into the same directory under the same config appears — so it is the entry type being ignored, not our output being malformed.
- *(docs)* Move the four questions the code already answered to resolved ([1c2df3f](https://github.com/murat-akpinar/ratodo/commit/1c2df3fdada5c5458b97456fc497ba487e0f7fbc))

### 🧪 Testing

- *(cli)* Pin what Live knows about its files ([4c122de](https://github.com/murat-akpinar/ratodo/commit/4c122de870e115e311c6f70013c723cc0cfd0364))
- *(cli)* Pin that the loop only reads the disk when the watcher spoke ([07be08a](https://github.com/murat-akpinar/ratodo/commit/07be08a3c36782475957e1523d114d95ac7f0be0))
- *(cli)* Pin the way back out of every state, and what p writes ([72ad547](https://github.com/murat-akpinar/ratodo/commit/72ad54773f68bb63fbe8be787ccaac126bb1f608))
`cargo mutants` found three holes, all of them in code added an hour ago and all of them the same shape: the *return* path was never driven. Deleting the `State::Done` arm of `toggle`, the `State::Cancelled` arm of `cancel`, or the `Purpose::Postpone` arm of `save_typed` broke nothing any test objected to — so "the same key both ways" and "p moves the date" were claims made in comments and documentation with nothing behind them.
## [0.1.0] - 2026-08-11

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
- *(ui)* Tick a task from the screen, and give the bottom line its job ([176d0f8](https://github.com/murat-akpinar/ratodo/commit/176d0f835e7b78c208bd01cc98cc27b5258cf390))
`spc` marks the selected task done and writes it. The row is rewritten **in place** rather than the list regrouped, so the task you just touched does not fly to the end of its group while you are looking at it — the first of the side-pane rules in docs/tui.md. A test through a real terminal confirms the whole path: one keystroke, one byte changed, in a file full of tables and prose ratodo does not understand.
- *(ui)* The empty screen and the key help ([e358b3a](https://github.com/murat-akpinar/ratodo/commit/e358b3ac4051f6eeabfffe3183bdb24986ca1ef0))
Two screens from docs/tui.md. The empty one teaches instead of apologising: the worked example lands the `@` and `#` syntax faster than any table, and it names the file path, because the promise of this product is that the file is yours and you should be told where it is on day one. A long `--file` path is shortened rather than left to run into the frame.
- *(cli)* Hand-written completions for bash, zsh and fish ([ad80c0c](https://github.com/murat-akpinar/ratodo/commit/ad80c0c7e5c00ffc53965a96e47d62493e0f597d))
`completions/ratodo.{bash,zsh,fish}`, and no `clap_complete` — that would be an eighth dependency for a subcommand list that is short and fixed.
- *(ui)* Open $EDITOR on e, and poll instead of parking a thread ([7190d34](https://github.com/murat-akpinar/ratodo/commit/7190d34b43d90b35c40a2ae6184bb22974535682))
`e` hands the terminal to `$VISUAL` or `$EDITOR`, waits, takes it back and re-reads the file. The escape hatch: whatever the tool cannot do, the file can, and the file is Markdown the user already knows how to edit.
- *(ui)* Fold a group with h, l and z ([d7601e1](https://github.com/murat-akpinar/ratodo/commit/d7601e19a36171b2d33fffea6503f4cf440dbb26))
The keys `lf`, `ranger` and `yazi` users arrive with: `h` collapses the group under the cursor, `l` opens it, `z` does whichever is the opposite of now.
- *(ui)* Delete with d, take it back with u ([97d615e](https://github.com/murat-akpinar/ratodo/commit/97d615e1a36a40347ef5f030076de8bb9df290cd))
`d` removes the selected task immediately and `u` puts it back — the trade docs/tui.md makes, where a confirmation prompt would tax every delete to catch the rare wrong one.
- *(ui)* Input mode, with a live parse preview ([59cdbd0](https://github.com/murat-akpinar/ratodo/commit/59cdbd006044488a68f1608e69f4d4c12fce1673))
`a` and `o` open a field on the bottom line and `⏎` opens it already holding the selected task. While it is open the keyboard belongs to it — `q` and `d` are letters in there, which is what makes "you can never be in a mode you did not open" true by construction. `esc` and `ctrl-c` both cancel, and `ctrl-c` in here never quits: somebody half-way through a sentence loses the sentence, not the session.
- *(ui)* Keep the selection on the task, not on its line ([14dfde6](https://github.com/murat-akpinar/ratodo/commit/14dfde60e5e9c43fe8c359bd8757900a372be211))
The cursor followed the raw line, so anything that rewrote it let go: a `ratodo done` in the next pane, a tag arriving over `git pull`, a date moved. The line is not the task.
- *(ui)* Show what is finished on the title rule ([501905f](https://github.com/murat-akpinar/ratodo/commit/501905f853f138f9ed72e1140eaec428376d39b6))
Eight cells and a `3/8` on the right of the top border. The screen had nothing to say about work that was done: `5 open · 1 overdue` counts what is left and stops there, which makes a todo list that never acknowledges the todo part.
- *(ui)* Lay the right-hand fields out in columns past eighty ([1bd16ff](https://github.com/murat-akpinar/ratodo/commit/1bd16ff10e09cd3309e07d888b09a30e9244e72c))
The right-aligned block only reads down its edge when every row ends in the same field, and they do not: `3d ago  !high  #ops` and `1d ago #home` are aligned as a blob, so the dates land somewhere different on every row. Past eighty columns the date, the priority and the tags become real columns, the title column is the widest title in the list, and the group rule stops where that column ends instead of running to an edge fifty columns away.
- *(ui)* Keep the markdown marker on the user's own headings ([9d0d3cb](https://github.com/murat-akpinar/ratodo/commit/9d0d3cb0ebaad4f01a30e38a8029954e54636def))
OVERDUE is ours and Work came out of the file, and as the same bold word plus the same rule nothing on the screen said which was which. The user's headings now keep the ## they already carry in the file: no second colour, no third level of hierarchy, and it survives the ASCII fallback.
- *(ui)* Give !high the weight the user meant by it ([22ef83d](https://github.com/murat-akpinar/ratodo/commit/22ef83d25f5378c5b8413a8289de4a0f4ea8aa60))
The one field somebody typed to mean urgent sat in the same grey as the date and the tags. It is bold now, in the row's own colour — weight rather than a twelfth theme role, so it still reads under NO_COLOR and does not collide with overdue on the rows that have both. !med and !low stay quiet, and a ticked task is not urgent however it was filed.
- *(ui)* Colour the date only when it presses ([e520b4a](https://github.com/murat-akpinar/ratodo/commit/e520b4aff0c3c682ca864dcd7b68f426a8e4eb59))
The date column is where the lateness is, and it was the one field saying so in grey while the title beside it went red. It takes overdue for a late task and today for one due today — the two roles the title already uses, so nothing new to theme — and stays dim for everything else. A finished task's date is dim whatever it says: it is neither late nor due.
- *(ui)* [**breaking**] Open the input as a box over the list ([3e04464](https://github.com/murat-akpinar/ratodo/commit/3e04464ccac7ff6bd753dc4bef77fcaa9b58f67e))
The bottom line was chosen so the screen would not change under the reader, and it was right about the wrong thing. This tool lives in a pane in the corner of a tiling layout, which puts that line at the bottom edge of the screen: every capture meant looking down there, away from the row being worked on. The head movement is the interruption.
- *(ui)* Colour the input field by what the parser took ([2eb9ef2](https://github.com/murat-akpinar/ratodo/commit/2eb9ef250b35b1e3bed4ab98d18eb2f229ec9132))
The structure of the shorthand was only visible in the preview under the field, a word at a time after the fact. It is on the words themselves now: @thu and the time it took go accent, #home goes tag, !high goes bold, and a @notaday stays plain text because that is what the file will hold.
- *(ui)* Teach the empty screen with the box it will be typed into ([f7341a9](https://github.com/murat-akpinar/ratodo/commit/f7341a98178d75b0679f73c3026250fc73af64b6))
The example was a line of text. It is now the same input box `a` opens, drawn by the same code, so the live parse under it already reads the shorthand back as a date. Below ten rows it goes back to a line — the example is the last thing a short pane loses.
- *(ui)* Rule off the input field from what it will become ([ff9aa07](https://github.com/murat-akpinar/ratodo/commit/ff9aa075a667fcd25d2bd6475f144aedca7a334b))
The field and the live parse sat in one box with nothing between them, so the caret looked like it could be moved down into the preview. The box takes a fifth row for the rule; a pane too short for one drops the rule, not the preview. The two cells where it meets the frame are set to `├` and `┤`, since a rule butting into `│` reads as a frame that broke.

### 🐛 Bug Fixes

- *(write)* Keep symlinks, file modes and the terminal safe ([cacac46](https://github.com/murat-akpinar/ratodo/commit/cacac4633669ff32ac492c449950f082162eac30))
A review of the write path turned up two defects that would have bitten exactly the audience this tool is for, plus two smaller ones.
- *(write)* Keep the backup out of the user's dotfiles, and the capture inside their sections ([83fca6d](https://github.com/murat-akpinar/ratodo/commit/83fca6d6e565a30b828ba91c7b979e18963e8c77))
Four defects that two audience design reviews turned up in code that was already pushed. Each one is small; each one would have been noticed by exactly the person this tool is for.
- *(cli)* Stop panicking when the reader closes the pipe ([6b8219c](https://github.com/murat-akpinar/ratodo/commit/6b8219c5d7f6429502d6cbea865e4ba8299906a6))
`ratodo list | head -3` closed the pipe half way through and `println!` turned the next write into a panic — a backtrace and exit 101 for a command that did nothing wrong, and `| head` is the first thing this audience types. Found by running the binary, not by a test.
- *(docs)* Match GitHub's anchor slugs, which hyphenate every space ([4a092ea](https://github.com/murat-akpinar/ratodo/commit/4a092eab7a82987273ace0f8d486f6864fb3d47d))
- *(ui)* Let the caret move through the line being typed ([c9f7aba](https://github.com/murat-akpinar/ratodo/commit/c9f7aba3797ac44dc2b897f1440a025c78539474))
The input field only ever appended: backspace took the last character and there was no way to reach any other one, so a typo four words back meant retyping four words. The caret now moves with the arrows, home and end, inserts and deletes where it stands, and the field scrolls with the caret rather than with the end of the line.
- *(ui)* Stop calling a finished task late ([55affb0](https://github.com/murat-akpinar/ratodo/commit/55affb0598cc4ffc376b5d76b6330db999ab4b1f))
A ticked line showing "1d ago" states something that stopped being true when the box was ticked, and contradicts the counts, which already leave finished work out of overdue. It falls through to the plain date instead. The task keeps its place in OVERDUE: membership there is positional.
- *(ui)* Take the ascii fallback into the help overlay ([de11fa8](https://github.com/murat-akpinar/ratodo/commit/de11fa898d9412094c62a4bf176f0af3edc07a73))
The overlay had the arrows and the enter symbol written into its key list as literals, so the one screen somebody opens because they are lost was the one screen the fallback did not reach. The buffer test never caught it because it does not open the overlay; it now opens the overlay, the input and its preview together and asserts the whole buffer is ASCII.

### 🚜 Refactor

- *(parse)* Drop line_no, which nothing read and nothing kept true ([53ce4ed](https://github.com/murat-akpinar/ratodo/commit/53ce4ed00eda86d57936296f0d2ecdfbcbd1ae23))

### 📚 Documentation

- Restructure the design record into a docs/ directory ([81aba9d](https://github.com/murat-akpinar/ratodo/commit/81aba9d1ee5bf9700ed26ebbbfd334abe2c3ad40))
Split the single 25 KB decision record into eleven focused documents under docs/, one question per file, and translate everything to English.
- *(ui)* Design every screen and settle the keymap ([4e6bef8](https://github.com/murat-akpinar/ratodo/commit/4e6bef82ab8479a27072dbb1ae284bf76a89503a))
The design record had one screen sketch and a seven-key table. Neither covered what the tool actually has to do while someone works next to it.
- Record what two audience design reviews found ([96d0a2c](https://github.com/murat-akpinar/ratodo/commit/96d0a2cdb9d64dcfe5a665061cced58ecc226454))
Two reviews were run against the design, one from each profile this tool claims to be for: a tiling-WM ricer and a terminal-bound developer. They ran separately and reached the same objection, which is why it is worth recording rather than arguing with.
- Point the readme at the command line that now exists ([f5e63b3](https://github.com/murat-akpinar/ratodo/commit/f5e63b36cdbc54e4547227e676370b79bc7704fc))
- Bring the readme and the record up to the v0.1.0 tag ([f067e5a](https://github.com/murat-akpinar/ratodo/commit/f067e5af2dc003f3893f752a6b8258fbbcc82af8))
The status blocks still said design phase and unreleased, the agenda in the readme was a hand-drawn mock that had drifted from the real screen, and the line claiming nothing pops over the list stopped being true when the input became a box. Install builds from the tag.

### 🧪 Testing

- Close the holes mutation testing found ([2809f2a](https://github.com/murat-akpinar/ratodo/commit/2809f2ac2e8e76a52be20e8885a5ba4e5bd87a3f))
The suite was green and partly decorative. `cargo mutants` breaks the source one edit at a time and checks that something goes red; on the first run 34 of 180 mutants survived, meaning 34 ways to break ratodo that no test objected to. It is now 0.
- *(ui)* Drive the help key through a terminal ([0818b40](https://github.com/murat-akpinar/ratodo/commit/0818b40b6ce0f6887884ac7ddb96ce1a600ba3e2))
The last missed mutant in the sweep: `helping = !helping` turned into `helping = helping` and nothing objected. The overlay itself was tested from both ends — the drawing one level down, the keymap one level up — and the wire between them was not.

### ⚙️ Miscellaneous Tasks

- Add agent working rules and a Rust gitignore ([6449064](https://github.com/murat-akpinar/ratodo/commit/644906428c4fa86dd1c0c64352274950cff90de5))
CLAUDE.md carries the workflow (write, self-review, test, commit, changelog, push), the conventional-commit format cliff.toml requires, and the nine hard invariants that must not be broken without an explicit decision — round-trip fidelity first among them.
- Keep changelog commits out of the changelog ([41aabd3](https://github.com/murat-akpinar/ratodo/commit/41aabd378b79b68a147e6c8b4b293d20877937d3))
- *(cli)* Settle the colour gate and trim the published crate ([551eb14](https://github.com/murat-akpinar/ratodo/commit/551eb14023586953f86669150b3d94c55d488728))
Two of the three things left before a tag.
