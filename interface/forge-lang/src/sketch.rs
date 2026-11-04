//! Sketches: Forge's entities and constraints laid on forge-sketch, solved by its solver. The
//! helpers (hull, slot, rect, poly, path) add the entities and constraints their meaning needs
//! (tangency held by the solver, not written as numbers), with exact starting positions.

use crate::error::{EntKind, Error};
use crate::syntax::Span;
use forge_math::{Frame, Point2, Point3, Vec2};
use forge_ops::Profile;
use forge_sketch::{Constraint, ConstraintId, Entity, EntityId, Sketch};

pub struct Sk {
    pub id: String,
    pub span: Span,
    pub frame: Frame,
    pub s: Sketch,
    /// Each entity's name path (for its faces' stable names) and the span that made it.
    pub ents: Vec<(EntityId, String, Span)>,
    pub cons: Vec<(ConstraintId, Span)>,
    /// `pt` and `near` points, in order: what `.points` gives.
    pub points: Vec<EntityId>,
    pub solved: bool,
    pub dof: usize,
    anon: usize,
    /// The block's names and what they were bound to.
    pub named: Vec<(String, crate::eval::Value)>,
    /// Its place in the history.
    pub idx: usize,
    /// Params its rows read, and whether the solver moves anything (a free point): then they
    /// are all observed, structural params.
    pub params: std::collections::BTreeSet<crate::error::ParamKey>,
    pub free: bool,
    /// The numbers that placed and sized entities, by role ("x", "y" of a point or a circle's
    /// written centre, "r"), with where they came from: what fixes that move or resize set.
    pub nums: Vec<(EntityId, &'static str, crate::values::Num)>,
}

type R<T> = Result<T, Error>;

/// One item of a hull: a point (r = 0) or a disc, in counter-clockwise order.
#[derive(Clone, Copy, Debug)]
pub struct Item {
    pub center: Point2,
    pub r: f64,
    pub ent: Option<EntityId>,
}

/// Where a 2D point comes from: coordinates (a new fixed point) or an existing point.
#[derive(Clone, Copy, Debug)]
pub enum P2 {
    At(Point2),
    Ent(EntityId),
}

impl Sk {
    pub fn new(id: String, span: Span, frame: Frame) -> Sk {
        Sk {
            id,
            span,
            frame,
            s: Sketch::new(frame),
            ents: vec![],
            cons: vec![],
            points: vec![],
            solved: false,
            dof: 0,
            anon: 0,
            named: vec![],
            idx: 0,
            params: Default::default(),
            free: false,
            nums: vec![],
        }
    }

    /// Records an entity made directly on the solver sketch, under its name (or anonymous).
    pub fn name(&mut self, e: EntityId, name: Option<&str>, span: Span) {
        self.named(e, name, span)
    }

    fn named(&mut self, e: EntityId, name: Option<&str>, span: Span) {
        let n = match name {
            Some(n) => n.to_string(),
            None => {
                self.anon += 1;
                format!("~{}", self.anon)
            }
        };
        self.ents.push((e, n, span));
    }

    fn con(&mut self, c: Constraint, span: Span) -> R<ConstraintId> {
        let id = self.s.add_constraint(c).map_err(|error| Error::SketchKernel {
            span,
            sketch: self.id.clone(),
            error,
        })?;
        self.cons.push((id, span));
        Ok(id)
    }

    fn kerr(&self, span: Span) -> impl Fn(forge_sketch::SketchError) -> Error + '_ {
        move |error| Error::SketchKernel {
            span,
            sketch: self.id.clone(),
            error,
        }
    }

    pub fn kind(&self, e: EntityId) -> EntKind {
        match self.s.entity(e) {
            Some(Entity::Point { .. }) => EntKind::Point,
            Some(Entity::Line { .. }) => EntKind::Line,
            Some(Entity::Circle { .. }) => EntKind::Circle,
            Some(Entity::Arc { .. }) => EntKind::Arc,
            _ => EntKind::Spline,
        }
    }

    pub fn pos(&self, e: EntityId) -> Point2 {
        self.s.point(e)
    }

    pub fn world(&self, p: Point2) -> Point3 {
        self.frame.plane_point(p)
    }

    /// A point: fixed where it is given, or free (a starting guess).
    pub fn point(&mut self, p: Point2, fixed: bool, name: Option<&str>, span: Span) -> R<EntityId> {
        let e = self.s.add_point(p);
        self.free |= !fixed;
        if fixed {
            self.con(Constraint::Fixed(e), span)?;
        }
        self.named(e, name, span);
        Ok(e)
    }

    pub fn p2(&mut self, p: P2, span: Span) -> R<EntityId> {
        match p {
            P2::Ent(e) => Ok(e),
            P2::At(xy) => self.point(xy, true, None, span),
        }
    }

    pub fn line(
        &mut self,
        a: EntityId,
        b: EntityId,
        cons: bool,
        name: Option<&str>,
        span: Span,
    ) -> R<EntityId> {
        let l = self.s.add_line(a, b).map_err(self.kerr(span))?;
        if cons {
            self.s.set_construction(l, true).map_err(self.kerr(span))?;
        }
        self.named(l, name, span);
        Ok(l)
    }

    pub fn circle(&mut self, c: EntityId, r: f64, name: Option<&str>, span: Span) -> R<EntityId> {
        let e = self.s.add_circle(c, r).map_err(self.kerr(span))?;
        self.con(Constraint::Radius(e, r), span)?;
        self.named(e, name, span);
        Ok(e)
    }

    /// A whole ellipse: its centre, one end of its major axis, its semi-minor axis.
    pub fn ellipse(
        &mut self,
        c: EntityId,
        m: EntityId,
        b: f64,
        name: Option<&str>,
        span: Span,
    ) -> R<EntityId> {
        let e = self.s.add_ellipse(c, m, b).map_err(self.kerr(span))?;
        self.con(Constraint::EllipseMinor(e, b), span)?;
        self.named(e, name, span);
        Ok(e)
    }

    /// A closed curve with no corners (a whole ellipse, a closed spline): a profile by itself.
    pub fn closed_curve(&self, e: EntityId) -> bool {
        matches!(self.s.entity(e), Some(Entity::Ellipse { ends: None, .. }))
    }

    pub fn arc(
        &mut self,
        c: EntityId,
        a: EntityId,
        b: EntityId,
        ccw: bool,
        name: Option<&str>,
        span: Span,
    ) -> R<EntityId> {
        let e = self.s.add_arc(c, a, b, ccw).map_err(self.kerr(span))?;
        self.named(e, name, span);
        Ok(e)
    }

    pub fn spline(&mut self, pts: &[EntityId], closed: bool, name: Option<&str>, span: Span) -> R<EntityId> {
        let e = self.s.add_spline(pts, closed).map_err(self.kerr(span))?;
        self.named(e, name, span);
        Ok(e)
    }

    /// Segment names inside a helper: `outline.3`.
    fn sub(name: Option<&str>, k: usize) -> Option<String> {
        name.map(|n| format!("{n}.{k}"))
    }

    pub fn polygon(&mut self, pts: &[P2], closed: bool, name: Option<&str>, span: Span) -> R<Vec<EntityId>> {
        let ids: Vec<EntityId> = pts.iter().map(|&p| self.p2(p, span)).collect::<R<_>>()?;
        let n = ids.len();
        let m = if closed { n } else { n - 1 };
        let mut segs = Vec::with_capacity(m);
        for k in 0..m {
            let nm = Self::sub(name, k);
            segs.push(self.line(ids[k], ids[(k + 1) % n], false, nm.as_deref(), span)?);
        }
        Ok(segs)
    }

    pub fn rect(
        &mut self,
        corner: Point2,
        w: f64,
        h: f64,
        name: Option<&str>,
        span: Span,
    ) -> R<Vec<EntityId>> {
        let (x, y) = (corner.x, corner.y);
        let pts = [
            P2::At(Point2::new(x, y)),
            P2::At(Point2::new(x + w, y)),
            P2::At(Point2::new(x + w, y + h)),
            P2::At(Point2::new(x, y + h)),
        ];
        self.polygon(&pts, true, name, span)
    }

    /// The convex outline round points and discs in counter-clockwise order: a line tangent to
    /// each disc it touches, an arc round each disc. The solver holds the tangency.
    pub fn hull(&mut self, items: &[Item], name: Option<&str>, span: Span) -> R<Vec<EntityId>> {
        let n = items.len();
        if n < 2 {
            return Err(Error::BadValue {
                span,
                what: "the number of hull items",
                value: n as f64,
            });
        }
        // where each tangent leaves item i and arrives at item i+1
        let mut leave = vec![Point2::new(0.0, 0.0); n];
        let mut arrive = vec![Point2::new(0.0, 0.0); n];
        for i in 0..n {
            let (a, b) = (items[i], items[(i + 1) % n]);
            let d = b.center - a.center;
            let len = d.length();
            let k = (a.r - b.r) / len.max(1e-300);
            if len < 1e-9 || k.abs() >= 1.0 {
                return Err(Error::BadValue {
                    span,
                    what: "a hull item's distance to the next (one disc inside another)",
                    value: len,
                });
            }
            let u = Vec2::new(d.x / len, d.y / len);
            let right = Vec2::new(u.y, -u.x);
            let nrm = Vec2::new(
                u.x * k + right.x * (1.0 - k * k).sqrt(),
                u.y * k + right.y * (1.0 - k * k).sqrt(),
            );
            leave[i] = a.center + nrm * a.r;
            arrive[(i + 1) % n] = b.center + nrm * b.r;
        }
        let mut ins = vec![];
        let mut outs = vec![];
        let mut arcs: Vec<Option<EntityId>> = vec![];
        let mut segs = vec![];
        for (i, it) in items.iter().enumerate() {
            let c = match it.ent {
                Some(e) => e,
                None => self.point(it.center, true, None, span)?,
            };
            if it.r > 0.0 {
                let pi = self.point(arrive[i], false, None, span)?;
                let po = self.point(leave[i], false, None, span)?;
                let nm = Self::sub(name, 2 * i);
                let a = self.arc(c, pi, po, true, nm.as_deref(), span)?;
                self.con(Constraint::Radius(a, it.r), span)?;
                ins.push(pi);
                outs.push(po);
                arcs.push(Some(a));
                segs.push(a);
            } else {
                ins.push(c);
                outs.push(c);
                arcs.push(None);
            }
        }
        let mut lines = vec![];
        for i in 0..n {
            let j = (i + 1) % n;
            let nm = Self::sub(name, 2 * i + 1);
            let l = self.line(outs[i], ins[j], false, nm.as_deref(), span)?;
            if let Some(a) = arcs[i] {
                self.con(Constraint::Tangent(l, a), span)?;
            }
            if let Some(a) = arcs[j] {
                self.con(Constraint::Tangent(l, a), span)?;
            }
            lines.push(l);
        }
        // in order round the outline: arc i (if any), then the line leaving it
        let mut out = vec![];
        for i in 0..n {
            if let Some(a) = arcs[i] {
                out.push(a);
            }
            out.push(lines[i]);
        }
        let _ = segs;
        Ok(out)
    }

    /// The direction a chain leaves its last segment at its end.
    fn heading(&self, seg: EntityId) -> Option<Vec2> {
        match self.s.entity(seg)? {
            Entity::Line { a, b } => (self.pos(b) - self.pos(a)).normalize(),
            Entity::Arc { center, end, ccw, .. } => {
                let r = self.pos(end) - self.pos(center);
                let t = Vec2::new(-r.y, r.x);
                (if ccw { t } else { -t }).normalize()
            }
            Entity::Spline { .. } => {
                let pts = self.s.spline_points(seg)?;
                let k = pts.len();
                (self.pos(pts[k - 1]) - self.pos(pts[k - 2])).normalize()
            }
            _ => None,
        }
    }

    /// An arc leaving `from` (the end of `prev`) tangent to it, ending at `to`.
    pub fn tangent_arc(
        &mut self,
        prev: EntityId,
        from: EntityId,
        to: EntityId,
        name: Option<&str>,
        span: Span,
    ) -> R<EntityId> {
        let t = self.heading(prev).ok_or(Error::WrongEntity {
            span,
            want: EntKind::Line,
            found: self.kind(prev),
        })?;
        let (p, q) = (self.pos(from), self.pos(to));
        let n = Vec2::new(-t.y, t.x);
        let pq = p - q;
        let den = 2.0 * n.dot(pq);
        if den.abs() < 1e-12 {
            return Err(Error::BadValue {
                span,
                what: "a tangent arc to a point straight ahead",
                value: 0.0,
            });
        }
        let s = -pq.length_sq() / den;
        let c = self.point(p + n * s, false, None, span)?;
        let a = self.arc(c, from, to, s > 0.0, name, span)?;
        self.tangent(prev, a, span)?;
        Ok(a)
    }

    /// Tangency between two curves; a spline end takes it as its end slope.
    pub fn tangent(&mut self, a: EntityId, b: EntityId, span: Span) -> R<()> {
        match (self.kind(a), self.kind(b)) {
            (EntKind::Spline, _) => {
                if let Some(c) = self.s.tangent_spline(a, b).map_err(self.kerr(span))? {
                    self.cons.push((c, span));
                }
            }
            (_, EntKind::Spline) => {
                if let Some(c) = self.s.tangent_spline(b, a).map_err(self.kerr(span))? {
                    self.cons.push((c, span));
                }
            }
            _ => {
                self.con(Constraint::Tangent(a, b), span)?;
            }
        }
        Ok(())
    }

    pub fn constrain(&mut self, c: Constraint, span: Span) -> R<()> {
        self.con(c, span).map(|_| ())
    }

    /// Solves; conflicting rows are an error, freedom left is reported.
    pub fn finish(&mut self) -> R<()> {
        let rep = self.s.solve();
        let an = self.s.analyze();
        if !an.conflicting.is_empty() {
            let rows = an
                .conflicting
                .iter()
                .filter_map(|c| self.cons.iter().find(|x| x.0 == *c).map(|x| x.1))
                .collect();
            return Err(Error::SketchConflict {
                span: self.span,
                sketch: self.id.clone(),
                rows,
            });
        }
        if rep.status != forge_sketch::Status::Solved {
            return Err(Error::SketchNotSolved {
                span: self.span,
                sketch: self.id.clone(),
                residual: rep.max_residual,
            });
        }
        self.dof = an.dof;
        self.solved = true;
        Ok(())
    }

    /// The stable id an entity's faces are named from: a hash of the sketch's id and the
    /// entity's name path, so adding entities elsewhere renames nothing.
    pub fn seg_id(&self, e: EntityId) -> u64 {
        let name = self.ents.iter().find(|x| x.0 == e).map_or("?", |x| x.1.as_str());
        let mut h = 0xcbf2_9ce4_8422_2325u64;
        for b in self.id.bytes().chain(*b"/").chain(name.bytes()) {
            h = (h ^ b as u64).wrapping_mul(0x0100_0000_01b3);
        }
        // clear of the bits a profile uses for pieces of a kinked curve
        (h >> 24) & 0xff_ffff_ffff
    }

    /// The region whose outside runs through one of `through` (any one if None and there is one).
    pub fn profile(&self, through: Option<&[EntityId]>, span: Span) -> R<Profile> {
        let ps = forge_sketch::profiles(&self.s).map_err(|error| Error::SketchProfile {
            span,
            sketch: self.id.clone(),
            error,
        })?;
        const PIECE: u64 = 0xffff << 48;
        let mut p = match through {
            None if ps.len() == 1 => ps.into_iter().next().expect("one"),
            None => {
                return Err(Error::Regions {
                    span,
                    sketch: self.id.clone(),
                    count: ps.len(),
                });
            }
            Some(ids) => ps
                .into_iter()
                .find(|p| {
                    p.loops[0]
                        .segments
                        .iter()
                        .any(|g| ids.iter().any(|e| e.0 == g.1 & !PIECE))
                })
                .ok_or(Error::Regions {
                    span,
                    sketch: self.id.clone(),
                    count: 0,
                })?,
        };
        for l in &mut p.loops {
            for g in &mut l.segments {
                let (piece, e) = (g.1 & PIECE, EntityId(g.1 & !PIECE));
                g.1 = piece | self.seg_id(e);
            }
        }
        Ok(p)
    }

    /// Every region of the sketch, its segments named as `profile` names them.
    pub fn profiles(&self, span: Span) -> R<Vec<Profile>> {
        let ps = forge_sketch::profiles(&self.s).map_err(|error| Error::SketchProfile {
            span,
            sketch: self.id.clone(),
            error,
        })?;
        const PIECE: u64 = 0xffff << 48;
        Ok(ps
            .into_iter()
            .map(|mut p| {
                for l in &mut p.loops {
                    for g in &mut l.segments {
                        let (piece, e) = (g.1 & PIECE, EntityId(g.1 & !PIECE));
                        g.1 = piece | self.seg_id(e);
                    }
                }
                p
            })
            .collect())
    }

    /// What the solved sketch is as forge-model keeps it.
    pub fn model(&self) -> forge_model::Sketch {
        forge_model::Sketch {
            plane: self.frame,
            on: None,
            at: None,
            rows: vec![],
            solved: self.s.clone(),
            driven: vec![],
            names: self
                .ents
                .iter()
                .filter(|e| !e.1.starts_with('~'))
                .map(|e| (e.1.clone(), e.0))
                .collect(),
            written: vec![],
            dims: vec![],
            driven_of: vec![],
        }
    }
}

impl std::fmt::Debug for Sk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Sk({})", self.id)
    }
}
