//! The drafting commands, each a state machine in AutoCAD's words (see `dwgcmd.rs` for how they
//! are fed): the draw commands (LINE, PLINE, CIRCLE, ARC, RECTANG, POLYGON, ELLIPSE, SPLINE,
//! POINT, TEXT, MTEXT, HATCH, the dimensions) and the modify commands (ERASE, MOVE, COPY,
//! ROTATE, SCALE, MIRROR, OFFSET, TRIM, EXTEND, FILLET, CHAMFER, BREAK, JOIN, LENGTHEN,
//! STRETCH, ARRAY, EXPLODE, MATCHPROP), and grip editing.

use super::dwgcmd::{Command, Cx, Flow, Ghost, Inp, Prompt, Pv, n4, want};
use forge_dwg::draft::commit::{add_like, copy_entity, explode_entity, replace, transform};
use forge_dwg::draft::curve::{Arc as CArc, Ellipse, Poly};
use forge_dwg::draft::dim::{self, Kind as DimKind, Style as DimStyle};
use forge_dwg::draft::entity::{curve_of, read_entity, to_data, write_curve};
use forge_dwg::draft::fillet::{self, Chamfer};
use forge_dwg::draft::geom::{PI, TAU};
use forge_dwg::draft::grip::{self, Grip};
use forge_dwg::draft::make::{self, Fill};
use forge_dwg::draft::ops::{self, Lengthen};
use forge_dwg::draft::{Affine, Bbox, Curve, V2, boundary, offset, v2, xform};
use forge_dwg::file::Drawing;
use forge_dwg::types::Data;

/// A drafting command by its name.
pub fn make(name: &str) -> Option<Box<dyn Command>> {
    Some(match name {
        "LINE" => Box::new(LineCmd::default()),
        "PLINE" => Box::new(PlineCmd::default()),
        "CIRCLE" => Box::new(CircleCmd::default()),
        "ARC" => Box::new(ArcCmd::default()),
        "RECTANG" => Box::new(RectCmd::default()),
        "POLYGON" => Box::new(PolygonCmd::default()),
        "ELLIPSE" => Box::new(EllipseCmd::default()),
        "SPLINE" => Box::new(SplineCmd::default()),
        "POINT" => Box::new(PointCmd),
        "TEXT" => Box::new(TextCmd::default()),
        "MTEXT" => Box::new(MTextCmd::default()),
        "HATCH" => Box::new(HatchCmd::default()),
        "DIM" | "DIMLINEAR" => Box::new(DimLinCmd::new(false)),
        "DIMALIGNED" => Box::new(DimLinCmd::new(true)),
        "DIMRADIUS" => Box::new(DimRadCmd::new(false)),
        "DIMDIAMETER" => Box::new(DimRadCmd::new(true)),
        "ERASE" => Box::new(EraseCmd),
        "MOVE" => Box::new(XfCmd::new(Xf::Move)),
        "COPY" => Box::new(XfCmd::new(Xf::Copy)),
        "ROTATE" => Box::new(XfCmd::new(Xf::Rotate)),
        "SCALE" => Box::new(XfCmd::new(Xf::Scale)),
        "MIRROR" => Box::new(XfCmd::new(Xf::Mirror)),
        "STRETCH" => Box::new(XfCmd::new(Xf::Stretch)),
        "OFFSET" => Box::new(OffsetCmd::default()),
        "TRIM" => Box::new(TrimCmd { extend: false }),
        "EXTEND" => Box::new(TrimCmd { extend: true }),
        "FILLET" => Box::new(FilletCmd::new(false)),
        "CHAMFER" => Box::new(FilletCmd::new(true)),
        "BREAK" => Box::new(BreakCmd::default()),
        "JOIN" => Box::new(JoinCmd),
        "LENGTHEN" => Box::new(LengthenCmd::default()),
        "ARRAY" => Box::new(ArrayCmd::default()),
        "EXPLODE" => Box::new(ExplodeCmd),
        "MATCHPROP" => Box::new(MatchCmd::default()),
        "DIMSCALE" => Box::new(DimScaleCmd),
        _ => return None,
    })
}

// ---- helpers

fn curve(d: &Drawing, h: u32) -> Option<Curve> {
    curve_of(d, h).ok().map(|f| f.curve)
}

/// A new entity in the space, on the current layer.
fn add(cx: &mut Cx, data: Data) -> Option<u32> {
    match cx.d.add_entity(cx.hist, cx.space, cx.layer, data) {
        Ok(h) => Some(h),
        Err(e) => {
            cx.say(format!("Can't add it: {}", e.0));
            None
        }
    }
}

fn add_curve(cx: &mut Cx, c: &Curve) -> Option<u32> {
    let data = to_data(c, 0.0)?;
    add(cx, data)
}

fn erase(cx: &mut Cx, h: u32) {
    if let Err(e) = cx.d.erase(cx.hist, h) {
        cx.say(format!("Can't erase it: {}", e.0));
    }
}

/// An entity's data replaced by curves (its type kept where it holds them).
fn put(cx: &mut Cx, h: u32, cs: &[Curve]) -> Vec<u32> {
    let Some(like) = cx.d.objects.get(&h).map(|o| o.data.clone()) else {
        return vec![];
    };
    let data: Vec<Data> = cs.iter().filter_map(|c| write_curve(c, &like)).collect();
    match replace(cx.d, cx.hist, h, data) {
        Ok(v) => v,
        Err(e) => {
            cx.say(format!("Can't change it: {}", e.0));
            vec![]
        }
    }
}

fn line(a: V2, b: V2) -> Ghost {
    Ghost::Curve(Curve::line(a, b))
}

/// The entities' map, applied: in place, or on copies. The handles changed or made.
fn apply(cx: &mut Cx, sel: &[u32], m: &Affine, copy: bool) -> Vec<u32> {
    let mirrtext = cx.set.mirrtext;
    let mut out = vec![];
    let mut failed = 0;
    for &h in sel {
        if !cx.d.objects.contains_key(&h) {
            continue;
        }
        let target = if copy {
            match copy_entity(cx.d, cx.hist, h) {
                Ok(n) => n,
                Err(_) => {
                    failed += 1;
                    continue;
                }
            }
        } else {
            h
        };
        match transform(cx.d, cx.hist, target, m, mirrtext) {
            Ok(true) => out.push(target),
            _ => {
                failed += 1;
                if copy {
                    let _ = cx.d.erase(cx.hist, target);
                }
            }
        }
    }
    if failed > 0 {
        cx.say(format!("{failed} object(s) could not be changed."));
    }
    out
}

/// The box round entities as drawn.
fn sel_box(pv_q: &super::dwgcmd::ViewQ, sel: &[u32]) -> Option<Bbox> {
    let mut b = Bbox::EMPTY;
    for &h in sel {
        if let Some((lo, hi)) = pv_q.geom_of(h).entity_box(h) {
            b.add(V2::from(lo));
            b.add(V2::from(hi));
        }
    }
    (!b.is_empty()).then_some(b)
}

// ---- LINE

#[derive(Default)]
struct LineCmd {
    pts: Vec<V2>,
    segs: Vec<u32>,
}

impl Command for LineCmd {
    fn prompt(&self) -> Prompt {
        match self.pts.last() {
            None => Prompt::new("Specify first point", want::POINT),
            Some(&b) => {
                let opts: &[&str] = if self.segs.len() >= 2 { &["Close", "Undo"] } else { &["Undo"] };
                Prompt::new("Specify next point", want::POINT | want::ENTER)
                    .opts(opts)
                    .base(Some(b))
            }
        }
    }
    fn feed(&mut self, cx: &mut Cx, inp: Inp) -> Flow {
        match inp {
            Inp::Point(p) => {
                if let Some(&b) = self.pts.last() {
                    if b.dist(p) == 0.0 {
                        return Flow::Bad("The points are the same.".into());
                    }
                    match add_curve(cx, &Curve::line(b, p)) {
                        Some(h) => self.segs.push(h),
                        None => return Flow::More,
                    }
                }
                self.pts.push(p);
                Flow::More
            }
            Inp::Key("Undo") => {
                if let Some(h) = self.segs.pop() {
                    erase(cx, h);
                    self.pts.pop();
                } else {
                    self.pts.clear();
                }
                Flow::More
            }
            Inp::Key("Close") => {
                if let (Some(&a), Some(&b)) = (self.pts.first(), self.pts.last()) {
                    add_curve(cx, &Curve::line(b, a));
                }
                Flow::Done
            }
            Inp::Enter => Flow::Done,
            _ => Flow::Bad("Point or option keyword required.".into()),
        }
    }
    fn preview(&self, _: &Pv, at: V2) -> Vec<Ghost> {
        self.pts.last().map(|&b| vec![line(b, at)]).unwrap_or_default()
    }
}

// ---- PLINE

#[derive(Default, Clone, Copy, PartialEq)]
enum PlStage {
    #[default]
    Next,
    WidthStart,
    WidthEnd(f64),
    SecondPt,
    ArcEnd(V2),
}

#[derive(Default)]
struct PlineCmd {
    pts: Vec<V2>,
    bulges: Vec<f64>,
    widths: Vec<[f64; 2]>,
    arc: bool,
    w: (f64, f64),
    h: Option<u32>,
    stage: PlStage,
    closed: bool,
}

impl PlineCmd {
    /// The direction the polyline leaves its last point.
    fn end_dir(&self) -> f64 {
        let n = self.pts.len();
        if n < 2 {
            return 0.0;
        }
        let (a, b) = (self.pts[n - 2], self.pts[n - 1]);
        let chord = (b - a).angle();
        let bulge = self.bulges[n - 2];
        chord + 2.0 * forge_math::dmath::atan(bulge)
    }
    fn tangent_bulge(&self, p: V2) -> f64 {
        let Some(&b) = self.pts.last() else { return 0.0 };
        let mut th = (p - b).angle() - self.end_dir();
        while th > PI {
            th -= TAU;
        }
        while th <= -PI {
            th += TAU;
        }
        forge_math::dmath::sin(th / 2.0) / forge_math::dmath::cos(th / 2.0)
    }
    fn poly(&self) -> Poly {
        let n = self.pts.len();
        let mut bulges = self.bulges.clone();
        bulges.resize(n, 0.0);
        let mut widths = self.widths.clone();
        widths.resize(n, [self.w.0, self.w.1]);
        let any_w = widths.iter().any(|w| w[0] != 0.0 || w[1] != 0.0);
        Poly {
            pts: self.pts.clone(),
            bulges,
            widths: if any_w { widths } else { vec![] },
            const_width: 0.0,
            closed: self.closed,
        }
    }
    fn write(&mut self, cx: &mut Cx) {
        if self.pts.len() < 2 {
            if let Some(h) = self.h.take() {
                erase(cx, h);
            }
            return;
        }
        let Some(data) = to_data(&Curve::Poly(self.poly()), 0.0) else {
            return;
        };
        match self.h {
            Some(h) => {
                let _ = cx.d.modify(cx.hist, h, |o| o.data = data);
            }
            None => self.h = add(cx, data),
        }
    }
    fn push(&mut self, cx: &mut Cx, p: V2, bulge: f64) {
        self.bulges.push(bulge);
        self.widths.push([self.w.0, self.w.1]);
        self.pts.push(p);
        // the next segment starts as wide as this one ends
        self.w.0 = self.w.1;
        self.write(cx);
    }
}

impl Command for PlineCmd {
    fn prompt(&self) -> Prompt {
        let base = self.pts.last().copied();
        if base.is_none() {
            return Prompt::new("Specify start point", want::POINT);
        }
        match self.stage {
            PlStage::WidthStart => Prompt::new("Specify starting width", want::NUM).dflt(n4(self.w.0)),
            PlStage::WidthEnd(s) => Prompt::new("Specify ending width", want::NUM).dflt(n4(s)),
            PlStage::SecondPt => Prompt::new("Specify second point on arc", want::POINT).base(base),
            PlStage::ArcEnd(_) => Prompt::new("Specify end point of arc", want::POINT).base(base),
            PlStage::Next if self.arc => {
                Prompt::new("Specify endpoint of arc", want::POINT | want::ENTER)
                    .opts(&["CLose", "Line", "Second pt", "Undo", "Width"])
                    .base(base)
            }
            PlStage::Next => {
                let o: &[&str] = if self.pts.len() >= 3 {
                    &["Arc", "Close", "Undo", "Width"]
                } else {
                    &["Arc", "Undo", "Width"]
                };
                Prompt::new("Specify next point", want::POINT | want::ENTER).opts(o).base(base)
            }
        }
    }
    fn feed(&mut self, cx: &mut Cx, inp: Inp) -> Flow {
        match (self.stage, inp) {
            (PlStage::WidthStart, Inp::Num(x)) => {
                self.stage = PlStage::WidthEnd(x);
                Flow::More
            }
            (PlStage::WidthStart, Inp::Enter) => {
                self.stage = PlStage::WidthEnd(self.w.0);
                Flow::More
            }
            (PlStage::WidthEnd(s), Inp::Num(x)) => {
                self.w = (s, x);
                self.stage = PlStage::Next;
                Flow::More
            }
            (PlStage::WidthEnd(s), Inp::Enter) => {
                self.w = (s, s);
                self.stage = PlStage::Next;
                Flow::More
            }
            (PlStage::SecondPt, Inp::Point(m)) => {
                self.stage = PlStage::ArcEnd(m);
                Flow::More
            }
            (PlStage::ArcEnd(m), Inp::Point(e)) => {
                let Some(&b) = self.pts.last() else { return Flow::More };
                match CArc::from_3_points(b, m, e) {
                    Some(a) => {
                        self.push(cx, e, a.bulge());
                        self.stage = PlStage::Next;
                        Flow::More
                    }
                    None => Flow::Bad("The points are in a line.".into()),
                }
            }
            (PlStage::Next, Inp::Point(p)) => {
                if self.pts.is_empty() {
                    self.pts.push(p);
                    cx.say(format!("Current line-width is {}", n4(self.w.0)));
                    return Flow::More;
                }
                if self.pts.last().is_some_and(|b| b.dist(p) == 0.0) {
                    return Flow::Bad("The points are the same.".into());
                }
                let bulge = if self.arc { self.tangent_bulge(p) } else { 0.0 };
                self.push(cx, p, bulge);
                Flow::More
            }
            (PlStage::Next, Inp::Key(k)) => {
                match k {
                    "Arc" => self.arc = true,
                    "Line" => self.arc = false,
                    "Width" => self.stage = PlStage::WidthStart,
                    "Second pt" => self.stage = PlStage::SecondPt,
                    "Undo" => {
                        if self.pts.len() > 1 {
                            self.pts.pop();
                            self.bulges.pop();
                            self.widths.pop();
                            self.write(cx);
                        } else {
                            self.pts.clear();
                        }
                    }
                    "Close" | "CLose" => {
                        if self.pts.len() >= 2 {
                            let first = self.pts[0];
                            let bulge = if self.arc { self.tangent_bulge(first) } else { 0.0 };
                            self.bulges.push(bulge);
                            self.widths.push([self.w.0, self.w.1]);
                            self.closed = true;
                            self.write(cx);
                        }
                        return Flow::Done;
                    }
                    _ => {}
                }
                Flow::More
            }
            (_, Inp::Enter) => Flow::Done,
            _ => Flow::Bad("Invalid input.".into()),
        }
    }
    fn preview(&self, _: &Pv, at: V2) -> Vec<Ghost> {
        let Some(&b) = self.pts.last() else { return vec![] };
        match self.stage {
            PlStage::Next if self.arc => {
                let bulge = self.tangent_bulge(at);
                match CArc::from_bulge(b, at, bulge) {
                    Some(a) => vec![Ghost::Curve(Curve::Arc(a))],
                    None => vec![line(b, at)],
                }
            }
            PlStage::Next => vec![line(b, at)],
            PlStage::ArcEnd(m) => CArc::from_3_points(b, m, at)
                .map(|a| vec![Ghost::Curve(Curve::Arc(a))])
                .unwrap_or_default(),
            _ => vec![],
        }
    }
}

// ---- CIRCLE

#[derive(Default, Clone, Copy, PartialEq)]
enum CStage {
    #[default]
    Center,
    Radius(V2),
    Diameter(V2),
    P2a,
    P2b(V2),
    P3a,
    P3b(V2),
    P3c(V2, V2),
    TtrA,
    TtrB(u32, V2),
    TtrR(u32, V2, u32, V2),
}

#[derive(Default)]
struct CircleCmd {
    stage: CStage,
    r: f64,
}

impl Command for CircleCmd {
    fn prompt(&self) -> Prompt {
        match self.stage {
            CStage::Center => Prompt::new("Specify center point for circle", want::POINT).opts(&["3P", "2P", "Ttr (tan tan radius)"]),
            CStage::Radius(c) => {
                let p = Prompt::new("Specify radius of circle", want::POINT | want::NUM)
                    .opts(&["Diameter"])
                    .base(Some(c));
                if self.r > 0.0 { p.dflt(n4(self.r)) } else { p }
            }
            CStage::Diameter(c) => Prompt::new("Specify diameter of circle", want::POINT | want::NUM).base(Some(c)),
            CStage::P2a => Prompt::new("Specify first end point of circle's diameter", want::POINT),
            CStage::P2b(a) => Prompt::new("Specify second end point of circle's diameter", want::POINT).base(Some(a)),
            CStage::P3a => Prompt::new("Specify first point on circle", want::POINT),
            CStage::P3b(a) => Prompt::new("Specify second point on circle", want::POINT).base(Some(a)),
            CStage::P3c(_, b) => Prompt::new("Specify third point on circle", want::POINT).base(Some(b)),
            CStage::TtrA => Prompt::new("Specify point on object for first tangent of circle", want::PICK),
            CStage::TtrB(..) => Prompt::new("Specify point on object for second tangent of circle", want::PICK),
            CStage::TtrR(..) => {
                let p = Prompt::new("Specify radius of circle", want::NUM);
                if self.r > 0.0 { p.dflt(n4(self.r)) } else { p }
            }
        }
    }
    fn feed(&mut self, cx: &mut Cx, inp: Inp) -> Flow {
        let done = |cx: &mut Cx, c: Curve| {
            add_curve(cx, &c);
            Flow::Done
        };
        match (self.stage, inp) {
            (CStage::Center, Inp::Point(c)) => self.stage = CStage::Radius(c),
            (CStage::Center, Inp::Key(k)) => {
                self.stage = match k {
                    "3P" => CStage::P3a,
                    "2P" => CStage::P2a,
                    _ => CStage::TtrA,
                }
            }
            (CStage::Radius(c), Inp::Point(p)) => {
                self.r = c.dist(p);
                return done(cx, Curve::circle(c, self.r));
            }
            (CStage::Radius(c), Inp::Num(r)) if r > 0.0 => {
                self.r = r;
                return done(cx, Curve::circle(c, r));
            }
            (CStage::Radius(c), Inp::Enter) if self.r > 0.0 => return done(cx, Curve::circle(c, self.r)),
            (CStage::Radius(c), Inp::Key(_)) => self.stage = CStage::Diameter(c),
            (CStage::Diameter(c), Inp::Point(p)) => return done(cx, Curve::circle(c, c.dist(p) / 2.0)),
            (CStage::Diameter(c), Inp::Num(d)) if d > 0.0 => return done(cx, Curve::circle(c, d / 2.0)),
            (CStage::P2a, Inp::Point(a)) => self.stage = CStage::P2b(a),
            (CStage::P2b(a), Inp::Point(b)) if a != b => return done(cx, Curve::circle(a.mid(b), a.dist(b) / 2.0)),
            (CStage::P3a, Inp::Point(a)) => self.stage = CStage::P3b(a),
            (CStage::P3b(a), Inp::Point(b)) => self.stage = CStage::P3c(a, b),
            (CStage::P3c(a, b), Inp::Point(c)) => match CArc::from_3_points(a, b, c) {
                Some(x) => return done(cx, Curve::circle(x.c, x.r)),
                None => return Flow::Bad("The points are in a line.".into()),
            },
            (CStage::TtrA, Inp::Pick(h, p)) => self.stage = CStage::TtrB(h, p),
            (CStage::TtrB(a, pa), Inp::Pick(h, p)) => self.stage = CStage::TtrR(a, pa, h, p),
            (CStage::TtrR(a, pa, b, pb), inp @ (Inp::Num(_) | Inp::Enter)) => {
                let r = match inp {
                    Inp::Num(r) => r,
                    _ => self.r,
                };
                if r <= 0.0 {
                    return Flow::Bad("Requires a positive radius.".into());
                }
                self.r = r;
                let (Some(ca), Some(cb)) = (curve(cx.d, a), curve(cx.d, b)) else {
                    return Flow::Bad("Those objects can't be tangent to a circle.".into());
                };
                return match fillet::fillet(&ca, pa, &cb, pb, r).and_then(|k| k.added) {
                    Some(Curve::Arc(x)) => done(cx, Curve::circle(x.c, x.r)),
                    _ => {
                        cx.say("Circle does not exist.");
                        Flow::Done
                    }
                };
            }
            _ => return Flow::Bad("Point or option keyword required.".into()),
        }
        Flow::More
    }
    fn preview(&self, _: &Pv, at: V2) -> Vec<Ghost> {
        match self.stage {
            CStage::Radius(c) => vec![Ghost::Curve(Curve::circle(c, c.dist(at).max(1e-12))), line(c, at)],
            CStage::Diameter(c) => vec![Ghost::Curve(Curve::circle(c, c.dist(at).max(1e-12) / 2.0))],
            CStage::P2b(a) => vec![Ghost::Curve(Curve::circle(a.mid(at), a.dist(at).max(1e-12) / 2.0))],
            CStage::P3c(a, b) => CArc::from_3_points(a, b, at)
                .map(|x| vec![Ghost::Curve(Curve::circle(x.c, x.r))])
                .unwrap_or_default(),
            _ => vec![],
        }
    }
}

// ---- ARC

#[derive(Default, Clone, Copy, PartialEq)]
enum AStage {
    #[default]
    Start,
    Second(V2),
    End3(V2, V2),
    CenterFirst,
    CStart(V2),
    /// The centre and start: the end, or an angle.
    CEnd(V2, V2),
    CAngle(V2, V2),
    /// The start: its centre.
    SCenter(V2),
    /// The start: its end.
    SEnd(V2),
    /// The start and end: the centre, or a radius.
    SECenter(V2, V2),
    SERadius(V2, V2),
}

#[derive(Default)]
struct ArcCmd {
    stage: AStage,
}

/// The arc counter-clockwise about c from s to the angle of e.
fn arc_ce(c: V2, s: V2, e: V2) -> Option<Curve> {
    let r = c.dist(s);
    (r > 0.0).then(|| Curve::arc(c, r, (s - c).angle(), (e - c).angle()))
}

/// The arc from s to e of radius r counter-clockwise (the minor one; the major for r < 0).
fn arc_ser(s: V2, e: V2, r: f64) -> Option<Curve> {
    let half = s.dist(e) / 2.0;
    if half == 0.0 || r.abs() < half {
        return None;
    }
    let m = s.mid(e);
    let h = (r * r - half * half).sqrt();
    let u = (e - s).unit()?.perp();
    let c = if r > 0.0 { m + u * h } else { m - u * h };
    arc_ce(c, s, e)
}

impl Command for ArcCmd {
    fn prompt(&self) -> Prompt {
        match self.stage {
            AStage::Start => Prompt::new("Specify start point of arc", want::POINT).opts(&["Center"]),
            AStage::Second(s) => Prompt::new("Specify second point of arc", want::POINT)
                .opts(&["Center", "End"])
                .base(Some(s)),
            AStage::End3(_, m) => Prompt::new("Specify end point of arc", want::POINT).base(Some(m)),
            AStage::CenterFirst => Prompt::new("Specify center point of arc", want::POINT),
            AStage::CStart(c) => Prompt::new("Specify start point of arc", want::POINT).base(Some(c)),
            AStage::CEnd(c, _) => Prompt::new("Specify end point of arc", want::POINT)
                .opts(&["Angle"])
                .base(Some(c)),
            AStage::CAngle(..) => Prompt::new("Specify included angle", want::NUM | want::POINT),
            AStage::SCenter(s) => Prompt::new("Specify center point of arc", want::POINT).base(Some(s)),
            AStage::SEnd(s) => Prompt::new("Specify end point of arc", want::POINT).base(Some(s)),
            AStage::SECenter(_, e) => Prompt::new("Specify center point of arc", want::POINT)
                .opts(&["Radius"])
                .base(Some(e)),
            AStage::SERadius(..) => Prompt::new("Specify radius of arc", want::NUM | want::POINT),
        }
    }
    fn feed(&mut self, cx: &mut Cx, inp: Inp) -> Flow {
        let done = |cx: &mut Cx, c: Option<Curve>| match c {
            Some(c) => {
                add_curve(cx, &c);
                Flow::Done
            }
            None => Flow::Bad("Invalid arc.".into()),
        };
        self.stage = match (self.stage, inp) {
            (AStage::Start, Inp::Point(s)) => AStage::Second(s),
            (AStage::Start, Inp::Key(_)) => AStage::CenterFirst,
            (AStage::Second(s), Inp::Point(m)) => AStage::End3(s, m),
            (AStage::Second(s), Inp::Key("Center")) => AStage::SCenter(s),
            (AStage::Second(s), Inp::Key(_)) => AStage::SEnd(s),
            (AStage::End3(s, m), Inp::Point(e)) => {
                return done(cx, CArc::from_3_points(s, m, e).map(Curve::Arc));
            }
            (AStage::CenterFirst, Inp::Point(c)) => AStage::CStart(c),
            (AStage::CStart(c), Inp::Point(s)) => AStage::CEnd(c, s),
            (AStage::SCenter(s), Inp::Point(c)) => AStage::CEnd(c, s),
            (AStage::CEnd(c, s), Inp::Point(e)) => return done(cx, arc_ce(c, s, e)),
            (AStage::CEnd(c, s), Inp::Key(_)) => AStage::CAngle(c, s),
            (AStage::CAngle(c, s), Inp::Num(a)) => {
                let r = c.dist(s);
                let a0 = (s - c).angle();
                let a = a.to_radians();
                return done(
                    cx,
                    (r > 0.0 && a != 0.0).then(|| {
                        Curve::Arc(CArc {
                            c,
                            r,
                            start: a0,
                            sweep: a.clamp(-TAU, TAU),
                        })
                    }),
                );
            }
            (AStage::SEnd(s), Inp::Point(e)) => AStage::SECenter(s, e),
            (AStage::SECenter(s, e), Inp::Point(c)) => return done(cx, arc_ce(c, s, e)),
            (AStage::SECenter(s, e), Inp::Key(_)) => AStage::SERadius(s, e),
            (AStage::SERadius(s, e), Inp::Num(r)) => return done(cx, arc_ser(s, e, r)),
            (AStage::SERadius(s, e), Inp::Point(p)) => return done(cx, arc_ser(s, e, e.dist(p))),
            _ => return Flow::Bad("Point or option keyword required.".into()),
        };
        Flow::More
    }
    fn preview(&self, _: &Pv, at: V2) -> Vec<Ghost> {
        let g = |c: Option<Curve>| c.map(|c| vec![Ghost::Curve(c)]).unwrap_or_default();
        match self.stage {
            AStage::Second(s) => vec![line(s, at)],
            AStage::End3(s, m) => g(CArc::from_3_points(s, m, at).map(Curve::Arc)),
            AStage::CStart(c) => vec![line(c, at)],
            AStage::CEnd(c, s) => {
                let mut v = g(arc_ce(c, s, at));
                v.push(Ghost::Dashed(c, at));
                v
            }
            AStage::SECenter(s, e) => g(arc_ce(at, s, e)),
            AStage::SEnd(s) | AStage::SCenter(s) => vec![line(s, at)],
            _ => vec![],
        }
    }
}

// ---- RECTANG

#[derive(Default, Clone, Copy, PartialEq)]
enum RStage {
    #[default]
    First,
    Fillet,
    Other(V2),
    DimL(V2),
    DimW(V2, f64),
    DimSide(V2, f64, f64),
}

#[derive(Default)]
struct RectCmd {
    stage: RStage,
    fillet: f64,
}

/// A rectangle (corners a, b) as a closed polyline, its corners rounded by r.
fn rect_poly(a: V2, b: V2, r: f64) -> Option<Curve> {
    let (lo, hi) = (v2(a.x.min(b.x), a.y.min(b.y)), v2(a.x.max(b.x), a.y.max(b.y)));
    if hi.x - lo.x <= 0.0 || hi.y - lo.y <= 0.0 {
        return None;
    }
    let c = [lo, v2(hi.x, lo.y), hi, v2(lo.x, hi.y)];
    let r = r.min((hi.x - lo.x) / 2.0).min((hi.y - lo.y) / 2.0);
    if r <= 0.0 {
        return Some(Curve::poly(&c, &[], true));
    }
    let mut pts = vec![];
    let mut bulges = vec![];
    let k = forge_math::dmath::sin(PI / 8.0) / forge_math::dmath::cos(PI / 8.0);
    for i in 0..4 {
        let (p, cur, n) = (c[(i + 3) % 4], c[i], c[(i + 1) % 4]);
        pts.push(cur + (p - cur).unit()? * r);
        bulges.push(k);
        pts.push(cur + (n - cur).unit()? * r);
        bulges.push(0.0);
    }
    Some(Curve::poly(&pts, &bulges, true))
}

impl Command for RectCmd {
    fn prompt(&self) -> Prompt {
        match self.stage {
            RStage::First => Prompt::new("Specify first corner point", want::POINT).opts(&["Fillet"]),
            RStage::Fillet => Prompt::new("Specify fillet radius for rectangles", want::NUM).dflt(n4(self.fillet)),
            RStage::Other(a) => Prompt::new("Specify other corner point", want::POINT)
                .opts(&["Dimensions"])
                .base(Some(a)),
            RStage::DimL(_) => Prompt::new("Specify length for rectangles", want::NUM),
            RStage::DimW(..) => Prompt::new("Specify width for rectangles", want::NUM),
            RStage::DimSide(a, ..) => Prompt::new("Specify other corner point", want::POINT).base(Some(a)),
        }
    }
    fn feed(&mut self, cx: &mut Cx, inp: Inp) -> Flow {
        self.stage = match (self.stage, inp) {
            (RStage::First, Inp::Point(a)) => RStage::Other(a),
            (RStage::First, Inp::Key(_)) => RStage::Fillet,
            (RStage::Fillet, Inp::Num(r)) if r >= 0.0 => {
                self.fillet = r;
                RStage::First
            }
            (RStage::Fillet, Inp::Enter) => RStage::First,
            (RStage::Other(a), Inp::Point(b)) => {
                return match rect_poly(a, b, self.fillet) {
                    Some(c) => {
                        add_curve(cx, &c);
                        Flow::Done
                    }
                    None => Flow::Bad("The corners must differ in x and y.".into()),
                };
            }
            (RStage::Other(a), Inp::Key(_)) => RStage::DimL(a),
            (RStage::DimL(a), Inp::Num(l)) if l > 0.0 => RStage::DimW(a, l),
            (RStage::DimW(a, l), Inp::Num(w)) if w > 0.0 => RStage::DimSide(a, l, w),
            (RStage::DimSide(a, l, w), Inp::Point(p)) => {
                let sx = if p.x >= a.x { l } else { -l };
                let sy = if p.y >= a.y { w } else { -w };
                if let Some(c) = rect_poly(a, a + v2(sx, sy), self.fillet) {
                    add_curve(cx, &c);
                }
                return Flow::Done;
            }
            _ => return Flow::Bad("Invalid input.".into()),
        };
        Flow::More
    }
    fn preview(&self, _: &Pv, at: V2) -> Vec<Ghost> {
        match self.stage {
            RStage::Other(a) => rect_poly(a, at, self.fillet).map(|c| vec![Ghost::Curve(c)]).unwrap_or_default(),
            _ => vec![],
        }
    }
}

// ---- POLYGON

#[derive(Default, Clone, Copy, PartialEq)]
enum PgStage {
    #[default]
    Sides,
    Center,
    Kind(V2),
    Radius(V2, bool),
    Edge1,
    Edge2(V2),
}

#[derive(Default)]
struct PolygonCmd {
    stage: PgStage,
    n: usize,
}

fn polygon(c: V2, n: usize, r: f64, a0: f64) -> Curve {
    let pts: Vec<V2> = (0..n).map(|k| c + V2::polar(a0 + TAU * k as f64 / n as f64, r)).collect();
    Curve::poly(&pts, &[], true)
}

impl PolygonCmd {
    fn sides(&self, cx_n: usize) -> usize {
        if self.n >= 3 { self.n } else { cx_n }
    }
    /// The polygon for a radius point (or a typed radius: None) about c.
    fn shape(&self, c: V2, inscribed: bool, at: Option<V2>, r: f64, n: usize) -> Curve {
        let half = PI / n as f64;
        let (r, dir) = match at {
            Some(p) => (c.dist(p), (p - c).angle()),
            None => (r, -PI / 2.0),
        };
        if inscribed {
            let a0 = if at.is_some() { dir } else { dir + half };
            polygon(c, n, r, a0)
        } else {
            // the radius to an edge's middle
            polygon(c, n, r / forge_math::dmath::cos(half), dir + half)
        }
    }
}

impl Command for PolygonCmd {
    fn prompt(&self) -> Prompt {
        match self.stage {
            PgStage::Sides => Prompt::new("Enter number of sides", want::NUM).dflt(self.n.max(3).to_string()),
            PgStage::Center => Prompt::new("Specify center of polygon", want::POINT).opts(&["Edge"]),
            PgStage::Kind(_) => Prompt::new("Enter an option", want::ENTER)
                .opts(&["Inscribed in circle", "Circumscribed about circle"])
                .dflt("I"),
            PgStage::Radius(c, _) => Prompt::new("Specify radius of circle", want::POINT | want::NUM).base(Some(c)),
            PgStage::Edge1 => Prompt::new("Specify first endpoint of edge", want::POINT),
            PgStage::Edge2(a) => Prompt::new("Specify second endpoint of edge", want::POINT).base(Some(a)),
        }
    }
    fn feed(&mut self, cx: &mut Cx, inp: Inp) -> Flow {
        if self.n == 0 {
            self.n = cx.set.polygon_sides;
        }
        self.stage = match (self.stage, inp) {
            (PgStage::Sides, Inp::Num(k)) if (3.0..=1024.0).contains(&k) => {
                self.n = k as usize;
                cx.set.polygon_sides = self.n;
                PgStage::Center
            }
            (PgStage::Sides, Inp::Enter) => PgStage::Center,
            (PgStage::Center, Inp::Point(c)) => PgStage::Kind(c),
            (PgStage::Center, Inp::Key(_)) => PgStage::Edge1,
            (PgStage::Kind(c), Inp::Key(k)) => PgStage::Radius(c, k.starts_with('I')),
            (PgStage::Kind(c), Inp::Enter) => PgStage::Radius(c, true),
            (PgStage::Radius(c, ins), Inp::Point(p)) if p != c => {
                let s = self.shape(c, ins, Some(p), 0.0, self.sides(cx.set.polygon_sides));
                add_curve(cx, &s);
                return Flow::Done;
            }
            (PgStage::Radius(c, ins), Inp::Num(r)) if r > 0.0 => {
                let s = self.shape(c, ins, None, r, self.sides(cx.set.polygon_sides));
                add_curve(cx, &s);
                return Flow::Done;
            }
            (PgStage::Edge1, Inp::Point(a)) => PgStage::Edge2(a),
            (PgStage::Edge2(a), Inp::Point(b)) if a != b => {
                let n = self.sides(cx.set.polygon_sides);
                let s = a.dist(b);
                let apothem = s / (2.0 * (forge_math::dmath::sin(PI / n as f64) / forge_math::dmath::cos(PI / n as f64)));
                let u = (b - a) / s;
                let c = a.mid(b) + u.perp() * apothem;
                let r = s / (2.0 * forge_math::dmath::sin(PI / n as f64));
                add_curve(cx, &polygon(c, n, r, (a - c).angle()));
                return Flow::Done;
            }
            _ => return Flow::Bad("Invalid input.".into()),
        };
        Flow::More
    }
    fn preview(&self, pv: &Pv, at: V2) -> Vec<Ghost> {
        match self.stage {
            PgStage::Radius(c, ins) if at != c => {
                vec![Ghost::Curve(self.shape(c, ins, Some(at), 0.0, self.sides(pv.set.polygon_sides)))]
            }
            _ => vec![],
        }
    }
}

// ---- ELLIPSE

#[derive(Default, Clone, Copy, PartialEq)]
enum EStage {
    #[default]
    Axis1,
    Axis2(V2),
    Center,
    CAxis(V2),
    Other(V2, V2),
    Start(Ellipse),
    End(Ellipse, f64),
}

#[derive(Default)]
struct EllipseCmd {
    stage: EStage,
    arc: bool,
}

fn ellipse_of(c: V2, major_end: V2, p: V2) -> Option<Ellipse> {
    let major = major_end - c;
    let a = major.len();
    let b = c.dist(p);
    if a == 0.0 || b == 0.0 {
        return None;
    }
    Some(if b <= a {
        Ellipse {
            c,
            major,
            ratio: b / a,
            start: 0.0,
            sweep: TAU,
        }
    } else {
        Ellipse {
            c,
            major: major.perp().unit()? * b,
            ratio: a / b,
            start: 0.0,
            sweep: TAU,
        }
    })
}

impl Command for EllipseCmd {
    fn prompt(&self) -> Prompt {
        match self.stage {
            EStage::Axis1 => Prompt::new("Specify axis endpoint of ellipse", want::POINT).opts(&["Arc", "Center"]),
            EStage::Axis2(a) => Prompt::new("Specify other endpoint of axis", want::POINT).base(Some(a)),
            EStage::Center => Prompt::new("Specify center of ellipse", want::POINT),
            EStage::CAxis(c) => Prompt::new("Specify endpoint of axis", want::POINT).base(Some(c)),
            EStage::Other(c, _) => Prompt::new("Specify distance to other axis", want::POINT | want::NUM).base(Some(c)),
            EStage::Start(e) => Prompt::new("Specify start angle", want::POINT | want::NUM).base(Some(e.c)),
            EStage::End(e, _) => Prompt::new("Specify end angle", want::POINT | want::NUM).base(Some(e.c)),
        }
    }
    fn feed(&mut self, cx: &mut Cx, inp: Inp) -> Flow {
        let finish = |me: &mut Self, cx: &mut Cx, e: Ellipse| {
            if me.arc {
                me.stage = EStage::Start(e);
                Flow::More
            } else {
                add_curve(cx, &Curve::Ellipse(e));
                Flow::Done
            }
        };
        let angle_of = |e: &Ellipse, inp: &Inp| -> Option<f64> {
            match inp {
                Inp::Point(p) => Some(e.angle_of(*p)),
                Inp::Num(a) => Some(e.param_of_angle(a.to_radians() + e.major.angle())),
                _ => None,
            }
        };
        self.stage = match (self.stage, &inp) {
            (EStage::Axis1, Inp::Point(a)) => EStage::Axis2(*a),
            (EStage::Axis1, Inp::Key("Arc")) => {
                self.arc = true;
                EStage::Axis1
            }
            (EStage::Axis1, Inp::Key(_)) => EStage::Center,
            (EStage::Axis2(a), Inp::Point(b)) if *b != a => EStage::Other(a.mid(*b), *b),
            (EStage::Center, Inp::Point(c)) => EStage::CAxis(*c),
            (EStage::CAxis(c), Inp::Point(b)) if *b != c => EStage::Other(c, *b),
            (EStage::Other(c, m), Inp::Point(p)) => match ellipse_of(c, m, *p) {
                Some(e) => return finish(self, cx, e),
                None => return Flow::Bad("Invalid ellipse.".into()),
            },
            (EStage::Other(c, m), Inp::Num(d)) if *d > 0.0 => {
                let p = c + (m - c).perp().unit().unwrap_or(v2(0.0, 1.0)) * *d;
                match ellipse_of(c, m, p) {
                    Some(e) => return finish(self, cx, e),
                    None => return Flow::Bad("Invalid ellipse.".into()),
                }
            }
            (EStage::Start(e), i) => match angle_of(&e, i) {
                Some(a) => EStage::End(e, a),
                None => return Flow::Bad("Requires an angle.".into()),
            },
            (EStage::End(e, s), i) => match angle_of(&e, i) {
                Some(t) => {
                    let mut sweep = (t - s).rem_euclid(TAU);
                    if sweep == 0.0 {
                        sweep = TAU;
                    }
                    add_curve(cx, &Curve::Ellipse(Ellipse { start: s, sweep, ..e }));
                    return Flow::Done;
                }
                None => return Flow::Bad("Requires an angle.".into()),
            },
            _ => return Flow::Bad("Invalid input.".into()),
        };
        Flow::More
    }
    fn preview(&self, _: &Pv, at: V2) -> Vec<Ghost> {
        match self.stage {
            EStage::Axis2(a) => vec![line(a, at)],
            EStage::CAxis(c) => vec![line(c, at)],
            EStage::Other(c, m) => ellipse_of(c, m, at)
                .map(|e| vec![Ghost::Curve(Curve::Ellipse(e)), line(c, at)])
                .unwrap_or_default(),
            EStage::End(e, s) => {
                let t = e.angle_of(at);
                let mut sweep = (t - s).rem_euclid(TAU);
                if sweep == 0.0 {
                    sweep = TAU;
                }
                vec![Ghost::Curve(Curve::Ellipse(Ellipse { start: s, sweep, ..e }))]
            }
            _ => vec![],
        }
    }
}

// ---- SPLINE

#[derive(Default)]
struct SplineCmd {
    pts: Vec<V2>,
}

impl Command for SplineCmd {
    fn prompt(&self) -> Prompt {
        match self.pts.len() {
            0 => Prompt::new("Specify first point", want::POINT),
            1 => Prompt::new("Enter next point", want::POINT).opts(&["Undo"]).base(self.pts.last().copied()),
            _ => Prompt::new("Enter next point", want::POINT | want::ENTER)
                .opts(&["Close", "Undo"])
                .base(self.pts.last().copied()),
        }
    }
    fn feed(&mut self, cx: &mut Cx, inp: Inp) -> Flow {
        match inp {
            Inp::Point(p) => {
                if self.pts.last() == Some(&p) {
                    return Flow::Bad("The points are the same.".into());
                }
                self.pts.push(p);
                Flow::More
            }
            Inp::Key("Undo") => {
                self.pts.pop();
                Flow::More
            }
            Inp::Key("Close") | Inp::Enter => {
                let closed = matches!(inp, Inp::Key(_));
                if let Some(d) = make::spline_through(&self.pts, closed) {
                    add(cx, d);
                }
                Flow::Done
            }
            _ => Flow::Bad("Invalid input.".into()),
        }
    }
    fn preview(&self, _: &Pv, at: V2) -> Vec<Ghost> {
        let mut pts = self.pts.clone();
        if pts.is_empty() || pts.last() == Some(&at) {
            return vec![];
        }
        pts.push(at);
        make::spline_through(&pts, false)
            .and_then(|d| read_entity(&d).ok())
            .map(|f| vec![Ghost::Curve(f.curve)])
            .unwrap_or_default()
    }
}

// ---- POINT

struct PointCmd;

impl Command for PointCmd {
    fn prompt(&self) -> Prompt {
        Prompt::new("Specify a point", want::POINT)
    }
    fn feed(&mut self, cx: &mut Cx, inp: Inp) -> Flow {
        match inp {
            Inp::Point(p) => {
                add(cx, make::point(p));
                Flow::Done
            }
            _ => Flow::Bad("Point required.".into()),
        }
    }
}

// ---- TEXT

#[derive(Default, Clone, Copy, PartialEq)]
enum TStage {
    #[default]
    Start,
    Height(V2),
    Rotation(V2, f64),
    Text(V2, f64, f64),
}

#[derive(Default)]
struct TextCmd {
    stage: TStage,
}

impl Command for TextCmd {
    fn prompt(&self) -> Prompt {
        match self.stage {
            TStage::Start => Prompt::new("Specify start point of text", want::POINT),
            TStage::Height(p) => Prompt::new("Specify height", want::POINT | want::NUM).base(Some(p)),
            TStage::Rotation(p, _) => Prompt::new("Specify rotation angle of text", want::POINT | want::NUM)
                .dflt("0")
                .base(Some(p)),
            TStage::Text(..) => Prompt::new("Enter text", want::TEXT),
        }
    }
    fn feed(&mut self, cx: &mut Cx, inp: Inp) -> Flow {
        self.stage = match (self.stage, inp) {
            (TStage::Start, Inp::Point(p)) => TStage::Height(p),
            (TStage::Height(p), Inp::Point(q)) if q != p => {
                cx.set.text_h = p.dist(q);
                TStage::Rotation(p, cx.set.text_h)
            }
            (TStage::Height(p), Inp::Num(h)) if h > 0.0 => {
                cx.set.text_h = h;
                TStage::Rotation(p, h)
            }
            (TStage::Height(p), Inp::Enter) => TStage::Rotation(p, cx.set.text_h),
            (TStage::Rotation(p, h), Inp::Point(q)) => TStage::Text(p, h, (q - p).angle()),
            (TStage::Rotation(p, h), Inp::Num(a)) => TStage::Text(p, h, a.to_radians()),
            (TStage::Rotation(p, h), Inp::Enter) => TStage::Text(p, h, 0.0),
            (TStage::Text(p, h, r), Inp::Text(s)) => {
                if s.is_empty() {
                    return Flow::Done;
                }
                let d = make::text(cx.d, p, h, r, &s);
                add(cx, d);
                // the next line under it
                TStage::Text(p - V2::polar(r, 1.0).perp() * (h * 5.0 / 3.0), h, r)
            }
            (TStage::Text(..), Inp::Enter) => return Flow::Done,
            _ => return Flow::Bad("Invalid input.".into()),
        };
        Flow::More
    }
    fn preview(&self, pv: &Pv, at: V2) -> Vec<Ghost> {
        match self.stage {
            TStage::Height(p) => vec![line(p, at)],
            TStage::Rotation(p, _) => vec![line(p, at)],
            TStage::Text(p, h, _) => vec![Ghost::Text(p, h, "|".into())],
            TStage::Start => vec![Ghost::Text(at, pv.set.text_h, "Text".into())],
        }
    }
}

// ---- MTEXT

#[derive(Default, Clone, Copy, PartialEq)]
enum MStage {
    #[default]
    First,
    Other(V2),
    Height(V2),
    Text(V2, V2),
}

#[derive(Default)]
struct MTextCmd {
    stage: MStage,
}

impl Command for MTextCmd {
    fn prompt(&self) -> Prompt {
        match self.stage {
            MStage::First => Prompt::new("Specify first corner", want::POINT),
            MStage::Other(a) => Prompt::new("Specify opposite corner", want::POINT)
                .opts(&["Height"])
                .base(Some(a)),
            MStage::Height(_) => Prompt::new("Specify height", want::NUM),
            MStage::Text(..) => Prompt::new("Enter the text (an empty line ends it)", want::TEXT | want::LINES),
        }
    }
    fn feed(&mut self, cx: &mut Cx, inp: Inp) -> Flow {
        self.stage = match (self.stage, inp) {
            (MStage::First, Inp::Point(a)) => MStage::Other(a),
            (MStage::Other(a), Inp::Key(_)) => MStage::Height(a),
            (MStage::Height(a), Inp::Num(h)) if h > 0.0 => {
                cx.set.text_h = h;
                MStage::Other(a)
            }
            (MStage::Other(a), Inp::Point(b)) => MStage::Text(a, b),
            (MStage::Text(a, b), Inp::Text(s)) => {
                let body = s.trim_end_matches('\n').replace("\r\n", "\n").replace('\n', "\\P");
                if !body.trim().is_empty() {
                    let top_left = v2(a.x.min(b.x), a.y.max(b.y));
                    let d = make::mtext(cx.d, top_left, (b.x - a.x).abs(), cx.set.text_h, &body);
                    add(cx, d);
                }
                return Flow::Done;
            }
            (MStage::Text(..), Inp::Enter) => return Flow::Done,
            _ => return Flow::Bad("Invalid input.".into()),
        };
        Flow::More
    }
    fn preview(&self, _: &Pv, at: V2) -> Vec<Ghost> {
        let rect = |a: V2, b: V2| {
            let pts = [a, v2(b.x, a.y), b, v2(a.x, b.y)];
            Ghost::Curve(Curve::poly(&pts, &[], true))
        };
        match self.stage {
            MStage::Other(a) => vec![rect(a, at)],
            MStage::Text(a, b) => vec![rect(a, b)],
            _ => vec![],
        }
    }
}

// ---- HATCH

#[derive(Default, Clone, Copy, PartialEq)]
enum HStage {
    #[default]
    Pick,
    Pattern,
    Scale,
    Angle,
}

#[derive(Default)]
struct HatchCmd {
    stage: HStage,
    made: Vec<u32>,
}

/// The boundary round p from the curves near it: a box round p grown until the loop found lies
/// inside it (so nothing outside could cut it), up to the drawing's extents.
pub fn find_boundary(d: &Drawing, q: &super::dwgcmd::ViewQ, p: V2) -> Option<boundary::Boundary> {
    let ext = q.extents()?;
    let size = (ext.1 - ext.0).abs_max().max(1e-9);
    // ends a part in a million of the drawing apart meet (drawings are full of such gaps)
    let tol = (size * 1e-6).max(1e-12);
    let mut r = (q.px * 200.0).max(size * 1e-4);
    for _ in 0..12 {
        let lo = p - v2(r, r);
        let hi = p + v2(r, r);
        let whole = lo.x <= ext.0.x && lo.y <= ext.0.y && hi.x >= ext.1.x && hi.y >= ext.1.y;
        let hs = q.in_box(lo, hi);
        let curves: Vec<Curve> = hs
            .iter()
            .filter(|h| !matches!(d.objects.get(h).map(|o| &o.data), Some(Data::Hatch(_))))
            .filter_map(|h| curve(d, *h))
            .collect();
        if let Some(b) = boundary::find(&curves, p, tol) {
            let bb = b.outer.iter().fold(Bbox::EMPTY, |x, c| x.union(c.bbox()));
            let within = bb.min.x > lo.x && bb.min.y > lo.y && bb.max.x < hi.x && bb.max.y < hi.y;
            if within || whole {
                return Some(b);
            }
        }
        if whole {
            return None;
        }
        r *= 4.0;
    }
    None
}

impl Command for HatchCmd {
    fn prompt(&self) -> Prompt {
        match self.stage {
            HStage::Pick => Prompt::new("Pick internal point", want::POINT | want::ENTER).opts(&["Pattern", "Scale", "Angle", "Undo"]),
            HStage::Pattern => Prompt::new("Enter a pattern name (SOLID, ANSI31, ANSI32, ANSI37, LINE, NET)", want::TEXT),
            HStage::Scale => Prompt::new("Specify a scale for the pattern", want::NUM),
            HStage::Angle => Prompt::new("Specify an angle for the pattern", want::NUM),
        }
    }
    fn feed(&mut self, cx: &mut Cx, inp: Inp) -> Flow {
        match (self.stage, inp) {
            (HStage::Pick, Inp::Point(p)) => {
                let Some(b) = find_boundary(cx.d, cx.q, p) else {
                    return Flow::Bad("Valid hatch boundary not found.".into());
                };
                let name = cx.set.hatch.clone();
                let fill = if name.eq_ignore_ascii_case("SOLID") {
                    Fill::Solid
                } else {
                    Fill::Pattern {
                        name,
                        angle: cx.set.hatch_angle,
                        scale: cx.set.hatch_scale,
                    }
                };
                match make::hatch(cx.d, &b, &fill) {
                    Some(h) => {
                        if let Some(x) = add(cx, h) {
                            self.made.push(x);
                        }
                        cx.say(format!("Boundary found: {} island(s)", b.islands.len()));
                    }
                    None => return Flow::Bad("Unknown pattern.".into()),
                }
                Flow::More
            }
            (HStage::Pick, Inp::Key("Undo")) => {
                if let Some(h) = self.made.pop() {
                    erase(cx, h);
                }
                Flow::More
            }
            (HStage::Pick, Inp::Key(k)) => {
                self.stage = match k {
                    "Pattern" => HStage::Pattern,
                    "Scale" => HStage::Scale,
                    _ => HStage::Angle,
                };
                Flow::More
            }
            (HStage::Pick, Inp::Enter) => Flow::Done,
            (HStage::Pattern, Inp::Text(s)) => {
                let s = s.trim().to_ascii_uppercase();
                if s != "SOLID" && make::pattern_lines(&s, 0.0, 1.0).is_none() {
                    return Flow::Bad(format!("Unknown pattern {s}."));
                }
                cx.set.hatch = s;
                self.stage = HStage::Pick;
                Flow::More
            }
            (HStage::Scale, Inp::Num(k)) if k > 0.0 => {
                cx.set.hatch_scale = k;
                self.stage = HStage::Pick;
                Flow::More
            }
            (HStage::Angle, Inp::Num(a)) => {
                cx.set.hatch_angle = a.to_radians();
                self.stage = HStage::Pick;
                Flow::More
            }
            (_, Inp::Enter) => {
                self.stage = HStage::Pick;
                Flow::More
            }
            _ => Flow::Bad("Invalid input.".into()),
        }
    }
}

// ---- dimensions

/// A made dimension as a rubber band.
fn dim_ghosts(m: &dim::Made) -> Vec<Ghost> {
    let mut out = vec![];
    for (d, defpt) in &m.pieces {
        if *defpt {
            continue;
        }
        match d {
            Data::MText(t) => out.push(Ghost::Text(V2::from(t.insert), t.text_height, t.text.text().replace("%%c", "\u{2205}"))),
            Data::Solid(s) => {
                let p: Vec<V2> = s.corners[..3].iter().map(|c| V2::from(*c)).collect();
                out.push(Ghost::Curve(Curve::poly(&p, &[], true)));
            }
            other => {
                if let Ok(f) = read_entity(other) {
                    out.push(Ghost::Curve(f.curve));
                }
            }
        }
    }
    out
}

#[derive(Clone, Copy, PartialEq)]
enum DStage {
    P1,
    Object,
    P2(V2),
    Loc(V2, V2),
    Angle(V2, V2),
}

struct DimLinCmd {
    aligned: bool,
    stage: DStage,
    /// Horizontal, vertical, or rotated by this; None: by where the line goes.
    forced: Option<f64>,
}

impl DimLinCmd {
    fn new(aligned: bool) -> DimLinCmd {
        DimLinCmd {
            aligned,
            stage: DStage::P1,
            forced: None,
        }
    }
    fn kind(&self, p1: V2, p2: V2, at: V2) -> DimKind {
        if self.aligned {
            return DimKind::Aligned { p1, p2, at };
        }
        let angle = self.forced.unwrap_or_else(|| {
            // outside the points' span across, inside it along: a vertical dimension
            let (x0, x1) = (p1.x.min(p2.x), p1.x.max(p2.x));
            let (y0, y1) = (p1.y.min(p2.y), p1.y.max(p2.y));
            if (at.x < x0 || at.x > x1) && at.y >= y0 && at.y <= y1 { PI / 2.0 } else { 0.0 }
        });
        DimKind::Linear { p1, p2, at, angle }
    }
}

impl Command for DimLinCmd {
    fn prompt(&self) -> Prompt {
        match self.stage {
            DStage::P1 => Prompt::new("Specify first extension line origin", want::POINT).dflt("select object"),
            DStage::Object => Prompt::new("Select object to dimension", want::PICK),
            DStage::P2(a) => Prompt::new("Specify second extension line origin", want::POINT).base(Some(a)),
            DStage::Loc(..) => {
                let p = Prompt::new("Specify dimension line location", want::POINT);
                if self.aligned { p } else { p.opts(&["Horizontal", "Vertical", "Rotated"]) }
            }
            DStage::Angle(..) => Prompt::new("Specify angle of dimension line", want::NUM).dflt("0"),
        }
    }
    fn feed(&mut self, cx: &mut Cx, inp: Inp) -> Flow {
        self.stage = match (self.stage, inp) {
            (DStage::P1, Inp::Point(a)) => DStage::P2(a),
            (DStage::P1, Inp::Enter) => DStage::Object,
            (DStage::Object, Inp::Pick(h, _)) => match curve(cx.d, h) {
                Some(c @ (Curve::Line { .. } | Curve::Arc(_))) => DStage::Loc(c.start(), c.end()),
                _ => return Flow::Bad("Select a line or an arc.".into()),
            },
            (DStage::P2(a), Inp::Point(b)) if a != b => DStage::Loc(a, b),
            (DStage::Loc(a, b), Inp::Key(k)) => {
                match k {
                    "Horizontal" => self.forced = Some(0.0),
                    "Vertical" => self.forced = Some(PI / 2.0),
                    _ => return self.set_stage(DStage::Angle(a, b)),
                }
                DStage::Loc(a, b)
            }
            (DStage::Angle(a, b), Inp::Num(x)) => {
                self.forced = Some(x.to_radians());
                DStage::Loc(a, b)
            }
            (DStage::Angle(a, b), Inp::Enter) => {
                self.forced = Some(0.0);
                DStage::Loc(a, b)
            }
            (DStage::Loc(a, b), Inp::Point(at)) => {
                let st = DimStyle::current(cx.d);
                let Some(m) = dim::make(cx.d, &st, self.kind(a, b, at)) else {
                    return Flow::Bad("Can't dimension that.".into());
                };
                cx.say(format!("Dimension text = {}", n4(m.measurement)));
                if let Err(e) = dim::add(cx.d, cx.hist, cx.space, cx.layer, m) {
                    cx.say(format!("Can't add it: {}", e.0));
                }
                return Flow::Done;
            }
            _ => return Flow::Bad("Invalid input.".into()),
        };
        Flow::More
    }
    fn preview(&self, pv: &Pv, at: V2) -> Vec<Ghost> {
        match self.stage {
            DStage::P2(a) => vec![line(a, at)],
            DStage::Loc(a, b) => {
                let st = DimStyle::current(pv.d);
                dim::make(pv.d, &st, self.kind(a, b, at)).map(|m| dim_ghosts(&m)).unwrap_or_default()
            }
            _ => vec![],
        }
    }
}

impl DimLinCmd {
    fn set_stage(&mut self, s: DStage) -> Flow {
        self.stage = s;
        Flow::More
    }
}

struct DimRadCmd {
    dia: bool,
    circle: Option<(V2, f64)>,
}

impl DimRadCmd {
    fn new(dia: bool) -> DimRadCmd {
        DimRadCmd { dia, circle: None }
    }
    fn kind(&self, c: V2, r: f64, at: V2) -> DimKind {
        if self.dia { DimKind::Diameter { c, r, at } } else { DimKind::Radius { c, r, at } }
    }
}

impl Command for DimRadCmd {
    fn prompt(&self) -> Prompt {
        match self.circle {
            None => Prompt::new("Select arc or circle", want::PICK),
            Some((c, _)) => Prompt::new("Specify dimension line location", want::POINT).base(Some(c)),
        }
    }
    fn feed(&mut self, cx: &mut Cx, inp: Inp) -> Flow {
        match (self.circle, inp) {
            (None, Inp::Pick(h, _)) => match curve(cx.d, h) {
                Some(Curve::Circle { c, r }) => {
                    self.circle = Some((c, r));
                    Flow::More
                }
                Some(Curve::Arc(a)) => {
                    self.circle = Some((a.c, a.r));
                    Flow::More
                }
                _ => Flow::Bad("Select an arc or a circle.".into()),
            },
            (Some((c, r)), Inp::Point(at)) => {
                let st = DimStyle::current(cx.d);
                if let Some(m) = dim::make(cx.d, &st, self.kind(c, r, at)) {
                    cx.say(format!("Dimension text = {}", n4(m.measurement)));
                    if let Err(e) = dim::add(cx.d, cx.hist, cx.space, cx.layer, m) {
                        cx.say(format!("Can't add it: {}", e.0));
                    }
                }
                Flow::Done
            }
            _ => Flow::Bad("Invalid input.".into()),
        }
    }
    fn preview(&self, pv: &Pv, at: V2) -> Vec<Ghost> {
        let Some((c, r)) = self.circle else { return vec![] };
        let st = DimStyle::current(pv.d);
        dim::make(pv.d, &st, self.kind(c, r, at)).map(|m| dim_ghosts(&m)).unwrap_or_default()
    }
}

// ---- DIMSCALE

/// DIMSCALE: the current dimension style's overall scale (and the header's), for dimensions
/// made from now on.
struct DimScaleCmd;

impl Command for DimScaleCmd {
    fn prompt(&self) -> Prompt {
        Prompt::new("Enter new value for DIMSCALE", want::NUM | want::ENTER)
    }
    fn feed(&mut self, cx: &mut Cx, inp: Inp) -> Flow {
        let Inp::Num(k) = inp else { return Flow::Done };
        if k <= 0.0 {
            return Flow::Bad("Requires a positive number.".into());
        }
        cx.hist.save_header(cx.d);
        if let Some(forge_dwg::header::Val::D(x)) = cx.d.header.get_mut("DIMSCALE") {
            *x = k;
        }
        if let Some(h) = cx.d.current("DIMSTYLE") {
            let _ = cx.d.modify(cx.hist, h, |o| {
                if let Data::DimStyle(s) = &mut o.data {
                    s.vars.scale = k;
                }
            });
        }
        Flow::Done
    }
}

// ---- ERASE

struct EraseCmd;

impl Command for EraseCmd {
    fn prompt(&self) -> Prompt {
        Prompt::new("Select objects", want::SEL)
    }
    fn feed(&mut self, cx: &mut Cx, inp: Inp) -> Flow {
        if let Inp::Sel(s) = inp {
            match cx.d.erase_many(cx.hist, &s) {
                Ok(()) => cx.say(format!("{} object(s) erased", s.len())),
                Err(e) => cx.say(format!("Can't erase them: {}", e.0)),
            }
        }
        Flow::Done
    }
}

// ---- MOVE, COPY, ROTATE, SCALE, MIRROR, STRETCH

#[derive(Clone, Copy, PartialEq, Debug)]
enum Xf {
    Move,
    Copy,
    Rotate,
    Scale,
    Mirror,
    Stretch,
}

#[derive(Clone, Copy, PartialEq)]
enum XStage {
    Sel,
    Base,
    Displacement,
    Second(V2),
    /// ROTATE's and SCALE's Reference: its first point, its second.
    Ref1(V2),
    Ref2(V2, V2),
    /// The reference taken (an angle, a length): the new one.
    New(V2, f64),
    /// MIRROR: erase the source?
    Erase(V2, V2),
}

struct XfCmd {
    kind: Xf,
    stage: XStage,
    sel: Vec<u32>,
    copy: bool,
    made: Vec<Vec<u32>>,
    window: Option<Bbox>,
}

impl XfCmd {
    fn new(kind: Xf) -> XfCmd {
        XfCmd {
            kind,
            stage: XStage::Sel,
            sel: vec![],
            copy: kind == Xf::Copy,
            made: vec![],
            window: None,
        }
    }
    /// The map for the second point (or number) after the base.
    fn map(&self, base: V2, at: V2) -> Option<Affine> {
        Some(match self.kind {
            Xf::Move | Xf::Copy | Xf::Stretch => Affine::translate(at - base),
            Xf::Rotate => Affine::rotate(base, (at - base).angle()),
            Xf::Scale => {
                let k = base.dist(at);
                if k <= 0.0 {
                    return None;
                }
                Affine::scale(base, k)
            }
            Xf::Mirror => {
                if at == base {
                    return None;
                }
                Affine::mirror(base, at)
            }
        })
    }
    fn stretch(&self, cx: &mut Cx, d: V2) {
        let Some(w) = self.window else {
            apply(cx, &self.sel, &Affine::translate(d), false);
            return;
        };
        for &h in &self.sel {
            match curve(cx.d, h) {
                Some(c) => {
                    if let Some(n) = xform::stretch(&c, &w, d) {
                        put(cx, h, &[n]);
                    }
                }
                None => {
                    // text, blocks, points: moved when their insertion point is in the window
                    let at = cx.d.objects.get(&h).and_then(|o| grip::grips(&o.data).first().map(|g| g.at));
                    if at.is_some_and(|p| w.contains(p)) {
                        apply(cx, &[h], &Affine::translate(d), false);
                    }
                }
            }
        }
    }
    fn go(&mut self, cx: &mut Cx, m: Affine) -> Flow {
        let made = apply(cx, &self.sel, &m, self.copy);
        let n = made.len();
        if self.kind == Xf::Copy {
            self.made.push(made);
            return Flow::More;
        }
        if !self.copy {
            cx.select_after = None;
        }
        let _ = n;
        Flow::Done
    }
}

impl Command for XfCmd {
    fn prompt(&self) -> Prompt {
        let what = match self.kind {
            Xf::Stretch => "Select objects to stretch by crossing-window",
            _ => "Select objects",
        };
        match self.stage {
            XStage::Sel => Prompt::new(what, want::SEL),
            XStage::Base => match self.kind {
                Xf::Mirror => Prompt::new("Specify first point of mirror line", want::POINT),
                Xf::Move | Xf::Copy | Xf::Stretch => {
                    Prompt::new("Specify base point", want::POINT).opts(&["Displacement"]).dflt("Displacement")
                }
                _ => Prompt::new("Specify base point", want::POINT),
            },
            XStage::Displacement => Prompt::new("Specify displacement", want::POINT).dflt("0.0000, 0.0000"),
            XStage::Second(b) => match self.kind {
                Xf::Move | Xf::Stretch => Prompt::new("Specify second point", want::POINT | want::ENTER)
                    .dflt("use first point as displacement")
                    .base(Some(b)),
                Xf::Copy => Prompt::new("Specify second point", want::POINT | want::ENTER)
                    .opts(&["Exit", "Undo"])
                    .dflt("Exit")
                    .base(Some(b)),
                Xf::Rotate => Prompt::new("Specify rotation angle", want::POINT | want::NUM)
                    .opts(&["Copy", "Reference"])
                    .dflt("0")
                    .base(Some(b)),
                Xf::Scale => Prompt::new("Specify scale factor", want::POINT | want::NUM)
                    .opts(&["Copy", "Reference"])
                    .base(Some(b)),
                Xf::Mirror => Prompt::new("Specify second point of mirror line", want::POINT).base(Some(b)),
            },
            XStage::Ref1(b) => {
                let w = if self.kind == Xf::Rotate { "Specify the reference angle" } else { "Specify reference length" };
                Prompt::new(w, want::POINT | want::NUM).dflt(if self.kind == Xf::Rotate { "0" } else { "1.0000" }).base(Some(b))
            }
            XStage::Ref2(_, p) => Prompt::new("Specify second point", want::POINT).base(Some(p)),
            XStage::New(b, _) => {
                let w = if self.kind == Xf::Rotate { "Specify the new angle" } else { "Specify new length" };
                Prompt::new(w, want::POINT | want::NUM).base(Some(b))
            }
            XStage::Erase(..) => Prompt::new("Erase source objects?", want::ENTER).opts(&["Yes", "No"]).dflt("No"),
        }
    }
    fn feed(&mut self, cx: &mut Cx, inp: Inp) -> Flow {
        match (self.stage, inp) {
            (XStage::Sel, Inp::Sel(s)) => {
                if s.is_empty() {
                    return Flow::Done;
                }
                self.sel = s;
                // STRETCH moves what is in the window the selection was made with
                self.window = cx.window.map(|(a, b)| Bbox::of([a, b]));
                self.stage = XStage::Base;
                Flow::More
            }
            (XStage::Base, Inp::Point(b)) => {
                self.stage = XStage::Second(b);
                Flow::More
            }
            (XStage::Base, Inp::Key(_) | Inp::Enter) if matches!(self.kind, Xf::Move | Xf::Copy | Xf::Stretch) => {
                self.stage = XStage::Displacement;
                Flow::More
            }
            (XStage::Displacement, Inp::Point(d)) => {
                if self.kind == Xf::Stretch {
                    self.stretch(cx, d);
                    return Flow::Done;
                }
                self.copy = self.kind == Xf::Copy;
                let f = self.go(cx, Affine::translate(d));
                if self.kind == Xf::Copy { Flow::Done } else { f }
            }
            (XStage::Displacement, Inp::Enter) => Flow::Done,
            (XStage::Second(b), Inp::Point(p)) => {
                if self.kind == Xf::Stretch {
                    self.stretch(cx, p - b);
                    return Flow::Done;
                }
                if self.kind == Xf::Mirror {
                    if p == b {
                        return Flow::Bad("The points are the same.".into());
                    }
                    self.stage = XStage::Erase(b, p);
                    return Flow::More;
                }
                match self.map(b, p) {
                    Some(m) => self.go(cx, m),
                    None => Flow::Bad("Invalid point.".into()),
                }
            }
            (XStage::Second(b), Inp::Num(x)) => {
                let m = match self.kind {
                    Xf::Rotate => Affine::rotate(b, x.to_radians()),
                    Xf::Scale if x > 0.0 => Affine::scale(b, x),
                    _ => return Flow::Bad("Requires a positive number.".into()),
                };
                self.go(cx, m)
            }
            (XStage::Second(b), Inp::Enter) => match self.kind {
                // the first point as the displacement
                Xf::Move => self.go(cx, Affine::translate(b)),
                Xf::Stretch => {
                    self.stretch(cx, b);
                    Flow::Done
                }
                Xf::Rotate => Flow::Done,
                _ => Flow::Done,
            },
            (XStage::Second(b), Inp::Key(k)) => match k {
                "Copy" => {
                    self.copy = true;
                    cx.say("Rotating/scaling a copy of the selected objects.");
                    Flow::More
                }
                "Reference" => {
                    self.stage = XStage::Ref1(b);
                    Flow::More
                }
                "Undo" => {
                    if let Some(last) = self.made.pop() {
                        for h in last {
                            erase(cx, h);
                        }
                    }
                    Flow::More
                }
                _ => Flow::Done,
            },
            (XStage::Ref1(b), Inp::Num(x)) => {
                let r = if self.kind == Xf::Rotate { x.to_radians() } else { x };
                if self.kind == Xf::Scale && r <= 0.0 {
                    return Flow::Bad("Requires a positive number.".into());
                }
                self.stage = XStage::New(b, r);
                Flow::More
            }
            (XStage::Ref1(b), Inp::Enter) => {
                self.stage = XStage::New(b, if self.kind == Xf::Rotate { 0.0 } else { 1.0 });
                Flow::More
            }
            (XStage::Ref1(b), Inp::Point(p)) => {
                self.stage = XStage::Ref2(b, p);
                Flow::More
            }
            (XStage::Ref2(b, p), Inp::Point(q)) => {
                let r = if self.kind == Xf::Rotate { (q - p).angle() } else { p.dist(q) };
                if self.kind == Xf::Scale && r <= 0.0 {
                    return Flow::Bad("Requires a positive length.".into());
                }
                self.stage = XStage::New(b, r);
                Flow::More
            }
            (XStage::New(b, r), inp @ (Inp::Num(_) | Inp::Point(_))) => {
                let m = match (self.kind, inp) {
                    (Xf::Rotate, Inp::Num(x)) => Affine::rotate(b, x.to_radians() - r),
                    (Xf::Rotate, Inp::Point(p)) => Affine::rotate(b, (p - b).angle() - r),
                    (_, Inp::Num(x)) if x > 0.0 => Affine::scale(b, x / r),
                    (_, Inp::Point(p)) if p != b => Affine::scale(b, b.dist(p) / r),
                    _ => return Flow::Bad("Invalid input.".into()),
                };
                self.go(cx, m)
            }
            (XStage::Erase(a, b), inp) => {
                let erase_src = matches!(inp, Inp::Key("Yes"));
                self.copy = !erase_src;
                self.go(cx, Affine::mirror(a, b))
            }
            _ => Flow::Bad("Invalid input.".into()),
        }
    }
    fn preview(&self, pv: &Pv, at: V2) -> Vec<Ghost> {
        let ents = |m: Affine| Ghost::Ents(self.sel.clone(), m);
        match self.stage {
            XStage::Second(b) => match self.kind {
                Xf::Mirror if at != b => vec![ents(Affine::mirror(b, at)), Ghost::Dashed(b, at)],
                Xf::Scale | Xf::Rotate => {
                    let mut v: Vec<Ghost> = self.map(b, at).map(ents).into_iter().collect();
                    v.push(Ghost::Dashed(b, at));
                    v
                }
                Xf::Stretch => {
                    let d = at - b;
                    let mut v = vec![line(b, at)];
                    if let Some(w) = self.window {
                        for &h in &self.sel {
                            if let Some(c) = curve(pv.d, h).and_then(|c| xform::stretch(&c, &w, d)) {
                                v.push(Ghost::Curve(c));
                            }
                        }
                    }
                    v
                }
                _ => self.map(b, at).map(|m| vec![ents(m), line(b, at)]).unwrap_or_default(),
            },
            XStage::New(b, r) => {
                let m = if self.kind == Xf::Rotate {
                    Some(Affine::rotate(b, (at - b).angle() - r))
                } else {
                    (at != b).then(|| Affine::scale(b, b.dist(at) / r))
                };
                m.map(|m| vec![ents(m), Ghost::Dashed(b, at)]).unwrap_or_default()
            }
            XStage::Ref2(_, p) => vec![Ghost::Dashed(p, at)],
            _ => vec![],
        }
    }
}


// ---- OFFSET

#[derive(Default, Clone, Copy, PartialEq)]
enum OStage {
    #[default]
    Dist,
    Dist2(V2),
    Obj,
    Side(u32),
}

#[derive(Default)]
struct OffsetCmd {
    stage: OStage,
    made: Vec<u32>,
}

impl OffsetCmd {
    fn result(&self, d: &Drawing, set: &super::dwgcmd::Settings, h: u32, at: V2) -> Option<Curve> {
        let c = curve(d, h)?;
        match set.offset {
            Some(dist) => offset::offset(&c, dist, at),
            None => offset::offset_through(&c, at),
        }
    }
}

impl Command for OffsetCmd {
    fn prompt(&self) -> Prompt {
        match self.stage {
            OStage::Dist => Prompt::new("Specify offset distance", want::NUM | want::POINT)
                .opts(&["Through"])
                .dflt("Through"),
            OStage::Dist2(a) => Prompt::new("Specify second point", want::POINT).base(Some(a)),
            OStage::Obj => Prompt::new("Select object to offset", want::PICK | want::ENTER)
                .opts(&["Exit", "Undo"])
                .dflt("Exit"),
            OStage::Side(_) => Prompt::new("Specify point on side to offset", want::POINT | want::ENTER)
                .opts(&["Exit", "Undo"])
                .dflt("Exit"),
        }
    }
    fn feed(&mut self, cx: &mut Cx, inp: Inp) -> Flow {
        match (self.stage, inp) {
            (OStage::Dist, Inp::Num(x)) if x > 0.0 => {
                cx.set.offset = Some(x);
                self.stage = OStage::Obj;
            }
            (OStage::Dist, Inp::Key(_) | Inp::Enter) => {
                cx.set.offset = None;
                self.stage = OStage::Obj;
            }
            (OStage::Dist, Inp::Point(a)) => self.stage = OStage::Dist2(a),
            (OStage::Dist2(a), Inp::Point(b)) if a != b => {
                cx.set.offset = Some(a.dist(b));
                self.stage = OStage::Obj;
            }
            (OStage::Obj, Inp::Pick(h, _)) => {
                if curve(cx.d, h).is_none() {
                    return Flow::Bad("Cannot offset that object.".into());
                }
                self.stage = OStage::Side(h);
            }
            (OStage::Side(h), Inp::Point(p)) => {
                match self.result(cx.d, cx.set, h, p) {
                    Some(c) => {
                        let like = cx.d.objects[&h].data.clone();
                        if let Some(data) = write_curve(&c, &like) {
                            match add_like(cx.d, cx.hist, h, data) {
                                Ok(n) => self.made.push(n),
                                Err(e) => cx.say(format!("Can't add it: {}", e.0)),
                            }
                        }
                    }
                    None => cx.say("Cannot offset that way."),
                }
                self.stage = OStage::Obj;
            }
            (_, Inp::Key("Undo")) => {
                if let Some(h) = self.made.pop() {
                    erase(cx, h);
                }
                self.stage = OStage::Obj;
            }
            (_, Inp::Key("Exit") | Inp::Enter) => return Flow::Done,
            _ => return Flow::Bad("Invalid input.".into()),
        }
        Flow::More
    }
    fn preview(&self, pv: &Pv, at: V2) -> Vec<Ghost> {
        match self.stage {
            OStage::Side(h) => self.result(pv.d, pv.set, h, at).map(|c| vec![Ghost::Curve(c)]).unwrap_or_default(),
            OStage::Dist2(a) => vec![line(a, at)],
            _ => vec![],
        }
    }
}

// ---- TRIM and EXTEND

struct TrimCmd {
    extend: bool,
}

/// The curves that might cross an entity: the entities in the cells its drawn segments pass.
fn edges_for(d: &Drawing, q: &super::dwgcmd::ViewQ, h: u32) -> Vec<Curve> {
    let mut segs = vec![];
    q.segments(h, 50_000, &mut segs);
    let mut hs: Vec<u32> = vec![];
    for (a, b) in segs {
        hs.extend(q.along(V2::from(a), V2::from(b)));
    }
    hs.sort_unstable();
    hs.dedup();
    hs.into_iter().filter(|x| *x != h).filter_map(|x| curve(d, x)).collect()
}

/// The curves that might stop an extension of `c` from the end nearer `pick`.
fn bounds_for(d: &Drawing, q: &super::dwgcmd::ViewQ, h: u32, c: &Curve, pick: V2) -> Vec<Curve> {
    let Some((lo, hi)) = q.extents() else { return vec![] };
    let far = (hi - lo).len() * 2.0 + 1.0;
    let start = ops::near_start(c, pick);
    let (t0, t1) = c.domain();
    let (p, dir) = if start {
        (c.start(), -c.tangent(t0))
    } else {
        (c.end(), c.tangent(t1))
    };
    let hs = match c {
        Curve::Arc(a) => q.in_box(a.c - v2(a.r, a.r), a.c + v2(a.r, a.r)),
        Curve::Ellipse(e) => {
            let r = e.major.len();
            q.in_box(e.c - v2(r, r), e.c + v2(r, r))
        }
        _ => q.along(p, p + dir * far),
    };
    hs.into_iter().filter(|x| *x != h).filter_map(|x| curve(d, x)).collect()
}

impl Command for TrimCmd {
    fn prompt(&self) -> Prompt {
        if self.extend {
            Prompt::new("Select object to extend or shift-select to trim", want::PICK | want::ENTER)
        } else {
            Prompt::new("Select object to trim or shift-select to extend", want::PICK | want::ENTER)
        }
    }
    fn feed(&mut self, cx: &mut Cx, inp: Inp) -> Flow {
        let Inp::Pick(h, p) = inp else {
            return if inp == Inp::Enter { Flow::Done } else { Flow::Bad("Select an object.".into()) };
        };
        let Some(c) = curve(cx.d, h) else {
            return Flow::Bad("Cannot trim or extend that object.".into());
        };
        let extend = self.extend != cx.shift;
        if extend {
            let bounds = bounds_for(cx.d, cx.q, h, &c, p);
            match ops::extend(&c, &bounds, p, false) {
                Some(n) => {
                    put(cx, h, &[n]);
                }
                None => cx.say("Object does not intersect an edge."),
            }
        } else {
            let edges = edges_for(cx.d, cx.q, h);
            match ops::trim(&c, &edges, p, false) {
                Some(pieces) => {
                    put(cx, h, &pieces);
                }
                None => {
                    // quick mode: nothing to trim to, so the object goes
                    erase(cx, h);
                    cx.say("No cutting edge: the object is erased.");
                }
            }
        }
        Flow::More
    }
}

// ---- FILLET and CHAMFER

#[derive(Clone, Copy, PartialEq)]
enum FStage {
    First,
    Second(u32, V2),
    Radius,
    Dist1,
    Dist2(f64),
    Poly,
}

struct FilletCmd {
    chamfer: bool,
    stage: FStage,
    multiple: bool,
}

impl FilletCmd {
    fn new(chamfer: bool) -> FilletCmd {
        FilletCmd {
            chamfer,
            stage: FStage::First,
            multiple: false,
        }
    }
    fn how(&self, set: &super::dwgcmd::Settings, corner: bool) -> Chamfer {
        if corner {
            Chamfer::Distances(0.0, 0.0)
        } else {
            Chamfer::Distances(set.chamfer.0, set.chamfer.1)
        }
    }
    fn settings(&self, set: &super::dwgcmd::Settings) -> String {
        if self.chamfer {
            format!("(TRIM mode) Current chamfer Dist1 = {}, Dist2 = {}", n4(set.chamfer.0), n4(set.chamfer.1))
        } else {
            format!("Current settings: Mode = TRIM, Radius = {}", n4(set.fillet_r))
        }
    }
    /// Which segment of a polyline a pick is on.
    fn seg_of(p: &Poly, at: V2) -> Option<usize> {
        let c = Curve::Poly(p.clone());
        let (t, _) = c.closest(at);
        Some((t.floor() as usize).min(p.nseg().checked_sub(1)?))
    }
    fn corner(&mut self, cx: &mut Cx, a: u32, pa: V2, b: u32, pb: V2) -> Flow {
        let corner = cx.shift;
        let r = if corner { 0.0 } else { cx.set.fillet_r };
        let how = self.how(cx.set, corner);
        let (Some(ca), Some(cb)) = (curve(cx.d, a), curve(cx.d, b)) else {
            return Flow::Bad("Cannot fillet that object.".into());
        };
        if a == b {
            // two segments of one polyline: the vertex between them
            let Curve::Poly(p) = &ca else {
                return Flow::Bad("Cannot fillet an object to itself.".into());
            };
            let (Some(i), Some(j)) = (Self::seg_of(p, pa), Self::seg_of(p, pb)) else {
                return Flow::Bad("Invalid pick.".into());
            };
            let n = p.nseg();
            let v = if (i + 1) % p.pts.len() == j && (i + 1 < n || p.closed) {
                (i + 1) % p.pts.len()
            } else if (j + 1) % p.pts.len() == i && (j + 1 < n || p.closed) {
                (j + 1) % p.pts.len()
            } else {
                return Flow::Bad("The segments are not next to each other.".into());
            };
            let np = if self.chamfer {
                fillet::chamfer_poly_vertex(p, v, how)
            } else {
                fillet::fillet_poly_vertex(p, v, r)
            };
            return match np {
                Some(np) => {
                    put(cx, a, &[Curve::Poly(np)]);
                    self.next()
                }
                None => Flow::Bad("Radius is too large.".into()),
            };
        }
        let k = if self.chamfer {
            fillet::chamfer(&ca, pa, &cb, pb, how)
        } else {
            fillet::fillet(&ca, pa, &cb, pb, r)
        };
        let Some(k) = k else {
            return Flow::Bad(if self.chamfer { "Distance is too large." } else { "Radius is too large." }.into());
        };
        let added = k.added.clone();
        put(cx, a, &[k.a]);
        put(cx, b, &[k.b]);
        if let Some(c) = added
            && let Some(like) = cx.d.objects.get(&a).map(|o| o.data.clone())
            && let Some(data) = write_curve(&c, &like)
        {
            let src = if cx.d.objects.contains_key(&a) { a } else { b };
            if let Err(e) = add_like(cx.d, cx.hist, src, data) {
                cx.say(format!("Can't add it: {}", e.0));
            }
        }
        self.next()
    }
    fn next(&mut self) -> Flow {
        if self.multiple {
            self.stage = FStage::First;
            Flow::More
        } else {
            Flow::Done
        }
    }
}

impl Command for FilletCmd {
    fn prompt(&self) -> Prompt {
        let opts: &[&str] = if self.chamfer {
            &["Polyline", "Distance", "Multiple"]
        } else {
            &["Polyline", "Radius", "Multiple"]
        };
        match self.stage {
            FStage::First => Prompt::new("Select first object", want::PICK).opts(opts),
            FStage::Second(..) => {
                Prompt::new("Select second object or shift-select to apply corner", want::PICK)
                    .opts(if self.chamfer { &["Distance"] } else { &["Radius"] })
            }
            FStage::Radius => Prompt::new("Specify fillet radius", want::NUM | want::ENTER),
            FStage::Dist1 => Prompt::new("Specify first chamfer distance", want::NUM),
            FStage::Dist2(d) => Prompt::new("Specify second chamfer distance", want::NUM).dflt(n4(d)),
            FStage::Poly => Prompt::new("Select 2D polyline", want::PICK),
        }
    }
    fn feed(&mut self, cx: &mut Cx, inp: Inp) -> Flow {
        if self.stage == FStage::First && matches!(inp, Inp::Pick(..)) {
            let s = self.settings(cx.set);
            cx.say(s);
        }
        match (self.stage, inp) {
            (FStage::First, Inp::Pick(h, p)) => self.stage = FStage::Second(h, p),
            (FStage::Second(a, pa), Inp::Pick(b, pb)) => return self.corner(cx, a, pa, b, pb),
            (_, Inp::Key("Radius")) => self.stage = FStage::Radius,
            (_, Inp::Key("Distance")) => self.stage = FStage::Dist1,
            (_, Inp::Key("Multiple")) => self.multiple = true,
            (_, Inp::Key("Polyline")) => self.stage = FStage::Poly,
            (FStage::Radius, Inp::Num(r)) if r >= 0.0 => {
                cx.set.fillet_r = r;
                self.stage = FStage::First;
            }
            (FStage::Radius, Inp::Enter) => self.stage = FStage::First,
            (FStage::Dist1, Inp::Num(d)) if d >= 0.0 => self.stage = FStage::Dist2(d),
            (FStage::Dist2(d1), Inp::Num(d)) if d >= 0.0 => {
                cx.set.chamfer = (d1, d);
                self.stage = FStage::First;
            }
            (FStage::Dist2(d1), Inp::Enter) => {
                cx.set.chamfer = (d1, d1);
                self.stage = FStage::First;
            }
            (FStage::Poly, Inp::Pick(h, _)) => {
                let Some(Curve::Poly(p)) = curve(cx.d, h) else {
                    return Flow::Bad("Not a polyline.".into());
                };
                let np = if self.chamfer {
                    fillet::chamfer_polyline(&p, self.how(cx.set, false))
                } else {
                    fillet::fillet_polyline(&p, cx.set.fillet_r)
                };
                let n = np.pts.len().saturating_sub(p.pts.len());
                put(cx, h, &[Curve::Poly(np)]);
                cx.say(format!("{n} lines were {}", if self.chamfer { "chamfered" } else { "filleted" }));
                return self.next();
            }
            (FStage::First, Inp::Enter) => return Flow::Done,
            _ => return Flow::Bad("Invalid input.".into()),
        }
        Flow::More
    }
    fn preview(&self, pv: &Pv, at: V2) -> Vec<Ghost> {
        let FStage::Second(a, pa) = self.stage else { return vec![] };
        let Some(h) = pv.q.pick(at, pv.q.px * 5.0) else { return vec![] };
        if h == a {
            return vec![];
        }
        let (Some(ca), Some(cb)) = (curve(pv.d, a), curve(pv.d, h)) else {
            return vec![];
        };
        let k = if self.chamfer {
            fillet::chamfer(&ca, pa, &cb, at, self.how(pv.set, false))
        } else {
            fillet::fillet(&ca, pa, &cb, at, pv.set.fillet_r)
        };
        k.map(|k| {
            let mut v = vec![Ghost::Curve(k.a), Ghost::Curve(k.b)];
            if let Some(c) = k.added {
                v.push(Ghost::Curve(c));
            }
            v
        })
        .unwrap_or_default()
    }
}

// ---- BREAK

#[derive(Default, Clone, Copy, PartialEq)]
enum BStage {
    #[default]
    Obj,
    Second(u32, V2),
    First(u32),
}

#[derive(Default)]
struct BreakCmd {
    stage: BStage,
}

impl Command for BreakCmd {
    fn prompt(&self) -> Prompt {
        match self.stage {
            BStage::Obj => Prompt::new("Select object", want::PICK),
            BStage::Second(_, p) => Prompt::new("Specify second break point", want::POINT)
                .opts(&["First point"])
                .base(Some(p)),
            BStage::First(_) => Prompt::new("Specify first break point", want::POINT),
        }
    }
    fn feed(&mut self, cx: &mut Cx, inp: Inp) -> Flow {
        match (self.stage, inp) {
            (BStage::Obj, Inp::Pick(h, p)) => {
                if curve(cx.d, h).is_none() {
                    return Flow::Bad("Cannot break that object.".into());
                }
                self.stage = BStage::Second(h, p);
                Flow::More
            }
            (BStage::Second(h, _), Inp::Key(_)) => {
                self.stage = BStage::First(h);
                Flow::More
            }
            (BStage::First(h), Inp::Point(p)) => {
                self.stage = BStage::Second(h, p);
                Flow::More
            }
            (BStage::Second(h, p1), Inp::Point(p2)) => {
                let Some(c) = curve(cx.d, h) else { return Flow::Done };
                let r = if p1 == p2 { ops::break_at(&c, p1, None) } else { ops::break_at(&c, p1, Some(p2)) };
                match r {
                    Some(pieces) => {
                        put(cx, h, &pieces);
                    }
                    None => cx.say("Cannot break it there."),
                }
                Flow::Done
            }
            _ => Flow::Bad("Invalid input.".into()),
        }
    }
}

// ---- JOIN

struct JoinCmd;

impl Command for JoinCmd {
    fn prompt(&self) -> Prompt {
        Prompt::new("Select objects to join", want::SEL)
    }
    fn feed(&mut self, cx: &mut Cx, inp: Inp) -> Flow {
        let Inp::Sel(sel) = inp else { return Flow::Done };
        let hs: Vec<u32> = sel.into_iter().filter(|h| curve(cx.d, *h).is_some()).collect();
        let cs: Vec<Curve> = hs.iter().filter_map(|h| curve(cx.d, *h)).collect();
        let joined = ops::join(&cs, None);
        let mut gone = 0;
        for j in joined.iter().filter(|j| j.from.len() > 1) {
            let first = hs[j.from[0]];
            put(cx, first, std::slice::from_ref(&j.curve));
            for &k in &j.from[1..] {
                erase(cx, hs[k]);
                gone += 1;
            }
        }
        cx.say(format!("{} objects joined into {}", hs.len(), hs.len() - gone));
        Flow::Done
    }
}

// ---- LENGTHEN

#[derive(Default, Clone, Copy, PartialEq)]
enum LStage {
    #[default]
    Measure,
    Delta,
    Percent,
    Total,
    Change,
}

#[derive(Default)]
struct LengthenCmd {
    stage: LStage,
}

impl Command for LengthenCmd {
    fn prompt(&self) -> Prompt {
        match self.stage {
            LStage::Measure => Prompt::new("Select an object to measure", want::PICK).opts(&["DElta", "Percent", "Total"]),
            LStage::Delta => Prompt::new("Enter delta length", want::NUM),
            LStage::Percent => Prompt::new("Enter percentage length", want::NUM),
            LStage::Total => Prompt::new("Specify total length", want::NUM),
            LStage::Change => Prompt::new("Select an object to change", want::PICK | want::ENTER),
        }
    }
    fn feed(&mut self, cx: &mut Cx, inp: Inp) -> Flow {
        match (self.stage, inp) {
            (LStage::Measure, Inp::Pick(h, _)) => {
                if let Some(c) = curve(cx.d, h) {
                    cx.say(format!("Current length: {}", n4(c.length())));
                }
            }
            (LStage::Measure, Inp::Key(k)) => {
                self.stage = match k {
                    "DElta" => LStage::Delta,
                    "Percent" => LStage::Percent,
                    _ => LStage::Total,
                }
            }
            (LStage::Delta, Inp::Num(x)) => {
                cx.set.lengthen = Lengthen::Delta(x);
                self.stage = LStage::Change;
            }
            (LStage::Percent, Inp::Num(x)) if x > 0.0 => {
                cx.set.lengthen = Lengthen::Percent(x);
                self.stage = LStage::Change;
            }
            (LStage::Total, Inp::Num(x)) if x > 0.0 => {
                cx.set.lengthen = Lengthen::Total(x);
                self.stage = LStage::Change;
            }
            (LStage::Change, Inp::Pick(h, p)) => match curve(cx.d, h).and_then(|c| ops::lengthen(&c, p, cx.set.lengthen)) {
                Some(n) => {
                    put(cx, h, &[n]);
                }
                None => cx.say("Cannot lengthen that object that way."),
            },
            (_, Inp::Enter) => return Flow::Done,
            _ => return Flow::Bad("Invalid input.".into()),
        }
        Flow::More
    }
}

// ---- ARRAY

#[derive(Default, Clone, Copy, PartialEq)]
enum ArStage {
    #[default]
    Sel,
    Kind,
    Rows,
    Cols(usize),
    RowSp(usize, usize),
    ColSp(usize, usize, f64),
    Center,
    Items(V2),
    Fill(V2, usize),
    Rot(V2, usize, f64),
}

#[derive(Default)]
struct ArrayCmd {
    stage: ArStage,
    sel: Vec<u32>,
    size: (f64, f64),
}

impl ArrayCmd {
    fn place(&self, cx: &mut Cx, maps: &[Affine]) {
        let mut n = 0;
        for m in maps.iter().skip(1) {
            n += apply(cx, &self.sel, m, true).len();
        }
        cx.say(format!("{n} copies made"));
    }
}

impl Command for ArrayCmd {
    fn prompt(&self) -> Prompt {
        match self.stage {
            ArStage::Sel => Prompt::new("Select objects", want::SEL),
            ArStage::Kind => Prompt::new("Enter array type", want::ENTER)
                .opts(&["Rectangular", "POlar"])
                .dflt("Rectangular"),
            ArStage::Rows => Prompt::new("Enter the number of rows", want::NUM).dflt("3"),
            ArStage::Cols(_) => Prompt::new("Enter the number of columns", want::NUM).dflt("4"),
            ArStage::RowSp(..) => Prompt::new("Specify the distance between rows", want::NUM).dflt(n4(self.size.1)),
            ArStage::ColSp(..) => Prompt::new("Specify the distance between columns", want::NUM).dflt(n4(self.size.0)),
            ArStage::Center => Prompt::new("Specify center point of array", want::POINT),
            ArStage::Items(_) => Prompt::new("Enter number of items in the array", want::NUM).dflt("6"),
            ArStage::Fill(..) => Prompt::new("Specify the angle to fill (+=ccw, -=cw)", want::NUM).dflt("360"),
            ArStage::Rot(..) => Prompt::new("Rotate arrayed objects?", want::ENTER).opts(&["Yes", "No"]).dflt("Yes"),
        }
    }
    fn feed(&mut self, cx: &mut Cx, inp: Inp) -> Flow {
        let num = |inp: &Inp, d: f64| match inp {
            Inp::Num(x) => Some(*x),
            Inp::Enter => Some(d),
            _ => None,
        };
        self.stage = match (self.stage, &inp) {
            (ArStage::Sel, Inp::Sel(s)) => {
                if s.is_empty() {
                    return Flow::Done;
                }
                self.sel = s.clone();
                let b = sel_box(cx.q, s).unwrap_or(Bbox {
                    min: V2::ZERO,
                    max: v2(1.0, 1.0),
                });
                self.size = (((b.max.x - b.min.x) * 1.5).max(1e-6), ((b.max.y - b.min.y) * 1.5).max(1e-6));
                ArStage::Kind
            }
            (ArStage::Kind, Inp::Key("POlar")) => ArStage::Center,
            (ArStage::Kind, Inp::Key(_) | Inp::Enter) => ArStage::Rows,
            (ArStage::Rows, i) => match num(i, 3.0) {
                Some(r) if r >= 1.0 => ArStage::Cols(r as usize),
                _ => return Flow::Bad("Requires a whole number of 1 or more.".into()),
            },
            (ArStage::Cols(r), i) => match num(i, 4.0) {
                Some(c) if c >= 1.0 => ArStage::RowSp(r, c as usize),
                _ => return Flow::Bad("Requires a whole number of 1 or more.".into()),
            },
            (ArStage::RowSp(r, c), i) => match num(i, self.size.1) {
                Some(s) => ArStage::ColSp(r, c, s),
                None => return Flow::Bad("Requires a distance.".into()),
            },
            (ArStage::ColSp(r, c, rs), i) => match num(i, self.size.0) {
                Some(cs) => {
                    let maps = xform::array_rect(r, c, rs, cs, 0.0);
                    self.place(cx, &maps);
                    return Flow::Done;
                }
                None => return Flow::Bad("Requires a distance.".into()),
            },
            (ArStage::Center, Inp::Point(c)) => ArStage::Items(*c),
            (ArStage::Items(c), i) => match num(i, 6.0) {
                Some(n) if n >= 2.0 => ArStage::Fill(c, n as usize),
                _ => return Flow::Bad("Requires a whole number of 2 or more.".into()),
            },
            (ArStage::Fill(c, n), i) => match num(i, 360.0) {
                Some(f) if f != 0.0 => ArStage::Rot(c, n, f.to_radians()),
                _ => return Flow::Bad("Requires a nonzero angle.".into()),
            },
            (ArStage::Rot(c, n, f), i) => {
                let rot = !matches!(i, Inp::Key("No"));
                let base = sel_box(cx.q, &self.sel).map(|b| b.center()).unwrap_or(c);
                let maps = xform::array_polar(c, n, f, rot, base);
                self.place(cx, &maps);
                return Flow::Done;
            }
            _ => return Flow::Bad("Invalid input.".into()),
        };
        Flow::More
    }
}

// ---- EXPLODE

struct ExplodeCmd;

impl Command for ExplodeCmd {
    fn prompt(&self) -> Prompt {
        Prompt::new("Select objects", want::SEL)
    }
    fn feed(&mut self, cx: &mut Cx, inp: Inp) -> Flow {
        let Inp::Sel(sel) = inp else { return Flow::Done };
        let (mut ok, mut not) = (0, 0);
        for h in sel {
            match explode_entity(cx.d, cx.hist, h) {
                Ok(v) if !v.is_empty() => ok += 1,
                _ => not += 1,
            }
        }
        if not > 0 {
            cx.say(format!("{not} object(s) could not be exploded."));
        }
        cx.say(format!("{ok} object(s) exploded."));
        Flow::Done
    }
}

// ---- MATCHPROP

#[derive(Default)]
struct MatchCmd {
    src: Option<u32>,
}

impl Command for MatchCmd {
    fn prompt(&self) -> Prompt {
        match self.src {
            None => Prompt::new("Select source object", want::PICK),
            Some(_) => Prompt::new("Select destination object(s)", want::SEL),
        }
    }
    fn feed(&mut self, cx: &mut Cx, inp: Inp) -> Flow {
        match (self.src, inp) {
            (None, Inp::Pick(h, _)) => {
                self.src = Some(h);
                cx.say("Current active settings: Color Layer Ltype Ltscale Lineweight");
                Flow::More
            }
            (Some(s), Inp::Sel(dst)) => {
                let Some(e) = cx.d.objects.get(&s).and_then(|o| o.ent.clone()) else {
                    return Flow::Done;
                };
                for h in dst {
                    let e = e.clone();
                    let _ = cx.d.modify(cx.hist, h, |o| {
                        if let Some(x) = o.ent.as_mut() {
                            x.layer = e.layer;
                            x.color = e.color.clone();
                            x.ltype_flags = e.ltype_flags;
                            x.ltype = e.ltype;
                            x.ltype_scale = e.ltype_scale;
                            x.lineweight = e.lineweight;
                        }
                    });
                }
                Flow::Done
            }
            _ => Flow::Bad("Invalid input.".into()),
        }
    }
}

// ---- grips

#[derive(Clone, Copy, PartialEq, Debug)]
enum GMode {
    Stretch,
    Move,
    Rotate,
    Scale,
    Mirror,
}

/// A grip taken: STRETCH by default; Enter (or Space) goes on to MOVE, ROTATE, SCALE, MIRROR.
pub struct GripEdit {
    at: V2,
    base: V2,
    held: Vec<(u32, Grip)>,
    sel: Vec<u32>,
    mode: GMode,
    copy: bool,
    base_next: bool,
}

impl GripEdit {
    pub fn new(at: V2, held: Vec<(u32, Grip)>, sel: Vec<u32>) -> GripEdit {
        GripEdit {
            at,
            base: at,
            held,
            sel,
            mode: GMode::Stretch,
            copy: false,
            base_next: false,
        }
    }
    fn map(&self, p: V2) -> Option<Affine> {
        let b = self.base;
        Some(match self.mode {
            GMode::Stretch | GMode::Move => Affine::translate(p - b),
            GMode::Rotate => Affine::rotate(b, (p - b).angle()),
            GMode::Scale => {
                let k = b.dist(p);
                if k <= 0.0 {
                    return None;
                }
                Affine::scale(b, k)
            }
            GMode::Mirror => {
                if p == b {
                    return None;
                }
                Affine::mirror(b, p)
            }
        })
    }
    fn stretched(&self, d: &Drawing, p: V2) -> Vec<(u32, Data)> {
        self.held
            .iter()
            .filter_map(|(h, g)| {
                let o = d.objects.get(h)?;
                Some((*h, grip::drag(&o.data, g.role, g.at, p)?))
            })
            .collect()
    }
}

impl Command for GripEdit {
    fn prompt(&self) -> Prompt {
        if self.base_next {
            return Prompt::new("Specify base point", want::POINT);
        }
        let (name, what) = match self.mode {
            GMode::Stretch => ("STRETCH", "Specify stretch point"),
            GMode::Move => ("MOVE", "Specify move point"),
            GMode::Rotate => ("ROTATE", "Specify rotation angle"),
            GMode::Scale => ("SCALE", "Specify scale factor"),
            GMode::Mirror => ("MIRROR", "Specify second point"),
        };
        let w = match self.mode {
            GMode::Rotate | GMode::Scale => want::POINT | want::NUM | want::ENTER,
            _ => want::POINT | want::ENTER,
        };
        Prompt::new(format!("** {name} ** {what}"), w)
            .opts(&["Base point", "Copy", "eXit"])
            .base(Some(self.base))
    }
    fn feed(&mut self, cx: &mut Cx, inp: Inp) -> Flow {
        match inp {
            Inp::Enter => {
                self.mode = match self.mode {
                    GMode::Stretch => GMode::Move,
                    GMode::Move => GMode::Rotate,
                    GMode::Rotate => GMode::Scale,
                    GMode::Scale => GMode::Mirror,
                    GMode::Mirror => GMode::Stretch,
                };
                Flow::More
            }
            Inp::Key("eXit") => Flow::Done,
            Inp::Key("Copy") => {
                self.copy = true;
                Flow::More
            }
            Inp::Key(_) => {
                self.base_next = true;
                Flow::More
            }
            Inp::Point(p) if self.base_next => {
                self.base = p;
                self.base_next = false;
                Flow::More
            }
            Inp::Point(p) => {
                if self.mode == GMode::Stretch {
                    if self.copy {
                        // a stretched copy of each held entity
                        for (h, data) in self.stretched(cx.d, p) {
                            let _ = add_like(cx.d, cx.hist, h, data);
                        }
                        return Flow::More;
                    }
                    for (h, data) in self.stretched(cx.d, p) {
                        let same = cx.d.objects.get(&h).is_some_and(|o| std::mem::discriminant(&o.data) == std::mem::discriminant(&data));
                        if same {
                            let _ = cx.d.modify(cx.hist, h, |o| o.data = data);
                        } else {
                            let _ = replace(cx.d, cx.hist, h, vec![data]);
                        }
                    }
                    cx.select_after = Some(self.sel.clone());
                    return Flow::Done;
                }
                let Some(m) = self.map(p) else { return Flow::Bad("Invalid point.".into()) };
                apply(cx, &self.sel, &m, self.copy);
                if self.copy {
                    return Flow::More;
                }
                cx.select_after = Some(self.sel.clone());
                Flow::Done
            }
            Inp::Num(x) => {
                let b = self.base;
                let m = match self.mode {
                    GMode::Rotate => Affine::rotate(b, x.to_radians()),
                    GMode::Scale if x > 0.0 => Affine::scale(b, x),
                    _ => return Flow::Bad("Requires a point.".into()),
                };
                apply(cx, &self.sel, &m, self.copy);
                if self.copy {
                    return Flow::More;
                }
                cx.select_after = Some(self.sel.clone());
                Flow::Done
            }
            _ => Flow::Bad("Invalid input.".into()),
        }
    }
    fn preview(&self, pv: &Pv, at: V2) -> Vec<Ghost> {
        if self.base_next {
            return vec![];
        }
        if self.mode == GMode::Stretch {
            let mut v: Vec<Ghost> = self
                .stretched(pv.d, at)
                .into_iter()
                .filter_map(|(_, d)| read_entity(&d).ok().map(|f| Ghost::Curve(f.curve)))
                .collect();
            v.push(line(self.at, at));
            return v;
        }
        let mut v: Vec<Ghost> = self.map(at).map(|m| Ghost::Ents(self.sel.clone(), m)).into_iter().collect();
        v.push(Ghost::Dashed(self.base, at));
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_drafting_command_is_made() {
        for name in [
            "LINE", "PLINE", "CIRCLE", "ARC", "RECTANG", "POLYGON", "ELLIPSE", "SPLINE", "POINT", "TEXT", "MTEXT",
            "HATCH", "DIM", "DIMLINEAR", "DIMALIGNED", "DIMRADIUS", "DIMDIAMETER", "ERASE", "MOVE", "COPY",
            "ROTATE", "SCALE", "MIRROR", "STRETCH", "OFFSET", "TRIM", "EXTEND", "FILLET", "CHAMFER", "BREAK",
            "JOIN", "LENGTHEN", "ARRAY", "EXPLODE", "MATCHPROP",
        ] {
            assert!(make(name).is_some(), "{name}");
            assert!(super::super::dwgdoc::command_of(name).is_some_and(|c| c.works), "{name}");
        }
    }

    #[test]
    fn a_rectangle_with_rounded_corners() {
        let c = rect_poly(v2(0.0, 0.0), v2(10.0, 5.0), 1.0).unwrap();
        let Curve::Poly(p) = &c else { panic!() };
        assert_eq!(p.pts.len(), 8);
        let l = c.length();
        let want = 2.0 * (8.0 + 3.0) + TAU;
        assert!((l - want).abs() < 1e-9, "{l} {want}");
    }
}
