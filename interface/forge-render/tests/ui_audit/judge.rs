//! The automatic half of the judge: what a popup or panel shows, read from its drawn text and its
//! pixels, against docs/ux/STANDARD.md. Each finding is a rule's short name and what it saw.
//! Hard findings are defects; soft ones are worth a look (a count that needs no unit is fine).

use crate::snap::{Snap, Text};
use egui::{Color32, Rect};

#[derive(Clone, Debug)]
pub struct Finding {
    pub rule: &'static str,
    pub hard: bool,
    pub what: String,
}

fn f(rule: &'static str, hard: bool, what: impl Into<String>) -> Finding {
    Finding {
        rule,
        hard,
        what: what.into(),
    }
}

/// The texts drawn inside the popup's region.
pub fn texts_in<'a>(s: &'a Snap, region: &[Rect]) -> Vec<&'a Text> {
    s.texts
        .iter()
        .filter(|t| !t.s.trim().is_empty() && region.iter().any(|r| r.expand(1.0).contains(t.rect.center())))
        .collect()
}

/// The texts drawn on top in the region: clipped to it, or painted over the topmost background
/// covering it (a panel beneath, covered, doesn't count).
pub fn on_top_in<'a>(s: &'a Snap, region: &[Rect]) -> Vec<&'a Text> {
    let floor = region
        .iter()
        .filter_map(|r| {
            // the topmost fill covering most of it (a panel beneath may cover it too)
            s.fills
                .iter()
                .filter(|(_, f)| f.intersect(*r).area() > 0.8 * r.area())
                .map(|x| x.0)
                .max()
        })
        .min()
        .unwrap_or(0);
    texts_in(s, region)
        .into_iter()
        .filter(|t| {
            region.iter().any(|r| r.expand(4.0).contains_rect(t.clip)) || (floor > 0 && t.idx > floor)
        })
        .collect()
}

/// Those the click put there: text already drawn in the same place before is what the popup
/// covers, not what it shows.
pub fn new_texts_in<'a>(s: &'a Snap, before: &Snap, region: &[Rect]) -> Vec<&'a Text> {
    on_top_in(s, region)
        .into_iter()
        .filter(|t| {
            !before
                .texts
                .iter()
                .any(|b| b.s == t.s && (b.rect.min - t.rect.min).length() < 1.0)
        })
        .collect()
}

pub fn popup(s: &Snap, before: &Snap, region: &[Rect], screen: Rect) -> Vec<Finding> {
    let mut out = Vec::new();
    for r in region {
        // an area's rect carries its margin and shadow: off-screen is more than that out
        let outside = r.area() - r.intersect(screen).area();
        if !screen.expand(16.0).contains_rect(*r) || outside > 0.1 * r.area() {
            out.push(f(
                "off-screen",
                true,
                format!(
                    "{:.0}x{:.0} at ({:.0},{:.0}) leaves the {:.0}x{:.0} window",
                    r.width(),
                    r.height(),
                    r.left(),
                    r.top(),
                    screen.width(),
                    screen.height()
                ),
            ));
        }
    }
    let texts = new_texts_in(s, before, region);
    if std::env::var_os("UI_AUDIT_DEBUG").is_some() {
        eprintln!(
            "region {region:?} fills {:?}",
            s.fills
                .iter()
                .filter(|f| region.iter().any(|r| f.1.intersects(*r)))
                .take(8)
                .collect::<Vec<_>>()
        );
        for t in &texts {
            eprintln!("  {:?} rect {:?} clip {:?} idx {}", t.s, t.rect, t.clip, t.idx);
        }
    }
    for t in &texts {
        let visible = t.rect.intersect(t.clip);
        // cut by its clip at the side: a layout too narrow for it. Cut only at the top or bottom
        // is a scrolling list's edge (the next row scrolls in): not a fault
        let c = t.clip.expand(1.0);
        let sideways = t.rect.left() < c.left() || t.rect.right() > c.right();
        let partly = visible.is_positive() && sideways;
        if partly {
            out.push(f("clipped", true, format!("\"{}\"", short(&t.s))));
        } else if t.elided {
            // cut short on purpose with an ellipsis (egui's truncate: the whole line on hover)
            out.push(f("truncated", false, format!("\"{}\"", short(&t.s))));
        }
        if t.size > 0.0 && t.size < 10.5 && visible.is_positive() {
            out.push(f(
                "tiny-text",
                true,
                format!("\"{}\" at {:.1} px", short(&t.s), t.size),
            ));
        }
        // a name the document gave (a feature's, a component's) is the person's, not a key
        let named = s.names.iter().any(|n| {
            t.s.trim() == n
                || t.s.split_whitespace().all(|w| {
                    s.names
                        .iter()
                        .any(|n| n == w.trim_matches(|c: char| !c.is_alphanumeric() && c != '_'))
                })
        });
        if !named && let Some(why) = debug_text(&t.s) {
            out.push(f("raw-text", true, format!("\"{}\" ({why})", short(&t.s))));
        }
        if visible.is_positive()
            && let Some(c) = contrast(s, t)
            && c < 3.0
        {
            out.push(f("contrast", true, format!("\"{}\" {:.1}:1", short(&t.s), c)));
        }
    }
    // texts drawn over each other
    for (i, a) in texts.iter().enumerate() {
        for b in &texts[i + 1..] {
            // what is seen of each: text a scroll area or a closed list clips away isn't drawn
            let (ra, rb) = (a.rect.intersect(a.clip), b.rect.intersect(b.clip));
            let x = ra.intersect(rb);
            if ra.is_positive() && rb.is_positive() && x.is_positive() && a.s != b.s {
                let small = ra.area().min(rb.area()).max(1.0);
                if x.area() / small > 0.25 {
                    out.push(f(
                        "overlap",
                        true,
                        format!("\"{}\" over \"{}\"", short(&a.s), short(&b.s)),
                    ));
                }
            }
        }
    }
    // a bare number with nothing on its row saying what unit it's in
    for t in &texts {
        if bare_number(&t.s) && !row_has_unit(t, &texts) {
            out.push(f(
                "no-unit",
                false,
                format!("\"{}\" in the row of {}", short(&t.s), row_label(t, &texts)),
            ));
        }
    }
    let area: f32 = region.iter().map(|r| r.area()).sum();
    if texts.len() <= 1 && area > 120.0 * 80.0 {
        out.push(f("empty", true, "a popup with nothing in it to read".to_string()));
    }
    dedup(out)
}

fn dedup(v: Vec<Finding>) -> Vec<Finding> {
    let mut out: Vec<Finding> = Vec::new();
    for x in v {
        if !out.iter().any(|y| y.rule == x.rule && y.what == x.what) {
            out.push(x);
        }
    }
    out
}

pub fn short(s: &str) -> String {
    let s = s.replace('\n', " ");
    if s.chars().count() > 48 {
        format!("{}…", s.chars().take(47).collect::<String>())
    } else {
        s
    }
}

/// Text a person shouldn't be shown: a Rust Debug dump, a raw key, a path in a temp directory.
pub fn debug_text(s: &str) -> Option<&'static str> {
    let t = s.trim();
    if [
        "Some(", "None)", "Ok(", "Err(", "{ ", " }", "::", "\\n", "unwrap", "panicked",
    ]
    .iter()
    .any(|k| t.contains(k))
        && !t.contains("::=")
    {
        return Some("debug text");
    }
    // an error's code before its words (`StillRead: plate reads plan`)
    if t.split(|c: char| c.is_whitespace()).any(|w| {
        let w = w.strip_suffix(':').unwrap_or("");
        w.len() > 4
            && w.chars().next().is_some_and(|c| c.is_ascii_uppercase())
            && w.chars().all(|c| c.is_ascii_alphanumeric())
            && w.chars().skip(1).any(|c| c.is_ascii_uppercase())
    }) {
        return Some("an error code");
    }
    if t.contains("/tmp/") || t.contains("fg-") && t.contains(".fg") && t.contains('/') {
        return Some("a temporary path");
    }
    if t.contains("NaN") || t.split_whitespace().any(|w| w == "inf" || w == "-inf") {
        return Some("not a number");
    }
    // a snake_case or dotted key standing alone (a TOML key), not a sentence
    let words: Vec<&str> = t.split_whitespace().collect();
    // a default name (`joint_1`, `loft1`) is a name the user keeps or changes, not a key
    let default_name = |w: &str| {
        let stem = w
            .trim_end_matches(|c: char| c.is_ascii_digit())
            .trim_end_matches('_');
        stem.len() < w.len() && stem.chars().all(|c| c.is_ascii_lowercase())
    };
    if words.len() <= 2
        && !words.iter().all(|w| default_name(w))
        && words.iter().any(|w| {
            let w = w.trim_matches(|c: char| !c.is_alphanumeric() && c != '_' && c != '.');
            (w.contains('_')
                && w.chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'))
                && w.len() > 3
        })
    {
        return Some("a raw key");
    }
    None
}

pub fn bare_number(s: &str) -> bool {
    let t = s.trim();
    let digits = t.chars().filter(|c| c.is_ascii_digit()).count();
    digits >= 2
        && t.contains('.')
        && t.chars()
            .all(|c| c.is_ascii_digit() || c == '.' || c == '-' || c == '+' || c == ',')
}

const UNITS: &[&str] = &[
    "mm", "in", "°", "deg", "%", "rpm", "kg", "g", "N", "kN", "s", "ms", "min", "m/min", "mm/min", "ipm",
    "sfm", "Nm", "W", "kW", "mm²", "mm³", "cm", "m", "ft", "\"", "x", "×", "/", "lb", "MPa", "µm", "µin",
];

/// Quantities with no unit: a number on their row is whole as it is.
const UNITLESS: &[&str] = &["K-factor", "ratio", "factor", "count", "scale"];

fn row_has_unit(t: &Text, all: &[&Text]) -> bool {
    all.iter().any(|o| {
        let same_row = (o.rect.center().y - t.rect.center().y).abs() < t.rect.height().max(8.0) * 0.6;
        same_row
            && (UNITLESS
                .iter()
                .any(|u| o.s.to_lowercase().contains(&u.to_lowercase()))
                || o.s
                    .split(|c: char| c.is_whitespace() || c == '(' || c == ')' || c == '·')
                    .any(|w| {
                        UNITS.contains(
                            &w.trim_start_matches(|c: char| c.is_ascii_digit() || c == '.' || c == '-'),
                        )
                    }))
    })
}

fn row_label(t: &Text, all: &[&Text]) -> String {
    all.iter()
        .filter(|o| {
            (o.rect.center().y - t.rect.center().y).abs() < t.rect.height().max(8.0) * 0.6
                && o.rect.center().x < t.rect.center().x
                && !bare_number(&o.s)
        })
        .min_by_key(|o| (t.rect.left() - o.rect.right()).abs() as i64)
        .map_or("(no label)".to_string(), |o| format!("\"{}\"", short(&o.s)))
}

fn lum(c: [u8; 3]) -> f32 {
    let ch = |v: u8| {
        let v = v as f32 / 255.0;
        if v <= 0.03928 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * ch(c[0]) + 0.7152 * ch(c[1]) + 0.0722 * ch(c[2])
}

pub fn ratio(a: [u8; 3], b: [u8; 3]) -> f32 {
    let (x, y) = (lum(a), lum(b));
    (x.max(y) + 0.05) / (x.min(y) + 0.05)
}

/// The text's colour against the commonest pixel in its box (the background it sits on).
fn contrast(s: &Snap, t: &Text) -> Option<f32> {
    let r = t.rect.intersect(t.clip);
    if r.width() < 3.0 || r.height() < 3.0 || s.w == 0 {
        return None;
    }
    let c = t.color;
    if c.a() == 0 {
        return None;
    }
    let mut counts: std::collections::HashMap<[u8; 3], usize> = Default::default();
    let (x0, y0) = (r.left().max(0.0) as usize, r.top().max(0.0) as usize);
    let (x1, y1) = ((r.right() as usize).min(s.w), (r.bottom() as usize).min(s.h));
    for y in y0..y1 {
        for x in x0..x1 {
            *counts.entry(s.px(x, y)).or_default() += 1;
        }
    }
    let bg = counts.into_iter().max_by_key(|(_, n)| *n)?.0;
    // the ink as drawn over that background (egui's colours are premultiplied)
    let a = c.a() as f32 / 255.0;
    let ink = |i: usize, v: u8| (v as f32 + bg[i] as f32 * (1.0 - a)).min(255.0) as u8;
    let Color32 { .. } = c;
    let drawn = [ink(0, c.r()), ink(1, c.g()), ink(2, c.b())];
    Some(ratio(drawn, bg))
}

pub fn finding(rule: &'static str, hard: bool, what: impl Into<String>) -> Finding {
    f(rule, hard, what)
}

/// A ribbon command's or menu item's label against STANDARD §5: Title Case on the ribbon, "…" as
/// one character and only where a dialog or more input follows.
pub fn label(name: &str, ribbon: bool, class_opens_dialog: bool) -> Vec<Finding> {
    let mut out = Vec::new();
    if name.contains("...") {
        out.push(f("label", false, format!("\"{name}\": three dots for \"…\"")));
    }
    if name.ends_with('…') && !class_opens_dialog {
        out.push(f(
            "label",
            false,
            format!("\"{name}\": \"…\" but no dialog follows"),
        ));
    }
    if ribbon {
        const SMALL: &[&str] = &[
            "of", "to", "and", "or", "the", "a", "an", "in", "on", "for", "from", "by", "as", "at", "into",
        ];
        let lower: Vec<&str> = name
            .split(|c: char| c.is_whitespace() || c == '/')
            .skip(1)
            .filter(|w| !w.is_empty() && !SMALL.contains(w))
            .filter(|w| w.chars().next().is_some_and(|c| c.is_ascii_lowercase()))
            .collect();
        if !lower.is_empty() {
            out.push(f(
                "label",
                false,
                format!("\"{name}\": not Title Case ({})", lower.join(", ")),
            ));
        }
    }
    out
}
