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
//! On a straight move the minimised function is convex in s on the range where the cutter covers
//! the node (a convex non-decreasing profile of a convex distance, plus a linear term), so a
//! ternary search finds it to 1e-12 in s; on an arc it is minimised over 64 samples per eighth of
//! a turn, then refined. This is a simulation in floating point at the nodes, not a proof: it is
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

fn tip_path<'a>(s: &'a Setup<'a>, seg: &'a Segment, t: u32) -> Result<impl Fn(f64) -> V3 + 'a, RemovalError> {
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
    Ok(move |u: f64| s.tip_in_part(&seg.at(u), t).unwrap().0)
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
    let mut removed = vec![0.0; tl.segments.len()];
    let mut air = 0.0;
    for (si, seg) in tl.segments.iter().enumerate() {
        let Some(t) = seg.tool else { continue };
        let tool = s.tool(t).ok_or(RemovalError::NoTool { line: seg.line, t })?;
        let (r, corner) = match tool.cut {
            Cut::Mill { r, corner } => (r, corner),
            Cut::Nose { .. } => {
                return Err(RemovalError::Shape {
                    line: seg.line,
                    why: "a turning insert on a milling map".into(),
                });
            }
        };
        let p = tip_path(s, seg, t)?;
        let (a, b) = (p(0.0), p(1.0));
        // the cells under the move's box, grown by r
        let (mut lo, mut hi) = ([a[0].min(b[0]), a[1].min(b[1])], [a[0].max(b[0]), a[1].max(b[1])]);
        if let Path::Arc { .. } = seg.path {
            // the arc's xy extent: sample it (its circle's box is within these plus the sag)
            for k in 0..=64 {
                let x = p(k as f64 / 64.0);
                lo = [lo[0].min(x[0]), lo[1].min(x[1])];
                hi = [hi[0].max(x[0]), hi[1].max(x[1])];
            }
            lo = [lo[0] - step, lo[1] - step];
            hi = [hi[0] + step, hi[1] + step];
        }
        // the widest piece decides the cells to visit
        let reach = tool.body.iter().map(|p| p.0).fold(r, f64::max);
        let lowest = a[2].min(b[2]);
        if lowest >= map.top {
            if seg.kind != SegKind::Rapid {
                air += seg.t_min;
            }
            continue;
        }
        let i0 = (((lo[0] - reach - map.x0) / step).floor().max(0.0)) as usize;
        let i1 = (((hi[0] + reach - map.x0) / step).ceil().max(0.0) as usize).min(nx - 1);
        let j0 = (((lo[1] - reach - map.y0) / step).floor().max(0.0)) as usize;
        let j1 = (((hi[1] + reach - map.y0) / step).ceil().max(0.0) as usize).min(ny - 1);
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
                let env = |u: f64| -> f64 {
                    let c = p(u);
                    let d = ((x - c[0]).powi(2) + (y - c[1]).powi(2)).sqrt();
                    match profile(r, corner, d) {
                        Some(g) => c[2] + g,
                        None => f64::INFINITY,
                    }
                };
                let low = match seg.path {
                    Path::Line => match within([a[0], a[1]], [b[0], b[1]], [x, y], r) {
                        Some((s0, s1)) => convex_min(&env, s0, s1),
                        None => f64::INFINITY,
                    },
                    Path::Arc { sweep, .. } => {
                        let n = ((sweep.abs() / (dmath::PI / 4.0)).ceil() as usize).max(1) * 64;
                        sampled_min(&env, n)
                    }
                };
                let new = low.max(map.bottom);
                // each non-cutting piece: a flat-bottomed cylinder `from` above the tip
                let after = new.min(cur);
                for (pi, (pr, from, _)) in tool.body.iter().enumerate().skip(1) {
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
    })
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
