//! Design for manufacture: a part's sheets against the shop each is checked by (a vendor profile,
//! or a shop written in the part), every violation with its rule, the measured value against the
//! limit, and where it is: the feature and line that made it, the faces of the finished part to
//! light, and its place on the flat pattern. All of it is measured on the kernel's exact geometry,
//! the flat's lines and arcs and the B-rep, never a mesh (forge-sheet's checks); this module only
//! gathers and places it.
//!
//! `forgecad query F dfm` and the JSON tools answer `answer`; the app reads `report`.

use crate::error::{Error, Warning};
use crate::sheet::{Note, SheetErr, SheetNote, SheetOut};
use crate::syntax::Span;
use crate::vendor::VendorTag;
use crate::{Options, Run};
use forge_agent::json::Json;
use forge_math::{Point2, Point3};
use forge_sheet::{self as fs, CutId, JointId, TrimRef, Violation, WallId};
use forge_topo::Name;
use std::collections::HashMap;
use std::path::PathBuf;

/// One thing the shop would not make as drawn (or a refusal that kept the part from building).
#[derive(Clone, Debug)]
pub struct Item {
    /// The rule broken: the violation's or refusal's kind (`HoleNearBend`, `VendorThickness`).
    pub rule: String,
    /// "violation", "warning" (information a shop gives), "info", or "refused".
    pub severity: String,
    pub text: String,
    /// The feature it is about, and where it is written.
    pub feature: String,
    pub span: Span,
    pub line: usize,
    /// The value measured and the limit it is held to (mm, or degrees for an angle), where the
    /// rule is one number against another.
    pub measured: Option<f64>,
    pub limit: Option<f64>,
    /// "min" (measured must reach the limit) or "max" (must not pass it).
    pub sense: Option<&'static str>,
    pub unit: &'static str,
    /// The violation's own record, as `fg run --json` gives it.
    pub data: Json,
    pub fixes: Vec<Json>,
    /// The finished part's faces to light, and the anchor text that named them.
    pub faces: Vec<Name>,
    pub anchor: Option<String>,
    /// Where on the flat pattern (developed coordinates, as the pattern and its DXF are drawn):
    /// one point, or the two ends of a measured span.
    pub flat: Vec<Point2>,
    /// Where on the folded part, on its reference skin.
    pub at: Option<Point3>,
}

/// One sheet's report.
#[derive(Clone, Debug)]
pub struct SheetReport {
    pub sheet: String,
    pub thickness: f64,
    pub vendor: Option<VendorTag>,
    /// Whether a shop checked it at all (none: only hardware's own rows).
    pub checked: bool,
    /// The rules for its stock and their limits at its radius (None: not stated, not checked).
    pub rules: Vec<(&'static str, Option<f64>)>,
    /// The shop's offered operations (None: not stated).
    pub operations: Option<Vec<&'static str>>,
    pub items: Vec<Item>,
}

#[derive(Clone, Debug, Default)]
pub struct Report {
    pub sheets: Vec<SheetReport>,
    /// Refusals: the part did not build as written (a thickness the vendor does not stock, ..).
    pub refused: Vec<Item>,
    /// Handles of faces (`@f..`) by name, for the text and the JSON.
    pub handles: HashMap<Name, String>,
}

impl Report {
    pub fn count(&self) -> usize {
        self.refused.len()
            + self
                .sheets
                .iter()
                .flat_map(|s| &s.items)
                .filter(|i| i.severity == "violation")
                .count()
    }
}

fn line_of(src: &str, span: Span) -> usize {
    src.get(..(span.lo as usize).min(src.len()))
        .map_or(0, |s| s.bytes().filter(|&b| b == b'\n').count() + 1)
}

/// The measured value, its limit and which way the limit holds, where one number is held to
/// another.
fn numbers(v: &Violation) -> (Option<f64>, Option<f64>, Option<&'static str>, &'static str) {
    use Violation as V;
    let min = |m: f64, l: f64| (Some(m), Some(l), Some("min"), "mm");
    let max = |m: f64, l: f64| (Some(m), Some(l), Some("max"), "mm");
    match *v {
        V::HoleNearBend { distance, limit, .. }
        | V::HoleNearEdge { distance, limit, .. }
        | V::HardwareNearEdge { distance, limit, .. }
        | V::HardwareNearHole { distance, limit, .. }
        | V::BendNearEdge { distance, limit, .. } => min(distance, limit),
        V::HoleTooSmall { diameter, limit, .. } => min(diameter, limit),
        V::SlotTooNarrow { width, limit, .. } | V::WebTooNarrow { width, limit, .. } => min(width, limit),
        V::FlangeTooShort { length, limit, .. } => min(length, limit),
        V::LipTooShort { width, need, .. } => min(width, need),
        V::ReliefUndersized {
            width,
            min_width,
            depth,
            min_depth,
            ..
        } => {
            if width < min_width {
                min(width, min_width)
            } else {
                min(depth, min_depth)
            }
        }
        V::BendTooLong { length, limit, .. } => max(length, limit),
        V::BoxFlangeTooTall { height, limit, .. } => max(height, limit),
        V::BendAngleOutOfRange {
            angle,
            min: lo,
            max: hi,
            ..
        } => {
            let (l, s) = if angle < lo { (lo, "min") } else { (hi, "max") };
            (Some(angle.to_degrees()), Some(l.to_degrees()), Some(s), "deg")
        }
        V::SheetTooHard { hardness, limit, .. } => (Some(hardness), Some(limit), Some("max"), "HRB"),
        V::HardwareThickness {
            thickness,
            min: lo,
            max: hi,
            ..
        } => {
            if thickness < lo {
                min(thickness, lo)
            } else {
                max(thickness, hi)
            }
        }
        V::FlatTooLarge { size, limit, .. } => max(size.1, limit.1),
        V::PartTooSmall { size, limit, .. } => {
            if size.0 < limit.0 {
                min(size.0, limit.0)
            } else {
                min(size.1, limit.1)
            }
        }
        V::RadiusNotTooled { radius, nearest, .. } => (Some(radius), Some(nearest), None, "mm"),
        V::ThicknessNotStocked {
            thickness, nearest, ..
        } => (Some(thickness), Some(nearest), None, "mm"),
        V::HardwareOverlapsHole { distance, hole, .. } => min(distance, 0.5 * hole),
        _ => (None, None, None, "mm"),
    }
}

/// Where a sheet's walls and bends lie, folded and flat.
struct Places<'a> {
    sheet: &'a fs::Sheet,
    flat: Option<fs::Layout>,
    folded: Option<&'a fs::Layout>,
}

impl Places<'_> {
    fn of(sh: &SheetOut) -> Places<'_> {
        Places {
            sheet: &sh.sheet,
            flat: fs::build_with(&sh.sheet, fs::Map::Flat).ok().map(|b| b.layout),
            folded: sh.layout.as_ref(),
        }
    }

    /// A point of wall `w`'s own coordinates: on the flat, and on the folded part.
    fn wall_point(&self, w: WallId, p: Point2) -> (Option<Point2>, Option<Point3>) {
        let flat = self
            .flat
            .as_ref()
            .and_then(|l| l.walls.iter().find(|x| x.wall == w))
            .map(|x| x.developed.apply(p));
        let folded = self
            .folded
            .and_then(|l| l.walls.iter().find(|x| x.wall == w))
            .map(|x| x.frame.point_to_world(Point3::new(p.x, p.y, 0.0)));
        (flat, folded)
    }

    /// A point of the flat: the wall it lies in, and where that is folded.
    fn flat_point(&self, q: Point2) -> Option<Point3> {
        let flat = self.flat.as_ref()?;
        for wp in &flat.walls {
            let d = q - wp.developed.origin;
            let y = forge_math::Vec2::new(-wp.developed.x.y, wp.developed.x.x);
            let local = Point2::new(d.dot(wp.developed.x), d.dot(y));
            let wall = self.sheet.wall(wp.wall)?;
            if wall.outline.first().is_some_and(|lp| inside(lp, local)) {
                return self.wall_point(wp.wall, local).1;
            }
        }
        None
    }

    /// The middle of a bend zone on the flat; on the folded part, the middle of its mold line on
    /// the flange (its x from the mold line, y along it).
    fn bend(&self, j: JointId) -> (Option<Point2>, Option<Point3>) {
        let flat = self
            .flat
            .as_ref()
            .and_then(|l| l.bends.iter().find(|b| b.joint == j))
            .map(|b| b.developed.apply(Point2::new(0.5 * b.allowance, 0.5 * b.length)));
        let folded = self.folded.and_then(|l| {
            let b = l.bends.iter().find(|b| b.joint == j)?;
            let w = l.walls.iter().find(|x| x.wall == b.b)?;
            Some(w.frame.point_to_world(Point3::new(0.0, 0.5 * b.length, 0.0)))
        });
        (flat, folded)
    }

    fn trim(&self, t: TrimRef) -> (Option<Point2>, Option<Point3>) {
        match t {
            TrimRef::Cut { cut, .. } => self.cut(cut),
            TrimRef::Outline { wall, segment } => {
                let Some(w) = self.sheet.wall(wall) else {
                    return (None, None);
                };
                let lp = w
                    .outline
                    .iter()
                    .find(|lp| lp.segments.iter().any(|s| s.1 == segment));
                match lp {
                    Some(lp) => self.wall_point(wall, middle(lp)),
                    None => (None, None),
                }
            }
            _ => (None, None),
        }
    }

    fn cut(&self, c: CutId) -> (Option<Point2>, Option<Point3>) {
        match self.sheet.cuts.iter().find(|x| x.id == c) {
            Some(cut) => self.wall_point(cut.wall, middle(&cut.lp)),
            None => (None, None),
        }
    }
}

/// A loop's middle: the mean of points along it.
fn middle(lp: &forge_ops::ProfileLoop) -> Point2 {
    let pts: Vec<Point2> = lp.segments.iter().flat_map(|(s, _)| s.samples(16)).collect();
    let n = pts.len().max(1) as f64;
    let (x, y) = pts.iter().fold((0.0, 0.0), |a, p| (a.0 + p.x, a.1 + p.y));
    Point2::new(x / n, y / n)
}

fn inside(lp: &forge_ops::ProfileLoop, q: Point2) -> bool {
    let poly: Vec<Point2> = lp.segments.iter().flat_map(|(s, _)| s.samples(16)).collect();
    let mut c = false;
    for i in 0..poly.len() {
        let (a, b) = (poly[i], poly[(i + 1) % poly.len()]);
        if (a.y > q.y) != (b.y > q.y) && q.x < a.x + (q.y - a.y) * (b.x - a.x) / (b.y - a.y) {
            c = !c;
        }
    }
    c
}

/// Where a violation is: its flat points, its folded point, and the role whose faces to light.
fn place(v: &Violation, p: &Places, owner: &str, sheet: &str) -> (Vec<Point2>, Option<Point3>, String) {
    use Violation as V;
    let one = |(f, a): (Option<Point2>, Option<Point3>), role: String| (f.into_iter().collect(), a, role);
    match *v {
        V::HoleNearBend { trim, .. }
        | V::HoleNearEdge { trim, .. }
        | V::HoleTooSmall { trim, .. }
        | V::SlotTooNarrow { trim, .. }
        | V::CutCrossesBend { trim, .. } => one(p.trim(trim), format!("{owner}.faces")),
        V::HardwareThickness { cut, .. }
        | V::HardwareNearEdge { cut, .. }
        | V::HardwareOverlapsHole { cut, .. }
        | V::HardwareNearHole { cut, .. }
        | V::LipTooShort { cut, .. }
        | V::SheetTooHard { cut, .. }
        | V::HardnessUnknown { cut, .. } => one(p.cut(cut), format!("{owner}.faces")),
        V::RadiusNotTooled { joint, .. }
        | V::BendTooLong { joint, .. }
        | V::BendAngleOutOfRange { joint, .. }
        | V::ReliefUndersized { joint, .. }
        | V::BoxFlangeTooTall { joint, .. } => one(p.bend(joint), format!("{owner}.bend")),
        V::FlangeTooShort { joint, .. } => one(p.bend(joint), format!("{owner}.faces")),
        V::BendNearEdge { joint, at, .. } => {
            let (bend, _) = p.bend(joint);
            let mut flat = vec![at];
            flat.extend(bend);
            (flat, p.flat_point(at), format!("{owner}.bend"))
        }
        V::WebTooNarrow { at, to, .. } => {
            let mid = Point2::new(0.5 * (at.x + to.x), 0.5 * (at.y + to.y));
            (vec![at, to], p.flat_point(mid), String::new())
        }
        V::OperationNotOffered { on, .. } => match on {
            fs::OpOn::Joint(j) => one(p.bend(j), format!("{owner}.bend")),
            fs::OpOn::Cut(c) => one(p.cut(c), format!("{owner}.faces")),
            _ => (vec![], None, format!("{owner}.faces")),
        },
        V::Form { .. } | V::Punch { .. } => (vec![], None, format!("{owner}.faces")),
        V::ThicknessNotStocked { .. } | V::FlatTooLarge { .. } | V::PartTooSmall { .. } => {
            (vec![], None, format!("{sheet}.faces"))
        }
    }
}

fn item_of(src: &str, n: &SheetNote, run: &Run, p: Option<&Places>, sheet: &str) -> Item {
    let w = Warning::Sheet(Box::new(n.clone()));
    let j = w.json(src);
    let (measured, limit, sense, unit) = match &n.note {
        Note::Violation(v) => numbers(v),
        Note::Warning(_) => (None, None, None, "mm"),
    };
    let (flat, at, role) = match (&n.note, p) {
        (Note::Violation(v), Some(p)) => place(v, p, &n.id, sheet),
        _ => (vec![], None, format!("{}.faces", n.id)),
    };
    let faces = run
        .anchors
        .as_ref()
        .and_then(|a| {
            a.role_faces
                .get(&role)
                .or_else(|| a.role_faces.get(&format!("{}.faces", n.id)))
        })
        .cloned()
        .unwrap_or_default();
    Item {
        rule: n.kind().into(),
        severity: n.severity().into(),
        text: n.text(),
        feature: n.id.clone(),
        span: n.span,
        line: line_of(src, n.span),
        measured,
        limit,
        sense,
        unit,
        data: j.get("data").cloned().unwrap_or(Json::Null),
        fixes: match j.get("fixes") {
            Some(Json::Arr(xs)) => xs.clone(),
            _ => vec![],
        },
        anchor: (!faces.is_empty()).then_some(role),
        faces,
        flat,
        at,
    }
}

fn refusal(src: &str, e: &Error) -> Item {
    let j = e.json(src);
    let span = e.span().unwrap_or_default();
    let (measured, limit) = match e {
        Error::Sheet(s) => match &**s {
            SheetErr::VendorThickness {
                thickness, offered, ..
            } => (
                Some(*thickness),
                offered
                    .iter()
                    .map(|x| x.0)
                    .min_by(|a, b| (a - thickness).abs().total_cmp(&(b - thickness).abs())),
            ),
            _ => (None, None),
        },
        _ => (None, None),
    };
    Item {
        rule: e.kind().into(),
        severity: "refused".into(),
        text: e.text(),
        feature: j
            .get("data")
            .and_then(|d| d.get("id"))
            .and_then(Json::str)
            .unwrap_or("")
            .into(),
        span,
        line: line_of(src, span),
        measured,
        limit,
        sense: None,
        unit: "mm",
        data: j.get("data").cloned().unwrap_or(Json::Null),
        fixes: match j.get("fixes") {
            Some(Json::Arr(xs)) => xs.clone(),
            _ => vec![],
        },
        faces: vec![],
        anchor: None,
        flat: vec![],
        at: None,
    }
}

/// The stock's rules at the sheet's radius, as checked (None: not stated).
fn rules(sh: &SheetOut) -> Vec<(&'static str, Option<f64>)> {
    let t = sh.sheet.gauge.thickness;
    let r = sh.sheet.gauge.radius;
    let Some(stock) = sh.shop.as_ref().and_then(|s| s.stock(t)) else {
        return vec![];
    };
    let at = |s: fs::Scaled| (s != fs::Scaled::NONE).then(|| s.at(t, r));
    let shop = sh.shop.as_ref().expect("a stock is a shop's");
    let p = &shop.process;
    let bent = sh
        .sheet
        .joints
        .iter()
        .any(|j| matches!(j.kind, fs::JointKind::Bend { .. }));
    let flat_max = if sh.sheet.coated {
        p.coated_max
    } else if bent {
        p.bent_max.or(p.flat_max)
    } else {
        p.flat_max
    };
    vec![
        ("min_flange", at(stock.min_flange)),
        ("hole_to_bend", at(stock.hole_to_bend)),
        ("hole_to_edge", at(stock.hole_to_edge)),
        ("bend_to_edge", at(stock.bend_to_edge)),
        ("min_hole", at(stock.min_hole)),
        ("min_slot", at(stock.min_slot)),
        ("min_web", at(stock.min_web)),
        ("relief_width", Some(stock.relief.bend.width.at(t, r))),
        ("relief_depth", Some(stock.relief.bend.depth.at(t, r))),
        (
            "max_bend_angle",
            stock.angles.or(p.angles).map(|a| a.1.to_degrees()),
        ),
        ("max_bend_length", stock.bend_length_max.or(p.bend_length_max)),
        ("max_flat_long", flat_max.map(|x| x.1)),
        ("max_flat_short", flat_max.map(|x| x.0)),
    ]
}

/// The DFM report of a part's source.
pub fn report(src: &str, dir: Option<PathBuf>, root: Option<PathBuf>) -> Report {
    let opts = Options {
        dir,
        root,
        anchors: true,
        ..Options::default()
    };
    let (run, _) = crate::run(src, opts, Default::default());
    let mut out = Report {
        handles: run
            .body
            .as_ref()
            .map(forge_agent::select::handles)
            .unwrap_or_default(),
        ..Report::default()
    };
    out.refused = run.errors.iter().map(|e| refusal(src, e)).collect();
    for sh in &run.sheets {
        let places = Places::of(sh);
        let mut items = vec![];
        for w in &run.warnings {
            let Warning::Sheet(n) = w else { continue };
            let mine = match &n.note {
                Note::Violation(v) => violation_feature(v) == sh.sheet.feature,
                Note::Warning(_) => false,
            };
            if mine {
                items.push(item_of(src, n, &run, Some(&places), &sh.id));
            }
        }
        out.sheets.push(SheetReport {
            sheet: sh.id.clone(),
            thickness: sh.sheet.gauge.thickness,
            vendor: sh.vendor.clone(),
            checked: sh.shop.is_some(),
            rules: rules(sh),
            operations: sh
                .shop
                .as_ref()
                .and_then(|s| s.operations.as_ref())
                .map(|ops| ops.iter().map(|o| o.name()).collect()),
            items,
        });
    }
    out
}

fn violation_feature(v: &Violation) -> fs::FeatureId {
    use Violation as V;
    match *v {
        V::Form { feature, .. }
        | V::Punch { feature, .. }
        | V::HoleNearBend { feature, .. }
        | V::HoleNearEdge { feature, .. }
        | V::CutCrossesBend { feature, .. }
        | V::HoleTooSmall { feature, .. }
        | V::ReliefUndersized { feature, .. }
        | V::HardwareThickness { feature, .. }
        | V::HardwareNearEdge { feature, .. }
        | V::FlatTooLarge { feature, .. }
        | V::PartTooSmall { feature, .. }
        | V::BendTooLong { feature, .. }
        | V::BoxFlangeTooTall { feature, .. }
        | V::BendAngleOutOfRange { feature, .. }
        | V::HardwareOverlapsHole { feature, .. }
        | V::HardwareNearHole { feature, .. }
        | V::LipTooShort { feature, .. }
        | V::SheetTooHard { feature, .. }
        | V::HardnessUnknown { feature, .. }
        | V::FlangeTooShort { feature, .. }
        | V::RadiusNotTooled { feature, .. }
        | V::ThicknessNotStocked { feature, .. }
        | V::SlotTooNarrow { feature, .. }
        | V::BendNearEdge { feature, .. }
        | V::WebTooNarrow { feature, .. }
        | V::OperationNotOffered { feature, .. } => feature,
    }
}

fn r6(v: f64) -> Json {
    Json::Num((v * 1e6).round() / 1e6)
}

fn p2(p: Point2) -> Json {
    Json::Arr(vec![r6(p.x), r6(p.y)])
}

impl Item {
    pub fn json(&self, handles: &HashMap<Name, String>) -> Json {
        let opt = |v: Option<f64>| v.map_or(Json::Null, r6);
        Json::obj(vec![
            ("rule", self.rule.as_str().into()),
            ("severity", self.severity.as_str().into()),
            ("text", self.text.as_str().into()),
            ("feature", self.feature.as_str().into()),
            ("line", self.line.into()),
            ("measured", opt(self.measured)),
            ("limit", opt(self.limit)),
            ("sense", self.sense.map_or(Json::Null, Json::from)),
            ("unit", self.unit.into()),
            (
                "location",
                Json::obj(vec![
                    ("anchor", self.anchor.as_deref().map_or(Json::Null, Json::from)),
                    (
                        "faces",
                        Json::Arr(
                            self.faces
                                .iter()
                                .filter_map(|n| handles.get(n))
                                .map(|h| h.as_str().into())
                                .collect(),
                        ),
                    ),
                    ("flat", Json::Arr(self.flat.iter().map(|p| p2(*p)).collect())),
                    (
                        "at",
                        self.at
                            .map_or(Json::Null, |p| Json::Arr(vec![r6(p.x), r6(p.y), r6(p.z)])),
                    ),
                ]),
            ),
            ("data", self.data.clone()),
            ("fixes", Json::Arr(self.fixes.clone())),
        ])
    }
}

pub fn vendor_json(v: &VendorTag) -> Json {
    Json::obj(vec![
        ("id", v.id.as_str().into()),
        ("name", v.name.as_str().into()),
        ("material", v.material.as_str().into()),
        ("source", v.source.as_deref().map_or(Json::Null, Json::from)),
        ("taken", v.taken.as_deref().map_or(Json::Null, Json::from)),
        ("note", v.note.as_deref().map_or(Json::Null, Json::from)),
        (
            "tolerance",
            Json::Obj(
                v.tolerance
                    .iter()
                    .map(|(k, t)| (k.clone(), Json::from(t.as_str())))
                    .collect(),
            ),
        ),
        ("verify", forge_vendor::VERIFY.into()),
    ])
}

impl Report {
    pub fn json(&self) -> Json {
        let sheets = self
            .sheets
            .iter()
            .map(|s| {
                Json::obj(vec![
                    ("sheet", s.sheet.as_str().into()),
                    ("thickness", r6(s.thickness)),
                    ("checked", s.checked.into()),
                    ("vendor", s.vendor.as_ref().map_or(Json::Null, vendor_json)),
                    (
                        "rules",
                        Json::Obj(
                            s.rules
                                .iter()
                                .map(|(k, v)| (k.to_string(), v.map_or(Json::Null, r6)))
                                .collect(),
                        ),
                    ),
                    (
                        "operations",
                        s.operations.as_ref().map_or(Json::Null, |o| {
                            Json::Arr(o.iter().map(|x| Json::from(*x)).collect())
                        }),
                    ),
                    (
                        "violations",
                        Json::Arr(s.items.iter().map(|i| i.json(&self.handles)).collect()),
                    ),
                ])
            })
            .collect();
        Json::obj(vec![
            ("ok", true.into()),
            ("query", "dfm".into()),
            ("count", self.count().into()),
            (
                "refused",
                Json::Arr(self.refused.iter().map(|i| i.json(&self.handles)).collect()),
            ),
            ("sheets", Json::Arr(sheets)),
        ])
    }

    pub fn text(&self) -> String {
        let mut t = String::new();
        let fm = crate::values::fmt;
        for i in &self.refused {
            t.push_str(&format!("refused  line {}  {}: {}\n", i.line, i.rule, i.text));
        }
        if self.sheets.is_empty() && self.refused.is_empty() {
            t.push_str("no sheet metal in the part: nothing to check\n");
        }
        for s in &self.sheets {
            match &s.vendor {
                Some(v) => t.push_str(&format!(
                    "{}: {} {} {} mm, data of {} ({}). {}\n",
                    s.sheet,
                    v.name,
                    v.material,
                    fm(s.thickness),
                    v.taken.as_deref().unwrap_or("undated"),
                    v.source.as_deref().unwrap_or("no source"),
                    forge_vendor::VERIFY
                )),
                None if s.checked => t.push_str(&format!("{}: the part's own shop\n", s.sheet)),
                None => t.push_str(&format!(
                    "{}: no shop or vendor; only hardware's own rows checked\n",
                    s.sheet
                )),
            }
            if s.items.is_empty() {
                t.push_str("  no violations\n");
            }
            for i in &s.items {
                let nums = match (i.measured, i.limit, i.sense) {
                    (Some(m), Some(l), Some(sense)) => {
                        format!(
                            " ({} {} {} {})",
                            fm(m),
                            if sense == "min" { "<" } else { ">" },
                            fm(l),
                            i.unit
                        )
                    }
                    _ => String::new(),
                };
                let faces: Vec<&str> = i
                    .faces
                    .iter()
                    .filter_map(|n| self.handles.get(n))
                    .map(String::as_str)
                    .collect();
                let at = if faces.is_empty() {
                    String::new()
                } else {
                    format!("  [{}]", faces.join(" "))
                };
                t.push_str(&format!(
                    "  {:<9} line {:<4} {:<20} {}: {}{nums}{at}\n",
                    i.severity, i.line, i.rule, i.feature, i.text
                ));
            }
        }
        t.push_str(&format!("{} to fix\n", self.count()));
        t
    }
}

/// `query F dfm`.
pub fn answer(src: &str, dir: Option<PathBuf>, root: Option<PathBuf>) -> crate::query::Answer {
    let r = report(src, dir, root);
    crate::query::Answer {
        ok: true,
        json: r.json(),
        text: r.text(),
    }
}
