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
