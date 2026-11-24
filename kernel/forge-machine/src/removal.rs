//! Material removal along the simulated motion: what the posted program leaves of the stock.
//!
//! - Milling with the tool along the part's +Z (3-axis, and 3+2 once a setup's rotaries are
//!   fixed so): a height map over the stock's footprint. Each node holds the material's top; a
//!   move lowers it to the least height of the cutter's lower surface over that node along the
//!   move, min over s of tip_z(s) + g(|p − tip_xy(s)|), with g the cutter's profile (flat, ball,
//!   bull).
//! - Turning: a radius map along the part's axis. Each node holds the outer radius; a move of the
//!   nose (a disc of radius rn) lowers it to the least radius the disc reaches at that z.
//!
//! On a move at one height (a line with both ends at the same z, or an arc whose joints turn the
//! tip on a plane circle or, with its end radius rounded, a spiral) the profile only rises with
//! distance, so the least is at the path's nearest point, found in closed form (a spiral's by a
//! few Newton steps from the node's own direction). Otherwise the minimised function is convex in
//! s on a straight move where the cutter covers the node (a convex non-decreasing profile of a
//! convex distance, plus a linear term), so a ternary search finds it to 1e-13 in s; on an arc it
//! is minimised over 64 samples per eighth of a turn, then refined. docs/cam-machine.md bounds
//! how far the closed form is from the search (Removal: the tip's two forms). This is a simulation in floating point at the nodes, not a proof: it is
//! what a reviewer looks at, and it finds rapids into material with the stock as cut so far,
//! which the collision check (initial stock) cannot.
//!
//! A move whose tool axis is not the part's +Z (a tilt) is refused for milling, with its line.
use crate::kin::V3;
use crate::sim::{Cut, Path, SegKind, Segment, Setup, Timeline};
use forge_math::dmath;

#[derive(Clone, Debug, PartialEq)]
pub enum RemovalError {
    /// The tool's axis is not the part's +Z here (milling removal is for a fixed vertical axis).
    Tilted {
        line: usize,
    },
    /// A segment carries a tool the setup does not have.
    NoTool {
        line: usize,
        t: u32,
    },
    /// The tool's cutting shape does not suit the map (a nose on a mill map, a cutter on a lathe's).
    Shape {
        line: usize,
        why: String,
    },
    Grid(String),
}

/// A rapid that removes material: a crash with the stock as cut so far.
#[derive(Clone, Debug, PartialEq)]
pub struct RapidCut {
    pub line: usize,
    pub segment: usize,
    /// The deepest the rapid would cut, mm, and where (part frame).
    pub depth: f64,
    pub at: V3,
}

/// A non-cutting piece of the tool (shank, holder: `SimTool::body[piece]`, piece ≥ 1) reaching
/// into material left so far.
#[derive(Clone, Debug, PartialEq)]
pub struct HolderHit {
    pub line: usize,
    pub segment: usize,
    pub piece: usize,
    pub depth: f64,
    pub at: V3,
}

/// Where a skip (probe) move's stylus ball first meets the stock as cut so far: the trigger.
/// Simulation-level (floating point at the nodes, like the removal): the stop the machine makes,
/// not a proof of it.
#[derive(Clone, Debug, PartialEq)]
pub struct Trigger {
    pub line: usize,
    pub segment: usize,
    /// The path parameter of the first contact, 0..=1.
    pub s: f64,
    /// The ball's tip (lowest point) at the contact, part frame.
    pub tip: V3,
    /// The stock node touched, part frame (its height as cut).
    pub at: V3,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HeightMap {
    pub x0: f64,
    pub y0: f64,
    pub step: f64,
    pub nx: usize,
    pub ny: usize,
    pub bottom: f64,
    pub top: f64,
    pub z: Vec<f64>,
}

impl HeightMap {
    pub fn at(&self, i: usize, j: usize) -> f64 {
        self.z[j * self.nx + i]
    }
    pub fn node(&self, i: usize, j: usize) -> (f64, f64) {
        (self.x0 + i as f64 * self.step, self.y0 + j as f64 * self.step)
    }
    /// The material's volume, the node heights as cells of step² (mm³).
    pub fn volume(&self) -> f64 {
        self.z.iter().map(|z| (z - self.bottom).max(0.0)).sum::<f64>() * self.step * self.step
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MillResult {
    pub map: HeightMap,
    pub rapid_cuts: Vec<RapidCut>,
    pub holder_hits: Vec<HolderHit>,
    /// Per segment, the volume it removed (node heights × step²).
    pub removed: Vec<f64>,
    /// Seconds (t_min) of feed moves that removed nothing.
    pub air_feed_s: f64,
    /// Each skip move's trigger with a probe in the spindle (none: it ran to its end without
    /// touching the stock: a probe would alarm "no contact" there).
    pub triggers: Vec<Trigger>,
    /// A probe's stylus into the stock on a move that is not a skip move: a crash.
    pub probe_hits: Vec<RapidCut>,
}

/// The cutter's lower surface height above its tip at distance d from its axis (None outside).
fn profile(r: f64, corner: f64, d: f64) -> Option<f64> {
    if d > r {
        return None;
    }
    let flat = r - corner;
    if d <= flat {
        return Some(0.0);
    }
    let e = d - flat;
    Some(corner - (corner * corner - e * e).max(0.0).sqrt())
}

/// How removal finds a node's lowest cutter height over a move. `Closed` (what `mill` uses): the
/// affine tip, and on a move at one height (a line, or an arc whose joints turn the tip on a
/// circle) the nearest point of the path in closed form; otherwise the search. `Affine`: the
/// affine tip and the search on every move. `Chained`: the pose chain at every s and the search.
/// The last two are kept to check the first against (docs/cam-machine.md).
#[doc(hidden)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TipForm {
    Closed,
    Affine,
    Chained,
}

fn tip_path<'a>(
    s: &'a Setup<'a>,
    seg: &'a Segment,
    t: u32,
    form: TipForm,
) -> Result<Box<dyn Fn(f64) -> V3 + 'a>, RemovalError> {
    // rotaries fixed over the segment, the axis the part's +Z at both ends
    for (j, jt) in s.def.joints.iter().enumerate() {
        if jt.ty == crate::def::JointType::Rotary && seg.q0[j] != seg.q1[j] {
            return Err(RemovalError::Tilted { line: seg.line });
        }
    }
    let (_, ax) = s
        .tip_in_part(&seg.q0, t)
        .ok_or(RemovalError::NoTool { line: seg.line, t })?;
    if (ax[0].abs() > 1e-12) || (ax[1].abs() > 1e-12) || (ax[2] - 1.0).abs() > 1e-12 {
        return Err(RemovalError::Tilted { line: seg.line });
    }
    let p: Option<Box<dyn Fn(f64) -> V3 + 'a>> = match form {
        TipForm::Closed | TipForm::Affine => s
            .tip_affine(seg, t)
            .map(|f| Box::new(f) as Box<dyn Fn(f64) -> V3>),
        TipForm::Chained => s.tip_along(seg, t).map(|f| Box::new(f) as Box<dyn Fn(f64) -> V3>),
    };
    p.ok_or(RemovalError::NoTool { line: seg.line, t })
}

/// Least of f over [a, b] for f convex there (ternary search, endpoints included).
fn convex_min(f: &dyn Fn(f64) -> f64, mut a: f64, mut b: f64) -> f64 {
    let (fa, fb) = (f(a), f(b));
    let mut best = fa.min(fb);
    for _ in 0..100 {
        if b - a <= 1e-13 {
            break;
        }
        let m1 = a + (b - a) / 3.0;
        let m2 = b - (b - a) / 3.0;
        let (f1, f2) = (f(m1), f(m2));
        best = best.min(f1).min(f2);
        if f1 <= f2 {
            b = m2;
        } else {
            a = m1;
        }
    }
    best.min(f(0.5 * (a + b)))
}

/// Least of f over [0, 1] for any continuous f: dense samples, then a local refinement.
/// The least u in [0, 1] with f(u) <= level, for an f that reaches the level somewhere (the
/// caller saw its minimum under it): 256 samples; the first sample at or under the level, or
/// else the minimiser (sampled, then golden-section refined in its neighbourhood: a contact
/// between two samples lies before it), brackets the crossing, which bisection finds to 1e-12.
fn first_contact(f: &dyn Fn(f64) -> f64, level: f64) -> f64 {
    if f(0.0) <= level {
        return 0.0;
    }
    let n = 256;
    let bisect = |mut a: f64, mut b: f64| {
        while b - a > 1e-12 {
            let m = 0.5 * (a + b);
            if f(m) <= level { b = m } else { a = m }
        }
        b
    };
    let mut best = (f64::INFINITY, 0usize);
    for k in 1..=n {
        let u = k as f64 / n as f64;
        let v = f(u);
        if v <= level {
            return bisect((k - 1) as f64 / n as f64, u);
        }
        if v < best.0 {
            best = (v, k);
        }
    }
    // the minimum falls between samples: refine it in the samples around the least one
    let (mut a, mut b) = (
        (best.1 as f64 - 1.0).max(0.0) / n as f64,
        (best.1 as f64 + 1.0).min(n as f64) / n as f64,
    );
    let g = 0.5 * (5f64.sqrt() - 1.0);
    for _ in 0..80 {
        let (c, d) = (b - g * (b - a), a + g * (b - a));
        if f(c) <= level {
            return bisect(a, c);
        }
        if f(c) < f(d) { b = d } else { a = c }
    }
    let m = 0.5 * (a + b);
    if f(m) <= level { bisect(a, m) } else { m }
}

fn sampled_min(f: &dyn Fn(f64) -> f64, n: usize) -> f64 {
    let mut best = f64::INFINITY;
    let mut at = 0.0;
    for i in 0..=n {
        let u = i as f64 / n as f64;
        let v = f(u);
        if v < best {
            best = v;
            at = u;
        }
    }
    let h = 1.0 / n as f64;
    let (a, b) = ((at - h).max(0.0), (at + h).min(1.0));
    best.min(convex_min(f, a, b))
}

/// An arc's tip path in the part's frame when it is a plane spiral at one height (a circle when
/// r = r1): the centre, the radius at joint angle a0 and its change per radian, the part-frame
/// angle of joint angle 0 and the sense joint angles turn in it (+1 ccw).
struct Circle {
    c: [f64; 2],
    r: f64,
    k: f64,
    alpha: f64,
    dir: f64,
    a0: f64,
    sweep: f64,
}

/// The circle, when no joint but the arc's moves and both joints' columns lie in the part's xy
/// plane, a unit vector and its quarter turn: then the tip is the joints' arc, turned and moved.
fn flat_circle(s: &Setup, seg: &Segment, t: u32) -> Option<Circle> {
    let Path::Arc {
        a,
        b,
        c,
        r,
        r1,
        a0,
        sweep,
    } = seg.path
    else {
        return None;
    };
    if sweep == 0.0 {
        return None;
    }
    let (base, cols) = s.tip_columns(seg, t)?;
    let col = |j: usize| cols.iter().find(|(k, _)| *k == j).map(|(_, v)| *v);
    if cols
        .iter()
        .any(|(k, _)| *k != a && *k != b && seg.q0[*k] != seg.q1[*k])
    {
        return None;
    }
    let (ca, cb) = (col(a)?, col(b)?);
    if ca[2] != 0.0 || cb[2] != 0.0 || ((ca[0] * ca[0] + ca[1] * ca[1]) - 1.0).abs() > 8.0 * f64::EPSILON {
        return None;
    }
    let dir = if cb[0] == -ca[1] && cb[1] == ca[0] {
        1.0
    } else if cb[0] == ca[1] && cb[1] == -ca[0] {
        -1.0
    } else {
        return None;
    };
    let (da, db) = (c[0] - seg.q0[a], c[1] - seg.q0[b]);
    Some(Circle {
        c: [
            base[0] + da * ca[0] + db * cb[0],
            base[1] + da * ca[1] + db * cb[1],
        ],
        r,
        k: (r1 - r) / sweep,
        alpha: dmath::atan2(ca[1], ca[0]),
        dir,
        a0,
        sweep,
    })
}

impl Circle {
    /// The xy distance from p to the arc (ends pa and pb): for each turn of the node's own
    /// direction that lies within the sweep, the nearest point of the spiral about it (Newton on
    /// the squared distance from that direction; a circle's is the direction itself), and the
    /// ends.
    fn dist(&self, p: [f64; 2], pa: [f64; 2], pb: [f64; 2]) -> f64 {
        let e = |q: [f64; 2]| ((p[0] - q[0]).powi(2) + (p[1] - q[1]).powi(2)).sqrt();
        let mut best = e(pa).min(e(pb));
        let w = [p[0] - self.c[0], p[1] - self.c[1]];
        let rho = (w[0] * w[0] + w[1] * w[1]).sqrt();
        let (lo, hi) = if self.sweep > 0.0 {
            (self.a0, self.a0 + self.sweep)
        } else {
            (self.a0 + self.sweep, self.a0)
        };
        let rad = |th: f64| self.r + self.k * (th - self.a0);
        if rho == 0.0 {
            // every point of the arc is its radius away: the least at an end
            return best.min(rad(lo).abs()).min(rad(hi).abs());
        }
        let two_pi = 2.0 * dmath::PI;
        // the node's direction as a joint angle, every turn of it within the sweep
        let th0 = self.dir * (dmath::atan2(w[1], w[0]) - self.alpha);
        let mut th = lo + (th0 - lo).rem_euclid(two_pi);
        while th <= hi {
            // f²(Δ)/2 = (ρ² + R² − 2ρR cos Δ)/2, R = rad(th + Δ): its derivative
            // g = Rk − ρk cos Δ + ρR sin Δ, and g' = k² + 2ρk sin Δ + ρR cos Δ
            let mut d = 0.0f64;
            for _ in 0..8 {
                let big_r = rad(th + d);
                let (sn, cs) = dmath::sin_cos(d);
                let g = big_r * self.k - rho * self.k * cs + rho * big_r * sn;
                let gp = self.k * self.k + 2.0 * rho * self.k * sn + rho * big_r * cs;
                if g == 0.0 || gp <= 0.0 {
                    break;
                }
                let nd = (d - g / gp).clamp(lo - th, hi - th);
                if nd == d {
                    break;
                }
                d = nd;
            }
            let big_r = rad(th + d);
            let (sn, cs) = dmath::sin_cos(d);
            let f = ((rho - big_r * cs).powi(2) + (big_r * sn).powi(2)).sqrt();
            best = best.min(f);
            th += two_pi;
        }
        best
    }
}

/// The xy distance from p to the segment a–b.
fn seg_dist(a: [f64; 2], b: [f64; 2], p: [f64; 2]) -> f64 {
    let d = [b[0] - a[0], b[1] - a[1]];
    let w = [p[0] - a[0], p[1] - a[1]];
    let l2 = d[0] * d[0] + d[1] * d[1];
    let t = if l2 > 0.0 {
        ((w[0] * d[0] + w[1] * d[1]) / l2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let e = [w[0] - t * d[0], w[1] - t * d[1]];
    (e[0] * e[0] + e[1] * e[1]).sqrt()
}

/// The s-range where a line from a to b (xy) passes within r of p.
fn within(a: [f64; 2], b: [f64; 2], p: [f64; 2], r: f64) -> Option<(f64, f64)> {
    let d = [b[0] - a[0], b[1] - a[1]];
    let w = [a[0] - p[0], a[1] - p[1]];
    let qa = d[0] * d[0] + d[1] * d[1];
    let qb = 2.0 * (d[0] * w[0] + d[1] * w[1]);
    let qc = w[0] * w[0] + w[1] * w[1] - r * r;
    if qa == 0.0 {
        return if qc <= 0.0 { Some((0.0, 1.0)) } else { None };
    }
    let disc = qb * qb - 4.0 * qa * qc;
    if disc < 0.0 {
        return None;
    }
    let sq = disc.sqrt();
    let s0 = ((-qb - sq) / (2.0 * qa)).max(0.0);
    let s1 = ((-qb + sq) / (2.0 * qa)).min(1.0);
    if s0 > s1 { None } else { Some((s0, s1)) }
}

/// Mills a box stock (part coordinates) through a timeline on a grid of `step`.
pub fn mill(s: &Setup, tl: &Timeline, min: V3, max: V3, step: f64) -> Result<MillResult, RemovalError> {
    mill_with(s, tl, min, max, step, TipForm::Closed)
}

/// `mill` with the tip's form chosen (tests check the affine form against the chained one).
#[doc(hidden)]
pub fn mill_with(
    s: &Setup,
    tl: &Timeline,
    min: V3,
    max: V3,
    step: f64,
    form: TipForm,
) -> Result<MillResult, RemovalError> {
    if step.is_nan() || step <= 0.0 || (0..3).any(|k| min[k].is_nan() || max[k].is_nan() || min[k] >= max[k])
    {
        return Err(RemovalError::Grid(
            "a stock box needs min < max and a step > 0".into(),
        ));
    }
    let nx = ((max[0] - min[0]) / step).floor() as usize + 1;
    let ny = ((max[1] - min[1]) / step).floor() as usize + 1;
    if nx * ny > 20_000_000 {
        return Err(RemovalError::Grid(format!(
            "{nx} × {ny} nodes: use a coarser step"
        )));
    }
    let mut map = HeightMap {
        x0: min[0],
        y0: min[1],
        step,
        nx,
        ny,
        bottom: min[2],
        top: max[2],
        z: vec![max[2]; nx * ny],
    };
    let mut rapid_cuts = Vec::new();
    let mut holder_hits = Vec::new();
    let mut triggers = Vec::new();
    let mut probe_hits = Vec::new();
    // the lines whose segment is a skip move (sim marks it)
    let skip_lines: Vec<usize> = tl
        .marks
        .iter()
        .filter(|m| {
            matches!(
                m.what,
                crate::sim::Happening::Skip { .. } | crate::sim::Happening::Measure { .. }
            )
        })
        .map(|m| m.line)
        .collect();
    let mut removed = vec![0.0; tl.segments.len()];
    let mut air = 0.0;
    for (si, seg) in tl.segments.iter().enumerate() {
        let Some(t) = seg.tool else { continue };
        let tool = s.tool(t).ok_or(RemovalError::NoTool { line: seg.line, t })?;
        let (r, corner, probe) = match tool.cut {
            Cut::Mill { r, corner } => (r, corner, false),
            Cut::Probe { r } => (r, r, true),
            Cut::Nose { .. } => {
                return Err(RemovalError::Shape {
                    line: seg.line,
                    why: "a turning insert on a milling map".into(),
                });
            }
        };
        let skip = probe && skip_lines.contains(&seg.line);
        // a probe's earliest contact on this move: (s, node)
        let mut first: Option<(f64, V3)> = None;
        let mut probe_hit: Option<(f64, V3)> = None;
        let p = tip_path(s, seg, t, form)?;
        let (a, b) = (p(0.0), p(1.0));
        // the cells under the move's box, grown by r
        let (mut lo, mut hi) = ([a[0].min(b[0]), a[1].min(b[1])], [a[0].max(b[0]), a[1].max(b[1])]);
        // the move's lowest tip: a line's or a helix's at an end; an arc in another plane (G18,
        // G19) can dip below both, so from samples less the longest chord between them
        let mut lowest = a[2].min(b[2]);
        if let Path::Arc { .. } = seg.path {
            // the arc's xy extent: sample it (its circle's box is within these plus the sag)
            let mut prev = a;
            let mut chord = 0.0f64;
            for k in 0..=64 {
                let x = p(k as f64 / 64.0);
                lo = [lo[0].min(x[0]), lo[1].min(x[1])];
                hi = [hi[0].max(x[0]), hi[1].max(x[1])];
                lowest = lowest.min(x[2]);
                chord = chord.max(
                    ((x[0] - prev[0]).powi(2) + (x[1] - prev[1]).powi(2) + (x[2] - prev[2]).powi(2)).sqrt(),
                );
                prev = x;
            }
            lowest -= chord;
            lo = [lo[0] - step, lo[1] - step];
            hi = [hi[0] + step, hi[1] + step];
        }
        // the widest piece decides the cells to visit
        let reach = tool.body.iter().map(|p| p.0).fold(r, f64::max);
        // the lowest a non-cutting piece starts above the tip
        let lowest_from = tool
            .body
            .iter()
            .skip(1)
            .map(|p| p.1)
            .fold(f64::INFINITY, f64::min);
        if lowest >= map.top {
            if seg.kind != SegKind::Rapid {
                air += seg.t_min;
            }
            continue;
        }
        // no piece above the cutter can come down to the stock's top on this move: then only the
        // cutter changes cells, and none farther than r (the same results, a smaller window)
        let reach = if lowest + lowest_from - 1e-6 >= map.top {
            r
        } else {
            reach
        };
        let i0 = (((lo[0] - reach - map.x0) / step).floor().max(0.0)) as usize;
        let i1 = (((hi[0] + reach - map.x0) / step).ceil().max(0.0) as usize).min(nx - 1);
        let j0 = (((lo[1] - reach - map.y0) / step).floor().max(0.0)) as usize;
        let j1 = (((hi[1] + reach - map.y0) / step).ceil().max(0.0) as usize).min(ny - 1);
        // the tip at one height over the move (exactly: the same bits at both ends of a line;
        // an arc's circle from its joints' columns)
        let flat = form == TipForm::Closed && matches!(seg.path, Path::Line) && a[2] == b[2];
        let circle = if form == TipForm::Closed {
            flat_circle(s, seg, t)
        } else {
            None
        };
        let mut holder: Option<HolderHit> = None;
        let mut vol = 0.0;
        let mut worst: Option<(f64, V3)> = None;
        for j in j0..=j1.min(ny - 1) {
            for i in i0..=i1.min(nx - 1) {
                let (x, y) = map.node(i, j);
                let cur = map.z[j * nx + i];
                if cur <= map.bottom {
                    continue;
                }
                // cells the move cannot change, skipped with the same results: already below the
                // tip's lowest point (no cutter reaches under it, every other piece is above the
                // tip), or, on a line, farther from the path than the widest piece reaches
                if cur <= lowest - 1e-6 {
                    continue;
                }
                if let Path::Line = seg.path
                    && within([a[0], a[1]], [b[0], b[1]], [x, y], reach).is_none()
                {
                    continue;
                }
                let env = |u: f64| -> f64 {
                    let c = p(u);
                    let d = ((x - c[0]).powi(2) + (y - c[1]).powi(2)).sqrt();
                    match profile(r, corner, d) {
                        Some(g) => c[2] + g,
                        None => f64::INFINITY,
                    }
                };
                let low = match seg.path {
                    // a line at one height: the cutter's lowest over the node is at the path's
                    // nearest point (its profile only rises with distance), in closed form
                    Path::Line if flat => {
                        let d = seg_dist([a[0], a[1]], [b[0], b[1]], [x, y]);
                        match profile(r, corner, d) {
                            Some(g) => a[2] + g,
                            None => f64::INFINITY,
                        }
                    }
                    Path::Line => match within([a[0], a[1]], [b[0], b[1]], [x, y], r) {
                        Some((s0, s1)) => convex_min(&env, s0, s1),
                        None => f64::INFINITY,
                    },
                    Path::Arc { .. } if circle.is_some() => {
                        let d = circle.as_ref().unwrap().dist([x, y], [a[0], a[1]], [b[0], b[1]]);
                        match profile(r, corner, d) {
                            Some(g) => a[2] + g,
                            None => f64::INFINITY,
                        }
                    }
                    Path::Arc { sweep, .. } => {
                        let n = ((sweep.abs() / (dmath::PI / 4.0)).ceil() as usize).max(1) * 64;
                        sampled_min(&env, n)
                    }
                };
                if probe {
                    // the ball removes nothing: a contact triggers (skip) or crashes (otherwise)
                    if low < cur - 1e-6 {
                        if skip {
                            let u = first_contact(&env, cur);
                            if first.is_none_or(|(f, _)| u < f) {
                                first = Some((u, [x, y, cur]));
                            }
                        } else if probe_hit.is_none_or(|(d, _)| cur - low > d) {
                            probe_hit = Some((cur - low, [x, y, low]));
                        }
                    }
                }
                let new = if probe { cur } else { low.max(map.bottom) };
                // each non-cutting piece: a flat-bottomed cylinder `from` above the tip
                let after = new.min(cur);
                // no piece above the cutter can reach below the move's lowest tip + its `from`:
                // when the cell is at or under that, none can hit it (the same results, skipped)
                let reachable = after > lowest + lowest_from - 1e-6;
                for (pi, (pr, from, _)) in tool.body.iter().enumerate().skip(1).filter(|_| reachable) {
                    let penv = |u: f64| -> f64 {
                        let c = p(u);
                        let d = ((x - c[0]).powi(2) + (y - c[1]).powi(2)).sqrt();
                        if d <= *pr { c[2] + from } else { f64::INFINITY }
                    };
                    let plow = match seg.path {
                        Path::Line => match within([a[0], a[1]], [b[0], b[1]], [x, y], *pr) {
                            Some((s0, s1)) => penv(s0).min(penv(s1)),
                            None => f64::INFINITY,
                        },
                        Path::Arc { sweep, .. } => sampled_min(
                            &penv,
                            ((sweep.abs() / (dmath::PI / 4.0)).ceil() as usize).max(1) * 64,
                        ),
                    };
                    let dz = after - plow;
                    if dz > 1e-6 && holder.as_ref().is_none_or(|h| dz > h.depth) {
                        holder = Some(HolderHit {
                            line: seg.line,
                            segment: si,
                            piece: pi,
                            depth: dz,
                            at: [x, y, plow],
                        });
                    }
                }
                if new < cur {
                    let dz = cur - new;
                    vol += dz * step * step;
                    map.z[j * nx + i] = new;
                    if seg.kind == SegKind::Rapid && dz > 1e-6 && worst.is_none_or(|(w, _)| dz > w) {
                        worst = Some((dz, [x, y, new]));
                    }
                }
            }
        }
        removed[si] = vol;
        holder_hits.extend(holder);
        if let Some((u, at)) = first {
            triggers.push(Trigger {
                line: seg.line,
                segment: si,
                s: u,
                tip: p(u),
                at,
            });
        }
        if let Some((depth, at)) = probe_hit {
            probe_hits.push(RapidCut {
                line: seg.line,
                segment: si,
                depth,
                at,
            });
        }
        if vol == 0.0 && seg.kind != SegKind::Rapid {
            air += seg.t_min;
        }
        if let Some((depth, at)) = worst {
            rapid_cuts.push(RapidCut {
                line: seg.line,
                segment: si,
                depth,
                at,
            });
        }
    }
    Ok(MillResult {
        map,
        rapid_cuts,
        holder_hits,
        removed,
        air_feed_s: air,
        triggers,
        probe_hits,
    })
}

/// The posted text through the simulation and the milling map with its skip moves settled: each
/// pass's triggers become the next pass's `skip_stops` (so the moves after a skip start at its
/// trigger) until they no longer change: at most one pass more than there are skip moves (a
/// pass can only change the moves after the earliest skip whose stop changed).
pub fn mill_probing(
    s: &Setup,
    text: &str,
    min: V3,
    max: V3,
    step: f64,
) -> Result<(Timeline, MillResult), String> {
    let mut su = s.clone();
    let mut prev: Option<Vec<(usize, f64)>> = None;
    let mut passes = usize::MAX;
    let mut k = 0;
    while k <= passes {
        k += 1;
        let tl = crate::sim::simulate(text, &su).map_err(|e| format!("{e:?}"))?;
        if passes == usize::MAX {
            passes = tl
                .marks
                .iter()
                .filter(|m| {
                    matches!(
                        m.what,
                        crate::sim::Happening::Skip { .. } | crate::sim::Happening::Measure { .. }
                    )
                })
                .count()
                + 1;
        }
        let r = mill(&su, &tl, min, max, step).map_err(|e| format!("{e:?}"))?;
        let stops: Vec<(usize, f64)> = r.triggers.iter().map(|t| (t.line, t.s)).collect();
        if prev.as_ref() == Some(&stops) || stops.is_empty() {
            return Ok((tl, r));
        }
        prev = Some(stops.clone());
        su.skip_stops = stops;
    }
    Err("the skip moves' triggers did not settle".into())
}

/// A turned stock's outer radius along z.
#[derive(Clone, Debug, PartialEq)]
pub struct RadiusMap {
    pub z0: f64,
    pub step: f64,
    pub r: Vec<f64>,
    pub r0: f64,
}

impl RadiusMap {
    pub fn z(&self, i: usize) -> f64 {
        self.z0 + i as f64 * self.step
    }
    pub fn volume(&self) -> f64 {
        self.r.iter().map(|r| dmath::PI * r * r).sum::<f64>() * self.step
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TurnResult {
    pub map: RadiusMap,
    pub rapid_cuts: Vec<RapidCut>,
    pub holder_hits: Vec<HolderHit>,
    pub removed: Vec<f64>,
}

/// Turns a bar of radius r0 from z0 to z1 (part coordinates, the part's axis +Z) through a
/// timeline, on nodes `step` apart. OD work: a node's radius drops to the least radius the nose
/// reaches there. The tool must stay in the part's x–z half-plane (y = 0) at every move's ends.
pub fn turn(
    s: &Setup,
    tl: &Timeline,
    r0: f64,
    z0: f64,
    z1: f64,
    step: f64,
) -> Result<TurnResult, RemovalError> {
    if step.is_nan() || step <= 0.0 || z0.is_nan() || z1.is_nan() || z0 >= z1 || r0.is_nan() || r0 <= 0.0 {
        return Err(RemovalError::Grid(
            "a bar needs r0 > 0, z0 < z1 and a step > 0".into(),
        ));
    }
    let n = ((z1 - z0) / step).floor() as usize + 1;
    let mut map = RadiusMap {
        z0,
        step,
        r: vec![r0; n],
        r0,
    };
    let mut rapid_cuts = Vec::new();
    let mut holder_hits = Vec::new();
    let mut removed = vec![0.0; tl.segments.len()];
    for (si, seg) in tl.segments.iter().enumerate() {
        let Some(t) = seg.tool else { continue };
        let tool = s.tool(t).ok_or(RemovalError::NoTool { line: seg.line, t })?;
        let (rn, cen) = match tool.cut {
            Cut::Nose { rn, centre } => (rn, centre),
            Cut::Mill { .. } => {
                return Err(RemovalError::Shape {
                    line: seg.line,
                    why: "a milling cutter on a turning map".into(),
                });
            }
            Cut::Probe { .. } => {
                return Err(RemovalError::Shape {
                    line: seg.line,
                    why: "a touch probe on a turning map (lathe probing is not modelled)".into(),
                });
            }
        };
        // the nose centre in (r, z)
        let centre = |u: f64| -> [f64; 2] {
            let (tip, _) = s.tip_in_part(&seg.at(u), t).unwrap();
            // the offset is along the part's x (the turret's X), then the distance from the axis
            let cx = tip[0] + cen[0];
            [(cx * cx + tip[1] * tip[1]).sqrt(), tip[2] + cen[1]]
        };
        let (a, b) = (centre(0.0), centre(1.0));
        if a[0] - rn >= map.r0 && b[0] - rn >= map.r0 && seg.path == Path::Line {
            continue;
        }
        let (zl, zh) = (a[1].min(b[1]) - rn, a[1].max(b[1]) + rn);
        let (zl, zh) = if let Path::Arc { .. } = seg.path {
            let mut l = zl;
            let mut h = zh;
            for k in 0..=64 {
                let c = centre(k as f64 / 64.0);
                l = l.min(c[1] - rn);
                h = h.max(c[1] + rn);
            }
            (l, h)
        } else {
            (zl, zh)
        };
        let i0 = (((zl - z0) / step).floor().max(0.0)) as usize;
        let i1 = (((zh - z0) / step).ceil().max(0.0) as usize).min(n - 1);
        let mut vol = 0.0;
        let mut worst: Option<(f64, V3)> = None;
        for i in i0..=i1 {
            let z = map.z(i);
            let env = |u: f64| -> f64 {
                let c = centre(u);
                let dz = z - c[1];
                if dz.abs() > rn {
                    f64::INFINITY
                } else {
                    c[0] - (rn * rn - dz * dz).sqrt()
                }
            };
            let low = match seg.path {
                Path::Line => {
                    // |z − zc(s)| ≤ rn: an s-range, linear in s
                    let (za, zb) = (a[1], b[1]);
                    let (s0, s1) = if za == zb {
                        if (z - za).abs() > rn {
                            continue;
                        }
                        (0.0, 1.0)
                    } else {
                        let u0 = (z - rn - za) / (zb - za);
                        let u1 = (z + rn - za) / (zb - za);
                        (u0.min(u1).max(0.0), u0.max(u1).min(1.0))
                    };
                    if s0 > s1 {
                        continue;
                    }
                    convex_min(&env, s0, s1)
                }
                Path::Arc { sweep, .. } => sampled_min(
                    &env,
                    ((sweep.abs() / (dmath::PI / 4.0)).ceil() as usize).max(1) * 64,
                ),
            };
            let new = low.max(0.0);
            let cur = map.r[i];
            if new < cur {
                vol += dmath::PI * (cur * cur - new * new) * step;
                map.r[i] = new;
                let d = cur - new;
                if seg.kind == SegKind::Rapid && d > 1e-6 && worst.is_none_or(|(w, _)| d > w) {
                    worst = Some((d, [new, 0.0, z]));
                }
            }
        }
        removed[si] = vol;
        if let Some((depth, at)) = worst {
            rapid_cuts.push(RapidCut {
                line: seg.line,
                segment: si,
                depth,
                at,
            });
        }
        // each non-cutting piece, radial from the tip: material above `tip_r + from` within ±r
        // of the tip's z touches it (sampled along the move: 33 points)
        let mut hit: Option<HolderHit> = None;
        for k in 0..=32 {
            let (tip, _) = s.tip_in_part(&seg.at(k as f64 / 32.0), t).unwrap();
            let tr = (tip[0] * tip[0] + tip[1] * tip[1]).sqrt();
            for (pi, (pr, from, _)) in tool.body.iter().enumerate().skip(1) {
                let (zl, zh) = (tip[2] - pr, tip[2] + pr);
                for i in 0..n {
                    let z = map.z(i);
                    if z < zl || z > zh {
                        continue;
                    }
                    let d = map.r[i] - (tr + from);
                    if d > 1e-6 && hit.as_ref().is_none_or(|h| d > h.depth) {
                        hit = Some(HolderHit {
                            line: seg.line,
                            segment: si,
                            piece: pi,
                            depth: d,
                            at: [tr + from, 0.0, z],
                        });
                    }
                }
            }
        }
        holder_hits.extend(hit);
    }
    Ok(TurnResult {
        map,
        rapid_cuts,
        holder_hits,
        removed,
    })
}
