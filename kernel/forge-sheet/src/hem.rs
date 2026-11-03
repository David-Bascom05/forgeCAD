//! Hems: a fold of half a turn or more off a straight wall edge, as a joint of its own kind. The
//! edge is the fold's outside (material inside: the hem keeps the wall's overall size), so the
//! hem's tangent line on the wall is R + t in from the edge, and the leg runs back from the fold's
//! far tangent line. Closed (flat), open (a gap), teardrop (past half a turn, the leg closing in)
//! and rolled (an arc only, or with a leg). A crushed hem has no inside radius and two skins
//! touching, which no closed solid has: it is modelled with the shop's closed gap, never below
//! `GAP_FLOOR`, a labelled convention. Its allowance has no radius formula, so it comes from the
//! shop's hem table; other hems take a table row if there is one, else the sheet's bend rule.
use crate::error::{FeatureId, JointId, SheetError, TrimRef, WallId};
use crate::graph::{JointKind, Sheet, Wall, straight};
use forge_math::{Point2, dmath, tol};
use forge_ops::{ProfileLoop, Segment};

/// The least gap a hem's two layers keep (as rips do: a solid has no touching skins), mm.
pub const GAP_FLOOR: f64 = 0.02;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum HemName {
    Closed,
    Open,
    Teardrop,
    Rolled,
}

impl HemName {
    pub fn name(self) -> &'static str {
        match self {
            HemName::Closed => "closed",
            HemName::Open => "open",
            HemName::Teardrop => "teardrop",
            HemName::Rolled => "rolled",
        }
    }
}

/// What a hem asks for. Angles are the fold's turn, radians.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum HemKind {
    /// Half a turn, flat: the shop's closed gap between the layers.
    Closed,
    /// Half a turn, `gap` between the layers.
    Open { gap: f64 },
    /// Past half a turn round inside radius `radius`, the leg closing back towards the wall.
    Teardrop { radius: f64, angle: f64 },
    /// Round inside radius `radius` through `angle`.
    Rolled { radius: f64, angle: f64 },
}

impl HemKind {
    pub fn name(self) -> HemName {
        match self {
            HemKind::Closed => HemName::Closed,
            HemKind::Open { .. } => HemName::Open,
            HemKind::Teardrop { .. } => HemName::Teardrop,
            HemKind::Rolled { .. } => HemName::Rolled,
        }
    }
}

/// A hem as the graph keeps it on its joint.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HemJoint {
    pub kind: HemName,
    /// Signed as `JointKind::Bend` (towards the reference side positive), up to a whole turn.
    pub angle: f64,
    /// Inside radius.
    pub radius: f64,
    /// From the shop's hem table; None: the sheet's bend rule.
    pub allowance: Option<f64>,
    /// No leg: the fold ends free at its far tangent line (a rolled hem's).
    pub free: bool,
}

impl HemJoint {
    /// Mold line (the edge, the fold's outside) to tangent line on the wall.
    pub fn setback(&self, t: f64) -> f64 {
        self.radius + t
    }
}

/// One measured hem allowance.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HemRow {
    pub kind: HemName,
    pub thickness: f64,
    /// Inside radius (closed hems: ignored).
    pub radius: f64,
    pub angle: f64,
    pub allowance: f64,
}

/// The shop's hems, as data.
#[derive(Clone, Debug, PartialEq)]
pub struct HemTable {
    /// The gap a closed hem is modelled with (at least `GAP_FLOOR`).
    pub closed_gap: f64,
    pub rows: Vec<HemRow>,
}

impl Default for HemTable {
    fn default() -> HemTable {
        HemTable {
            closed_gap: GAP_FLOOR,
            rows: Vec::new(),
        }
    }
}

impl HemTable {
    pub fn row(&self, kind: HemName, t: f64, radius: f64, angle: f64) -> Option<&HemRow> {
        let same = |a: f64, b: f64| (a - b).abs() <= 1e-9 * a.abs().max(1.0);
        self.rows.iter().find(|r| {
            r.kind == kind
                && same(r.thickness, t)
                && (kind == HemName::Closed || same(r.radius, radius))
                && same(r.angle, angle)
        })
    }
}

/// Why a hem is refused.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum HemError {
    /// A gap between the layers below `limit` (the floor, `GAP_FLOOR`).
    Gap {
        gap: f64,
        limit: f64,
    },
    /// A turn outside (lo, hi) for this kind.
    Angle {
        kind: HemName,
        angle: f64,
        lo: f64,
        hi: f64,
    },
    Radius {
        radius: f64,
        limit: f64,
    },
    /// A leg not positive (only a rolled hem may have none).
    Leg {
        kind: HemName,
        leg: f64,
        limit: f64,
    },
    /// The wall reaches `have` from the edge, the fold takes `need` of it.
    NoRoom {
        wall: WallId,
        have: f64,
        need: f64,
    },
    /// A closed hem's allowance has no radius formula: the shop's table needs a row.
    NotTabled {
        kind: HemName,
        thickness: f64,
    },
}

/// A hem on an edge: its kind, its leg (from the fold's far tangent line), and which way it folds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HemSpec {
    pub kind: HemKind,
    pub leg: f64,
    /// Towards the reference side (the wall's top).
    pub up: bool,
}

fn err(feature: FeatureId, joint: JointId, why: HemError) -> SheetError {
    SheetError::Hem { feature, joint, why }
}

/// How far a wall reaches from its edge p0 -> p1 (the wall on its left).
fn reach(w: &Wall, p0: Point2, p1: Point2) -> f64 {
    let d = (p1 - p0) * (1.0 / p0.distance(p1));
    w.outline
        .iter()
        .flat_map(|l| &l.segments)
        .flat_map(|(s, _)| s.samples(16))
        .map(|q| d.cross(q - p0))
        .fold(0.0, f64::max)
}

impl Sheet {
    /// A hem on straight outline segment `segment` of `wall`, as long as it: a new wall (the leg)
    /// joined by a `JointKind::Hem`. Allowances from `table` where it has a row.
    pub fn hem(
        &mut self,
        wall: WallId,
        segment: u64,
        spec: HemSpec,
        table: Option<&HemTable>,
    ) -> Result<(WallId, JointId), SheetError> {
        let feature = self.feature;
        let t = self.gauge.thickness;
        let joint = JointId(self.next_joint);
        let w = self.wall(wall).ok_or(SheetError::NoSuchWall { feature, wall })?;
        let trim = TrimRef::Outline { wall, segment };
        let (a, b) = straight(w, segment).map_err(|e| match e {
            None => SheetError::NoSuchSegment {
                feature,
                wall,
                segment,
            },
            Some(curve) => SheetError::NotStraight { feature, trim, curve },
        })?;
        if let Some(j) = self
            .joints
            .iter()
            .find(|j| (j.a == wall && j.on_a == segment) || (j.b == wall && j.on_b == segment))
        {
            return Err(SheetError::EdgeTaken {
                feature,
                trim,
                joint: j.id,
            });
        }
        let fail = |why| Err(err(feature, joint, why));
        let name = spec.kind.name();
        let half = dmath::PI;
        let whole = dmath::TAU - tol::ANGULAR;
        let gap_of = |gap: f64| {
            if gap.is_nan() || gap < GAP_FLOOR {
                Err(err(
                    feature,
                    joint,
                    HemError::Gap {
                        gap,
                        limit: GAP_FLOOR,
                    },
                ))
            } else {
                Ok(0.5 * gap)
            }
        };
        let (radius, angle) = match spec.kind {
            HemKind::Closed => (gap_of(table.map_or(GAP_FLOOR, |x| x.closed_gap))?, half),
            HemKind::Open { gap } => (gap_of(gap)?, half),
            HemKind::Teardrop { radius, angle } | HemKind::Rolled { radius, angle } => {
                let lo = if name == HemName::Teardrop {
                    half + tol::ANGULAR
                } else {
                    tol::ANGULAR
                };
                if angle.is_nan() || angle <= lo || angle >= whole {
                    return fail(HemError::Angle {
                        kind: name,
                        angle,
                        lo,
                        hi: whole,
                    });
                }
                if radius.is_nan() || radius <= tol::LINEAR {
                    return fail(HemError::Radius {
                        radius,
                        limit: tol::LINEAR,
                    });
                }
                (radius, angle)
            }
        };
        let free = name == HemName::Rolled && spec.leg == 0.0;
        if !free && (spec.leg.is_nan() || spec.leg <= tol::LINEAR) {
            return fail(HemError::Leg {
                kind: name,
                leg: spec.leg,
                limit: tol::LINEAR,
            });
        }
        let allowance = match table.and_then(|x| x.row(name, t, radius, angle)) {
            Some(r) => Some(r.allowance),
            None if name == HemName::Closed => {
                return fail(HemError::NotTabled {
                    kind: name,
                    thickness: t,
                });
            }
            None => None,
        };
        let hj = HemJoint {
            kind: name,
            angle: if spec.up { angle } else { -angle },
            radius,
            allowance,
            free,
        };
        let s = hj.setback(t);
        // the wall's reach from the edge, less what bends on edges parallel to it take
        let d = (b - a) * (1.0 / a.distance(b));
        let taken: f64 = self
            .joints
            .iter()
            .filter_map(|j| {
                let seg = if j.a == wall {
                    j.on_a
                } else if j.b == wall {
                    j.on_b
                } else {
                    return None;
                };
                let (p, q) = straight(w, seg).ok()?;
                let e = (q - p) * (1.0 / p.distance(q));
                (e.cross(d).abs() <= 1e-9).then(|| match j.kind {
                    JointKind::Bend { angle, radius } => crate::build::setback_of(&self.gauge, radius, angle),
                    JointKind::Hem(h) => h.setback(t),
                    JointKind::Rip { .. } | JointKind::Tangent => 0.0,
                })
            })
            .sum();
        let have = reach(w, a, b) - taken;
        if have - s <= tol::LINEAR {
            return fail(HemError::NoRoom { wall, have, need: s });
        }
        let id = WallId(self.next_wall);
        let len = a.distance(b);
        let x = s + spec.leg.max(0.0);
        let p = Point2::new;
        let c = [p(0.0, 0.0), p(x, 0.0), p(x, len), p(0.0, len)];
        let outline = ProfileLoop::new(
            (0..4)
                .map(|i| {
                    (
                        Segment::Line {
                            a: c[i],
                            b: c[(i + 1) % 4],
                        },
                        i as u64,
                    )
                })
                .collect(),
        );
        self.walls.push(Wall {
            id,
            outline: vec![outline],
        });
        self.joints.push(crate::graph::Joint {
            id: joint,
            a: wall,
            b: id,
            on_a: segment,
            on_b: 3,
            kind: JointKind::Hem(hj),
        });
        self.next_wall += 1;
        self.next_joint += 1;
        Ok((id, joint))
    }
}
