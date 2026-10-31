//! Random part histories over every feature kind, written as part files. Each line is made for the
//! part as it stands: placed on its faces, its edges and faces picked by selectors (`near` a
//! point, `of` or `between` labelled lines), never by handle or line number, so a history keeps
//! its meaning when the shrinker drops lines above. A line the kernel refuses with a clear error
//! is dropped and the history goes on; a panic, an invalid body, an open mesh, or an n-ary regen
//! that disagrees with the sequential one is a failure.

use crate::model::Model;
use crate::select::{self, Ent};
use forge_geom::CurveGeom;
use forge_math::rng::Rng;
use forge_math::{Box3, Point3, Vec3, dmath};
use forge_mesh::{Quality, tessellate};
use forge_model::{Feature, Part, Source};
use forge_topo::{Body, FaceId};
use std::panic::{AssertUnwindSafe, catch_unwind};

/// One feature kind's generator: lines (a work plane may come first) for the part as it
/// stands, or none where the kind has nothing to work on.
pub struct Maker {
    pub name: &'static str,
    weight: usize,
    make: fn(&mut G) -> Option<Vec<String>>,
}

/// Every feature kind the part file has, each with how often it is drawn.
pub const MAKERS: &[Maker] = &[
    Maker {
        name: "extrude",
        weight: 7,
        make: extrude,
    },
    Maker {
        name: "revolve",
        weight: 3,
        make: revolve,
    },
    Maker {
        name: "ball",
        weight: 2,
        make: ball,
    },
    Maker {
        name: "fillet",
        weight: 7,
        make: fillet,
    },
    Maker {
        name: "chamfer",
        weight: 4,
        make: chamfer,
    },
    Maker {
        name: "shell",
        weight: 1,
        make: shell,
    },
    Maker {
        name: "draft",
        weight: 2,
        make: draft,
    },
    Maker {
        name: "hole",
        weight: 5,
        make: hole,
    },
    Maker {
        name: "pattern",
        weight: 3,
        make: pattern,
    },
    Maker {
        name: "sweep",
        weight: 2,
        make: sweep,
    },
    Maker {
        name: "coil",
        weight: 1,
        make: coil,
    },
    Maker {
        name: "loft",
        weight: 2,
        make: loft,
    },
    Maker {
        name: "plane",
        weight: 1,
        make: plane,
    },
    Maker {
        name: "facefillet",
        weight: 2,
        make: facefillet,
    },
    Maker {
        name: "fullround",
        weight: 1,
        make: fullround,
    },
    Maker {
        name: "param",
        weight: 1,
        make: param,
    },
    Maker {
        name: "sketch",
        weight: 2,
        make: sketch,
    },
    Maker {
        name: "rib",
        weight: 2,
        make: rib,
    },
    Maker {
        name: "offset",
        weight: 1,
        make: offset,
    },
    Maker {
        name: "thicken",
        weight: 1,
        make: thicken,
    },
    Maker {
        name: "delete",
        weight: 1,
        make: delete,
    },
    Maker {
        name: "replace",
        weight: 1,
        make: replace,
    },
    Maker {
        name: "move",
        weight: 1,
        make: move_faces,
    },
    Maker {
        name: "axis",
        weight: 1,
        make: axis,
    },
    Maker {
        name: "point",
        weight: 1,
        make: point,
    },
    Maker {
        name: "ucs",
        weight: 1,
        make: ucs,
    },
    Maker {
        name: "combine",
        weight: 1,
        make: combine,
    },
    Maker {
        name: "split",
        weight: 1,
        make: split,
    },
    Maker {
        name: "splitface",
        weight: 1,
        make: splitface,
    },
    Maker {
        name: "movebody",
        weight: 1,
        make: movebody,
    },
    Maker {
        name: "copybody",
        weight: 1,
        make: copybody,
    },
    Maker {
        name: "thread",
        weight: 1,
        make: thread,
    },
    Maker {
        name: "text",
        weight: 1,
        make: text,
    },
];

/// The seeds the tests run.
pub const SEEDS: [u64; 34] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28,
    29, 30, 31, 32, 33,
];

/// Mesh chord for the volume checks.
const CHORD: f64 = 0.004;

/// What a generator sees: the part so far and the dice.
pub struct G<'a> {
    r: &'a mut Rng,
    m: &'a Model,
    body: &'a Body,
    bx: Box3,
    label: &'a mut usize,
}

impl G<'_> {
    fn label(&mut self) -> String {
        *self.label += 1;
        format!("L{}:", *self.label)
    }
    fn range(&mut self, lo: f64, hi: f64) -> f64 {
        self.r.range(lo, hi)
    }
    fn one_in(&mut self, n: usize) -> bool {
        self.r.below(n) == 0
    }
    fn pick<T: Clone>(&mut self, xs: &[T]) -> Option<T> {
        (!xs.is_empty()).then(|| xs[self.r.below(xs.len())].clone())
    }
    /// A size on the part's scale.
    fn size(&mut self, lo: f64, hi: f64) -> f64 {
        let d = self.bx.diagonal();
        self.range(lo * d, hi * d)
    }
    /// The labels of earlier lines whose kind passes `keep`.
    fn labels(&self, keep: impl Fn(&Feature) -> bool) -> Vec<String> {
        self.m
            .features
            .iter()
            .zip(&self.m.labels)
            .filter(|(f, _)| keep(f))
            .filter_map(|(_, l)| l.clone())
            .collect()
    }
    fn planes(&self) -> Vec<String> {
        self.labels(|f| matches!(f, Feature::Plane(_)))
    }
    /// Labels of solid features a pattern may copy.
    fn solids(&self) -> Vec<String> {
        self.labels(|f| {
            !matches!(
                f,
                Feature::Plane(_)
                    | Feature::Param(_)
                    | Feature::Sketch(_)
                    | Feature::Work(_)
                    | Feature::Edit(_)
                    | Feature::Fillet { .. }
                    | Feature::FaceFillet { .. }
                    | Feature::FullRound { .. }
                    | Feature::Chamfer { .. }
                    | Feature::Shell { .. }
                    | Feature::Draft { .. }
                    | Feature::Pattern { of: Source::Body, .. }
            )
        })
    }
    fn op(&mut self) -> &'static str {
        match self.r.below(40) {
            0 => "keep",
            k if k < 20 => "cut",
            _ => "add",
        }
    }
    /// A point on a random planar face and the face's outward normal (the top of the box
    /// sometimes, or when there is no planar face).
    fn spot(&mut self) -> (Point3, Vec3) {
        let planar: Vec<FaceId> = self
            .body
            .face_ids()
            .filter(|&f| select::plane_normal(self.body, f).is_some())
            .collect();
        match self.pick(&planar) {
            Some(f) if !self.one_in(5) => {
                let n = select::plane_normal(self.body, f).expect("planar");
                let fb = self.body.face_box(f);
                let edges = select::edges_of_face(self.body, f);
                let at = match self.pick(&edges) {
                    Some(e) => {
                        let m = select::centre(self.body, Ent::Edge(e));
                        let t = self.range(0.15, 0.6);
                        m + (fb.center() - m) * t
                    }
                    None => fb.center(),
                };
                (at, n)
            }
            _ => {
                let (x, y) = (
                    self.range(self.bx.x.lo, self.bx.x.hi),
                    self.range(self.bx.y.lo, self.bx.y.hi),
                );
                (Point3::new(x, y, self.bx.z.hi), Vec3::Z)
            }
        }
    }
    /// `n`, tipped a little now and then.
    fn tilt(&mut self, n: Vec3) -> Vec3 {
        if self.one_in(4) {
            let t = Vec3::new(
                self.range(-0.3, 0.3),
                self.range(-0.3, 0.3),
                self.range(-0.3, 0.3),
            );
            (n + t).normalize().map_or(n, |u| u.get())
        } else {
            n
        }
    }
    /// A selector (kind words `what`, then `near` the entity's centre) that picks exactly `e`
    /// on the part as it stands.
    fn sel(&self, e: Ent, what: &str) -> Option<String> {
        let all = if matches!(e, Ent::Face(_)) {
            "faces"
        } else {
            "edges"
        };
        let s = format!("({all}{what} near {})", p3(select::centre(self.body, e)));
        let (_, hits) = self.m.select(&s, None).ok()?;
        (hits == [e]).then_some(s)
    }
    /// A selector over the whole part that finds between 1 and `most` entities.
    fn found(&self, s: &str, most: usize) -> bool {
        self.m
            .select(s, None)
            .is_ok_and(|(_, h)| !h.is_empty() && h.len() <= most)
    }
    /// Edges where two faces meet at an angle.
    fn creases(&self) -> Vec<forge_topo::EdgeId> {
        self.body
            .edge_ids()
            .filter(|&e| matches!(select::crease(self.body, e), Some("convex" | "concave")))
            .collect()
    }
    /// Up to `n` edges (by selectors) and the length of the shortest.
    fn edges(&mut self, n: usize) -> Option<(Vec<String>, f64)> {
        let creases = self.creases();
        // a whole feature's rims or seams at once, sometimes
        if self.one_in(3) {
            let ls = self.solids();
            let a = self.pick(&ls)?;
            let b = self.pick(&ls)?;
            let words = [
                "convex",
                "concave",
                "circle",
                "line",
                "convex circle",
                "concave line",
            ];
            let w = self.pick(&words)?;
            let s = if a != b && self.one_in(2) {
                format!("(edges between {} {})", trim(&a), trim(&b))
            } else {
                format!("(edges {w} of {})", trim(&a))
            };
            if self.found(&s, 12) {
                let (_, hits) = self.m.select(&s, None).ok()?;
                let len = hits
                    .iter()
                    .filter_map(|e| match e {
                        Ent::Edge(x) => Some(select::length(self.body, *x)),
                        _ => None,
                    })
                    .fold(f64::INFINITY, f64::min);
                return Some((vec![s], len));
            }
        }
        let mut out = Vec::new();
        let mut len = f64::INFINITY;
        for _ in 0..1 + self.r.below(n) {
            let e = self.pick(&creases)?;
            let what = match (select::crease(self.body, e), self.r.below(3)) {
                (Some(c), 0) => format!(" {c}"),
                _ => String::new(),
            };
            if let Some(s) = self.sel(Ent::Edge(e), &what)
                && !out.contains(&s)
            {
                len = len.min(select::length(self.body, e));
                out.push(s);
            }
        }
        (!out.is_empty()).then_some((out, len))
    }
    /// A face of the part: planar if `planar`, and passing `keep` on its normal.
    fn face(&mut self, planar: bool, keep: impl Fn(Option<Vec3>) -> bool) -> Option<(FaceId, String)> {
        let fs: Vec<FaceId> = self
            .body
            .face_ids()
            .filter(|&f| {
                let n = select::plane_normal(self.body, f);
                (!planar || n.is_some()) && keep(n)
            })
            .collect();
        for _ in 0..4 {
            let f = self.pick(&fs)?;
            if let Some(s) = self.sel(Ent::Face(f), if planar { " plane" } else { "" }) {
                return Some((f, s));
            }
        }
        None
    }
    /// An end for a solid travelling from `at` along `dir`: a work plane ahead (made here if
    /// there is none), or a face of an earlier feature.
    fn upto(&mut self, at: Point3, dir: Vec3, out: &mut Vec<String>) -> Option<String> {
        if self.one_in(3) {
            let ls = self.solids();
            let l = self.pick(&ls)?;
            let s = format!("(faces of {})", trim(&l));
            let (_, hits) = self.m.select(&s, None).ok()?;
            let f = self.pick(&hits)?;
            let what = format!(" of {}", trim(&l));
            let face = self.sel(f, &what)?;
            return Some(format!("to {} {face}{}", trim(&l), self.offset()));
        }
        let planes = self.planes();
        let p = match self.pick(&planes) {
            Some(p) if self.one_in(2) => trim(&p).to_string(),
            _ => {
                let d = self.size(0.05, 0.4);
                let z = self.tilt(dir);
                let l = self.label();
                out.push(format!("{l} plane z {} at {}", v3(z), p3(at + dir * d)));
                trim(&l).to_string()
            }
        };
        Some(format!("to {p}{}", self.offset()))
    }
    fn offset(&mut self) -> String {
        if self.one_in(3) {
            format!(" off {}", n(self.range(-2.0, 2.0)))
        } else {
            String::new()
        }
    }
    /// Profile loops round the sketch origin, about `s` across.
    fn loops(&mut self, s: f64) -> String {
        let h = s / 2.0;
        let mut out = match self.r.below(6) {
            0 | 1 => format!("rect {} {} {} {}", n(-h), n(-h * 0.6), n(s), n(s * 0.6)),
            2 => format!(
                "rrect {} {} {} {} {}",
                n(-h),
                n(-h * 0.6),
                n(s),
                n(s * 0.6),
                n(s * self.range(0.05, 0.29))
            ),
            3 => format!("circle 0 0 {}", n(h)),
            4 => {
                let k = 3 + self.r.below(4);
                let mut pts = String::from("poly");
                for i in 0..k {
                    let a = dmath::TAU * (i as f64 + self.range(-0.3, 0.3)) / k as f64;
                    let (sa, ca) = dmath::sin_cos(a);
                    let rr = h * self.range(0.6, 1.0);
                    pts.push_str(&format!(" {} {}", n(rr * ca), n(rr * sa)));
                }
                pts
            }
            _ => format!("loop line {} 0 {} 0 arc 0 0 {} 0 180", n(-h), n(h), n(h)),
        };
        if self.one_in(5) {
            out.push_str(&format!(" circle 0 {} {}", n(s * 0.05), n(s * 0.12)));
        }
        out
    }
}

/// A label without its colon.
fn trim(l: &str) -> &str {
    l.trim_end_matches(':')
}

/// A number as Rust writes it: the shortest that reads back the same.
fn fmt_num(x: f64) -> String {
    format!("{x}")
}

/// A generated value, to two decimals.
fn rd(x: f64) -> f64 {
    (x * 100.0).round() / 100.0 + 0.0
}

fn rp(p: Point3) -> Point3 {
    Point3::new(rd(p.x), rd(p.y), rd(p.z))
}

fn n(x: f64) -> String {
    fmt_num(rd(x))
}

/// A derived number, exactly.
fn x(v: f64) -> String {
    fmt_num(v + 0.0)
}

fn p3(p: Point3) -> String {
    format!("{} {} {}", n(p.x), n(p.y), n(p.z))
}

fn v3(v: Vec3) -> String {
    let r = |c: f64| fmt_num((c * 1000.0).round() / 1000.0 + 0.0);
    format!("{} {} {}", r(v.x), r(v.y), r(v.z))
}

fn xyz(p: Point3) -> String {
    format!("{} {} {}", x(p.x), x(p.y), x(p.z))
}

fn axis_dir(v: Vec3) -> String {
    format!("{} {} {}", x(v.x), x(v.y), x(v.z))
}

/// The first line: a block, a rounded or round plate, or a turned stepped shaft.
fn base(r: &mut Rng) -> String {
    let (w, d, t) = (r.range(40.0, 150.0), r.range(30.0, 120.0), r.range(8.0, 40.0));
    match r.below(10) {
        0 => format!("L0: extrude {} circle 0 0 {}", n(t), n(w.min(d) / 2.0)),
        1 => format!(
            "L0: extrude {} rrect 0 0 {} {} {}",
            n(t),
            n(w),
            n(d),
            n(r.range(1.0, 0.45 * w.min(d)))
        ),
        2 => {
            let (r0, r1) = (r.range(6.0, 20.0), r.range(10.0, 30.0));
            format!(
                "L0: revolve xz axis y poly 0 0 {} 0 {} {} {} {} {} {} 0 {}",
                n(r0),
                n(r0),
                n(t),
                n(r1),
                n(t),
                n(r1),
                n(t + w / 3.0),
                n(t + w / 3.0)
            )
        }
        _ => format!("L0: extrude {} rect 0 0 {} {}", n(t), n(w), n(d)),
    }
}

fn extrude(g: &mut G) -> Option<Vec<String>> {
    let mut out = Vec::new();
    // now and then another body
    let op = if g.one_in(15) { "new" } else { g.op() };
    let (at, nrm) = g.spot();
    let s = g.size(0.04, 0.3);
    let depth = g.size(0.03, 0.25);
    // a cut sketched on the face going in, an add sunk a little going out
    let (start, dir) = if op == "add" {
        (at - nrm * g.range(0.0, 1.0), nrm)
    } else {
        (at + nrm * g.range(0.0, 1.0), -nrm)
    };
    let z = g.tilt(dir);
    let ext = match g.r.below(12) {
        0 => format!("sym {}", n(depth)),
        1 => format!("two {} {}", n(depth), n(g.range(0.5, 3.0))),
        2 | 3 if op == "cut" => ["thru", "thru back", "thru both"][g.r.below(3)].to_string(),
        4 => ["next", "next back"][g.r.below(2)].to_string(),
        5 | 6 => g.upto(start, z, &mut out)?,
        _ => n(depth),
    };
    let frame = match g.planes().last() {
        Some(p) if g.one_in(8) => format!("on {} at {}", trim(p), p3(start)),
        _ => format!("z {} at {}", v3(z), p3(start)),
    };
    let l = g.label();
    out.push(format!("{l} extrude {op} {ext} {frame} {}", g.loops(s)));
    Some(out)
}

fn revolve(g: &mut G) -> Option<Vec<String>> {
    let mut out = Vec::new();
    let op = g.op();
    let (at, _) = g.spot();
    let rad = g.size(0.04, 0.2);
    let tube = rad * g.range(0.1, 0.5);
    // sketch (u along the axis, v out from it)
    let (u0, v0) = (g.range(-tube, tube), rad);
    let which = g.r.below(3);
    let axis = [" axis x", " axis 0 0 0 -1", ""][which];
    let place = |u: f64, v: f64| match which {
        0 => (u, v),
        1 => (v, -u),
        _ => (v, u),
    };
    let prof = match g.r.below(3) {
        0 => {
            let (cx, cy) = place(u0, v0);
            format!("circle {} {} {}", n(cx), n(cy), n(tube))
        }
        k => {
            let pts: Vec<(f64, f64)> = if k == 1 {
                vec![
                    (-tube, v0 - tube),
                    (tube, v0 - tube),
                    (tube, v0 + tube),
                    (-tube, v0 + tube),
                ]
            } else {
                vec![(-tube, v0 - tube), (tube, v0 - tube), (0.0, v0 + tube)]
            };
            let mut s = String::from("poly");
            for (u, v) in pts {
                let (a, b) = place(u, v);
                s.push_str(&format!(" {} {}", n(a), n(b)));
            }
            s
        }
    };
    let turn = match g.r.below(8) {
        0 => format!(" {}", n(g.range(30.0, 300.0))),
        1 => format!(" sym {}", n(g.range(20.0, 150.0))),
        2 => format!(" two {} {}", n(g.range(20.0, 150.0)), n(g.range(10.0, 90.0))),
        3 => {
            // a plane through the axis (the standing sketch's y), turned about it
            let a = g.range(0.3, 2.8);
            let (s, c) = dmath::sin_cos(a);
            let l = g.label();
            out.push(format!("{l} plane z {} {} 0 at {}", n(-s), n(c), p3(at)));
            format!(" to {}{}", trim(&l), if g.one_in(3) { " off 5" } else { "" })
        }
        _ => String::new(),
    };
    let lift = at.z + g.range(-tube, tube);
    let l = g.label();
    out.push(format!(
        "{l} revolve {op}{turn} xz at {} {} {}{axis} {prof}",
        n(at.x),
        n(at.y),
        n(lift)
    ));
    Some(out)
}

fn ball(g: &mut G) -> Option<Vec<String>> {
    let op = g.op();
    let (at, nrm) = g.spot();
    let r = g.size(0.02, 0.15);
    let c = at + nrm * (r * g.range(-0.8, 0.8));
    let l = g.label();
    Some(vec![format!("{l} ball {op} {} at {}", n(r), p3(c))])
}

/// A fillet radius law no bigger than `most`.
fn radius(g: &mut G, most: f64) -> String {
    let r = |g: &mut G| n((most * g.range(0.1, 1.0)).max(0.01));
    match g.r.below(6) {
        0 => format!("{}..{}", r(g), r(g)),
        1 => format!("0:{},{}:{},1:{}", r(g), n(g.range(0.2, 0.8)), r(g), r(g)),
        _ => r(g),
    }
}

fn fillet(g: &mut G) -> Option<Vec<String>> {
    let (edges, len) = g.edges(3)?;
    let most = (len * 0.4).min(g.bx.diagonal() * 0.05);
    let mut line = format!("{} fillet {}", g.label(), radius(g, most));
    // a law's start, or a corner's setback, at a vertex of the part
    let verts = select::verts(g.body);
    let vert = |g: &mut G| -> Option<String> {
        let v = g.pick(&verts)?;
        let p = g.body.vertex(v).point;
        Some(format!("(verts near {})", p3(p)))
    };
    if g.one_in(6)
        && let Some(v) = vert(g)
    {
        line.push_str(&format!(" from {v}"));
    }
    for (k, e) in edges.iter().enumerate() {
        if k > 0 && g.one_in(3) {
            line.push_str(&format!(" {}", radius(g, most)));
        }
        line.push_str(&format!(" {e}"));
    }
    if g.one_in(6)
        && let Some(v) = vert(g)
    {
        line.push_str(&format!(" setback {v} {}", n(most * g.range(0.5, 2.0))));
    }
    Some(vec![line])
}

fn chamfer(g: &mut G) -> Option<Vec<String>> {
    // chamfers run along lines and circles
    let creases: Vec<_> = g
        .creases()
        .into_iter()
        .filter(|&e| select::curve_kind(&g.body.edge(e).curve) != "curve")
        .collect();
    let e = g.pick(&creases)?;
    let len = select::length(g.body, e);
    let d = |g: &mut G| n((len * 0.3).min(g.bx.diagonal() * 0.04) * g.range(0.1, 1.0) + 0.01);
    let first = g.sel(Ent::Edge(e), "")?;
    let size = match g.r.below(4) {
        k @ (0 | 1) => {
            // the face the first distance is measured on: one of the edge's own
            let fs = select::faces_of_edge(g.body, e);
            let f = g.pick(&fs)?;
            let face = g.sel(Ent::Face(f), "")?;
            if k == 0 {
                format!("{} {} face {face}", d(g), d(g))
            } else {
                format!("{} ang {} face {face}", d(g), n(g.range(20.0, 70.0)))
            }
        }
        _ => d(g),
    };
    let mut line = format!("{} chamfer {size} {first}", g.label());
    if g.one_in(2)
        && let Some((more, _)) = g.edges(2)
    {
        for m in more {
            line.push_str(&format!(" {m}"));
        }
    }
    Some(vec![line])
}

fn shell(g: &mut G) -> Option<Vec<String>> {
    let t = g.size(0.003, 0.02);
    let out = if g.one_in(4) { " out" } else { "" };
    let mut line = format!("{} shell {}{out}", g.label(), n(t + 0.1));
    for _ in 0..g.r.below(3) {
        if g.one_in(2) {
            line.push_str(" (faces plane normal +z top)");
        } else if let Some((_, s)) = g.face(true, |_| true) {
            line.push_str(&format!(" {s}"));
        }
    }
    Some(vec![line])
}

fn draft(g: &mut G) -> Option<Vec<String>> {
    let a = g.range(1.0, 8.0) * if g.one_in(2) { -1.0 } else { 1.0 };
    let z = g.range(g.bx.z.lo, g.bx.z.hi);
    let mut line = format!("{} draft {} xy at 0 0 {}", g.label(), n(a), n(z));
    let mut any = false;
    for _ in 0..1 + g.r.below(4) {
        // walls: planes square to the pull, or round walls along it
        let wall = |nm: Option<Vec3>| nm.is_none_or(|v| v.z.abs() < 1e-9);
        let planar = !g.one_in(4);
        if let Some((_, s)) = g.face(planar, wall)
            && !line.contains(&s)
        {
            line.push_str(&format!(" {s}"));
            any = true;
        }
    }
    any.then_some(vec![line])
}

fn hole(g: &mut G) -> Option<Vec<String>> {
    let mut out = Vec::new();
    let (at, nrm) = g.spot();
    let d = g.size(0.01, 0.08) + 0.5;
    let head = match g.r.below(4) {
        0 => format!(" cb {} {}", n(d * g.range(1.4, 2.0)), n(d * g.range(0.3, 1.0))),
        1 => format!(
            " cs {} {}",
            n(d * g.range(1.4, 2.2)),
            ["90", "82", "100", "120"][g.r.below(4)]
        ),
        _ => String::new(),
    };
    let end = match g.r.below(6) {
        0 | 1 => "thru".to_string(),
        2 => "next".to_string(),
        3 => g.upto(at, -nrm, &mut out)?,
        _ => n(g.size(0.03, 0.2)),
    };
    let pt = if g.one_in(3) { " pt 118" } else { "" };
    let thread = if g.one_in(4) {
        format!(" thread {} {}", n(d * 1.2), n(d * 0.15 + 0.2))
    } else {
        String::new()
    };
    // a standard size, its fit and seat, now and then
    let size = if g.one_in(3) {
        let m = ["M3", "M4", "M5", "M6", "M8", "M10", "M8x1", "1/4-20", "10-32"][g.r.below(9)];
        let std = ["", " iso", " ansi"][g.r.below(3)];
        let fit = ["", " close", " normal", " loose", " tap"][g.r.below(5)];
        let seat = ["", "", " cbore", " csink"][g.r.below(4)];
        format!("{m}{std}{fit}{seat}")
    } else {
        n(d)
    };
    let (head, thread) = if size.starts_with(['M', '1']) {
        (String::new(), String::new())
    } else {
        (head, thread)
    };
    let l = g.label();
    out.push(format!(
        "{l} hole {size} z {} at {}{head} {end}{pt}{thread}",
        v3(-nrm),
        p3(at)
    ));
    Some(out)
}

fn pattern(g: &mut G) -> Option<Vec<String>> {
    let src = if g.one_in(4) {
        "body".to_string()
    } else {
        let ls = g.solids();
        let mut s = trim(&g.pick(&ls[1.min(ls.len())..])?).to_string();
        if g.one_in(3)
            && let Some(o) = g.pick(&ls[1.min(ls.len())..])
            && trim(&o) != s
        {
            s = format!("{s} {}", trim(&o));
        }
        s
    };
    let c = g.bx.center();
    let step = g.size(0.05, 0.3);
    // copies of the whole part are many n-ary booleans at once: a few
    let most = if src == "body" { 1 } else { 3 };
    let axes = g.labels(|f| matches!(f, Feature::Work(forge_model::Work::Axis(_))));
    let points = g.labels(|f| matches!(f, Feature::Work(forge_model::Work::Point(_))));
    let layout = match g.r.below(5) {
        3 => {
            let a = g.pick(&axes)?;
            let how = if g.one_in(2) { "fit".to_string() } else { n(step) };
            let opts = format!(
                "{}{}",
                if g.one_in(3) { " fixed" } else { "" },
                if g.one_in(3) { " rev" } else { "" }
            );
            format!("path {} {} {how}{opts}", trim(&a), 2 + g.r.below(3))
        }
        4 => {
            let mut l = String::from("pts");
            for _ in 0..1 + g.r.below(3) {
                match g.pick(&points) {
                    Some(p) if g.one_in(2) => l.push_str(&format!(" {}", trim(&p))),
                    _ => {
                        let q = Point3::new(
                            g.range(g.bx.x.lo, g.bx.x.hi),
                            g.range(g.bx.y.lo, g.bx.y.hi),
                            g.range(g.bx.z.lo, g.bx.z.hi),
                        );
                        l.push_str(&format!(" {}", p3(q)));
                    }
                }
            }
            l
        }
        0 => {
            let a = g.range(0.0, dmath::TAU);
            let (s, co) = dmath::sin_cos(a);
            let mut l = format!("lin {} {} 0 {} {}", n(co), n(s), 2 + g.r.below(most), n(step));
            if most > 1 && g.one_in(3) {
                l.push_str(&format!(
                    " {} {} 0 {} {}",
                    n(-s),
                    n(co),
                    2 + g.r.below(2),
                    n(step)
                ));
            }
            l
        }
        1 => {
            let mut l = format!(
                "circ {} {} 0 0 0 1 {}",
                n(c.x + g.range(-step, step)),
                n(c.y + g.range(-step, step)),
                2 + g.r.below(2 * most)
            );
            match g.r.below(3) {
                0 => l.push_str(&format!(" {}", n(g.range(30.0, 300.0)))),
                1 => l.push_str(&format!(" {} step", n(g.range(10.0, 90.0)))),
                _ => {}
            }
            l
        }
        _ => {
            let a = g.range(0.0, dmath::PI);
            let (s, co) = dmath::sin_cos(a);
            let off = g.range(-0.6, 0.6) * g.bx.diagonal() / 2.0;
            let o = Point3::new(c.x + co * off, c.y + s * off, 0.0);
            format!("mirror z {} {} 0 at {}", n(co), n(s), p3(o))
        }
    };
    Some(vec![format!("{} pattern {src} {layout}", g.label())])
}

/// A path from `a` along unit `t`: a straight run, a bend of radius `rad` towards unit `nrm`
/// through `turn` degrees, and a run out; and where it ends.
fn bent_path(a: Point3, t: Vec3, nrm: Vec3, l1: f64, rad: f64, turn: f64, l2: f64) -> String {
    let b = a + t * l1;
    let c = b + nrm * rad;
    let z = (-nrm).cross(t);
    let (s, co) = dmath::sin_cos(turn / 180.0 * dmath::PI);
    let e = c + (-nrm) * (rad * co) + t * (rad * s);
    let te = nrm * s + t * co;
    format!(
        "line {} {} arc z {} x {} at {} {} 0 {} line {} {}",
        xyz(a),
        xyz(b),
        axis_dir(z),
        axis_dir(-nrm),
        xyz(c),
        x(rad),
        x(turn),
        xyz(e),
        xyz(e + te * l2)
    )
}

fn sweep(g: &mut G) -> Option<Vec<String>> {
    let mut out = Vec::new();
    let op = g.op();
    let (at, nrm) = g.spot();
    let r = g.size(0.01, 0.04);
    // along the face, half sunk, or diving into it
    let dirs = [Vec3::X, -Vec3::X, Vec3::Y, -Vec3::Y, Vec3::Z, -Vec3::Z];
    let t = g.pick(&dirs)?;
    let bends: Vec<Vec3> = dirs.iter().copied().filter(|d| d.dot(t) == 0.0).collect();
    let b = g.pick(&bends)?;
    let a = rp(at - t * g.size(0.05, 0.2) + nrm * (r * 0.5));
    let rad = rd(r * g.range(1.5, 5.0));
    let turn = [90.0, 45.0, 60.0, 120.0, 30.0][g.r.below(5)];
    let (l1, l2) = (rd(g.size(0.05, 0.3)), rd(g.size(0.0, 0.3)).max(0.5));
    let path = bent_path(a, t, b, l1, rad, turn, l2);
    let s = 2.0 * r;
    let prof = match g.r.below(3) {
        0 => format!("rect {} {} {} {}", n(-r), n(-r), n(s), n(s)),
        1 => format!("rrect {} {} {} {} {}", n(-r), n(-r), n(s), n(s), n(r * 0.3)),
        _ => format!("circle 0 0 {}", n(r)),
    };
    let ends = match g.r.below(4) {
        0 => format!(" extend {} {}", n(g.range(0.0, 5.0)), n(g.range(0.0, 5.0))),
        1 => {
            let up = g.upto(a + t * l1, t, &mut out)?;
            // ends on a face are for the far end only: a plane past the bend
            if up.contains("(faces") {
                String::new()
            } else {
                format!(" {up}")
            }
        }
        _ => String::new(),
    };
    let l = g.label();
    out.push(format!(
        "{l} sweep {op} z {} x {} at {} {prof} path {path}{ends}",
        axis_dir(t),
        axis_dir(b),
        xyz(a)
    ));
    Some(out)
}

fn coil(g: &mut G) -> Option<Vec<String>> {
    let mut out = Vec::new();
    let op = g.op();
    let (at, _) = g.spot();
    let rad = g.size(0.03, 0.15);
    let r = rad * g.range(0.08, 0.25);
    let prof = if g.one_in(2) {
        format!("circle {} 0 {}", n(rad), n(r))
    } else {
        format!("rect {} {} {} {}", n(rad - r), n(-r), n(2.0 * r), n(2.0 * r))
    };
    let pitch = 2.0 * r * g.range(1.3, 3.0);
    let turns = g.range(1.0, 5.0);
    let down = g.one_in(3);
    let size = match g.r.below(4) {
        0 => format!("pitch {} turns {}", n(pitch), n(turns)),
        1 => format!("turns {} height {}", n(turns), n(pitch * turns)),
        2 => format!("pitch {} height {}", n(pitch), n(pitch * turns)),
        _ => {
            let dir = if down { -Vec3::Z } else { Vec3::Z };
            let up = g.upto(at, dir, &mut out)?;
            format!("pitch {} {up}", n(pitch))
        }
    };
    let taper = if g.one_in(4) {
        format!(" taper {}", n(g.range(-10.0, 10.0)))
    } else {
        String::new()
    };
    let left = if g.one_in(3) { " left" } else { "" };
    let axis = if down { " axis 0 0 0 -1" } else { "" };
    let z0 = at.z + g.range(-2.0 * r, 2.0 * r);
    let l = g.label();
    out.push(format!(
        "{l} coil {op} xz at {} {} {}{axis} {prof} {size}{taper}{left}",
        n(at.x),
        n(at.y),
        n(z0)
    ));
    Some(out)
}

fn loft(g: &mut G) -> Option<Vec<String>> {
    let op = g.op();
    let (at, _) = g.spot();
    let k = 2 + g.r.below(2);
    let round = g.one_in(3);
    let smooth = k == 3 || g.one_in(2);
    let mut z = at.z - g.range(0.0, 2.0);
    let mut line = format!("{} loft {op}", g.label());
    if smooth {
        line.push_str(" smooth");
        if g.one_in(3) {
            line.push_str(&format!(" start {}", n(g.range(0.2, 2.0))));
        }
        if g.one_in(3) {
            line.push_str(&format!(" end {}", n(g.range(0.2, 2.0))));
        }
    }
    let mut corners = Vec::new();
    for i in 0..k {
        let s = g.size(0.03, 0.15);
        let twist = !smooth && g.one_in(3);
        let (sa, ca) = if twist {
            dmath::sin_cos(g.range(-0.6, 0.6))
        } else {
            (0.0, 1.0)
        };
        let frame = if twist {
            format!(
                "z 0 0 1 x {} {} 0 at {} {} {}",
                n(ca),
                n(sa),
                n(at.x),
                n(at.y),
                n(z)
            )
        } else {
            format!("xy at {} {} {}", n(at.x), n(at.y), n(z))
        };
        if round {
            line.push_str(&format!(" profile {frame} circle 0 0 {}", n(s / 2.0)));
        } else {
            let h = rd(s / 2.0);
            line.push_str(&format!(
                " profile {frame} rect {} {} {} {}",
                x(-h),
                x(-h),
                x(2.0 * h),
                x(2.0 * h)
            ));
            corners.push(Point3::new(rd(at.x) - h, rd(at.y) - h, rd(z)));
        }
        if i + 1 < k {
            z += g.size(0.05, 0.2);
        }
    }
    if smooth && k == 2 && !round && g.one_in(2) {
        // a straight rail through each section's first corner
        line.push_str(&format!(" rail 0 line {} {}", xyz(corners[0]), xyz(corners[1])));
    } else if smooth && g.one_in(3) {
        let a = rp(Point3::new(at.x, at.y, at.z - 3.0));
        line.push_str(&format!(
            " centre line {} {}",
            xyz(a),
            xyz(Point3::new(a.x, a.y, rd(z + 3.0)))
        ));
    }
    Some(vec![line])
}

fn plane(g: &mut G) -> Option<Vec<String>> {
    let c = g.bx.center();
    let z = g.tilt(Vec3::Z);
    let h = match g.r.below(3) {
        0 => g.bx.z.hi + g.size(0.02, 0.3),
        1 => g.bx.z.lo - g.size(0.02, 0.3),
        _ => g.range(g.bx.z.lo, g.bx.z.hi),
    };
    Some(vec![format!(
        "{} plane z {} at {}",
        g.label(),
        v3(z),
        p3(Point3::new(c.x, c.y, h))
    )])
}

fn facefillet(g: &mut G) -> Option<Vec<String>> {
    // the two faces of a crease, or the faces either side of a planar strip
    let creases = g.creases();
    let e = g.pick(&creases)?;
    let fs = select::faces_of_edge(g.body, e);
    let (a, b) = (
        g.sel(Ent::Face(*fs.first()?), "")?,
        g.sel(Ent::Face(*fs.get(1)?), "")?,
    );
    let r = (select::length(g.body, e) * 0.2).min(g.bx.diagonal() * 0.03) * g.range(0.1, 1.0);
    Some(vec![format!(
        "{} facefillet {} {a} to {b}",
        g.label(),
        n(r.max(0.05))
    )])
}

fn fullround(g: &mut G) -> Option<Vec<String>> {
    // a planar face between two planar neighbours facing apart, square to it
    let b = g.body;
    let planar: Vec<FaceId> = b
        .face_ids()
        .filter(|&f| select::plane_normal(b, f).is_some())
        .collect();
    let mut found = Vec::new();
    for &m in &planar {
        let nm = select::plane_normal(b, m)?;
        let near: Vec<FaceId> = select::edges_of_face(b, m)
            .into_iter()
            .flat_map(|e| select::faces_of_edge(b, e))
            .filter(|&f| f != m && select::plane_normal(b, f).is_some_and(|n| n.dot(nm).abs() < 1e-9))
            .collect();
        for &s1 in &near {
            for &s2 in &near {
                let (n1, n2) = (select::plane_normal(b, s1)?, select::plane_normal(b, s2)?);
                if n1.dot(n2) < -1.0 + 1e-9 {
                    found.push((s1, m, s2));
                }
            }
        }
    }
    let (s1, m, s2) = g.pick(&found)?;
    let (a, mid, c) = (
        g.sel(Ent::Face(s1), "")?,
        g.sel(Ent::Face(m), "")?,
        g.sel(Ent::Face(s2), "")?,
    );
    Some(vec![format!("{} fullround {a} {mid} {c}", g.label())])
}

fn param(g: &mut G) -> Option<Vec<String>> {
    let k =
        g.m.features
            .iter()
            .filter(|f| matches!(f, Feature::Param(_)))
            .count();
    let v = g.size(0.02, 0.1);
    let expr = match (k, g.r.below(3)) {
        (0, _) | (_, 0) => n(v),
        (_, 1) => format!("len{} * {} + 1", k - 1, n(g.range(0.5, 1.5))),
        _ => format!("max(len{}, {}) / 2", k - 1, n(v)),
    };
    let unit = if g.one_in(3) { " mm" } else { "" };
    Some(vec![format!("len{k} = {expr}{unit}")])
}

/// A dimension: a number, or now and then an earlier parameter (sized as numbers here are).
fn dim(g: &mut G, v: f64) -> String {
    let ps: Vec<String> =
        g.m.features
            .iter()
            .filter_map(|f| match f {
                Feature::Param(p) if p.value > 0.0 => Some(p.name.clone()),
                _ => None,
            })
            .collect();
    match g.pick(&ps) {
        Some(p) if g.one_in(3) => format!("{p}*{}", n(v / g.bx.diagonal() * 20.0 + 0.1)),
        _ => n(v),
    }
}

fn sketch(g: &mut G) -> Option<Vec<String>> {
    let (at, nrm) = g.spot();
    let s = g.size(0.04, 0.2);
    let l = g.label();
    let place = match g.face(true, |_| true) {
        Some((_, f)) if g.one_in(3) => format!("on {f}"),
        _ => format!("z {} at {}", v3(nrm), p3(at)),
    };
    let w = dim(g, s);
    let mut out = vec![
        format!("{l} sketch {place}"),
        format!("  rect {} {} {} {}", n(-s / 2.0), n(-s / 3.0), n(s), n(s * 0.66)),
    ];
    if g.one_in(2) {
        out.push(format!("  circle 0 0 {}", n((s * 0.15).max(0.1))));
        out.push(format!("  dia c1 {}", n((s * g.range(0.1, 0.3)).max(0.1))));
    }
    out.push(format!("  dist l1 {w}"));
    out.push("end".into());
    let op = g.op();
    let depth = g.size(0.02, 0.15);
    let d = dim(g, depth);
    let next = g.label();
    out.push(format!("{next} extrude {op} {d} {}", trim(&l)));
    Some(out)
}

fn rib(g: &mut G) -> Option<Vec<String>> {
    // a gusset across a concave straight edge: in the plane square to it, from one face to the
    // other, filling the corner between
    let b = g.body;
    let edges: Vec<forge_topo::EdgeId> = g
        .creases()
        .into_iter()
        .filter(|&e| {
            select::crease(b, e) == Some("concave") && select::curve_kind(&b.edge(e).curve) == "line"
        })
        .collect();
    let e = g.pick(&edges)?;
    let fs = select::faces_of_edge(b, e);
    let (na, nb) = (
        select::plane_normal(b, *fs.first()?)?,
        select::plane_normal(b, *fs.get(1)?)?,
    );
    let m = select::centre(b, Ent::Edge(e));
    let dir = b.edge(e).curve.deriv(b.edge(e).range.mid()).d1.normalize()?.get();
    let x = (nb - dir * nb.dot(dir)).normalize()?.get();
    let y = dir.cross(x);
    let reach = (select::length(b, e) * 0.5).min(g.size(0.03, 0.15));
    let (p, q) = ((reach, 0.0), (reach * na.dot(x), reach * na.dot(y)));
    // the corner (the sketch origin) on the path's left
    let left = (q.0 - p.0) * (0.0 - p.1) - (q.1 - p.1) * (0.0 - p.0) > 0.0;
    let (p, q) = if left { (p, q) } else { (q, p) };
    let thick = (g.size(0.005, 0.02) + 0.2).min(select::length(b, e) * 0.3);
    let draft = if g.one_in(4) {
        format!(" draft {}", n(g.range(0.5, 5.0)))
    } else {
        String::new()
    };
    let side = ["", "", " pos", " neg"][g.r.below(4)];
    Some(vec![format!(
        "{} rib {}{side}{draft} z {} x {} at {} line {} {} {} {}",
        g.label(),
        n(thick),
        axis_dir(dir),
        axis_dir(x),
        xyz(m),
        x_(p.0),
        x_(p.1),
        x_(q.0),
        x_(q.1)
    )])
}

/// A derived sketch coordinate, to a thousandth.
fn x_(v: f64) -> String {
    fmt_num((v * 1000.0).round() / 1000.0 + 0.0)
}

fn offset(g: &mut G) -> Option<Vec<String>> {
    let planar = !g.one_in(4);
    let (_, f) = g.face(planar, |_| true)?;
    let d = g.size(0.002, 0.02) * if g.one_in(2) { -1.0 } else { 1.0 };
    Some(vec![format!("{} offset {} {f}", g.label(), n(d))])
}

fn thicken(g: &mut G) -> Option<Vec<String>> {
    let planar = !g.one_in(4);
    let (_, f) = g.face(planar, |_| true)?;
    let op = ["add", "cut", "add", "new"][g.r.below(4)];
    let side = [" neg", " sym", "", ""][g.r.below(4)];
    let t = g.size(0.005, 0.04) + 0.2;
    Some(vec![format!("{} thicken {op} {}{side} {f}", g.label(), n(t))])
}

fn delete(g: &mut G) -> Option<Vec<String>> {
    // a hole, boss or pocket made by one earlier line, or a blend strip
    let ls = g.solids();
    let l = g.pick(&ls[1.min(ls.len())..])?;
    let s = if g.one_in(3) {
        let blends: Vec<FaceId> = g
            .body
            .face_ids()
            .filter(|&f| select::surface_kind(&g.body.face(f).surface) == "blend")
            .collect();
        let f = g.pick(&blends)?;
        g.sel(Ent::Face(f), " blend")?
    } else {
        let s = format!("(faces of {})", trim(&l));
        if !g.found(&s, 40) {
            return None;
        }
        s
    };
    Some(vec![format!("{} delete {s}", g.label())])
}

fn replace(g: &mut G) -> Option<Vec<String>> {
    // a planar face put on a work plane near and almost parallel to it
    let (f, s) = g.face(true, |_| true)?;
    let nrm = select::plane_normal(g.body, f)?;
    let at = select::centre(g.body, Ent::Face(f)) + nrm * g.size(-0.03, 0.03);
    let z = g.tilt(nrm);
    let l = g.label();
    let plane = format!("{l} plane z {} at {}", v3(z), p3(at));
    Some(vec![
        plane,
        format!("{} replace {s} with {}", g.label(), trim(&l)),
    ])
}

fn move_faces(g: &mut G) -> Option<Vec<String>> {
    let ls = g.solids();
    let l = g.pick(&ls[1.min(ls.len())..])?;
    let s = format!("(faces of {})", trim(&l));
    if !g.found(&s, 40) {
        return None;
    }
    let d = g.size(0.005, 0.05);
    let how = if g.one_in(3) {
        let c = g.bx.center();
        format!("turn {} {} 0 0 1", n(g.range(-5.0, 5.0)), p3(c))
    } else {
        format!(
            "by {} {} {}",
            n(g.range(-d, d)),
            n(g.range(-d, d)),
            n(g.range(-d, d) * 0.3)
        )
    };
    Some(vec![format!("{} move {s} {how}", g.label())])
}

/// A face, edge or vertex an earlier line made, as `FEAT (selector)`: `what` the kind words.
fn made(g: &mut G, all: &str, what: &str) -> Option<String> {
    let ls = g.solids();
    let l = g.pick(&ls)?;
    let of = if all == "verts" {
        String::new()
    } else {
        format!(" of {}", trim(&l))
    };
    let s = format!("({all} {what}{of})");
    let (_, hits) = g.m.select(&s, None).ok()?;
    let e = g.pick(&hits)?;
    let near = format!("({all} {what}{of} near {})", p3(select::centre(g.body, e)));
    let (_, hits) = g.m.select(&near, None).ok()?;
    (hits == [e]).then(|| format!("{} {near}", trim(&l)))
}

fn axis(g: &mut G) -> Option<Vec<String>> {
    let by = match g.r.below(3) {
        0 => made(g, "edges", "line")?,
        1 => made(g, "faces", "cyl")?,
        _ => format!("pts {} {}", made(g, "verts", "")?, made(g, "verts", "")?),
    };
    Some(vec![format!("{} axis {by}", g.label())])
}

fn point(g: &mut G) -> Option<Vec<String>> {
    let by = match g.r.below(3) {
        0 => made(g, "verts", "")?,
        1 => made(g, "edges", "circle")?,
        _ => format!("mid {}", made(g, "edges", "line")?),
    };
    Some(vec![format!("{} point {by}", g.label())])
}

fn ucs(g: &mut G) -> Option<Vec<String>> {
    let (at, nrm) = g.spot();
    let z = g.tilt(nrm);
    Some(vec![format!("{} ucs z {} at {}", g.label(), v3(z), p3(at))])
}

/// A cutting plane through the part, tipped a little.
fn cutter(g: &mut G) -> String {
    let c = g.bx.center();
    let axis = [Vec3::X, Vec3::Y, Vec3::Z][g.r.below(3)];
    let z = g.tilt(axis);
    let at = c + z * g.size(-0.2, 0.2);
    format!("z {} at {}", v3(z), p3(at))
}

/// A rigid motion: moved, turned about a line, or both.
fn motion(g: &mut G) -> String {
    let d = g.size(0.05, 0.6);
    let by = format!(
        "by {} {} {}",
        n(g.range(-d, d)),
        n(g.range(-d, d)),
        n(g.range(-d, d) * 0.3)
    );
    let c = g.bx.center();
    let rot = format!("rot {} 0 0 1 {}", p3(c), n(g.range(-180.0, 180.0)));
    match g.r.below(3) {
        0 => by,
        1 => rot,
        _ => format!("{by} {rot}"),
    }
}

/// Labels of the lines that made bodies (the first line, a later `new`, a split or a copy).
fn bodies(g: &G) -> Vec<String> {
    let made = |k: &str, f: &Feature| {
        matches!(k, "split" | "copybody")
            || matches!(
                f,
                Feature::Extrude {
                    op: forge_model::Combine::New,
                    ..
                } | Feature::Revolve {
                    op: forge_model::Combine::New,
                    ..
                }
            )
    };
    g.m.kinds
        .iter()
        .zip(&g.m.features)
        .zip(&g.m.labels)
        .filter(|((k, f), _)| made(k, f))
        .filter_map(|(_, l)| l.as_ref().map(|l| trim(l).to_string()))
        .collect()
}

fn combine(g: &mut G) -> Option<Vec<String>> {
    let bs = bodies(g);
    let (t, u) = (g.pick(&bs)?, g.pick(&bs)?);
    if t == u {
        return None;
    }
    let op = ["add", "cut", "keep"][g.r.below(3)];
    let keep = if g.one_in(3) { " keep" } else { "" };
    Some(vec![format!("{} combine {op} {t} {u}{keep}", g.label())])
}

fn split(g: &mut G) -> Option<Vec<String>> {
    let side = ["", " +", " -"][g.r.below(3)];
    let by = format!("plane {}", cutter(g));
    Some(vec![format!("{} split {by}{side}", g.label())])
}

fn splitface(g: &mut G) -> Option<Vec<String>> {
    let (f, s) = g.face(true, |_| true)?;
    let by = if g.one_in(2) {
        format!("plane {}", cutter(g))
    } else {
        // a line across the face, drawn on its plane
        let nrm = select::plane_normal(g.body, f)?;
        let at = select::centre(g.body, Ent::Face(f));
        let r = g.size(0.02, 0.2);
        format!("curve z {} at {} line {} 0 {} 0", v3(nrm), p3(at), n(-r), n(r))
    };
    Some(vec![format!("{} splitface {s} by {by}", g.label())])
}

fn movebody(g: &mut G) -> Option<Vec<String>> {
    Some(vec![format!("{} movebody {}", g.label(), motion(g))])
}

fn copybody(g: &mut G) -> Option<Vec<String>> {
    Some(vec![format!("{} copybody {}", g.label(), motion(g))])
}

fn thread(g: &mut G) -> Option<Vec<String>> {
    let end = match g.r.below(3) {
        0 => "full".to_string(),
        _ => n(g.size(0.03, 0.2)),
    };
    let opts = format!(
        "{}{}{}",
        if g.one_in(4) { " left" } else { "" },
        if g.one_in(3) { " chamfer" } else { "" },
        if g.one_in(4) { " runout" } else { "" }
    );
    // the ISO coarse size nearest, now and then a fine pitch
    let fine = g.one_in(4);
    let spec = |d: f64| {
        let sizes = [
            2.0, 2.5, 3.0, 4.0, 5.0, 6.0, 8.0, 10.0, 12.0, 16.0, 20.0, 24.0, 30.0,
        ];
        let m = sizes
            .iter()
            .copied()
            .min_by(|a, b| (a - d).abs().total_cmp(&(b - d).abs()))
            .unwrap_or(6.0);
        if fine {
            format!("M{}x{}", fmt_num(m), fmt_num(rd(m * 0.1)))
        } else {
            format!("M{}", fmt_num(m))
        }
    };
    let line = match g.r.below(3) {
        // the thread of a hole drilled with one
        0 => {
            let holes = g.labels(|f| matches!(f, Feature::Hole(h) if h.thread.is_some()));
            let h = g.pick(&holes)?;
            format!("thread hole {} {end}{opts}", trim(&h))
        }
        // round a cylinder face of an earlier line
        1 => {
            let ls = g.solids();
            let l = g.pick(&ls)?;
            let s = format!("(faces cyl of {})", trim(&l));
            let (_, hits) = g.m.select(&s, None).ok()?;
            let f = g.pick(&hits)?;
            let r = select::radius(g.body, f)?;
            let what = format!(" cyl of {}", trim(&l));
            let face = g.sel(f, &what)?;
            let flip = if g.one_in(2) { " flip" } else { "" };
            format!(
                "thread {} face {} {face}{flip} {end}{opts}",
                spec(2.0 * r),
                trim(&l)
            )
        }
        // into the part at a spot, inside or outside
        _ => {
            let (at, nrm) = g.spot();
            let d = g.size(0.02, 0.08) + 2.0;
            let side = if g.one_in(2) { "int" } else { "ext" };
            format!(
                "thread {} z {} at {} {side} {end}{opts}",
                spec(d),
                v3(-nrm),
                p3(at)
            )
        }
    };
    Some(vec![format!("{} {line}", g.label())])
}

fn text(g: &mut G) -> Option<Vec<String>> {
    let (at, nrm) = g.spot();
    let words = ["DO", "OD", "D", "ODD"];
    let w = words[g.r.below(words.len())];
    let size = g.size(0.03, 0.15) + 1.0;
    let depth = g.size(0.005, 0.03) + 0.2;
    let op = if g.one_in(2) { "add" } else { "cut" };
    let wrap = if g.one_in(4) {
        format!(" wrap {}", n(g.size(0.05, 0.3) + 5.0))
    } else {
        String::new()
    };
    Some(vec![format!(
        "{} text {op} \"{w}\" font \"{FONT}\" {} {} z {} at {}{wrap}",
        g.label(),
        n(size),
        n(depth),
        v3(nrm),
        p3(at)
    )])
}

/// The font text lines name: the test font the suite's text part writes.
const FONT: &str = "forgecad-test.ttf";

/// A model of `src` that finds the test font.
pub fn model(src: &str) -> Model {
    let mut m = Model::default();
    let font = forge_model::suite::text()
        .features
        .into_iter()
        .find_map(|f| match f {
            Feature::Text(t) => t.font.font,
            _ => None,
        });
    m.fonts = font.map(|f| {
        let find: Box<crate::model::Fonts> = Box::new(move |_| Ok(f.clone()));
        std::sync::Arc::from(find)
    });
    m.update(src);
    m
}

/// A history's result.
#[derive(Clone, Debug, PartialEq)]
pub enum Verdict {
    /// Valid, closed, and the n-ary regen agrees.
    Ok,
    /// A line refused with a clear error (its kind, the message).
    Refused(String),
    /// A kernel failure: what kind (stable across instances, for grouping).
    Fail(String),
}

/// A generated history: its lines, each tried line's fate, and each failure with the history
/// that shows it.
pub struct Run {
    /// How many times the finished part's n-ary regen took the sequential one for a reference it
    /// names otherwise.
    pub name_fallbacks: usize,
    pub src: String,
    /// Per tried line: its kind, and None if it built, else the refusal or failure.
    pub tried: Vec<(&'static str, Option<String>)>,
    pub fails: Vec<(String, String)>,
    /// Each refusal with the history it ends (the census of what the kernel will not build).
    pub refused: Vec<(String, String)>,
    /// The slowest line's seconds and the history it ends.
    pub slowest: (f64, String),
}

/// The message with names, handles and numbers blanked (to `*` and `9`), so one kind of
/// failure groups however many places it shows up.
fn kind_of_msg(msg: &str) -> String {
    let mut out = String::new();
    let mut cs = msg.chars().peekable();
    while let Some(c) = cs.next() {
        if c == '#' || c == '@' {
            while cs.peek().is_some_and(|d| d.is_ascii_hexdigit()) {
                cs.next();
            }
            out.push('*');
        } else if c.is_ascii_digit() || c == '-' && cs.peek().is_some_and(|d| d.is_ascii_digit()) {
            while cs
                .peek()
                .is_some_and(|d| d.is_ascii_digit() || *d == '.' || *d == 'e' || *d == '-')
            {
                cs.next();
            }
            out.push('9');
        } else {
            out.push(c);
        }
    }
    out.chars().take(100).collect()
}

/// A refusal names what is wrong with the request. A kernel bug says Internal (a boolean lost
/// its way) or Invalid (an operation made a bad body and caught it); a line the parser cannot
/// read is the generator's bug.
fn classify(kind: &str, msg: &str) -> Verdict {
    let bug = ["Internal", "Invalid(", "no closed solid"]
        .iter()
        .any(|w| msg.contains(w));
    let unread = [
        "expected",
        "unexpected",
        "unknown",
        "no feature",
        "too soon",
        "a word the file uses",
    ]
    .iter()
    .any(|w| msg.starts_with(w) || msg.contains(&format!(": {w}")));
    let what = format!("{kind}: {}", kind_of_msg(msg));
    if bug || unread {
        Verdict::Fail(what)
    } else {
        Verdict::Refused(what)
    }
}

fn panic_text(p: Box<dyn std::any::Any + Send>) -> String {
    let m = p
        .downcast_ref::<&str>()
        .map(|s| s.to_string())
        .or_else(|| p.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "?".into());
    kind_of_msg(&m)
}

fn bbox(b: &Body) -> Box3 {
    let mut fs = b.face_ids();
    let first = fs
        .next()
        .map(|f| b.face_box(f))
        .unwrap_or(Box3::from_point(Point3::ORIGIN));
    fs.fold(first, |a, f| a.hull(&b.face_box(f)))
}

/// Each body after `n` lines valid on its own (separate bodies may overlap).
fn valid_after(kind: &str, m: &Model, n: usize) -> Verdict {
    m.solids_after(n)
        .iter()
        .map(|s| valid(kind, &s.body))
        .find(|v| *v != Verdict::Ok)
        .unwrap_or(Verdict::Ok)
}

/// The checks after a history built: its part's validity at line `i` (kind `kind`).
fn valid(kind: &str, b: &Body) -> Verdict {
    match catch_unwind(AssertUnwindSafe(|| forge_topo::check(b))) {
        Err(p) => Verdict::Fail(format!("{kind}: check panics: {}", panic_text(p))),
        Ok(p) if !p.is_empty() => {
            let s = format!("{:?}", p[0]);
            Verdict::Fail(format!(
                "{kind}: invalid {}",
                s.split([' ', '{', '(']).next().unwrap_or("")
            ))
        }
        Ok(_) => Verdict::Ok,
    }
}

/// The finished part closes up and the n-ary regen makes the same solid.
fn finish(m: &Model) -> Verdict {
    if m.body().is_none() {
        return Verdict::Ok;
    }
    let q = Quality {
        chord: CHORD,
        ..Quality::default()
    };
    let measure = |b: &Body, path: &str| -> Result<(f64, f64), Verdict> {
        let b = b.compact();
        match catch_unwind(AssertUnwindSafe(|| tessellate(&b, &q))) {
            Err(p) => Err(Verdict::Fail(format!("{path} mesh: panic {}", panic_text(p)))),
            Ok(Err(e)) => Err(Verdict::Fail(format!(
                "{path} mesh: {}",
                kind_of_msg(&format!("{e:?}"))
            ))),
            Ok(Ok(t)) if !t.open_edges().is_empty() => Err(Verdict::Fail(format!("{path} mesh: open edges"))),
            Ok(Ok(t)) if t.volume() <= 0.0 => Err(Verdict::Fail(format!("{path} mesh: volume <= 0"))),
            Ok(Ok(t)) => Ok((t.volume(), t.area())),
        }
    };
    // every body, on both regens
    let sum = |bs: Vec<&Body>, path: &str| -> Result<(f64, f64), Verdict> {
        let mut t = (0.0, 0.0);
        for b in bs {
            let (v, a) = measure(b, path)?;
            t = (t.0 + v, t.1 + a);
        }
        Ok(t)
    };
    let seq = m.solids_after(m.built());
    let (v, a) = match sum(seq.iter().map(|s| &s.body).collect(), "seq") {
        Ok(x) => x,
        Err(f) => return f,
    };
    let part = Part {
        features: m.features.clone(),
    };
    let nary = match catch_unwind(AssertUnwindSafe(|| part.regenerate_nary_solids(1))) {
        Err(p) => return Verdict::Fail(format!("nary: panic {}", panic_text(p))),
        Ok(Err(e)) => {
            return Verdict::Fail(format!("nary: {}", kind_of_msg(&crate::model::explain(&e).0)));
        }
        Ok(Ok(b)) => b,
    };
    if nary.len() != seq.len() {
        return Verdict::Fail(format!("nary: {} bodies, sequential {}", nary.len(), seq.len()));
    }
    for s in &nary {
        if let f @ Verdict::Fail(_) = valid("nary", &s.body) {
            return f;
        }
    }
    match sum(nary.iter().map(|s| &s.body).collect(), "nary") {
        Err(f) => f,
        // both meshes sag inside curved faces by under a chord: they differ by less than that
        Ok((vn, _)) if (vn - v).abs() > CHORD * a + 1e-9 * v.abs() => {
            Verdict::Fail("nary: volume differs".into())
        }
        Ok(_) => Verdict::Ok,
    }
}

/// Regenerates a part file line by line, checking every stage, then the finished part.
pub fn judge(src: &str) -> Verdict {
    judge_model(src).0
}

/// `judge`, and the model it built (none if it panicked).
pub fn judge_model(src: &str) -> (Verdict, Option<Model>) {
    let run = catch_unwind(AssertUnwindSafe(|| {
        let m = model(src);
        for i in 0..m.built() {
            let kind = m.kinds[i];
            if let f @ Verdict::Fail(_) = valid_after(kind, &m, i + 1) {
                return (f, m);
            }
        }
        if let Some(f) = &m.error {
            let kind = m.lines[f.at].toks[0].clone();
            let v = classify(&kind, &f.msg);
            return (v, m);
        }
        let v = finish(&m);
        (v, m)
    }));
    match run {
        Ok((v, m)) => (v, Some(m)),
        Err(p) => (Verdict::Fail(format!("panic: {}", panic_text(p))), None),
    }
}

/// Grows a history of up to `len` lines from `seed`. A line the kernel refuses, or fails on, is
/// dropped and the history goes on from the part before it.
pub fn run(seed: u64, len: usize) -> Run {
    run_kinds(seed, len, u64::MAX)
}

/// `run` drawing only the kinds whose bit in `kinds` is set (of MAKERS, in order; empty counts as all).
/// A history with few kinds piles them on each other, which all kinds at once rarely do (swarm
/// testing: Groce et al., ISSTA 2012).
pub fn run_kinds(seed: u64, len: usize, kinds: u64) -> Run {
    let kinds = if kinds == 0 { u64::MAX } else { kinds };
    let makers: Vec<&Maker> = MAKERS
        .iter()
        .enumerate()
        .filter(|(i, _)| kinds >> (i % 64) & 1 == 1)
        .map(|(_, m)| m)
        .collect();
    let mut r = Rng::new(seed.wrapping_mul(0x2545_f491_4f6c_dd1d).wrapping_add(0x7e57));
    let mut src = base(&mut r) + "\n";
    let mut m = model(&src);
    let mut tried = Vec::new();
    let mut fails = Vec::new();
    let mut refused = Vec::new();
    let mut slowest = (0.0, String::new());
    let mut label = 0;
    let total: usize = makers.iter().map(|k| k.weight).sum();
    let mut built = 0;
    for _ in 0..len * 3 {
        if built >= len {
            break;
        }
        // the part as the selectors see it (ids into the uncompacted stage)
        let Some(body) = m.body() else { break };
        // nothing left to place features on
        let d = bbox(body).diagonal();
        if d.is_nan() || d <= 1e-6 {
            break;
        }
        let mut pick = r.below(total);
        let maker = makers
            .iter()
            .find(|k| {
                let hit = pick < k.weight;
                pick = pick.saturating_sub(k.weight);
                hit
            })
            .expect("weights cover the draw");
        let lines = {
            let mut g = G {
                r: &mut r,
                m: &m,
                bx: bbox(body),
                body,
                label: &mut label,
            };
            (maker.make)(&mut g)
        };
        let Some(lines) = lines else { continue };
        let cand = format!("{src}{}\n", lines.join("\n"));
        // HISTORY_LAST=F: each line tried is written to F first, so a line that never ends is there
        if let Some(f) = std::env::var_os("HISTORY_LAST") {
            let _ = std::fs::write(f, &cand);
        }
        let t = std::time::Instant::now();
        let verdict = match catch_unwind(AssertUnwindSafe(|| m.update(&cand))) {
            Err(p) => {
                // the model is half updated: start it afresh
                m = model(&src);
                Verdict::Fail(format!("{}: panic: {}", maker.name, panic_text(p)))
            }
            Ok(()) => match &m.error {
                Some(f) => classify(&m.lines[f.at].toks[0], &f.msg),
                None => valid_after(maker.name, &m, m.built()),
            },
        };
        let secs = t.elapsed().as_secs_f64();
        if secs > slowest.0 {
            slowest = (secs, cand.clone());
        }
        match verdict {
            Verdict::Ok => {
                tried.push((maker.name, None));
                src = cand;
                built += 1;
            }
            Verdict::Refused(why) => {
                tried.push((maker.name, Some(why.clone())));
                refused.push((why, cand));
                m.update(&src);
            }
            Verdict::Fail(why) => {
                tried.push((maker.name, Some(why.clone())));
                fails.push((why, cand));
                m.update(&src);
            }
        }
    }
    let before = forge_model::nary_name_fallbacks();
    if let Verdict::Fail(what) = finish(&m) {
        fails.push((what, src.clone()));
    }
    let name_fallbacks = forge_model::nary_name_fallbacks() - before;
    Run {
        name_fallbacks,
        src,
        tried,
        fails,
        refused,
        slowest,
    }
}

/// The fewest lines of `src` that still fail as `want` does.
pub fn shrink(src: &str, want: &Verdict) -> String {
    let mut lines: Vec<String> = src.lines().map(String::from).collect();
    let same = |ls: &[String]| judge(&(ls.join("\n") + "\n")) == *want;
    // chunks from big to single lines, from the end (the cheapest to rebuild)
    let mut chunk = lines.len() / 2;
    while chunk >= 1 {
        let mut end = lines.len();
        while end > 1 {
            let start = end.saturating_sub(chunk).max(1);
            let mut t = lines.clone();
            t.drain(start..end);
            if same(&t) {
                lines = t;
                end = start.min(lines.len());
            } else {
                end = start;
            }
        }
        chunk /= 2;
    }
    // the first line too, when another can start the part
    if lines.len() > 1 && same(&lines[1..]) {
        lines.remove(0);
    }
    // then each line's references, one at a time
    for i in 0..lines.len() {
        let mut k = 0;
        loop {
            let refs: Vec<(usize, usize)> = spans(&lines[i]);
            if k >= refs.len() || refs.len() < 2 {
                break;
            }
            let (a, b) = refs[k];
            let mut t = lines.clone();
            t[i] = format!("{}{}", lines[i][..a].trim_end(), &lines[i][b..]);
            if same(&t) {
                lines = t;
            } else {
                k += 1;
            }
        }
    }
    lines.join("\n") + "\n"
}

/// Where a line's `(selector)` references are.
fn spans(line: &str) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut start = None;
    for (i, c) in line.char_indices() {
        match c {
            '(' => start = Some(i),
            ')' => {
                if let Some(a) = start.take() {
                    out.push((a, i + 1));
                }
            }
            _ => {}
        }
    }
    out
}
