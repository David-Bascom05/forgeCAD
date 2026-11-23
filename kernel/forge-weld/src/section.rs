//! Sections: the cross-section a member is swept with, as exact loops of lines and arcs in the
//! section's own plane (x to the member's left, y up), and the measures a frame reads off them
//! exactly: the support function (how far the section reaches in a direction), its bounding
//! radius, box, area and centroid.

use forge_math::{Point2, Vec2, dmath};
use forge_ops::{Profile, ProfileLoop, Segment};

/// What a section is, for the cut list and the defaults that follow from it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Square or rectangular hollow section.
    Rhs,
    /// Circular hollow section.
    Chs,
    /// Angle, equal or unequal legs.
    Angle,
    /// Channel with parallel flanges.
    Channel,
    /// I-beam with parallel flanges.
    Beam,
    /// Flat bar.
    Flat,
    /// A section drawn in a sketch.
    Sketched,
}

impl Kind {
    pub fn word(self) -> &'static str {
        match self {
            Kind::Rhs => "rhs",
            Kind::Chs => "chs",
            Kind::Angle => "angle",
            Kind::Channel => "channel",
            Kind::Beam => "beam",
            Kind::Flat => "flat",
            Kind::Sketched => "sketch",
        }
    }
}

/// A section: its outline first (counter-clockwise), then its holes (clockwise), in the
/// section's own coordinates.
#[derive(Clone, Debug, PartialEq)]
pub struct Section {
    pub name: String,
    pub kind: Kind,
    pub loops: Vec<Vec<Segment>>,
}

/// Why a section cannot be made.
#[derive(Clone, Debug, PartialEq)]
pub enum SectionError {
    /// A designation the library does not hold.
    UnknownSize(String),
    /// A size that makes no section (a wall thicker than half the width, a radius that does
    /// not fit): which size, and why.
    BadSize(&'static str),
    /// A section drawn with a spline or ellipse: a member's walls are exact only for lines
    /// and arcs.
    Curve,
}

/// Whether angle `a` lies on the arc starting at `start` sweeping `sweep` (either sense).
fn on_arc(a: f64, start: f64, sweep: f64) -> bool {
    let (lo, w) = if sweep >= 0.0 {
        (start, sweep)
    } else {
        (start + sweep, -sweep)
    };
    (a - lo).rem_euclid(dmath::TAU) <= w
}

/// The segment's end points and, for arcs and circles, (centre, radius, start, sweep).
fn arc_of(s: &Segment) -> Option<(Point2, f64, f64, f64)> {
    match *s {
        Segment::Arc {
            center,
            radius,
            start,
            sweep,
        } => Some((center, radius, start, sweep)),
        Segment::Circle { center, radius, ccw } => {
            Some((center, radius, 0.0, if ccw { dmath::TAU } else { -dmath::TAU }))
        }
        _ => None,
    }
}

/// The loop a closed polygon makes with each corner rounded by its radius (0: sharp). The
/// points run counter-clockwise; a corner turning left is rounded convex, one turning right
/// concave, each arc tangent to both sides.
pub fn rounded_polygon(pts: &[(Point2, f64)]) -> Result<Vec<Segment>, SectionError> {
    let n = pts.len();
    // where each corner's rounding meets the side before and the side after
    let mut ends = Vec::with_capacity(n);
    for i in 0..n {
        let (v, r) = pts[i];
        let p = pts[(i + n - 1) % n].0;
        let q = pts[(i + 1) % n].0;
        let (Some(u), Some(w)) = ((v - p).normalize(), (q - v).normalize()) else {
            return Err(SectionError::BadSize("two corners in one place"));
        };
        if r <= 0.0 {
            ends.push((v, v, None));
            continue;
        }
        let turn = dmath::atan2(u.cross(w), u.dot(w));
        if turn.abs() < 1e-12 {
            return Err(SectionError::BadSize("a rounded corner with no turn"));
        }
        let (hs, hc) = dmath::sin_cos(turn.abs() / 2.0);
        let d = r * hs / hc;
        let (t1, t2) = (v - u * d, v + w * d);
        // the centre: to the left of the side for a left turn, to its right for a right one
        let n1 = if turn > 0.0 { u.perp() } else { -u.perp() };
        let c = t1 + n1 * r;
        let a0 = dmath::atan2(t1.y - c.y, t1.x - c.x);
        ends.push((t1, t2, Some((c, r, a0, turn))));
    }
    let mut segs = Vec::new();
    for i in 0..n {
        let (_, t2, _) = ends[i];
        let (t1n, _, _) = ends[(i + 1) % n];
        if let Some((c, r, a0, sw)) = ends[i].2 {
            segs.push(Segment::Arc {
                center: c,
                radius: r,
                start: a0,
                sweep: sw,
            });
        }
        let len = (t1n - t2).length();
        if len < -1e-12 {
            return Err(SectionError::BadSize("corner radii larger than their sides"));
        }
        // the side from this corner's rounding to the next's; the roundings must leave it
        // running forwards
        let side = pts[(i + 1) % n].0 - pts[i].0;
        if (t1n - t2).dot(side) < -1e-9 {
            return Err(SectionError::BadSize("corner radii larger than their sides"));
        }
        if len > 1e-12 {
            segs.push(Segment::Line { a: t2, b: t1n });
        }
    }
    Ok(segs)
}

fn reversed(l: &[Segment]) -> Vec<Segment> {
    l.iter().rev().map(|s| s.reversed()).collect()
}

fn rect_pts(x0: f64, y0: f64, x1: f64, y1: f64, r: f64) -> Vec<(Point2, f64)> {
    vec![
        (Point2::new(x0, y0), r),
        (Point2::new(x1, y0), r),
        (Point2::new(x1, y1), r),
        (Point2::new(x0, y1), r),
    ]
}

fn positive(v: f64, what: &'static str) -> Result<f64, SectionError> {
    if v.is_finite() && v > 0.0 {
        Ok(v)
    } else {
        Err(SectionError::BadSize(what))
    }
}

/// The outer and inner corner radii EN 10219-2 gives a cold-formed hollow section of wall `t`.
pub fn cold_formed_radii(t: f64) -> (f64, f64) {
    if t <= 6.0 {
        (2.0 * t, t)
    } else if t <= 10.0 {
        (2.5 * t, 1.5 * t)
    } else {
        (3.0 * t, 2.0 * t)
    }
}

impl Section {
    /// A rectangular hollow section `w` wide (x) and `h` high (y), wall `t`, outer and inner
    /// corner radii `ro`, `ri`; centred on its box.
    pub fn rhs(name: String, w: f64, h: f64, t: f64, ro: f64, ri: f64) -> Result<Section, SectionError> {
        positive(w, "w")?;
        positive(h, "h")?;
        positive(t, "t")?;
        if 2.0 * t >= w.min(h) {
            return Err(SectionError::BadSize("a wall thicker than half the section"));
        }
        if ro < 0.0 || ri < 0.0 || 2.0 * ro > w.min(h) || 2.0 * ri > w.min(h) - 2.0 * t {
            return Err(SectionError::BadSize("a corner radius that does not fit"));
        }
        let (a, b) = (w / 2.0, h / 2.0);
        let outer = rounded_polygon(&rect_pts(-a, -b, a, b, ro))?;
        let inner = rounded_polygon(&rect_pts(-a + t, -b + t, a - t, b - t, ri))?;
        Ok(Section {
            name,
            kind: Kind::Rhs,
            loops: vec![outer, reversed(&inner)],
        })
    }

    /// A circular hollow section of outside diameter `d`, wall `t`, centred.
    pub fn chs(name: String, d: f64, t: f64) -> Result<Section, SectionError> {
        positive(d, "d")?;
        positive(t, "t")?;
        let r = d / 2.0;
        let mut loops = vec![vec![Segment::Circle {
            center: Point2::ORIGIN,
            radius: r,
            ccw: true,
        }]];
        if t < r {
            loops.push(vec![Segment::Circle {
                center: Point2::ORIGIN,
                radius: r - t,
                ccw: false,
            }]);
        } else if t > r {
            return Err(SectionError::BadSize("a wall thicker than the radius"));
        }
        Ok(Section {
            name,
            kind: Kind::Chs,
            loops,
        })
    }

    /// A flat bar `w` wide (x), `t` thick (y), centred.
    pub fn flat(name: String, w: f64, t: f64) -> Result<Section, SectionError> {
        positive(w, "w")?;
        positive(t, "t")?;
        Ok(Section {
            name,
            kind: Kind::Flat,
            loops: vec![rounded_polygon(&rect_pts(
                -w / 2.0,
                -t / 2.0,
                w / 2.0,
                t / 2.0,
                0.0,
            ))?],
        })
    }

    /// An angle: leg `a` up (y), leg `b` across (x), both `t` thick, the heel at the box's
    /// bottom left; root radius `r1` inside the corner, toe radius `r2` on each leg's inner
    /// tip. Centred on its box.
    pub fn angle(name: String, a: f64, b: f64, t: f64, r1: f64, r2: f64) -> Result<Section, SectionError> {
        positive(a, "a")?;
        positive(b, "b")?;
        positive(t, "t")?;
        if t >= a.min(b) {
            return Err(SectionError::BadSize("legs no longer than their thickness"));
        }
        if r1 < 0.0 || r2 < 0.0 || r2 > t || r1 + r2 > a.min(b) - t {
            return Err(SectionError::BadSize("a radius that does not fit"));
        }
        let (x0, y0) = (-b / 2.0, -a / 2.0);
        let p = |x: f64, y: f64, r: f64| (Point2::new(x0 + x, y0 + y), r);
        let outer = rounded_polygon(&[
            p(0.0, 0.0, 0.0),
            p(b, 0.0, 0.0),
            p(b, t, r2),
            p(t, t, r1),
            p(t, a, r2),
            p(0.0, a, 0.0),
        ])?;
        Ok(Section {
            name,
            kind: Kind::Angle,
            loops: vec![outer],
        })
    }

    /// A channel with parallel flanges: `h` high (y), flanges `b` wide running to +x from the
    /// web, web `tw`, flanges `tf`, root radius `r`. Centred on its box.
    pub fn channel(name: String, h: f64, b: f64, tw: f64, tf: f64, r: f64) -> Result<Section, SectionError> {
        positive(h, "h")?;
        positive(b, "b")?;
        positive(tw, "tw")?;
        positive(tf, "tf")?;
        if tw >= b || 2.0 * tf >= h {
            return Err(SectionError::BadSize("a web or flange thicker than the section"));
        }
        if r < 0.0 || r > b - tw || 2.0 * r > h - 2.0 * tf {
            return Err(SectionError::BadSize("a root radius that does not fit"));
        }
        let (x0, y0) = (-b / 2.0, -h / 2.0);
        let p = |x: f64, y: f64, r: f64| (Point2::new(x0 + x, y0 + y), r);
        let outer = rounded_polygon(&[
            p(0.0, 0.0, 0.0),
            p(b, 0.0, 0.0),
            p(b, tf, 0.0),
            p(tw, tf, r),
            p(tw, h - tf, r),
            p(b, h - tf, 0.0),
            p(b, h, 0.0),
            p(0.0, h, 0.0),
        ])?;
        Ok(Section {
            name,
            kind: Kind::Channel,
            loops: vec![outer],
        })
    }

    /// An I-beam with parallel flanges: `h` high (y, the web up), flanges `b` wide, web `tw`,
    /// flanges `tf`, root radius `r`. Centred.
    pub fn beam(name: String, h: f64, b: f64, tw: f64, tf: f64, r: f64) -> Result<Section, SectionError> {
        positive(h, "h")?;
        positive(b, "b")?;
        positive(tw, "tw")?;
        positive(tf, "tf")?;
        if tw >= b || 2.0 * tf >= h {
            return Err(SectionError::BadSize("a web or flange thicker than the section"));
        }
        if r < 0.0 || 2.0 * r > b - tw || 2.0 * r > h - 2.0 * tf {
            return Err(SectionError::BadSize("a root radius that does not fit"));
        }
        let (hb, hh, hw) = (b / 2.0, h / 2.0, tw / 2.0);
        let p = |x: f64, y: f64, r: f64| (Point2::new(x, y), r);
        let outer = rounded_polygon(&[
            p(-hb, -hh, 0.0),
            p(hb, -hh, 0.0),
            p(hb, -hh + tf, 0.0),
            p(hw, -hh + tf, r),
            p(hw, hh - tf, r),
            p(hb, hh - tf, 0.0),
            p(hb, hh, 0.0),
            p(-hb, hh, 0.0),
            p(-hb, hh - tf, 0.0),
            p(-hw, hh - tf, r),
            p(-hw, -hh + tf, r),
            p(-hb, -hh + tf, 0.0),
        ])?;
        Ok(Section {
            name,
            kind: Kind::Beam,
            loops: vec![outer],
        })
    }

    /// A section from a sketch's loops, outline first (any sense: each is turned to the
    /// section's), lines and arcs only.
    pub fn sketched(name: String, loops: Vec<Vec<Segment>>) -> Result<Section, SectionError> {
        if loops.iter().flatten().any(|s| matches!(s, Segment::Spline(_))) {
            return Err(SectionError::Curve);
        }
        let mut out = Vec::with_capacity(loops.len());
        for (k, l) in loops.into_iter().enumerate() {
            let a = loop_area(&l);
            let ccw = a > 0.0;
            out.push(if ccw == (k == 0) { l } else { reversed(&l) });
        }
        Ok(Section {
            name,
            kind: Kind::Sketched,
            loops: out,
        })
    }

    /// Whether a point is material: inside the outline and outside every hole (crossings of a ray
    /// along +x with the loops, counted; lines and arcs exactly).
    pub fn contains(&self, p: Point2) -> bool {
        let mut inside = false;
        for seg in self.loops.iter().flatten() {
            if let Some((c, r, st, sw)) = arc_of(seg) {
                let dy = p.y - c.y;
                if dy.abs() >= r {
                    continue;
                }
                let dx = (r * r - dy * dy).sqrt();
                for x in [c.x - dx, c.x + dx] {
                    // a crossing at the arc's ends counted once: half-open in angle
                    let a = dmath::atan2(dy, x - c.x);
                    if x > p.x && on_arc(a, st, sw) {
                        inside = !inside;
                    }
                }
            } else {
                let (a, b) = (seg.start(), seg.end());
                if (a.y > p.y) != (b.y > p.y) {
                    let x = a.x + (p.y - a.y) * (b.x - a.x) / (b.y - a.y);
                    if x > p.x {
                        inside = !inside;
                    }
                }
            }
        }
        inside
    }

    /// Its outline alone: the solid the member fills (its envelope).
    pub fn outline(&self) -> Section {
        Section {
            name: self.name.clone(),
            kind: self.kind,
            loops: self.loops.iter().take(1).cloned().collect(),
        }
    }

    /// The section moved so that `p` is its origin, turned by `turn` about it, and with
    /// `mirror` flipped in its y axis (x to -x) first.
    pub fn placed(&self, p: Point2, turn: f64, mirror: bool) -> Section {
        let (s, c) = dmath::sin_cos(turn);
        let map = |q: Point2| {
            let x = q.x - p.x;
            let y = q.y - p.y;
            let x = if mirror { -x } else { x };
            Point2::new(c * x - s * y, s * x + c * y)
        };
        let seg = |sg: &Segment| -> Segment {
            match *sg {
                Segment::Line { a, b } => Segment::Line { a: map(a), b: map(b) },
                Segment::Arc {
                    center,
                    radius,
                    start,
                    sweep,
                } => {
                    let (st, sw) = if mirror {
                        (dmath::PI - start, -sweep)
                    } else {
                        (start, sweep)
                    };
                    Segment::Arc {
                        center: map(center),
                        radius,
                        start: st + turn,
                        sweep: sw,
                    }
                }
                Segment::Circle { center, radius, ccw } => Segment::Circle {
                    center: map(center),
                    radius,
                    ccw: ccw != mirror,
                },
                Segment::Spline(ref sp) => Segment::Spline(sp.clone()),
            }
        };
        let loops = self
            .loops
            .iter()
            .map(|l| {
                let m: Vec<Segment> = l.iter().map(seg).collect();
                // a mirror turns each loop's sense: back to outline ccw, holes cw
                if mirror { reversed(&m) } else { m }
            })
            .collect();
        Section {
            name: self.name.clone(),
            kind: self.kind,
            loops,
        }
    }

    /// How far the outline reaches along `w`: the largest p . w over the section (exact: the
    /// loops' ends, and each arc's farthest point where `w` points into its sweep).
    pub fn support(&self, w: Vec2) -> f64 {
        let mut best = f64::NEG_INFINITY;
        let wl = w.length();
        let wa = dmath::atan2(w.y, w.x);
        for s in self.loops.first().into_iter().flatten() {
            best = best.max(s.start().to_vec().dot(w)).max(s.end().to_vec().dot(w));
            if let Some((c, r, st, sw)) = arc_of(s)
                && wl > 0.0
                && on_arc(wa, st, sw)
            {
                best = best.max(c.to_vec().dot(w) + r * wl);
            }
        }
        best
    }

    /// The largest distance of the section from its origin (its bounding radius).
    pub fn radius(&self) -> f64 {
        let mut best: f64 = 0.0;
        for s in self.loops.first().into_iter().flatten() {
            best = best
                .max(s.start().to_vec().length())
                .max(s.end().to_vec().length());
            if let Some((c, r, st, sw)) = arc_of(s) {
                let cl = c.to_vec().length();
                // the farthest point is away from the origin through the centre (any point of a
                // circle centred on it)
                let a = if cl > 0.0 { dmath::atan2(c.y, c.x) } else { st };
                if cl == 0.0 || on_arc(a, st, sw) {
                    best = best.max(cl + r);
                }
            }
        }
        best
    }

    /// The section's box: (least x, least y), (greatest x, greatest y).
    pub fn bbox(&self) -> (Point2, Point2) {
        let ex = |w: Vec2| self.support(w);
        (
            Point2::new(-ex(Vec2::new(-1.0, 0.0)), -ex(Vec2::new(0.0, -1.0))),
            Point2::new(ex(Vec2::new(1.0, 0.0)), ex(Vec2::new(0.0, 1.0))),
        )
    }

    /// The material's area (holes taken off).
    pub fn area(&self) -> f64 {
        self.loops.iter().map(|l| loop_area(l)).sum()
    }

    /// The material's centroid, exact for lines and arcs.
    pub fn centroid(&self) -> Point2 {
        let (mut a, mut mx, mut my) = (0.0, 0.0, 0.0);
        for s in self.loops.iter().flatten() {
            let (da, dx, dy) = moments(s);
            a += da;
            mx += dx;
            my += dy;
        }
        Point2::new(mx / a, my / a)
    }

    /// The kernel's profile of the section on `plane`, its segments numbered from `first`.
    pub fn profile(&self, plane: forge_math::Frame, first: u64) -> Profile {
        let mut id = first;
        let loops = self
            .loops
            .iter()
            .map(|l| {
                ProfileLoop::new(
                    l.iter()
                        .map(|s| {
                            id += 1;
                            (s.clone(), id - 1)
                        })
                        .collect(),
                )
            })
            .collect();
        Profile { plane, loops }
    }
}

fn loop_area(l: &[Segment]) -> f64 {
    l.iter().map(|s| moments(s).0).sum()
}

/// A segment's share of the area and of its first moments (x, y), by Green's theorem:
/// A = 1/2 \oint (x dy - y dx), Mx = 1/2 \oint x^2 dy, My = -1/2 \oint y^2 dx.
fn moments(s: &Segment) -> (f64, f64, f64) {
    if let Some((c, r, t0, sw)) = arc_of(s) {
        let t1 = t0 + sw;
        let (s0, c0) = dmath::sin_cos(t0);
        let (s1, c1) = dmath::sin_cos(t1);
        let a = 0.5 * (r * (c.x * (s1 - s0) - c.y * (c1 - c0)) + r * r * sw);
        // \int cos^2 = t/2 + sin 2t / 4, \int cos^3 = sin - sin^3 / 3
        let icos2 = sw / 2.0 + (s1 * c1 - s0 * c0) / 2.0;
        let isin2 = sw / 2.0 - (s1 * c1 - s0 * c0) / 2.0;
        let icos3 = (s1 - s1 * s1 * s1 / 3.0) - (s0 - s0 * s0 * s0 / 3.0);
        let isin3 = (-c1 + c1 * c1 * c1 / 3.0) - (-c0 + c0 * c0 * c0 / 3.0);
        let mx = 0.5 * (r * c.x * c.x * (s1 - s0) + 2.0 * r * r * c.x * icos2 + r * r * r * icos3);
        let my = 0.5 * (-r * c.y * c.y * (c1 - c0) + 2.0 * r * r * c.y * isin2 + r * r * r * isin3);
        return (a, mx, my);
    }
    let (p, q) = (s.start(), s.end());
    // along the line: 1/2 \int x^2 dy = dy (px^2 + px qx + qx^2) / 6, and alike for y
    let cr = p.x * q.y - q.x * p.y;
    let (dx, dy) = (q.x - p.x, q.y - p.y);
    (
        cr / 2.0,
        dy * (p.x * p.x + p.x * q.x + q.x * q.x) / 6.0,
        -dx * (p.y * p.y + p.y * q.y + q.y * q.y) / 6.0,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn near(a: f64, b: f64, tol: f64) {
        assert!((a - b).abs() <= tol, "{a} vs {b}");
    }

    #[test]
    fn hollow_rect_area_and_box() {
        let s = Section::rhs("t".into(), 40.0, 30.0, 3.0, 6.0, 3.0).unwrap();
        let pi = dmath::PI;
        let outer = 40.0 * 30.0 - (4.0 - pi) * 36.0;
        let inner = 34.0 * 24.0 - (4.0 - pi) * 9.0;
        near(s.area(), outer - inner, 1e-9);
        let (lo, hi) = s.bbox();
        near(lo.x, -20.0, 1e-12);
        near(hi.y, 15.0, 1e-12);
        let c = s.centroid();
        near(c.x, 0.0, 1e-9);
        near(c.y, 0.0, 1e-9);
        // the support at 45 degrees: the corner arc's centre (14, 9) out by r
        let w = Vec2::new(1.0, 1.0) / 2f64.sqrt();
        near(s.support(w), (14.0 + 9.0) / 2f64.sqrt() + 6.0, 1e-9);
        near(s.radius(), (14.0f64 * 14.0 + 81.0).sqrt() + 6.0, 1e-9);
    }

    #[test]
    fn angle_centroid() {
        // a sharp angle: two rectangles, 50 x 5 up and 45 x 5 across
        let s = Section::angle("L".into(), 50.0, 50.0, 5.0, 0.0, 0.0).unwrap();
        let a = 250.0 + 225.0;
        near(s.area(), a, 1e-9);
        let cx = (250.0 * 2.5 + 225.0 * (5.0 + 22.5)) / a - 25.0;
        let c = s.centroid();
        near(c.x, cx, 1e-9);
        near(c.y, cx, 1e-9);
        // a root fillet adds (1 - pi/4) r^2
        let r = Section::angle("L".into(), 50.0, 50.0, 5.0, 7.0, 0.0).unwrap();
        near(r.area(), a + (1.0 - dmath::PI / 4.0) * 49.0, 1e-9);
    }

    #[test]
    fn mirror_and_turn_keep_area_and_sense() {
        let s = Section::angle("L".into(), 60.0, 40.0, 5.0, 6.0, 3.0).unwrap();
        for m in [false, true] {
            let p = s.placed(Point2::new(3.0, -2.0), 0.7, m);
            near(p.area(), s.area(), 1e-9);
            let c = s.centroid();
            let pc = p.centroid();
            let x = if m { -(c.x - 3.0) } else { c.x - 3.0 };
            let y = c.y + 2.0;
            let (sn, cs) = dmath::sin_cos(0.7);
            near(pc.x, cs * x - sn * y, 1e-9);
            near(pc.y, sn * x + cs * y, 1e-9);
        }
    }

    /// The exact centroid against a fine polygon of the outline.
    #[test]
    fn centroid_against_polygon() {
        let s = Section::angle("L".into(), 60.0, 40.0, 5.0, 6.0, 3.0).unwrap();
        let mut pts = vec![];
        for sg in &s.loops[0] {
            let v = sg.samples(4000);
            pts.extend(v[..v.len() - 1].iter().copied());
        }
        let (mut a, mut x, mut y) = (0.0, 0.0, 0.0);
        for i in 0..pts.len() {
            let (p0, p1) = (pts[i], pts[(i + 1) % pts.len()]);
            let cr = p0.x * p1.y - p1.x * p0.y;
            a += cr / 2.0;
            x += (p0.x + p1.x) * cr / 6.0;
            y += (p0.y + p1.y) * cr / 6.0;
        }
        let c = s.centroid();
        near(c.x, x / a, 1e-5);
        near(c.y, y / a, 1e-5);
    }

    #[test]
    fn beam_and_channel() {
        let b = Section::beam("I".into(), 200.0, 100.0, 5.6, 8.5, 12.0).unwrap();
        let a = 2.0 * 100.0 * 8.5 + (200.0 - 17.0) * 5.6 + 4.0 * (1.0 - dmath::PI / 4.0) * 144.0;
        near(b.area(), a, 1e-9);
        let c = Section::channel("U".into(), 100.0, 55.0, 4.5, 7.5, 10.0).unwrap();
        let a = 2.0 * 55.0 * 7.5 + 85.0 * 4.5 + 2.0 * (1.0 - dmath::PI / 4.0) * 100.0;
        near(c.area(), a, 1e-9);
        let chs = Section::chs("O".into(), 48.3, 3.2).unwrap();
        near(
            chs.area(),
            dmath::PI * (24.15f64.powi(2) - 20.95f64.powi(2)),
            1e-9,
        );
        near(chs.support(Vec2::new(0.6, 0.8)), 24.15, 1e-12);
    }
}
