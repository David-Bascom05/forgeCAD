//! The Library Builder's units (docs/ux/STANDARD.md §5): a value shows in the unit its record
//! keeps it in, and that unit is the one its citation reads. The 850 kN press shown as "850 N"
//! (and its mm/s speeds as "m/min") is what this guards.

use forge_render::shell::lib_form::value_text;
use forge_vendor::toml::Item;

#[test]
fn the_trubend_5085_sheet_shows_850_kn() {
    let presses = forge_brake_builder::builtin_of("press");
    let recs = forge_brake_builder::parse_kind("press", &presses[0].1).unwrap();
    let r = recs
        .iter()
        .find(|r| r.id == "trumpf-trubend-5085")
        .expect("the TruBend 5085");
    let force = match forge_render::shell::machine_brake::get(&r.table, "force") {
        Some(Item::Num(x)) => *x,
        other => panic!("{other:?}"),
    };
    let unit = forge_brake_builder::fields("press")
        .into_iter()
        .find(|f| f.path == "force")
        .unwrap()
        .unit
        .suffix();
    assert_eq!(value_text(force, unit), "850 kN");
    let work = forge_brake_builder::fields("press")
        .into_iter()
        .find(|f| f.path == "speeds.work")
        .unwrap()
        .unit
        .suffix();
    assert_eq!(value_text(25.0, work), "25 mm/s");
    assert_eq!(value_text(86.0, "deg"), "86°");
    assert_eq!(value_text(10.0, "mm"), "10 mm");
}

/// The unit words a citation may read a value in, per the unit the sheet shows it in (a length
/// may be cited in inches: the record converts it).
fn compatible(unit: &str) -> &'static [&'static str] {
    match unit {
        "mm" => &["mm", "in", "inch", "inches", "\""],
        "°" => &["deg", "°", "degree", "degrees"],
        "kN" => &["kN", "t", "ton", "tons", "tonne", "tonnes", "tonf"],
        "kN/m" => &[
            "kN/m", "t/m", "ton/m", "tons/m", "kN/mm", "t/ft", "tons/ft", "ton/ft",
        ],
        "mm/s" => &["mm/s", "mm/sec", "in/s", "in/sec", "ipm", "in/min", "mm/min"],
        "rpm" => &["rpm", "min-1", "1/min", "min⁻¹"],
        "kg" => &["kg", "lb", "lbs"],
        "N" => &["N", "lbf"],
        "m/min" => &["m/min", "sfm"],
        _ => &[],
    }
}

/// Every unit word the audit knows, to tell a citation that names a unit from one that names none.
const WORDS: &[&str] = &[
    "mm", "in", "inch", "inches", "deg", "°", "degree", "degrees", "kN", "N", "t", "ton", "tons", "tonne",
    "tonnes", "tonf", "kN/m", "t/m", "kN/mm", "t/ft", "tons/ft", "ton/ft", "mm/s", "mm/sec", "in/s",
    "in/sec", "m/min", "mm/min", "in/min", "ipm", "rpm", "min-1", "1/min", "kg", "lb", "lbs", "lbf",
];

/// The unit words in a citation's text, as whole tokens.
fn words_in(text: &str) -> Vec<String> {
    text.split(|c: char| c.is_whitespace() || ",;()[]|=:".contains(c))
        .map(|w| w.trim_matches(|c: char| c == '.' || c == '\'' || c == '`'))
        // "160mm", "86°": the unit glued to its number
        .map(|w| w.trim_start_matches(|c: char| c.is_ascii_digit() || c == '.' || c == '-'))
        .filter(|w| WORDS.contains(w))
        .map(str::to_string)
        .collect()
}

#[test]
fn every_cited_number_reads_in_its_sheets_unit() {
    let mut bad = Vec::new();
    let mut checked = 0;
    for kind in ["punch", "die", "press"] {
        let fields = forge_brake_builder::fields(kind);
        for (_, text) in forge_brake_builder::builtin_of(kind) {
            for r in forge_brake_builder::parse_kind(kind, &text).unwrap() {
                let Some(Item::Tables(cites)) = r.table.get("cite") else {
                    continue;
                };
                for c in cites {
                    let (Some(Item::Str(path)), Some(Item::Str(value))) = (c.get("path"), c.get("value"))
                    else {
                        continue;
                    };
                    let Some(f) = fields.iter().find(|f| &f.path == path) else {
                        continue;
                    };
                    if !matches!(f.ty, forge_library::schema::Ty::Num) {
                        continue;
                    }
                    let unit = f.unit.suffix();
                    let ok = compatible(unit);
                    let named = words_in(value);
                    if named.is_empty() {
                        continue;
                    }
                    if ok.is_empty() {
                        bad.push(format!(
                            "{kind} {} {path}: shown in `{unit}`, which the audit does not know",
                            r.id
                        ));
                        continue;
                    }
                    checked += 1;
                    if !named.iter().any(|w| ok.contains(&w.as_str())) {
                        bad.push(format!(
                            "{kind} {} {path}: shown in {unit}, cited \"{value}\"",
                            r.id
                        ));
                    }
                }
            }
        }
    }
    assert!(checked > 100, "only {checked} citations name a unit");
    assert!(bad.is_empty(), "{} of {checked}:\n{}", bad.len(), bad.join("\n"));
}

#[test]
fn every_cited_tool_number_reads_in_its_sheets_unit() {
    use forge_tooling::toml::{Val, text};
    let mut bad = Vec::new();
    let mut checked = 0;
    for (word, _, _) in forge_tooling_builder::KINDS {
        let (entries, _) = forge_tooling_builder::dynamic::catalog(word, None, None);
        for e in entries {
            let Some(Val::Tables(cites)) = e.def.table.get("cite") else {
                continue;
            };
            for c in cites {
                let (Some(path), Some(value)) = (text(c, "path"), text(c, "value")) else {
                    continue;
                };
                if !matches!(e.def.table.get(path), Some(Val::Num(_))) || path == "nof" {
                    continue;
                }
                let unit = forge_render::shell::machine_tooling::unit_of(path).trim();
                let named = words_in(value);
                if named.is_empty() {
                    continue;
                }
                checked += 1;
                let ok = compatible(unit);
                if !named.iter().any(|w| ok.contains(&w.as_str())) {
                    bad.push(format!(
                        "{word} {} {path}: shown in `{unit}`, cited \"{value}\"",
                        e.def.id
                    ));
                }
            }
        }
    }
    assert!(checked > 50, "only {checked} citations name a unit");
    assert!(bad.is_empty(), "{} of {checked}:\n{}", bad.len(), bad.join("\n"));
}
