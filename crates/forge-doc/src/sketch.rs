//! Sketch blocks: rows of geometry, constraints and dimensions built into a
//! `forge_sketch::Sketch` and solved, and the profiles, points and lines later lines take
//! from it. Grammar in AGENTS.md.

use crate::text::Want;
use forge_geom::Circle;
use forge_math::{Frame, Point2, dmath, tol};
use forge_model::Sketch;
use forge_ops::{PathSeg, Profile};
use forge_sketch::{Constraint as C, ConstraintId, Entity, EntityId, ExtractError, SketchError, Status};

type R<T> = Result<T, String>;
/// Reads a number token (a value or an expression) as the slot wants it.
pub type Num<'a> = dyn Fn(&str, Want) -> R<f64> + 'a;

/// Builds a sketch's rows on `plane` and solves it from the coordinates as written.
pub fn build(plane: Frame, rows: Vec<Vec<String>>, num: &Num) -> R<Sketch> {
    let mut b = B {
        s: forge_sketch::Sketch::new(plane),
        names: Vec::new(),
        made: Vec::new(),
        driven: Vec::new(),
        num,
    };
    for (k, row) in rows.iter().enumerate() {
        b.row(k, row).map_err(|e| format!("`{}`: {e}", row.join(" ")))?;
    }
    if b.s.solve().status != Status::Solved {
        let a = b.s.analyze();
        let say = |ids: &[ConstraintId]| {
            let mut ks: Vec<usize> = ids
                .iter()
                .filter_map(|c| b.made.iter().find(|m| m.0 == *c).map(|m| m.1))
                .collect();
            ks.sort_unstable();
            ks.dedup();
            ks.iter()
                .map(|&k| format!("`{}`", rows[k].join(" ")))
                .collect::<Vec<_>>()
                .join(", ")
        };
        let mut msg = format!("the sketch does not solve: {} conflict", say(&a.conflicting));
        let more: Vec<ConstraintId> = a
            .redundant
            .iter()
            .filter(|c| !a.conflicting.contains(c))
            .copied()
            .collect();
        if !more.is_empty() {
            msg += &format!("; with {}", say(&more));
        }
        return Err(msg);
    }
    let implied: Vec<usize> =
        b.s.implied()
            .iter()
            .filter_map(|c| b.made.iter().find(|m| m.0 == *c).map(|m| m.1))
            .collect();
    if let Some(&k) = implied.first() {
        let row = rows[k].join(" ");
        let fix = if DIMENSIONS.contains(&rows[k][0].as_str()) {
            let mut r = rows[k].clone();
            r.pop();
            format!(
                "remove it, or make it driven (it then shows its value): `ref {}`",
                r.join(" ")
            )
        } else {
            "remove it".to_string()
        };
        return Err(format!(
            "`{row}` is implied by the rows above it (over-constrained): {fix}"
        ));
    }
    let driven = b
        .driven
        .iter()
        .map(|(k, kw, es)| {
            Ok((
                *k,
                b.measure(kw, es)
                    .map_err(|e| format!("`{}`: {e}", rows[*k].join(" ")))?,
            ))
        })
        .collect::<R<_>>()?;
    Ok(Sketch {
        plane,
        on: None,
        at: None,
        rows,
        solved: b.s,
        driven,
        names: b.names,
    })
}

/// The rows that dimension, and which take `ref`.
pub const DIMENSIONS: [&str; 6] = ["dist", "hdist", "vdist", "rad", "dia", "ang"];

struct B<'a, 'n> {
    s: forge_sketch::Sketch,
    names: Vec<(String, EntityId)>,
    /// Each constraint and the row that made it.
    made: Vec<(ConstraintId, usize)>,
    /// Each driven dimension: its row, kind and entities.
    driven: Vec<(usize, String, Vec<EntityId>)>,
    num: &'a Num<'n>,
}

/// Whether a token names a sketch entity or one of its points (`o p1 l2 a3.c c4`) rather
/// than being a number.
pub fn is_ref(t: &str) -> bool {
    let base = t
        .split_once('.')
        .map_or(t, |(b, p)| if ["a", "b", "c"].contains(&p) { b } else { "" });
    base == "o"
        || base.len() > 1
            && base.starts_with(['p', 'l', 'a', 'c'])
            && base[1..].bytes().all(|c| c.is_ascii_digit())
}

fn bad(e: SketchError) -> String {
    match e {
        SketchError::WrongKind(_) => "an entity of the wrong kind for this row".into(),
        SketchError::BadValue => "a value out of range (lengths and radii > 0)".into(),
        e => format!("{e:?}"),
    }
}

impl B<'_, '_> {
    fn add(&mut self, k: usize, c: C) -> R<()> {
        let id = self.s.add_constraint(c).map_err(bad)?;
        self.made.push((id, k));
        Ok(())
    }

    fn name(&mut self, kind: char, id: EntityId) {
        let n = self
            .names
            .iter()
            .filter(|(s, _)| s.starts_with(kind) && s != "o")
            .count()
            + 1;
        self.names.push((format!("{kind}{n}"), id));
    }

    /// The entity a name means (`o` made on first use: the fixed origin).
    fn entity(&mut self, t: &str) -> R<EntityId> {
        if let Some((_, id)) = self.names.iter().find(|(n, _)| n == t) {
            return Ok(*id);
        }
        if t == "o" {
            let id = self.s.add_point(Point2::ORIGIN);
            self.s.add_constraint(C::Fixed(id)).map_err(bad)?;
            self.names.push(("o".into(), id));
            return Ok(id);
        }
        Err(format!("no entity '{t}' above this row"))
    }

    /// A point: `name.a|b|c`, a point's name, or a circle's or arc's (its centre).
    fn point_ref(&mut self, t: &str) -> R<EntityId> {
        let (base, part) = t.split_once('.').unwrap_or((t, ""));
        let e = self.entity(base)?;
        let kind = self.s.entity(e).expect("named");
        Ok(match (kind, part) {
            (Entity::Point { .. }, "") => e,
            (Entity::Line { a, .. }, "a") | (Entity::Arc { start: a, .. }, "a") => a,
            (Entity::Line { b, .. }, "b") | (Entity::Arc { end: b, .. }, "b") => b,
            (Entity::Circle { center, .. } | Entity::Arc { center, .. }, "" | "c") => center,
            _ => return Err(format!("'{t}' is not a point")),
        })
    }

    fn row(&mut self, k: usize, row: &[String]) -> R<()> {
        let mut c = Cur { t: row, k: 1 };
        let cons = row.last().is_some_and(|t| t == "cons");
        if cons {
            c.t = &row[..row.len() - 1];
        }
        let made = self.names.len();
        if row[0] == "ref" {
            let kw = row.get(1).map_or("", |s| s.as_str());
            if !DIMENSIONS.contains(&kw) {
                return Err(format!("ref goes before a dimension ({})", DIMENSIONS.join(" ")));
            }
            c.k = 2;
            let mut es = Vec::new();
            while !c.done() {
                es.push(self.any(&mut c)?);
            }
            let want = if matches!(kw, "dist" | "ang") {
                1..=2
            } else if kw == "rad" || kw == "dia" {
                1..=1
            } else {
                2..=2
            };
            if !want.contains(&es.len()) {
                return Err(format!(
                    "ref {kw} names {} entities, no value",
                    if want.start() == want.end() {
                        want.start().to_string()
                    } else {
                        "1 or 2".into()
                    }
                ));
            }
            self.driven.push((k, kw.to_string(), es));
            return Ok(());
        }
        match row[0].as_str() {
            "pt" => {
                let p = self.point(&mut c)?;
                self.name('p', p);
            }
            "line" => {
                let (a, b) = (self.point(&mut c)?, self.point(&mut c)?);
                let l = self.s.add_line(a, b).map_err(bad)?;
                self.name('l', l);
            }
            "poly" => {
                let open = c.t.last().is_some_and(|t| t == "open");
                if open {
                    c.t = &c.t[..c.t.len() - 1];
                }
                let mut pts = Vec::new();
                while !c.done() {
                    pts.push(self.point(&mut c)?);
                }
                if pts.len() < 2 {
                    return Err("poly needs two points or more".into());
                }
                let n = pts.len() - open as usize;
                for i in 0..n {
                    let l = self.s.add_line(pts[i], pts[(i + 1) % pts.len()]).map_err(bad)?;
                    self.name('l', l);
                }
            }
            "rect" => {
                let (x, y) = (self.len(&mut c)?, self.len(&mut c)?);
                let (w, h) = (self.len(&mut c)?, self.len(&mut c)?);
                let p = [(x, y), (x + w, y), (x + w, y + h), (x, y + h)]
                    .map(|(x, y)| self.s.add_point(Point2::new(x, y)));
                let mut ls = [EntityId(0); 4];
                for i in 0..4 {
                    ls[i] = self.s.add_line(p[i], p[(i + 1) % 4]).map_err(bad)?;
                    self.name('l', ls[i]);
                }
                self.add(k, C::Horizontal(ls[0]))?;
                self.add(k, C::Horizontal(ls[2]))?;
                self.add(k, C::Vertical(ls[1]))?;
                self.add(k, C::Vertical(ls[3]))?;
            }
            "circle" => {
                let center = self.point(&mut c)?;
                let r = self.len(&mut c)?;
                let e = self.s.add_circle(center, r).map_err(bad)?;
                self.name('c', e);
            }
            "arc" => {
                let (center, a, b) = (self.point(&mut c)?, self.point(&mut c)?, self.point(&mut c)?);
                let ccw = !c.eat("cw");
                let e = self.s.add_arc(center, a, b, ccw).map_err(bad)?;
                self.name('a', e);
            }
            _ => {
                if cons {
                    return Err("only geometry rows take `cons`".into());
                }
                return self.constraint(k, &mut c);
            }
        }
        if cons {
            for i in made..self.names.len() {
                self.s.set_construction(self.names[i].1, true).map_err(bad)?;
            }
        }
        c.end()
    }

    fn constraint(&mut self, k: usize, c: &mut Cur) -> R<()> {
        let kw = c.t[0].as_str();
        let line = |s: &Self, e: EntityId| matches!(s.s.entity(e), Some(Entity::Line { .. }));
        let point = |s: &Self, e: EntityId| matches!(s.s.entity(e), Some(Entity::Point { .. }));
        match kw {
            "fix" => {
                while !c.done() {
                    let t = c.next()?;
                    let e = if t.contains('.') {
                        self.point_ref(t)?
                    } else {
                        self.entity(t)?
                    };
                    let pts = match self.s.entity(e).expect("named") {
                        Entity::Point { .. } => vec![e],
                        Entity::Line { a, b } => vec![a, b],
                        Entity::Circle { center, .. } => {
                            let r = self.s.radius(e);
                            self.add(k, C::Radius(e, r))?;
                            vec![center]
                        }
                        Entity::Arc {
                            center, start, end, ..
                        } => vec![center, start, end],
                    };
                    for p in pts {
                        self.add(k, C::Fixed(p))?;
                    }
                }
            }
            "co" => {
                let (a, b) = (self.any(c)?, self.any(c)?);
                let (p, q) = if point(self, a) { (a, b) } else { (b, a) };
                if !point(self, p) {
                    return Err("co needs a point".into());
                }
                let con = match self.s.entity(q).expect("named") {
                    Entity::Point { .. } => C::Coincident(p, q),
                    Entity::Line { .. } => C::PointOnLine(p, q),
                    _ => C::PointOnCircle(p, q),
                };
                self.add(k, con)?;
            }
            "hor" | "ver" => {
                let mut es = Vec::new();
                while !c.done() {
                    es.push(self.any(c)?);
                }
                if es.len() == 2 && !es.iter().any(|&e| line(self, e)) {
                    let (p, q) = (self.as_point(es[0])?, self.as_point(es[1])?);
                    let con = if kw == "hor" {
                        C::VerticalDistance(p, q, 0.0)
                    } else {
                        C::HorizontalDistance(p, q, 0.0)
                    };
                    return self.add(k, con);
                }
                if es.is_empty() {
                    return Err(format!("{kw} needs lines, or two points"));
                }
                for e in es {
                    self.add(
                        k,
                        if kw == "hor" {
                            C::Horizontal(e)
                        } else {
                            C::Vertical(e)
                        },
                    )?;
                }
            }
            "par" | "perp" | "tan" | "conc" => {
                let (a, b) = (self.entity_tok(c)?, self.entity_tok(c)?);
                let con = match kw {
                    "par" => C::Parallel(a, b),
                    "perp" => C::Perpendicular(a, b),
                    "tan" => C::Tangent(a, b),
                    _ => C::Concentric(a, b),
                };
                self.add(k, con)?;
            }
            "eq" => {
                let a = self.entity_tok(c)?;
                let mut n = 0;
                while !c.done() {
                    let b = self.entity_tok(c)?;
                    self.add(k, C::Equal(a, b))?;
                    n += 1;
                }
                if n == 0 {
                    return Err("eq needs two entities or more".into());
                }
            }
            "mid" => {
                let p = self.point_tok(c)?;
                let l = self.entity_tok(c)?;
                self.add(k, C::Midpoint(p, l))?;
            }
            "sym" => {
                let (p, q) = (self.point_tok(c)?, self.point_tok(c)?);
                let l = self.entity_tok(c)?;
                self.add(k, C::Symmetric(p, q, l))?;
            }
            "dist" => {
                let a = self.any(c)?;
                if is_ref(c.peek()) {
                    let b = self.any(c)?;
                    let d = self.len(c)?;
                    let con = match (line(self, a), line(self, b)) {
                        (false, false) => C::Distance(self.as_point(a)?, self.as_point(b)?, d),
                        (true, false) => C::DistancePointLine(self.as_point(b)?, a, d),
                        (false, true) => C::DistancePointLine(self.as_point(a)?, b, d),
                        _ => return Err("dist between two lines: use par and a point to line dist".into()),
                    };
                    self.add(k, con)?;
                } else {
                    if !line(self, a) {
                        return Err("dist with one entity needs a line".into());
                    }
                    let d = self.len(c)?;
                    self.add(k, C::Length(a, d))?;
                }
            }
            "hdist" | "vdist" => {
                let (p, q) = (self.point_tok(c)?, self.point_tok(c)?);
                let d = self.len(c)?;
                let con = if kw == "hdist" {
                    C::HorizontalDistance(p, q, d)
                } else {
                    C::VerticalDistance(p, q, d)
                };
                self.add(k, con)?;
            }
            "rad" | "dia" => {
                let e = self.entity_tok(c)?;
                let d = self.len(c)?;
                self.add(
                    k,
                    if kw == "rad" {
                        C::Radius(e, d)
                    } else {
                        C::Diameter(e, d)
                    },
                )?;
            }
            "ang" => {
                let (a, b) = (self.entity_tok(c)?, self.entity_tok(c)?);
                let t = (self.num)(c.next()?, Want::Angle)?;
                self.add(k, C::Angle(a, b, t))?;
            }
            _ => {
                return Err(
                    "unknown row (pt line poly rect circle arc | fix co hor ver par perp tan eq conc mid sym | dist hdist vdist rad dia ang)"
                        .into(),
                );
            }
        }
        c.end()
    }

    /// What a driven dimension measures on the solved sketch.
    fn measure(&self, kw: &str, es: &[EntityId]) -> R<f64> {
        let q = &self.s;
        let pt = |e: EntityId| self.as_point(e).map(|p| q.point(p));
        let line = |e: EntityId| match q.entity(e) {
            Some(Entity::Line { a, b }) => Some((q.point(a), q.point(b))),
            _ => None,
        };
        let need_line = |e: EntityId| line(e).ok_or_else(|| "a line is needed".to_string());
        Ok(match (kw, es) {
            ("dist", [l]) => {
                let (a, b) = need_line(*l)?;
                a.distance(b)
            }
            ("dist", [x, y]) => match (line(*x), line(*y)) {
                (None, None) => pt(*x)?.distance(pt(*y)?),
                (Some((a, b)), None) | (None, Some((a, b))) => {
                    let p = if line(*x).is_some() { pt(*y)? } else { pt(*x)? };
                    ((b - a).cross(p - a) / a.distance(b)).abs()
                }
                _ => return Err("dist between two lines: use par and a point to line dist".into()),
            },
            ("hdist", [x, y]) => pt(*y)?.x - pt(*x)?.x,
            ("vdist", [x, y]) => pt(*y)?.y - pt(*x)?.y,
            ("rad" | "dia", [c]) => {
                if !matches!(q.entity(*c), Some(Entity::Circle { .. } | Entity::Arc { .. })) {
                    return Err("rad and dia need a circle or arc".into());
                }
                q.radius(*c) * if kw == "dia" { 2.0 } else { 1.0 }
            }
            ("ang", [x, y]) => {
                let ((a, b), (c, d)) = (need_line(*x)?, need_line(*y)?);
                let (u, v) = (b - a, d - c);
                dmath::atan2(u.cross(v), u.dot(v))
            }
            _ => return Err(format!("ref {kw}: wrong entities")),
        })
    }

    /// A point named, or a new one at X Y.
    fn point(&mut self, c: &mut Cur) -> R<EntityId> {
        if is_ref(c.peek()) {
            return self.point_tok(c);
        }
        let (x, y) = (self.len(c)?, self.len(c)?);
        Ok(self.s.add_point(Point2::new(x, y)))
    }

    fn point_tok(&mut self, c: &mut Cur) -> R<EntityId> {
        let t = c.next()?;
        self.point_ref(t)
    }

    fn entity_tok(&mut self, c: &mut Cur) -> R<EntityId> {
        let t = c.next()?;
        if t.contains('.') {
            return Err(format!("'{t}' is a point; this needs a line, circle or arc"));
        }
        self.entity(t)
    }

    /// An entity, or a point of one (`l1.a`).
    fn any(&mut self, c: &mut Cur) -> R<EntityId> {
        let t = c.next()?;
        if t.contains('.') {
            self.point_ref(t)
        } else {
            self.entity(t)
        }
    }

    /// A point as a point: a circle or arc stands for its centre.
    fn as_point(&self, e: EntityId) -> R<EntityId> {
        match self.s.entity(e).expect("named") {
            Entity::Point { .. } => Ok(e),
            Entity::Circle { center, .. } | Entity::Arc { center, .. } => Ok(center),
            Entity::Line { .. } => Err("a line where a point is wanted".into()),
        }
    }

    fn len(&mut self, c: &mut Cur) -> R<f64> {
        (self.num)(c.next()?, Want::Len)
    }
}

/// A cursor over a row's tokens.
struct Cur<'a> {
    t: &'a [String],
    k: usize,
}

impl Cur<'_> {
    fn peek(&self) -> &str {
        self.t.get(self.k).map_or("", |s| s.as_str())
    }
    fn done(&self) -> bool {
        self.k >= self.t.len()
    }
    fn next(&mut self) -> R<&str> {
        let t = self.t.get(self.k).ok_or("the row ends too soon")?;
        self.k += 1;
        Ok(t)
    }
    fn eat(&mut self, w: &str) -> bool {
        let hit = self.peek() == w;
        self.k += hit as usize;
        hit
    }
    fn end(&self) -> R<()> {
        match self.t.get(self.k) {
            Some(t) => Err(format!("unexpected '{t}'")),
            None => Ok(()),
        }
    }
}

fn id_of(s: &Sketch, t: &str) -> R<EntityId> {
    s.names
        .iter()
        .find(|(n, _)| n == t)
        .map(|n| n.1)
        .ok_or_else(|| format!("the sketch has no '{t}'"))
}

/// The segment id an entity's profile edges carry: from its kind and count (`l3`), so it
/// stays put when other kinds are added or dimensions change.
fn stable(name: &str) -> u64 {
    let n: u64 = name[1..].parse().unwrap_or(0);
    let k = match name.as_bytes()[0] {
        b'l' => 0,
        b'a' => 1,
        _ => 2,
    };
    n << 2 | k
}

fn name_of(s: &Sketch, id: u64) -> &str {
    s.names
        .iter()
        .find(|(_, e)| e.0 == id)
        .map_or("?", |n| n.0.as_str())
}

/// The sketch's one region, or the one whose outside loop runs through `through`.
pub fn profile(s: &Sketch, through: Option<&str>) -> R<Profile> {
    let ps = forge_sketch::profiles(&s.solved).map_err(|e| {
        let n = |id: EntityId| name_of(s, id.0).to_string();
        match e {
            ExtractError::Empty => "the sketch encloses nothing".into(),
            ExtractError::OpenChain { entity } => format!("the chain through {} is open", n(entity)),
            ExtractError::Branch { entity } => {
                format!("more than two curves meet at an end of {}", n(entity))
            }
            ExtractError::Degenerate { entity } => format!("{} has no length", n(entity)),
            ExtractError::SelfIntersection { a, b } => format!("{} and {} cross", n(a), n(b)),
            ExtractError::Profile(e) => format!("{e:?}"),
        }
    })?;
    let mut p = match through {
        None if ps.len() == 1 => ps.into_iter().next().expect("one"),
        None => {
            let outs: Vec<&str> = ps.iter().map(|p| name_of(s, p.loops[0].segments[0].1)).collect();
            return Err(format!(
                "the sketch has {} regions; name one by an entity on its outside: {}",
                ps.len(),
                outs.join(" ")
            ));
        }
        Some(t) => {
            let id = id_of(s, t)?.0;
            ps.into_iter()
                .find(|p| p.loops[0].segments.iter().any(|g| g.1 == id))
                .ok_or_else(|| format!("{t} is not on the outside of a region"))?
        }
    };
    for l in &mut p.loops {
        for g in &mut l.segments {
            g.1 = stable(name_of(s, g.1));
        }
    }
    Ok(p)
}

/// The sketch's one chain of lines and arcs, or the one through `through`, end to end in 3D:
/// a path to sweep along.
pub fn path(s: &Sketch, through: Option<&str>) -> R<Vec<PathSeg>> {
    let q = &s.solved;
    // each line or arc: its id and its ends
    let mut segs: Vec<(EntityId, Point2, Point2)> = Vec::new();
    for id in q.entity_ids().filter(|&id| !q.is_construction(id)) {
        match q.entity(id) {
            Some(Entity::Line { a, b }) => segs.push((id, q.point(a), q.point(b))),
            Some(Entity::Arc { start, end, .. }) => segs.push((id, q.point(start), q.point(end))),
            _ => {}
        }
    }
    let meets = |p: Point2, q: Point2| p.distance(q) <= tol::LINEAR;
    // the chain through the first segment, or through the one named
    let first = match through {
        Some(t) => {
            let id = id_of(s, t)?;
            segs.iter()
                .position(|g| g.0 == id)
                .ok_or_else(|| format!("{t} is not a line or arc of the sketch"))?
        }
        None if segs.is_empty() => return Err("the sketch has no lines or arcs".into()),
        None => 0,
    };
    let mut chain = vec![first];
    // grow from the first piece's end, then from its start
    for (at_end, mut free) in [(true, segs[first].2), (false, segs[first].1)] {
        loop {
            let next: Vec<usize> = (0..segs.len())
                .filter(|j| !chain.contains(j) && (meets(segs[*j].1, free) || meets(segs[*j].2, free)))
                .collect();
            let &[j] = &next[..] else {
                if next.len() > 1 {
                    return Err(format!(
                        "the path branches at an end of {}",
                        name_of(s, segs[next[0]].0.0)
                    ));
                }
                break;
            };
            free = if meets(segs[j].1, free) {
                segs[j].2
            } else {
                segs[j].1
            };
            if at_end {
                chain.push(j);
            } else {
                chain.insert(0, j);
            }
        }
    }
    if through.is_none() && chain.len() < segs.len() {
        let other = (0..segs.len())
            .find(|j| !chain.contains(j))
            .expect("one left out");
        return Err(format!(
            "the sketch has more than one chain; name one by an entity on it: {} or {}",
            name_of(s, segs[first].0.0),
            name_of(s, segs[other].0.0)
        ));
    }
    // walk the chain from the end the first piece does not share
    let mut at = match chain.get(1) {
        Some(&n) if meets(segs[chain[0]].2, segs[n].1) || meets(segs[chain[0]].2, segs[n].2) => {
            segs[chain[0]].1
        }
        Some(_) => segs[chain[0]].2,
        None => segs[chain[0]].1,
    };
    let plane = s.plane;
    let mut out = Vec::new();
    for &k in &chain {
        let (id, a, b) = segs[k];
        let forward = meets(a, at);
        at = if forward { b } else { a };
        out.push(match q.entity(id).expect("a segment") {
            Entity::Arc { center, ccw, .. } => {
                let c = q.point(center);
                let (a0, a1) = (
                    dmath::atan2(a.y - c.y, a.x - c.x),
                    dmath::atan2(b.y - c.y, b.x - c.x),
                );
                let sweep = if ccw {
                    dmath::wrap_tau(a1 - a0)
                } else {
                    -dmath::wrap_tau(a0 - a1)
                };
                let circle = Circle {
                    frame: plane.with_origin(plane.plane_point(c)),
                    radius: a.distance(c),
                };
                if forward {
                    PathSeg::Arc {
                        circle,
                        start: a0,
                        sweep,
                    }
                } else {
                    PathSeg::Arc {
                        circle,
                        start: a0 + sweep,
                        sweep: -sweep,
                    }
                }
            }
            _ => {
                let (p, q) = if forward { (a, b) } else { (b, a) };
                PathSeg::Line {
                    a: plane.plane_point(p),
                    b: plane.plane_point(q),
                }
            }
        });
    }
    Ok(out)
}

/// A point of the sketch (`p1`, `l2.b`, `c1` for its centre), in its plane.
pub fn point(s: &Sketch, t: &str) -> R<Point2> {
    let (base, part) = t.split_once('.').unwrap_or((t, ""));
    let e = id_of(s, base)?;
    let q = &s.solved;
    let at = |p: EntityId| q.point(p);
    Ok(match (q.entity(e).expect("named"), part) {
        (Entity::Point { .. }, "") => at(e),
        (Entity::Line { a, .. } | Entity::Arc { start: a, .. }, "a") => at(a),
        (Entity::Line { b, .. } | Entity::Arc { end: b, .. }, "b") => at(b),
        (Entity::Circle { center, .. } | Entity::Arc { center, .. }, "" | "c") => at(center),
        _ => return Err(format!("'{t}' is not a point")),
    })
}

/// A line of the sketch: its ends, in its plane.
pub fn line(s: &Sketch, t: &str) -> R<(Point2, Point2)> {
    match s.solved.entity(id_of(s, t)?) {
        Some(Entity::Line { a, b }) => Ok((s.solved.point(a), s.solved.point(b))),
        _ => Err(format!("'{t}' is not a line")),
    }
}

/// Degrees of freedom left, and the named entities not pinned down.
pub fn freedom(s: &Sketch) -> (usize, Vec<String>) {
    let a = s.solved.analyze();
    let loose = s
        .names
        .iter()
        .filter(|(n, id)| n != "o" && !a.fully_constrained.contains(id))
        .map(|n| n.0.clone())
        .collect();
    (a.dof, loose)
}
