use crate::dual::{D, N, V};
use forge_math::{Frame, Point2};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntityId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ConstraintId(pub u64);

/// Sketch geometry. Points own two parameters, a circle owns its radius; lines
/// and arcs are defined by points, so shared points join them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Entity {
    Point {
        x: usize,
        y: usize,
    },
    Line {
        a: EntityId,
        b: EntityId,
    },
    Circle {
        center: EntityId,
        r: usize,
    },
    /// Runs from `start` to `end` around `center`, counter-clockwise when `ccw`.
    /// Its radius is |start - center|; |end - center| is held equal to it.
    Arc {
        center: EntityId,
        start: EntityId,
        end: EntityId,
        ccw: bool,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Constraint {
    /// Holds a point where it was when the constraint was added.
    Fixed(EntityId),
    Coincident(EntityId, EntityId),
    Horizontal(EntityId),
    Vertical(EntityId),
    /// Point to point.
    Distance(EntityId, EntityId, f64),
    /// Point to line, keeping the side the point is on.
    DistancePointLine(EntityId, EntityId, f64),
    Length(EntityId, f64),
    Radius(EntityId, f64),
    Diameter(EntityId, f64),
    /// Two lines of equal length, or two circles/arcs of equal radius.
    Equal(EntityId, EntityId),
    Parallel(EntityId, EntityId),
    Perpendicular(EntityId, EntityId),
    /// Angle from the first line's direction to the second's, counter-clockwise (radians).
    Angle(EntityId, EntityId, f64),
    PointOnLine(EntityId, EntityId),
    PointOnCircle(EntityId, EntityId),
    Midpoint(EntityId, EntityId),
    /// Two points mirrored about a line.
    Symmetric(EntityId, EntityId, EntityId),
    /// Line and circle/arc, or two circles/arcs; the side (and inner or outer
    /// contact) is taken from the geometry when the constraint is added.
    Tangent(EntityId, EntityId),
    Concentric(EntityId, EntityId),
    /// second.x - first.x = d
    HorizontalDistance(EntityId, EntityId, f64),
    /// second.y - first.y = d
    VerticalDistance(EntityId, EntityId, f64),
}

#[derive(Clone, Debug, PartialEq)]
pub enum SketchError {
    UnknownEntity(EntityId),
    UnknownConstraint(ConstraintId),
    /// The entity is not of a kind this constraint or operation accepts.
    WrongKind(EntityId),
    BadValue,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct EntityRec {
    pub id: EntityId,
    pub kind: Entity,
    pub construction: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ConstraintRec {
    pub id: ConstraintId,
    pub c: Constraint,
    /// Values captured when added: Fixed's position, or the side of a tangency.
    pub data: [f64; 2],
}

/// A parametric 2D sketch on a plane.
#[derive(Clone, Debug, PartialEq)]
pub struct Sketch {
    pub plane: Frame,
    pub(crate) params: Vec<f64>,
    pub(crate) entities: Vec<EntityRec>,
    pub(crate) constraints: Vec<ConstraintRec>,
    next_constraint: u64,
}

/// One constraint's residual rows: one or two.
pub(crate) struct Rows {
    pub d: [D; 2],
    pub n: usize,
}

impl Rows {
    fn one(a: D) -> Rows {
        Rows {
            d: [a, D::c(0.0)],
            n: 1,
        }
    }
    fn two(a: D, b: D) -> Rows {
        Rows { d: [a, b], n: 2 }
    }
}

/// Parameters touched by one constraint and their values.
pub(crate) struct Local<'a> {
    x: &'a [f64],
    pub vars: [usize; N],
    pub n: usize,
}

impl<'a> Local<'a> {
    pub fn new(x: &'a [f64]) -> Self {
        Local {
            x,
            vars: [0; N],
            n: 0,
        }
    }
    fn var(&mut self, i: usize) -> D {
        let slot = match self.vars[..self.n].iter().position(|&v| v == i) {
            Some(s) => s,
            None => {
                assert!(self.n < N, "a constraint touches more than {N} parameters");
                self.vars[self.n] = i;
                self.n += 1;
                self.n - 1
            }
        };
        let mut d = D::c(self.x[i]);
        d.g[slot] = 1.0;
        d
    }
}

fn sign(v: f64) -> f64 {
    if v < 0.0 { -1.0 } else { 1.0 }
}

impl Sketch {
    pub fn new(plane: Frame) -> Self {
        Sketch {
            plane,
            params: Vec::new(),
            entities: Vec::new(),
            constraints: Vec::new(),
            next_constraint: 1,
        }
    }

    fn push(&mut self, kind: Entity) -> EntityId {
        let id = EntityId(self.entities.len() as u64 + 1);
        self.entities.push(EntityRec {
            id,
            kind,
            construction: false,
        });
        id
    }

    pub fn add_point(&mut self, p: Point2) -> EntityId {
        let x = self.params.len();
        self.params.extend([p.x, p.y]);
        self.push(Entity::Point { x, y: x + 1 })
    }

    pub fn add_line(&mut self, a: EntityId, b: EntityId) -> Result<EntityId, SketchError> {
        self.expect_point(a)?;
        self.expect_point(b)?;
        Ok(self.push(Entity::Line { a, b }))
    }

    /// A line with two new end points.
    pub fn add_line_points(&mut self, a: Point2, b: Point2) -> EntityId {
        let (pa, pb) = (self.add_point(a), self.add_point(b));
        self.push(Entity::Line { a: pa, b: pb })
    }

    pub fn add_circle(&mut self, center: EntityId, radius: f64) -> Result<EntityId, SketchError> {
        self.expect_point(center)?;
        if !(radius > 0.0 && radius.is_finite()) {
            return Err(SketchError::BadValue);
        }
        let r = self.params.len();
        self.params.push(radius);
        Ok(self.push(Entity::Circle { center, r }))
    }

    pub fn add_arc(
        &mut self,
        center: EntityId,
        start: EntityId,
        end: EntityId,
        ccw: bool,
    ) -> Result<EntityId, SketchError> {
        self.expect_point(center)?;
        self.expect_point(start)?;
        self.expect_point(end)?;
        Ok(self.push(Entity::Arc {
            center,
            start,
            end,
            ccw,
        }))
    }

    pub fn set_construction(&mut self, id: EntityId, on: bool) -> Result<(), SketchError> {
        self.rec_mut(id)?.construction = on;
        Ok(())
    }

    pub fn is_construction(&self, id: EntityId) -> bool {
        self.rec(id).is_ok_and(|r| r.construction)
    }

    pub fn entity(&self, id: EntityId) -> Option<Entity> {
        self.rec(id).ok().map(|r| r.kind)
    }

    pub fn entity_ids(&self) -> impl Iterator<Item = EntityId> + '_ {
        self.entities.iter().map(|r| r.id)
    }

    pub fn constraint_ids(&self) -> impl Iterator<Item = ConstraintId> + '_ {
        self.constraints.iter().map(|r| r.id)
    }

    pub fn constraint(&self, id: ConstraintId) -> Option<Constraint> {
        self.constraints.iter().find(|r| r.id == id).map(|r| r.c)
    }

    pub fn point(&self, id: EntityId) -> Point2 {
        match self.rec(id).map(|r| r.kind) {
            Ok(Entity::Point { x, y }) => Point2::new(self.params[x], self.params[y]),
            _ => panic!("{id:?} is not a point"),
        }
    }

    /// Radius of a circle or arc.
    pub fn radius(&self, id: EntityId) -> f64 {
        match self.rec(id).map(|r| r.kind) {
            Ok(Entity::Circle { r, .. }) => self.params[r],
            Ok(Entity::Arc { center, start, .. }) => self.point(start).distance(self.point(center)),
            _ => panic!("{id:?} is not round"),
        }
    }

    pub fn params(&self) -> &[f64] {
        &self.params
    }

    /// Replaces all parameter values (e.g. to restore a snapshot); the length must match.
    pub fn set_params(&mut self, p: &[f64]) -> Result<(), SketchError> {
        if p.len() != self.params.len() {
            return Err(SketchError::BadValue);
        }
        self.params.copy_from_slice(p);
        Ok(())
    }

    pub fn move_point(&mut self, id: EntityId, p: Point2) -> Result<(), SketchError> {
        let (x, y) = self.point_params(id)?;
        self.params[x] = p.x;
        self.params[y] = p.y;
        Ok(())
    }

    pub(crate) fn rec(&self, id: EntityId) -> Result<&EntityRec, SketchError> {
        id.0.checked_sub(1)
            .and_then(|i| self.entities.get(i as usize))
            .filter(|r| r.id == id)
            .ok_or(SketchError::UnknownEntity(id))
    }

    fn rec_mut(&mut self, id: EntityId) -> Result<&mut EntityRec, SketchError> {
        id.0.checked_sub(1)
            .and_then(|i| self.entities.get_mut(i as usize))
            .filter(|r| r.id == id)
            .ok_or(SketchError::UnknownEntity(id))
    }

    pub(crate) fn point_params(&self, id: EntityId) -> Result<(usize, usize), SketchError> {
        match self.rec(id)?.kind {
            Entity::Point { x, y } => Ok((x, y)),
            _ => Err(SketchError::WrongKind(id)),
        }
    }

    /// Every parameter an entity depends on.
    pub(crate) fn entity_params(&self, id: EntityId) -> Vec<usize> {
        let pts = |ids: &[EntityId]| -> Vec<usize> {
            ids.iter()
                .flat_map(|&p| self.point_params(p).map(|(x, y)| [x, y]).unwrap_or_default())
                .collect()
        };
        match self.rec(id).map(|r| r.kind) {
            Ok(Entity::Point { x, y }) => vec![x, y],
            Ok(Entity::Line { a, b }) => pts(&[a, b]),
            Ok(Entity::Circle { center, r }) => {
                let mut v = pts(&[center]);
                v.push(r);
                v
            }
            Ok(Entity::Arc {
                center, start, end, ..
            }) => pts(&[center, start, end]),
            Err(_) => vec![],
        }
    }

    fn expect_point(&self, id: EntityId) -> Result<(), SketchError> {
        self.point_params(id).map(|_| ())
    }
    fn expect_line(&self, id: EntityId) -> Result<(), SketchError> {
        match self.rec(id)?.kind {
            Entity::Line { .. } => Ok(()),
            _ => Err(SketchError::WrongKind(id)),
        }
    }
    fn is_round(&self, id: EntityId) -> Result<bool, SketchError> {
        Ok(matches!(
            self.rec(id)?.kind,
            Entity::Circle { .. } | Entity::Arc { .. }
        ))
    }
    fn expect_round(&self, id: EntityId) -> Result<(), SketchError> {
        if self.is_round(id)? {
            Ok(())
        } else {
            Err(SketchError::WrongKind(id))
        }
    }

    pub fn add_constraint(&mut self, c: Constraint) -> Result<ConstraintId, SketchError> {
        use Constraint::*;
        let finite = |v: f64| {
            if v.is_finite() {
                Ok(())
            } else {
                Err(SketchError::BadValue)
            }
        };
        let positive = |v: f64| {
            if v > 0.0 && v.is_finite() {
                Ok(())
            } else {
                Err(SketchError::BadValue)
            }
        };
        match c {
            Fixed(p) => self.expect_point(p)?,
            Coincident(p, q) => {
                self.expect_point(p)?;
                self.expect_point(q)?;
            }
            Concentric(a, b) => {
                self.expect_round(a)?;
                self.expect_round(b)?;
            }
            Horizontal(l) | Vertical(l) => self.expect_line(l)?,
            Distance(p, q, d) => {
                self.expect_point(p)?;
                self.expect_point(q)?;
                positive(d)?;
            }
            DistancePointLine(p, l, d) => {
                self.expect_point(p)?;
                self.expect_line(l)?;
                finite(d)?;
                if d < 0.0 {
                    return Err(SketchError::BadValue);
                }
            }
            Length(l, d) => {
                self.expect_line(l)?;
                positive(d)?;
            }
            Radius(e, d) | Diameter(e, d) => {
                self.expect_round(e)?;
                positive(d)?;
            }
            Equal(a, b) => {
                if self.is_round(a)? != self.is_round(b)? {
                    return Err(SketchError::WrongKind(b));
                }
                if !self.is_round(a)? {
                    self.expect_line(a)?;
                    self.expect_line(b)?;
                }
            }
            Parallel(a, b) | Perpendicular(a, b) => {
                self.expect_line(a)?;
                self.expect_line(b)?;
            }
            Angle(a, b, t) => {
                self.expect_line(a)?;
                self.expect_line(b)?;
                finite(t)?;
            }
            PointOnLine(p, l) | Midpoint(p, l) => {
                self.expect_point(p)?;
                self.expect_line(l)?;
            }
            PointOnCircle(p, e) => {
                self.expect_point(p)?;
                self.expect_round(e)?;
            }
            Symmetric(p, q, l) => {
                self.expect_point(p)?;
                self.expect_point(q)?;
                self.expect_line(l)?;
            }
            Tangent(a, b) => {
                let (ra, rb) = (self.is_round(a)?, self.is_round(b)?);
                if !ra && !rb {
                    return Err(SketchError::WrongKind(b));
                }
                if !ra {
                    self.expect_line(a)?;
                }
                if !rb {
                    self.expect_line(b)?;
                }
            }
            HorizontalDistance(p, q, d) | VerticalDistance(p, q, d) => {
                self.expect_point(p)?;
                self.expect_point(q)?;
                finite(d)?;
            }
        }
        let data = self.capture(c);
        let id = ConstraintId(self.next_constraint);
        self.next_constraint += 1;
        self.constraints.push(ConstraintRec { id, c, data });
        Ok(id)
    }

    pub fn remove_constraint(&mut self, id: ConstraintId) -> Result<(), SketchError> {
        let i = self
            .constraints
            .iter()
            .position(|r| r.id == id)
            .ok_or(SketchError::UnknownConstraint(id))?;
        self.constraints.remove(i);
        Ok(())
    }

    /// What a constraint needs to remember from the geometry at the moment it is added.
    fn capture(&self, c: Constraint) -> [f64; 2] {
        let x = &self.params;
        match c {
            Constraint::Fixed(p) => {
                let q = self.point(p);
                [q.x, q.y]
            }
            Constraint::DistancePointLine(p, l, _) => {
                let mut loc = Local::new(x);
                let pt = self.pt(&mut loc, p);
                [sign(self.signed_dist(&mut loc, pt, l).v), 0.0]
            }
            Constraint::Tangent(a, b) => {
                let (ra, rb) = (
                    self.is_round(a).unwrap_or(false),
                    self.is_round(b).unwrap_or(false),
                );
                let mut loc = Local::new(x);
                if ra && rb {
                    let (c1, c2) = (self.center(&mut loc, a), self.center(&mut loc, b));
                    let (r1, r2) = (self.rad(&mut loc, a).v, self.rad(&mut loc, b).v);
                    let d = c1.sub(c2).len().v;
                    // 0 = outside each other; otherwise the sign of r1 - r2 for inner contact
                    if (d - (r1 + r2)).abs() <= (d - (r1 - r2).abs()).abs() {
                        [0.0, 0.0]
                    } else {
                        [sign(r1 - r2), 0.0]
                    }
                } else {
                    let (l, round) = if ra { (b, a) } else { (a, b) };
                    let c = self.center(&mut loc, round);
                    [sign(self.signed_dist(&mut loc, c, l).v), 0.0]
                }
            }
            _ => [0.0, 0.0],
        }
    }

    pub(crate) fn pt(&self, loc: &mut Local, id: EntityId) -> V {
        let (x, y) = self.point_params(id).expect("validated point");
        V {
            x: loc.var(x),
            y: loc.var(y),
        }
    }

    fn line(&self, loc: &mut Local, id: EntityId) -> (V, V) {
        match self.rec(id).map(|r| r.kind) {
            Ok(Entity::Line { a, b }) => (self.pt(loc, a), self.pt(loc, b)),
            _ => panic!("validated line"),
        }
    }

    fn center(&self, loc: &mut Local, id: EntityId) -> V {
        match self.rec(id).map(|r| r.kind) {
            Ok(Entity::Circle { center, .. }) | Ok(Entity::Arc { center, .. }) => self.pt(loc, center),
            _ => panic!("validated round"),
        }
    }

    fn rad(&self, loc: &mut Local, id: EntityId) -> D {
        match self.rec(id).map(|r| r.kind) {
            Ok(Entity::Circle { r, .. }) => loc.var(r),
            Ok(Entity::Arc { center, start, .. }) => self.pt(loc, start).sub(self.pt(loc, center)).len(),
            _ => panic!("validated round"),
        }
    }

    /// Signed distance from a point to a line (positive on the line's left).
    fn signed_dist(&self, loc: &mut Local, p: V, l: EntityId) -> D {
        let (a, b) = self.line(loc, l);
        let dir = b.sub(a);
        dir.cross(p.sub(a)) / dir.len()
    }

    /// Residual rows of one constraint (each in millimetres) and the parameters they depend on.
    pub(crate) fn residuals<'x>(&self, rec: &ConstraintRec, x: &'x [f64]) -> (Local<'x>, Rows) {
        use Constraint::*;
        let mut loc = Local::new(x);
        let l = &mut loc;
        let rows = match rec.c {
            Fixed(p) => {
                let q = self.pt(l, p);
                Rows::two(q.x - D::c(rec.data[0]), q.y - D::c(rec.data[1]))
            }
            Coincident(p, q) => {
                let d = self.pt(l, q).sub(self.pt(l, p));
                Rows::two(d.x, d.y)
            }
            Concentric(a, b) => {
                let d = self.center(l, b).sub(self.center(l, a));
                Rows::two(d.x, d.y)
            }
            Horizontal(e) => {
                let (a, b) = self.line(l, e);
                Rows::one(b.y - a.y)
            }
            Vertical(e) => {
                let (a, b) = self.line(l, e);
                Rows::one(b.x - a.x)
            }
            Distance(p, q, d) => Rows::one(self.pt(l, q).sub(self.pt(l, p)).len() - D::c(d)),
            DistancePointLine(p, e, d) => {
                let q = self.pt(l, p);
                Rows::one(self.signed_dist(l, q, e) - D::c(rec.data[0] * d))
            }
            Length(e, d) => {
                let (a, b) = self.line(l, e);
                Rows::one(b.sub(a).len() - D::c(d))
            }
            Radius(e, d) => Rows::one(self.rad(l, e) - D::c(d)),
            Diameter(e, d) => Rows::one(self.rad(l, e) * 2.0 - D::c(d)),
            Equal(a, b) => {
                if self.is_round(a).unwrap_or(false) {
                    Rows::one(self.rad(l, a) - self.rad(l, b))
                } else {
                    let (a0, a1) = self.line(l, a);
                    let (b0, b1) = self.line(l, b);
                    Rows::one(a1.sub(a0).len() - b1.sub(b0).len())
                }
            }
            Parallel(a, b) | Perpendicular(a, b) | Angle(a, b, _) => {
                let (a0, a1) = self.line(l, a);
                let (b0, b1) = self.line(l, b);
                let (u, v) = (a1.sub(a0), b1.sub(b0));
                let theta = match rec.c {
                    Parallel(..) => 0.0,
                    Perpendicular(..) => forge_math::dmath::FRAC_PI_2,
                    Angle(_, _, t) => t,
                    _ => unreachable!(),
                };
                let (s, c) = forge_math::dmath::sin_cos(theta);
                // |u| sin(phi - theta), phi the angle from u to v
                Rows::one((u.cross(v) * c - u.dot(v) * s) / v.len())
            }
            PointOnLine(p, e) => {
                let q = self.pt(l, p);
                Rows::one(self.signed_dist(l, q, e))
            }
            PointOnCircle(p, e) => {
                let q = self.pt(l, p);
                let c = self.center(l, e);
                Rows::one(q.sub(c).len() - self.rad(l, e))
            }
            Midpoint(p, e) => {
                let q = self.pt(l, p);
                let (a, b) = self.line(l, e);
                let m = a.add(b).scale(0.5);
                Rows::two(q.x - m.x, q.y - m.y)
            }
            Symmetric(p, q, e) => {
                let (pp, qq) = (self.pt(l, p), self.pt(l, q));
                let (a, b) = self.line(l, e);
                let dir = b.sub(a);
                let m = pp.add(qq).scale(0.5);
                let len = dir.len();
                Rows::two(dir.cross(m.sub(a)) / len, dir.dot(qq.sub(pp)) / len)
            }
            Tangent(a, b) => {
                let (ra, rb) = (
                    self.is_round(a).unwrap_or(false),
                    self.is_round(b).unwrap_or(false),
                );
                if ra && rb {
                    let d = self.center(l, a).sub(self.center(l, b)).len();
                    let (r1, r2) = (self.rad(l, a), self.rad(l, b));
                    if rec.data[0] == 0.0 {
                        Rows::one(d - (r1 + r2))
                    } else {
                        Rows::one(d - (r1 - r2) * rec.data[0])
                    }
                } else {
                    let (e, round) = if ra { (b, a) } else { (a, b) };
                    let c = self.center(l, round);
                    if let Some(p) = self.joint(e, round) {
                        // at a shared end the radius there is square to the line: the distance
                        // form would touch zero quadratically and converge slowly
                        let (a, b) = self.line(l, e);
                        let dir = b.sub(a);
                        Rows::one(dir.dot(self.pt(l, p).sub(c)) / dir.len())
                    } else {
                        let r = self.rad(l, round);
                        Rows::one(self.signed_dist(l, c, e) - r * rec.data[0])
                    }
                }
            }
            HorizontalDistance(p, q, d) => Rows::one(self.pt(l, q).x - self.pt(l, p).x - D::c(d)),
            VerticalDistance(p, q, d) => Rows::one(self.pt(l, q).y - self.pt(l, p).y - D::c(d)),
        };
        (loc, rows)
    }

    /// The end a line shares with an arc, if it shares one.
    fn joint(&self, line: EntityId, arc: EntityId) -> Option<EntityId> {
        match (self.rec(line).ok()?.kind, self.rec(arc).ok()?.kind) {
            (Entity::Line { a, b }, Entity::Arc { start, end, .. }) => {
                [a, b].into_iter().find(|p| *p == start || *p == end)
            }
            _ => None,
        }
    }

    /// The implicit row of an arc: its end stays on its radius.
    pub(crate) fn arc_residual<'x>(&self, id: EntityId, x: &'x [f64]) -> Option<(Local<'x>, D)> {
        let Ok(Entity::Arc {
            center, start, end, ..
        }) = self.rec(id).map(|r| r.kind)
        else {
            return None;
        };
        let mut loc = Local::new(x);
        let c = self.pt(&mut loc, center);
        let r = self.pt(&mut loc, end).sub(c).len() - self.pt(&mut loc, start).sub(c).len();
        Some((loc, r))
    }
}
