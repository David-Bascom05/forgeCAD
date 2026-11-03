//! The flat pattern as 2D drawing data and as DXF: the cut contours (each closed, in order) and a
//! line down the middle of each bend zone, all in the first wall's own coordinates (the plane the
//! flat body lies on). Curves are exact: lines, arcs and the trims' own NURBS.
use crate::error::JointId;
use forge_math::{Point2, dmath};
use std::fmt::Write;

#[derive(Clone, Debug, PartialEq)]
pub enum Curve2 {
    Line {
        a: Point2,
        b: Point2,
    },
    /// Counter-clockwise from `start` when `sweep` is positive (radians).
    Arc {
        c: Point2,
        r: f64,
        start: f64,
        sweep: f64,
    },
    /// A clamped NURBS: `knots` has `points.len() + degree + 1` values.
    Spline {
        degree: usize,
        knots: Vec<f64>,
        points: Vec<Point2>,
        weights: Option<Vec<f64>>,
    },
}

impl Curve2 {
    pub fn ends(&self) -> (Point2, Point2) {
        match self {
            Curve2::Line { a, b } => (*a, *b),
            Curve2::Arc { c, r, start, sweep } => {
                let at = |t: f64| {
                    let (s, k) = dmath::sin_cos(t);
                    Point2::new(c.x + r * k, c.y + r * s)
                };
                (at(*start), at(start + sweep))
            }
            Curve2::Spline { points, .. } => (points[0], points[points.len() - 1]),
        }
    }
}

/// A bend's line on the flat: the middle of its zone, from one end of the mold line to the other.
#[derive(Clone, Debug, PartialEq)]
pub struct BendLine {
    pub joint: JointId,
    pub a: Point2,
    pub b: Point2,
    /// Signed as `JointKind::Bend`: positive turns towards the reference skin.
    pub angle: f64,
    pub radius: f64,
    /// The zone's width on the flat.
    pub allowance: f64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Pattern {
    pub contours: Vec<Vec<Curve2>>,
    pub bends: Vec<BendLine>,
    /// Turret hits: tool, centre and angle on the flat.
    pub hits: Vec<crate::punch::Hit>,
    /// Forms: tool, centre, angle and direction on the flat.
    pub forms: Vec<crate::form::FormMark>,
}

/// Layers the DXF puts things on.
pub const CUT_LAYER: &str = "CUT";
pub const BEND_LAYER: &str = "BEND";
/// What a shop reads off the drawing: each bend's direction, angle and inside radius, and the
/// part's material, thickness and bend rule.
pub const NOTES_LAYER: &str = "BEND_NOTES";
/// Turret hits: a point at each hit's centre and the tool id as text, turned to the hit.
pub const PUNCH_LAYER: &str = "PUNCH";
/// Forms: a point at each form's centre and "TOOL UP" or "TOOL DOWN" as text, turned to it.
pub const FORM_LAYER: &str = "FORM";

/// The pattern as an AutoCAD R2000 (AC1015) DXF in millimetres: contours on `CUT_LAYER`, bend
/// lines on `BEND_LAYER`. Lines, arcs, circles (a whole-turn arc) and NURBS splines, exactly.
pub fn dxf(p: &Pattern) -> String {
    write_dxf(p, None)
}

/// `dxf`, with notes on `NOTES_LAYER`: at each bend line's middle, along it, `UP 90.0 DEG IR
/// 2.00` (UP: the flange turns towards the drawing's viewer, the reference skin; DOWN: away), and
/// `part` below the pattern.
pub fn dxf_noted(p: &Pattern, part: &str) -> String {
    write_dxf(p, Some(part))
}

/// A bend's note: its direction as the drawing is seen (from the reference skin), its angle and
/// its inside radius.
pub fn bend_note(b: &BendLine) -> String {
    format!(
        "{} {:.1} DEG IR {:.2}",
        if b.angle >= 0.0 { "UP" } else { "DOWN" },
        b.angle.abs().to_degrees(),
        b.radius
    )
}

/// The notes' text height (mm).
const NOTE_H: f64 = 3.0;

fn write_dxf(p: &Pattern, notes: Option<&str>) -> String {
    let mut w = Dxf::default();
    // handles: tables and blocks first, entities after
    let ms_rec = 0x1F;
    let ps_rec = 0x1E;
    w.pair(0, "SECTION");
    w.pair(2, "HEADER");
    w.pair(9, "$ACADVER");
    w.pair(1, "AC1015");
    w.pair(9, "$INSUNITS");
    w.pair(70, "4");
    w.pair(9, "$MEASUREMENT");
    w.pair(70, "1");
    // the handle seed is written last, once every handle is known
    w.pair(9, "$HANDSEED");
    let seed_at = w.out.len();
    w.pair(5, "SEED");
    w.pair(0, "ENDSEC");

    w.pair(0, "SECTION");
    w.pair(2, "TABLES");
    w.table("VPORT", 0x8, |w| {
        let h = w.handle();
        w.pair(0, "VPORT");
        w.hex(5, h);
        w.hex(330, 0x8);
        w.pair(100, "AcDbSymbolTableRecord");
        w.pair(100, "AcDbViewportTableRecord");
        w.pair(2, "*Active");
        w.pair(70, "0");
        for (c, v) in [(10, 0.0), (20, 0.0), (11, 1.0), (21, 1.0), (12, 0.0), (22, 0.0)] {
            w.num(c, v);
        }
        for (c, v) in [
            (13, 0.0),
            (23, 0.0),
            (14, 10.0),
            (24, 10.0),
            (15, 10.0),
            (25, 10.0),
        ] {
            w.num(c, v);
        }
        for (c, v) in [(16, 0.0), (26, 0.0), (36, 1.0), (17, 0.0), (27, 0.0), (37, 0.0)] {
            w.num(c, v);
        }
        for (c, v) in [
            (40, 100.0),
            (41, 1.0),
            (42, 50.0),
            (43, 0.0),
            (44, 0.0),
            (50, 0.0),
            (51, 0.0),
        ] {
            w.num(c, v);
        }
        for (c, v) in [
            (71, "0"),
            (72, "100"),
            (73, "1"),
            (74, "3"),
            (75, "0"),
            (76, "0"),
            (77, "0"),
            (78, "0"),
        ] {
            w.pair(c, v);
        }
    });
    w.table("LTYPE", 0x5, |w| {
        for name in ["ByBlock", "ByLayer", "Continuous"] {
            let h = w.handle();
            w.pair(0, "LTYPE");
            w.hex(5, h);
            w.hex(330, 0x5);
            w.pair(100, "AcDbSymbolTableRecord");
            w.pair(100, "AcDbLinetypeTableRecord");
            w.pair(2, name);
            w.pair(70, "0");
            w.pair(3, if name == "Continuous" { "Solid line" } else { "" });
            w.pair(72, "65");
            w.pair(73, "0");
            w.num(40, 0.0);
        }
    });
    w.table("LAYER", 0x2, |w| {
        let mut layers = vec![
            ("0", "7"),
            (CUT_LAYER, "7"),
            (BEND_LAYER, "1"),
            (PUNCH_LAYER, "5"),
        ];
        // the FORM layer only where there are forms: other parts' files stay as they were
        if !p.forms.is_empty() {
            layers.push((FORM_LAYER, "6"));
        }
        if notes.is_some() {
            layers.push((NOTES_LAYER, "3"));
        }
        for (name, colour) in layers {
            let h = w.handle();
            w.pair(0, "LAYER");
            w.hex(5, h);
            w.hex(330, 0x2);
            w.pair(100, "AcDbSymbolTableRecord");
            w.pair(100, "AcDbLayerTableRecord");
            w.pair(2, name);
            w.pair(70, "0");
            w.pair(62, colour);
            w.pair(6, "Continuous");
            w.pair(370, "-3");
            w.pair(390, "F");
        }
    });
    w.table("STYLE", 0x3, |w| {
        let h = w.handle();
        w.pair(0, "STYLE");
        w.hex(5, h);
        w.hex(330, 0x3);
        w.pair(100, "AcDbSymbolTableRecord");
        w.pair(100, "AcDbTextStyleTableRecord");
        w.pair(2, "Standard");
        w.pair(70, "0");
        w.num(40, 0.0);
        w.num(41, 1.0);
        w.num(50, 0.0);
        w.pair(71, "0");
        w.num(42, 2.5);
        w.pair(3, "txt");
        w.pair(4, "");
    });
    w.table("VIEW", 0x6, |_| {});
    w.table("UCS", 0x7, |_| {});
    w.table("APPID", 0x9, |w| {
        let h = w.handle();
        w.pair(0, "APPID");
        w.hex(5, h);
        w.hex(330, 0x9);
        w.pair(100, "AcDbSymbolTableRecord");
        w.pair(100, "AcDbRegAppTableRecord");
        w.pair(2, "ACAD");
        w.pair(70, "0");
    });
    w.table("DIMSTYLE", 0xA, |w| {
        let h = w.handle();
        w.pair(0, "DIMSTYLE");
        w.hex(105, h);
        w.hex(330, 0xA);
        w.pair(100, "AcDbSymbolTableRecord");
        w.pair(100, "AcDbDimStyleTableRecord");
        w.pair(2, "Standard");
        w.pair(70, "0");
    });
    w.table("BLOCK_RECORD", 0x1, |w| {
        for (name, h) in [("*Model_Space", ms_rec), ("*Paper_Space", ps_rec)] {
            w.pair(0, "BLOCK_RECORD");
            w.hex(5, h);
            w.hex(330, 0x1);
            w.pair(100, "AcDbSymbolTableRecord");
            w.pair(100, "AcDbBlockTableRecord");
            w.pair(2, name);
            w.pair(340, "0");
        }
    });
    w.pair(0, "ENDSEC");

    w.pair(0, "SECTION");
    w.pair(2, "BLOCKS");
    for (name, rec, paper) in [("*Model_Space", ms_rec, false), ("*Paper_Space", ps_rec, true)] {
        let (b, e) = (w.handle(), w.handle());
        w.pair(0, "BLOCK");
        w.hex(5, b);
        w.hex(330, rec);
        w.pair(100, "AcDbEntity");
        if paper {
            w.pair(67, "1");
        }
        w.pair(8, "0");
        w.pair(100, "AcDbBlockBegin");
        w.pair(2, name);
        w.pair(70, "0");
        for c in [10, 20, 30] {
            w.num(c, 0.0);
        }
        w.pair(3, name);
        w.pair(1, "");
        w.pair(0, "ENDBLK");
        w.hex(5, e);
        w.hex(330, rec);
        w.pair(100, "AcDbEntity");
        if paper {
            w.pair(67, "1");
        }
        w.pair(8, "0");
        w.pair(100, "AcDbBlockEnd");
    }
    w.pair(0, "ENDSEC");

    w.pair(0, "SECTION");
    w.pair(2, "ENTITIES");
    for c in p.contours.iter().flatten() {
        w.curve(c, CUT_LAYER, ms_rec);
    }
    for b in &p.bends {
        w.curve(&Curve2::Line { a: b.a, b: b.b }, BEND_LAYER, ms_rec);
    }
    if let Some(part) = notes {
        for b in &p.bends {
            // along the line, read left to right, a little off it
            let d = b.b - b.a;
            let mut ang = dmath::atan2(d.y, d.x);
            if ang > dmath::PI / 2.0 + 1e-9 {
                ang -= dmath::PI;
            } else if ang <= -dmath::PI / 2.0 + 1e-9 {
                ang += dmath::PI;
            }
            let (s, c) = dmath::sin_cos(ang);
            let mid = b.a + d * 0.5;
            let at = Point2::new(mid.x - s * NOTE_H * 0.5, mid.y + c * NOTE_H * 0.5);
            w.text(&bend_note(b), at, ang.to_degrees(), NOTES_LAYER, ms_rec);
        }
        let (lo, _) = bounds(p);
        w.text(
            part,
            Point2::new(lo.x, lo.y - 2.0 * NOTE_H),
            0.0,
            NOTES_LAYER,
            ms_rec,
        );
    }
    let marks = p
        .hits
        .iter()
        .map(|h| (h.at, h.angle, h.tool.clone(), PUNCH_LAYER));
    let forms = p.forms.iter().map(|f| {
        let way = if f.up { "UP" } else { "DOWN" };
        (f.at, f.angle, format!("{} {way}", f.tool), FORM_LAYER)
    });
    for (at, angle, text, layer) in marks.chain(forms) {
        let h = crate::punch::Hit {
            tool: text,
            at,
            angle,
        };
        let layer: &str = layer;
        w.head("POINT", layer, ms_rec);
        w.pair(100, "AcDbPoint");
        for (k, v) in [(10, h.at.x), (20, h.at.y), (30, 0.0)] {
            w.num(k, v);
        }
        w.head("TEXT", layer, ms_rec);
        w.pair(100, "AcDbText");
        for (k, v) in [
            (10, h.at.x),
            (20, h.at.y),
            (30, 0.0),
            (40, 1.0),
            (50, h.angle.to_degrees()),
        ] {
            w.num(k, v);
        }
        w.pair(1, &h.tool);
        w.pair(100, "AcDbText");
    }
    w.pair(0, "ENDSEC");

    w.pair(0, "SECTION");
    w.pair(2, "OBJECTS");
    let (root, group) = (0xC, 0xD);
    w.pair(0, "DICTIONARY");
    w.hex(5, root);
    w.pair(330, "0");
    w.pair(100, "AcDbDictionary");
    w.pair(281, "1");
    w.pair(3, "ACAD_GROUP");
    w.hex(350, group);
    w.pair(0, "DICTIONARY");
    w.hex(5, group);
    w.hex(330, root);
    w.pair(100, "AcDbDictionary");
    w.pair(281, "1");
    w.pair(0, "ENDSEC");
    w.pair(0, "EOF");

    let seed = format!("{:X}", w.next);
    let line = "  5\nSEED\n";
    let mut out = w.out;
    let at = seed_at + out[seed_at..].find(line).expect("the seed placeholder");
    out.replace_range(at..at + line.len(), &format!("  5\n{seed}\n"));
    out
}

struct Dxf {
    out: String,
    next: u32,
}

impl Default for Dxf {
    fn default() -> Dxf {
        // 0x1..0x1F are the fixed table and block-record handles above
        Dxf {
            out: String::new(),
            next: 0x20,
        }
    }
}

impl Dxf {
    fn pair(&mut self, code: u16, v: &str) {
        let _ = write!(self.out, "{code:>3}\n{v}\n");
    }
    fn num(&mut self, code: u16, v: f64) {
        // the shortest text that reads back as the same double
        let _ = write!(self.out, "{code:>3}\n{v:?}\n");
    }
    fn hex(&mut self, code: u16, h: u32) {
        let _ = write!(self.out, "{code:>3}\n{h:X}\n");
    }
    fn handle(&mut self) -> u32 {
        let h = self.next;
        self.next += 1;
        h
    }
    fn table(&mut self, name: &str, h: u32, rows: impl FnOnce(&mut Dxf)) {
        let start = self.out.len();
        rows(self);
        let body = self.out.split_off(start);
        let n = body.matches(&format!("  0\n{name}\n")).count();
        self.pair(0, "TABLE");
        self.pair(2, name);
        self.hex(5, h);
        self.pair(330, "0");
        self.pair(100, "AcDbSymbolTable");
        self.pair(70, &n.to_string());
        if name == "DIMSTYLE" {
            self.pair(100, "AcDbDimStyleTable");
        }
        self.out.push_str(&body);
        self.pair(0, "ENDTAB");
    }
    fn head(&mut self, kind: &str, layer: &str, owner: u32) {
        let h = self.handle();
        self.pair(0, kind);
        self.hex(5, h);
        self.hex(330, owner);
        self.pair(100, "AcDbEntity");
        self.pair(8, layer);
    }
    fn text(&mut self, text: &str, at: Point2, degrees: f64, layer: &str, owner: u32) {
        self.head("TEXT", layer, owner);
        self.pair(100, "AcDbText");
        for (k, v) in [(10, at.x), (20, at.y), (30, 0.0), (40, NOTE_H)] {
            self.num(k, v);
        }
        self.pair(1, text);
        self.num(50, degrees);
        self.pair(7, "Standard");
        self.pair(100, "AcDbText");
    }
    fn curve(&mut self, c: &Curve2, layer: &str, owner: u32) {
        match c {
            Curve2::Line { a, b } => {
                self.head("LINE", layer, owner);
                self.pair(100, "AcDbLine");
                for (k, v) in [(10, a.x), (20, a.y), (30, 0.0), (11, b.x), (21, b.y), (31, 0.0)] {
                    self.num(k, v);
                }
            }
            Curve2::Arc { c, r, start, sweep } => {
                let whole = sweep.abs() >= dmath::TAU * (1.0 - 1e-12);
                self.head(if whole { "CIRCLE" } else { "ARC" }, layer, owner);
                self.pair(100, "AcDbCircle");
                for (k, v) in [(10, c.x), (20, c.y), (30, 0.0), (40, *r)] {
                    self.num(k, v);
                }
                if !whole {
                    // DXF arcs run counter-clockwise from 50 to 51, in degrees
                    let (lo, hi) = if *sweep > 0.0 {
                        (*start, start + sweep)
                    } else {
                        (start + sweep, *start)
                    };
                    self.pair(100, "AcDbArc");
                    self.num(50, lo.to_degrees());
                    self.num(51, hi.to_degrees());
                }
            }
            Curve2::Spline {
                degree,
                knots,
                points,
                weights,
            } => {
                self.head("SPLINE", layer, owner);
                self.pair(100, "AcDbSpline");
                for (k, v) in [(210, 0.0), (220, 0.0), (230, 1.0)] {
                    self.num(k, v);
                }
                // planar, rational when weighted
                let flags = 8 + if weights.is_some() { 4 } else { 0 };
                self.pair(70, &flags.to_string());
                self.pair(71, &degree.to_string());
                self.pair(72, &knots.len().to_string());
                self.pair(73, &points.len().to_string());
                self.pair(74, "0");
                self.num(42, 1e-10);
                self.num(43, 1e-10);
                for k in knots {
                    self.num(40, *k);
                }
                for wt in weights.iter().flatten() {
                    self.num(41, *wt);
                }
                for q in points {
                    self.num(10, q.x);
                    self.num(20, q.y);
                    self.num(30, 0.0);
                }
            }
        }
    }
}

/// The corners of a box round the pattern's contours (arcs by their circles, splines by their
/// control points: never inside the curves).
fn bounds(p: &Pattern) -> (Point2, Point2) {
    let mut lo = Point2::new(f64::INFINITY, f64::INFINITY);
    let mut hi = Point2::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
    let mut add = |q: Point2| {
        lo = Point2::new(lo.x.min(q.x), lo.y.min(q.y));
        hi = Point2::new(hi.x.max(q.x), hi.y.max(q.y));
    };
    for c in p.contours.iter().flatten() {
        match c {
            Curve2::Line { a, b } => {
                add(*a);
                add(*b);
            }
            Curve2::Arc { c, r, .. } => {
                add(Point2::new(c.x - r, c.y - r));
                add(Point2::new(c.x + r, c.y + r));
            }
            Curve2::Spline { points, .. } => points.iter().for_each(|q| add(*q)),
        }
    }
    if lo.x > hi.x {
        return (Point2::new(0.0, 0.0), Point2::new(0.0, 0.0));
    }
    (lo, hi)
}
