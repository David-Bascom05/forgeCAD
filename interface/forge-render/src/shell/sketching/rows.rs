//! The sketcher's working form: the rows its tools write (`line 0 0 20 0`, `rect X Y W H`,
//! `hor l1`, `dist l1 20`, `project edge T`), built straight into a `forge_sketch::Sketch` and
//! solved as the drawing changes, with the points, lines and regions the tools read back. The
//! rows live only in the app while a sketch is open: `fgsketch` writes them as a Forge block and
//! reads a block back into them. Values in a row are numbers, or Forge expressions read through
//! the sketcher's `Num` (see `forge_value`).

use forge_math::{Frame, Point2, dmath, tol};
use forge_model::Sketch;
use forge_ops::Profile;
use forge_sketch::project::flat;
pub use forge_sketch::project::{Flat, Source};
use forge_sketch::{Constraint as C, ConstraintId, Entity, EntityId, ExtractError, SketchError, Status};
use forge_topo::Name;

type R<T> = Result<T, String>;

/// What a number slot of a row wants: a length (mm), an angle (degrees in the row, radians out)
/// or a count.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Want {
    Len,
    Angle,
    Count,
}

/// Reads a number token (a value or an expression) as the slot wants it.
pub type Num<'a> = dyn Fn(&str, Want) -> R<f64> + 'a;

/// What one reference of a `project` row gives: a piece of the part to lay flat, or (a sketch
/// opened from its Forge block) the image the block solved to.
#[derive(Clone, Debug, PartialEq)]
pub enum Src {
    Model(Source),
    Laid(Flat),
}

/// What one reference of a `project` row names: each source with its entity's name, and whether
/// it is always just one (an edge), so rows may name it (`j1`).
#[derive(Clone, Debug, PartialEq)]
pub struct Refd {
    pub sources: Vec<(Name, Src)>,
    pub one: bool,
}

/// Resolves a `project` row's tokens after the word.
pub type Sources<'a> = dyn Fn(&[String]) -> R<Vec<Refd>> + 'a;

/// A row's words: split at spaces outside brackets and quotes, so `edges(between: (a, b))` is one.
pub fn tokens(text: &str) -> Vec<String> {
    let mut toks: Vec<String> = Vec::new();
    let mut start = None;
    let mut depth = 0;
    let (mut quoted, mut escaped) = (false, false);
    for (k, c) in text.char_indices() {
        match start {
            Some(s) if c.is_whitespace() && depth == 0 && !quoted => {
                toks.push(text[s..k].to_string());
                start = None;
            }
            None if c.is_whitespace() => continue,
            None => start = Some(k),
            _ => {}
        }
        if quoted {
            (quoted, escaped) = (escaped || c != '"', !escaped && c == '\\');
        } else if c == '"' {
            quoted = true;
        } else {
            depth = (depth + (c == '(') as i32 - (c == ')') as i32).max(0);
        }
    }
    if let Some(s) = start {
        toks.push(text[s..].to_string());
    }
    toks
}

/// Builds a sketch's rows on `plane` and solves it from the coordinates as written.
/// `was`, the sketch this line read as before, lends the solve every part of it that did
/// not change.
pub fn build(plane: Frame, rows: Vec<Vec<String>>, num: &Num, was: Option<&Sketch>) -> R<Sketch> {
    let none = |_: &[String]| Err("this needs a part to project from".to_string());
    build_on(plane, rows, num, &none, was)
}

/// `build`, with what `project` rows name.
pub fn build_on<'n>(
    plane: Frame,
    rows: Vec<Vec<String>>,
    num: &Num<'n>,
    src: &Sources<'n>,
    was: Option<&Sketch>,
) -> R<Sketch> {
    if let Some(s) = was.and_then(|w| values_only(plane, &rows, num, w)) {
        return Ok(Sketch { rows, ..s });
    }
    let mut b = B {
        s: forge_sketch::Sketch::new(plane),
        names: Vec::new(),
        made: Vec::new(),
        driven: Vec::new(),
        num,
        src,
        pins: Vec::new(),
        flats: Vec::new(),
    };
    for (k, row) in rows.iter().enumerate() {
        b.row(k, row).map_err(|e| format!("`{}`: {e}", row.join(" ")))?;
    }
    ids_of_rows(&rows)?;
    let written = b.s.params().to_vec();
    if b.s.solve_after(was.map(|w| &w.solved)).status != Status::Solved {
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
    let dims = b
        .made
        .iter()
        .filter(|m| DIMENSIONS.contains(&rows[m.1][0].as_str()))
        .map(|m| (m.1, m.0))
        .collect();
    let driven_of = b.driven.iter().map(|d| (d.1.clone(), d.2.clone())).collect();
    Ok(Sketch {
        plane,
        on: None,
        at: None,
        rows,
        solved: b.s,
        driven,
        names: b.names,
        written,
        dims,
        driven_of,
    })
}

/// `was` again with new dimension values, when nothing else in the rows changed: the
/// dimensions set in place and solved from the coordinates as written, which is what reading
/// the rows afresh gives. None when anything else changed, or it fails (the full read says why).
fn values_only(plane: Frame, rows: &[Vec<String>], num: &Num, was: &Sketch) -> Option<Sketch> {
    if was.plane != plane || was.rows.len() != rows.len() {
        return None;
    }
    // every token but a dimension's value is a number, a name or a word, as before
    const WORDS: [&str; 35] = [
        "ids", "pt", "line", "poly", "rect", "circle", "arc", "fix", "co", "hor", "ver", "par", "perp",
        "tan", "eq", "conc", "colin", "mid", "sym", "dist", "hdist", "vdist", "rad", "dia", "ang", "ref",
        "cw", "cons", "open", "spline", "bspline", "ellipse", "earc", "knots", "weights",
    ];
    let fixed = |t: &str| t.parse::<f64>().is_ok() || is_ref(t) || WORDS.contains(&t);
    for (a, b) in rows.iter().zip(&was.rows) {
        let dim = DIMENSIONS.contains(&a[0].as_str());
        let n = a.len() - dim as usize;
        if a.len() != b.len() || a[..n] != b[..n] || !a[..n].iter().all(|t| fixed(t)) {
            return None;
        }
    }
    let mut q = was.solved.clone();
    q.set_params(&was.written).ok()?;
    for &(k, c) in &was.dims {
        let row = &rows[k];
        let want = if row[0] == "ang" { Want::Angle } else { Want::Len };
        q.set_value(c, num(row.last()?, want).ok()?).ok()?;
    }
    if q.solve_after(Some(&was.solved)).status != Status::Solved || !q.implied().is_empty() {
        return None;
    }
    let b = B {
        s: q,
        names: was.names.clone(),
        made: Vec::new(),
        driven: Vec::new(),
        num,
        src: &|_| Ok(Vec::new()),
        pins: Vec::new(),
        flats: Vec::new(),
    };
    let driven = was
        .driven
        .iter()
        .zip(&was.driven_of)
        .map(|(d, (kw, es))| b.measure(kw, es).ok().map(|v| (d.0, v)))
        .collect::<Option<Vec<_>>>()?;
    Some(Sketch {
        plane,
        on: None,
        at: None,
        rows: Vec::new(),
        solved: b.s,
        driven,
        names: b.names,
        written: was.written.clone(),
        dims: was.dims.clone(),
        driven_of: was.driven_of.clone(),
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
    src: &'a Sources<'n>,
    /// The projected points so far: where each is held.
    pins: Vec<(Point2, EntityId)>,
    /// The projected lines, circles and arcs so far.
    flats: Vec<EntityId>,
}

/// Whether a token names a sketch entity or one of its points (`o p1 l2 a3.c c4`) rather
/// than being a number.
pub fn is_ref(t: &str) -> bool {
    let base = t.split_once('.').map_or(t, |(b, p)| {
        if ["a", "b", "c", "m"].contains(&p) || (b.starts_with('s') && p.bytes().all(|c| c.is_ascii_digit()))
        {
            b
        } else {
            ""
        }
    });
    base == "o"
        || base.len() > 1
            && base.starts_with(['p', 'l', 'a', 'c', 's', 'e', 'j'])
            && base[1..].bytes().all(|c| c.is_ascii_digit())
}

fn bad(e: SketchError) -> String {
    match e {
        SketchError::WrongKind(_) => "an entity of the wrong kind for this row".into(),
        SketchError::BadValue => "a value out of range (lengths and radii > 0)".into(),
        e => format!("{e:?}"),
    }
}

/// Whether a projected entity is this one again: the same held points, the same way.
fn same_flat(s: &forge_sketch::Sketch, e: EntityId, kind: Entity, r: f64) -> bool {
    match (s.entity(e), kind) {
        (Some(Entity::Line { a, b }), Entity::Line { a: x, b: y }) => (a, b) == (x, y) || (a, b) == (y, x),
        (Some(Entity::Circle { center, .. }), Entity::Circle { center: c, .. }) => {
            center == c && (s.radius(e) - r).abs() <= tol::LINEAR
        }
        (
            Some(Entity::Arc {
                center,
                start,
                end,
                ccw,
            }),
            Entity::Arc {
                center: c,
                start: x,
                end: y,
                ccw: w,
            },
        ) => center == c && ((start, end, ccw) == (x, y, w) || (start, end, ccw) == (y, x, !w)),
        _ => false,
    }
}

impl B<'_, '_> {
    fn add(&mut self, k: usize, c: C) -> R<()> {
        let id = self.s.add_constraint(c).map_err(bad)?;
        self.made.push((id, k));
        Ok(())
    }

    /// A held point at `p`; one already within LINEAR is that point, so what meets there shares it.
    fn pin(&mut self, k: usize, p: Point2) -> R<EntityId> {
        if let Some(at) = self.pins.iter().find(|q| q.0.distance(p) <= tol::LINEAR) {
            return Ok(at.1);
        }
        let id = self.s.add_point(p);
        self.add(k, C::Fixed(id))?;
        self.pins.push((p, id));
        Ok(id)
    }

    /// The entity for a laid-flat source: the one already made if the same again.
    fn laid(&mut self, k: usize, f: Flat) -> R<EntityId> {
        if let Flat::Spline(pts, closed) = &f {
            let ids = pts.iter().map(|&p| self.pin(k, p)).collect::<R<Vec<_>>>()?;
            return self.s.add_spline(&ids, *closed).map_err(bad);
        }
        if let Flat::Ellipse {
            center,
            major,
            b,
            ends,
        } = f
        {
            let (c, m) = (self.pin(k, center)?, self.pin(k, major)?);
            let ends = match ends {
                Some((p, q, ccw)) => Some((self.pin(k, p)?, self.pin(k, q)?, ccw)),
                None => None,
            };
            let same = |s: &forge_sketch::Sketch, e: EntityId| match s.entity(e) {
                Some(Entity::Ellipse {
                    center,
                    major,
                    b: r,
                    ends: x,
                }) => {
                    let ends_same = match (x, ends) {
                        (None, None) => true,
                        (Some((p, q, w)), Some((p2, q2, w2))) => {
                            (p, q, w) == (p2, q2, w2) || (p, q, w) == (q2, p2, !w2)
                        }
                        _ => false,
                    };
                    center == c && major == m && (s.params()[r] - b).abs() <= tol::LINEAR && ends_same
                }
                _ => false,
            };
            if let Some(&e) = self.flats.iter().find(|&&e| same(&self.s, e)) {
                return Ok(e);
            }
            let e = match ends {
                Some((p, q, ccw)) => self.s.add_ellipse_arc(c, m, b, p, q, ccw),
                None => self.s.add_ellipse(c, m, b),
            }
            .map_err(bad)?;
            // an arc's ends, held, hold its minor axis
            if ends.is_none() {
                self.add(k, C::EllipseMinor(e, b))?;
            }
            self.flats.push(e);
            return Ok(e);
        }
        let r = if let Flat::Circle(_, r) = f { r } else { 0.0 };
        let kind = match f {
            Flat::Point(p) => return self.pin(k, p),
            Flat::Line(a, b) => {
                let (a, b) = (self.pin(k, a)?, self.pin(k, b)?);
                if a == b {
                    return Ok(a);
                }
                Entity::Line { a, b }
            }
            Flat::Circle(c, _) => Entity::Circle {
                center: self.pin(k, c)?,
                r: 0,
            },
            Flat::Arc {
                center,
                start,
                end,
                ccw,
            } => Entity::Arc {
                center: self.pin(k, center)?,
                start: self.pin(k, start)?,
                end: self.pin(k, end)?,
                ccw,
            },
            Flat::Ellipse { .. } | Flat::Spline(..) => unreachable!("made above"),
        };
        if let Some(&e) = self.flats.iter().find(|&&e| same_flat(&self.s, e, kind, r)) {
            return Ok(e);
        }
        let e = match kind {
            Entity::Line { a, b } => self.s.add_line(a, b),
            Entity::Circle { center, .. } => self.s.add_circle(center, r),
            Entity::Arc {
                center,
                start,
                end,
                ccw,
            } => self.s.add_arc(center, start, end, ccw),
            _ => unreachable!("only those are made above"),
        }
        .map_err(bad)?;
        if matches!(kind, Entity::Circle { .. }) {
            self.add(k, C::Radius(e, r))?;
        }
        self.flats.push(e);
        Ok(e)
    }

    /// `project REF..`: each reference's geometry, held where it falls on the plane.
    fn project(&mut self, k: usize, c: &Cur) -> R<()> {
        if c.done() {
            return Err("project needs an edge, face, vertex or work point".into());
        }
        // `approx TOL` at the end: what has no exact image is fitted within TOL, and says so
        let mut toks = &c.t[c.k..];
        let mut approx = None;
        if toks.len() > 2 && toks[toks.len() - 2] == "approx" {
            let t = (self.num)(&toks[toks.len() - 1], Want::Len)?;
            if !(t >= 1e-6 && t.is_finite()) {
                return Err("approx takes a tolerance of 0.000001 mm or more".into());
            }
            approx = Some(t);
            toks = &toks[..toks.len() - 2];
        }
        let refs = (self.src)(toks)?;
        for r in refs {
            for (n, s) in &r.sources {
                let f = match s {
                    Src::Model(s) => flat(&self.s.plane, s, approx)?,
                    Src::Laid(f) => f.clone(),
                };
                let e = self.laid(k, f)?;
                if r.one {
                    self.name('j', e);
                } else {
                    // how many a face or selector gives can change: no row may name one, and its
                    // faces take their names from the edge it came from
                    self.names.push((hidden(*n), e));
                }
            }
        }
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
        let hint = if self.names.iter().any(|n| n.0.starts_with('#')) {
            "; a face's or selector's curves can change in number, so rows can't name them: list the edges, `project @e.. @e..`"
        } else {
            ""
        };
        Err(format!("no entity '{t}' above this row{hint}"))
    }

    /// A point: `name.a|b|c`, a point's name, or a circle's or arc's (its centre).
    fn point_ref(&mut self, t: &str) -> R<EntityId> {
        let (base, part) = t.split_once('.').unwrap_or((t, ""));
        let e = self.entity(base)?;
        let kind = self.s.entity(e).expect("named");
        if let Entity::Spline { .. } = kind {
            let pts = self.s.spline_points(e).expect("a spline");
            return match part {
                "a" => Ok(pts[0]),
                "b" => Ok(pts[pts.len() - 1]),
                n => n
                    .parse::<usize>()
                    .ok()
                    .and_then(|i| pts.get(i).copied())
                    .ok_or_else(|| {
                        format!(
                            "'{t}' is not a point of the spline ({} points, from 0)",
                            pts.len()
                        )
                    }),
            };
        }
        Ok(match (kind, part) {
            (Entity::Point { .. }, "") => e,
            (Entity::Ellipse { center, .. }, "" | "c") => center,
            (Entity::Ellipse { major, .. }, "m") => major,
            (
                Entity::Ellipse {
                    ends: Some((a, _, _)),
                    ..
                },
                "a",
            ) => a,
            (
                Entity::Ellipse {
                    ends: Some((_, b, _)),
                    ..
                },
                "b",
            ) => b,
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
        if row[0] == "ids" {
            return ids_of_rows(std::slice::from_ref(&row.to_vec())).map(|_| ());
        }
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
            "project" => {
                self.project(k, &c)?;
                c.k = c.t.len();
            }
            "arc" => {
                let (center, a, b) = (self.point(&mut c)?, self.point(&mut c)?, self.point(&mut c)?);
                let ccw = !c.eat("cw");
                let e = self.s.add_arc(center, a, b, ccw).map_err(bad)?;
                self.name('a', e);
            }
            "spline" | "bspline" => {
                let degree = if row[0] == "bspline" {
                    (self.num)(c.next()?, Want::Count)? as usize
                } else {
                    0
                };
                let mut pts = Vec::new();
                while !c.done() && !["open", "knots", "weights"].contains(&c.peek()) {
                    pts.push(self.point(&mut c)?);
                }
                let (mut knots, mut weights) = (None, None);
                while c.peek() == "knots" || c.peek() == "weights" {
                    let weighted = c.next()? == "weights";
                    let mut v = Vec::new();
                    while !c.done() && !["open", "knots", "weights"].contains(&c.peek()) {
                        v.push(self.len(&mut c)?);
                    }
                    *if weighted { &mut weights } else { &mut knots } = Some(v);
                }
                let closed = !c.eat("open");
                let e = if row[0] == "spline" {
                    if pts.len() < 2 || (closed && pts.len() < 3) {
                        return Err("spline needs two points, or three to close".into());
                    }
                    self.s.add_spline(&pts, closed).map_err(bad)?
                } else {
                    self.s
                        .add_bspline(degree, &pts, knots, weights, closed)
                        .map_err(|e| format!("{}: bspline needs more points than its degree", bad(e)))?
                };
                self.name('s', e);
            }
            "ellipse" | "earc" => {
                let (center, major) = (self.point(&mut c)?, self.point(&mut c)?);
                let b = self.len(&mut c)?;
                let e = if row[0] == "ellipse" {
                    self.s.add_ellipse(center, major, b).map_err(bad)?
                } else {
                    let (s0, s1) = (self.point(&mut c)?, self.point(&mut c)?);
                    let ccw = !c.eat("cw");
                    self.s
                        .add_ellipse_arc(center, major, b, s0, s1, ccw)
                        .map_err(bad)?
                };
                self.name('e', e);
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
                        Entity::Spline { .. } => self.s.spline_points(e).expect("a spline").to_vec(),
                        Entity::Ellipse {
                            center, major, ends, ..
                        } => {
                            let mut v = vec![center, major];
                            if let Some((a, b, _)) = ends {
                                v.extend([a, b]);
                            }
                            v
                        }
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
                    Entity::Ellipse { .. } => C::PointOnEllipse(p, q),
                    Entity::Spline { .. } => {
                        return Err("co on a spline: name its point (s1.a, s1.b, s1.0, s1.1 ..)".into());
                    }
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
                if kw == "tan" {
                    let kind = |s: &Self, e: EntityId| s.s.entity(e).expect("named");
                    match (kind(self, a), kind(self, b)) {
                        (Entity::Spline { .. }, _) | (_, Entity::Spline { .. }) => {
                            let (sp, other) = if matches!(kind(self, a), Entity::Spline { .. }) {
                                (a, b)
                            } else {
                                (b, a)
                            };
                            if let Some(id) = self.s.tangent_spline(sp, other).map_err(|_| {
                                "tan on a spline: a line or arc that ends at the spline's start or end"
                                    .to_string()
                            })? {
                                self.made.push((id, k));
                            }
                            return c.end();
                        }
                        (Entity::Ellipse { .. }, Entity::Line { .. })
                        | (Entity::Line { .. }, Entity::Ellipse { .. }) => {
                            let (l, e) = if matches!(kind(self, a), Entity::Line { .. }) {
                                (a, b)
                            } else {
                                (b, a)
                            };
                            let ids = self.s.tangent_ellipse(l, e).map_err(bad)?;
                            self.made.extend(ids.map(|id| (id, k)));
                            return c.end();
                        }
                        _ => {}
                    }
                }
                let con = match kw {
                    "par" => C::Parallel(a, b),
                    "perp" => C::Perpendicular(a, b),
                    "tan" => C::Tangent(a, b),
                    _ => C::Concentric(a, b),
                };
                self.add(k, con)?;
            }
            "colin" => {
                // parallel, and the second's start on the first
                let (a, b) = (self.entity_tok(c)?, self.entity_tok(c)?);
                let Some(Entity::Line { a: start, .. }) = self.s.entity(b) else {
                    return Err("colin takes two lines".into());
                };
                if !line(self, a) {
                    return Err("colin takes two lines".into());
                }
                self.add(k, C::Parallel(a, b))?;
                self.add(k, C::PointOnLine(start, a))?;
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
                if matches!(self.s.entity(e), Some(Entity::Ellipse { .. })) {
                    let d = if kw == "rad" { d } else { 0.5 * d };
                    self.add(k, C::EllipseMinor(e, d))?;
                    return c.end();
                }
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
                    "unknown row (pt line poly rect circle arc project spline bspline ellipse earc | fix co hor ver par perp tan eq conc colin mid sym | dist hdist vdist rad dia ang)"
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
            ("rad" | "dia", [c]) if matches!(q.entity(*c), Some(Entity::Ellipse { .. })) => {
                let Some(Entity::Ellipse { b, .. }) = q.entity(*c) else {
                    unreachable!("just matched")
                };
                q.params()[b] * if kw == "dia" { 2.0 } else { 1.0 }
            }
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
            Entity::Circle { center, .. } | Entity::Arc { center, .. } | Entity::Ellipse { center, .. } => {
                Ok(center)
            }
            Entity::Line { .. } => Err("a line where a point is wanted".into()),
            Entity::Spline { .. } => Err("a spline where a point is wanted: s1.a, s1.b, s1.0 ..".into()),
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
    if let Some(h) = name.strip_prefix('#') {
        return (u64::from_str_radix(h, 16).unwrap_or(0) >> 20) << 2 | 3 | 1 << 46;
    }
    let n: u64 = name[1..].parse().unwrap_or(0);
    let k = match name.as_bytes()[0] {
        b'l' => 0,
        b'a' => 1,
        b's' => 3,
        // two bits ran out: ellipses sit above any other kind's count
        b'e' => return n << 2 | 3 | 1 << 40,
        b'j' => return n << 2 | 3 | 1 << 41,
        _ => 2,
    };
    n << 2 | k
}

/// Where unlisted entities' ids go in a sketch with an `ids` row: past any a position can be, so a
/// line drawn later never takes a listed entity's id.
const UNLISTED: u64 = 1 << 32;

/// `ids NAME N ..`: these entities keep these segment ids, whatever else is added to the sketch.
/// What a profile written inline had, when it became a sketch: its faces keep their names.
fn ids_of_rows(rows: &[Vec<String>]) -> R<Vec<(String, u64)>> {
    let mut out: Vec<(String, u64)> = Vec::new();
    for row in rows.iter().filter(|r| r.first().is_some_and(|w| w == "ids")) {
        for pair in row[1..].chunks(2) {
            let [name, n] = pair else {
                return Err("ids NAME N ..".into());
            };
            let n: u64 = n.parse().map_err(|_| format!("ids: '{n}' is not an id"))?;
            if n >= UNLISTED {
                return Err(format!("ids: {n} is too large"));
            }
            if out.iter().any(|o| o.0 == *name) {
                return Err(format!("ids: {name} is listed twice"));
            }
            if out.iter().any(|o| o.1 == n) {
                return Err(format!("ids: two entities have {n}"));
            }
            out.push((name.clone(), n));
        }
    }
    Ok(out)
}

/// A profile segment's entity id, what a cut of a kinked or closed curve adds taken off (the
/// piece number stays on the stable id, so piece 0 reads as the entity).
const PIECE: u64 = 0xffff << 48;

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
            let outs: Vec<&str> = ps
                .iter()
                .map(|p| match name_of(s, p.loops[0].segments[0].1 & !PIECE) {
                    n if n.starts_with('#') => "(a face's curve: list its edges in `project` to name it)",
                    n => n,
                })
                .collect();
            return Err(format!(
                "the sketch has {} regions; name one by an entity on its outside: {}",
                ps.len(),
                outs.join(" ")
            ));
        }
        Some(t) => {
            let id = id_of(s, t)?.0;
            ps.into_iter()
                .find(|p| p.loops[0].segments.iter().any(|g| g.1 & !PIECE == id))
                .ok_or_else(|| format!("{t} is not on the outside of a region"))?
        }
    };
    let listed = ids_of_rows(&s.rows)?;
    for l in &mut p.loops {
        for g in &mut l.segments {
            // an arc dimensioned by a sketch made from a line has that radius exactly, not the one
            // its rounded points give back
            if !listed.is_empty()
                && let forge_ops::Segment::Arc { radius, .. } = &mut g.0
                && let Some(d) = s
                    .solved
                    .constraint_ids()
                    .find_map(|c| match s.solved.constraint(c) {
                        Some(C::Radius(e, d)) if e.0 == g.1 => Some(d),
                        _ => None,
                    })
            {
                *radius = d;
            }
            let (piece, name) = (g.1 & PIECE, name_of(s, g.1 & !PIECE));
            g.1 = piece
                | match listed.iter().find(|o| o.0 == name) {
                    Some(o) => o.1,
                    None if listed.is_empty() => stable(name),
                    None => stable(name) + UNLISTED,
                };
        }
    }
    Ok(p)
}

/// A closed region of a sketch, as a line that reads it would name it.
#[derive(Clone, Debug, PartialEq)]
pub struct Region {
    /// An entity on its outside: what picks it out of the others.
    pub through: String,
    /// Its outside, then its holes, as points around each.
    pub loops: Vec<Vec<Point2>>,
    /// Its area (less its holes) and the length round every loop, exact from its curves (a
    /// spline's length to its samples).
    pub area: f64,
    pub perimeter: f64,
    /// Each loop's own area and length, in `loops`' order.
    pub loop_areas: Vec<f64>,
    pub loop_lengths: Vec<f64>,
}

/// A profile curve's length: exact for lines, arcs and circles.
fn segment_length(g: &forge_ops::Segment) -> f64 {
    use forge_ops::Segment;
    match g {
        Segment::Line { a, b } => (*b - *a).length(),
        Segment::Arc { radius, sweep, .. } => radius * sweep.abs(),
        Segment::Circle { radius, .. } => core::f64::consts::TAU * radius,
        Segment::Spline(_) => g.samples(512).windows(2).map(|w| (w[1] - w[0]).length()).sum(),
    }
}

/// Points along a curve, about one per `STEP` radians where it bends.
fn along(g: &forge_ops::Segment) -> Vec<Point2> {
    const STEP: f64 = core::f64::consts::TAU / 64.0;
    let arc = |c: Point2, r: f64, t0: f64, sweep: f64| -> Vec<Point2> {
        let n = ((sweep.abs() / STEP).ceil() as usize).max(1);
        (0..=n)
            .map(|k| {
                let t = t0 + sweep * k as f64 / n as f64;
                Point2::new(c.x + r * dmath::cos(t), c.y + r * dmath::sin(t))
            })
            .collect()
    };
    match g {
        forge_ops::Segment::Line { a, b } => vec![*a, *b],
        forge_ops::Segment::Arc {
            center,
            radius,
            start,
            sweep,
        } => arc(*center, *radius, *start, *sweep),
        forge_ops::Segment::Circle { center, radius, .. } => {
            arc(*center, *radius, 0.0, core::f64::consts::TAU)
        }
        forge_ops::Segment::Spline(_) => g.samples(64),
    }
}

/// The closed regions it has now: none if what's drawn doesn't close.
pub fn regions(s: &Sketch) -> Vec<Region> {
    let Ok(ps) = forge_sketch::profiles(&s.solved) else {
        return Vec::new();
    };
    ps.iter()
        .map(|p| Region {
            // one no row can name (a face's projected curves) leaves it to be the sketch's only region
            through: p.loops[0]
                .segments
                .iter()
                .map(|g| name_of(s, g.1 & !PIECE))
                .find(|n| !n.starts_with('#'))
                .unwrap_or("")
                .to_string(),
            loops: p
                .loops
                .iter()
                .map(|l| l.segments.iter().flat_map(|g| along(&g.0)).collect())
                .collect(),
            area: p.area(),
            perimeter: p
                .loops
                .iter()
                .flat_map(|l| &l.segments)
                .map(|g| segment_length(&g.0))
                .sum(),
            loop_areas: p.loops.iter().map(|l| l.signed_area().abs()).collect(),
            loop_lengths: p
                .loops
                .iter()
                .map(|l| l.segments.iter().map(|g| segment_length(&g.0)).sum())
                .collect(),
        })
        .collect()
}

/// What it draws: each curve as points, whether it's construction; and its end points.
pub fn drawing(s: &Sketch) -> (Vec<(Vec<Point2>, bool)>, Vec<Point2>) {
    let q = &s.solved;
    let (mut curves, mut points) = (Vec::new(), Vec::new());
    for id in q.entity_ids() {
        let faint = q.is_construction(id);
        let g = match q.entity(id) {
            Some(Entity::Spline { .. }) => {
                if let Some(pts) = q.spline_points(id) {
                    points.extend(pts.iter().map(|p| q.point(*p)));
                }
                curves.push((q.spline_samples(id, 64), faint));
                continue;
            }
            Some(Entity::Ellipse { center, .. }) => {
                points.push(q.point(center));
                curves.push((q.spline_samples(id, 64), faint));
                continue;
            }
            Some(Entity::Line { a, b }) => {
                points.extend([q.point(a), q.point(b)]);
                forge_ops::Segment::Line {
                    a: q.point(a),
                    b: q.point(b),
                }
            }
            Some(Entity::Circle { center, .. }) => {
                points.push(q.point(center));
                forge_ops::Segment::Circle {
                    center: q.point(center),
                    radius: q.radius(id),
                    ccw: true,
                }
            }
            Some(Entity::Arc {
                center,
                start,
                end,
                ccw,
            }) => {
                let (c, a, b) = (q.point(center), q.point(start), q.point(end));
                points.extend([a, b, c]);
                let t0 = dmath::atan2(a.y - c.y, a.x - c.x);
                let t1 = dmath::atan2(b.y - c.y, b.x - c.x);
                let turn = core::f64::consts::TAU;
                let sweep = if ccw {
                    (t1 - t0).rem_euclid(turn)
                } else {
                    -(t0 - t1).rem_euclid(turn)
                };
                forge_ops::Segment::Arc {
                    center: c,
                    radius: a.distance(c),
                    start: t0,
                    sweep,
                }
            }
            _ => continue,
        };
        curves.push((along(&g), faint));
    }
    (curves, points)
}

/// A point of the sketch (`p1`, `l2.b`, `c1` for its centre), in its plane.
pub fn point(s: &Sketch, t: &str) -> R<Point2> {
    id_of(s, t.split_once('.').map_or(t, |p| p.0))?;
    s.point(t).ok_or_else(|| format!("'{t}' is not a point"))
}

/// A line of the sketch: its ends, in its plane.
pub fn line(s: &Sketch, t: &str) -> R<(Point2, Point2)> {
    match s.solved.entity(id_of(s, t)?) {
        Some(Entity::Line { a, b }) => Ok((s.solved.point(a), s.solved.point(b))),
        _ => Err(format!("'{t}' is not a line")),
    }
}

/// The name of a result of `project` that rows can't name: `#` starts a comment, so none can write it.
fn hidden(n: Name) -> String {
    format!("#{:016x}", n.0)
}

/// Whether a name in a sketch is `project`'s.
pub fn is_projected(name: &str) -> bool {
    name.starts_with(['j', '#'])
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
    use super::*;
    use forge_geom::{Circle, Curve};
    use forge_math::{Interval, Point3};

    fn built(rows: &[&str]) -> R<Sketch> {
        let toks: Vec<Vec<String>> = rows.iter().map(|r| tokens(r)).collect();
        let num = |t: &str, _: Want| t.parse::<f64>().map_err(|e| e.to_string());
        build(Frame::default(), toks, &num, None)
    }

    fn seg_ids(s: &Sketch) -> Vec<u64> {
        let mut v: Vec<u64> = profile(s, None)
            .unwrap()
            .loops
            .iter()
            .flat_map(|l| l.segments.iter().map(|g| g.1))
            .collect();
        v.sort_unstable();
        v
    }

    #[test]
    fn listed_entities_keep_their_ids_and_what_comes_later_can_never_take_one() {
        let s = built(&["rect 0 0 10 10", "ids l1 0 l2 1 l3 2 l4 3"]).unwrap();
        assert_eq!(seg_ids(&s), [0, 1, 2, 3]);
        // a hole added later: its own id, nowhere near; the rest as they were
        let s = built(&["rect 0 0 10 10", "ids l1 0 l2 1 l3 2 l4 3", "circle 5 5 1"]).unwrap();
        let v = seg_ids(&s);
        assert_eq!(&v[..4], [0, 1, 2, 3]);
        assert!(v[4] >= UNLISTED, "{v:?}");
        // l1's stable id is 4: listing someone else as 4 doesn't meet it
        let s = built(&["rect 0 0 10 10", "ids l2 4", "circle 5 5 1"]).unwrap();
        let v = seg_ids(&s);
        let mut d = v.clone();
        d.dedup();
        assert_eq!(v.len(), d.len(), "{v:?}");
        // without the row, nothing changes
        assert_eq!(seg_ids(&built(&["rect 0 0 10 10"]).unwrap()), [4, 8, 12, 16]);
    }

    fn edge(curve: Curve, range: Interval, start: Point3, end: Point3) -> Source {
        Source::Edge {
            curve,
            range,
            start,
            end,
            closed: false,
        }
    }

    #[test]
    fn an_arc_whose_ends_a_boolean_settled_off_it_still_solves_held() {
        let k = Circle {
            frame: Frame::default(),
            radius: 5.0,
        };
        let e = edge(
            Curve::Circle(k),
            Interval::new(0.0, 1.5),
            Point3::new(5.0 + 4e-6, 0.0, 0.0),
            Point3::new(5.0 * dmath::cos(1.5), 5.0 * dmath::sin(1.5), 0.0),
        );
        let src = |_: &[String]| {
            Ok(vec![Refd {
                sources: vec![(Name::NONE, Src::Model(e.clone()))],
                one: true,
            }])
        };
        let num = |t: &str, _: Want| t.parse::<f64>().map_err(|e| e.to_string());
        let s = build_on(
            Frame::default(),
            vec![vec!["project".into(), "edge".into(), "e".into()]],
            &num,
            &src,
            None,
        )
        .unwrap();
        assert_eq!(freedom(&s).0, 0);
    }

    #[test]
    fn two_entities_cannot_share_an_id() {
        assert!(
            built(&["rect 0 0 10 10", "ids l1 0 l2 0"])
                .unwrap_err()
                .contains("two entities")
        );
        assert!(
            built(&["rect 0 0 10 10", "ids l1 0", "ids l1 1"])
                .unwrap_err()
                .contains("twice")
        );
        assert!(built(&["rect 0 0 10 10", "ids l1 x"]).is_err());
        assert!(built(&["rect 0 0 10 10", "ids l1"]).is_err());
        assert!(built(&["rect 0 0 10 10", "ids l1 99999999999"]).is_err());
    }
}
