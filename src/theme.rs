//! Colours, the built-in themes and the `theme.conf` parser. See docs/theming.md.

use ratatui::style::Color;

/// Twelve roles, named after what they do rather than after any one palette.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Theme {
    pub background: Color,
    pub foreground: Color,
    pub dim: Color,
    pub border: Color,
    pub selection: Color,
    pub accent: Color,
    pub overdue: Color,
    pub today: Color,
    pub done: Color,
    pub done_text: Color,
    pub tag: Color,
    pub priority: Color,
}

/// A role's name paired with the way to reach it.
type Role = (&'static str, fn(&mut Theme) -> &mut Color);

/// The one list. The parser, `dump` and the "unknown key" warning all read it,
/// so a twelfth role cannot be added to the struct and forgotten in two places.
const KEYS: [Role; 12] = [
    ("background", |t| &mut t.background),
    ("foreground", |t| &mut t.foreground),
    ("dim", |t| &mut t.dim),
    ("border", |t| &mut t.border),
    ("selection", |t| &mut t.selection),
    ("accent", |t| &mut t.accent),
    ("overdue", |t| &mut t.overdue),
    ("today", |t| &mut t.today),
    ("done", |t| &mut t.done),
    ("done_text", |t| &mut t.done_text),
    ("tag", |t| &mut t.tag),
    ("priority", |t| &mut t.priority),
];

impl Theme {
    /// Every role in file order, for `ratodo theme dump`.
    pub fn pairs(mut self) -> Vec<(&'static str, Color)> {
        KEYS.iter()
            .map(|(name, at)| (*name, *at(&mut self)))
            .collect()
    }

    fn set(&mut self, key: &str, value: Color) -> bool {
        match KEYS.iter().find(|(name, _)| *name == key) {
            Some((_, at)) => {
                *at(self) = value;
                true
            }
            None => false,
        }
    }

    /// `NO_COLOR=1`. The `○ ✓ !` symbols carry the meaning on their own — see
    /// docs/design.md#rules — which is what makes this cheap enough to honour.
    pub fn plain() -> Self {
        let mut theme = MOCHA;
        for (_, at) in &KEYS {
            *at(&mut theme) = Color::Reset;
        }
        theme
    }

    /// A `theme.conf` this theme would parse back to. `dump` exists so nobody
    /// starts from an empty file.
    pub fn dump(self) -> String {
        let mut out = String::from("# ratodo theme — see docs/theming.md\n\n");
        for (name, colour) in self.pairs() {
            out.push_str(&format!("{name:<11}= {}\n", write_colour(colour)));
        }
        out
    }
}

const fn rgb(hex: u32) -> Color {
    Color::Rgb(
        ((hex >> 16) & 0xff) as u8,
        ((hex >> 8) & 0xff) as u8,
        (hex & 0xff) as u8,
    )
}

// -- ALAN START: built-in themes --
// Every one ships `background: Color::Reset`. A translucent terminal is normal
// for this audience, and a tool that paints an opaque rectangle into it gets
// closed rather than configured — docs/theming.md.

pub const MOCHA: Theme = Theme {
    background: Color::Reset,
    foreground: rgb(0xcdd6f4),
    dim: rgb(0xa6adc8),
    border: rgb(0x6c7086),
    selection: rgb(0x313244),
    accent: rgb(0xcba6f7),
    overdue: rgb(0xf38ba8),
    today: rgb(0xfab387),
    done: rgb(0xa6e3a1),
    done_text: rgb(0x7f849c),
    tag: rgb(0x89b4fa),
    priority: rgb(0xf9e2af),
};

const LATTE: Theme = Theme {
    background: Color::Reset,
    foreground: rgb(0x4c4f69),
    dim: rgb(0x6c6f85),
    border: rgb(0x9ca0b0),
    selection: rgb(0xccd0da),
    accent: rgb(0x8839ef),
    overdue: rgb(0xd20f39),
    today: rgb(0xfe640b),
    done: rgb(0x40a02b),
    done_text: rgb(0x8c8fa1),
    tag: rgb(0x1e66f5),
    priority: rgb(0xdf8e1d),
};

const GRUVBOX_DARK: Theme = Theme {
    background: Color::Reset,
    foreground: rgb(0xebdbb2),
    dim: rgb(0xa89984),
    border: rgb(0x504945),
    selection: rgb(0x3c3836),
    accent: rgb(0xd3869b),
    overdue: rgb(0xfb4934),
    today: rgb(0xfe8019),
    done: rgb(0xb8bb26),
    done_text: rgb(0x928374),
    tag: rgb(0x83a598),
    priority: rgb(0xfabd2f),
};

const NORD: Theme = Theme {
    background: Color::Reset,
    foreground: rgb(0xd8dee9),
    dim: rgb(0x9aa5b6),
    border: rgb(0x4c566a),
    selection: rgb(0x3b4252),
    accent: rgb(0x88c0d0),
    overdue: rgb(0xbf616a),
    today: rgb(0xd08770),
    done: rgb(0xa3be8c),
    done_text: rgb(0x616e88),
    tag: rgb(0x81a1c1),
    priority: rgb(0xebcb8b),
};

const DRACULA: Theme = Theme {
    background: Color::Reset,
    foreground: rgb(0xf8f8f2),
    dim: rgb(0x9aa4c8),
    border: rgb(0x44475a),
    selection: rgb(0x44475a),
    accent: rgb(0xbd93f9),
    overdue: rgb(0xff5555),
    today: rgb(0xffb86c),
    done: rgb(0x50fa7b),
    done_text: rgb(0x6272a4),
    tag: rgb(0x8be9fd),
    priority: rgb(0xf1fa8c),
};

/// ANSI 0–15 only, so every colour comes from the terminal's own palette. This
/// is the pywal / wallust / base16 answer — set it once and ratodo re-themes
/// itself whenever the wallpaper does — and also the one that works on a bare
/// TTY with no truecolor.
const TERMINAL: Theme = Theme {
    background: Color::Reset,
    foreground: Color::Indexed(7),
    dim: Color::Indexed(8),
    border: Color::Indexed(8),
    selection: Color::Indexed(8),
    accent: Color::Indexed(5),
    overdue: Color::Indexed(1),
    today: Color::Indexed(3),
    done: Color::Indexed(2),
    done_text: Color::Indexed(8),
    tag: Color::Indexed(4),
    priority: Color::Indexed(6),
};

pub const BUILT_IN: [(&str, Theme); 6] = [
    ("catppuccin-mocha", MOCHA),
    ("catppuccin-latte", LATTE),
    ("gruvbox-dark", GRUVBOX_DARK),
    ("nord", NORD),
    ("dracula", DRACULA),
    ("terminal", TERMINAL),
];
// -- ALAN END --

pub fn built_in(name: &str) -> Option<Theme> {
    BUILT_IN
        .iter()
        .find(|(known, _)| *known == name)
        .map(|(_, theme)| *theme)
}

/// The eight ANSI names, in index order, then their bright forms.
const ANSI: [&str; 8] = [
    "black", "red", "green", "yellow", "blue", "magenta", "cyan", "white",
];

/// `none`, `#rrggbb`, `#rgb`, an index 0–15, or an ANSI name. See
/// docs/theming.md#value-forms.
pub fn parse_colour(text: &str) -> Option<Color> {
    let text = text.trim();

    if text.eq_ignore_ascii_case("none") || text.eq_ignore_ascii_case("default") {
        return Some(Color::Reset);
    }

    if let Some(hex) = text.strip_prefix('#') {
        let expanded = match hex.len() {
            // `#c9f` is `#ccaa99`-style: each digit doubled, not padded with a
            // zero, or half the shorthand palette comes out darker than it looks.
            3 => hex.chars().flat_map(|c| [c, c]).collect(),
            6 => hex.to_string(),
            _ => return None,
        };
        let value = u32::from_str_radix(&expanded, 16).ok()?;
        return Some(rgb(value));
    }

    // An index resolves through the terminal's own palette, which is the point:
    // it follows whatever the user already themed.
    if let Ok(index) = text.parse::<u8>() {
        return (index <= 15).then_some(Color::Indexed(index));
    }

    let (name, offset) = match text.strip_prefix("bright_") {
        Some(rest) => (rest, 8),
        None => (text, 0),
    };
    let index = ANSI.iter().position(|known| *known == name)?;
    Some(Color::Indexed(index as u8 + offset))
}

fn write_colour(colour: Color) -> String {
    match colour {
        Color::Reset => "none".to_string(),
        Color::Rgb(r, g, b) => format!("#{r:02x}{g:02x}{b:02x}"),
        Color::Indexed(i) if i < 8 => ANSI[i as usize].to_string(),
        Color::Indexed(i) if i < 16 => format!("bright_{}", ANSI[(i - 8) as usize]),
        Color::Indexed(i) => i.to_string(),
        other => format!("{other:?}").to_lowercase(),
    }
}

/// What a `theme.conf` said, and everything wrong with it.
///
/// Warnings are returned rather than printed: this module has no terminal, and
/// **a broken theme file must never stop the program from starting** — the
/// caller decides where to put the complaints.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parsed {
    pub theme: Theme,
    pub warnings: Vec<String>,
}

/// `key = value` per line; a `#` in the first column is a comment and anywhere
/// else it is part of a colour.
///
/// `theme = <name>` is applied **before** the individual keys whatever line it
/// sits on. Reading it in order would mean a `theme =` at the bottom of the file
/// silently undoing every override above it.
pub fn parse(text: &str) -> Parsed {
    let mut warnings = Vec::new();
    let entries: Vec<(usize, &str, &str)> = text
        .lines()
        .enumerate()
        .filter_map(|(i, raw)| {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            match line.split_once('=') {
                Some((key, value)) => Some((i + 1, key.trim(), value.trim())),
                None => {
                    warnings.push(format!("line {}: expected `key = value`", i + 1));
                    None
                }
            }
        })
        .collect();

    let mut theme = MOCHA;
    for (no, _, value) in entries.iter().filter(|(_, key, _)| *key == "theme") {
        match built_in(value) {
            Some(base) => theme = base,
            None => warnings.push(format!("line {no}: no built-in theme called `{value}`")),
        }
    }

    for (no, key, value) in entries.iter().filter(|(_, key, _)| *key != "theme") {
        let Some(colour) = parse_colour(value) else {
            warnings.push(format!("line {no}: `{value}` is not a colour"));
            continue;
        };
        if !theme.set(key, colour) {
            warnings.push(format!("line {no}: no theme key called `{key}`"));
        }
    }

    Parsed { theme, warnings }
}

/// The whole precedence chain from docs/theming.md#precedence, in one place:
/// built-in default → `theme =` → individual keys → `--theme` → `NO_COLOR`.
pub fn resolve(config: Option<&str>, flag: Option<&str>, no_colour: bool) -> Parsed {
    let mut parsed = config.map_or(
        Parsed {
            theme: MOCHA,
            warnings: Vec::new(),
        },
        parse,
    );

    if let Some(name) = flag {
        match built_in(name) {
            // Wholesale, per the documented order: `--theme` sits below only
            // NO_COLOR, so it wins over the file's individual keys too.
            Some(theme) => parsed.theme = theme,
            None => parsed
                .warnings
                .push(format!("no built-in theme called `{name}`")),
        }
    }

    if no_colour {
        parsed.theme = Theme::plain();
    }
    parsed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_built_in_leaves_the_background_alone() {
        for (name, theme) in BUILT_IN {
            assert_eq!(
                theme.background,
                Color::Reset,
                "{name} paints over a translucent terminal"
            );
        }
    }

    /// Twelve roles, and every built-in fills all of them. A theme with a role
    /// left at the default is how one palette leaks into another.
    #[test]
    fn every_built_in_sets_every_other_role() {
        assert_eq!(KEYS.len(), 12);
        for (name, theme) in BUILT_IN {
            for (key, colour) in theme.pairs() {
                if key == "background" {
                    continue;
                }
                assert_ne!(colour, Color::Reset, "{name}.{key} was never set");
            }
        }
    }

    #[test]
    fn the_terminal_theme_uses_nothing_but_the_terminals_own_palette() {
        for (key, colour) in built_in("terminal").unwrap().pairs() {
            match colour {
                Color::Reset => {}
                Color::Indexed(i) => assert!(i <= 15, "{key} is outside ANSI 0-15"),
                other => panic!("{key} is {other:?}, which needs truecolor"),
            }
        }
    }

    #[test]
    fn the_documented_names_are_the_ones_that_resolve() {
        for (name, _) in BUILT_IN {
            assert!(built_in(name).is_some(), "{name}");
        }
        assert!(built_in("solarized").is_none());
        assert!(built_in("").is_none());
    }

    #[test]
    fn the_value_forms() {
        assert_eq!(parse_colour("#cba6f7"), Some(Color::Rgb(0xcb, 0xa6, 0xf7)));
        assert_eq!(parse_colour("none"), Some(Color::Reset));
        assert_eq!(parse_colour("default"), Some(Color::Reset));
        assert_eq!(parse_colour("4"), Some(Color::Indexed(4)));
        assert_eq!(parse_colour("15"), Some(Color::Indexed(15)));
        assert_eq!(parse_colour("blue"), Some(Color::Indexed(4)));
        assert_eq!(parse_colour("bright_black"), Some(Color::Indexed(8)));
        assert_eq!(parse_colour("  #cba6f7  "), Some(Color::Rgb(203, 166, 247)));
    }

    /// `#c9f` doubles each digit. Padding with a zero instead would make every
    /// shorthand colour darker than the one the user picked.
    #[test]
    fn short_hex_doubles_each_digit() {
        assert_eq!(parse_colour("#c9f"), Some(Color::Rgb(0xcc, 0x99, 0xff)));
        assert_eq!(parse_colour("#fff"), Some(Color::Rgb(255, 255, 255)));
        assert_eq!(parse_colour("#000"), Some(Color::Rgb(0, 0, 0)));
    }

    #[test]
    fn what_is_not_a_colour() {
        for text in [
            "",
            "#",
            "#12",
            "#12345",
            "#1234567",
            "#gggggg",
            "16",
            "256",
            "-1",
            "puce",
            "bright_puce",
            "bright_",
            "0x4",
        ] {
            assert_eq!(parse_colour(text), None, "{text:?} was accepted");
        }
    }

    #[test]
    fn a_file_sets_what_it_names_and_leaves_the_rest() {
        let parsed = parse("accent = #ff0000\ntag = blue\n");
        assert!(parsed.warnings.is_empty(), "{:?}", parsed.warnings);
        assert_eq!(parsed.theme.accent, Color::Rgb(255, 0, 0));
        assert_eq!(parsed.theme.tag, Color::Indexed(4));
        assert_eq!(parsed.theme.foreground, MOCHA.foreground);
    }

    #[test]
    fn a_hash_starts_a_comment_only_at_the_start_of_a_line() {
        let parsed = parse("  # a comment\n\naccent = #cba6f7\n");
        assert!(parsed.warnings.is_empty(), "{:?}", parsed.warnings);
        assert_eq!(parsed.theme.accent, Color::Rgb(0xcb, 0xa6, 0xf7));
    }

    #[test]
    fn a_named_theme_becomes_the_base_and_keys_override_it() {
        let parsed = parse("theme = nord\naccent = #ff0000\n");
        assert!(parsed.warnings.is_empty(), "{:?}", parsed.warnings);
        assert_eq!(parsed.theme.accent, Color::Rgb(255, 0, 0));
        assert_eq!(parsed.theme.overdue, NORD.overdue, "the rest is still nord");
    }

    /// Read in order, a `theme =` at the bottom would quietly undo every
    /// override above it — and a config file that punishes line order is a bug
    /// report nobody can describe.
    #[test]
    fn a_named_theme_applies_first_wherever_it_is_written() {
        let top = parse("theme = nord\naccent = #ff0000\n");
        let bottom = parse("accent = #ff0000\ntheme = nord\n");
        assert_eq!(top.theme, bottom.theme);
    }

    /// The rule that outranks every other rule in this module.
    #[test]
    fn nothing_in_a_broken_file_stops_the_program() {
        let parsed = parse(
            "theme = nonsense\n\
             accent = puce\n\
             wibble = #ff0000\n\
             a line with no equals sign\n\
             tag = blue\n",
        );

        // Four complaints, one per broken line. The order they come out in is
        // not a contract, so this asks which ones rather than in what sequence.
        assert_eq!(parsed.warnings.len(), 4, "{:?}", parsed.warnings);
        for expected in [
            "no built-in theme",
            "not a colour",
            "no theme key",
            "key = value",
        ] {
            assert!(
                parsed.warnings.iter().any(|w| w.contains(expected)),
                "nothing said {expected:?}: {:?}",
                parsed.warnings
            );
        }

        assert_eq!(
            parsed.theme.tag,
            Color::Indexed(4),
            "a good line after four bad ones was dropped"
        );
        assert_eq!(parsed.theme.accent, MOCHA.accent, "a bad value fell back");
    }

    #[test]
    fn a_warning_says_which_line() {
        let parsed = parse("# comment\n\naccent = puce\n");
        assert!(
            parsed.warnings[0].starts_with("line 3:"),
            "{:?}",
            parsed.warnings
        );
    }

    #[test]
    fn no_file_at_all_is_the_default_theme() {
        let parsed = resolve(None, None, false);
        assert_eq!(parsed.theme, MOCHA);
        assert!(parsed.warnings.is_empty());
    }

    /// The order in docs/theming.md#precedence: `--theme` sits below only
    /// NO_COLOR, so it replaces the file's individual keys as well as its base.
    #[test]
    fn the_flag_outranks_the_file_and_no_colour_outranks_everything() {
        let config = "theme = nord\naccent = #ff0000\n";

        let file_only = resolve(Some(config), None, false);
        assert_eq!(file_only.theme.accent, Color::Rgb(255, 0, 0));

        let flagged = resolve(Some(config), Some("dracula"), false);
        assert_eq!(flagged.theme, DRACULA);

        let plain = resolve(Some(config), Some("dracula"), true);
        assert!(plain.theme.pairs().iter().all(|(_, c)| *c == Color::Reset));
    }

    #[test]
    fn an_unknown_theme_on_the_command_line_warns_and_keeps_going() {
        let parsed = resolve(None, Some("solarized"), false);
        assert_eq!(parsed.theme, MOCHA);
        assert_eq!(parsed.warnings.len(), 1);
        assert!(parsed.warnings[0].contains("solarized"));
    }

    /// `theme dump > theme.conf` has to produce a file that parses back to what
    /// was dumped, or the command is a trap.
    #[test]
    fn a_dump_parses_back_to_the_theme_it_came_from() {
        for (name, theme) in BUILT_IN {
            let round_tripped = parse(&theme.dump());
            assert!(
                round_tripped.warnings.is_empty(),
                "{name}: {:?}",
                round_tripped.warnings
            );
            assert_eq!(round_tripped.theme, theme, "{name} did not survive a dump");
        }
    }

    /// 0–15 is deliberate: those are the sixteen the user's own terminal theme
    /// defines, which is the whole reason for the form. The 256-colour cube
    /// above them is fixed and follows nobody's palette, so `parse_colour`
    /// refuses it — and a `Theme` built in code that carries one still has to
    /// dump as *something*. The number is the only honest answer, and it will
    /// not read back, which this asserts rather than hides.
    #[test]
    fn an_index_above_the_ansi_range_dumps_as_a_number_and_does_not_return() {
        assert_eq!(write_colour(Color::Indexed(0)), "black");
        assert_eq!(write_colour(Color::Indexed(7)), "white");
        assert_eq!(write_colour(Color::Indexed(8)), "bright_black");
        assert_eq!(write_colour(Color::Indexed(15)), "bright_white");
        assert_eq!(write_colour(Color::Indexed(16)), "16");
        assert_eq!(write_colour(Color::Indexed(200)), "200");

        assert_eq!(parse_colour("16"), None);
        assert_eq!(parse_colour("200"), None);
    }

    #[test]
    fn a_dump_names_every_key_and_reads_as_a_conf_file() {
        let text = MOCHA.dump();
        for (key, _) in KEYS {
            assert!(text.contains(&format!("{key:<11}=")), "{key} is missing");
        }
        assert!(text.starts_with("# "), "{text}");
        assert!(text.contains("background = none"), "{text}");
    }
}
