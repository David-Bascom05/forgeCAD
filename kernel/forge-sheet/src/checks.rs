//! A sheet against what the shop can make (`ShopProfile`): its thickness stocked, each bend's
//! radius tooled, each flange long enough for the die; each hole and slot far enough from the
//! bends and from the edge, and wide enough; and the hardware pressed in, against its own
//! catalogue row. Violations name the feature and entity with the measured value and the limit;
//! they never change geometry.
//!
//! Distances are taken on the flat, in developed terms: a hole to a bend is measured to the bend
//! zone's edge (its tangent line), 0 when the hole reaches into the zone; a hole to the edge, to
//! the flat's outer contour. A cut that meets the outer contour is a notch, not a hole, and is
//! not checked as one. Mounting holes are checked by their hardware's rules, not the shop's hole
//! rules; bends count as edges for hardware.
use crate::build::{Layout, Map, build_with};
use crate::error::{CutId, JointId, SheetWarning, Side, TrimRef, Violation};
use crate::graph::{JointKind, Sheet, Wall, straight};
use crate::hardware::Source;
use crate::pattern::Curve2;
use crate::region::{Piece, pieces};
use crate::shop::{ShopProfile, Stock};
use forge_geom::spline::Spline;
use forge_math::{Point2, Point3, Vec2, dmath};
use forge_ops::Segment;
use std::sync::Arc;

/// Radii match the tooled ones to this.
const RADIUS_MATCH: f64 = 1e-9;
/// Distances closer than this are a touch.
const TOUCH: f64 = 1e-9;

pub fn check(sheet: &Sheet, shop: &ShopProfile) -> Vec<Violation> {
    let feature = sheet.feature;
    let t = sheet.gauge.thickness;
    let mut out = Vec::new();
    let Some(stock) = shop.stock(t) else {
        out.push(Violation::ThicknessNotStocked {
            feature,
            thickness: t,
            nearest: shop.nearest_thickness(t).unwrap_or(f64::NAN),
        });
        out.extend(mounts(sheet, None));
        return out;
    };
    let min_flange = stock.min_flange.at(t, 0.0);
    for j in &sheet.joints {
        let JointKind::Bend { radius, .. } = j.kind else {
            continue;
        };
        if !stock.radii.iter().any(|r| (r - radius).abs() <= RADIUS_MATCH) {
            let nearest = stock
                .radii
                .iter()
                .copied()
                .min_by(|a, b| (a - radius).abs().total_cmp(&(b - radius).abs()))
                .unwrap_or(f64::NAN);
            out.push(Violation::RadiusNotTooled {
                feature,
                joint: j.id,
                radius,
                nearest,
            });
        }
        for side in [Side::A, Side::B] {
            let (wall, segment) = match side {
                Side::A => (j.a, j.on_a),
                Side::B => (j.b, j.on_b),
            };
            let Some(w) = sheet.wall(wall) else { continue };
            let Ok((p0, p1)) = straight(w, segment) else {
                continue;
            };
            let length = reach(w, p0, p1);
            if length < min_flange {
                out.push(Violation::FlangeTooShort {
                    feature,
                    wall,
                    joint: j.id,
                    length,
                    limit: min_flange,
                });
            }
        }
    }
    out.extend(holes(sheet, stock));
    out.extend(mounts(sheet, Some(stock)));
    out
}

/// Hardware against its catalogue rows, with the stock's hardness when there is a stock.
pub fn mounts(sheet: &Sheet, stock: Option<&Stock>) -> Vec<Violation> {
    let feature = sheet.feature;
    let t = sheet.gauge.thickness;
    let mut out = Vec::new();
    if sheet.mounts.is_empty() {
        return out;
    }
    let flat = Flat::of(sheet);
    for m in &sheet.mounts {
        let p = &m.part;
        if t < p.sheet_min || t > p.sheet_max {
            out.push(Violation::HardwareThickness {
                feature,
                cut: m.cut,
                thickness: t,
                min: p.sheet_min,
                max: p.sheet_max,
            });
        }
        match stock.and_then(|s| s.hardness) {
            None => out.push(Violation::HardnessUnknown {
                feature,
                cut: m.cut,
                limit: p.max_hardness,
            }),
            Some(h) if h > p.max_hardness => out.push(Violation::SheetTooHard {
                feature,
                cut: m.cut,
                hardness: h,
                limit: p.max_hardness,
            }),
            Some(_) => {}
        }
        let Some(flat) = &flat else { continue };
        let Some(place) = flat.place(m.wall) else { continue };
        let c = place.apply(m.at);
        let mut best = (f64::INFINITY, Vec2::new(0.0, 0.0));
        for q in &flat.outer {
            let e = q.at(q.nearest(c));
            let d = c.distance(e);
            if d < best.0 {
                best = (d, unit(c - e));
            }
        }
        for z in &flat.zones {
            let (d, away) = z.to_point(c);
            if d < best.0 {
                best = (d, away);
            }
        }
        if best.0 < p.edge {
            out.push(Violation::HardwareNearEdge {
                feature,
                cut: m.cut,
                distance: best.0,
                limit: p.edge,
                away: place.local_vec(best.1),
            });
        }
    }
    out
}

/// Hardware whose numbers are placeholders.
pub fn warnings(sheet: &Sheet) -> Vec<SheetWarning> {
    sheet
        .mounts
        .iter()
        .filter(|m| m.part.source == Source::Placeholder)
        .map(|m| SheetWarning::UnverifiedHardware {
            feature: sheet.feature,
            cut: m.cut,
        })
        .collect()
}

/// Holes and slots: their width, and their distance from the bends and the edge.
fn holes(sheet: &Sheet, stock: &Stock) -> Vec<Violation> {
    let feature = sheet.feature;
    let t = sheet.gauge.thickness;
    let mut out = Vec::new();
    let mounted: Vec<CutId> = sheet.mounts.iter().map(|m| m.cut).collect();
    let cuts: Vec<_> = sheet.cuts.iter().filter(|c| !mounted.contains(&c.id)).collect();
    if cuts.is_empty() {
        return out;
    }
    let Some(flat) = Flat::of(sheet) else {
        return out;
    };
    for c in cuts {
        let Some(place) = flat.place(c.wall) else { continue };
        let own = pieces(&c.lp);
        let dev: Vec<(Piece, u64)> = own.iter().map(|(p, id)| (moved(p, &place), *id)).collect();
        let trim = |segment: u64| TrimRef::Cut { cut: c.id, segment };
        // a notch: an edge feature, not a hole
        let edge = gap(&dev, &flat.outer);
        if edge.0 <= TOUCH {
            continue;
        }
        // its width: across a round hole, else the widest circle it holds
        match round(&c.lp.segments) {
            Some(r) => {
                let limit = stock.min_hole.at(t, 0.0);
                if 2.0 * r < limit {
                    out.push(Violation::HoleTooSmall {
                        feature,
                        trim: trim(c.lp.segments[0].1),
                        diameter: 2.0 * r,
                        limit,
                        thickness: t,
                    });
                }
            }
            None => {
                let width = 2.0 * inscribed(&own);
                let limit = stock.min_slot.at(t, 0.0);
                if width < limit {
                    out.push(Violation::SlotTooNarrow {
                        feature,
                        trim: trim(c.lp.segments[0].1),
                        width,
                        limit,
                        thickness: t,
                    });
                }
            }
        }
        let limit = stock.hole_to_edge.at(t, 0.0);
        if edge.0 < limit {
            out.push(Violation::HoleNearEdge {
                feature,
                trim: trim(edge.3),
                distance: edge.0,
                limit,
                away: place.local_vec(unit(edge.1 - edge.2)),
            });
        }
        for z in &flat.zones {
            let limit = stock.hole_to_bend.at(t, z.radius);
            let (d, segment) = z.to_loop(&dev);
            if d < limit {
                let centre = dev
                    .iter()
                    .map(|p| p.0.at(0.5).to_vec())
                    .fold(Vec2::new(0.0, 0.0), |a, b| a + b)
                    * (1.0 / dev.len() as f64);
                out.push(Violation::HoleNearBend {
                    feature,
                    trim: trim(segment),
                    joint: z.joint,
                    distance: d,
                    limit,
                    thickness: t,
                    away: place.local_vec(z.away(Point2::ORIGIN + centre)),
                });
            }
        }
    }
    out
}

/// The radius of a loop that is one circle (or arcs of one).
fn round(segs: &[(Segment, u64)]) -> Option<f64> {
    let mut circle: Option<(Point2, f64)> = None;
    for (s, _) in segs {
        let (c, r) = match s {
            Segment::Circle { center, radius, .. } => (*center, *radius),
            Segment::Arc { center, radius, .. } => (*center, *radius),
            _ => return None,
        };
        match circle {
            None => circle = Some((c, r)),
            Some((c0, r0)) if c0.distance(c) <= TOUCH && (r0 - r).abs() <= TOUCH => {}
            Some(_) => return None,
        }
    }
    circle.map(|c| c.1)
}

/// A strip's place in the developed plane (a copy of `build::Place2`'s motion, with its inverse).
#[derive(Clone, Copy, Debug)]
struct Place {
    o: Point2,
    x: Vec2,
}

impl Place {
    fn y(&self) -> Vec2 {
        Vec2::new(-self.x.y, self.x.x)
    }
    fn apply(&self, p: Point2) -> Point2 {
        self.o + self.x * p.x + self.y() * p.y
    }
    fn local(&self, p: Point2) -> Point2 {
        let d = p - self.o;
        Point2::new(d.dot(self.x), d.dot(self.y()))
    }
    fn local_vec(&self, v: Vec2) -> Vec2 {
        Vec2::new(v.dot(self.x), v.dot(self.y()))
    }
    fn angle(&self) -> f64 {
        dmath::atan2(self.x.y, self.x.x)
    }
}

struct Zone {
    joint: JointId,
    radius: f64,
    place: Place,
    ba: f64,
    len: f64,
    sides: Vec<Piece>,
}

impl Zone {
    fn inside(&self, p: Point2) -> bool {
        let q = self.place.local(p);
        q.x >= 0.0 && q.x <= self.ba && q.y >= 0.0 && q.y <= self.len
    }
    /// The way out of the zone across it, from p's side of its middle.
    fn away(&self, p: Point2) -> Vec2 {
        if self.place.local(p).x < 0.5 * self.ba {
            -self.place.x
        } else {
            self.place.x
        }
    }
    fn to_point(&self, p: Point2) -> (f64, Vec2) {
        if self.inside(p) {
            return (0.0, self.away(p));
        }
        let mut best = (f64::INFINITY, Vec2::new(0.0, 0.0));
        for s in &self.sides {
            let e = s.at(s.nearest(p));
            if p.distance(e) < best.0 {
                best = (p.distance(e), unit(p - e));
            }
        }
        best
    }
    /// A loop's distance from the zone (0 reaching in), and its nearest segment.
    fn to_loop(&self, l: &[(Piece, u64)]) -> (f64, u64) {
        for (p, id) in l {
            if (0..=8).any(|k| self.inside(p.at(k as f64 / 8.0))) {
                return (0.0, *id);
            }
        }
        let sides: Vec<(Piece, u64)> = self.sides.iter().map(|s| (s.clone(), 0)).collect();
        let g = gap(l, &sides.iter().map(|s| s.0.clone()).collect::<Vec<_>>());
        (g.0, g.3)
    }
}

/// The flat: each wall's place, the outer contour and the bend zones, in developed terms.
struct Flat {
    layout: Layout,
    outer: Vec<Piece>,
    zones: Vec<Zone>,
}

impl Flat {
    fn of(sheet: &Sheet) -> Option<Flat> {
        let b = build_with(sheet, Map::Flat).ok()?;
        let pattern = b.pattern?;
        let contours: Vec<Vec<Piece>> = pattern
            .contours
            .iter()
            .map(|c| c.iter().filter_map(piece_of).collect())
            .collect();
        let area = |c: &Vec<Piece>| c.iter().map(Piece::area_term).sum::<f64>().abs();
        let outer = contours
            .into_iter()
            .max_by(|a, b| area(a).total_cmp(&area(b)))
            .unwrap_or_default();
        let zones = b
            .layout
            .bends
            .iter()
            .map(|z| {
                let place = Place {
                    o: z.developed.origin,
                    x: z.developed.x,
                };
                let c = [
                    (0.0, 0.0),
                    (z.allowance, 0.0),
                    (z.allowance, z.length),
                    (0.0, z.length),
                ]
                .map(|(u, v)| place.apply(Point2::new(u, v)));
                Zone {
                    joint: z.joint,
                    radius: z.radius,
                    place,
                    ba: z.allowance,
                    len: z.length,
                    sides: (0..4)
                        .map(|i| Piece::Line {
                            a: c[i],
                            b: c[(i + 1) % 4],
                        })
                        .collect(),
                }
            })
            .collect();
        Some(Flat {
            layout: b.layout,
            outer,
            zones,
        })
    }
    fn place(&self, w: crate::error::WallId) -> Option<Place> {
        self.layout.walls.iter().find(|x| x.wall == w).map(|x| Place {
            o: x.developed.origin,
            x: x.developed.x,
        })
    }
}

fn piece_of(c: &Curve2) -> Option<Piece> {
    Some(match c {
        Curve2::Line { a, b } => Piece::Line { a: *a, b: *b },
        Curve2::Arc { c, r, start, sweep } => Piece::Arc {
            c: *c,
            r: *r,
            t0: *start,
            t1: start + sweep,
        },
        Curve2::Spline {
            degree,
            knots,
            points,
            weights,
        } => {
            let ctrl: Vec<Point3> = points.iter().map(|p| Point3::new(p.x, p.y, 0.0)).collect();
            let s = Spline::from_nurbs(*degree, knots, &ctrl, weights.as_deref())?;
            Piece::Spline {
                s: Arc::new(s),
                t0: knots[0],
                t1: knots[knots.len() - 1],
            }
        }
    })
}

fn moved(p: &Piece, by: &Place) -> Piece {
    match p {
        Piece::Line { a, b } => Piece::Line {
            a: by.apply(*a),
            b: by.apply(*b),
        },
        Piece::Arc { c, r, t0, t1 } => Piece::Arc {
            c: by.apply(*c),
            r: *r,
            t0: t0 + by.angle(),
            t1: t1 + by.angle(),
        },
        Piece::Spline { s, t0, t1 } => Piece::Spline {
            s: Arc::new(s.mapped(|q| {
                let m = by.apply(Point2::new(q.x, q.y));
                Point3::new(m.x, m.y, 0.0)
            })),
            t0: *t0,
            t1: *t1,
        },
    }
}

fn unit(v: Vec2) -> Vec2 {
    let n = v.length();
    if n > 0.0 { v * (1.0 / n) } else { v }
}

/// The least distance between two sets of pieces: (distance, the point on a, the point on b, a's
/// segment id). Alternating nearest points from starts along each piece of a.
fn gap(a: &[(Piece, u64)], b: &[Piece]) -> (f64, Point2, Point2, u64) {
    let mut best = (f64::INFINITY, Point2::ORIGIN, Point2::ORIGIN, 0);
    for (pa, id) in a {
        for pb in b {
            for k in 0..=16 {
                let mut u = k as f64 / 16.0;
                let mut p = pa.at(u);
                let mut q = pb.at(pb.nearest(p));
                for _ in 0..40 {
                    let u2 = pa.nearest(q);
                    let p2 = pa.at(u2);
                    let q2 = pb.at(pb.nearest(p2));
                    let done = (u2 - u).abs() < 1e-14;
                    (u, p, q) = (u2, p2, q2);
                    if done {
                        break;
                    }
                }
                let d = p.distance(q);
                if d < best.0 {
                    best = (d, p, q, *id);
                }
            }
        }
    }
    best
}

/// The radius of the widest circle a loop holds: a grid, then a shrinking pattern search.
fn inscribed(l: &[(Piece, u64)]) -> f64 {
    let poly: Vec<Point2> = l
        .iter()
        .flat_map(|(p, _)| (0..32).map(move |k| p.at(k as f64 / 32.0)))
        .collect();
    let inside = |q: Point2| {
        let mut c = false;
        for i in 0..poly.len() {
            let (a, b) = (poly[i], poly[(i + 1) % poly.len()]);
            if (a.y > q.y) != (b.y > q.y) && q.x < a.x + (q.y - a.y) * (b.x - a.x) / (b.y - a.y) {
                c = !c;
            }
        }
        c
    };
    let clear = |q: Point2| {
        if !inside(q) {
            return 0.0;
        }
        l.iter()
            .map(|(p, _)| q.distance(p.at(p.nearest(q))))
            .fold(f64::INFINITY, f64::min)
    };
    let (mut lo, mut hi) = (
        Point2::new(f64::INFINITY, f64::INFINITY),
        Point2::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
    );
    for p in &poly {
        lo = Point2::new(lo.x.min(p.x), lo.y.min(p.y));
        hi = Point2::new(hi.x.max(p.x), hi.y.max(p.y));
    }
    let n = 40;
    let mut best = (0.0, lo);
    for i in 0..=n {
        for j in 0..=n {
            let q = Point2::new(
                lo.x + (hi.x - lo.x) * i as f64 / n as f64,
                lo.y + (hi.y - lo.y) * j as f64 / n as f64,
            );
            let d = clear(q);
            if d > best.0 {
                best = (d, q);
            }
        }
    }
    let mut step = (hi.x - lo.x).max(hi.y - lo.y) / n as f64;
    while step > 1e-10 * (1.0 + best.0) {
        let mut moved = false;
        for (dx, dy) in [
            (1.0, 0.0),
            (-1.0, 0.0),
            (0.0, 1.0),
            (0.0, -1.0),
            (1.0, 1.0),
            (-1.0, -1.0),
            (1.0, -1.0),
            (-1.0, 1.0),
        ] {
            let q = best.1 + Vec2::new(dx, dy) * step;
            let d = clear(q);
            if d > best.0 {
                best = (d, q);
                moved = true;
            }
        }
        if !moved {
            step *= 0.5;
        }
    }
    best.0
}

/// How far the wall's outline reaches from the mold line p0 -> p1 (the wall on its left).
fn reach(w: &Wall, p0: Point2, p1: Point2) -> f64 {
    let d = (p1 - p0) * (1.0 / p0.distance(p1));
    let left = |q: Point2| d.cross(q - p0);
    w.outline
        .iter()
        .flat_map(|l| &l.segments)
        .flat_map(|(s, _)| s.samples(16))
        .map(left)
        .fold(0.0, f64::max)
}
