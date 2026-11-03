//! Direct construction of a sheet part from a chain of walls and bends, with a flat region as its
//! trim: the region is cut at the bends' tangent lines, each strip's faces traced, and the one
//! topology laid out through either map (folded, or flat). No booleans, nothing recognised.
//!
//! x runs across the bends in the flat, y along them, w depth from the bends' inside face. Strip k
//! is a wall if k is even, a bend if odd. Formulas: prior-art.md 3.1.
use crate::region::Piece;
use forge_geom::ruled::{Rail, Ruled};
use forge_geom::wrap::{Flat, Wrap};
use forge_geom::{Circle, Curve, CurveGeom, Cylinder, Line, Plane, Surface, SurfaceGeom};
use forge_math::{Frame, Interval, Point2, Point3, Vec3, dmath, tol};
use forge_topo::{Body, EdgeId, FaceId, Name, VertexId};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bend {
    pub radius: f64,
    pub angle: f64,
    /// The bend allowance, from whatever rule the shop uses; Rn = ba / angle.
    pub ba: f64,
}

/// Walls by their flat length between tangent lines (or to the free end), bends between them.
#[derive(Clone, Debug, PartialEq)]
pub struct Chain {
    pub t: f64,
    pub walls: Vec<f64>,
    pub bends: Vec<Bend>,
}

/// Why a part cannot be built, with the entity and the numbers.
#[derive(Clone, Debug, PartialEq)]
pub enum BuildError {
    /// `walls` must be one more than `bends`.
    ChainShape { walls: usize, bends: usize },
    /// A wall left with no length once setbacks are taken.
    WallTooShort { wall: usize, length: f64, limit: f64 },
    /// A bend's values out of range.
    BendRange {
        bend: usize,
        radius: f64,
        angle: f64,
        ba: f64,
    },
    /// A straight trim edge lies along tangent line `line` (flat x), from y0 to y1.
    AlongBendLine { line: usize, x: f64, y0: f64, y1: f64 },
    /// A trim touches tangent line `line` without crossing it.
    TouchesBendLine {
        line: usize,
        at: Point2,
        gap: f64,
        limit: f64,
    },
    /// A trim curve kind that this construction does not cross bends with.
    SplineTrim { loop_index: usize, piece: usize },
    /// An odd number of crossings on a tangent line: the loops are not closed regions.
    OddCrossings { line: usize, count: usize },
    /// A strip face with no outer loop for a hole (at the hole's point).
    HoleOutside { strip: usize, at: Point2 },
    /// A wall that folds on itself (a kernel ruled surface refused it).
    WallFolds {
        loop_index: usize,
        piece: usize,
        strip: usize,
    },
    /// A trim edge too short to carry a curve.
    EdgeTooShort {
        loop_index: usize,
        piece: usize,
        length: f64,
        limit: f64,
    },
}

const TOL: f64 = tol::LINEAR;

/// Which world the topology goes in.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Map {
    Folded,
    Flat,
}

/// A strip, placed. `x0..x1` in flat x.
#[derive(Clone, Copy, Debug)]
pub struct Strip {
    pub x0: f64,
    pub x1: f64,
    pub kind: StripKind,
}

#[derive(Clone, Copy, Debug)]
pub enum StripKind {
    /// Flat x = x0 is the frame's origin; local (x - x0, y, -w).
    Plane { frame: Frame },
    /// Round the axis frame: s = x - x0, h = -y; radii R + w.
    Round {
        axis: Frame,
        r: f64,
        unroll: f64,
        angle: f64,
    },
}

impl Strip {
    pub fn point(&self, x: f64, y: f64, w: f64) -> Point3 {
        match self.kind {
            StripKind::Plane { frame } => frame.point_to_world(Point3::new(x - self.x0, y, -w)),
            StripKind::Round { axis, r, unroll, .. } => {
                let rho = r + w;
                let (sn, cs) = dmath::sin_cos((x - self.x0) / unroll);
                axis.point_to_world(Point3::new(rho * cs, rho * sn, -y))
            }
        }
    }
    /// The inverse of `point`: (x, y, w).
    pub fn unfold(&self, p: Point3) -> (f64, f64, f64) {
        match self.kind {
            StripKind::Plane { frame } => {
                let l = frame.point_to_local(p);
                (l.x + self.x0, l.y, -l.z)
            }
            StripKind::Round { axis, r, unroll, .. } => {
                let l = axis.point_to_local(p);
                let rho = (l.x * l.x + l.y * l.y).sqrt();
                (self.x0 + unroll * dmath::atan2(l.y, l.x), -l.z, rho - r)
            }
        }
    }
}

/// The strips of a chain under a map: the fold map's placements, composed along the chain.
pub fn layout(c: &Chain, map: Map) -> Result<Vec<Strip>, BuildError> {
    if c.walls.len() != c.bends.len() + 1 {
        return Err(BuildError::ChainShape {
            walls: c.walls.len(),
            bends: c.bends.len(),
        });
    }
    let mut out = Vec::with_capacity(c.walls.len() + c.bends.len());
    let mut frame = Frame::WORLD;
    let mut x = 0.0;
    for (i, &len) in c.walls.iter().enumerate() {
        if len <= TOL {
            return Err(BuildError::WallTooShort {
                wall: i,
                length: len,
                limit: TOL,
            });
        }
        let start = match map {
            Map::Folded => frame,
            Map::Flat => Frame::WORLD.with_origin(Point3::new(x, 0.0, 0.0)),
        };
        out.push(Strip {
            x0: x,
            x1: x + len,
            kind: StripKind::Plane { frame: start },
        });
        x += len;
        if let Some(b) = c.bends.get(i) {
            if !(b.radius > TOL && b.angle > tol::ANGULAR && b.angle < dmath::PI && b.ba > TOL) {
                return Err(BuildError::BendRange {
                    bend: i,
                    radius: b.radius,
                    angle: b.angle,
                    ba: b.ba,
                });
            }
            let (sn, cs) = dmath::sin_cos(b.angle);
            let axis_local = Frame::from_axes(
                Point3::new(len, 0.0, b.radius),
                Vec3::new(0.0, 0.0, -1.0),
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(0.0, -1.0, 0.0),
            )
            .expect("square axes");
            let kind = match map {
                Map::Folded => StripKind::Round {
                    axis: frame.compose(&axis_local),
                    r: b.radius,
                    unroll: b.ba / b.angle,
                    angle: b.angle,
                },
                Map::Flat => StripKind::Plane {
                    frame: Frame::WORLD.with_origin(Point3::new(x, 0.0, 0.0)),
                },
            };
            out.push(Strip {
                x0: x,
                x1: x + b.ba,
                kind,
            });
            x += b.ba;
            let next = Frame::from_axes(
                Point3::new(len + b.radius * sn, 0.0, b.radius * (1.0 - cs)),
                Vec3::new(cs, 0.0, sn),
                Vec3::new(0.0, 1.0, 0.0),
                Vec3::new(-sn, 0.0, cs),
            )
            .expect("square axes");
            frame = frame.compose(&next);
        }
    }
    Ok(out)
}

/// A piece cut to one strip.
#[derive(Clone, Debug)]
struct Cut {
    piece: Piece,
    strip: usize,
}

fn split_at(p: &Piece, lines: &[f64], li: usize, pi: usize) -> Result<Vec<f64>, BuildError> {
    let mut us = Vec::new();
    match *p {
        Piece::Line { a, b } => {
            for (k, &x) in lines.iter().enumerate() {
                if (a.x - x).abs() <= TOL && (b.x - x).abs() <= TOL {
                    return Err(BuildError::AlongBendLine {
                        line: k,
                        x,
                        y0: a.y,
                        y1: b.y,
                    });
                }
                if (a.x < x - TOL && b.x > x + TOL) || (a.x > x + TOL && b.x < x - TOL) {
                    us.push((x - a.x) / (b.x - a.x));
                }
            }
        }
        Piece::Arc { c, r, t0, t1 } => {
            let (lo, hi) = (t0.min(t1), t0.max(t1));
            for (k, &x) in lines.iter().enumerate() {
                let q = (x - c.x) / r;
                if q.abs() > 1.0 + TOL / r {
                    continue;
                }
                // the arc's extreme at this line: it touches without crossing
                let extreme = if q > 0.0 { 0.0 } else { dmath::PI };
                if (1.0 - q.abs()) * r <= TOL {
                    let k0 = ((lo - extreme) / dmath::TAU).ceil();
                    if extreme + k0 * dmath::TAU <= hi {
                        return Err(BuildError::TouchesBendLine {
                            line: k,
                            at: Point2::new(x, c.y),
                            gap: (1.0 - q.abs()) * r,
                            limit: TOL,
                        });
                    }
                    continue;
                }
                let a = dmath::acos_clamped(q);
                for b in [a, -a] {
                    let k0 = ((lo - b) / dmath::TAU).ceil() as i64;
                    let k1 = ((hi - b) / dmath::TAU).floor() as i64;
                    for j in k0..=k1 {
                        us.push((b + dmath::TAU * j as f64 - t0) / (t1 - t0));
                    }
                }
            }
            // an arc turning further than half way goes round in two, so ruled walls can carry on past their ends
            let turns = ((hi - lo) / (dmath::PI * 0.99)).ceil() as usize;
            for i in 1..turns.max(1) {
                us.push(i as f64 / turns as f64);
            }
        }
        Piece::Spline { .. } => {
            return Err(BuildError::SplineTrim {
                loop_index: li,
                piece: pi,
            });
        }
    }
    us.retain(|u| *u > 1e-12 && *u < 1.0 - 1e-12);
    us.sort_by(f64::total_cmp);
    Ok(us)
}

/// One step round a strip face: a piece of a loop, or part of a tangent line.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Step {
    Piece(usize, usize),
    /// Line, the crossing pair on it, upward (true) or downward.
    Joint(usize, usize, bool),
}

/// The curve of a piece at depth w in its strip, its range, and whether the run is with the curve.
fn edge_curve(s: &Strip, p: &Piece, w: f64) -> Option<(Curve, Interval, bool)> {
    let line = |a: Point2, b: Point2| {
        let (pa, pb) = (s.point(a.x, a.y, w), s.point(b.x, b.y, w));
        Some((
            Curve::Line(Line::through(pa, pb)?),
            Interval::new(0.0, pa.distance(pb)),
            true,
        ))
    };
    match (s.kind, p) {
        (StripKind::Plane { .. }, &Piece::Line { a, b }) => line(a, b),
        (StripKind::Plane { frame }, &Piece::Arc { c, r, t0, t1 }) => {
            let o = frame.point_to_world(Point3::new(c.x - s.x0, c.y, -w));
            let (lo, hi) = (t0.min(t1), t0.max(t1));
            Some((
                Curve::Circle(Circle {
                    frame: frame.with_origin(o),
                    radius: r,
                }),
                Interval::new(lo, hi),
                t1 > t0,
            ))
        }
        (StripKind::Round { .. }, &Piece::Line { a, b }) if a.x == b.x => line(a, b),
        (StripKind::Round { axis, r, unroll, .. }, &Piece::Line { a, b }) if a.y == b.y => {
            let (p0, p1) = ((a.x - s.x0) / unroll, (b.x - s.x0) / unroll);
            Some((
                Curve::Circle(Circle {
                    frame: axis.with_origin(axis.point_to_world(Point3::new(0.0, 0.0, -a.y))),
                    radius: r + w,
                }),
                Interval::new(p0.min(p1), p0.max(p1)),
                p1 > p0,
            ))
        }
        (StripKind::Round { axis, r, unroll, .. }, _) => Some((
            Wrap {
                axis,
                radius: r + w,
                unroll,
                flat: flat_of(s.x0, p)?,
            }
            .curve(),
            Interval::new(0.0, 1.0),
            true,
        )),
        _ => None,
    }
}

/// A piece in the bend's (s, h).
fn flat_of(x0: f64, p: &Piece) -> Option<Flat> {
    match *p {
        Piece::Line { a, b } => Some(Flat::Line {
            a: Point2::new(a.x - x0, -a.y),
            b: Point2::new(b.x - x0, -b.y),
        }),
        Piece::Arc { c, r, t0, t1 } => Some(Flat::Arc {
            center: Point2::new(c.x - x0, -c.y),
            radius: r,
            start: -t0,
            sweep: -(t1 - t0),
        }),
        Piece::Spline { .. } => None,
    }
}

/// The surface of the side wall along a piece, between depths 0 and t.
fn side_surface(s: &Strip, p: &Piece, t: f64) -> Option<Surface> {
    match (s.kind, p) {
        (StripKind::Plane { frame }, &Piece::Arc { c, r, .. }) => Some(Surface::Cylinder(Cylinder {
            frame: frame.with_origin(frame.point_to_world(Point3::new(c.x - s.x0, c.y, 0.0))),
            radius: r,
        })),
        (StripKind::Round { axis, r, unroll, .. }, _) if !matches!(p, &Piece::Line { a, b } if a.x == b.x || a.y == b.y) =>
        {
            let wrap = |w: f64| {
                Some(Rail::Wrap(Wrap {
                    axis,
                    radius: r + w,
                    unroll,
                    flat: flat_of(s.x0, p)?,
                }))
            };
            Ruled::new(wrap(t)?, wrap(0.0)?).map(Ruled::surface)
        }
        // a plane: straight along the piece and straight through the sheet
        _ => {
            let (a, m) = (p.at(0.0), p.at(0.5));
            let along = match *p {
                Piece::Line { a, b } => {
                    let (pa, pb) = (s.point(a.x, a.y, 0.0), s.point(b.x, b.y, 0.0));
                    let ma = s.point(m.x, m.y, 0.0);
                    if matches!(s.kind, StripKind::Round { .. }) && a.y == b.y {
                        // round the bend at one y: the plane square to the axis
                        (ma - pa).cross(pb - pa)
                    } else {
                        pb - pa
                    }
                }
                _ => return None,
            };
            let up = s.point(m.x, m.y, 0.0) - s.point(m.x, m.y, t);
            let round_level =
                matches!(s.kind, StripKind::Round { .. }) && matches!(*p, Piece::Line { a, b } if a.y == b.y);
            let normal = if round_level { along } else { along.cross(up) };
            Some(Surface::Plane(Plane::new(Frame::from_zx(
                s.point(a.x, a.y, t),
                normal.normalize()?,
                up,
            )?)))
        }
    }
}

/// A part: the chain, and its flat region's loops (the outline counter-clockwise, then holes).
pub fn build(c: &Chain, loops: &[Vec<Piece>], map: Map, root: Name) -> Result<Body, BuildError> {
    let strips = layout(c, map)?;
    let lines: Vec<f64> = strips[..strips.len() - 1].iter().map(|s| s.x1).collect();
    let strip_of = |x: f64| lines.iter().position(|&q| x < q).unwrap_or(lines.len());
    // cut every piece at the lines
    let mut cuts: Vec<Vec<Cut>> = Vec::with_capacity(loops.len());
    for (li, l) in loops.iter().enumerate() {
        let mut row = Vec::with_capacity(l.len());
        for (pi, p) in l.iter().enumerate() {
            let mut us = vec![0.0];
            us.extend(split_at(p, &lines, li, pi)?);
            us.push(1.0);
            for w in us.windows(2) {
                let q = p.sub(w[0], w[1]);
                row.push(Cut {
                    strip: strip_of(q.at(0.5).x),
                    piece: q,
                });
            }
        }
        cuts.push(row);
    }
    // the crossings on each line, by y: consecutive pairs bound the material
    let mut on: Vec<Vec<(f64, usize, usize)>> = vec![Vec::new(); lines.len()];
    for (l, row) in cuts.iter().enumerate() {
        let n = row.len();
        for k in 0..n {
            let prev = row[(k + n - 1) % n].strip;
            let here = row[k].strip;
            let at = row[k].piece.at(0.0);
            if prev != here {
                on[prev.min(here)].push((at.y, l, k));
            } else {
                // a corner sitting on a line without crossing it
                for (j, &x) in lines.iter().enumerate() {
                    if (at.x - x).abs() <= TOL {
                        return Err(BuildError::TouchesBendLine {
                            line: j,
                            at,
                            gap: (at.x - x).abs(),
                            limit: TOL,
                        });
                    }
                }
            }
        }
    }
    for (j, row) in on.iter_mut().enumerate() {
        if row.len() % 2 == 1 {
            return Err(BuildError::OddCrossings {
                line: j,
                count: row.len(),
            });
        }
        row.sort_by(|a, b| a.0.total_cmp(&b.0));
    }
    let pair_of = |j: usize, l: usize, k: usize| -> (usize, usize) {
        let i = on[j]
            .iter()
            .position(|c| c.1 == l && c.2 == k)
            .expect("a crossing on its line");
        (i / 2, i)
    };
    // trace each strip's faces
    let mut used: Vec<Vec<bool>> = cuts.iter().map(|r| vec![false; r.len()]).collect();
    // per strip: (outer steps, hole step lists)
    let mut faces: Vec<Vec<(Vec<Step>, Vec<Vec<Step>>)>> = vec![Vec::new(); strips.len()];
    let mut holes: Vec<(usize, Vec<Step>)> = Vec::new();
    let flat_at = |st: &Step| -> Vec<Point2> {
        match *st {
            Step::Piece(l, k) => (0..8).map(|i| cuts[l][k].piece.at(i as f64 / 8.0)).collect(),
            Step::Joint(j, pr, up) => {
                let y = if up { on[j][2 * pr].0 } else { on[j][2 * pr + 1].0 };
                vec![Point2::new(lines[j], y)]
            }
        }
    };
    let area_of = |steps: &[Step]| -> f64 {
        let pts: Vec<Point2> = steps.iter().flat_map(&flat_at).collect();
        let n = pts.len();
        (0..n)
            .map(|i| 0.5 * pts[i].to_vec().cross(pts[(i + 1) % n].to_vec()))
            .sum()
    };
    for (l0, row) in cuts.iter().enumerate() {
        for k0 in 0..row.len() {
            if used[l0][k0] {
                continue;
            }
            let s = row[k0].strip;
            let mut steps = Vec::new();
            let (mut l, mut k) = (l0, k0);
            loop {
                used[l][k] = true;
                steps.push(Step::Piece(l, k));
                let n = cuts[l].len();
                let nx = (k + 1) % n;
                if cuts[l][nx].strip == s {
                    k = nx;
                } else {
                    // on to the other end of this stretch of tangent line, and off into the strip again
                    let j = s.min(cuts[l][nx].strip);
                    let (pair, i) = pair_of(j, l, nx);
                    let right_side = s == j; // the strip is left of the line, so its face runs up it
                    if right_side != (i % 2 == 0) {
                        return Err(BuildError::OddCrossings {
                            line: j,
                            count: on[j].len(),
                        });
                    }
                    steps.push(Step::Joint(j, pair, right_side));
                    let other = i ^ 1;
                    let (_, l2, k2) = on[j][other];
                    l = l2;
                    k = k2;
                }
                if l == l0 && k == k0 {
                    break;
                }
            }
            if area_of(&steps) > 0.0 {
                faces[s].push((steps, Vec::new()));
            } else {
                holes.push((s, steps));
            }
        }
    }
    for (s, steps) in holes {
        let Step::Piece(l, k) = steps[0] else {
            continue;
        };
        let at = cuts[l][k].piece.at(0.5);
        let found = faces[s].iter().position(|(o, _)| {
            let pts: Vec<Point2> = o.iter().flat_map(&flat_at).collect();
            let n = pts.len();
            let mut odd = false;
            for i in 0..n {
                let (a, b) = (pts[i], pts[(i + 1) % n]);
                if (a.y > at.y) != (b.y > at.y) && a.x + (b.x - a.x) * (at.y - a.y) / (b.y - a.y) > at.x {
                    odd = !odd;
                }
            }
            odd
        });
        let Some(f) = found else {
            return Err(BuildError::HoleOutside { strip: s, at });
        };
        faces[s][f].1.push(steps);
    }
    // the body: level 0 at depth t, level 1 at depth 0 (so "up" is the way out of the bend's inside)
    let depth = [c.t, 0.0];
    let mut body = Body::new();
    let shell = body.add_shell();
    let key = |l: usize, k: usize| Name::NONE.with(l as u64).with(k as u64);
    let verts: Vec<Vec<Vec<VertexId>>> = depth
        .iter()
        .enumerate()
        .map(|(j, &w)| {
            cuts.iter()
                .enumerate()
                .map(|(l, row)| {
                    row.iter()
                        .enumerate()
                        .map(|(k, cut)| {
                            let q = cut.piece.at(0.0);
                            let name = root.with_str("vertex").with(j as u64).with_name(key(l, k));
                            body.add_vertex(strips[cut.strip].point(q.x, q.y, w), name)
                        })
                        .collect()
                })
                .collect()
        })
        .collect();
    let mut edges: [Vec<Vec<(EdgeId, bool)>>; 2] = [Vec::new(), Vec::new()];
    for (j, &w) in depth.iter().enumerate() {
        for (l, row) in cuts.iter().enumerate() {
            let n = row.len();
            let mut out = Vec::with_capacity(n);
            for (k, cut) in row.iter().enumerate() {
                let too_short = || BuildError::EdgeTooShort {
                    loop_index: l,
                    piece: k,
                    length: cut.piece.at(0.0).distance(cut.piece.at(1.0)),
                    limit: TOL,
                };
                let (curve, range, fw) =
                    edge_curve(&strips[cut.strip], &cut.piece, w).ok_or_else(too_short)?;
                let (a, b) = (verts[j][l][k], verts[j][l][(k + 1) % n]);
                let (start, end) = if fw { (a, b) } else { (b, a) };
                let name = root.with_str("section").with(j as u64).with_name(key(l, k));
                out.push((body.add_edge(curve, range, start, end, name), fw));
            }
            edges[j].push(out);
        }
    }
    // the tangent lines' stretches, on each level, lower crossing to upper
    let mut joints: [Vec<Vec<EdgeId>>; 2] = [Vec::new(), Vec::new()];
    for (lev, &w) in depth.iter().enumerate() {
        for (j, row) in on.iter().enumerate() {
            let mut es = Vec::with_capacity(row.len() / 2);
            for (i, pr) in row.chunks_exact(2).enumerate() {
                let (a, b) = (verts[lev][pr[0].1][pr[0].2], verts[lev][pr[1].1][pr[1].2]);
                let (pa, pb) = (body.vertex(a).point, body.vertex(b).point);
                let line = Line::through(pa, pb).ok_or(BuildError::EdgeTooShort {
                    loop_index: pr[0].1,
                    piece: pr[0].2,
                    length: pa.distance(pb),
                    limit: TOL,
                })?;
                let name = root
                    .with_str("joint")
                    .with(lev as u64)
                    .with(j as u64)
                    .with(i as u64);
                es.push(body.add_edge(Curve::Line(line), Interval::new(0.0, pa.distance(pb)), a, b, name));
            }
            let _ = w;
            joints[lev].push(es);
        }
    }
    // the side walls
    for (l, row) in cuts.iter().enumerate() {
        let n = row.len();
        let mut rails = Vec::with_capacity(n);
        for k in 0..n {
            let (p0, p1) = (
                body.vertex(verts[0][l][k]).point,
                body.vertex(verts[1][l][k]).point,
            );
            let line = Line::through(p0, p1).ok_or(BuildError::EdgeTooShort {
                loop_index: l,
                piece: k,
                length: p0.distance(p1),
                limit: TOL,
            })?;
            let name = root.with_str("rail").with_name(key(l, k));
            rails.push(body.add_edge(
                Curve::Line(line),
                Interval::new(0.0, p0.distance(p1)),
                verts[0][l][k],
                verts[1][l][k],
                name,
            ));
        }
        for (k, cut) in row.iter().enumerate() {
            let st = &strips[cut.strip];
            let surface = side_surface(st, &cut.piece, c.t).ok_or(BuildError::WallFolds {
                loop_index: l,
                piece: k,
                strip: cut.strip,
            })?;
            let (e, fw) = edges[0][l][k];
            let ed = body.edge(e);
            let mid = ed.curve.deriv(ed.range.mid());
            let along = if fw { mid.d1 } else { -mid.d1 };
            let m = cut.piece.at(0.5);
            let up = st.point(m.x, m.y, 0.0) - st.point(m.x, m.y, c.t);
            let same = surface
                .normal(surface.project(mid.p))
                .is_none_or(|nat| nat.dot(along.cross(up)) > 0.0);
            let face = body.add_face(shell, surface, same, root.with_str("side").with_name(key(l, k)));
            body.add_loop(
                face,
                &[
                    (e, fw),
                    (rails[(k + 1) % n], true),
                    (edges[1][l][k].0, !edges[1][l][k].1),
                    (rails[k], false),
                ],
            );
        }
    }
    // the two skins of each strip's faces
    for (s, list) in faces.iter().enumerate() {
        for (fi, (outer, inner)) in list.iter().enumerate() {
            for lev in 0..2 {
                let top = lev == 1;
                let (surface, same) = skin(&strips[s], depth[lev], top);
                let name = root
                    .with_str("skin")
                    .with(lev as u64)
                    .with(s as u64)
                    .with(fi as u64);
                let face = body.add_face(shell, surface, same, name);
                let add = |body: &mut Body, face: FaceId, steps: &[Step]| {
                    let mut uses: Vec<(EdgeId, bool)> = steps
                        .iter()
                        .map(|st| match *st {
                            Step::Piece(l, k) => edges[lev][l][k],
                            Step::Joint(j, pr, up) => (joints[lev][j][pr], up),
                        })
                        .collect();
                    if !top {
                        uses.reverse();
                        for u in &mut uses {
                            u.1 = !u.1;
                        }
                    }
                    body.add_loop(face, &uses);
                };
                add(&mut body, face, outer);
                for h in inner {
                    add(&mut body, face, h);
                }
            }
        }
    }
    Ok(body)
}

fn skin(s: &Strip, w: f64, top: bool) -> (Surface, bool) {
    match s.kind {
        StripKind::Plane { frame } => {
            let o = frame.point_to_world(Point3::new(0.0, 0.0, -w));
            (Surface::Plane(Plane::new(frame.with_origin(o))), top)
        }
        // the inside face (depth 0) looks towards the axis
        StripKind::Round { axis, r, .. } => (
            Surface::Cylinder(Cylinder {
                frame: axis,
                radius: r + w,
            }),
            !top,
        ),
    }
}

/// A rectangle in the flat, as loops for `build`.
pub fn rectangle(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<Piece> {
    let p = |x, y| Point2::new(x, y);
    let c = [p(x0, y0), p(x1, y0), p(x1, y1), p(x0, y1)];
    (0..4)
        .map(|i| Piece::Line {
            a: c[i],
            b: c[(i + 1) % 4],
        })
        .collect()
}

/// A circle in the flat, clockwise (a hole), as two arcs.
pub fn hole(c: Point2, r: f64) -> Vec<Piece> {
    let pi = dmath::PI;
    vec![
        Piece::Arc {
            c,
            r,
            t0: 0.0,
            t1: -pi,
        },
        Piece::Arc {
            c,
            r,
            t0: -pi,
            t1: -2.0 * pi,
        },
    ]
}

/// The chain from lengths to the outside mold line: L1..Ln from free edge or apex to apex, K per
/// bend; a wall loses the outside setback at each bend on it.
pub fn from_mold(t: f64, legs: &[f64], radius: f64, angle: f64, k: f64) -> Chain {
    let ossb = (radius + t) * dmath::sin(angle) / (1.0 + dmath::cos(angle));
    let n = legs.len();
    let walls = legs
        .iter()
        .enumerate()
        .map(|(i, l)| l - if i > 0 { ossb } else { 0.0 } - if i + 1 < n { ossb } else { 0.0 })
        .collect();
    let bend = Bend {
        radius,
        angle,
        ba: angle * (radius + k * t),
    };
    Chain {
        t,
        walls,
        bends: vec![bend; n - 1],
    }
}

pub fn total_length(c: &Chain) -> f64 {
    c.walls.iter().sum::<f64>() + c.bends.iter().map(|b| b.ba).sum::<f64>()
}
