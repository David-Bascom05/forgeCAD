//! Tools as exact solids of revolution, and their signed distance in certified intervals.
//!
//! A body is the solid swept by revolving, about the z axis, the region of the (ρ, z)
//! half-plane bounded by the axis and an open chain of lines and arcs that leaves the axis at
//! its bottom and returns to it at its top. Its *side* (the chain between the bottom and top
//! caps) rises: z never decreases along it, so the body is {z0 ≤ z ≤ z1, ρ ≤ w(z)} for one radius
//! function w. Each arc lies in one half of its circle (left or right of its centre), so w is a
//! single branch on it.
//!
//! The signed distance of a 3D point is the 2D signed distance of (ρ, z) to that region with the
//! axis left out (the axis is inside the solid, not its surface): positive outside. The
//! evaluation is the natural interval extension of the formula, so for a box of points it gives
//! an interval holding the signed distance of every point of the box. No error budget is
//! assumed: every float operation rounds outward.
use crate::ival::Ival;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PSeg {
    /// From a to b, (ρ, z).
    Line { a: [f64; 2], b: [f64; 2] },
    /// From a to b on the circle about c of radius r, counter-clockwise in (ρ, z) or not; both
    /// ends exactly on the circle (the constructors make them so), spanning less than a half
    /// turn.
    Arc {
        a: [f64; 2],
        b: [f64; 2],
        c: [f64; 2],
        r: f64,
        ccw: bool,
    },
}

impl PSeg {
    fn a(&self) -> [f64; 2] {
        match *self {
            PSeg::Line { a, .. } | PSeg::Arc { a, .. } => a,
        }
    }
    fn b(&self) -> [f64; 2] {
        match *self {
            PSeg::Line { b, .. } | PSeg::Arc { b, .. } => b,
        }
    }
}

/// What a body is, for the limit it is held to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    /// The flutes: may touch the part's target-plus-leave within the tolerance.
    Cutter,
    /// Neck, shank, holder, extensions: must keep a clearance from everything.
    NonCutting,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Body {
    pub chain: Vec<PSeg>,
    pub part: Part,
    pub z0: f64,
    pub z1: f64,
    /// The largest radius.
    pub rmax: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ToolError {
    /// A required dimension is missing or out of range: its name.
    Bad(&'static str),
    /// The chain does not start and end on the axis, or its side falls.
    Chain,
}

impl Body {
    /// A body from its chain (validated: starts and ends on the axis, the side rises).
    pub fn new(chain: Vec<PSeg>, part: Part) -> Result<Body, ToolError> {
        if chain.is_empty() {
            return Err(ToolError::Chain);
        }
        let first = chain[0].a();
        let last = chain[chain.len() - 1].b();
        if first[0] != 0.0 || last[0] != 0.0 || last[1] <= first[1] {
            return Err(ToolError::Chain);
        }
        let mut rmax = 0.0f64;
        let mut i = 0;
        while i < chain.len() {
            let (a, b) = (chain[i].a(), chain[i].b());
            if i + 1 < chain.len() && b != chain[i + 1].a() {
                return Err(ToolError::Chain);
            }
            if a[0] < 0.0 || b[0] < 0.0 || a[1] < first[1] || b[1] > last[1] {
                return Err(ToolError::Chain);
            }
            // caps are horizontal and only at the ends; elsewhere z must not fall
            let cap = (i == 0 || i + 1 == chain.len()) && a[1] == b[1];
            if !cap && b[1] < a[1] {
                return Err(ToolError::Chain);
            }
            if let PSeg::Arc { a, b, c, .. } = chain[i]
                && ((a[0] - c[0]) * (b[0] - c[0]) < 0.0)
            {
                // an arc crossing its centre's vertical: split it in the constructor
                return Err(ToolError::Chain);
            }
            if a[0] > rmax {
                rmax = a[0];
            }
            if b[0] > rmax {
                rmax = b[0];
            }
            i += 1;
        }
        Ok(Body {
            chain,
            part,
            z0: first[1],
            z1: last[1],
            rmax,
        })
    }

    /// The radius function's range over the heights z (within [z0, z1]).
    fn w_range(&self, z: Ival) -> Ival {
        let mut out = Ival {
            lo: f64::INFINITY,
            hi: f64::NEG_INFINITY,
        };
        let mut i = 0;
        while i < self.chain.len() {
            let s = self.chain[i];
            let (a, b) = (s.a(), s.b());
            let (zl, zh) = (a[1].min(b[1]), a[1].max(b[1]));
            if zh >= z.lo && zl <= z.hi {
                let zc = Ival {
                    lo: z.lo.max(zl),
                    hi: z.hi.min(zh),
                };
                let w = if zl == zh {
                    // a cap: every radius along it
                    Ival::new(a[0].min(b[0]), a[0].max(b[0]))
                } else {
                    match s {
                        PSeg::Line { .. } => {
                            // ρ = a.ρ + (z − a.z) (b.ρ − a.ρ) / (b.z − a.z)
                            let k = Ival::pt(b[0])
                                .sub(Ival::pt(a[0]))
                                .div(Ival::pt(b[1]).sub(Ival::pt(a[1])));
                            Ival::pt(a[0]).add(zc.sub(Ival::pt(a[1])).mul(k))
                        }
                        PSeg::Arc { c, r, .. } => {
                            let dz = zc.sub(Ival::pt(c[1]));
                            let h = Ival::pt(r).sqr().sub(dz.sqr()).sqrt();
                            if a[0] + b[0] >= 2.0 * c[0] {
                                Ival::pt(c[0]).add(h)
                            } else {
                                Ival::pt(c[0]).sub(h)
                            }
                        }
                    }
                };
                out = Ival {
                    lo: out.lo.min(w.lo),
                    hi: out.hi.max(w.hi),
                };
            }
            i += 1;
        }
        out
    }

    /// 1 surely inside (closed), -1 surely outside, 0 unknown.
    fn inside(&self, rho: Ival, z: Ival) -> i32 {
        if z.hi < self.z0 || z.lo > self.z1 {
            return -1;
        }
        if z.lo < self.z0 || z.hi > self.z1 {
            return 0;
        }
        let w = self.w_range(z);
        if rho.hi <= w.lo {
            1
        } else if rho.lo > w.hi {
            -1
        } else {
            0
        }
    }

    /// An interval holding the distance from every point of the box (ρ, z) to the boundary.
    fn dist(&self, rho: Ival, z: Ival) -> Ival {
        let mut lo = f64::INFINITY;
        let mut hi = f64::INFINITY;
        let mut i = 0;
        while i < self.chain.len() {
            // a segment whose box is at least `hi` away changes neither end of the minimum
            // (its interval lies at or above hi ≥ lo): skipped, the result is the same
            if hi.is_finite() && box_gap(&self.chain[i], rho, z) >= hi {
                i += 1;
                continue;
            }
            let d = seg_dist(&self.chain[i], rho, z);
            if d.lo < lo {
                lo = d.lo;
            }
            if d.hi < hi {
                hi = d.hi;
            }
            i += 1;
        }
        Ival { lo, hi }
    }

    /// An interval holding the signed distance (positive outside) of every point (x, y, z) of
    /// the box, in the tool's frame (tip at the origin, axis +z).
    pub fn sd(&self, x: Ival, y: Ival, z: Ival) -> Ival {
        let rho = x.sqr().add(y.sqr()).sqrt();
        self.sd_rz(rho, z)
    }

    pub fn sd_rz(&self, rho: Ival, z: Ival) -> Ival {
        let d = self.dist(rho, z);
        match self.inside(rho, z) {
            1 => d.neg(),
            -1 => d,
            _ => {
                // an inside point is no deeper than its gap to the side, (w(z) − ρ), or to
                // either cap: each is a distance to a boundary point
                let zc = Ival {
                    lo: z.lo.max(self.z0),
                    hi: z.hi.min(self.z1),
                };
                let w = if zc.lo <= zc.hi {
                    self.w_range(zc)
                } else {
                    Ival::pt(0.0)
                };
                let side = Ival::pt(w.hi).sub(Ival::pt(rho.lo)).hi;
                let bot = Ival::pt(z.hi).sub(Ival::pt(self.z0)).hi;
                let top = Ival::pt(self.z1).sub(Ival::pt(z.lo)).hi;
                // no point of the box is strictly inside: then every one is at its distance
                if side <= 0.0 || bot <= 0.0 || top <= 0.0 {
                    return d;
                }
                let mut deep = d.hi;
                if side < deep {
                    deep = side;
                }
                if bot < deep {
                    deep = bot;
                }
                if top < deep {
                    deep = top;
                }
                if deep < 0.0 {
                    deep = 0.0;
                }
                Ival { lo: -deep, hi: d.hi }
            }
        }
    }
}

/// A lower bound on the distance from the box (ρ, z) to the segment: the larger axis gap to a
/// box holding the segment (rounded down).
fn box_gap(s: &PSeg, rho: Ival, z: Ival) -> f64 {
    let (x0, y0, x1, y1) = match *s {
        PSeg::Line { a, b } => (a[0].min(b[0]), a[1].min(b[1]), a[0].max(b[0]), a[1].max(b[1])),
        PSeg::Arc { c, r, .. } => (
            Ival::pt(c[0]).sub(Ival::pt(r)).lo,
            Ival::pt(c[1]).sub(Ival::pt(r)).lo,
            Ival::pt(c[0]).add(Ival::pt(r)).hi,
            Ival::pt(c[1]).add(Ival::pt(r)).hi,
        ),
    };
    let gx = Ival::pt(x0)
        .sub(Ival::pt(rho.hi))
        .lo
        .max(Ival::pt(rho.lo).sub(Ival::pt(x1)).lo);
    let gy = Ival::pt(y0)
        .sub(Ival::pt(z.hi))
        .lo
        .max(Ival::pt(z.lo).sub(Ival::pt(y1)).lo);
    gx.max(gy).max(0.0)
}

fn len2(x: Ival, y: Ival) -> Ival {
    x.sqr().add(y.sqr()).sqrt()
}

/// An interval holding the distance from every point of the box to the segment.
pub fn seg_dist(s: &PSeg, rho: Ival, z: Ival) -> Ival {
    match *s {
        PSeg::Line { a, b } => {
            let (dx, dy) = (
                Ival::pt(b[0]).sub(Ival::pt(a[0])),
                Ival::pt(b[1]).sub(Ival::pt(a[1])),
            );
            let (px, py) = (rho.sub(Ival::pt(a[0])), z.sub(Ival::pt(a[1])));
            let dd = dx.sqr().add(dy.sqr());
            let t = px.mul(dx).add(py.mul(dy)).div(dd);
            // only what the answer needs
            if t.hi <= 0.0 {
                return len2(px, py);
            }
            if t.lo >= 1.0 {
                return len2(rho.sub(Ival::pt(b[0])), z.sub(Ival::pt(b[1])));
            }
            let perp = px.mul(dy).sub(py.mul(dx)).abs().div(dd.sqrt());
            if t.lo >= 0.0 && t.hi <= 1.0 {
                return perp;
            }
            let da = len2(px, py);
            let db = len2(rho.sub(Ival::pt(b[0])), z.sub(Ival::pt(b[1])));
            Ival {
                lo: perp.lo,
                hi: da.hi.min(db.hi),
            }
        }
        PSeg::Arc { a, b, c, r, ccw } => {
            let (vx, vy) = (rho.sub(Ival::pt(c[0])), z.sub(Ival::pt(c[1])));
            let (ax, ay) = (a[0] - c[0], a[1] - c[1]);
            let (bx, by) = (b[0] - c[0], b[1] - c[1]);
            // within the sector from a to b (less than a half turn): cross(ua, v) and cross(v, ub)
            // both of the arc's sign
            let c1 = Ival::pt(ax).mul(vy).sub(Ival::pt(ay).mul(vx));
            let c2 = vx.mul(Ival::pt(by)).sub(vy.mul(Ival::pt(bx)));
            let (c1, c2) = if ccw { (c1, c2) } else { (c1.neg(), c2.neg()) };
            let circ = len2(vx, vy).sub(Ival::pt(r)).abs();
            if c1.lo >= 0.0 && c2.lo >= 0.0 {
                return circ;
            }
            let da = len2(rho.sub(Ival::pt(a[0])), z.sub(Ival::pt(a[1])));
            let db = len2(rho.sub(Ival::pt(b[0])), z.sub(Ival::pt(b[1])));
            let ends = Ival {
                lo: da.lo.min(db.lo),
                hi: da.hi.min(db.hi),
            };
            if c1.hi < 0.0 || c2.hi < 0.0 {
                return ends;
            }
            Ival {
                lo: circ.lo,
                hi: ends.hi,
            }
        }
    }
}

/// A tool: its bodies, cutter first, all in the tool's frame (tip at the origin, axis +z).
#[derive(Clone, Debug, PartialEq)]
pub struct Shape {
    pub bodies: Vec<Body>,
}

impl Shape {
    pub fn rmax(&self) -> f64 {
        let mut m = 0.0f64;
        let mut i = 0;
        while i < self.bodies.len() {
            if self.bodies[i].rmax > m {
                m = self.bodies[i].rmax;
            }
            i += 1;
        }
        m
    }
    pub fn top(&self) -> f64 {
        let mut m = 0.0f64;
        let mut i = 0;
        while i < self.bodies.len() {
            if self.bodies[i].z1 > m {
                m = self.bodies[i].z1;
            }
            i += 1;
        }
        m
    }
}

/// The cutting end of a tool, by type. Dimensions in mm, angles in degrees (included angle).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Cutter {
    /// Flat end mill: diameter.
    Flat { d: f64 },
    /// Ball end mill: diameter.
    Ball { d: f64 },
    /// Bull nose: diameter and corner radius.
    Bull { d: f64, re: f64 },
    /// Chamfer mill / spot drill / countersink: tip diameter, included angle, largest diameter.
    Chamfer { d_tip: f64, angle: f64, d: f64 },
    /// Twist drill: diameter, point angle.
    Drill { d: f64, angle: f64 },
}

fn half_tan(angle_deg: f64) -> Result<f64, ToolError> {
    // tan(angle/2) for the standard included angles, exact where it can be
    let a = angle_deg;
    if !(a > 0.0 && a < 180.0) {
        return Err(ToolError::Bad("angle"));
    }
    // a rational approximation is not exact; the cone is *defined* by this slope, which is the
    // f64 nearest tan(angle/2) by forge_math-free evaluation: the series below
    Ok(tan_deg(a * 0.5))
}

/// tan of an angle in degrees, by its Taylor series in radians after range reduction to
/// [0, 45]: deterministic everywhere (no platform libm). Accurate to a few ulps.
fn tan_deg(deg: f64) -> f64 {
    let x = deg * (std::f64::consts::PI / 180.0);
    // sin and cos series to 25 terms: |x| <= pi/2 here
    let x2 = x * x;
    let mut s = 0.0;
    let mut c = 0.0;
    let mut ts = x;
    let mut tc = 1.0;
    let mut k = 0;
    while k < 25 {
        s += ts;
        c += tc;
        let n = (2 * k + 2) as f64;
        ts = -ts * x2 / (n * (n + 1.0));
        tc = -tc * x2 / ((n - 1.0) * n);
        k += 1;
    }
    s / c
}

impl Cutter {
    /// The cutter's side chain from the tip at (0, 0) up to (radius, height) where the flutes'
    /// full diameter begins; returns the chain and that point.
    fn tip(&self) -> Result<(Vec<PSeg>, [f64; 2]), ToolError> {
        match *self {
            Cutter::Flat { d } => {
                if !(d > 0.0) {
                    return Err(ToolError::Bad("dc"));
                }
                let r = 0.5 * d;
                Ok((
                    vec![PSeg::Line {
                        a: [0.0, 0.0],
                        b: [r, 0.0],
                    }],
                    [r, 0.0],
                ))
            }
            Cutter::Ball { d } => {
                if !(d > 0.0) {
                    return Err(ToolError::Bad("dc"));
                }
                let r = 0.5 * d;
                Ok((
                    vec![PSeg::Arc {
                        a: [0.0, 0.0],
                        b: [r, r],
                        c: [0.0, r],
                        r,
                        ccw: true,
                    }],
                    [r, r],
                ))
            }
            Cutter::Bull { d, re } => {
                let r = 0.5 * d;
                if !(d > 0.0) {
                    return Err(ToolError::Bad("dc"));
                }
                if !(re > 0.0 && re <= r) {
                    return Err(ToolError::Bad("re"));
                }
                let flat = r - re;
                let mut v = Vec::new();
                if flat > 0.0 {
                    v.push(PSeg::Line {
                        a: [0.0, 0.0],
                        b: [flat, 0.0],
                    });
                }
                v.push(PSeg::Arc {
                    a: [flat, 0.0],
                    b: [r, re],
                    c: [flat, re],
                    r: re,
                    ccw: true,
                });
                Ok((v, [r, re]))
            }
            Cutter::Chamfer { d_tip, angle, d } => {
                if !(d > 0.0 && d_tip >= 0.0 && d_tip < d) {
                    return Err(ToolError::Bad("dc"));
                }
                let t = half_tan(angle)?;
                let (r0, r) = (0.5 * d_tip, 0.5 * d);
                let h = (r - r0) / t;
                let mut v = Vec::new();
                if r0 > 0.0 {
                    v.push(PSeg::Line {
                        a: [0.0, 0.0],
                        b: [r0, 0.0],
                    });
                }
                v.push(PSeg::Line {
                    a: [r0, 0.0],
                    b: [r, h],
                });
                Ok((v, [r, h]))
            }
            Cutter::Drill { d, angle } => {
                if !(d > 0.0) {
                    return Err(ToolError::Bad("dc"));
                }
                let t = half_tan(angle)?;
                let r = 0.5 * d;
                let h = r / t;
                Ok((
                    vec![PSeg::Line {
                        a: [0.0, 0.0],
                        b: [r, h],
                    }],
                    [r, h],
                ))
            }
        }
    }

    pub fn radius(&self) -> f64 {
        match *self {
            Cutter::Flat { d }
            | Cutter::Ball { d }
            | Cutter::Bull { d, .. }
            | Cutter::Chamfer { d, .. }
            | Cutter::Drill { d, .. } => 0.5 * d,
        }
    }
}

/// A holder or extension: a stack of (z above its nose, diameter) rings; between rings the
/// profile is straight, so a stack is cylinders and cone frusta. z from 0 (the nose) up.
#[derive(Clone, Debug, PartialEq)]
pub struct Holder {
    pub profile: Vec<[f64; 2]>,
}

/// A complete tool: cutter type, flute length, shank diameter, stick-out (gauge: tip to the
/// holder's nose) and the holder.
pub fn assemble(
    cut: &Cutter,
    lcf: f64,
    dcon: f64,
    stickout: f64,
    holder: Option<&Holder>,
) -> Result<Shape, ToolError> {
    let (mut chain, top) = cut.tip()?;
    let r = cut.radius();
    if !(lcf > top[1]) {
        return Err(ToolError::Bad("lcf"));
    }
    if !(dcon > 0.0) {
        return Err(ToolError::Bad("dcon"));
    }
    if !(stickout > lcf) {
        return Err(ToolError::Bad("stickout"));
    }
    chain.push(PSeg::Line { a: top, b: [r, lcf] });
    chain.push(PSeg::Line {
        a: [r, lcf],
        b: [0.0, lcf],
    });
    let cutter = Body::new(chain, Part::Cutter)?;
    // the shank above the flutes, up to the holder's nose
    let rs = 0.5 * dcon;
    let shank = Body::new(
        vec![
            PSeg::Line {
                a: [0.0, lcf],
                b: [rs, lcf],
            },
            PSeg::Line {
                a: [rs, lcf],
                b: [rs, stickout],
            },
            PSeg::Line {
                a: [rs, stickout],
                b: [0.0, stickout],
            },
        ],
        Part::NonCutting,
    )?;
    let mut bodies = vec![cutter, shank];
    if let Some(h) = holder {
        if h.profile.len() < 2 {
            return Err(ToolError::Bad("holder"));
        }
        let mut ch = Vec::new();
        let p0 = h.profile[0];
        ch.push(PSeg::Line {
            a: [0.0, stickout + p0[0]],
            b: [0.5 * p0[1], stickout + p0[0]],
        });
        let mut i = 1;
        while i < h.profile.len() {
            let (a, b) = (h.profile[i - 1], h.profile[i]);
            ch.push(PSeg::Line {
                a: [0.5 * a[1], stickout + a[0]],
                b: [0.5 * b[1], stickout + b[0]],
            });
            i += 1;
        }
        let pl = h.profile[h.profile.len() - 1];
        ch.push(PSeg::Line {
            a: [0.5 * pl[1], stickout + pl[0]],
            b: [0.0, stickout + pl[0]],
        });
        bodies.push(Body::new(ch, Part::NonCutting)?);
    }
    Ok(Shape { bodies })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64) -> Ival {
        Ival::pt(x)
    }

    #[test]
    fn signed_distance_of_a_flat_and_a_ball() {
        let t = assemble(&Cutter::Flat { d: 6.0 }, 15.0, 6.0, 30.0, None).unwrap();
        let c = &t.bodies[0];
        // on the bottom face: 0; below it: positive; beside the flutes: the radial gap
        let s = c.sd(p(1.0), p(0.0), p(0.0));
        assert!(s.lo <= 0.0 && s.hi >= 0.0 && s.width() < 1e-12);
        let s = c.sd(p(0.0), p(0.0), p(-2.0));
        assert!((s.lo - 2.0).abs() < 1e-12 && s.lo <= 2.0 && s.hi >= 2.0);
        let s = c.sd(p(5.0), p(0.0), p(5.0));
        assert!(s.lo <= 2.0 && s.hi >= 2.0 && s.width() < 1e-12);
        // inside
        let s = c.sd(p(0.0), p(0.0), p(5.0));
        assert!(s.hi < 0.0 && s.lo <= -3.0 && s.hi >= -3.0);
        let b = assemble(&Cutter::Ball { d: 6.0 }, 15.0, 6.0, 30.0, None).unwrap();
        // 45° below the ball's centre, at distance 4 from it: 1 outside
        let k = 4.0 / 2f64.sqrt();
        let s = b.bodies[0].sd(p(k), p(0.0), p(3.0 - k));
        assert!(
            s.lo <= 1.0 + 1e-12 && s.hi >= 1.0 - 1e-12 && s.width() < 1e-9,
            "{s:?}"
        );
        // a box straddling the surface holds 0
        let s = b.bodies[0].sd(Ival::new(0.0, 0.5), p(0.0), Ival::new(-0.1, 0.1));
        assert!(s.lo < 0.0 && s.hi > 0.0);
        // tan(59°) for a 118° drill point
        assert!((tan_deg(59.0) - 1.6642794823505178).abs() < 1e-14);
    }
}
