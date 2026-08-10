## [unreleased]

### 🚀 Features

- *(parse)* Read and write todo.md without disturbing it ([24b15aa](https://github.com/murat-akpinar/ratodo/commit/24b15aab8b996a43926ae106a09386bc0e06c450))
The pure core of the product, with no terminal involved: parse, write, capture, and the two tests that the whole design rests on.

### 🐛 Bug Fixes

- *(write)* Keep symlinks, file modes and the terminal safe ([cacac46](https://github.com/murat-akpinar/ratodo/commit/cacac4633669ff32ac492c449950f082162eac30))
A review of the write path turned up two defects that would have bitten exactly the audience this tool is for, plus two smaller ones.
- *(write)* Keep the backup out of the user's dotfiles, and the capture inside their sections ([83fca6d](https://github.com/murat-akpinar/ratodo/commit/83fca6d6e565a30b828ba91c7b979e18963e8c77))
Four defects that two audience design reviews turned up in code that was already pushed. Each one is small; each one would have been noticed by exactly the person this tool is for.

### 📚 Documentation

- Restructure the design record into a docs/ directory ([81aba9d](https://github.com/murat-akpinar/ratodo/commit/81aba9d1ee5bf9700ed26ebbbfd334abe2c3ad40))
Split the single 25 KB decision record into eleven focused documents under docs/, one question per file, and translate everything to English.
- *(ui)* Design every screen and settle the keymap ([4e6bef8](https://github.com/murat-akpinar/ratodo/commit/4e6bef82ab8479a27072dbb1ae284bf76a89503a))
The design record had one screen sketch and a seven-key table. Neither covered what the tool actually has to do while someone works next to it.
- Record what two audience design reviews found ([96d0a2c](https://github.com/murat-akpinar/ratodo/commit/96d0a2cdb9d64dcfe5a665061cced58ecc226454))
Two reviews were run against the design, one from each profile this tool claims to be for: a tiling-WM ricer and a terminal-bound developer. They ran separately and reached the same objection, which is why it is worth recording rather than arguing with.

### 🧪 Testing

- Close the holes mutation testing found ([2809f2a](https://github.com/murat-akpinar/ratodo/commit/2809f2ac2e8e76a52be20e8885a5ba4e5bd87a3f))
The suite was green and partly decorative. `cargo mutants` breaks the source one edit at a time and checks that something goes red; on the first run 34 of 180 mutants survived, meaning 34 ways to break ratodo that no test objected to. It is now 0.

### ⚙️ Miscellaneous Tasks

- Add agent working rules and a Rust gitignore ([6449064](https://github.com/murat-akpinar/ratodo/commit/644906428c4fa86dd1c0c64352274950cff90de5))
CLAUDE.md carries the workflow (write, self-review, test, commit, changelog, push), the conventional-commit format cliff.toml requires, and the nine hard invariants that must not be broken without an explicit decision — round-trip fidelity first among them.
- Keep changelog commits out of the changelog ([41aabd3](https://github.com/murat-akpinar/ratodo/commit/41aabd378b79b68a147e6c8b4b293d20877937d3))
