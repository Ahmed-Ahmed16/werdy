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
    assert_eq!(tokens.len(), 20, "colour table rows in tokens.md");
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

fn rule_body<'a>(css: &'a str, selector: &str) -> Option<&'a str> {
    css.match_indices('{').find_map(|(open, _)| {
        let start = css[..open].rfind(['{', '}']).map_or(0, |index| index + 1);
        let head = css[start..open].rsplit("*/").next()?.trim();
        let matches = head.split(',').any(|part| part.trim() == selector);
        matches.then(|| css[open + 1..].split('}').next()).flatten()
    })
}

fn has_declaration(body: &str, property: &str, value: &str) -> bool {
    body.lines()
        .any(|line| line.trim() == format!("{property}: {value};"))
}

fn media_block<'a>(css: &'a str, query: &str) -> Option<&'a str> {
    let marker = format!("@media {query} {{");
    let start = css.find(&marker)? + marker.len();
    let mut depth = 1;
    for (index, ch) in css[start..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => depth -= 1,
            _ => {}
        }
        if depth == 0 {
            return Some(&css[start..start + index]);
        }
    }
    None
}

fn media_rule_has(css: &str, query: &str, selector: &str, declaration: (&str, &str)) -> bool {
    let marker = format!("@media {query} {{");
    css.match_indices(&marker).any(|(start, _)| {
        media_block(&css[start..], query)
            .and_then(|block| rule_body(block, selector))
            .is_some_and(|body| has_declaration(body, declaration.0, declaration.1))
    })
}

fn touch_sized(css: &str, selector: &str) -> bool {
    let Some(body) = rule_body(css, selector) else {
        return false;
    };
    let height = has_declaration(body, "height", "var(--touch-min)")
        || has_declaration(body, "min-height", "var(--touch-min)");
    let width = has_declaration(body, "width", "var(--touch-min)")
        || has_declaration(body, "min-width", "var(--touch-min)");
    height && (!matches!(selector, ".mark-button" | ".step-button") || width)
}

#[test]
fn responsive_rules_cover_phone_tablet_and_wide_web() {
    let tablet = media_block(APP_CSS, "(min-width: 600px)").expect("tablet rules");
    let expanded = media_block(APP_CSS, "(min-width: 840px)").expect("expanded rules");
    let wide = media_block(APP_CSS, "(min-width: 1280px)").expect("wide rules");
    assert!(
        rule_body(APP_CSS, ".panes").is_some_and(|body| has_declaration(
            body,
            "grid-template-columns",
            "minmax(0, 1fr)"
        ))
    );
    assert!(rule_body(tablet, ".note-dialog").is_some());
    assert!(
        rule_body(expanded, ".rail").is_some_and(|body| has_declaration(body, "display", "flex")
            && has_declaration(body, "position", "sticky"))
    );
    assert!(
        rule_body(expanded, ".panes").is_some_and(|body| has_declaration(
            body,
            "grid-template-columns",
            "minmax(0, 3fr) minmax(0, 2fr)"
        ))
    );
    assert!(media_rule_has(
        APP_CSS,
        "(min-width: 840px)",
        ".week-card",
        ("display", "block")
    ));
    assert!(
        rule_body(wide, ".panes").is_some_and(|body| has_declaration(
            body,
            "max-width",
            "var(--content-max)"
        ))
    );
    for selector in [
        ".werdy-root",
        ".panes",
        ".report-panes",
        ".page-content",
        ".onboarding-content",
    ] {
        for block in APP_CSS.split(&format!("{selector} {{")).skip(1) {
            let declarations = block.split('}').next().unwrap_or_default();
            assert!(
                !declarations.lines().any(|line| {
                    let rule = line.trim();
                    rule.starts_with("width:") && rule.ends_with("px;")
                }),
                "{selector} must stay fluid"
            );
        }
    }
}

#[test]
fn interactive_controls_keep_touch_size_and_visible_focus() {
    assert!(TOKENS_CSS.contains("--touch-min: 48px;"));
    assert!(APP_CSS.contains("button:focus-visible"));
    assert!(APP_CSS.contains("input:focus-visible"));
    assert!(APP_CSS.contains("select:focus-visible"));
    assert!(APP_CSS.contains("textarea:focus-visible"));
    assert!(APP_CSS.contains("a:focus-visible"));
    for selector in [
        ".mark-button",
        ".step-button",
        ".rail-item",
        ".bottom-item",
        ".period-chip",
    ] {
        assert!(
            touch_sized(APP_CSS, selector),
            "{selector} must use the touch token"
        );
    }
}

#[test]
fn slot_reminder_switch_keeps_native_checkbox_and_visible_states() {
    let reminders = include_str!("ui/reminders.rs");
    assert!(reminders.contains("label { class: \"reminder-switch\","));
    assert!(reminders.contains("r#type: \"checkbox\", checked: rule.is_some()"));
    assert!(reminders.contains("event.checked()"));
    assert!(reminders.contains("span { {i18n.t(&format!(\"slot-{slot}\"))} }"));
    assert!(touch_sized(APP_CSS, ".reminder-switch"));
    for (selector, property, value) in [
        (
            ".reminder-switch span::before",
            "background",
            "var(--track)",
        ),
        (
            ".reminder-switch input:checked + span::before",
            "background",
            "var(--primary)",
        ),
        (
            ".reminder-switch span::after",
            "background",
            "var(--text-muted)",
        ),
        (
            ".reminder-switch input:checked + span::after",
            "background",
            "var(--on-primary)",
        ),
        (
            ".reminder-switch input:focus-visible + span::before",
            "outline",
            "var(--stroke-strong) solid var(--primary)",
        ),
    ] {
        assert!(
            rule_body(APP_CSS, selector).is_some_and(|body| has_declaration(body, property, value)),
            "missing switch state: {selector}"
        );
    }
}

#[test]
fn reminder_switch_thumb_moves_from_inline_start_in_both_directions() {
    for (selector, position) in [
        (".reminder-switch span::after", "var(--gap-xs)"),
        (
            ".reminder-switch input:checked + span::after",
            "calc(var(--touch-min) - var(--gap-l) - var(--gap-xs))",
        ),
    ] {
        let body = rule_body(APP_CSS, selector).expect("switch thumb rule");
        assert!(has_declaration(body, "inset-inline-start", position));
        assert!(!body.contains("left:"));
        assert!(!body.contains("right:"));
    }
}

#[test]
fn compact_week_keeps_note_signs_and_short_today_label() {
    assert!(!APP_CSS.contains(".week-card .report-day-cell small {"));
    for selector in [
        ".week-card .report-score-detail",
        ".week-card .report-today-label",
    ] {
        assert!(
            rule_body(APP_CSS, selector)
                .is_some_and(|body| has_declaration(body, "display", "none"))
        );
    }
    assert!(
        rule_body(APP_CSS, ".week-card .report-today-compact")
            .is_some_and(|body| has_declaration(body, "display", "inline"))
    );
    assert!(include_str!("ui/reports.rs").contains("reports-today-short"));
}
