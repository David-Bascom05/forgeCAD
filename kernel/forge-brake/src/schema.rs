//! The library's record fields, for a builder's form (MACHINE-BUILDER's shared library builder):
//! per record kind (punch, die, press) each field's path as the TOML holds it, its kind, unit,
//! range, whether it is required, whether it is working geometry (published or an exact
//! derivation, never an estimate), and a line of help. The catalog (catalog.rs) reads exactly
//! these; a test keeps the two in step.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Number,
    Text,
    /// One of `choices`.
    Choice,
    /// `[lo, hi]`.
    Range,
    /// `[{ length, count }, ..]`.
    Segments,
    /// `[[x, z], ..]`, one closed polygon in the tool's section.
    Outline,
    Bool,
    /// `[text, ..]`.
    Texts,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Field {
    pub path: &'static str,
    pub kind: Kind,
    pub unit: &'static str,
    pub range: Option<(f64, f64)>,
    pub choices: &'static [&'static str],
    pub required: bool,
    /// Working geometry: refused when unknown, and only `published` (or an exact derivation)
    /// is acceptable.
    pub working: bool,
    pub help: &'static str,
}

const fn f(
    path: &'static str,
    kind: Kind,
    unit: &'static str,
    range: Option<(f64, f64)>,
    required: bool,
    working: bool,
    help: &'static str,
) -> Field {
    Field {
        path,
        kind,
        unit,
        range,
        choices: &[],
        required,
        working,
        help,
    }
}

pub const CLAMPS: &[&str] = &[
    "promecam",
    "wila-ns",
    "american",
    "trumpf",
    "bystronic-rf",
    "amada-afh",
    "amada-std",
];

const COMMON: [Field; 5] = [
    f(
        "id",
        Kind::Text,
        "",
        None,
        true,
        false,
        "unique: maker/part number",
    ),
    f("maker", Kind::Text, "", None, true, false, "the maker"),
    f(
        "name",
        Kind::Text,
        "",
        None,
        false,
        false,
        "the maker's name for it",
    ),
    f(
        "real",
        Kind::Bool,
        "",
        None,
        false,
        false,
        "a real maker's product: every value it holds needs a cite",
    ),
    f("note", Kind::Text, "", None, false, false, "anything else"),
];

pub const PUNCH: &[Field] = &[
    COMMON[0],
    COMMON[1],
    COMMON[2],
    COMMON[3],
    COMMON[4],
    Field {
        choices: CLAMPS,
        ..f(
            "clamp",
            Kind::Choice,
            "",
            None,
            true,
            false,
            "the clamping system (tang) it is made for",
        )
    },
    Field {
        choices: &["straight", "gooseneck", "acute", "sash"],
        ..f("kind", Kind::Choice, "", None, true, false, "its form")
    },
    f(
        "height",
        Kind::Number,
        "mm",
        Some((1.0, 2000.0)),
        true,
        true,
        "tip to the clamping shoulder",
    ),
    f(
        "tip_radius",
        Kind::Number,
        "mm",
        Some((0.0, 100.0)),
        true,
        true,
        "the tip's radius",
    ),
    f(
        "tip_angle",
        Kind::Number,
        "deg",
        Some((5.0, 179.0)),
        true,
        true,
        "included angle at the tip",
    ),
    f(
        "width",
        Kind::Number,
        "mm",
        Some((1.0, 1000.0)),
        true,
        false,
        "body width about the tip line (an envelope: err large)",
    ),
    f(
        "tip_height",
        Kind::Number,
        "mm",
        Some((0.5, 2000.0)),
        false,
        true,
        "height of the straight flanks (the working part)",
    ),
    f(
        "relief.depth",
        Kind::Number,
        "mm",
        Some((0.0, 1000.0)),
        false,
        true,
        "gooseneck throat depth behind the tip line (required for a gooseneck)",
    ),
    f(
        "relief.z0",
        Kind::Number,
        "mm",
        Some((0.0, 2000.0)),
        false,
        false,
        "the throat's floor above the tip",
    ),
    f(
        "relief.z1",
        Kind::Number,
        "mm",
        Some((0.0, 2000.0)),
        false,
        false,
        "the throat's top above the tip",
    ),
    f(
        "max_load",
        Kind::Number,
        "kN/m",
        Some((1.0, 100000.0)),
        false,
        false,
        "rated load per metre (unknown: tonnage not checked against it)",
    ),
    f(
        "segments",
        Kind::Segments,
        "mm",
        None,
        false,
        false,
        "lengths held and how many of each",
    ),
    f(
        "outline",
        Kind::Outline,
        "mm",
        None,
        false,
        false,
        "an explicit section in place of the generated one (tip at the origin, z up)",
    ),
];

pub const DIE: &[Field] = &[
    COMMON[0],
    COMMON[1],
    COMMON[2],
    COMMON[3],
    COMMON[4],
    Field {
        choices: CLAMPS,
        ..f(
            "clamp",
            Kind::Choice,
            "",
            None,
            true,
            false,
            "the clamping system it is made for",
        )
    },
    f(
        "v",
        Kind::Number,
        "mm",
        Some((1.0, 1000.0)),
        true,
        true,
        "V opening at the top",
    ),
    f(
        "angle",
        Kind::Number,
        "deg",
        Some((5.0, 179.0)),
        true,
        true,
        "included angle of the V",
    ),
    f(
        "shoulder_radius",
        Kind::Number,
        "mm",
        Some((0.0, 100.0)),
        true,
        true,
        "the V's shoulder (edge) radius",
    ),
    f(
        "height",
        Kind::Number,
        "mm",
        Some((1.0, 2000.0)),
        true,
        true,
        "die height",
    ),
    f(
        "width",
        Kind::Number,
        "mm",
        Some((1.0, 2000.0)),
        true,
        false,
        "body width (an envelope: err large)",
    ),
    f(
        "floor",
        Kind::Number,
        "mm",
        Some((0.1, 2000.0)),
        false,
        true,
        "V depth to a flat floor, if it stops short of the sharp bottom",
    ),
    f(
        "max_load",
        Kind::Number,
        "kN/m",
        Some((1.0, 100000.0)),
        false,
        false,
        "rated load per metre",
    ),
    f(
        "segments",
        Kind::Segments,
        "mm",
        None,
        false,
        false,
        "lengths held and how many of each",
    ),
    f(
        "outline",
        Kind::Outline,
        "mm",
        None,
        false,
        false,
        "an explicit section (V centred on x = 0, top at z = 0)",
    ),
];

pub const PRESS: &[Field] = &[
    COMMON[0],
    COMMON[1],
    f("model", Kind::Text, "", None, true, false, "the maker's model"),
    COMMON[3],
    COMMON[4],
    Field {
        choices: CLAMPS,
        ..f(
            "clamp",
            Kind::Choice,
            "",
            None,
            false,
            false,
            "its tool clamping system (unknown: tools must be named)",
        )
    },
    f(
        "force",
        Kind::Number,
        "kN",
        Some((1.0, 1e6)),
        false,
        false,
        "press force (unknown: tonnage not checked)",
    ),
    f(
        "bed_length",
        Kind::Number,
        "mm",
        Some((1.0, 100000.0)),
        true,
        true,
        "bending length",
    ),
    f(
        "housing_gap",
        Kind::Number,
        "mm",
        Some((1.0, 100000.0)),
        true,
        true,
        "clear width between the side housings",
    ),
    f(
        "open_height",
        Kind::Number,
        "mm",
        Some((1.0, 10000.0)),
        true,
        true,
        "daylight, table to beam",
    ),
    f(
        "stroke",
        Kind::Number,
        "mm",
        Some((0.0, 10000.0)),
        true,
        true,
        "ram stroke",
    ),
    f(
        "clamps_included",
        Kind::Bool,
        "",
        None,
        false,
        false,
        "the open height is measured from the clamps (then open height and stroke are checked)",
    ),
    f(
        "throat",
        Kind::Number,
        "mm",
        Some((1.0, 10000.0)),
        false,
        false,
        "throat depth behind the tool line (unknown: the frame is not a body; parts must fit the housing gap)",
    ),
    f(
        "frame.thickness",
        Kind::Number,
        "mm",
        Some((1.0, 10000.0)),
        false,
        false,
        "each side housing's thickness along the bed",
    ),
    f(
        "frame.depth",
        Kind::Number,
        "mm",
        Some((1.0, 100000.0)),
        false,
        false,
        "each housing's depth behind the throat",
    ),
    f(
        "ram.front",
        Kind::Number,
        "mm",
        Some((0.0, 10000.0)),
        false,
        false,
        "upper beam's reach in front of the tool line",
    ),
    f(
        "ram.back",
        Kind::Number,
        "mm",
        Some((0.0, 10000.0)),
        false,
        false,
        "upper beam's reach behind it",
    ),
    f(
        "bed.front",
        Kind::Number,
        "mm",
        Some((0.0, 10000.0)),
        false,
        false,
        "lower beam's reach in front",
    ),
    f(
        "bed.back",
        Kind::Number,
        "mm",
        Some((0.0, 10000.0)),
        false,
        false,
        "lower beam's reach behind",
    ),
    f(
        "table_height",
        Kind::Number,
        "mm",
        Some((0.0, 5000.0)),
        false,
        false,
        "working height above the floor (drawn only)",
    ),
    f(
        "speeds.approach",
        Kind::Number,
        "mm/s",
        Some((0.0, 10000.0)),
        false,
        false,
        "ram rapid approach",
    ),
    f(
        "speeds.work",
        Kind::Number,
        "mm/s",
        Some((0.0, 10000.0)),
        false,
        false,
        "ram working speed",
    ),
    f(
        "speeds.return",
        Kind::Number,
        "mm/s",
        Some((0.0, 10000.0)),
        false,
        false,
        "ram return",
    ),
    f(
        "backgauge.axes",
        Kind::Texts,
        "",
        None,
        false,
        false,
        "X, R, Z1, Z2, X1, X2, ..",
    ),
    f(
        "backgauge.x",
        Kind::Range,
        "mm",
        None,
        false,
        false,
        "stop face range behind the V centre (with r: the backgauge is checked)",
    ),
    f(
        "backgauge.r",
        Kind::Range,
        "mm",
        None,
        false,
        false,
        "finger height range above the die top",
    ),
    f(
        "backgauge.z",
        Kind::Range,
        "mm",
        None,
        false,
        false,
        "fingers' range along the bed from its centre",
    ),
    f(
        "backgauge.fingers",
        Kind::Number,
        "",
        Some((1.0, 12.0)),
        false,
        false,
        "how many fingers",
    ),
    f(
        "backgauge.finger.depth",
        Kind::Number,
        "mm",
        Some((1.0, 1000.0)),
        false,
        false,
        "finger depth behind its stop face",
    ),
    f(
        "backgauge.finger.height",
        Kind::Number,
        "mm",
        Some((1.0, 1000.0)),
        false,
        false,
        "finger height",
    ),
    f(
        "backgauge.finger.width",
        Kind::Number,
        "mm",
        Some((1.0, 1000.0)),
        false,
        false,
        "finger width along the bed",
    ),
    f(
        "backgauge.retract",
        Kind::Number,
        "mm",
        Some((0.0, 1000.0)),
        false,
        false,
        "how far the fingers back off once the punch holds the part",
    ),
];

/// Each record's cite row fields (docs/cam-machine.md, Provenance).
pub const CITE: &[Field] = &[
    f(
        "path",
        Kind::Text,
        "",
        None,
        true,
        false,
        "the value cited (a field's path), or `record`",
    ),
    Field {
        choices: &["published", "estimate", "unknown", "conflict", "user"],
        ..f(
            "status",
            Kind::Choice,
            "",
            None,
            true,
            false,
            "where it comes from",
        )
    },
    f(
        "source",
        Kind::Text,
        "",
        None,
        false,
        false,
        "URL or document (published, conflict)",
    ),
    f(
        "taken",
        Kind::Text,
        "",
        None,
        false,
        false,
        "YYYY-MM-DD read (with a source)",
    ),
    f(
        "value",
        Kind::Text,
        "",
        None,
        false,
        false,
        "the source's statement, restated with units",
    ),
    f(
        "basis",
        Kind::Text,
        "",
        None,
        false,
        false,
        "how it was derived and which way it errs (estimate, unknown)",
    ),
    f(
        "note",
        Kind::Text,
        "",
        None,
        false,
        false,
        "what disagrees (conflict)",
    ),
];

/// The record kinds: (TOML table name, fields).
pub const RECORDS: &[(&str, &[Field])] = &[("punch", PUNCH), ("die", DIE), ("press", PRESS)];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::Catalog;

    /// A record with every required field from the schema reads; one without a required
    /// working field is listed UNKNOWN or refused.
    #[test]
    fn the_schema_matches_the_reader() {
        let val = |fl: &Field| -> String {
            match fl.kind {
                Kind::Number => format!("{}", fl.range.map_or(1.0, |r| (r.0 + 1.0).max(r.0) * 10.0)),
                Kind::Text => format!("\"x/{}\"", fl.path),
                Kind::Choice => format!("\"{}\"", fl.choices[0]),
                Kind::Bool => "false".into(),
                _ => unreachable!(),
            }
        };
        for (table, fields) in RECORDS {
            let mut lines = vec![format!("[[{table}]]")];
            let mut tables: Vec<(String, Vec<String>)> = Vec::new();
            for fl in fields.iter().filter(|f| f.required) {
                let v = match (table, fl.path) {
                    (_, "id") => format!("\"x/{table}\""),
                    (&"punch", "height") => "100".into(),
                    (&"punch", "tip_radius") => "1".into(),
                    (&"punch", "tip_angle") => "88".into(),
                    (&"punch", "width") => "26".into(),
                    (&"die", "v") => "12".into(),
                    (&"die", "angle") => "88".into(),
                    (&"die", "shoulder_radius") => "1".into(),
                    (&"die", "height") => "60".into(),
                    (&"die", "width") => "40".into(),
                    (&"press", "open_height") => "400".into(),
                    (&"press", "stroke") => "200".into(),
                    (&"press", "bed_length") => "3000".into(),
                    (&"press", "housing_gap") => "2600".into(),
                    _ => val(fl),
                };
                match fl.path.split_once('.') {
                    Some((t, k)) => match tables.iter_mut().find(|x| x.0 == t) {
                        Some(x) => x.1.push(format!("{k} = {v}")),
                        None => tables.push((t.into(), vec![format!("{k} = {v}")])),
                    },
                    None => lines.push(format!("{} = {v}", fl.path)),
                }
            }
            for (t, kv) in tables {
                lines.push(format!("{t} = {{ {} }}", kv.join(", ")));
            }
            let src = lines.join("\n") + "\n";
            let c =
                Catalog::from_sources(&[("t.toml", &src)]).unwrap_or_else(|e| panic!("{table}: {e}\n{src}"));
            let ok = match *table {
                "punch" => c.punches[0].punch.is_ok(),
                "die" => c.dies[0].die.is_ok(),
                _ => c.presses[0].press.is_ok(),
            };
            assert!(ok, "{table}: {src}");
        }
    }
}
