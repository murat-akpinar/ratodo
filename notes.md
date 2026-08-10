# Working notebook

> [docs/](docs/README.md) is the record of **decisions** — settled things.
> This file is **raw thinking**: loose ends, hunches, the idea graveyard.
> When something settles here it moves into `docs/` and gets deleted from here.

---

## The seed idea (2026-08-10)

Kept verbatim, in the original Turkish, because it is the historical record of
where this came from:

> - CLI rust ile Ratatui kütüphanesi ile bir todo, planlayıcı araç düşünüyorum.
> - Bir çok kişi vim açıp yazıp txt veya md kaydediyor düşüncem şu bu araç linux
>   kurduğumda i3 hyprland sway tarzı tiling kullanan kişiler için kolay olmasını
>   düşünyorum.
> - Terminal açıp komut girerek veya direkt vscode gibi code . diyip
>   açabilecekleri bir şekilde düşünüyorum. Böylece bir iş yaprken bir anda oraya
>   veri todo girebilecek.
> - .config/proje-ismi/dosyalar şeklinde tutmayı düşnüyüorum
> - todo listemde ki şeyler 1 dosyada tutmak istiyorum böylece kişi dotfile içine
>   github'a yedekleyebilri oradak çebilir değişikleri diğer pclerine secron
>   edebilir.
> - linux ortamında ki takvime entegre olabilmesi lazım

In English, and these six points are **the constitution of the design**. When a
feature is up for debate the question is: *which of these does it serve?* If none,
it is not in v1.

| # | The raw point | Where it landed |
|---|---|---|
| 1 | A todo/planner in Rust + ratatui | The foundation. One binary, offline |
| 2 | For i3 / Hyprland / sway users | The audience. The palette, the keymap and the v4 waybar module all come from here |
| 3 | Fast entry by command, without breaking flow | `ratodo add "..."` → writes, exits, no TUI. The reason the product exists |
| 4 | `.config/project-name/` | `~/.config/ratodo/todo.md`. A deliberate XDG deviation |
| 5 | One file, dotfiles, GitHub, several machines | A single Markdown file. **Sync is not the tool's job** — it is the user's git |
| 6 | Integrate with the Linux calendar | One-way `todo.ics` (VTODO). We make the file; subscribing is the user's job |

**Points 4 and 5 were in conflict**, and 5 won: by XDG, user data belongs in
`~/.local/share/`, but nobody puts that in their dotfiles.

---

## Hunches on the open questions

The questions themselves live in
[docs/decisions.md](docs/decisions.md#open-questions). These are the leanings,
which are not decisions:

- Completed tasks: **stay in place** in v1, `ratodo archive` in v2.
- Multiple lists: live with `--file` first and see whether it hurts.
- `.ics` regeneration: on every `add`. Simple, and the file is tiny.
- `* [ ]` and `+ [ ]`: recognise them when reading, always write `- [ ]`.

---

## Idea graveyard

Not in v1, but not thrown away either. Written down here so they stop circling
in my head and inflating v1.

- `ratodo status --json` → **a waybar / eww module.** "3 open · 1 overdue" in the
  bar. Probably the single biggest win for this audience. v4.
- Overdue notifications via `notify-send`. v4.
- `ratodo done "invoice"` — fuzzy match, mark done without opening the TUI.
  *(Made it into v1's command list.)*
- `ratodo log` — "what did I finish today". For people who write weekly reports.
- `ratodo undo` — restore the last change from `.bak`.
- Automatic git commit (a `--commit` flag). Tempting, but touching the user's git
  is dangerous even opt-in.
- A tmux popup / Hyprland scratchpad binding — an example config for the README.
- Picking tasks with fzf (`ratodo done $(ratodo list | fzf)`). Needs a
  `--porcelain` output format.
- `~2026-09-01` defer syntax (hide until this date). v3.
- Sharing themes as separate files under `~/.config/ratodo/themes/`. Wait until
  someone actually asks.
- An encrypted list — **no.** The file stays plain text; that is the whole logic
  of the product.

---

## Things to watch while building

- Column alignment with wide characters. `şğüöçİI` and 🚀 are in the fixtures on
  purpose — a TUI that counts bytes instead of display width will look broken.
- The first `git diff` after a real day of use. If completing one task moves more
  than one line, round-trip fidelity is already broken.
- Whether `ratodo add` really feels like two seconds. If it doesn't, the product
  has no reason to exist. Time it honestly.
