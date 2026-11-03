//! Forming tools: a press stroke that shapes a wall without cutting it (a dimple, an extruded
//! hole). Folded, the form is exact surfaces of revolution (die and punch radii as tori, the wall
//! as a cone or cylinder, the crown or the collar's lip as a plane) stitched onto the wall's
//! skins where its rim leaves them, no booleans. The flat stays plain but for real cuts (an
//! extruded hole's pierce, from the tool's data) and carries each form's centre, angle and
//! direction on a layer of its own. The form's material is checked against the rest of the
//! folded part like any strip; the shop's tools and rules (depth per t, keep-out from bends and
//! edges, spacing, directions) are data.
//!
//! Profiles are drawn in (r, d): r from the form's axis, d the depth from the skin the tool
//! presses from (the near skin), the other skin at d = t. Louvres and lances are typed and
//! refused until they are built.
use crate::error::{SheetError, WallId};
use crate::graph::{JointKind, Sheet};
use crate::region::{Piece, Region, holds, pieces, subtract};
use crate::shop::Scaled;
use forge_geom::{Cone, Cylinder, Surface, Torus};
use forge_math::{Box3, Frame, Interval, Point2, Point3, Vec2, Vec3, dmath, tol};
use forge_ops::{ProfileLoop, Segment};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum FormName {
    Dimple,
    ExtrudedHole,
    Louvre,
    Lance,
}

impl FormName {
    pub fn name(self) -> &'static str {
        match self {
            FormName::Dimple => "dimple",
            FormName::ExtrudedHole => "extruded hole",
            FormName::Louvre => "louvre",
            FormName::Lance => "lance",
        }
    }
}

/// A forming tool's shape. Radii are inside radii (the die's on the far skin at the rim, the
/// punch's on the near skin at a dimple's crown).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FormShape {
    /// `d` across the rim (where the wall leaves the sheet), `depth` from the near skin to the
    /// crown's near skin, the wall at `angle` to the sheet (below a right angle).
    Dimple {
        d: f64,
        depth: f64,
        angle: f64,
        die_r: f64,
        punch_r: f64,
    },
    /// A collar: bore `d` (the near skin's cylinder), `height` from the near skin to the lip,
    /// rounded over the die radius; `pierce` the hole cut in the flat before forming.
    ExtrudedHole {
        d: f64,
        height: f64,
        die_r: f64,
        pierce: f64,
    },
    /// Typed, not built.
    Louvre { length: f64, width: f64, height: f64 },
    /// Typed, not built.
    Lance { length: f64, width: f64, height: f64 },
}

impl FormShape {
    /// What forming adds to a part's folded volume on a sheet `t` thick, exactly (Pappus): the
    /// formed material less the disc of sheet inside its rim.
    pub fn volume_change(&self, t: f64) -> Result<f64, FormError> {
        Ok(Profile::of(self, t)?.volume_change(t))
    }

    /// The rim's radius on a sheet `t` thick: where the form leaves the wall.
    pub fn rim(&self, t: f64) -> Result<f64, FormError> {
        Ok(Profile::of(self, t)?.r1)
    }

    pub fn name(&self) -> FormName {
        match self {
            FormShape::Dimple { .. } => FormName::Dimple,
            FormShape::ExtrudedHole { .. } => FormName::ExtrudedHole,
            FormShape::Louvre { .. } => FormName::Louvre,
            FormShape::Lance { .. } => FormName::Lance,
        }
    }
}

/// A forming tool in the shop's library, with the directions the press can form it.
#[derive(Clone, Debug, PartialEq)]
pub struct FormTool {
    pub id: String,
    pub shape: FormShape,
    pub up: bool,
    pub down: bool,
}

/// The shop's forming rules, each scaled by t (and the tool's die radius as R): the least
/// distance from a form's rim to a bend's tangent line and to the part's edge, the least web
/// between two forms' rims, and the deepest stroke.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FormRules {
    pub bend: Scaled,
    pub edge: Scaled,
    pub spacing: Scaled,
    pub depth: Scaled,
}

impl Default for FormRules {
    /// Rules of thumb, not a standard: 3 t + R to a bend, 3 t to an edge, 2 t between forms,
    /// a stroke at most 6 t.
    fn default() -> FormRules {
        let s = |per_t, per_r| Scaled {
            per_t,
            per_r,
            plus: 0.0,
        };
        FormRules {
            bend: s(3.0, 1.0),
            edge: s(3.0, 0.0),
            spacing: s(2.0, 0.0),
            depth: s(6.0, 0.0),
        }
    }
}

/// One form, as the graph keeps it: `up` presses towards the wall's reference side (+z), so the
/// back skin is the near one; otherwise the reference skin is.
#[derive(Clone, Debug, PartialEq)]
pub struct Form {
    pub wall: WallId,
    pub tool: FormTool,
    pub at: Point2,
    pub angle: f64,
    pub up: bool,
}

/// A form on the flat, for the DXF.
#[derive(Clone, Debug, PartialEq)]
pub struct FormMark {
    pub tool: String,
    pub at: Point2,
    pub angle: f64,
    /// Towards the flat's +z (its reference side).
    pub up: bool,
}

/// Why a form is refused.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FormError {
    /// A size not positive or out of its range (`which` names it).
    Size {
        which: &'static str,
        value: f64,
        limit: f64,
    },
    /// Not built yet.
    Unbuilt { kind: FormName },
    /// The depth leaves no straight wall between the die and punch radii.
    TooShallow { depth: f64, limit: f64 },
    /// The crown's radius left inside the punch radius's turn: the tool closes on itself.
    CrownTooSmall { crown: f64, limit: f64 },
    /// The form's rim (`radius` from `at`) does not lie inside its wall, clear of its bends'
    /// zones, other cuts and other forms: `trim` is what it meets.
    NotClear {
        form: usize,
        trim: crate::error::TrimRef,
    },
}

/// A form against the shop. `form` indexes `Sheet::forms`.
#[derive(Clone, Debug, PartialEq)]
pub enum FormViolation {
    NotInShop {
        tool: String,
    },
    /// The shop's tool does not form this way.
    Direction {
        tool: String,
        up: bool,
    },
    TooDeep {
        depth: f64,
        limit: f64,
    },
    /// The rim `distance` from a bend's tangent line; `away` (unit, the wall's coordinates) is
    /// the way to move it.
    NearBend {
        joint: crate::error::JointId,
        distance: f64,
        limit: f64,
        away: Vec2,
    },
    /// The rim `distance` from the wall's free edge `segment`.
    NearEdge {
        segment: u64,
        distance: f64,
        limit: f64,
        away: Vec2,
    },
    /// The web between this form's rim and form `other`'s.
    Spacing {
        other: usize,
        web: f64,
        limit: f64,
    },
}

/// A piece of a profile in (r, d): an arc's points are c + rho (cos a, sin a), from a0 to a1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Prof {
    Line { a: Point2, b: Point2 },
    Arc { c: Point2, rho: f64, a0: f64, a1: f64 },
}

impl Prof {
    pub(crate) fn at(&self, u: f64) -> Point2 {
        match *self {
            Prof::Line { a, b } => a.lerp(b, u),
            Prof::Arc { c, rho, a0, a1 } => {
                let (s, k) = dmath::sin_cos(a0 + (a1 - a0) * u);
                Point2::new(c.x + rho * k, c.y + rho * s)
            }
        }
    }
    fn segment(&self) -> Segment {
        match *self {
            Prof::Line { a, b } => Segment::Line { a, b },
            Prof::Arc { c, rho, a0, a1 } => Segment::Arc {
                center: c,
                radius: rho,
                start: a0,
                sweep: a1 - a0,
            },
        }
    }
    fn reversed(&self) -> Prof {
        match *self {
            Prof::Line { a, b } => Prof::Line { a: b, b: a },
            Prof::Arc { c, rho, a0, a1 } => Prof::Arc {
                c,
                rho,
                a0: a1,
                a1: a0,
            },
        }
    }
}

/// How a form's two skins close: a dimple's crown (a disc on each), an extruded hole's lip (one
/// annulus joining them).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Close {
    Crown,
    Lip,
}

/// A built form's profile: the near skin's chain and the far skin's, piece by piece from the rim
/// inwards (each far piece the near one's offset by t), and how they close.
#[derive(Clone, Debug)]
pub(crate) struct Profile {
    pub near: Vec<Prof>,
    pub far: Vec<Prof>,
    pub close: Close,
    /// The rim's radius: where the wall leaves the sheet.
    pub r1: f64,
    /// The deepest d the material reaches.
    pub reach: f64,
    /// The die radius (R in the rules).
    pub die_r: f64,
    /// The stroke: the crown's or lip's depth below the near skin.
    pub depth: f64,
    /// The flat's pierce radius, if the flat is cut.
    pub pierce: Option<f64>,
    /// The cross-section's region, and its boundary less the axis (for depths).
    section: Region,
    rims: Vec<Piece>,
}

fn size(which: &'static str, value: f64, limit: f64) -> FormError {
    FormError::Size { which, value, limit }
}

fn positive(which: &'static str, v: f64) -> Result<f64, FormError> {
    if v.is_finite() && v > tol::LINEAR {
        Ok(v)
    } else {
        Err(size(which, v, tol::LINEAR))
    }
}

impl Profile {
    pub(crate) fn of(shape: &FormShape, t: f64) -> Result<Profile, FormError> {
        let p = Point2::new;
        let half = dmath::PI / 2.0;
        let (near, far, close, r1, reach, die_r, depth, pierce) = match *shape {
            FormShape::Louvre { .. } | FormShape::Lance { .. } => {
                return Err(FormError::Unbuilt { kind: shape.name() });
            }
            FormShape::Dimple {
                d,
                depth,
                angle,
                die_r,
                punch_r,
            } => {
                let r1 = 0.5 * positive("d", d)?;
                let h = positive("depth", depth)?;
                let (rd, rp) = (positive("die_r", die_r)?, positive("punch_r", punch_r)?);
                // a cone between the sheet and the axis: not flat, not a cylinder (Cone's range)
                let (lo, hi) = (1e-6, half - tol::ANGULAR);
                if !(angle > lo && angle < hi) {
                    return Err(size("angle", angle, if angle <= lo { lo } else { hi }));
                }
                let (sb, cb) = dmath::sin_cos(angle);
                let (rho1, rho2) = (rd + t, rp);
                // the rim's die radius turns round the rim's circle: a ring torus
                if r1 <= rho1 + tol::LINEAR {
                    return Err(size("d", d, 2.0 * (rho1 + tol::LINEAR)));
                }
                let turn = (rho1 + rho2) * (1.0 - cb);
                let len = (h - turn) / sb;
                if len <= tol::LINEAR {
                    return Err(FormError::TooShallow {
                        depth: h,
                        limit: turn + tol::LINEAR * sb,
                    });
                }
                let r2 = r1 - (rho1 + rho2) * sb - len * cb;
                // the far skin's punch radius turns round a circle r2 out: a ring torus
                let need = rho2 + t + tol::LINEAR;
                if r2 <= need {
                    return Err(FormError::CrownTooSmall {
                        crown: r2,
                        limit: need,
                    });
                }
                let c1 = p(r1, rho1);
                let c2 = p(r2, h - rho2);
                let a1 = (-half, -half - angle);
                let a2 = (half - angle, half);
                let q1 = Prof::Arc {
                    c: c1,
                    rho: rho1,
                    a0: a1.0,
                    a1: a1.1,
                }
                .at(1.0);
                let q2 = Prof::Arc {
                    c: c2,
                    rho: rho2,
                    a0: a2.0,
                    a1: a2.1,
                }
                .at(0.0);
                let n = Vec2::new(sb, cb) * t;
                let near = vec![
                    Prof::Arc {
                        c: c1,
                        rho: rho1,
                        a0: a1.0,
                        a1: a1.1,
                    },
                    Prof::Line { a: q1, b: q2 },
                    Prof::Arc {
                        c: c2,
                        rho: rho2,
                        a0: a2.0,
                        a1: a2.1,
                    },
                ];
                let far = vec![
                    Prof::Arc {
                        c: c1,
                        rho: rho1 - t,
                        a0: a1.0,
                        a1: a1.1,
                    },
                    Prof::Line { a: q1 + n, b: q2 + n },
                    Prof::Arc {
                        c: c2,
                        rho: rho2 + t,
                        a0: a2.0,
                        a1: a2.1,
                    },
                ];
                (near, far, Close::Crown, r1, h + t, rd, h, None)
            }
            FormShape::ExtrudedHole {
                d,
                height,
                die_r,
                pierce,
            } => {
                let rb = 0.5 * positive("d", d)?;
                let hh = positive("height", height)?;
                let rd = positive("die_r", die_r)?;
                let pr = 0.5 * positive("pierce", pierce)?;
                if pr >= rb - tol::LINEAR {
                    return Err(size("pierce", pierce, d));
                }
                let rho1 = rd + t;
                if hh - rho1 <= tol::LINEAR {
                    return Err(FormError::TooShallow {
                        depth: hh,
                        limit: rho1 + tol::LINEAR,
                    });
                }
                let r1 = rb + rho1;
                let c1 = p(r1, rho1);
                let near = vec![
                    Prof::Arc {
                        c: c1,
                        rho: rho1,
                        a0: -half,
                        a1: -dmath::PI,
                    },
                    Prof::Line {
                        a: p(rb, rho1),
                        b: p(rb, hh),
                    },
                ];
                let far = vec![
                    Prof::Arc {
                        c: c1,
                        rho: rd,
                        a0: -half,
                        a1: -dmath::PI,
                    },
                    Prof::Line {
                        a: p(rb + t, rho1),
                        b: p(rb + t, hh),
                    },
                ];
                (near, far, Close::Lip, r1, hh, rd, hh, Some(pr))
            }
        };
        // the cross-section, counter-clockwise: the far chain out to the rim, down the rim's
        // line, the near chain in, and closed by the crown and the axis or by the lip
        let mut loop_: Vec<(Prof, bool)> = Vec::new();
        let last_near = near.last().expect("a chain").at(1.0);
        let last_far = far.last().expect("a chain").at(1.0);
        match close {
            Close::Crown => {
                loop_.push((
                    Prof::Line {
                        a: p(0.0, last_far.y),
                        b: last_far,
                    },
                    true,
                ));
            }
            Close::Lip => {}
        }
        for f in far.iter().rev() {
            loop_.push((f.reversed(), true));
        }
        loop_.push((
            Prof::Line {
                a: p(r1, t),
                b: p(r1, 0.0),
            },
            true,
        ));
        for n in &near {
            loop_.push((*n, true));
        }
        match close {
            Close::Crown => {
                loop_.push((
                    Prof::Line {
                        a: last_near,
                        b: p(0.0, last_near.y),
                    },
                    true,
                ));
                // the axis is no boundary of the solid of revolution
                loop_.push((
                    Prof::Line {
                        a: p(0.0, last_near.y),
                        b: p(0.0, last_far.y),
                    },
                    false,
                ));
            }
            Close::Lip => loop_.push((
                Prof::Line {
                    a: last_near,
                    b: last_far,
                },
                true,
            )),
        }
        let lp = ProfileLoop::new(
            loop_
                .iter()
                .enumerate()
                .map(|(i, (q, _))| (q.segment(), i as u64))
                .collect(),
        );
        let area: f64 = pieces(&lp).iter().map(|q| q.0.area_term()).sum();
        let lp = if area < 0.0 {
            ProfileLoop::new(
                loop_
                    .iter()
                    .enumerate()
                    .rev()
                    .map(|(i, (q, _))| (q.reversed().segment(), i as u64))
                    .collect(),
            )
        } else {
            lp
        };
        let section = subtract(std::slice::from_ref(&lp), &[]).unwrap_or_default();
        let rims = loop_
            .iter()
            .filter(|q| q.1)
            .flat_map(|(q, _)| {
                pieces(&ProfileLoop::new(vec![(q.segment(), 0)]))
                    .into_iter()
                    .map(|x| x.0)
            })
            .collect();
        Ok(Profile {
            near,
            far,
            close,
            r1,
            reach,
            die_r,
            depth,
            pierce,
            section,
            rims,
        })
    }

    /// How far (r, d) lies inside the form's material: its distance to the section's boundary
    /// (the axis is none), None outside.
    pub(crate) fn depth_at(&self, q: Point2) -> Option<f64> {
        if !self.section.parts.iter().any(|c| holds(c, q)) {
            return None;
        }
        Some(
            self.rims
                .iter()
                .map(|b| b.at(b.nearest(q)).distance(q))
                .fold(f64::INFINITY, f64::min),
        )
    }

    /// The formed material's volume (Pappus, piece by piece) less the disc of sheet it replaces.
    pub(crate) fn volume_change(&self, t: f64) -> f64 {
        moment_volume(self, t) - dmath::PI * self.r1 * self.r1 * t
    }
}

/// 2 pi times the first moment about the axis of the material between the chains.
fn moment_volume(p: &Profile, t: f64) -> f64 {
    let mut m = 0.0;
    for (a, b) in p.near.iter().zip(&p.far) {
        m += match (*a, *b) {
            (Prof::Arc { c, rho, a0, a1 }, Prof::Arc { rho: rho_b, .. }) => {
                // an annular sector: integral of (c.x + s cos a) s ds da
                let (lo, hi) = (rho.min(rho_b), rho.max(rho_b));
                let sweep = (a1 - a0).abs();
                let (s0, s1) = if a1 > a0 { (a0, a1) } else { (a1, a0) };
                c.x * 0.5 * (hi * hi - lo * lo) * sweep
                    + (hi.powi(3) - lo.powi(3)) / 3.0 * (dmath::sin(s1) - dmath::sin(s0))
            }
            (Prof::Line { a, b: e }, Prof::Line { a: fa, b: fb }) => {
                // a rectangle t across: its area times its centroid's r
                let len = a.distance(e);
                len * t * 0.25 * (a.x + e.x + fa.x + fb.x)
            }
            _ => unreachable!("chains match piece by piece"),
        };
    }
    if p.close == Close::Crown {
        let n = p.near.last().expect("a chain").at(1.0);
        m += t * 0.5 * n.x * n.x;
    }
    dmath::TAU * m
}

impl Sheet {
    /// Forms `tool` on `wall` at `at` (its coordinates), turned `angle`, pressed `up` (towards
    /// the reference side) or down. All or nothing.
    pub fn form(
        &mut self,
        wall: WallId,
        tool: &FormTool,
        at: Point2,
        angle: f64,
        up: bool,
    ) -> Result<usize, SheetError> {
        let feature = self.feature;
        self.wall(wall).ok_or(SheetError::NoSuchWall { feature, wall })?;
        let form = self.forms.len();
        let err = |why| SheetError::Form { feature, form, why };
        if !(at.x.is_finite() && at.y.is_finite()) {
            return Err(err(size(
                "at",
                if at.x.is_finite() { at.y } else { at.x },
                f64::MAX,
            )));
        }
        if !angle.is_finite() {
            return Err(err(size("angle", angle, f64::MAX)));
        }
        Profile::of(&tool.shape, self.gauge.thickness).map_err(err)?;
        self.forms.push(Form {
            wall,
            tool: tool.clone(),
            at,
            angle,
            up,
        });
        Ok(self.forms.len() - 1)
    }
}

/// The forms on the flat: each at its wall's developed place, up as the flat's +z.
pub fn marks(sheet: &Sheet, layout: &crate::build::Layout) -> Vec<FormMark> {
    sheet
        .forms
        .iter()
        .filter_map(|f| {
            let w = layout.walls.iter().find(|w| w.wall == f.wall)?;
            // every wall develops reference side up: the direction carries over
            let d = w.developed;
            Some(FormMark {
                tool: f.tool.id.clone(),
                at: d.apply(f.at),
                angle: f.angle + dmath::atan2(d.x.y, d.x.x),
                up: f.up,
            })
        })
        .collect()
}

/// Each form against the shop's library and rules.
pub fn check(sheet: &Sheet, tools: &[FormTool], rules: &FormRules) -> Vec<(usize, FormViolation)> {
    let t = sheet.gauge.thickness;
    let mut out = Vec::new();
    let profiles: Vec<Option<Profile>> = sheet
        .forms
        .iter()
        .map(|f| Profile::of(&f.tool.shape, t).ok())
        .collect();
    for (i, f) in sheet.forms.iter().enumerate() {
        let Some(p) = &profiles[i] else { continue };
        match tools
            .iter()
            .find(|x| x.id == f.tool.id && x.shape == f.tool.shape)
        {
            None => out.push((
                i,
                FormViolation::NotInShop {
                    tool: f.tool.id.clone(),
                },
            )),
            Some(x) if !(if f.up { x.up } else { x.down }) => out.push((
                i,
                FormViolation::Direction {
                    tool: f.tool.id.clone(),
                    up: f.up,
                },
            )),
            Some(_) => {}
        }
        let deepest = rules.depth.at(t, p.die_r);
        if p.depth > deepest + tol::LINEAR {
            out.push((
                i,
                FormViolation::TooDeep {
                    depth: p.depth,
                    limit: deepest,
                },
            ));
        }
        let Some(w) = sheet.wall(f.wall) else { continue };
        // each straight outline edge: a bend's (to its tangent line) or a free edge
        for l in &w.outline {
            for (seg, id) in &l.segments {
                let joint = sheet
                    .joints
                    .iter()
                    .find(|j| (j.a == f.wall && j.on_a == *id) || (j.b == f.wall && j.on_b == *id));
                let setback = match joint.map(|j| j.kind) {
                    Some(JointKind::Bend { angle, radius }) => {
                        crate::build::setback_of(&sheet.gauge, radius, angle)
                    }
                    Some(JointKind::Hem(h)) => h.setback(t),
                    Some(_) => 0.0,
                    None => 0.0,
                };
                // the nearest point of the edge (a line: exactly), and the way away from it
                let (near, away) = match *seg {
                    Segment::Line { a, b } if joint.is_some() => {
                        let d = (b - a) * (1.0 / a.distance(b));
                        let inward = Vec2::new(-d.y, d.x);
                        ((f.at - a).dot(inward), inward)
                    }
                    _ => {
                        let pc = pieces(&ProfileLoop::new(vec![(seg.clone(), *id)]));
                        let Some((q, _)) = pc.first() else { continue };
                        let foot = q.at(q.nearest(f.at));
                        let v = f.at - foot;
                        let len = v.length();
                        (
                            len,
                            if len > 0.0 {
                                v * (1.0 / len)
                            } else {
                                Vec2::new(0.0, 0.0)
                            },
                        )
                    }
                };
                let distance = near - setback - p.r1;
                match joint {
                    Some(j) => {
                        let r = match j.kind {
                            JointKind::Bend { radius, .. } => radius,
                            JointKind::Hem(h) => h.radius,
                            _ => 0.0,
                        };
                        let limit = rules.bend.at(t, r);
                        if distance < limit - tol::LINEAR {
                            out.push((
                                i,
                                FormViolation::NearBend {
                                    joint: j.id,
                                    distance,
                                    limit,
                                    away,
                                },
                            ));
                        }
                    }
                    None => {
                        let limit = rules.edge.at(t, 0.0);
                        if distance < limit - tol::LINEAR {
                            out.push((
                                i,
                                FormViolation::NearEdge {
                                    segment: *id,
                                    distance,
                                    limit,
                                    away,
                                },
                            ));
                        }
                    }
                }
            }
        }
        let limit = rules.spacing.at(t, 0.0);
        for (k, g) in sheet.forms.iter().enumerate().skip(i + 1) {
            let Some(q) = &profiles[k] else { continue };
            if g.wall != f.wall {
                continue;
            }
            let web = f.at.distance(g.at) - p.r1 - q.r1;
            if web < limit - tol::LINEAR {
                out.push((i, FormViolation::Spacing { other: k, web, limit }));
            }
        }
    }
    out
}

/// A form as the build lays it out: its wall's strip, the wall's folded frame and its frame under
/// the map, its centre in the wall's coordinates, its direction and profile.
pub(crate) struct FormGeo {
    pub index: usize,
    pub strip: usize,
    pub folded: Frame,
    pub frame: Frame,
    pub c: Point2,
    pub up: bool,
    pub p: Profile,
}

/// A curve of a form's, for the clash check: an arc o + r (cos a x + sin a y), or a segment.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Curve3 {
    Arc {
        o: Point3,
        x: Vec3,
        y: Vec3,
        r: f64,
        a0: f64,
        a1: f64,
    },
    Seg(Point3, Point3),
}

impl Curve3 {
    pub(crate) fn at(&self, u: f64) -> Point3 {
        match *self {
            Curve3::Arc { o, x, y, r, a0, a1 } => {
                let (s, k) = dmath::sin_cos(a0 + (a1 - a0) * u);
                o + x * (r * k) + y * (r * s)
            }
            Curve3::Seg(a, b) => a + (b - a) * u,
        }
    }
    /// A box holding the curve over fractions `u`: the chord's grown by the arc's sagitta (an arc
    /// within half a turn stays between its chord's ends), a whole circle's past half a turn.
    pub(crate) fn bbox(&self, u: Interval) -> Box3 {
        let (p, q) = (self.at(u.lo), self.at(u.hi));
        let chord = Box3::from_points([p, q]);
        match *self {
            Curve3::Seg(..) => chord.grow(tol::LINEAR),
            Curve3::Arc { o, r, a0, a1, .. } => {
                let turn = (a1 - a0).abs() * u.width();
                if turn <= dmath::PI {
                    chord.grow(r * (1.0 - dmath::cos(0.5 * turn)) + tol::LINEAR)
                } else {
                    Box3::from_points([o]).grow(r + tol::LINEAR)
                }
            }
        }
    }
}

impl FormGeo {
    /// The world direction of +d.
    pub(crate) fn down(&self, f: &Frame) -> Vec3 {
        let z = f.z().get();
        if self.up { z } else { -z }
    }
    pub(crate) fn radial(&self, f: &Frame, phi: f64) -> Vec3 {
        let (s, k) = dmath::sin_cos(phi);
        f.x().get() * k + f.y().get() * s
    }
    /// The axis's point at depth d.
    pub(crate) fn axis(&self, f: &Frame, d: f64, t: f64) -> Point3 {
        let z = if self.up { -t + d } else { -d };
        f.point_to_world(Point3::new(self.c.x, self.c.y, z))
    }
    /// (r, d) at angle phi.
    pub(crate) fn point(&self, f: &Frame, phi: f64, q: Point2, t: f64) -> Point3 {
        self.axis(f, q.y, t) + self.radial(f, phi) * q.x
    }
    /// The wall's frame moved onto the axis at depth d.
    pub(crate) fn axis_frame(&self, f: &Frame, d: f64, t: f64) -> Frame {
        f.with_origin(self.axis(f, d, t))
    }

    /// The surface a profile piece sweeps.
    pub(crate) fn surface(&self, f: &Frame, q: &Prof, t: f64) -> Option<Surface> {
        match *q {
            Prof::Arc { c, rho, .. } => Torus::new(self.axis_frame(f, c.y, t), c.x, rho).map(Surface::Torus),
            Prof::Line { a, b } if (a.x - b.x).abs() <= 1e-12 * a.x.abs().max(1.0) => {
                Some(Surface::Cylinder(Cylinder {
                    frame: self.axis_frame(f, a.y, t),
                    radius: a.x,
                }))
            }
            Prof::Line { a, b } => {
                // from the smaller end, its axis the way the radius grows
                let (lo, hi) = if a.x <= b.x { (a, b) } else { (b, a) };
                let k = (hi.x - lo.x) / (hi.y - lo.y);
                let down = self.down(f);
                let z = if k > 0.0 { down } else { -down };
                let frame = Frame::from_zx(self.axis(f, lo.y, t), z.normalize()?, f.x().get())?;
                Cone::new(frame, lo.x, dmath::atan(k.abs())).map(Surface::Cone)
            }
        }
    }

    /// Where (r, d) lies in the form's material (folded): its depth, None outside.
    pub(crate) fn depth(&self, p: Point3, t: f64) -> Option<f64> {
        let l = self.folded.point_to_local(p);
        let (x, y) = (l.x - self.c.x, l.y - self.c.y);
        let d = if self.up { l.z + t } else { -l.z };
        self.p.depth_at(Point2::new((x * x + y * y).sqrt(), d))
    }

    /// Whether a box may hold a point deeper than `limit` in the form's material (folded).
    pub(crate) fn may_reach(&self, bx: &Box3, limit: f64, t: f64) -> bool {
        let corners = [0, 1, 2, 3, 4, 5, 6, 7].map(|k| {
            self.folded.point_to_local(Point3::new(
                if k & 1 == 0 { bx.x.lo } else { bx.x.hi },
                if k & 2 == 0 { bx.y.lo } else { bx.y.hi },
                if k & 4 == 0 { bx.z.lo } else { bx.z.hi },
            ))
        });
        let l = Box3::from_points(corners);
        let near = |i: Interval, c: f64| {
            let (lo, hi) = (i.lo - c, i.hi - c);
            if lo > 0.0 {
                lo
            } else if hi < 0.0 {
                -hi
            } else {
                0.0
            }
        };
        let r = (near(l.x, self.c.x).powi(2) + near(l.y, self.c.y).powi(2)).sqrt();
        let (d0, d1) = if self.up {
            (l.z.lo + t, l.z.hi + t)
        } else {
            (-l.z.hi, -l.z.lo)
        };
        r < self.p.r1 - limit && d1 > limit && d0 < self.p.reach - limit
    }

    /// The world box of the form's material (folded): its cylinder's.
    pub(crate) fn bbox(&self, t: f64) -> Box3 {
        let r = self.p.r1;
        let (z0, z1) = if self.up {
            (-t, -t + self.p.reach)
        } else {
            (-self.p.reach, 0.0)
        };
        Box3::from_points([0, 1, 2, 3, 4, 5, 6, 7].map(|k| {
            self.folded.point_to_world(Point3::new(
                self.c.x + if k & 1 == 0 { -r } else { r },
                self.c.y + if k & 2 == 0 { -r } else { r },
                if k & 4 == 0 { z0 } else { z1 },
            ))
        }))
        .grow(tol::LINEAR)
    }

    /// The form's edges (folded) for the clash check: every node's circle on both chains, and
    /// the profile (both chains, the closing pieces, the rim's line through the sheet) at four
    /// angles.
    pub(crate) fn curves(&self, t: f64) -> Vec<Curve3> {
        let f = &self.folded;
        let mut out = Vec::new();
        let (x, y) = (f.x().get(), f.y().get());
        for chain in [&self.p.near, &self.p.far] {
            for q in chain
                .iter()
                .map(|q| q.at(0.0))
                .chain(chain.last().map(|q| q.at(1.0)))
            {
                out.push(Curve3::Arc {
                    o: self.axis(f, q.y, t),
                    x,
                    y,
                    r: q.x,
                    a0: 0.0,
                    a1: dmath::TAU,
                });
            }
        }
        let mut profile: Vec<Prof> = self.p.near.iter().chain(&self.p.far).copied().collect();
        let (n, m) = (
            self.p.near.last().expect("a chain").at(1.0),
            self.p.far.last().expect("a chain").at(1.0),
        );
        match self.p.close {
            Close::Crown => {
                profile.push(Prof::Line {
                    a: n,
                    b: Point2::new(0.0, n.y),
                });
                profile.push(Prof::Line {
                    a: m,
                    b: Point2::new(0.0, m.y),
                });
            }
            Close::Lip => profile.push(Prof::Line { a: n, b: m }),
        }
        profile.push(Prof::Line {
            a: Point2::new(self.p.r1, 0.0),
            b: Point2::new(self.p.r1, t),
        });
        for k in 0..4 {
            let phi = k as f64 * dmath::PI / 2.0;
            let down = self.down(f);
            let radial = self.radial(f, phi);
            for q in &profile {
                out.push(match *q {
                    Prof::Line { a, b } => Curve3::Seg(self.point(f, phi, a, t), self.point(f, phi, b, t)),
                    Prof::Arc { c, rho, a0, a1 } => Curve3::Arc {
                        o: self.point(f, phi, c, t),
                        x: radial,
                        y: down,
                        r: rho,
                        a0,
                        a1,
                    },
                });
            }
        }
        out
    }
}
