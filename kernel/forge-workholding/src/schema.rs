//! The field schema of every kind (for a library editor: MACHINE-BUILDER's shared builder), a
//! record's TOML, and a new record made from another (`forgecad workholding new NAME --from X`).
use crate::{Cite, KINDS, Kind, Library, Record, WhError};

/// What one field means.
#[derive(Clone, Debug, PartialEq)]
pub struct FieldSpec {
    pub name: &'static str,
    /// "mm", "N", "kPa", "deg", "rpm", "N·m", "count", "mm[]" (a list).
    pub unit: &'static str,
    pub required: bool,
    pub meaning: &'static str,
}

#[derive(Clone, Debug, PartialEq)]
pub struct KindSchema {
    pub kind: Kind,
    pub name: &'static str,
    pub about: &'static str,
    pub lathe: bool,
    pub fields: Vec<FieldSpec>,
}

/// (field, unit, meaning): one meaning per name across every kind.
pub const FIELDS: &[(&str, &str, &str)] = &[
    ("jaw_width", "mm", "along the jaw face, across the clamp axis"),
    (
        "jaw_height",
        "mm",
        "the jaw's gripping face height, from the bed to the jaw's top",
    ),
    ("jaw_thickness", "mm", "through the jaw, along the clamp axis"),
    ("max_opening", "mm", "the widest stock across the jaws"),
    ("min_opening", "mm", "the narrowest stock across the jaws"),
    (
        "body_length",
        "mm",
        "the body's length (along the clamp axis for a vise or dovetail, along the push for a clamp)",
    ),
    ("body_width", "mm", "the body's width across that"),
    (
        "height",
        "mm",
        "from the table (or the device's underside) to its top: a vise's jaw top, a dovetail's grip top, a clamp's top",
    ),
    ("clamp_force", "N", "the largest clamping force published"),
    ("weight", "kg", "the device's weight"),
    (
        "grip",
        "mm",
        "how much of the stock the jaws hold (a dovetail's depth, a serrated grip's depth)",
    ),
    (
        "repeatability",
        "mm",
        "the published repeatability or centring accuracy",
    ),
    ("angle", "deg", "a dovetail's or a centre point's included angle"),
    (
        "min_width",
        "mm",
        "the narrowest dovetail tenon the fixture takes",
    ),
    ("max_width", "mm", "the widest dovetail tenon the fixture takes"),
    (
        "width",
        "mm",
        "across (a blank's or plate's width; a strap's width; a parallel's thickness; a steady rest's axial width)",
    ),
    ("thickness", "mm", "a blank's or strap's or plate's thickness"),
    ("heights", "mm[]", "the heights a parallel set has"),
    ("length", "mm", "a parallel's, strap's or plate's length"),
    ("accuracy", "mm", "a parallel set's published accuracy"),
    ("stud_d", "mm", "the stud's or pull stud's diameter"),
    (
        "stud_above",
        "mm",
        "how far the stud and nut stand above the strap's top",
    ),
    ("slot", "mm", "the T-slot width it fits"),
    ("reach", "mm", "the strap's reach past the stud"),
    (
        "range_min",
        "mm",
        "the smallest size it holds (a collet's, chuck's, driver's, rest's diameter; a clamp's travel)",
    ),
    ("range_max", "mm", "the largest size it holds"),
    ("screw", "", "its screw (text, e.g. M8)"),
    ("torque", "N·m", "the screw's largest torque"),
    ("step", "mm", "a step block's rise per step"),
    ("steps", "count", "a step block's number of steps"),
    ("tread", "mm", "a step's depth"),
    ("pitch", "mm", "a hole grid's or zero-point grid's pitch"),
    ("hole_d", "mm", "a grid hole's diameter"),
    (
        "hole_offset",
        "mm",
        "the first hole's distance from the plate's corner, each way",
    ),
    ("dowel_d", "mm", "a dowel's diameter"),
    ("dowel_h", "mm", "how far a dowel stands above the plate"),
    ("module_d", "mm", "a zero-point module's diameter"),
    ("module_h", "mm", "a zero-point module's (or plate's) height"),
    ("pull_force", "N", "a zero-point module's pull-in force"),
    ("hold_force", "N", "a zero-point module's holding force"),
    ("d", "mm", "a riser's diameter"),
    ("depth", "mm", "a tombstone's depth behind a face"),
    ("faces", "count", "a tombstone's faces"),
    (
        "vacuum",
        "kPa",
        "the vacuum below atmosphere at the plate (the pump's: measure it)",
    ),
    (
        "seal_inset",
        "mm",
        "the gasket's inset from the part's edge, each side",
    ),
    ("flow", "m³/h", "the pump's flow"),
    ("jaws", "count", "a chuck's jaws"),
    ("body_d", "mm", "the body's outside diameter"),
    (
        "body_len",
        "mm",
        "the body's length along the spindle, behind the jaws or the face",
    ),
    (
        "jaw_len",
        "mm",
        "how far the jaws stand in front of the body (their grip length along the bar)",
    ),
    ("grip_min", "mm", "the smallest diameter the jaws grip"),
    ("grip_max", "mm", "the largest diameter the jaws grip"),
    ("max_rpm", "rpm", "the largest speed published"),
    ("through_hole", "mm", "the bore through the chuck"),
    ("stroke", "mm", "the jaws' stroke on the diameter"),
    ("grip_force", "N", "the largest gripping force published"),
    (
        "nose_d",
        "mm",
        "the nose's (or the collet nut's) outside diameter",
    ),
    ("nose_len", "mm", "the nose's length in front of the body"),
    ("collet_len", "mm", "the collet's length"),
    ("collet_d", "mm", "the collet's outside diameter"),
    ("pins", "count", "a face driver's drive pins"),
    (
        "point_len",
        "mm",
        "a centre's point projection (tip to the head's face)",
    ),
    ("quill_d", "mm", "the body's diameter behind the head"),
    ("frame_r", "mm", "the steady rest's frame radius about the axis"),
];

const ABOUT: &[(Kind, &str)] = &[
    (
        Kind::Vise,
        "a machine vise: a fixed and a moving jaw closing on the stock, on the vise's bed or parallels",
    ),
    (
        Kind::SelfCentring,
        "a self-centring vise: both jaws close on the centre (5-axis)",
    ),
    (
        Kind::Dovetail,
        "a dovetail fixture: grips a tenon machined on the stock's underside (5-axis)",
    ),
    (Kind::SoftJaws, "a soft-jaw blank, machined to the part"),
    (Kind::Parallels, "a parallel set the stock sits on in a vise"),
    (
        Kind::StrapClamp,
        "a strap clamp on a T-slot stud, its heel on a step block",
    ),
    (
        Kind::ToeClamp,
        "a low-profile toe clamp pushing on the stock's side",
    ),
    (Kind::StepBlock, "a step block carrying a strap's heel"),
    (
        Kind::EdgeClamp,
        "an edge clamp (Mitee-Bite style) pushing on the stock's side",
    ),
    (Kind::FixturePlate, "a fixture plate with a hole grid and dowels"),
    (
        Kind::ZeroPoint,
        "a zero-point module or plate on a published grid",
    ),
    (Kind::Riser, "a round riser under a 5-axis device"),
    (Kind::Tombstone, "a tombstone (HMC) the stock seats on"),
    (
        Kind::Vacuum,
        "a vacuum plate; its hold-down is a stated check with the user's inputs",
    ),
    (
        Kind::Custom,
        "the user's own fixture, given as bodies (boxes, cylinders) with its seat at z = 0",
    ),
    (Kind::Chuck, "a lathe chuck (3 or 4 jaws)"),
    (Kind::Collet, "a collet chuck or nose (5C, 16C, ER)"),
    (Kind::FaceDriver, "a face driver (needs a tailstock centre)"),
    (Kind::Centre, "a tailstock centre"),
    (Kind::SteadyRest, "a steady rest"),
];

fn spec(name: &'static str, required: bool) -> FieldSpec {
    let (unit, meaning) = FIELDS
        .iter()
        .find(|f| f.0 == name)
        .map(|f| (f.1, f.2))
        .unwrap_or(("", ""));
    FieldSpec {
        name,
        unit,
        required,
        meaning,
    }
}

/// Every kind's schema.
pub fn schema() -> Vec<KindSchema> {
    KINDS
        .iter()
        .map(|(k, n, need, may)| KindSchema {
            kind: *k,
            name: n,
            about: ABOUT.iter().find(|a| a.0 == *k).map(|a| a.1).unwrap_or(""),
            lathe: k.is_lathe(),
            fields: need
                .iter()
                .map(|f| spec(f, true))
                .chain(may.iter().map(|f| spec(f, false)))
                .collect(),
        })
        .collect()
}

fn js(s: &str) -> String {
    let mut o = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

/// The schema as JSON (`forgecad.workholding-schema/1`): kinds, each with its fields.
pub fn schema_json() -> String {
    let kinds: Vec<String> = schema()
        .iter()
        .map(|k| {
            let f: Vec<String> = k
                .fields
                .iter()
                .map(|f| {
                    format!(
                        "{{\"name\":{},\"unit\":{},\"required\":{},\"meaning\":{}}}",
                        js(f.name),
                        js(f.unit),
                        f.required,
                        js(f.meaning)
                    )
                })
                .collect();
            format!(
                "{{\"kind\":{},\"about\":{},\"lathe\":{},\"fields\":[{}]}}",
                js(k.name),
                js(k.about),
                k.lathe,
                f.join(",")
            )
        })
        .collect();
    format!(
        "{{\"schema\":\"forgecad.workholding-schema/1\",\"provenance\":[\"cited\",\"standin\",\"user\",\"unknown\",\"conflict\"],\"kinds\":[{}]}}",
        kinds.join(",")
    )
}

fn num(x: f64) -> String {
    if x.fract() == 0.0 && x.abs() < 1e15 {
        format!("{}", x as i64)
    } else {
        format!("{x}")
    }
}

impl Record {
    /// The record as our TOML (reads back to an equal record; custom bodies are written as
    /// their boxes and cylinders).
    pub fn to_toml(&self) -> String {
        let mut o = String::from("[[workholding]]\n");
        let q = |k: &str, v: &str| format!("{k} = {}\n", js(v));
        o += &q("name", &self.name);
        o += &q("kind", self.kind.name());
        if self.standin {
            o += "standin = true\n";
            o += &q("basis", &self.source);
        } else {
            if !self.vendor.is_empty() {
                o += &q("vendor", &self.vendor);
            }
            if !self.product.is_empty() {
                o += &q("product", &self.product);
            }
            if !self.source.is_empty() {
                o += &q("source", &self.source);
            }
        }
        if !self.note.is_empty() {
            o += &q("note", &self.note);
        }
        if !self.unknown.is_empty() {
            let u: Vec<String> = self.unknown.iter().map(|x| js(x)).collect();
            o += &format!("unknown = [{}]\n", u.join(", "));
        }
        if !self.conflict.is_empty() {
            let u: Vec<String> = self.conflict.iter().map(|x| js(x)).collect();
            o += &format!("conflict = [{}]\n", u.join(", "));
        }
        for (k, v) in &self.vals {
            o += &format!("{k} = {}\n", num(*v));
        }
        for (k, v) in &self.lists {
            let s: Vec<String> = v.iter().map(|x| num(*x)).collect();
            o += &format!("{k} = [{}]\n", s.join(", "));
        }
        for (k, v) in &self.words {
            o += &q(k, v);
        }
        for c in &self.cites {
            let f: Vec<String> = c.fields.iter().map(|x| js(x)).collect();
            o += &format!(
                "[[workholding.cite]]\nfields = [{}]\nsource = {}\ntaken = {}\n",
                f.join(", "),
                js(&c.source),
                js(&c.taken)
            );
        }
        for (b, f) in &self.parts {
            o += &format!("[[workholding.part]]\nbody = {}\nfile = {}\n", js(b), js(f));
        }
        for b in &self.bodies {
            o += &format!(
                "[[workholding.body]]\nname = {}\nrole = {}\n",
                js(&b.name),
                js(b.role.name())
            );
            if let Some(p) = &b.part {
                o += &q("part", p);
            }
            for p in &b.prims {
                let v3 = |a: [f64; 3]| format!("[{}, {}, {}]", num(a[0]), num(a[1]), num(a[2]));
                match *p {
                    crate::Prim::Block { o: c, axes, size } if axes == crate::geom::IDENT => {
                        let hi = [c[0] + size[0], c[1] + size[1], c[2] + size[2]];
                        o += &format!("[[workholding.body.box]]\nmin = {}\nmax = {}\n", v3(c), v3(hi));
                    }
                    crate::Prim::Block { .. } => {
                        let (lo, hi) = p.bounds();
                        o += &format!(
                            "# an oriented block, written as its bounding box\n[[workholding.body.box]]\nmin = {}\nmax = {}\n",
                            v3(lo),
                            v3(hi)
                        );
                    }
                    crate::Prim::Cyl { base, axis, r, h } => {
                        o += &format!(
                            "[[workholding.body.cyl]]\nbase = {}\naxis = {}\nr = {}\nh = {}\n",
                            v3(base),
                            v3(axis),
                            num(r),
                            num(h)
                        );
                    }
                }
            }
        }
        o
    }
}

/// A new record named `name` made from the library's `from`: every value kept, each cited as
/// "user (from FROM: its source)", provenance user; values the user then gives replace them.
pub fn new_from(lib: &Library, name: &str, from: &str, given: &[(&str, f64)]) -> Result<Record, WhError> {
    if lib.get(name).is_ok() {
        return Err(WhError::Duplicate {
            name: name.to_string(),
            files: ["the library".into(), "the new record".into()],
        });
    }
    let src = lib.get(from)?;
    let mut r = src.clone();
    r.name = name.to_string();
    r.standin = false;
    r.vendor = if src.vendor.is_empty() {
        "user".into()
    } else {
        src.vendor.clone()
    };
    r.product = format!("derived from {from}");
    r.source = String::new();
    // the source's cites stay with the values they cite; a stand-in's numbers are cited as
    // copied from it, with its basis (not a maker's value); given values become the user's
    if src.standin {
        let mut fields: Vec<String> = r.vals.keys().chain(r.lists.keys()).cloned().collect();
        fields.sort();
        r.cites = if fields.is_empty() {
            vec![]
        } else {
            vec![Cite {
                fields,
                source: format!("stand-in (copied from {from}): {}", src.source),
                taken: String::new(),
            }]
        };
    }
    for (k, v) in given {
        r = r.with(k, *v)?;
        for c in &mut r.cites {
            if c.source == "given in the job" {
                c.source = "user".into();
            }
        }
    }
    Ok(r)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_field_has_a_meaning_and_records_round_trip() {
        for k in schema() {
            assert!(!k.about.is_empty(), "{}", k.name);
            for f in &k.fields {
                assert!(!f.meaning.is_empty(), "{}.{}", k.name, f.name);
            }
        }
        let j = schema_json();
        assert!(
            j.starts_with("{\"schema\":\"forgecad.workholding-schema/1\"") && j.contains("\"jaw_thickness\"")
        );
        let l = Library::builtin();
        for (r, _) in &l.records {
            let back = crate::parse_file("x.toml", &r.to_toml())
                .unwrap_or_else(|e| panic!("{}: {e}\n{}", r.name, r.to_toml()));
            let mut a = r.clone();
            a.taken.clear();
            let mut b = back[0].clone();
            b.taken.clear();
            assert_eq!(a, b, "{}", r.name);
        }
    }

    #[test]
    fn a_user_record_from_a_product() {
        let l = Library::builtin();
        let r = new_from(&l, "my-dx6", "kurt-dx6", &[("jaw_thickness", 22.0)]).unwrap();
        assert_eq!(r.get("jaw_thickness"), Ok(22.0));
        assert!(r.cite("jaw_thickness").unwrap().starts_with("user"));
        // a copied value keeps its maker's cite; a stand-in's copied numbers say so
        assert!(
            r.cite("jaw_width").unwrap().starts_with("https://"),
            "{:?}",
            r.cite("jaw_width")
        );
        let s = new_from(&l, "my-vise", "standin-vise-150", &[]).unwrap();
        assert!(
            s.cite("jaw_width")
                .unwrap()
                .starts_with("stand-in (copied from standin-vise-150)")
        );
        assert!(crate::parse_file("x.toml", &s.to_toml()).is_ok());
        // it reads back as a user's file would
        let back = crate::parse_file("cam/workholding/mine.toml", &r.to_toml()).unwrap();
        assert_eq!(back[0].get("jaw_thickness"), Ok(22.0));
        assert!(new_from(&l, "kurt-dx6", "kurt-dx6", &[]).is_err());
        assert!(new_from(&l, "x", "kurt-dx6", &[("nope", 1.0)]).is_err());
    }
}
