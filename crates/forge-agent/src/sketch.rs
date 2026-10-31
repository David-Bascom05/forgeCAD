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
    Ok(Sketch {
        plane,
        on: None,
        at: None,
        rows,
        solved: b.s,
        names: b.names,
    })
}

struct B<'a, 'n> {
    s: forge_sketch::Sketch,
    names: Vec<(String, EntityId)>,
    /// Each constraint and the row that made it.
    made: Vec<(ConstraintId, usize)>,
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

#[cfg(test)]
mod tests {
    use crate::model::Model;
    use crate::select;

    fn built(src: &str) -> Model {
        let m = Model::new(src);
        if let Some(f) = &m.error {
            panic!("line {}: {} ({})\n{src}", m.lines[f.at].no, f.msg, f.hint);
        }
        m
    }

    /// The finished part, every value and name (caches left out).
    fn body(src: &str) -> String {
        format!("{:?}", built(src).body().expect("a part").compact())
    }

    /// The first error's message and hint.
    fn error(src: &str) -> (usize, String, String) {
        let m = Model::new(src);
        let f = m.error.expect("fails");
        (m.lines[f.at].no, f.msg, f.hint)
    }

    /// The mean centre of the faces a selector finds (a hole's two half walls, say), rounded.
    fn at(m: &Model, sel: &str) -> [f64; 3] {
        let (b, ents) = m.select(sel, None).expect("selects");
        let n = ents.len() as f64;
        assert!(n > 0.0, "{sel}");
        let c = ents
            .iter()
            .map(|&e| select::centre(b, e))
            .fold([0.0; 3], |s, c| [s[0] + c.x, s[1] + c.y, s[2] + c.z]);
        c.map(|v| (v / n * 1e6).round() / 1e6 + 0.0)
    }

    #[test]
    fn parameters_are_values_typed_in() {
        let with = "w = 40\nh = w/2 + 5 mm\nn = 2 + 1\na = 90/n deg\nr = min(w, h)/10\n\
                    extrude w/4 rect -w/2 -h/2 w h\n\
                    hole r*2 -xy at 0 0 (w/4) thru\n\
                    pattern hole1 circ 0 0 0 0 0 1 n a*n*2\n\
                    fillet r/2..r (edges along +z of extrude1)\n";
        // the same lines, so the same feature ids and names
        let typed = "w = 40\nh = w/2 + 5 mm\nn = 2 + 1\na = 90/n deg\nr = min(w, h)/10\n\
                     extrude 10 rect -20 -12.5 40 25\nhole 5 -xy at 0 0 10 thru\n\
                     pattern hole1 circ 0 0 0 0 0 1 3 180\nfillet 1.25..2.5 (edges along +z of extrude1)\n";
        assert_eq!(body(with), body(typed));
    }

    #[test]
    fn parameter_errors_say_what() {
        let e = |src: &str| error(src).1;
        assert_eq!(
            e("w = 3\nextrude wal rect 0 0 1 1\n"),
            "expected a number or a parameter, found 'wal'"
        );
        assert_eq!(e("w = w + 1\n"), "'w' refers to itself");
        assert_eq!(e("w = 1\nw = 2\n"), "parameter 'w' is already defined above");
        assert_eq!(
            e("rect = 1\n"),
            "'rect' is a word the file uses; pick another name"
        );
        assert_eq!(e("l2 = 1\n"), "'l2' is a word the file uses; pick another name");
        assert_eq!(
            e("a = 30 deg\nextrude a rect 0 0 1 1\n"),
            "'a' is in deg; a length is wanted"
        );
        assert_eq!(
            e("w = 3 mm\nrevolve w rect 1 0 1 1\n"),
            "'w' is in mm; an angle is wanted"
        );
        assert_eq!(e("a = 30 deg\nb = a + 2 mm\n"), "'a + 2' is in deg");
        assert_eq!(e("w = 3 mm\nb = w + 2 deg\n"), "'w + 2' is in mm");
        assert_eq!(e("a = 30 deg\nw = 2 mm\nb = a*w\n"), "mixes mm and deg");
        let (line, msg, hint) = error("a = b + 1\nb = 2\n");
        assert_eq!((line, msg.as_str()), (1, "unknown parameter 'b'"));
        assert_eq!(
            hint,
            "b is defined on line 2, below; define parameters above their use"
        );
        let (_, msg, _) = error(
            "n = 2.5\nextrude 1 rect 0 0 1 1\nhole 0.2 -xy at 0.5 0.5 1 thru\npattern hole1 lin 1 0 0 n 0.1\n",
        );
        assert_eq!(msg, "expected a count, found 'n' (2.5)");
    }

    #[test]
    fn changing_a_parameter_rebuilds_from_its_first_use() {
        let src = |w: f64| {
            format!(
                "w = {w}\nd = 2\nextrude 10 rect 0 0 30 30\nhole 3 -xy at 5 5 10 thru\n\
                 hole d*2 -xy at 15 15 10 thru\nhole w -xy at 25 25 10 thru\n"
            )
        };
        let mut m = built(&src(4.0));
        m.update(&src(5.0));
        assert_eq!(m.rebuilt, 1, "only the last hole uses w");
        assert_eq!(
            format!("{:?}", m.body().expect("built").compact()),
            body(&src(5.0))
        );
        m.update(&src(5.0).replace("d = 2", "d = 2.5"));
        assert_eq!(m.rebuilt, 2);
        m.update(&src(5.0).replace("d = 2", "d = 4/2"));
        assert_eq!(m.rebuilt, 2, "same values: nothing rebuilt past the hole using d");
        // the same, line for line
        let m2 = built(&src(5.0).replace("d = 2", "d = 4/2"));
        assert_eq!(
            m.body().expect("built").compact(),
            m2.body().expect("built").compact()
        );
    }

    const PLATE: &str = "w = 40\nh = 25\n\
        sk: sketch\n  rect 0 0 38 27\n  circle 20 12 4\n  fix l1.a\n  dist l1 w\n  dist l2 h\n\
          dia c1 8\n  hdist l1.a c1 (w/2)\n  vdist l1.a c1 h/2\nend\n\
        base: extrude 6 sk l1\n";

    #[test]
    fn a_sketch_drives_an_extrude() {
        let m = built(PLATE);
        let b = m.body().expect("built");
        let bb = b
            .face_ids()
            .fold(forge_math::Box3::EMPTY, |x, f| x.hull(&b.face_box(f)));
        assert_eq!(
            [bb.x.lo, bb.y.lo, bb.x.hi, bb.y.hi].map(|v| v.round()),
            [0.0, 0.0, 40.0, 25.0]
        );
        assert_eq!(at(&m, "faces cyl"), [20.0, 12.5, 3.0]);
        // as much as the same plate typed in
        let typed = "extrude 6 rect 0 0 40 25 circle 20 12.5 4 cw\n";
        let (mut a, mut b) = (built(PLATE), built(typed));
        assert!((a.stats().0 - b.stats().0).abs() < 1e-6);
    }

    /// Fillets named by handle survive a dimension change and a new circle in the sketch.
    #[test]
    fn edges_from_a_sketch_keep_their_names() {
        let m = built(PLATE);
        let (b, ents) = m.select("edges convex along +z", None).expect("selects");
        let names: Vec<String> = ents
            .iter()
            .map(|&e| format!("@{:016x}", select::name(b, e).0))
            .collect();
        assert_eq!(names.len(), 4);
        let src = format!("{PLATE}fillet 2 {}\n", names.join(" "));
        let before = built(&src);
        let after = built(&src.replace("w = 40", "w = 55").replace("h = 25", "h = 30"));
        let more = built(&src.replace(
            "  circle 20 12 4\n",
            "  circle 20 12 4\n  circle 5 5 1\n  rad c2 1\n",
        ));
        for m in [&before, &after, &more] {
            assert_eq!(m.select("faces cyl r=2", None).expect("selects").1.len(), 4);
        }
        assert_eq!(at(&after, "faces cyl r=4"), [27.5, 15.0, 3.0]);
    }

    #[test]
    fn sketches_on_faces_and_holes_at_sketch_points() {
        let src = format!(
            "{PLATE}top: sketch on (faces top of base)\n  pt 30 5\n  circle 30 20 3\n  rad c1 2\nend\n\
             hole 3 -on top p1 thru\nhole 2 -on top c1 4\nextrude to top rect 41 0 5 5\n"
        );
        let m = built(&src);
        assert_eq!(at(&m, "faces cyl r=1.5"), [30.0, 5.0, 3.0]);
        assert_eq!(at(&m, "faces cyl r=1"), [30.0, 20.0, 4.0]);
        // the extrude up to the sketch's plane is as tall as the plate
        let b = m.body().expect("built");
        let bb = b
            .face_ids()
            .fold(forge_math::Box3::EMPTY, |x, f| x.hull(&b.face_box(f)));
        assert_eq!((bb.x.hi.round(), bb.z.hi.round()), (46.0, 6.0));
        let (_, msg, _) = error(&src.replace("(faces top of base)", "(faces cyl)"));
        assert_eq!(msg, "(faces cyl) is not a planar face");
    }

    /// A sketch on a work plane follows it, and the file keeps saying where it is.
    #[test]
    fn a_sketch_follows_its_plane() {
        let src = |z: f64| {
            format!(
                "h = {z}\nextrude 20 rect 0 0 30 30\nplane at 0 0 h\nsk: sketch -on plane1 at 0 0 h\n  pt 10 -10\n\
                 end\nsketch on sk p1\n  circle 0 0 3\nend\nhole 6 on sketch2 c1 4\n"
            )
        };
        let mut m = built(&src(12.0));
        m.update(&src(20.0));
        assert_eq!(at(&m, "faces cyl r=3"), [10.0, 10.0, 18.0]);
        let printed = crate::text::print(&crate::text::parse(&src(20.0)).expect("parses"));
        assert!(
            printed.contains("\nsketch -on plane1 at 0 0 20\n  pt 10 -10\nend\nsketch on sketch1 p1\n"),
            "{printed}"
        );
    }

    #[test]
    fn a_revolve_about_a_sketch_line() {
        let src = "sketch xz\n  line 0 -5 0 25 cons\n  rect 10 0 5 20\n  fix l2.a\n  dist l2 5\n  dist l3 20\n\
                   end\nrevolve sketch1 axis l1\n";
        let typed = "revolve xz rect 10 0 5 20\n";
        let (mut a, mut b) = (built(src), built(typed));
        assert!((a.stats().0 - b.stats().0).abs() < 1e-6);
        assert_eq!(a.body().map(|b| b.faces.len()), b.body().map(|b| b.faces.len()));
    }

    /// As much solid as the same shapes typed in.
    fn same_volume(src: &str, typed: &str) {
        let (mut a, mut b) = (built(src), built(typed));
        let (va, vb) = (a.stats().0, b.stats().0);
        assert!(va > 0.0 && (va - vb).abs() < 1e-6 * vb, "{va} {vb}");
    }

    #[test]
    fn sweeps_lofts_and_coils_from_sketches() {
        same_volume(
            "sketch xz\n  line 0 0 0 30\n  arc 10 30 l1.b 10 40 cw\n  line a1.b 25 40\nend\n\
             sweep circle 0 0 2 path sketch1\n",
            "sweep circle 0 0 2 path line 0 0 0 0 0 30 arc xz at 10 0 30 10 180 -90 line 10 0 40 25 0 40\n",
        );
        // the same path named by a piece, drawn back to front
        same_volume(
            "sketch xz\n  line 25 40 10 40\n  arc 10 30 10 40 0 30\n  line 0 30 0 0\n  circle 50 50 3\nend\n\
             sweep z -1 0 0 at 25 0 40 circle 0 0 2 path sketch1 a1\n",
            "sweep z -1 0 0 at 25 0 40 circle 0 0 2 path line 25 0 40 10 0 40 arc xz at 10 0 30 10 90 90 line 0 0 30 0 0 0\n",
        );
        same_volume(
            "sketch\n  rect 0 0 10 10\nend\nsketch at 0 0 20\n  rect 2 2 6 6\nend\nloft profile sketch1 profile sketch2\n",
            "loft profile rect 0 0 10 10 profile at 0 0 20 rect 2 2 6 6\n",
        );
        same_volume(
            "sketch xz\n  circle 10 0 1\nend\ncoil sketch1 pitch 5 turns 2\n",
            "coil xz circle 10 0 1 pitch 5 turns 2\n",
        );
        let e = |rows: &str| {
            error(&format!(
                "sketch xz\n{rows}end\nsweep circle 0 0 1 path sketch1\n"
            ))
            .1
        };
        assert_eq!(
            e("  line 0 0 0 10\n  line 0 10 5 10\n  line 0 10 -5 10\n"),
            "the path branches at an end of l2"
        );
        assert_eq!(
            e("  line 0 0 0 10\n  line 5 0 5 10\n"),
            "the sketch has more than one chain; name one by an entity on it: l1 or l2"
        );
    }

    #[test]
    fn every_constraint_and_dimension() {
        let src = "r = 3\nsketch\n\
            poly 0 0 50 0 50 30 0 30\n\
            pt 25 15\n\
            pt 43 22\n\
            line 10 10 20 12 cons\n\
            line 30 10 40 12 cons\n\
            line 25 -5 25 45 cons\n\
            circle 12 22 3\n\
            circle 12 22 1.5 cons\n\
            circle 40 22 3\n\
            circle 45 15 5 cons\n\
            arc 25 40 15 40 35 40 cw cons\n\
            fix o l1.a\n\
            co l1.a o\n\
            hor l1 l3\n\
            ver l2 l4\n\
            hdist o l2.a 50\n\
            vdist l1.a l3.a 30\n\
            mid p1 l3\n\
            hor l5\n\
            par l5 l6\n\
            hor l5.a l6.a\n\
            dist l5 10\n\
            eq l5 l6\n\
            dist l5.a l1 10\n\
            hdist o l5.a 10\n\
            perp l5 l4\n\
            ver l7\n\
            co l7.a l1\n\
            hdist o l7.a 25\n\
            sym l5.a l6.b l7\n\
            conc c1 c2\n\
            rad c1 r\n\
            dia c2 r\n\
            eq c1 c3\n\
            hor c1 c3\n\
            dist c1 c3 28\n\
            hdist o c1 12\n\
            vdist o c1 22\n\
            co p2 c3\n\
            tan l2 c4\n\
            rad c4 2\n\
            vdist o c4 15\n\
            co a1.c l7\n\
            sym a1.a a1.b l7\n\
            dist a1.a a1.b 20\n\
            ang l1 l7 90\n\
            end\nextrude 4 sketch1 l1\n";
        let m = built(src);
        let forge_model::Feature::Sketch(s) = &m.features[1] else {
            panic!("a sketch")
        };
        let pt = |t: &str| super::point(s, t).map(|p| [p.x, p.y].map(|v| (v * 1e9).round() / 1e9 + 0.0));
        assert_eq!(pt("p1"), Ok([25.0, 30.0]));
        assert_eq!(pt("l6.a"), Ok([30.0, 10.0]));
        assert_eq!(pt("l6.b"), Ok([40.0, 10.0]));
        assert_eq!(pt("c3"), Ok([40.0, 22.0]));
        assert_eq!(pt("c4"), Ok([48.0, 15.0]));
        assert_eq!(pt("a1.a").map(|p| p[0]), Ok(15.0));
        let p2 = super::point(s, "p2").expect("a point");
        assert!((p2.distance(forge_math::Point2::new(40.0, 22.0)) - 3.0).abs() < 1e-9);
        // p2 round c3, the arc's centre and ends up and down the axis, the axis' top
        assert_eq!(
            super::freedom(s),
            (4, ["p2", "l7", "a1"].map(String::from).to_vec())
        );
        assert_eq!(m.select("faces cyl r=3", None).expect("selects").1.len(), 2);
    }

    #[test]
    fn sketch_diagnostics() {
        let e = |rows: &str| error(&format!("w = 3\nsketch\n{rows}end\nextrude 1 sketch1\n"));
        let (line, msg, hint) = e("  rect 0 0 10 10\n  dist l1 w\n  dist l1 5\n");
        assert_eq!(line, 2);
        assert_eq!(
            msg,
            "the sketch does not solve: `dist l1 w`, `dist l1 5` conflict"
        );
        assert!(hint.starts_with("change or remove one"), "{hint}");
        assert_eq!(
            e("  rect 0 0 10 10\n  dia c9 3\n").1,
            "`dia c9 3`: no entity 'c9' above this row"
        );
        assert_eq!(
            e("  rect 0 0 10 10\n  dia l1 3\n").1,
            "`dia l1 3`: an entity of the wrong kind for this row"
        );
        assert_eq!(
            e("  rect 0 0 10 10\n  dist l1 -3\n").1,
            "`dist l1 -3`: a value out of range (lengths and radii > 0)"
        );
        assert_eq!(
            e("  rect 0 0 10 10\n  glue l1\n").1.split(':').nth(1),
            Some(
                " unknown row (pt line poly rect circle arc | fix co hor ver par perp tan eq conc mid sym | dist hdist vdist rad dia ang)"
            )
        );
        assert_eq!(
            e("  rect 0 0 10 10\n  circle 30 30 2\n").1,
            "the sketch has 2 regions; name one by an entity on its outside: c1 l1"
        );
        assert_eq!(
            e("  poly 0 0 10 0 10 10 open\n").1,
            "the chain through l1 is open"
        );
        assert_eq!(
            e("  rect 0 0 10 10\n  line 5 -5 5 15\n").1,
            "the chain through l5 is open"
        );
        assert_eq!(e("  rect 0 0 10 10\n  rect 5 5 10 10\n").1, "l2 and l5 cross");
        assert_eq!(e("  rect 0 0 10 10 cons\n").1, "the sketch encloses nothing");
        assert_eq!(
            e("  rect 0 0 10 10\n  fix l1.d\n").1,
            "`fix l1.d`: 'l1.d' is not a point"
        );
        assert_eq!(
            e("  rect 0 0 10 10\n  co l1 l2\n").1,
            "`co l1 l2`: co needs a point"
        );
        // under-constrained is fine, and says how free it is
        let m = built("sketch\n  rect 0 0 10 10\nend\nextrude 1 sketch1\n");
        let forge_model::Feature::Sketch(s) = &m.features[0] else {
            panic!("a sketch")
        };
        assert_eq!(
            super::freedom(s),
            (
                4,
                vec!["l1", "l2", "l3", "l4"]
                    .into_iter()
                    .map(String::from)
                    .collect()
            )
        );
    }

    /// A sketch written at rest reads back the same, every time and through print.
    #[test]
    fn sketches_and_parameters_print_back() {
        let src = format!("{PLATE}fillet 1 (edges convex along +z)\n");
        let part =
            crate::text::parse(&src.replace("(edges convex along +z)", "@0000000000000001")).expect("parses");
        let printed = crate::text::print(&part);
        assert!(
            printed.starts_with("w = 40\nh = 25\nsketch\n  rect 0 0 38 27\n"),
            "{printed}"
        );
        assert!(
            printed.contains("\n  vdist l1.a c1 h/2\nend\nextrude 6 "),
            "{printed}"
        );
        assert_eq!(crate::text::parse(&printed).expect("parses"), part);
        let m = built(&src);
        let mut m2 = built(&src.replace("h/2", "(h / 2)"));
        assert_eq!(m.body(), m2.body());
        m2.update(&src);
        assert_eq!(
            m2.rebuilt, 0,
            "the sketch reads the same, so nothing below rebuilds"
        );
    }
}
