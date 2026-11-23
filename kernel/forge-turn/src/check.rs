//! The certified clearance of a lathe tool moving in the (ρ, z) half-plane.
//!
//! The obstacle H is a region of the half-plane (the part's half-profile, the in-process stock,
//! the chuck's envelope), given by its boundary pieces. A body of the tool (insert.rs) is, slab by
//! slab, inside (K ⊕ B_r') × [a, b] in depth; its image, moved by the reference point's translation
//! g(s) along a segment, lies in
//!
//!   Z ⊕ B_r',  Z = K ⊕ [0, e]·x̂ ⊕ [g(0), g(1)] = conv(P),
//!
//! P the core's vertices moved to the parallelogram's four corners (insert.rs: e bounds the smear
//! of depth b at the slab's least |x|). For each piece f of the obstacle's boundary near Z, any
//! axis w gives dist(Z, f) ≥ (min over conv f of w·y − max over P of w·p) / |w| (Turn.lean
//! `sep_axis_hull`): the axes tried are the direction from f to the nearest point of P, the
//! normals of K's edges and of the generators, and f's own normal. When every piece is at least
//! zl > 0 from Z, Z is outside the obstacle or inside it whole (`clear_of_frontier`: Z is convex),
//! and one point of it decides which, exactly (region::inside_loop); then
//! dist(Z ⊕ B_r', H) ≥ zl − r' (`turn_clear_ball`).
//!
//! The half-profile's segment on the axis is no boundary of the part (the part goes on across
//! the axis), so the checks are made in the full section: Hf = H ∪ mirror(H). With σ(x, y, z) =
//! (sign(x)·√(x² + y²), z), π = |σ|, and dist(π(B), H) ≥ dist(σ(B), Hf) (a folded point's
//! distance to H is its own or its mirror's). σ moves a point of the section outward from the
//! axis by at most e, on its own side: σ(B) ⊆ (Z₊ ∪ Z₋) ⊕ B_r', Z± = K ⊕ [0, ±e]·x̂ ⊕ [g(0), g(1)],
//! Z₋ needed only when the outline reaches x < 0. A drill on the centre line (a solid of
//! revolution about the axis) is its own σ-image: no smear.
//!
//! An arc move is cut into chords; the path lies within the chord's sagitta, plus its ends' gap
//! off the circle, plus the difference of the radii at its two ends (a control's arc between
//! ends at unequal radii stays between them: a hypothesis on the control, H.arc), of the chord.
//!
//! Every f64 sum that places a point of P rounds: the bound subtracts a rounding allowance
//! (twice the ulp of the largest coordinate per addition, on both axes).
use crate::insert::Body;
use forge_cam_geom::ival::Ival;
use forge_cam_geom::real::Ctx;
use forge_cam_geom::region::{self, Exact, Region, Seg};
use forge_cam_geom::slab::Sec;
use forge_math::dmath;

/// One piece of an obstacle's boundary: the curve, its end gap (0 for a segment), its box.
#[derive(Clone, Debug)]
struct OPiece {
    sec: Sec,
    gap: f64,
    bx: [f64; 4],
}

/// An obstacle: a region of the half-plane, its boundary in pieces the checker takes, and the
/// bound mu on its distance from the true set (a clearance must hold by mu more).
pub struct Obstacle {
    pub name: String,
    pieces: Vec<OPiece>,
    loop_boxes: Vec<[f64; 4]>,
    region: Region,
    cx: Ctx,
    ex: Exact,
    pub mu: f64,
    pub bbox: [f64; 4],
    /// The region is a half-plane one standing for its full section (see `of`).
    fold: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ObstacleError {
    /// The region is not valid (region::validate): why.
    Region(String),
}

fn ip(x: f64) -> Ival {
    Ival::pt(x)
}

fn len2(u: [Ival; 2]) -> Ival {
    u[0].sqr().add(u[1].sqr()).sqrt()
}

fn sub(a: [f64; 2], b: [f64; 2]) -> [Ival; 2] {
    [ip(a[0]).sub(ip(b[0])), ip(a[1]).sub(ip(b[1]))]
}

fn merge(b: &mut [f64; 4], c: [f64; 4]) {
    b[0] = b[0].min(c[0]);
    b[1] = b[1].min(c[1]);
    b[2] = b[2].max(c[2]);
    b[3] = b[3].max(c[3]);
}

/// The pieces of an arc, each at most a quarter turn, with the gap of their stored ends off the
/// circle of radius r about c.
fn arc_pieces(a: [f64; 2], b: [f64; 2], c: [f64; 2], ccw: bool, out: &mut Vec<OPiece>) {
    let ra = len2(sub(a, c));
    let rb = len2(sub(b, c));
    let r = 0.5 * (ra.mid() + rb.mid());
    let gap_of = |p: [f64; 2]| {
        let d = len2(sub(p, c)).sub(ip(r)).abs();
        d.hi
    };
    let ang = |p: [f64; 2]| dmath::atan2(p[1] - c[1], p[0] - c[0]);
    let (t0, mut t1) = (ang(a), ang(b));
    let full = a == b;
    if ccw {
        while t1 <= t0 {
            t1 += std::f64::consts::TAU;
        }
        if full {
            t1 = t0 + std::f64::consts::TAU;
        }
    } else {
        while t1 >= t0 {
            t1 -= std::f64::consts::TAU;
        }
        if full {
            t1 = t0 - std::f64::consts::TAU;
        }
    }
    let n = (((t1 - t0).abs() / (0.5 * std::f64::consts::PI)).ceil() as usize).max(1);
    let mut prev = a;
    for i in 1..=n {
        let p = if i == n {
            b
        } else {
            let t = t0 + (t1 - t0) * i as f64 / n as f64;
            [c[0] + r * dmath::cos(t), c[1] + r * dmath::sin(t)]
        };
        let gap = gap_of(prev).max(gap_of(p));
        let sec = Sec::Arc {
            c,
            r,
            a: prev,
            b: p,
            ccw,
        };
        let bx = [
            ip(c[0]).sub(ip(r)).lo.min(prev[0]).min(p[0]),
            ip(c[1]).sub(ip(r)).lo.min(prev[1]).min(p[1]),
            ip(c[0]).add(ip(r)).hi.max(prev[0]).max(p[0]),
            ip(c[1]).add(ip(r)).hi.max(prev[1]).max(p[1]),
        ];
        out.push(OPiece { sec, gap, bx });
        prev = p;
    }
}

/// The region reflected in the axis (x → −x), its loops reversed so it stays on their left.
pub fn mirror(r: &Region) -> Region {
    let m = |p: region::P2| region::p2(-p.x, p.y);
    Region {
        loops: r
            .loops
            .iter()
            .map(|l| {
                region::Loop {
                    segs: l
                        .segs
                        .iter()
                        .map(|sg| match *sg {
                            Seg::Line { a, b } => Seg::Line { a: m(a), b: m(b) },
                            Seg::Arc { a, b, c, ccw } => Seg::Arc {
                                a: m(a),
                                b: m(b),
                                c: m(c),
                                ccw: !ccw,
                            },
                        })
                        .collect(),
                }
                .reversed()
            })
            .collect(),
    }
}

impl Obstacle {
    /// The obstacle of a half-plane region (x ≥ 0) as its full section Hf = H ∪ mirror(H): the
    /// boundary pieces of H off the axis and their mirror images (the segments on the axis are
    /// no boundary of Hf: H and its mirror meet there), and inside(q) ⇔ (|q.x|, q.z) ∈ H.
    pub fn of(name: &str, half: &Region, mu: f64) -> Result<Obstacle, ObstacleError> {
        let mut o = Obstacle::of_full(name, half, mu)?;
        let mut pieces = Vec::new();
        let mut loop_boxes = Vec::new();
        let mut bbox = [f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY];
        let mut start = 0;
        for l in &half.loops {
            // the pieces of this loop in o.pieces: as many as of_full made for it
            let mut n = 0;
            for sg in &l.segs {
                n += match *sg {
                    Seg::Line { .. } => 1,
                    Seg::Arc { .. } => {
                        let mut v = Vec::new();
                        if let Seg::Arc { a, b, c, ccw } = *sg {
                            arc_pieces([a.x, a.y], [b.x, b.y], [c.x, c.y], ccw, &mut v);
                        }
                        v.len()
                    }
                };
            }
            let mine = &o.pieces[start..start + n];
            start += n;
            let mut on_axis = false;
            let mut lb = [f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY];
            for p in mine {
                if let Sec::Seg { a, b } = p.sec
                    && a[0] == 0.0
                    && b[0] == 0.0
                {
                    on_axis = true;
                    continue;
                }
                if p.bx[0] <= 0.0 {
                    on_axis = true;
                }
                merge(&mut lb, p.bx);
                pieces.push(p.clone());
                let m = |q: [f64; 2]| [-q[0], q[1]];
                let sec = match p.sec {
                    Sec::Seg { a, b } => Sec::Seg { a: m(a), b: m(b) },
                    Sec::Arc { c, r, a, b, ccw } => Sec::Arc {
                        c: m(c),
                        r,
                        a: m(a),
                        b: m(b),
                        ccw: !ccw,
                    },
                };
                pieces.push(OPiece {
                    sec,
                    gap: p.gap,
                    bx: [-p.bx[2], p.bx[1], -p.bx[0], p.bx[3]],
                });
            }
            if lb[0] > lb[2] {
                continue;
            }
            let mb = [-lb[2], lb[1], -lb[0], lb[3]];
            if on_axis {
                // one boundary with its mirror (joined across the axis, or touching it)
                let mut u = lb;
                merge(&mut u, mb);
                loop_boxes.push(u);
            } else {
                loop_boxes.push(lb);
                loop_boxes.push(mb);
            }
            merge(&mut bbox, lb);
            merge(&mut bbox, mb);
        }
        o.pieces = pieces;
        o.loop_boxes = loop_boxes;
        o.bbox = bbox;
        o.fold = true;
        Ok(o)
    }

    /// The obstacle of a region of the whole plane as it is.
    pub fn of_full(name: &str, region: &Region, mu: f64) -> Result<Obstacle, ObstacleError> {
        let mut cx = Ctx::new();
        let ex = region::validate(&mut cx, region).map_err(|e| ObstacleError::Region(format!("{e:?}")))?;
        let mut pieces = Vec::new();
        let mut loop_boxes = Vec::new();
        let mut bbox = [f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY];
        for l in &region.loops {
            let first = pieces.len();
            for s in &l.segs {
                match *s {
                    Seg::Line { a, b } => {
                        let (a, b) = ([a.x, a.y], [b.x, b.y]);
                        pieces.push(OPiece {
                            sec: Sec::Seg { a, b },
                            gap: 0.0,
                            bx: [a[0].min(b[0]), a[1].min(b[1]), a[0].max(b[0]), a[1].max(b[1])],
                        });
                    }
                    Seg::Arc { a, b, c, ccw } => {
                        arc_pieces([a.x, a.y], [b.x, b.y], [c.x, c.y], ccw, &mut pieces)
                    }
                }
            }
            let mut lb = [f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY];
            for p in &pieces[first..] {
                merge(&mut lb, p.bx);
            }
            merge(&mut bbox, lb);
            loop_boxes.push(lb);
        }
        Ok(Obstacle {
            name: name.into(),
            pieces,
            loop_boxes,
            region: region.clone(),
            cx,
            ex,
            mu,
            bbox,
            fold: false,
        })
    }

    /// Whether q is inside the obstacle (exact; q must not be on its boundary). For a folded
    /// obstacle, q on the axis is moved off it by `off` (less than its distance to the boundary).
    pub fn inside(&mut self, q: [f64; 2]) -> bool {
        self.inside_off(q, 0.0)
    }

    fn inside_off(&mut self, q: [f64; 2], off: f64) -> bool {
        let q = if self.fold {
            [if q[0] == 0.0 { off } else { q[0].abs() }, q[1]]
        } else {
            q
        };
        let m = self.cx.mark();
        let p = forge_cam_geom::arr::pt(&mut self.cx, q[0], q[1]);
        let mut odd = false;
        for l in 0..self.ex.loops.len() {
            if region::inside_loop(&mut self.cx, &self.ex, l, p) {
                odd = !odd;
            }
        }
        self.cx.release(m);
        odd
    }

    pub fn region(&self) -> &Region {
        &self.region
    }
}

/// A move of the tool's reference point in (ρ, z).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Move {
    Line {
        a: [f64; 2],
        b: [f64; 2],
    },
    Arc {
        a: [f64; 2],
        b: [f64; 2],
        c: [f64; 2],
        ccw: bool,
    },
}

impl Move {
    pub fn start(&self) -> [f64; 2] {
        match *self {
            Move::Line { a, .. } | Move::Arc { a, .. } => a,
        }
    }
    pub fn end(&self) -> [f64; 2] {
        match *self {
            Move::Line { b, .. } | Move::Arc { b, .. } => b,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Verdict {
    /// Proven: every point of the body's image keeps at least `lower` from the obstacle.
    Clear { lower: f64 },
    /// A real witness: the body at depth 0 (a real section of the tool) comes within `d` of the
    /// obstacle, less than needed, near `at`.
    Hit { d: f64, at: [f64; 2] },
    /// Neither proven nor refuted at this resolution: refused.
    Undecided { lower: f64 },
}

impl Verdict {
    pub fn lower(&self) -> f64 {
        match *self {
            Verdict::Clear { lower } | Verdict::Undecided { lower } => lower,
            Verdict::Hit { d, .. } => d,
        }
    }
}

/// The rounding allowance for sums of coordinates no larger than m in magnitude, k additions.
fn round_allow(m: f64, k: f64) -> f64 {
    let u = f64::EPSILON * m.abs().max(1.0);
    // per axis k additions of at most an ulp each; both axes: √2 < 1.5
    1.5 * k * u
}

/// The least distance from Z = conv(P) to the obstacle's pieces near it, by separating axes:
/// for a piece f and any axis w, dist(Z, f) ≥ (min_{y ∈ conv f} w·y − max_{p ∈ P} w·p) / |w|
/// (Turn.lean `sep_axis_hull`), the max over P being the max over conv(P), the min over a piece
/// its min over its convex hull. The axes tried: the direction from the piece to the point of P
/// nearest it (f64; any axis is sound), the normals of K's edges and of the two generators, and
/// the piece's own normal. Z = K ⊕ [0, g₁] ⊕ [0, g₂] + p₀ is conv(P), P = the core's vertices
/// moved by p₀ and the parallelogram's four corners. `cut`: pieces whose box gap is at least
/// this are skipped (their distance is at least the box gap). A piece whose bound stays at 0
/// gives 0 (its distance not proven positive).
fn sat_lower(core: &[[f64; 2]], p0: [f64; 2], g1: [f64; 2], g2: [f64; 2], obs: &Obstacle, cut: f64, zbox: &[f64; 4]) -> f64 {
    let n = core.len();
    let corners = [[0.0, 0.0], g1, g2, [g1[0] + g2[0], g1[1] + g2[1]]];
    let mut pts: Vec<[f64; 2]> = Vec::with_capacity(4 * n);
    for k in core {
        for t in corners {
            pts.push([p0[0] + k[0] + t[0], p0[1] + k[1] + t[1]]);
        }
    }
    // the axes every piece shares: K's edge normals and the generators' (both senses)
    let mut axes: Vec<[f64; 2]> = Vec::with_capacity(n + 4);
    for i in 0..n {
        let (u, v) = (core[i], core[(i + 1) % n]);
        let d = [v[0] - u[0], v[1] - u[1]];
        if d != [0.0, 0.0] {
            axes.push([d[1], -d[0]]);
            axes.push([-d[1], d[0]]);
        }
    }
    for g in [g1, g2] {
        if g != [0.0, 0.0] {
            axes.push([g[1], -g[0]]);
            axes.push([-g[1], g[0]]);
        }
    }
    let mut best = f64::INFINITY;
    for f in &obs.pieces {
        let bg = box_gap(zbox, &f.bx);
        if bg >= cut || bg >= best {
            continue;
        }
        let mut piece_best = piece_gap(&pts, &axes, &f.sec, f.gap, cut);
        // a concave arc's hull reaches over what it faces: if the whole arc is not separated,
        // its sub-arcs are (their hulls lie within their sagitta of the arc), the least of them
        if piece_best < cut
            && let Sec::Arc { c, r, a, b, ccw } = f.sec
        {
            let ang = |p: [f64; 2]| dmath::atan2(p[1] - c[1], p[0] - c[0]);
            let (t0, mut t1) = (ang(a), ang(b));
            if ccw {
                while t1 <= t0 {
                    t1 += dmath::TAU;
                }
            } else {
                while t1 >= t0 {
                    t1 -= dmath::TAU;
                }
            }
            let mut k = 4;
            while k <= 64 {
                let mut sub_min = f64::INFINITY;
                let mut prev = a;
                for i in 1..=k {
                    let p = if i == k {
                        b
                    } else {
                        let t = t0 + (t1 - t0) * i as f64 / k as f64;
                        [c[0] + r * dmath::cos(t), c[1] + r * dmath::sin(t)]
                    };
                    let g = len2(sub(prev, c)).sub(ip(r)).abs().hi.max(len2(sub(p, c)).sub(ip(r)).abs().hi);
                    let sec = Sec::Arc { c, r, a: prev, b: p, ccw };
                    sub_min = sub_min.min(piece_gap(&pts, &axes, &sec, f.gap.max(g), cut));
                    prev = p;
                    if sub_min <= piece_best {
                        break;
                    }
                }
                piece_best = piece_best.max(sub_min);
                if piece_best >= cut {
                    break;
                }
                k *= 4;
            }
        }
        if piece_best < best {
            best = piece_best;
        }
    }
    best
}

/// The best separating-axis gap of one piece from conv(P): the direction from P's nearest point
/// toward the piece, the piece's own normal (a segment), and the shared axes.
fn piece_gap(pts: &[[f64; 2]], axes: &[[f64; 2]], sec: &Sec, gap: f64, cut: f64) -> f64 {
    let mut own: [[f64; 2]; 3] = [[0.0, 0.0]; 3];
    let mut no = 0;
    if let Sec::Seg { a, b } = *sec {
        let d = [b[0] - a[0], b[1] - a[1]];
        if d != [0.0, 0.0] {
            own[0] = [d[1], -d[0]];
            own[1] = [-d[1], d[0]];
            no = 2;
        }
    }
    let mut near = (f64::INFINITY, [0.0, 0.0]);
    for p in pts {
        let q = approx_nearest(sec, *p);
        let d2 = (p[0] - q[0]).powi(2) + (p[1] - q[1]).powi(2);
        if d2 < near.0 {
            // from Z toward the piece: the axis along which the piece lies beyond Z
            near = (d2, [q[0] - p[0], q[1] - p[1]]);
        }
    }
    if near.1 != [0.0, 0.0] {
        own[no] = near.1;
        no += 1;
    }
    let mut best = 0.0f64;
    for w in own[..no].iter().chain(axes.iter()) {
        let g = axis_gap(pts, sec, gap, *w);
        if g > best {
            best = g;
            if best >= cut {
                break;
            }
        }
    }
    best
}

/// A point of the piece near p (f64, for choosing an axis only).
fn approx_nearest(s: &Sec, p: [f64; 2]) -> [f64; 2] {
    match *s {
        Sec::Seg { a, b } => {
            let d = [b[0] - a[0], b[1] - a[1]];
            let dd = d[0] * d[0] + d[1] * d[1];
            let t = if dd > 0.0 {
                (((p[0] - a[0]) * d[0] + (p[1] - a[1]) * d[1]) / dd).clamp(0.0, 1.0)
            } else {
                0.0
            };
            [a[0] + t * d[0], a[1] + t * d[1]]
        }
        Sec::Arc { c, r, a, b, ccw } => {
            let v = [p[0] - c[0], p[1] - c[1]];
            let l = (v[0] * v[0] + v[1] * v[1]).sqrt();
            if l > 0.0 && sector(c, a, b, ccw, [ip(v[0]), ip(v[1])]) > 0 {
                return [c[0] + r * v[0] / l, c[1] + r * v[1] / l];
            }
            let da = (p[0] - a[0]).powi(2) + (p[1] - a[1]).powi(2);
            let db = (p[0] - b[0]).powi(2) + (p[1] - b[1]).powi(2);
            if da <= db { a } else { b }
        }
    }
}

/// 1 if the direction d (from the centre) is surely inside the arc's sector, -1 surely outside,
/// 0 unknown (arcs under a half turn).
fn sector(c: [f64; 2], a: [f64; 2], b: [f64; 2], ccw: bool, d: [Ival; 2]) -> i32 {
    let (ua, ub) = (sub(a, c), sub(b, c));
    let cr = |u: [Ival; 2], v: [Ival; 2]| u[0].mul(v[1]).sub(u[1].mul(v[0]));
    let (c1, c2) = if ccw { (cr(ua, d), cr(d, ub)) } else { (cr(d, ua), cr(ub, d)) };
    if c1.lo >= 0.0 && c2.lo >= 0.0 {
        1
    } else if c1.hi < 0.0 || c2.hi < 0.0 {
        -1
    } else {
        0
    }
}

/// (min over the piece's convex hull of w·y − max over P of w·p) / |w|, rounded down; ≤ 0 when
/// the axis does not separate. An arc's ends off its circle by `gap` count twice, as slab::dist
/// takes them.
fn axis_gap(pts: &[[f64; 2]], s: &Sec, gap: f64, w: [f64; 2]) -> f64 {
    let wi = [ip(w[0]), ip(w[1])];
    let dot = |p: [f64; 2]| wi[0].mul(ip(p[0])).add(wi[1].mul(ip(p[1])));
    let mut pmax = f64::NEG_INFINITY;
    for p in pts {
        pmax = pmax.max(dot(*p).hi);
    }
    let wl = len2(wi);
    let fmin = match *s {
        Sec::Seg { a, b } => dot(a).lo.min(dot(b).lo),
        Sec::Arc { c, r, a, b, ccw } => {
            let mut m = dot(a).lo.min(dot(b).lo);
            // the arc's point in direction −w, if it may be in the sector
            if sector(c, a, b, ccw, [wi[0].neg(), wi[1].neg()]) >= 0 {
                m = m.min(dot(c).sub(ip(r).mul(wl)).lo);
            }
            ip(m).sub(ip(2.0 * gap).mul(wl)).lo
        }
    };
    let num = ip(fmin).sub(ip(pmax)).lo;
    if !(num > 0.0) {
        return 0.0;
    }
    ip(num).div(wl).lo.max(0.0)
}

fn box_gap(a: &[f64; 4], b: &[f64; 4]) -> f64 {
    let gx = ip(b[0]).sub(ip(a[2])).lo.max(ip(a[0]).sub(ip(b[2])).lo).max(0.0);
    let gy = ip(b[1]).sub(ip(a[3])).lo.max(ip(a[1]).sub(ip(b[3])).lo).max(0.0);
    ip(gx).sqr().add(ip(gy).sqr()).sqrt().lo
}

fn core_box(core: &[[f64; 2]], p: [f64; 2]) -> [f64; 4] {
    let mut b = [f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY];
    for k in core {
        let q = [p[0] + k[0], p[1] + k[1]];
        merge(&mut b, [q[0], q[1], q[0], q[1]]);
    }
    b
}

/// The number of depth slabs a body is cut into (more: a tighter smear bound).
pub const SLABS: usize = 6;

/// The clearance of one body moving along a straight move: the least distance proven, over the
/// body's slabs (`need`: what must be proven, for the early exit).
fn body_line(body: &Body, a: [f64; 2], b: [f64; 2], obs: &mut Obstacle, need: f64, slabs: usize) -> Verdict {
    let mut coords = 0.0f64;
    for k in &body.core {
        coords = coords.max(k[0].abs()).max(k[1].abs());
    }
    for p in [a, b] {
        coords = coords.max(p[0].abs()).max(p[1].abs());
    }
    // the sides the outline reaches: +x always, −x when some of it is at x < 0 at either end
    let xmin_core = body.core.iter().map(|k| k[0]).fold(f64::INFINITY, f64::min);
    let reach_neg = ip(a[0].min(b[0])).add(ip(xmin_core)).sub(ip(body.r)).lo < 0.0;
    let sides: &[f64] = if body.on_axis || !reach_neg {
        &[1.0]
    } else {
        &[1.0, -1.0]
    };
    let mut lower = f64::INFINITY;
    let mut checked_point = false;
    let n = if body.on_axis || body.depth[1] == 0.0 {
        1
    } else {
        slabs.max(1)
    };
    let core = &body.core;
    let mg = [b[0] - a[0], b[1] - a[1]];
    for &side in sides {
        for j in 0..n {
            let (d0, d1) = (body.depth[0], body.depth[1]);
            let top = d0 + (d1 - d0) * j as f64 / n as f64;
            let bot = if j + 1 == n {
                d1
            } else {
                d0 + (d1 - d0) * (j + 1) as f64 / n as f64
            };
            // the slab's radius: the outline shrunk by its relief at the slab's top
            let rj = if body.relief > 0.0 {
                ip(body.r).sub(ip(top - d0).mul(ip(body.relief))).hi.max(0.0)
            } else {
                body.r
            };
            // the smear: √(xm² + bot²) − xm at the least |x| of the slab's section (0 when it
            // reaches the axis)
            let e = if body.on_axis || bot == 0.0 {
                0.0
            } else {
                let xm = if reach_neg {
                    0.0
                } else {
                    ip(a[0].min(b[0])).add(ip(xmin_core)).sub(ip(rj)).lo.max(0.0)
                };
                ip(xm).sqr().add(ip(bot).sqr()).sqrt().sub(ip(xm)).hi
            };
            let g1 = [side * e, 0.0];
            let mut zbox = core_box(core, a);
            merge(&mut zbox, core_box(core, b));
            if side > 0.0 {
                zbox[2] = ip(zbox[2]).add(ip(e)).hi;
            } else {
                zbox[0] = ip(zbox[0]).sub(ip(e)).lo;
            }
            let need_core = need + rj;
            let cut = ip(need_core).add(ip(obs.mu)).add(ip(1.0)).hi;
            let raw = sat_lower(core, a, g1, mg, obs, cut, &zbox);
            let allow = round_allow(coords + e, 3.0);
            let zl = ip(raw).sub(ip(allow)).sub(ip(obs.mu)).lo;
            if zl <= 0.0 {
                return Verdict::Undecided { lower: zl - rj };
            }
            // every piece of ∂Hf misses Z (each at least zl > 0 from it), so by
            // clear_of_frontier one point decides whether Z is outside Hf
            // and Z is outside Hf: one point of it (the core's first vertex at the start: in
            // every slab's Z on both sides, and at least zl from ∂Hf)
            if !checked_point {
                let q = [a[0] + core[0][0], a[1] + core[0][1]];
                if obs.inside_off(q, 0.5 * zl) {
                    // inside by at least zl: a real point of the tool when it is at depth 0 on
                    // the x ≥ 0 side (σ is the identity there, and dist to Hf is dist to H)
                    if body.depth[0] == 0.0 && q[0] >= 0.0 && -zl < need {
                        return Verdict::Hit { d: -zl, at: q };
                    }
                    return Verdict::Undecided { lower: -zl };
                }
                checked_point = true;
            }
            let l = ip(zl).sub(ip(rj)).lo;
            lower = lower.min(l);
            if lower < need {
                return Verdict::Undecided { lower };
            }
        }
    }
    Verdict::Clear { lower }
}

/// A real witness that the body comes nearer than `need`: at either end of the move, a core
/// vertex q at depth 0 with its disc in x ≥ 0 (where π is the identity at y = 0), either outside
/// the obstacle with an upper bound on dist(q, H) − r below need, or inside it by a proven
/// positive depth.
fn witness(body: &Body, a: [f64; 2], b: [f64; 2], obs: &mut Obstacle, need: f64) -> Option<Verdict> {
    if body.depth[0] != 0.0 || body.core.is_empty() {
        return None;
    }
    let pts: Vec<[f64; 2]> = [a, b]
        .iter()
        .flat_map(|p| body.core.iter().map(move |k| [p[0] + k[0], p[1] + k[1]]))
        .collect();
    for q in pts {
        if q[0] - body.r < 0.0 {
            continue;
        }
        let mut up = f64::INFINITY;
        let mut lo = f64::INFINITY;
        for f in &obs.pieces {
            let d = forge_cam_geom::slab::pt_sec(q, &f.sec);
            up = up.min(ip(d.hi).add(ip(2.0 * f.gap)).hi);
            lo = lo.min(ip(d.lo).sub(ip(2.0 * f.gap)).lo);
        }
        let lo = ip(lo).sub(ip(obs.mu)).lo;
        if lo > 0.0 && obs.inside_off(q, 0.5 * lo) {
            // a real point of the tool (depth 0, x ≥ 0) at least lo inside the obstacle
            let d = ip(-lo).sub(ip(body.r)).hi;
            if d < need {
                return Some(Verdict::Hit { d, at: q });
            }
            continue;
        }
        let d = ip(up).add(ip(obs.mu)).sub(ip(body.r)).hi;
        if d < need && !obs.inside(q) {
            return Some(Verdict::Hit { d, at: q });
        }
    }
    None
}

/// The clearance of a body along a move, at least `need` proven, or why not. An arc move is cut
/// into up to 64 chords until the chords' allowance is small against the margin.
pub fn clearance(body: &Body, mv: &Move, obs: &mut Obstacle, need: f64) -> Verdict {
    match *mv {
        Move::Line { a, b } => {
            let v = body_line(body, a, b, obs, need, SLABS);
            if let Verdict::Undecided { .. } = v {
                if let Some(h) = witness(body, a, b, obs, need) {
                    return h;
                }
                // a finer depth cut tightens the smear bound
                return body_line(body, a, b, obs, need, 4 * SLABS);
            }
            v
        }
        Move::Arc { a, b, c, ccw } => {
            let ra = len2(sub(a, c));
            let rb = len2(sub(b, c));
            let r = 0.5 * (ra.mid() + rb.mid());
            let dr = ra.sub(rb).abs().hi;
            let ang = |p: [f64; 2]| dmath::atan2(p[1] - c[1], p[0] - c[0]);
            let (t0, mut t1) = (ang(a), ang(b));
            if ccw {
                while t1 <= t0 {
                    t1 += std::f64::consts::TAU;
                }
            } else {
                while t1 >= t0 {
                    t1 -= std::f64::consts::TAU;
                }
            }
            let mut n = ((t1 - t0).abs() / 0.2).ceil().max(1.0) as usize;
            loop {
                let mut lower = f64::INFINITY;
                let mut prev = a;
                let mut undecided = false;
                for i in 1..=n {
                    let p = if i == n {
                        b
                    } else {
                        let t = t0 + (t1 - t0) * i as f64 / n as f64;
                        [c[0] + r * dmath::cos(t), c[1] + r * dmath::sin(t)]
                    };
                    // the arc between prev and p lies within the chord's sagitta, the ends' gap
                    // off the circle and the radius difference
                    let half = len2(sub(p, prev)).scale(0.5);
                    let rr = ra.hull(rb);
                    let sag = rr.sub(rr.sqr().sub(half.sqr()).max(ip(0.0)).sqrt()).hi.max(0.0);
                    let g = len2(sub(prev, c))
                        .sub(ip(r))
                        .abs()
                        .hi
                        .max(len2(sub(p, c)).sub(ip(r)).abs().hi);
                    let allow = ip(sag).add(ip(g)).add(ip(dr)).hi;
                    let v = clearance(body, &Move::Line { a: prev, b: p }, obs, need + allow);
                    match v {
                        // a witness on a chord is a real path point (the chord's ends are on
                        // the arc, up to their gap): a hit only against the arc's own need
                        Verdict::Hit { d, .. } if ip(d).add(ip(g)).hi < need => return v,
                        Verdict::Hit { d, .. } => {
                            undecided = true;
                            lower = lower.min(ip(d).sub(ip(allow)).lo);
                        }
                        Verdict::Undecided { lower: l } => {
                            undecided = true;
                            lower = lower.min(ip(l).sub(ip(allow)).lo);
                        }
                        Verdict::Clear { lower: l } => lower = lower.min(ip(l).sub(ip(allow)).lo),
                    }
                    prev = p;
                }
                if !undecided && lower >= need {
                    return Verdict::Clear { lower };
                }
                if n >= 64 {
                    return Verdict::Undecided { lower };
                }
                n *= 4;
            }
        }
    }
}
