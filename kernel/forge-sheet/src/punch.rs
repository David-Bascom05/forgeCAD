//! Cut punches: turret tools from the shop's library (data), each hit a cut of the tool's shape at
//! a point and angle on a wall, so folded and flat are exact like any cut and the hole checks
//! (edge, bend, size) read them. The flat pattern and its DXF also carry each hit (tool id,
//! position, angle) on a layer of their own, for the punch program. Checks of their own: the tool
//! in the shop's library, the hit inside its wall's flat (never across a bend zone), and the web
//! between hits at least the shop's minimum.
use crate::error::{CutId, SheetError, WallId};
use crate::graph::{JointKind, Sheet, straight};
use crate::shop::Scaled;
use forge_math::{Point2, Vec2, dmath, tol};
use forge_ops::{ProfileLoop, Segment};

/// A turret tool's shape, centred on the hit, x along the tool's angle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ToolShape {
    Round {
        d: f64,
    },
    /// `length` overall along x, `width` across (round ends).
    Obround {
        length: f64,
        width: f64,
    },
    Square {
        side: f64,
    },
    Rect {
        length: f64,
        width: f64,
    },
    /// A round of diameter `d` with a flat `flat` from the centre, on +x.
    D {
        d: f64,
        flat: f64,
    },
    /// A round of diameter `d` with a slot `width` wide running `length` from the centre along +x.
    Keyhole {
        d: f64,
        width: f64,
        length: f64,
    },
    Hex {
        across_flats: f64,
    },
    /// `nx` by `ny` rounds of diameter `d` at pitch (`px`, `py`), centred on the hit.
    Cluster {
        d: f64,
        px: f64,
        py: f64,
        nx: u32,
        ny: u32,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Tool {
    pub id: String,
    pub shape: ToolShape,
}

/// The shop's punching rules: the least web left between two hits.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PunchRules {
    pub spacing: Scaled,
}

impl Default for PunchRules {
    /// A web of one thickness: the common turret rule of thumb.
    fn default() -> PunchRules {
        PunchRules {
            spacing: Scaled {
                per_t: 1.0,
                per_r: 0.0,
                plus: 0.0,
            },
        }
    }
}

/// One hit, as the graph keeps it.
#[derive(Clone, Debug, PartialEq)]
pub struct Punch {
    pub wall: WallId,
    pub tool: Tool,
    pub at: Point2,
    pub angle: f64,
    pub cuts: Vec<CutId>,
}

/// A hit on the flat, for the DXF.
#[derive(Clone, Debug, PartialEq)]
pub struct Hit {
    pub tool: String,
    pub at: Point2,
    pub angle: f64,
}

/// Why a punch is refused.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PunchError {
    /// A size not positive, or one that leaves the shape degenerate (`which` names it).
    Size {
        which: &'static str,
        value: f64,
        limit: f64,
    },
}

/// A punch against the shop. `punch` indexes `Sheet::punches`.
#[derive(Clone, Debug, PartialEq)]
pub enum PunchViolation {
    /// The shop's library has no tool of this id (or one of another shape).
    NotInShop { tool: String },
    /// The hit reaches `depth` past a bend's tangent line into its zone (or past the mold line).
    AcrossBend {
        joint: crate::error::JointId,
        depth: f64,
    },
    /// The web between this hit and hit `other` is `web`, under `limit`.
    Spacing { other: usize, web: f64, limit: f64 },
}

fn rot(at: Point2, angle: f64) -> impl Fn(f64, f64) -> Point2 {
    let (s, c) = dmath::sin_cos(angle);
    move |x, y| Point2::new(at.x + c * x - s * y, at.y + s * x + c * y)
}

fn loop_of(segs: Vec<Segment>) -> ProfileLoop {
    ProfileLoop::new(segs.into_iter().enumerate().map(|(i, s)| (s, i as u64)).collect())
}

fn poly(p: &[Point2]) -> ProfileLoop {
    let n = p.len();
    loop_of(
        (0..n)
            .map(|i| Segment::Line {
                a: p[i],
                b: p[(i + 1) % n],
            })
            .collect(),
    )
}

fn circle(c: Point2, r: f64) -> ProfileLoop {
    loop_of(vec![Segment::Circle {
        center: c,
        radius: r,
        ccw: true,
    }])
}

impl ToolShape {
    /// The loops the tool cuts at `at`, turned `angle`, counter-clockwise; exact lines and arcs.
    pub fn loops(&self, at: Point2, angle: f64) -> Result<Vec<ProfileLoop>, PunchError> {
        let pos = |which: &'static str, v: f64| {
            if v.is_nan() || v <= tol::LINEAR {
                Err(PunchError::Size {
                    which,
                    value: v,
                    limit: tol::LINEAR,
                })
            } else {
                Ok(v)
            }
        };
        let p = rot(at, angle);
        Ok(match *self {
            ToolShape::Round { d } => vec![circle(at, 0.5 * pos("d", d)?)],
            ToolShape::Square { side } => {
                let h = 0.5 * pos("side", side)?;
                vec![poly(&[p(-h, -h), p(h, -h), p(h, h), p(-h, h)])]
            }
            ToolShape::Rect { length, width } => {
                let (a, b) = (0.5 * pos("length", length)?, 0.5 * pos("width", width)?);
                vec![poly(&[p(-a, -b), p(a, -b), p(a, b), p(-a, b)])]
            }
            ToolShape::Obround { length, width } => {
                let r = 0.5 * pos("width", width)?;
                let a = 0.5 * pos("length", length)? - r;
                if a <= tol::LINEAR {
                    return Err(PunchError::Size {
                        which: "length",
                        value: length,
                        limit: width,
                    });
                }
                let (c0, c1) = (p(-a, 0.0), p(a, 0.0));
                vec![loop_of(vec![
                    Segment::Line {
                        a: p(-a, -r),
                        b: p(a, -r),
                    },
                    Segment::Arc {
                        center: c1,
                        radius: r,
                        start: angle - dmath::PI / 2.0,
                        sweep: dmath::PI,
                    },
                    Segment::Line {
                        a: p(a, r),
                        b: p(-a, r),
                    },
                    Segment::Arc {
                        center: c0,
                        radius: r,
                        start: angle + dmath::PI / 2.0,
                        sweep: dmath::PI,
                    },
                ])]
            }
            ToolShape::D { d, flat } => {
                let r = 0.5 * pos("d", d)?;
                if flat.is_nan() || flat <= tol::LINEAR || flat >= r - tol::LINEAR {
                    return Err(PunchError::Size {
                        which: "flat",
                        value: flat,
                        limit: r,
                    });
                }
                // the flat at x = flat, the arc round the rest
                let h = (r * r - flat * flat).sqrt();
                let a0 = dmath::atan2(h, flat);
                vec![loop_of(vec![
                    Segment::Arc {
                        center: at,
                        radius: r,
                        start: angle + a0,
                        sweep: dmath::TAU - 2.0 * a0,
                    },
                    Segment::Line {
                        a: p(flat, -h),
                        b: p(flat, h),
                    },
                ])]
            }
            ToolShape::Keyhole { d, width, length } => {
                let r = 0.5 * pos("d", d)?;
                let w = 0.5 * pos("width", width)?;
                if w >= r - tol::LINEAR || length.is_nan() || length <= r + tol::LINEAR {
                    return Err(PunchError::Size {
                        which: if w >= r - tol::LINEAR { "width" } else { "length" },
                        value: if w >= r - tol::LINEAR { width } else { length },
                        limit: if w >= r - tol::LINEAR { d } else { r },
                    });
                }
                let x0 = (r * r - w * w).sqrt();
                let a0 = dmath::atan2(w, x0);
                vec![loop_of(vec![
                    Segment::Arc {
                        center: at,
                        radius: r,
                        start: angle + a0,
                        sweep: dmath::TAU - 2.0 * a0,
                    },
                    Segment::Line {
                        a: p(x0, -w),
                        b: p(length, -w),
                    },
                    Segment::Line {
                        a: p(length, -w),
                        b: p(length, w),
                    },
                    Segment::Line {
                        a: p(length, w),
                        b: p(x0, w),
                    },
                ])]
            }
            ToolShape::Hex { across_flats } => {
                let r = 0.5 * pos("across_flats", across_flats)? / dmath::cos(dmath::PI / 6.0);
                let pts: Vec<Point2> = (0..6)
                    .map(|k| {
                        let (s, c) = dmath::sin_cos(k as f64 * dmath::PI / 3.0);
                        p(r * c, r * s)
                    })
                    .collect();
                vec![poly(&pts)]
            }
            ToolShape::Cluster { d, px, py, nx, ny } => {
                let r = 0.5 * pos("d", d)?;
                if nx == 0 || ny == 0 {
                    return Err(PunchError::Size {
                        which: "count",
                        value: 0.0,
                        limit: 1.0,
                    });
                }
                for (which, pitch, n) in [("px", px, nx), ("py", py, ny)] {
                    if n > 1 && (pitch.is_nan() || pitch <= 2.0 * r + tol::LINEAR) {
                        return Err(PunchError::Size {
                            which,
                            value: pitch,
                            limit: 2.0 * r,
                        });
                    }
                }
                let mut v = Vec::new();
                for i in 0..nx {
                    for j in 0..ny {
                        let x = (i as f64 - 0.5 * (nx - 1) as f64) * px;
                        let y = (j as f64 - 0.5 * (ny - 1) as f64) * py;
                        v.push(circle(p(x, y), r));
                    }
                }
                v
            }
        })
    }

    /// The area the tool takes out, exactly.
    pub fn area(&self) -> f64 {
        let pi = dmath::PI;
        match *self {
            ToolShape::Round { d } => 0.25 * pi * d * d,
            ToolShape::Square { side } => side * side,
            ToolShape::Rect { length, width } => length * width,
            ToolShape::Obround { length, width } => (length - width) * width + 0.25 * pi * width * width,
            ToolShape::D { d, flat } => {
                let r = 0.5 * d;
                let h = (r * r - flat * flat).sqrt();
                // the circle less the segment beyond the flat
                pi * r * r - (r * r * dmath::acos(flat / r) - flat * h)
            }
            ToolShape::Keyhole { d, width, length } => {
                let (r, w) = (0.5 * d, 0.5 * width);
                let x0 = (r * r - w * w).sqrt();
                // the circle, plus the slot past it: rectangle to `length` less the circle's
                // segment beyond x0
                let segment = r * r * dmath::acos(x0 / r) - x0 * w;
                pi * r * r + (length - x0) * 2.0 * w - segment
            }
            ToolShape::Hex { across_flats } => 0.5 * 3.0f64.sqrt() * across_flats * across_flats,
            ToolShape::Cluster { d, nx, ny, .. } => 0.25 * pi * d * d * (nx * ny) as f64,
        }
    }
}

impl Sheet {
    /// Hits `tool` on `wall` at `at` (its coordinates), turned `angle`: its loops cut, the hit
    /// recorded. All or nothing.
    pub fn punch(&mut self, wall: WallId, tool: &Tool, at: Point2, angle: f64) -> Result<usize, SheetError> {
        let feature = self.feature;
        self.wall(wall).ok_or(SheetError::NoSuchWall { feature, wall })?;
        let loops = tool
            .shape
            .loops(at, angle)
            .map_err(|why| SheetError::Punch { feature, why })?;
        let mut work = self.clone();
        let mut cuts = Vec::with_capacity(loops.len());
        for l in loops {
            cuts.push(work.cut(wall, l)?);
        }
        work.punches.push(Punch {
            wall,
            tool: tool.clone(),
            at,
            angle,
            cuts,
        });
        *self = work;
        Ok(self.punches.len() - 1)
    }
}

/// Samples of a loop's boundary, exact at every vertex and arc extreme it passes.
fn boundary(l: &ProfileLoop) -> Vec<Point2> {
    l.segments.iter().flat_map(|(s, _)| s.samples(64)).collect()
}

/// The hits on the flat: each at its wall's developed place.
pub fn hits(sheet: &Sheet, layout: &crate::build::Layout) -> Vec<Hit> {
    sheet
        .punches
        .iter()
        .filter_map(|p| {
            let w = layout.walls.iter().find(|w| w.wall == p.wall)?;
            let d = w.developed;
            Some(Hit {
                tool: p.tool.id.clone(),
                at: d.apply(p.at),
                angle: p.angle + dmath::atan2(d.x.y, d.x.x),
            })
        })
        .collect()
}

/// Each punch against the shop's library and rules.
pub fn check(sheet: &Sheet, tools: &[Tool], rules: &PunchRules) -> Vec<(usize, PunchViolation)> {
    let t = sheet.gauge.thickness;
    let mut out = Vec::new();
    let shapes: Vec<Vec<Point2>> = sheet
        .punches
        .iter()
        .map(|p| {
            p.tool
                .shape
                .loops(p.at, p.angle)
                .unwrap_or_default()
                .iter()
                .flat_map(boundary)
                .collect()
        })
        .collect();
    for (i, p) in sheet.punches.iter().enumerate() {
        if !tools.iter().any(|x| x.id == p.tool.id && x.shape == p.tool.shape) {
            out.push((
                i,
                PunchViolation::NotInShop {
                    tool: p.tool.id.clone(),
                },
            ));
        }
        // inside the wall's flat: every point at least the setback in from each bend's mold line
        for j in &sheet.joints {
            let (seg, side_a) = if j.a == p.wall {
                (j.on_a, true)
            } else if j.b == p.wall {
                (j.on_b, false)
            } else {
                continue;
            };
            let Some(w) = sheet.wall(p.wall) else { continue };
            let Ok((a, b)) = straight(w, seg) else { continue };
            let s = match j.kind {
                JointKind::Bend { angle, radius } => crate::build::setback_of(&sheet.gauge, radius, angle),
                JointKind::Hem(h) => h.setback(t),
                _ => continue,
            };
            let _ = side_a;
            let d = (b - a) * (1.0 / a.distance(b));
            let inward = Vec2::new(-d.y, d.x);
            let len = a.distance(b);
            let worst = shapes[i]
                .iter()
                .filter(|q| {
                    let along = (**q - a).dot(d);
                    along > -tol::LINEAR && along < len + tol::LINEAR
                })
                .map(|q| (*q - a).dot(inward))
                .fold(f64::INFINITY, f64::min);
            if worst < s - tol::LINEAR {
                out.push((
                    i,
                    PunchViolation::AcrossBend {
                        joint: j.id,
                        depth: s - worst,
                    },
                ));
            }
        }
        // the web to every later hit on the same wall
        let limit = rules.spacing.at(t, 0.0);
        for (k, q) in sheet.punches.iter().enumerate().skip(i + 1) {
            if q.wall != p.wall {
                continue;
            }
            let web = web(&sheet.punches[i], q, &shapes[i], &shapes[k]);
            if web < limit - tol::LINEAR {
                out.push((i, PunchViolation::Spacing { other: k, web, limit }));
            }
        }
    }
    out
}

/// The web between two hits: exact for two rounds, else the least distance between their
/// boundaries' samples (overlapping shapes give 0).
fn web(p: &Punch, q: &Punch, a: &[Point2], b: &[Point2]) -> f64 {
    if let (ToolShape::Round { d: d1 }, ToolShape::Round { d: d2 }) = (p.tool.shape, q.tool.shape) {
        return (p.at.distance(q.at) - 0.5 * (d1 + d2)).max(0.0);
    }
    a.iter()
        .flat_map(|x| b.iter().map(move |y| x.distance(*y)))
        .fold(f64::INFINITY, f64::min)
}
