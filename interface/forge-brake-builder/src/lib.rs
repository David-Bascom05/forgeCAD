//! forge-brake's library (press-brake punches, dies and presses; kernel/forge-brake/src/catalog.rs)
//! as forge-library kinds, for the shared library builder (research/cam/library-builder.md):
//! each record is its TOML table as written (canonical: its values, then its cites), checked by
//! forge-brake's own reader; the form's fields come from forge_brake::schema; a preview is the
//! tool's exact section (lines and arcs, sampled) and its length along the bed.
use forge_brake::catalog::Catalog;
use forge_brake::schema::{self, Kind as FKind};
use forge_library::Kind;
use forge_library::schema::{Field, Ty, Unit};
use forge_vendor::toml::{self, Item, Table};

/// One record: which kind, its id, and its table.
#[derive(Clone, Debug, PartialEq)]
pub struct Rec {
    pub kind: &'static str,
    pub id: String,
    pub table: Table,
}

/// Tables written inline (`relief = { .. }`): every nested table but the cite rows.
const INLINE: &[&str] = &[
    "relief",
    "speeds",
    "frame",
    "ram",
    "bed",
    "backgauge",
    "finger",
    "segments",
];

/// The records of `kind` in a file's text, each read by forge-brake (a record it refuses is an
/// error, where and why).
pub fn parse_kind(kind: &'static str, text: &str) -> Result<Vec<Rec>, String> {
    Catalog::from_sources(&[("file", text)]).map_err(|e| e.to_string())?;
    let root = toml::parse(text).map_err(|e| format!("line {}: {}", e.line, e.why))?;
    let Some(Item::Tables(rows)) = root.get(kind) else {
        return Ok(Vec::new());
    };
    rows.iter()
        .map(|t| match t.get("id") {
            Some(Item::Str(id)) => Ok(Rec {
                kind,
                id: id.clone(),
                table: t.clone(),
            }),
            _ => Err(format!("a {kind} with no id")),
        })
        .collect()
}

/// A file of records, canonical: the header as comment lines, then each record.
pub fn write_kind(kind: &str, header: &str, rs: &[Rec]) -> Result<String, String> {
    let mut root = Table::default();
    root.set(kind, Item::Tables(rs.iter().map(|r| r.table.clone()).collect()));
    let mut out = String::new();
    for l in header.lines() {
        out.push_str(&format!("# {l}\n"));
    }
    out.push_str(&toml::write(&root, INLINE));
    // what is written must read back as what was given
    let back = parse_kind(
        match kind {
            "punch" => "punch",
            "die" => "die",
            _ => "press",
        },
        &out,
    )?;
    if back.iter().map(|r| &r.table).ne(rs.iter().map(|r| &r.table)) {
        return Err("the records do not write canonically".into());
    }
    Ok(out)
}

/// The built-in records of a kind, one file of them.
pub fn builtin_of(kind: &'static str) -> Vec<(String, String)> {
    let mut rs = Vec::new();
    for (_, text) in forge_brake::catalog::BUILTIN {
        if let Ok(mut v) = parse_kind(kind, text) {
            rs.append(&mut v);
        }
    }
    if rs.is_empty() {
        return Vec::new();
    }
    let text = write_kind(kind, "", &rs).expect("the built-in records write");
    vec![(format!("{kind}s.toml"), text)]
}

macro_rules! kinds {
    ($($ty:ident, $word:literal, $title:literal;)*) => {
        $(
            #[doc = concat!("The `", $word, "` kind.")]
            pub struct $ty;
            impl Kind for $ty {
                type Record = Rec;
                const WORD: &'static str = $word;
                // under the project's and the user's cam/ until forge-library takes a root of
                // its own (bend/tooling, as forge-lang's search reads)
                const FOLDER: &'static str = concat!("bend-", $word, "s");
                fn builtin() -> Vec<(String, String)> {
                    builtin_of($word)
                }
                fn parse(text: &str) -> Result<Vec<Rec>, String> {
                    parse_kind($word, text)
                }
                fn write(header: &str, rs: &[Rec]) -> Result<String, String> {
                    write_kind($word, header, rs)
                }
                fn id(r: &Rec) -> &str {
                    &r.id
                }
            }
        )*
        /// Every kind: (TOML key, title).
        pub const KINDS: &[(&str, &str)] = &[$(($word, $title)),*];
    };
}

kinds! {
    Punches, "punch", "Press-brake punch";
    Dies, "die", "Press-brake die";
    Presses, "press", "Press brake";
}

/// The form's fields of a kind, from forge-brake's schema (working geometry: required and cited).
pub fn fields(kind: &str) -> Vec<Field> {
    let src = match kind {
        "punch" => schema::PUNCH,
        "die" => schema::DIE,
        "press" => schema::PRESS,
        _ => return Vec::new(),
    };
    src.iter()
        .map(|f| {
            let unit = match f.unit {
                "mm" => Unit::Length,
                "deg" => Unit::Angle,
                "mm/s" => Unit::Speed,
                "kN" => Unit::Force,
                _ => Unit::None,
            };
            let ty = match f.kind {
                FKind::Number => Ty::Num,
                FKind::Text => Ty::Text,
                FKind::Choice => Ty::Choice(f.choices),
                FKind::Range => Ty::Range,
                FKind::Bool => Ty::Bool,
                FKind::Texts => Ty::Text,
                FKind::Segments => Ty::Points(&[("length", Unit::Length), ("count", Unit::None)]),
                FKind::Outline => Ty::Points(&[("x", Unit::Length), ("z", Unit::Length)]),
            };
            let group = if f.working {
                "Working geometry"
            } else if f.path.contains('.') {
                f.path.split('.').next().unwrap_or("")
            } else {
                "Record"
            };
            let mut fl = Field::new(f.path, "", ty, unit).group(leak(group)).help(f.help);
            fl.label = f.path;
            if !f.required {
                fl = fl.optional("unknown");
            }
            if matches!(f.path, "id" | "maker" | "name" | "model" | "real" | "note") {
                fl = fl.uncited();
            }
            fl
        })
        .collect()
}

fn leak(s: &str) -> &'static str {
    match s {
        "Working geometry" => "Working geometry",
        "Record" => "Record",
        "relief" => "relief",
        "frame" => "frame",
        "ram" => "ram",
        "bed" => "bed",
        "speeds" => "speeds",
        "backgauge" => "backgauge",
        _ => "",
    }
}

/// Templates to start a new record from: the stand-in sets' tools and press (forgecad's own
/// numbers, `real = false`: every value to be replaced by the maker's, cited).
pub fn templates() -> Vec<Rec> {
    let t = forge_brake::library::std_eu();
    let mut out = Vec::new();
    let deg = |r: f64| (r * 180.0 / std::f64::consts::PI * 1e6).round() / 1e6;
    let row = |kind: &'static str, id: &str, kv: Vec<(&str, Item)>| {
        let mut tb = Table::default();
        tb.set("id", Item::Str(format!("template/{id}")));
        tb.set("maker", Item::Str("template".into()));
        tb.set("real", Item::Bool(false));
        for (k, v) in kv {
            tb.set(k, v);
        }
        Rec {
            kind,
            id: format!("template/{id}"),
            table: tb,
        }
    };
    if let Some(p) = t.punches.first() {
        out.push(row(
            "punch",
            "punch",
            vec![
                ("clamp", Item::Str("promecam".into())),
                ("kind", Item::Str("straight".into())),
                ("height", Item::Num(p.height)),
                ("tip_radius", Item::Num(p.tip_radius)),
                ("tip_angle", Item::Num(deg(p.tip_angle))),
                ("width", Item::Num(p.width)),
            ],
        ));
    }
    if let Some(d) = t.dies.first() {
        out.push(row(
            "die",
            "die",
            vec![
                ("clamp", Item::Str("promecam".into())),
                ("v", Item::Num(d.v)),
                ("angle", Item::Num(deg(d.angle))),
                ("shoulder_radius", Item::Num(d.shoulder_radius)),
                ("height", Item::Num(d.height)),
                ("width", Item::Num(d.width)),
            ],
        ));
    }
    let p = &t.press;
    out.push(row(
        "press",
        "press",
        vec![
            ("model", Item::Str("template".into())),
            ("bed_length", Item::Num(p.bed_length)),
            ("housing_gap", Item::Num(p.housing_gap)),
            ("open_height", Item::Num(p.open_height)),
            ("stroke", Item::Num(p.stroke)),
        ],
    ));
    out
}

/// A check's finding (as forge-library's builders give them).
#[derive(Clone, Debug, PartialEq)]
pub struct Finding {
    pub path: String,
    pub what: String,
    pub fix: String,
    /// A value not published: the check could not run on it (not a failure).
    pub not_checked: bool,
}

/// The record checked by forge-brake's reader: refusals, unknown working geometry, unknown press
/// values (rules reported not checked), and a real record's values without a cite.
pub fn check(r: &Rec) -> Vec<Finding> {
    let mut root = Table::default();
    root.set(r.kind, Item::Tables(vec![r.table.clone()]));
    let text = toml::write(&root, INLINE);
    let cat = match Catalog::from_sources(&[("record", &text)]) {
        Ok(c) => c,
        Err(e) => {
            return vec![Finding {
                path: String::new(),
                what: e.to_string(),
                fix: "correct the value or its cite".into(),
                not_checked: false,
            }];
        }
    };
    let unknown = |fields: &[String], what: &str| -> Vec<Finding> {
        fields
            .iter()
            .map(|f| Finding {
                path: f.clone(),
                what: format!("{f}: {what}"),
                fix: format!("enter the maker's published {f}, cited"),
                not_checked: false,
            })
            .collect()
    };
    match r.kind {
        "punch" => match cat.punches.first().map(|p| &p.punch) {
            Some(Err(f)) => unknown(f, "unknown: the tool cannot be simulated"),
            _ => Vec::new(),
        },
        "die" => match cat.dies.first().map(|d| &d.die) {
            Some(Err(f)) => unknown(f, "unknown: the tool cannot be simulated"),
            _ => Vec::new(),
        },
        _ => match cat.presses.first().map(|p| &p.press) {
            Some(Err(f)) => unknown(f, "unknown: the press cannot be simulated"),
            Some(Ok(p)) => p
                .unchecked
                .iter()
                .map(|u| Finding {
                    path: u.clone(),
                    what: format!("{u}: not published; the rules needing it report not checked"),
                    fix: format!("enter the maker's published {u}, cited"),
                    not_checked: true,
                })
                .chain(p.estimated.iter().map(|e| Finding {
                    path: e.clone(),
                    what: format!("{e}: an envelope that errs large (no published geometry)"),
                    fix: format!("enter the {e}'s published dimensions"),
                    not_checked: true,
                }))
                .collect(),
            None => Vec::new(),
        },
    }
}

/// A preview: each section's closed outline in the press's (x, z) plane, mm, and its length
/// along the bed (its longest segment, else 100 mm). A press: its frame's housing and beams
/// as rectangles across the bed.
pub fn preview(r: &Rec) -> Result<Vec<(String, Vec<[f64; 2]>, f64)>, Finding> {
    let mut root = Table::default();
    root.set(r.kind, Item::Tables(vec![r.table.clone()]));
    let text = toml::write(&root, INLINE);
    let bad = |what: String| Finding {
        path: String::new(),
        what,
        fix: "complete the working geometry".into(),
        not_checked: false,
    };
    let cat = Catalog::from_sources(&[("record", &text)]).map_err(|e| bad(e.to_string()))?;
    let outline = |reg: &forge_brake::geo::Region| -> Vec<[f64; 2]> {
        reg.loops
            .iter()
            .flatten()
            .flat_map(|s| {
                let n = match s {
                    forge_brake::geo::Seg::Line { .. } => 1,
                    forge_brake::geo::Seg::Arc { .. } => 12,
                };
                (0..n).map(move |i| {
                    let p = s.point(i as f64 / n as f64);
                    [p.x, p.y]
                })
            })
            .collect()
    };
    let len = |segs: &[forge_brake::Segment]| segs.iter().map(|s| s.length).fold(0.0, f64::max).max(100.0);
    match r.kind {
        "punch" => {
            let p = cat.punches.first().ok_or_else(|| bad("no punch".into()))?;
            let p = p
                .punch
                .as_ref()
                .map_err(|f| bad(format!("unknown: {}", f.join(", "))))?;
            Ok(vec![
                ("tip".into(), outline(&p.tip_section()), len(&p.segments)),
                ("body".into(), outline(&p.body_section()), len(&p.segments)),
            ])
        }
        "die" => {
            let d = cat.dies.first().ok_or_else(|| bad("no die".into()))?;
            let d = d
                .die
                .as_ref()
                .map_err(|f| bad(format!("unknown: {}", f.join(", "))))?;
            Ok(vec![("die".into(), outline(&d.section()), len(&d.segments))])
        }
        _ => {
            let p = cat.presses.first().ok_or_else(|| bad("no press".into()))?;
            let p = p
                .press
                .as_ref()
                .map_err(|f| bad(format!("unknown: {}", f.join(", "))))?;
            let rect = |x0: f64, z0: f64, x1: f64, z1: f64| vec![[x0, z0], [x1, z0], [x1, z1], [x0, z1]];
            let mut v = vec![
                (
                    "ram".into(),
                    rect(-p.ram.front, p.open_height, p.ram.back, p.open_height + 600.0),
                    p.bed_length,
                ),
                (
                    "bed".into(),
                    rect(-p.bed.front, -800.0, p.bed.back, 0.0),
                    p.bed_length,
                ),
            ];
            if let Some(f) = p.frame {
                v.push((
                    "housing".into(),
                    rect(f.throat, -800.0, f.throat + f.depth, p.open_height + 600.0),
                    f.thickness,
                ));
            }
            Ok(v)
        }
    }
}
