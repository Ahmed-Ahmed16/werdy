//! Checks that styles follow the token file (ADR 0024).

const TOKENS_MD: &str = include_str!("../../../docs/design/tokens.md");
const TOKENS_CSS: &str = include_str!("../assets/tokens.css");
const APP_CSS: &str = include_str!("../assets/app.css");

/// Reads `(name, light, dark)` rows from the colour table in tokens.md.
fn colour_tokens() -> Vec<(String, String, String)> {
    TOKENS_MD
        .split("## Colour")
        .nth(1)
        .and_then(|rest| rest.split("\n## ").next())
        .unwrap_or_default()
        .lines()
        .filter_map(|line| {
            let cells: Vec<&str> = line.split('|').map(str::trim).collect();
            let name = cells.get(1)?.strip_prefix('`')?.strip_suffix('`')?;
            Some((
                name.to_owned(),
                first_hex(cells.get(2)?)?,
                first_hex(cells.get(3)?)?,
            ))
        })
        .collect()
}

fn first_hex(cell: &str) -> Option<String> {
    let start = cell.find('#')?;
    let hex: String = cell[start..]
        .chars()
        .take_while(|c| *c == '#' || c.is_ascii_hexdigit())
        .collect();
    (hex.len() == 7).then_some(hex)
}

fn luminance(hex: &str) -> Option<f64> {
    let rgb = u32::from_str_radix(hex.strip_prefix('#')?, 16).ok()?;
    let channel = |shift| {
        let value = f64::from((rgb >> shift) & 0xff_u32) / 255.0;
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    Some(0.2126 * channel(16) + 0.7152 * channel(8) + 0.0722 * channel(0))
}

fn contrast(first: &str, second: &str) -> Option<f64> {
    let a = luminance(first)?;
    let b = luminance(second)?;
    Some((a.max(b) + 0.05) / (a.min(b) + 0.05))
}

#[test]
fn text_and_background_token_pairs_pass_contrast_in_both_themes() {
    use std::collections::BTreeMap;

    let tokens: BTreeMap<_, _> = colour_tokens()
        .into_iter()
        .map(|(name, light, dark)| (name, (light, dark)))
        .collect();
    let surfaces = ["bg", "surface", "surface-muted", "track", "primary-soft"];
    let mut pairs = Vec::new();
    for foreground in ["text", "text-muted", "primary"] {
        for background in surfaces {
            pairs.push((foreground, background));
        }
    }
    pairs.extend([
        ("on-primary", "primary"),
        ("on-mark-done", "mark-done"),
        ("on-mark-partial", "mark-partial"),
        ("on-mark-missed", "mark-missed"),
        ("danger", "bg"),
        ("danger", "surface"),
    ]);
    for (foreground, background) in pairs {
        let text = tokens.get(foreground).expect("text token exists");
        let ground = tokens.get(background).expect("background token exists");
        for (theme, first, second) in [("light", &text.0, &ground.0), ("dark", &text.1, &ground.1)]
        {
            let ratio = contrast(first, second).expect("valid hex colours");
            assert!(
                ratio >= 4.5,
                "{theme} {foreground} on {background}: {ratio:.2}"
            );
        }
    }
}

/// Returns raw colours written as hex, rgb or hsl.
fn raw_colours(css: &str) -> Vec<String> {
    let mut found = Vec::new();
    for (index, _) in css.match_indices('#') {
        let digits = css[index + 1..]
            .chars()
            .take_while(char::is_ascii_hexdigit)
            .count();
        if matches!(digits, 3 | 4 | 6 | 8) {
            found.push(css[index..index + 1 + digits].to_owned());
        }
    }
    for function in ["rgb(", "rgba(", "hsl(", "hsla("] {
        if css.contains(function) {
            found.push(function.to_owned());
        }
    }
    found
}

#[test]
fn every_colour_token_has_its_light_and_dark_value_in_css() {
    let tokens = colour_tokens();
    assert_eq!(tokens.len(), 19, "colour table rows in tokens.md");
    for (name, light, dark) in tokens {
        let light_rule = format!("--{name}: {light};");
        let dark_rule = format!("--{name}: {dark};");
        assert_eq!(
            TOKENS_CSS.matches(&light_rule).count(),
            1,
            "{light_rule} once"
        );
        assert_eq!(
            TOKENS_CSS.matches(&dark_rule).count(),
            2,
            "{dark_rule} twice"
        );
    }
}

#[test]
fn app_css_uses_no_raw_colours() {
    assert_eq!(raw_colours(APP_CSS), Vec::<String>::new());
}

#[test]
fn the_raw_colour_check_finds_hex_and_functions() {
    assert_eq!(
        raw_colours("a { color: #fff; b: rgb(0 0 0); c: #21 }"),
        ["#fff", "rgb("]
    );
}
