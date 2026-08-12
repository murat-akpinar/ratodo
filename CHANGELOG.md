## [0.7.2] - 2026-08-12

### 🐛 Bug Fixes

- *(write)* Flatten the backup name to what a filesystem accepts ([942547d](https://github.com/murat-akpinar/ratodo/commit/942547d3782f62f319faf4dc008188bc40749d53))
canonicalize on Windows answers with a verbatim path, so the slug kept the ? and the : that NTFS refuses. Every write past the first — which is the first that takes a backup — died on os error 123, and in the TUI that unwound out of the program on the first key that changed anything.
- *(ui)* Read altgr as a layout rather than a ctrl chord ([975fb94](https://github.com/murat-akpinar/ratodo/commit/975fb940752585e0cb302fc6745270c429741b93))
Windows reports AltGr as ctrl+alt, and on the Turkish, German and Polish layouts AltGr is how # @ and $ are typed at all. Dropping them as chords left the three characters the syntax is made of untypeable in the capture bar, and made altgr-c quit the program.
- *(docs)* Compare the skip prefix on a normalised separator ([8c1b6f3](https://github.com/murat-akpinar/ratodo/commit/8c1b6f3d454bdecb88c863d378f210f7411b647c))

### 📚 Documentation

- Name the windows paths and the xdg test decision ([cfe55b1](https://github.com/murat-akpinar/ratodo/commit/cfe55b1d5a6977fbd6816ee81245720150365859))

### 🧪 Testing

- *(cli)* Gate the read-only directory case on unix ([7e30cdf](https://github.com/murat-akpinar/ratodo/commit/7e30cdf4d5c072663a18ac8e5b5c5e50d3e28dca))

### ⚙️ Miscellaneous Tasks

- Point the PKGBUILD at v0.7.1 ([24958c1](https://github.com/murat-akpinar/ratodo/commit/24958c1ffb2f19c45d51eed31bc910c36ae2ab6a))
- *(release)* V0.7.2 ([1c9a99b](https://github.com/murat-akpinar/ratodo/commit/1c9a99b5a9f85f624b9a50aea407e002b0f63134))
## [0.7.1] - 2026-08-11

### 📚 Documentation

- Stop a release for a human look before cargo publish ([063c924](https://github.com/murat-akpinar/ratodo/commit/063c9242b130cbbf210c711d5092328bfcb87e84))

### 🎨 Styling

- *(ui)* Light the box labels by weight, and put them in upper case ([6fa065b](https://github.com/murat-akpinar/ratodo/commit/6fa065bfb619f331cef26ecb9a47a955ce88a250))
`ADD`, `EDIT` and `PUT OFF` are `foreground` and bold now - full brightness against the dim caret beside them - and `COPY` keeps the accent, because `COPY` is the only one with news. Lit, without a seventh meaning on the screen: a thirteenth theme role was the alternative, and the limit was never the palette but the other five built-ins and the reader.
- *(ui)* A finished row keeps its priority colour ([aaf7f5e](https://github.com/murat-akpinar/ratodo/commit/aaf7f5ee7b10e1561c08f88b6d2eba17b15def57))
`!med` on a ticked task went grey, which read as the colour having failed rather than as the task being done - and it sat next to an open `!high` in the same group, so it read as a bug.

### ⚙️ Miscellaneous Tasks

- Point the PKGBUILD at v0.7.0 ([1801138](https://github.com/murat-akpinar/ratodo/commit/1801138859cbe09a6a533d433d5e78edd5a30956))
- *(release)* V0.7.1 ([e5124a7](https://github.com/murat-akpinar/ratodo/commit/e5124a755501d3c4dc0c09fd503f4ed626307374))
## [0.7.0] - 2026-08-11

### 🚀 Features

- *(theme)* One colour, one job - and a role of its own for the priority ([a3956e1](https://github.com/murat-akpinar/ratodo/commit/a3956e19b9295e474eab74f5c54fab5f04fd4c22))
Mauve had become the answer to six questions: the group headings, the input box border, the focused date cell, the keys in `?`, and - both added today - the priority and every box label. A colour that answers two questions answers neither, and the screen read as noise.

### 🎨 Styling

- *(ui)* Draw the priority in the accent, in two weights ([571577f](https://github.com/murat-akpinar/ratodo/commit/571577f5316b679195a9e3c267ae9b34436c6c4d))
`!high` is the accent and bold, `!med` the accent, `!low` stays dim. Three levels the eye can sort without reading them, which is what the column is for.

### ⚙️ Miscellaneous Tasks

- Point the PKGBUILD at v0.6.0 ([596d654](https://github.com/murat-akpinar/ratodo/commit/596d654e56cb8f190d906090dc0d3622edfb4195))
- *(release)* V0.7.0 ([7dedb1f](https://github.com/murat-akpinar/ratodo/commit/7dedb1f8d74c9d8afc8c2c7030f47aac45b2dc2e))
## [0.6.0] - 2026-08-11

### 🚀 Features

- *(ui)* Open the input box on today's date ([c57e559](https://github.com/murat-akpinar/ratodo/commit/c57e5597e0d053dc4d3293465b34382b7c26fbb6))
`a` gave you an empty line, and the date was the one field the tool can guess right most of the time. The box now opens with `@today ` in it and the caret after it, one backspace from gone.

### 🐛 Bug Fixes

- *(ui)* Put the opening date behind the caret, not in front of it ([61453d2](https://github.com/murat-akpinar/ratodo/commit/61453d2b8b43b31bfe97ea9817fd8c7b74c638bb))
The box opened on `@2026-08-12 ` with the caret after it, so the date the tool guessed was the leftmost thing in it and the title the user came to type went second. It is now ` @2026-08-12` with the caret at the front: the title goes where the written line has it and where the row on the screen reads it.

### 📚 Documentation

- Record a demo gif for the readme ([b392334](https://github.com/murat-akpinar/ratodo/commit/b39233479513e7e649519f32c8db5a14ac3f21a4))
scripts/demo.py drives a release build on a pty inside one throwaway kitty window and lets menyoki record it, against a throwaway XDG tree so the real ~/.config/ratodo is never in scope. The window size is the compositor's call under a tiling WM, so the pty is sized from the window rather than the other way round, and the session ends by closing the pty rather than with q — a quit tears the alternate screen down, and a bare prompt is the frame a looping gif rests on.
- Put !high back in the readme's agenda mockup ([b5dcca8](https://github.com/murat-akpinar/ratodo/commit/b5dcca821820870b8c01c78a241d982ae30422d0))
Rendered the mockup's own scenario through TestBackend at its own 62 columns: the overdue row carries `!high` between the date and the tags and the mockup did not, and the date field opened on `[11]` where the page's today is the 10th. Everything else — the frame, the progress bar, the hint bar at that width, both input boxes — came back byte-identical.

### 🎨 Styling

- *(ui)* Light every input box label, not only copy ([9cbe9d0](https://github.com/murat-akpinar/ratodo/commit/9cbe9d0f2a3fade342312ca3be8cab9f94602a66))
The box is a mode and the label is what names it, so a mode whose name is the dimmest thing in it is a mode you have to look for. `copy` keeps its job on the word alone, which is the only thing that ever carried the meaning - the colour only ever made somebody glance.

### ⚙️ Miscellaneous Tasks

- Point the PKGBUILD at v0.5.0 ([ed2a068](https://github.com/murat-akpinar/ratodo/commit/ed2a068ba862a7aec3dafb93aaab1095381478d7))
- *(release)* V0.6.0 ([4971fe6](https://github.com/murat-akpinar/ratodo/commit/4971fe63ce76ffb4ff8a3fc4444f61d14d56eaba))
## [0.5.0] - 2026-08-11

### 🚀 Features

- *(ui)* Rule the columns, and say copy on the copy box ([d145d87](https://github.com/murat-akpinar/ratodo/commit/d145d875fa081aa83f4739b872316097801f1ad4))
Both out of the same report: the row read as one run-on line, and the box `y` fills looked exactly like the box `a` fills.

### ⚙️ Miscellaneous Tasks

- Point the PKGBUILD at v0.4.0 ([2cbd2b0](https://github.com/murat-akpinar/ratodo/commit/2cbd2b06bea68646a62b6c6150f850d966bf0129))
- *(release)* V0.5.0 ([9a4b816](https://github.com/murat-akpinar/ratodo/commit/9a4b8169e7b82198140baacdccec360030e856d2))
## [0.4.0] - 2026-08-11

### 🚀 Features

- *(ui)* A date field on tab ([e9ff077](https://github.com/murat-akpinar/ratodo/commit/e9ff0777df2b0ac473fe86abc97eea121ec9b3a3))
`@2026-13-45` is a date the text box takes and the preview can only say is wrong. `tab` opens a field where that date does not exist: `↑ ↓` on the part in brackets, `← →` between the three, and digits filling them in order, so `13082026` is the 13th of August in eight keystrokes — a part that cannot take another digit hands the cursor on by itself.

### ⚙️ Miscellaneous Tasks

- Add a flake and an Arch PKGBUILD ([ccf4d16](https://github.com/murat-akpinar/ratodo/commit/ccf4d163f5096928a051580c5259c3608aabc974))
Two ways in that are not `cargo install`, which is what NixOS and Arch users were left with after v0.3.0.
- *(release)* V0.4.0 ([f4e40ea](https://github.com/murat-akpinar/ratodo/commit/f4e40ea9c184a1358b97c476478601166facee63))
## [0.3.0] - 2026-08-11

### 🚀 Features

- *(ui)* [**breaking**] Swap the delete and cancel keys ([e1e5b41](https://github.com/murat-akpinar/ratodo/commit/e1e5b41b9b829b63dffeef2ab58a160c94d129c3))
`d` cancels and `X` deletes, the other way round from v0.2.0. The shift was on the wrong key: cancelling is reversible — `d` again takes it back and the row stays in the file as `- [-]` — while deleting takes a line out of the user's file behind one level of undo that a `q` spends. The key that costs the most is the one that should cost a shift, and `d` sitting a row from `j` and `k` made the cheap key the destructive one.
- *(ui)* Say so when an @ can never be a date ([fe648c4](https://github.com/murat-akpinar/ratodo/commit/fe648c41e49faa4dd7e517084256198963dc43be))
`@2026-13-45` resolves to nothing, so the word falls back to being part of the title. The fallback stays — a word we did not understand belongs to the user — but it was silent, and the preview went quiet in exactly the moment it should speak.
- *(ui)* Copy the selected task with y ([4e0e6fd](https://github.com/murat-akpinar/ratodo/commit/4e0e6fd709e617eb06193c9fb0a25a85f17d707a))
A task that is nearly one already on the list had no way in but `a` and the whole line again. `y` opens the input box pre-filled with the task under the cursor, as a new one: edit it, `⏎` saves a second task, `esc` writes nothing.
- *(cli)* Route a capture with $list ([203f98b](https://github.com/murat-akpinar/ratodo/commit/203f98b600a0eb20e2dd3f7db0fc5c66579782a3))
`a` wrote to todo.md and nothing else, so capturing into work.md meant leaving the TUI. `$work` in the sentence sends that one capture to work.md — a fourth sigil beside @ # !, read by the same capture::parts, and previewed as `→ work.md` the way `@thu` is previewed as a date.
- *(ui)* Name the sigils in an empty input box ([1cc0db1](https://github.com/murat-akpinar/ratodo/commit/1cc0db16a08deb87911d029d09791e59d070fea8))
An empty `a` box drew a blank preview row while the empty `p` box has always said `how long? 2 3d 1w fri`. It now reads `@thu #home !high $list` in the dim, by example rather than by name — the syntax and the hint in one word each, which is what the preview already does for anything typed. It goes the moment there is something to report, and `$list` appears only when there is more than one list to address.

### 🐛 Bug Fixes

- *(ui)* Fit y copy on the hint bar at eighty columns ([1322b1d](https://github.com/murat-akpinar/ratodo/commit/1322b1dc6ace46f3942ee55bed4f1af175b29cb9))
The bar takes keys until one does not fit, so `y copy` needed 81 columns and eighty is the width a terminal opens at unless somebody moved it. The newest key was invisible at the one width most people run.
- *(ui)* Stop a length of time at a year ([34edf79](https://github.com/murat-akpinar/ratodo/commit/34edf79e1036c24e73f51716180a8eb1c0e7df55))
Reported from use: a keyboard that stutters turns `22` into `2222` in the `p` box, and both are perfectly good arithmetic — twenty-two days and six years — so the file took the second one without a word.

### 📚 Documentation

- Put what is left at the top of todo.md ([bb63dcb](https://github.com/murat-akpinar/ratodo/commit/bb63dcb23567cdef37025ca82ff00ab96055ab78))
- Record the swapped keys and the date-entry complaint ([853092b](https://github.com/murat-akpinar/ratodo/commit/853092b87394b2f002a280c054d12777d6e4ce82))
todo.md and notes.md still had `d` deleting. The build record keeps what shipped, with the swap noted where it happened rather than rewritten out.
- Write up $list and the four-field input before building either ([0c88832](https://github.com/murat-akpinar/ratodo/commit/0c888323141dc5c13edf47afb29d8e5a53c1435f))
Asked for as one sentence and it is two pieces: `$work` routing a capture, and the input box split into fields with `tab` between them. The second reverses the decision of 2026-08-11, so it gets the reversal written in decisions.md before a line of it exists.

### ⚙️ Miscellaneous Tasks

- *(release)* V0.3.0 ([5971f75](https://github.com/murat-akpinar/ratodo/commit/5971f757f6045b2d76f85525a90ebd80e0d23623))
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
