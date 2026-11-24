//! Workholding as a kind of the shared library builder (research/cam/library-builder.md; the
//! records, their reader and writer, and the placements are forge-workholding's).
//!
//! - `Workholding`: `impl forge_library::Kind` (folder `cam/workholding`).
//! - `fields(kind)`: each workholding kind's form (forge_library::schema::Field), from
//!   forge-workholding's schema: units, required or optional, the meaning as help.
//! - `templates()`: the stand-ins, one or more per kind, to start a new record from.
//! - `check(record)`: the record placed on a reference stock (or bar): every finding with the
//!   field, what, and the fix; fields not published are "not checked", not failures.
//! - `preview(record)`: its bodies as named blocks and cylinders (a lathe device as cylinders
//!   about +z), what the builder's 3D view draws.
//! - `cli::run`: `forgecad workholding list|new|check|show|schema`.
pub mod cli;

use forge_library::schema::{Field, Ty, Unit};
use forge_workholding::{Box3, Kind as WKind, Library, Prim, Record, WhError, lathe, mill};

pub struct Workholding;

impl forge_library::Kind for Workholding {
    type Record = Record;
    const WORD: &'static str = "workholding";
    const FOLDER: &'static str = "workholding";
    fn builtin() -> Vec<(String, String)> {
        forge_workholding::BUILTIN
            .iter()
            .map(|(f, s)| (f.strip_prefix("builtin/").unwrap_or(f).to_string(), s.to_string()))
            .collect()
    }
    fn parse(text: &str) -> Result<Vec<Record>, String> {
        forge_workholding::parse_file("", text).map_err(|e| e.to_string())
    }
    fn write(header: &str, records: &[Record]) -> Result<String, String> {
        let mut o = String::new();
        for l in header.lines() {
            o += &format!("# {l}\n");
        }
        for r in records {
            o.push('\n');
            o += &r.to_toml();
        }
        Ok(o)
    }
    fn id(r: &Record) -> &str {
        &r.name
    }
}

fn unit(u: &str) -> Unit {
    match u {
        "mm" | "mm[]" => Unit::Length,
        "deg" => Unit::Angle,
        "rpm" => Unit::Rpm,
        "kg" => Unit::Mass,
        "N" => Unit::Force,
        "kPa" => Unit::Pressure,
        "N·m" => Unit::Torque,
        "m³/h" => Unit::Flow,
        _ => Unit::None,
    }
}

const RATED: &[&str] = &[
    "clamp_force",
    "weight",
    "repeatability",
    "accuracy",
    "pull_force",
    "hold_force",
    "max_rpm",
    "grip_force",
    "torque",
    "flow",
];

/// A workholding kind's form: one field per number (a parallel set's heights a table of one
/// column), required or optional as the kind says, the meaning as help, grouped into the sizes
/// that make its bodies and the ratings that do not.
pub fn fields(kind: WKind) -> Vec<Field> {
    let k = forge_workholding::schema::schema()
        .into_iter()
        .find(|s| s.kind == kind)
        .expect("every kind has a schema");
    k.fields
        .iter()
        .map(|f| {
            let ty = match (f.name, f.unit) {
                (_, "mm[]") => Ty::Points(&[("height", Unit::Length)]),
                (_, "count") => Ty::Int,
                (_, "") => Ty::Text,
                _ => Ty::Num,
            };
            let mut fl = Field::new(f.name, f.name, ty, unit(f.unit))
                .group(if RATED.contains(&f.name) { "rated" } else { "size" })
                .help(f.meaning);
            if !f.required {
                fl = fl.optional("not given");
            }
            fl
        })
        .collect()
}

/// The stand-ins (`standin = true`), to start a new record of a kind from.
pub fn templates() -> Vec<Record> {
    Library::builtin()
        .records
        .into_iter()
        .map(|(r, _)| r)
        .filter(|r| r.standin)
        .collect()
}

/// A check's finding.
#[derive(Clone, Debug, PartialEq)]
pub struct Finding {
    /// The field it is about ("" for the record as a whole).
    pub path: String,
    pub what: String,
    pub fix: String,
    /// A field not published: the check could not run (not a failure).
    pub not_checked: bool,
}

fn finding(e: WhError) -> Finding {
    match e {
        WhError::Unpublished { field, fix, .. } => Finding {
            what: format!("`{field}` is not published (or its published values disagree): not checked"),
            path: field,
            fix,
            not_checked: true,
        },
        WhError::Missing { field, fix, .. } => Finding {
            what: format!("`{field}` is missing"),
            path: field,
            fix,
            not_checked: false,
        },
        WhError::Bad { field, why, .. } => Finding {
            what: format!("`{field}` {why}"),
            path: field,
            fix: "correct it".into(),
            not_checked: false,
        },
        WhError::Placement { what, fix, .. } => Finding {
            path: String::new(),
            what,
            fix,
            not_checked: false,
        },
        e => Finding {
            path: String::new(),
            what: e.to_string(),
            fix: String::new(),
            not_checked: false,
        },
    }
}

/// The reference stock a mill device is checked and previewed on: a 60 × 40 × 30 block at the
/// origin, sized to fit the device where it can (a vise's opening, a plate's area).
fn ref_stock(r: &Record) -> Box3 {
    let fit = |k: &str, dflt: f64| r.opt(k).map_or(dflt, |v| (0.5 * v).min(dflt));
    let x = match r.kind {
        WKind::Vise | WKind::SelfCentring => fit("max_opening", 60.0),
        WKind::FixturePlate | WKind::Vacuum => fit("length", 60.0),
        WKind::Tombstone => fit("width", 60.0),
        _ => 60.0,
    };
    let y = match r.kind {
        WKind::FixturePlate | WKind::Vacuum => fit("width", 40.0),
        WKind::Tombstone => fit("height", 40.0),
        _ => 40.0,
    };
    let z = r.opt("jaw_height").map_or(30.0, |h| h.max(30.0));
    Box3 {
        min: [0.0; 3],
        max: [x, y, z],
    }
}

/// The device placed for a check or a preview: its bodies' prims by name, or the reason it
/// could not be placed.
fn place(r: &Record) -> Result<Vec<(String, Vec<Prim>)>, WhError> {
    let lib = Library::builtin();
    let s = ref_stock(r);
    let w = |p: Vec<forge_workholding::Placed>| forge_workholding::Workholding { placed: p }.fixtures(false);
    Ok(match r.kind {
        WKind::Vise | WKind::SelfCentring => {
            let jh = r.get("jaw_height")?;
            w(vec![mill::vise(
                "preview",
                r,
                &s,
                0,
                jh.min(s.size()[2]),
                None,
                None,
            )?])
        }
        WKind::Dovetail => {
            let t = 0.5 * (r.get("min_width")? + r.get("max_width")?);
            let s = Box3 {
                min: [0.0; 3],
                max: [t.max(r.get("body_length")?.min(t * 1.5)), 40.0, 30.0],
            };
            w(vec![mill::dovetail("preview", r, &s, 0, t)?])
        }
        WKind::SoftJaws => {
            let v = lib.get("standin-vise-150")?;
            let s = Box3 {
                min: [0.0; 3],
                max: [40.0, 40.0, r.get("height")?.max(30.0)],
            };
            w(vec![mill::vise(
                "preview",
                v,
                &s,
                0,
                r.get("height")?.min(s.size()[2]),
                None,
                Some(r),
            )?])
        }
        WKind::Parallels => {
            let v = lib.get("standin-vise-150")?;
            let jh = v.get("jaw_height")?;
            // the tallest parallel under the jaws' height: the shallowest grip
            let h = r
                .list("heights")?
                .iter()
                .copied()
                .filter(|h| *h < jh)
                .fold(0.0, f64::max);
            w(vec![mill::vise("preview", v, &s, 0, jh - h, Some(r), None)?])
        }
        WKind::StrapClamp => {
            let st = lib.get("standin-step-block-25")?;
            let l = r.get("length")?;
            w(vec![mill::strap_clamp(
                "preview",
                r,
                st,
                &s,
                0.0,
                [0.5 * s.max[0], s.max[1]],
                [0.0, -1.0],
                0.15 * l,
                0.5 * l,
            )?])
        }
        WKind::ToeClamp | WKind::EdgeClamp => w(vec![mill::side_clamp(
            "preview",
            r,
            &s,
            0.0,
            [0.0, 0.5 * s.max[1]],
            [1.0, 0.0],
        )?]),
        WKind::StepBlock => {
            let sc = lib.get("standin-strap-m12")?;
            let l = sc.get("length")?;
            w(vec![mill::strap_clamp(
                "preview",
                sc,
                r,
                &s,
                0.0,
                [0.5 * s.max[0], s.max[1]],
                [0.0, -1.0],
                0.15 * l,
                0.5 * l,
            )?])
        }
        WKind::FixturePlate => w(vec![mill::fixture_plate("preview", r, &s, [0.0, 0.0], &[])?]),
        WKind::ZeroPoint => {
            let v = lib.get("standin-centring-vise-80")?;
            let s = Box3 {
                min: [-20.0, -20.0, 0.0],
                max: [20.0, 20.0, 30.0],
            };
            let mut top = mill::vise("vise", v, &s, 0, v.get("jaw_height")?, None, None)?;
            let z = mill::zero_point("preview", r, None, &mut top, [0.0, 0.0], &[(0, 0)])?;
            w(vec![z])
        }
        WKind::Riser => {
            let zp = lib.get("standin-zero-point-52")?;
            let v = lib.get("standin-centring-vise-80")?;
            let s = Box3 {
                min: [-20.0, -20.0, 0.0],
                max: [20.0, 20.0, 30.0],
            };
            let mut top = mill::vise("vise", v, &s, 0, v.get("jaw_height")?, None, None)?;
            let z = mill::zero_point("preview", zp, Some(r), &mut top, [0.0, 0.0], &[(0, 0)])?;
            w(vec![z])
        }
        WKind::Tombstone => {
            let m = s.mid();
            w(vec![mill::tombstone("preview", r, &s, [m[0], m[1]])?])
        }
        WKind::Vacuum => w(vec![mill::vacuum("preview", r, &s)?]),
        WKind::Custom => w(vec![mill::custom(
            "preview",
            r,
            &forge_workholding::Pose::ID,
            vec![],
        )?]),
        WKind::Chuck => {
            let d = 0.5 * (r.get("grip_min")? + r.get("grip_max")?);
            lathe::chuck("preview", r, d, 0.0, r.get("jaw_len")?)?.cylinders()
        }
        WKind::Collet => {
            let d = r.get("range_max")?;
            lathe::collet("preview", r, d, 0.0)?.cylinders()
        }
        WKind::FaceDriver => {
            let d = 0.5 * (r.get("range_min")? + r.get("range_max")?);
            lathe::face_driver("preview", r, d, 0.0)?.cylinders()
        }
        WKind::Centre => lathe::centre("preview", r, 0.0, 2.0, r.get("body_d")?)?.cylinders(),
        WKind::SteadyRest => {
            let d = r.get("range_max")?;
            lathe::steady_rest("preview", r, d, 0.0)?.cylinders()
        }
    })
}

/// The record's checks: its reading (done by the parser: a record in hand has passed it) and
/// its placement on a reference stock or bar. Empty: it places.
pub fn check(r: &Record) -> Vec<Finding> {
    match place(r) {
        Ok(_) => Vec::new(),
        Err(e) => vec![finding(e)],
    }
}

/// The record's bodies for the builder's 3D view (part frame; a lathe device about +z from the
/// chuck face), or why there are none yet.
pub fn preview(r: &Record) -> Result<Vec<(String, Vec<Prim>)>, Finding> {
    place(r).map_err(finding)
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_library::{Catalog, Kind as _};

    #[test]
    fn builtin_set_loads_writes_byte_stable_and_every_kind_has_a_form() {
        let cat = Catalog::<Workholding>::from_sources(&[], &[]);
        assert!(cat.errors.is_empty(), "{:?}", cat.errors);
        assert!(cat.entries.len() > 90, "{}", cat.entries.len());
        let rs: Vec<Record> = cat.entries.iter().map(|e| e.def.clone()).collect();
        let t = Workholding::write("all", &rs).unwrap();
        let back = Workholding::parse(&t).unwrap();
        assert_eq!(back, rs);
        assert_eq!(Workholding::write("all", &back).unwrap(), t);
        for k in WKind::all() {
            assert!(!fields(k).is_empty() || k == WKind::Custom, "{}", k.name());
        }
        assert!(fields(WKind::Vacuum).iter().any(|f| f.unit == Unit::Pressure));
    }

    #[test]
    fn templates_place_and_products_say_what_they_lack() {
        for t in templates() {
            assert!(check(&t).is_empty(), "{}: {:?}", t.name, check(&t));
            assert!(!preview(&t).unwrap().is_empty(), "{}", t.name);
        }
        let l = Library::builtin();
        // a vise whose jaw thickness is unpublished still previews (its envelope)
        assert!(check(l.get("kurt-dx6").unwrap()).is_empty());
        // a chuck whose jaws are not published: not checked, naming the field
        let f = check(l.get("kitagawa-b208").unwrap());
        assert!(f[0].not_checked && !f[0].path.is_empty(), "{f:?}");
    }
}
