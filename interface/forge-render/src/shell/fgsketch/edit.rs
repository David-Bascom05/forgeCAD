//! A Forge sketch block opened in the sketcher, and written back in place. Each statement of the
//! block reads as the sketcher's rows (a fixed point gets its `fix`, a circle's radius and a rect's sides
//! their dimensions), and each number in them remembers where it is written: a param
//! (`SetParam`), a number in the text (put back there), or an expression (driven: refused). At
//! Finish only what changed is written: kept statements keep their bytes and their names, gone
//! ones are taken out, new rows are added as `block` writes them, in one ReplaceFeature.

use super::super::fgtool::ToolError;
use super::super::sketching::rows::{D, Flat, Part, Pt, Ref, Refd, Row, Src, V};
use super::{Out, fmt};
use forge_lang::error::{Literal, Op, ParamKey};
use forge_lang::syntax::{self, Arg, Expr, ExprKind, Span, Stmt, StmtKind, Unit};
use forge_lang::values::Dim;
use forge_sketch::{Entity, EntityId};
use std::collections::{HashMap, HashSet};

/// Where a number of a row is written in the file.
#[derive(Clone, Debug)]
enum How {
    /// A param read by its bare name.
    Param(ParamKey, Dim),
    /// A number, with its unit's suffix.
    Lit(&'static str),
    /// An expression: driven by what it reads.
    Driven(String),
}

#[derive(Clone, Debug)]
struct Slot {
    span: Span,
    how: How,
}

#[derive(Clone, Debug)]
struct St {
    span: Span,
    name: Option<String>,
    /// The Forge names of what its row makes, in the order the row names them.
    ents: Vec<String>,
}

#[derive(Clone, Debug)]
struct RowOf {
    st: usize,
    /// Its statement's own row (the others are its fixes and dimensions).
    main: bool,
    /// Each of its values' place in the file (in `Row::values` order), where it has one.
    slots: Vec<Option<Slot>>,
}

/// A sketch block as the sketcher's rows, with the way back.
#[derive(Clone, Debug)]
pub struct Opened {
    pub id: String,
    pub rows: Vec<Row>,
    /// What the rows' dimension words read: the params the block sees, by bare name.
    pub params: Vec<(String, String)>,
    /// What the sketcher can't hold yet, named with its line: then it opens to be looked at only.
    pub read_only: Option<String>,
    src: String,
    stmts: Vec<St>,
    of: Vec<RowOf>,
    taken: HashSet<String>,
    /// How many params the block has of its own.
    own: usize,
    /// What each `project` row's reference came to as the block solved it, by its words.
    pub sources: HashMap<String, Result<Vec<Refd>, String>>,
}

struct Rd<'a> {
    src: &'a str,
    view: &'a forge_lang::SketchView,
    /// The params a bare name reads in the block: the block's own, then shared ones.
    params: &'a HashMap<String, (ParamKey, Dim)>,
    env: &'a [(String, String)],
    /// Forge's name of an entity (`l1`, `r[2]`) to the row's word.
    words: HashMap<String, String>,
    named: HashMap<char, usize>,
    rows: Vec<Row>,
    of: Vec<RowOf>,
    sources: HashMap<String, Result<Vec<Refd>, String>>,
}

/// A point argument: where it is, and how it's written.
enum PArg {
    Fixed(Option<Slot>, Option<Slot>),
    Free(Option<Slot>, Option<Slot>),
    Word(String),
}

fn arg<'e>(args: &'e [Arg], k: usize, key: &str) -> Option<&'e Expr> {
    args.iter()
        .find(|a| a.name.as_ref().is_some_and(|n| n.0 == key))
        .or_else(|| args.iter().filter(|a| a.name.is_none()).nth(k))
        .map(|a| &a.value)
}

fn call(e: &Expr) -> Option<(&str, &[Arg])> {
    match &e.kind {
        ExprKind::Call(f, args) => match &f.kind {
            ExprKind::Name(n) => Some((n.as_str(), args)),
            _ => None,
        },
        _ => None,
    }
}

/// A solved coordinate as written in a row: short where that's the value, else in full (a
/// fixed point the solver must find where the part has it).
fn exact(v: f64) -> String {
    let s = fmt(v);
    if s.parse::<f64>().is_ok_and(|x| (x - v).abs() < 1e-12) {
        s
    } else {
        format!("{v}")
    }
}

fn text(src: &str, s: Span) -> &str {
    &src[s.lo as usize..s.hi as usize]
}

impl Rd<'_> {
    fn slot(&self, e: &Expr) -> Slot {
        let how = match &e.kind {
            ExprKind::Num(_, u) => How::Lit(match u {
                Unit::Deg => "deg",
                Unit::Mm => "mm",
                Unit::None => "",
            }),
            ExprKind::Name(n) => match self.params.get(n) {
                Some((k, d)) => How::Param(k.clone(), *d),
                None => How::Driven(n.clone()),
            },
            _ => How::Driven(text(self.src, e.span).to_string()),
        };
        Slot { span: e.span, how }
    }

    /// How a dimension's value reads in a row: a param's name, a number, an expression the
    /// sketcher can evaluate; None if it can't (it's shown measured, as a reference).
    fn value(&self, s: &Slot) -> Option<String> {
        let t = text(self.src, s.span);
        match &s.how {
            How::Param(..) => Some(t.to_string()),
            How::Lit(_) => {
                let v = t.trim_end_matches(char::is_alphabetic).parse::<f64>().ok()?;
                Some(fmt(v))
            }
            How::Driven(_) => {
                // what it reads: the params that are numbers
                let env = |name: &str| {
                    let v = &self.env.iter().find(|p| p.0 == name)?.1;
                    v.parse::<f64>().is_ok().then(|| v.clone())
                };
                super::super::sketching::value_of(t, &env).ok()?;
                Some(format!("({t})"))
            }
        }
    }

    fn word(&self, e: &Expr) -> Option<String> {
        match &e.kind {
            ExprKind::Name(n) => self.words.get(n).cloned(),
            ExprKind::Index(b, k) => match (&b.kind, &k.kind) {
                (ExprKind::Name(n), ExprKind::Num(i, _)) => self.words.get(&format!("{n}[{i}]")).cloned(),
                _ => None,
            },
            ExprKind::Attr(b, m) => {
                let w = self.word(b)?;
                let part = match m.as_str() {
                    "a" => "a",
                    "b" => "b",
                    "center" => "c",
                    _ => return None,
                };
                Some(format!("{}.{part}", w.split('.').next()?))
            }
            _ => None,
        }
    }

    fn point(&self, e: &Expr) -> Option<PArg> {
        match &e.kind {
            // a point written as coordinates is Forge's fixed point, but the sketcher holds it
            // where it is, free, so a constraint added (Parallel, Tangent) moves it and is written
            // back in its numbers, never refused as over-constrained
            ExprKind::Tuple(xs) if xs.len() == 2 => {
                Some(PArg::Free(Some(self.slot(&xs[0])), Some(self.slot(&xs[1]))))
            }
            _ => match call(e) {
                Some(("near", a)) if a.len() == 2 => Some(PArg::Free(
                    Some(self.slot(&a[0].value)),
                    Some(self.slot(&a[1].value)),
                )),
                Some(("pt", a)) if a.len() == 2 => Some(PArg::Fixed(
                    Some(self.slot(&a[0].value)),
                    Some(self.slot(&a[1].value)),
                )),
                _ => self.word(e).map(PArg::Word),
            },
        }
    }

    fn at(&self, p: EntityId) -> (f64, f64) {
        let q = self.view.s.point(p);
        (q.x, q.y)
    }

    /// A projected entity as the block solved it: its image, to be held where it is.
    fn laid(&self, id: EntityId) -> Option<Flat> {
        let s = &self.view.s;
        let p = |q: EntityId| s.point(q);
        Some(match s.entity(id)? {
            Entity::Point { .. } => Flat::Point(s.point(id)),
            Entity::Line { a, b } => Flat::Line(p(a), p(b)),
            Entity::Circle { center, .. } => Flat::Circle(p(center), s.radius(id)),
            Entity::Arc {
                center,
                start,
                end,
                ccw,
            } => Flat::Arc {
                center: p(center),
                start: p(start),
                end: p(end),
                ccw,
            },
            Entity::Ellipse {
                center,
                major,
                b,
                ends,
            } => Flat::Ellipse {
                center: p(center),
                major: p(major),
                b: s.params()[b],
                ends: ends.map(|(x, y, ccw)| (p(x), p(y), ccw)),
            },
            Entity::Spline { .. } => {
                let pts: Vec<_> = s.spline_points(id)?.iter().map(|&q| p(q)).collect();
                let ends = s.spline_samples(id, 4);
                let closed =
                    ends.len() > 1 && ends[0].distance(ends[ends.len() - 1]) < 1e-12 && pts.len() > 2;
                Flat::Spline(pts, closed)
            }
        })
    }

    fn next(&mut self, kind: char) -> String {
        let c = self.named.entry(kind).or_default();
        *c += 1;
        format!("{kind}{c}")
    }

    fn push(&mut self, st: usize, main: bool, row: Row, slots: Vec<Option<Slot>>) {
        debug_assert_eq!(row.values().len(), slots.len(), "{row}");
        self.rows.push(row);
        self.of.push(RowOf { st, main, slots });
    }

    /// One statement's rows, or what about it the sketcher can't hold.
    fn stmt(&mut self, s: &Stmt, stmts: &mut Vec<St>) -> Result<(), String> {
        let what = |w: &str| Err(w.to_string());
        let (name, e) = match &s.kind {
            StmtKind::Assign { name, value } => (Some(name.as_str()), value),
            StmtKind::Expr(e) => (None, e),
            StmtKind::For { .. } => return what("a `for` loop"),
            StmtKind::If { .. } => return what("an `if`"),
            StmtKind::Block { .. } => return what("a `path`"),
            _ => return what("a statement of its own"),
        };
        let Some((f, args)) = call(e) else {
            return what("a value bound in the sketch");
        };
        if args.iter().any(|a| matches!(a.value.kind, ExprKind::Comp { .. })) {
            return what("a list comprehension");
        }
        let id = name.and_then(|n| {
            self.view
                .named
                .iter()
                .find(|x| x.0 == n)
                .map(|x| (x.1, x.2.clone()))
        });
        let ent = || -> Result<(EntityId, Vec<EntityId>), String> {
            if name.is_none() {
                return Err(format!("an unnamed `{f}`"));
            }
            id.clone().ok_or_else(|| format!("`{f}` the sketch didn't make"))
        };
        let num = |v: f64| V::of(&fmt(v));
        let unsupported = || Err(format!("a `{f}` reference the sketcher can't name"));
        let st = stmts.len();
        let mut ents = vec![];
        let mut fixes: Vec<Ref> = vec![];
        let mut dims: Vec<(Row, Vec<Option<Slot>>)> = vec![];
        let mut slots: Vec<Option<Slot>> = vec![];
        // a point in the row: its numbers (with their slots) or the sketch's point it shares
        let put = |slots: &mut Vec<Option<Slot>>,
                   fixes: &mut Vec<Ref>,
                   p: Option<PArg>,
                   at: (f64, f64),
                   me: Ref|
         -> Option<Pt> {
            let at = Pt::At(V::of(&exact(at.0)), V::of(&exact(at.1)));
            match p? {
                PArg::Word(w) => Some(Pt::of(&w)),
                PArg::Fixed(x, y) => {
                    slots.extend([x, y]);
                    fixes.push(me);
                    Some(at)
                }
                PArg::Free(x, y) => {
                    slots.extend([x, y]);
                    Some(at)
                }
            }
        };
        let dim = |d: D, on: Vec<Ref>, v: Option<(String, Slot)>| match v {
            Some((v, s)) => (Row::Dim(d, on, Some(V::of(&v))), vec![Some(s)]),
            None => (Row::Dim(d, on, None), vec![]),
        };
        let row = match f {
            "pt" | "near" => {
                let (e, _) = ent()?;
                let n = self.next('p');
                let p = PArg::Free(
                    Some(self.slot(&args[0].value)),
                    args.get(1).map(|a| self.slot(&a.value)),
                );
                let p = if f == "pt" {
                    match p {
                        PArg::Free(x, y) => PArg::Fixed(x, y),
                        p => p,
                    }
                } else {
                    p
                };
                let p = put(&mut slots, &mut fixes, Some(p), self.at(e), Ref::to(&n)).expect("a point");
                ents.push((n, name.unwrap_or("").to_string()));
                Row::Pt(p)
            }
            "line" => {
                let (e, _) = ent()?;
                let Some(Entity::Line { a, b }) = self.view.s.entity(e) else {
                    return unsupported();
                };
                let n = self.next('l');
                let pa = arg(args, 0, "a").and_then(|x| self.point(x));
                let pb = arg(args, 1, "b").and_then(|x| self.point(x));
                let (Some(pa), Some(pb)) = (
                    put(&mut slots, &mut fixes, pa, self.at(a), Ref::a(&n)),
                    put(&mut slots, &mut fixes, pb, self.at(b), Ref::b(&n)),
                ) else {
                    return unsupported();
                };
                ents.push((n, name.unwrap_or("").to_string()));
                let row = Row::Line(pa, pb);
                if arg(args, 9, "construction").is_some_and(|c| matches!(c.kind, ExprKind::Bool(true))) {
                    row.cons()
                } else {
                    row
                }
            }
            "circle" => {
                let (e, _) = ent()?;
                let Some(Entity::Circle { center, .. }) = self.view.s.entity(e) else {
                    return unsupported();
                };
                let n = self.next('c');
                let pc = arg(args, 0, "center").and_then(|x| self.point(x));
                let Some(pc) = put(&mut slots, &mut fixes, pc, self.at(center), Ref::center(&n)) else {
                    return unsupported();
                };
                slots.push(None);
                if let Some(r) = arg(args, 1, "r") {
                    let s = self.slot(r);
                    dims.push(dim(D::Rad, vec![Ref::to(&n)], self.value(&s).map(|v| (v, s))));
                }
                ents.push((n, name.unwrap_or("").to_string()));
                Row::Circle(pc, num(self.view.s.radius(e)))
            }
            "arc" => {
                let (e, _) = ent()?;
                let Some(Entity::Arc {
                    center,
                    start,
                    end,
                    ccw,
                }) = self.view.s.entity(e)
                else {
                    return unsupported();
                };
                let n = self.next('a');
                let mut pts = vec![];
                for (key, p, me) in [
                    ("center", center, Ref::center(&n)),
                    ("from", start, Ref::a(&n)),
                    ("to", end, Ref::b(&n)),
                ] {
                    let pp = arg(args, 9, key).and_then(|x| self.point(x));
                    let Some(q) = put(&mut slots, &mut fixes, pp, self.at(p), me) else {
                        return unsupported();
                    };
                    pts.push(q);
                }
                ents.push((n, name.unwrap_or("").to_string()));
                let [c, a, b]: [Pt; 3] = pts.try_into().expect("three");
                Row::Arc(c, a, b, !ccw)
            }
            "rect" => {
                let (_, segs) = ent()?;
                let lines: Vec<(EntityId, EntityId)> = segs
                    .iter()
                    .filter_map(|s| match self.view.s.entity(*s) {
                        Some(Entity::Line { a, b }) => Some((a, b)),
                        _ => None,
                    })
                    .collect();
                if lines.len() != 4 {
                    return unsupported();
                }
                let (p0, p1, p2) = (self.at(lines[0].0), self.at(lines[0].1), self.at(lines[1].1));
                let ns: Vec<String> = (0..4).map(|_| self.next('l')).collect();
                let corner = arg(args, 9, "corner");
                let size = arg(args, 9, "size");
                let pair = |e: Option<&Expr>| match e.map(|e| &e.kind) {
                    Some(ExprKind::Tuple(xs)) if xs.len() == 2 => {
                        Some((self.slot(&xs[0]), self.slot(&xs[1])))
                    }
                    _ => None,
                };
                let (cx, cy) = pair(corner).map_or((None, None), |(x, y)| (Some(x), Some(y)));
                slots.extend([cx, cy, None, None]);
                fixes.push(Ref::a(&ns[0]));
                match pair(size).filter(|_| corner.is_some()) {
                    Some((sw, sh)) => {
                        for (d, l, s) in [(D::HDist, &ns[0], sw), (D::VDist, &ns[1], sh)] {
                            let on = vec![Ref::a(l), Ref::b(l)];
                            dims.push(dim(d, on, self.value(&s).map(|v| (v, s))));
                        }
                    }
                    // by its centre, or sized otherwise: held where it is
                    None => fixes.push(Ref::a(&ns[2])),
                }
                let base = name.unwrap_or("");
                for (k, l) in ns.into_iter().enumerate() {
                    ents.push((l, format!("{base}[{k}]")));
                }
                Row::Rect(
                    V::of(&exact(p0.0)),
                    V::of(&exact(p0.1)),
                    num(p1.0 - p0.0),
                    num(p2.1 - p1.1),
                )
            }
            "poly" | "spline" => {
                let (e, segs) = ent()?;
                let Some(ExprKind::List(ps)) = arg(args, 0, "points").map(|x| &x.kind) else {
                    return what("a list of points built from other values");
                };
                let cv = arg(args, 9, "cv").is_some_and(|c| matches!(c.kind, ExprKind::Bool(true)));
                let given = |b: bool| {
                    arg(args, 9, "closed").is_some_and(|c| matches!(c.kind, ExprKind::Bool(x) if x == b))
                };
                // a through-spline is open unless `closed: true`; a polygon and control points
                // closed unless `closed: false`
                let closed = if f == "spline" && !cv {
                    given(true)
                } else {
                    !given(false)
                };
                let at: Vec<EntityId> = if f == "poly" {
                    let ls: Vec<(EntityId, EntityId)> = segs
                        .iter()
                        .filter_map(|s| match self.view.s.entity(*s) {
                            Some(Entity::Line { a, b }) => Some((a, b)),
                            _ => None,
                        })
                        .collect();
                    let mut v: Vec<EntityId> = ls.iter().map(|l| l.0).collect();
                    if !closed && let Some(l) = ls.last() {
                        v.push(l.1);
                    }
                    v
                } else {
                    self.view
                        .s
                        .spline_points(e)
                        .map(<[EntityId]>::to_vec)
                        .unwrap_or_default()
                };
                if at.len() != ps.len() {
                    return unsupported();
                }
                let first = self.named.get(&'l').copied().unwrap_or(0) + 1;
                let sides = ps.len() - usize::from(!closed);
                let sn = if f == "spline" { Some(self.next('s')) } else { None };
                let degree = 3.min(ps.len().saturating_sub(1 + usize::from(closed))).max(1);
                if cv {
                    slots.push(None);
                }
                let mut pts = vec![];
                for (k, p) in ps.iter().enumerate() {
                    let me = match &sn {
                        Some(s) => Ref {
                            base: s.clone(),
                            part: Part::K(k),
                        },
                        None if k < sides => Ref::a(&format!("l{}", first + k)),
                        None => Ref::b(&format!("l{}", first + k - 1)),
                    };
                    let pp = self.point(p);
                    let Some(q) = put(&mut slots, &mut fixes, pp, self.at(at[k]), me) else {
                        return unsupported();
                    };
                    pts.push(q);
                }
                let base = name.unwrap_or("");
                match &sn {
                    Some(s) => ents.push((s.clone(), base.to_string())),
                    None => {
                        for k in 0..sides {
                            let l = self.next('l');
                            ents.push((l, format!("{base}[{k}]")));
                        }
                    }
                }
                match (f, cv) {
                    ("poly", _) => Row::Poly(pts, !closed),
                    (_, true) => Row::BSpline(V::N(degree as f64), pts, !closed),
                    _ => Row::Spline(pts, !closed),
                }
            }
            "ellipse" => {
                let (e, _) = ent()?;
                let Some(Entity::Ellipse {
                    center,
                    major,
                    b,
                    ends: None,
                }) = self.view.s.entity(e)
                else {
                    return unsupported();
                };
                let n = self.next('e');
                let mut pts = vec![];
                for (key, p, me) in [
                    ("center", center, Ref::center(&n)),
                    (
                        "major",
                        major,
                        Ref {
                            base: n.clone(),
                            part: Part::M,
                        },
                    ),
                ] {
                    let pp = arg(args, 9, key).and_then(|x| self.point(x));
                    let Some(q) = put(&mut slots, &mut fixes, pp, self.at(p), me) else {
                        return unsupported();
                    };
                    pts.push(q);
                }
                slots.push(None);
                if let Some(bx) = arg(args, 9, "b") {
                    let s = self.slot(bx);
                    dims.push(dim(D::Rad, vec![Ref::to(&n)], self.value(&s).map(|v| (v, s))));
                }
                ents.push((n, name.unwrap_or("").to_string()));
                let [c, m]: [Pt; 2] = pts.try_into().expect("two");
                Row::Ellipse(c, m, num(self.view.s.params()[b]))
            }
            "tangent" | "coincident" | "horizontal" | "vertical" | "parallel" | "perpendicular" | "equal"
            | "midpoint" | "concentric" | "collinear" | "symmetric" | "fix" | "distance" | "radius"
            | "angle"
                if name.is_none() =>
            {
                let valued = matches!(f, "distance" | "radius" | "angle");
                let n = args.len() - usize::from(valued);
                let mut ws = vec![];
                for a in &args[..n] {
                    match self.word(&a.value) {
                        Some(x) => ws.push(Ref::name(&x)),
                        None => return unsupported(),
                    }
                }
                if valued {
                    let d = match f {
                        "distance" => D::Dist,
                        "radius" => D::Rad,
                        _ => D::Ang,
                    };
                    let s = self.slot(&args[n].value);
                    let (row, sl) = dim(d, ws, self.value(&s).map(|v| (v, s)));
                    slots = sl;
                    row
                } else {
                    let two = |ws: &[Ref]| match ws {
                        [a, b] => Some((a.clone(), b.clone())),
                        _ => None,
                    };
                    let row = match f {
                        "horizontal" => Some(Row::Hor(ws)),
                        "vertical" => Some(Row::Ver(ws)),
                        "fix" => Some(Row::Fix(ws)),
                        "equal" => Some(Row::Eq(ws)),
                        "symmetric" => match &ws[..] {
                            [a, b, c] => Some(Row::Sym(a.clone(), b.clone(), c.clone())),
                            _ => None,
                        },
                        "tangent" => two(&ws).map(|(a, b)| Row::Tan(a, b)),
                        "coincident" => two(&ws).map(|(a, b)| Row::Co(a, b)),
                        "parallel" => two(&ws).map(|(a, b)| Row::Par(a, b)),
                        "perpendicular" => two(&ws).map(|(a, b)| Row::Perp(a, b)),
                        "midpoint" => two(&ws).map(|(a, b)| Row::Mid(a, b)),
                        "concentric" => two(&ws).map(|(a, b)| Row::Conc(a, b)),
                        _ => two(&ws).map(|(a, b)| Row::Colin(a, b)),
                    };
                    let Some(row) = row else {
                        return unsupported();
                    };
                    row
                }
            }
            "project" => {
                let (e, segs) = ent()?;
                let of = arg(args, 0, "of").ok_or_else(|| "a `project` of nothing".to_string())?;
                let of_text = text(self.src, of.span).to_string();
                let ids = if segs.is_empty() { vec![e] } else { segs };
                let one = ids.len() == 1;
                let laid = ids
                    .iter()
                    .map(|&id| self.laid(id))
                    .collect::<Option<Vec<Flat>>>()
                    .ok_or_else(|| "a `project` the sketcher can't hold".to_string())?;
                let sources = laid
                    .into_iter()
                    .enumerate()
                    .map(|(k, f)| (forge_topo::Name(k as u64 + 1), Src::Laid(f)))
                    .collect();
                self.sources
                    .insert(of_text.clone(), Ok(vec![Refd { sources, one }]));
                let approx = match arg(args, 1, "approx") {
                    Some(a) => {
                        let s = self.slot(a);
                        let v = self
                            .value(&s)
                            .ok_or_else(|| "an `approx` the sketcher can't read".to_string())?;
                        slots.push(Some(s));
                        Some(V::of(&v))
                    }
                    None => None,
                };
                if one {
                    let n = self.next('j');
                    ents.push((n, name.unwrap_or("").to_string()));
                }
                Row::Project {
                    of: of_text,
                    many: !one,
                    approx,
                }
            }
            f => return Err(format!("`{f}`")),
        };
        stmts.push(St {
            span: s.span,
            name: name.map(str::to_string),
            ents: ents.iter().map(|e| e.1.clone()).collect(),
        });
        for (named, fg) in ents {
            // a loop's sides are named by index; the loop itself has no single word
            self.words.insert(fg, named);
        }
        self.push(st, true, row, slots);
        for x in fixes {
            self.push(st, false, Row::Fix(vec![x]), vec![]);
        }
        for (d, sl) in dims {
            self.push(st, false, d, sl);
        }
        Ok(())
    }
}

/// The component a sketch id is inside (`peg.s`: `peg`, its statements, `s`), if it is.
fn component_of<'p>(prog: &'p [Stmt], id: &'p str) -> Option<(&'p str, &'p [Stmt], &'p str)> {
    let (c, rest) = id.split_once('.')?;
    prog.iter().find_map(|s| match &s.kind {
        StmtKind::Block {
            kind: syntax::BlockKind::Component,
            name,
            body,
            ..
        } if name == c => Some((c, body.as_slice(), rest)),
        _ => None,
    })
}

/// The program's sketch statement `id` (in a component, `peg.s`), and the first of the params
/// written just above it.
fn find<'p>(prog: &'p [Stmt], id: &'p str) -> Option<(&'p Stmt, &'p Stmt)> {
    let (prog, id) = match component_of(prog, id) {
        Some((_, body, rest)) => (body, rest),
        None => (prog, id),
    };
    let k = prog.iter().position(
        |s| matches!(&s.kind, StmtKind::Block { kind: syntax::BlockKind::Sketch, name, .. } if name == id),
    )?;
    let mut j = k;
    while j > 0
        && matches!(&prog[j - 1].kind, StmtKind::Param { owner, .. } if owner.first().is_some_and(|o| o.name == id))
    {
        j -= 1;
    }
    Some((&prog[j], &prog[k]))
}

/// A sketch's own name: `s` of `peg.s`.
fn own_name(prog: &[Stmt], id: &str) -> String {
    component_of(prog, id).map_or(id, |x| x.2).to_string()
}

/// Opens sketch `id` of `src` (fonts and used files from `dir`).
pub fn open(src: &str, id: &str, dir: Option<std::path::PathBuf>) -> Result<Opened, ToolError> {
    let no = |why: String| ToolError::NoForgeForm {
        tool: "Edit Sketch",
        why,
    };
    let prog = syntax::parse(src).map_err(|e| no(e.text()))?;
    let (_, block) = find(&prog, id).ok_or_else(|| no(format!("{id} is not a sketch of this part")))?;
    let comp = component_of(&prog, id).map(|x| (x.0.to_string(), x.1));
    // what it projects is read from the part, so the part is built for it
    let projects = text(src, block.span).contains("project(");
    let opts = forge_lang::Options {
        build: projects,
        dir: dir.clone(),
        ..forge_lang::Options::default()
    };
    let view = match forge_lang::sketch_view(src, id, opts.clone()) {
        Some(v) => v,
        None => forge_lang::sketch_view(
            src,
            id,
            forge_lang::Options {
                build: true,
                ..opts.clone()
            },
        )
        .ok_or_else(|| {
            no(format!(
                "{id} didn't evaluate: fix what the issues list says first"
            ))
        })?,
    };
    let (run, _) = forge_lang::run(src, opts, Default::default());
    let mut params: HashMap<String, (ParamKey, Dim)> = HashMap::new();
    let mut env = vec![];
    // a component's sketch: its params by their defaults (read as values: a call sets them), its
    // own params under the component's name
    let owner: Vec<&str> = match &comp {
        Some((c, _)) => vec![c.as_str(), id.split_once('.').map_or(id, |x| x.1)],
        None => vec![id],
    };
    let own = |p: &ParamKey| {
        p.owner.len() == owner.len()
            && p.owner
                .iter()
                .zip(&owner)
                .all(|(s, o)| s.name == *o && s.index.is_empty())
    };
    if let Some((c, _)) = &comp
        && let Some(StmtKind::Block { args, .. }) = prog.iter().find_map(|s| match &s.kind {
            k @ StmtKind::Block { name, .. } if name == c => Some(k),
            _ => None,
        })
    {
        for a in args {
            if let Some((n, _)) = &a.name {
                let t = src[a.value.span.lo as usize..a.value.span.hi as usize]
                    .trim()
                    .to_string();
                env.push((n.clone(), t));
            }
        }
    }
    let locals: Vec<&forge_lang::eval::ParamInfo> = run
        .locals
        .iter()
        .chain(run.components.iter().flat_map(|c| &c.locals))
        .collect();
    // shared first, so the block's own of the same name wins
    for p in run
        .shared
        .iter()
        .chain(locals.iter().copied().filter(|p| own(&p.key)))
    {
        if let forge_lang::eval::Value::Num(n) = &p.value {
            let v = if n.dim == Dim::Angle {
                n.v.to_degrees()
            } else {
                n.v
            };
            params.insert(p.key.name.clone(), (p.key.clone(), n.dim));
            env.retain(|e: &(String, String)| e.0 != p.key.name);
            env.push((p.key.name.clone(), fmt(v)));
        }
    }
    let StmtKind::Block { body, .. } = &block.kind else {
        unreachable!("a sketch block")
    };
    let mut rd = Rd {
        src,
        view: &view,
        params: &params,
        env: &env,
        words: HashMap::new(),
        named: HashMap::new(),
        rows: vec![],
        of: vec![],
        sources: HashMap::new(),
    };
    let mut stmts = vec![];
    let mut read_only = None;
    for s in body {
        if let Err(what) = rd.stmt(s, &mut stmts)
            && read_only.is_none()
        {
            let line = s.span.lines(src).0;
            read_only = Some(format!(
                "{what} (line {line}) has no form in the sketcher yet; edit it in the source"
            ));
        }
    }
    let (rows, of, sources) = (rd.rows, rd.of, rd.sources);
    let mut taken: HashSet<String> = forge_lang::edit::bound(&prog).into_iter().collect();
    taken.extend(forge_lang::edit::bound(body));
    taken.extend(params.keys().cloned());
    let mine: Vec<String> = locals
        .iter()
        .filter(|p| own(&p.key))
        .map(|p| p.key.name.clone())
        .collect();
    if let Some((_, body)) = &comp {
        taken.extend(forge_lang::edit::bound(body));
    }
    taken.extend(mine.iter().cloned());
    Ok(Opened {
        id: id.to_string(),
        rows,
        params: env,
        read_only,
        src: src.to_string(),
        stmts,
        of,
        taken,
        own: mine.len(),
        sources,
    })
}

/// A row with its entities named as Forge names them, numbers as they are: what's compared.
fn canon(row: &Row, names: &HashMap<String, String>) -> Row {
    let mut r = row.clone();
    for x in r.refs_mut() {
        if let Some(n) = names.get(&x.base) {
            x.base = n.clone();
        }
    }
    r
}

/// A row with its values left out: its shape.
fn shape(row: &Row) -> Row {
    let mut r = row.clone();
    for v in r.values_mut() {
        *v = V::X(String::new());
    }
    r
}

impl Opened {
    /// The row name to Forge name of every entity the rows make, row `k < kept.len()` being
    /// opened row `kept[k]`; new rows' entities have none yet.
    fn names(&self, rows: &[Row], kept: &[usize]) -> (HashMap<String, String>, Vec<Vec<String>>) {
        let mut out = HashMap::new();
        let mut per = vec![];
        let mut named: HashMap<char, usize> = HashMap::new();
        for (k, r) in rows.iter().enumerate() {
            let of = kept.get(k).map(|&o| &self.of[o]);
            let ents: Vec<String> = match of {
                Some(o) if o.main => self.stmts[o.st].ents.clone(),
                _ => vec![],
            };
            for (i, kind) in r.made().into_iter().enumerate() {
                let c = named.entry(kind).or_default();
                *c += 1;
                if let Some(n) = ents.get(i).filter(|n| !n.is_empty()) {
                    out.insert(format!("{kind}{c}"), n.clone());
                }
            }
            per.push(ents);
        }
        (out, per)
    }

    /// The edit Finish makes: nothing when nothing changed; else the block put back in its place
    /// (kept statements as they were written, changed numbers where they're written, gone ones
    /// out, new rows added) and the params set. `kept[k]` is the opened row that row k is.
    pub fn write_back(&self, rows: &[Row], kept: &[usize]) -> Result<Vec<Op>, ToolError> {
        let opened = self
            .names(&self.rows, &(0..self.rows.len()).collect::<Vec<_>>())
            .0;
        let (now, per) = self.names(rows, kept);
        let mut sets: Vec<Op> = vec![];
        // byte span to its new text
        let mut splices: Vec<(Span, String)> = vec![];
        let mut rewrite: Vec<(usize, usize)> = vec![];
        let src = &self.src;
        for (k, r) in rows.iter().enumerate().take(kept.len()) {
            let o = kept[k];
            let (a, b) = (canon(&self.rows[o], &opened), canon(r, &now));
            if a == b {
                continue;
            }
            let of = &self.of[o];
            // the same row, only values changed: each is put back where it's written
            let (va, vb) = (a.values(), b.values());
            let same = shape(&a) == shape(&b)
                && va
                    .iter()
                    .zip(&vb)
                    .enumerate()
                    .all(|(i, (x, y))| x == y || of.slots.get(i).is_some_and(Option::is_some));
            if !same {
                if of.main {
                    rewrite.push((k, of.st));
                    continue;
                }
                return Err(ToolError::NoForgeForm {
                    tool: "sketch",
                    why: format!("the change to `{r}` has no Forge form here; write it in the source"),
                });
            }
            for (i, (x, y)) in va.iter().zip(&vb).enumerate() {
                if x == y {
                    continue;
                }
                let s = of.slots[i].as_ref().expect("a slot");
                let v = match y {
                    V::N(v) => *v,
                    V::X(y) => {
                        let t = y.strip_prefix('(').and_then(|t| t.strip_suffix(')')).unwrap_or(y);
                        splices.push((s.span, t.to_string()));
                        continue;
                    }
                };
                match &s.how {
                    How::Param(key, dim) => {
                        sets.retain(|op| !matches!(op, Op::SetParam { param, .. } if param == key));
                        sets.push(Op::SetParam {
                            param: key.clone(),
                            value: Literal { value: v, dim: *dim },
                        });
                    }
                    How::Lit(unit) => splices.push((s.span, format!("{}{unit}", fmt(v)))),
                    How::Driven(e) => {
                        return Err(ToolError::Driven {
                            value: fmt(v),
                            expr: e.clone(),
                        });
                    }
                }
            }
        }
        let gone: Vec<usize> = (0..self.stmts.len())
            .filter(|st| {
                !kept
                    .iter()
                    .take(rows.len())
                    .any(|&o| self.of[o].st == *st && self.of[o].main)
            })
            .collect();
        let from = kept.len().min(rows.len());
        let parsed = syntax::parse(src).map_err(|e| ToolError::NoForgeForm {
            tool: "sketch",
            why: e.text(),
        })?;
        let mut o = Out::new(&own_name(&parsed, &self.id), self.taken.clone());
        o.base = self.own;
        // names the new rows read, and what rewritten ones keep
        o.rows_from(rows, from, &per)?;
        let fresh: Vec<String> = std::mem::take(&mut o.rows);
        for &(k, st) in &rewrite {
            o.keep = self.stmts[st].name.iter().cloned().collect();
            o.row(k, &rows[k], &HashMap::new(), &HashMap::new())?;
            let t = std::mem::take(&mut o.rows).join("\n  ");
            splices.retain(|s| !(self.stmts[st].span.lo <= s.0.lo && s.0.hi <= self.stmts[st].span.hi));
            splices.push((self.stmts[st].span, t));
        }
        for &st in &gone {
            let s = self.stmts[st].span;
            splices.retain(|x| !(s.lo <= x.0.lo && x.0.hi <= s.hi));
            // the whole line: its indent and its line break
            let lo = src[..s.lo as usize].rfind('\n').map_or(0, |n| n + 1);
            let hi = src[s.hi as usize..]
                .find('\n')
                .map_or(src.len(), |n| s.hi as usize + n + 1);
            splices.push((Span::new(lo, hi), String::new()));
        }
        if splices.is_empty() && fresh.is_empty() && o.params.is_empty() && o.binds.is_empty() {
            return Ok(sets);
        }
        let prog = syntax::parse(src).map_err(|e| ToolError::NoForgeForm {
            tool: "sketch",
            why: e.text(),
        })?;
        let (first, block) = find(&prog, &self.id).expect("opened from this text");
        let (lo, hi) = (first.span.lo as usize, block.span.hi as usize);
        // new rows go in above the block's closing brace, new params above the block
        let close = src[..hi - 1].rfind('\n').map_or(hi - 1, |n| n + 1);
        let inline = !src[close..hi - 1].trim().is_empty();
        let mut rows_text: String = fresh.iter().map(|r| format!("  {r}\n")).collect();
        let close = if inline && !fresh.is_empty() {
            rows_text.insert(0, '\n');
            hi - 1
        } else {
            close
        };
        splices.push((Span::new(close, close), rows_text));
        splices.push((
            Span::new(block.span.lo as usize, block.span.lo as usize),
            o.params.concat() + &o.binds.concat(),
        ));
        splices.sort_by_key(|s| (s.0.lo, s.0.hi));
        let mut text = String::new();
        let mut at = lo;
        for (s, t) in &splices {
            text.push_str(&src[at..s.lo as usize]);
            text.push_str(t);
            at = s.hi as usize;
        }
        text.push_str(&src[at..hi]);
        let mut ops = vec![Op::ReplaceFeature {
            id: self.id.clone(),
            text,
        }];
        ops.extend(sets);
        Ok(ops)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::sketching::rows::dsl::*;
    use crate::shell::sketching::{SketchFor, SketchMode};

    fn mode(o: &Opened) -> SketchMode {
        let mut m = SketchMode::new(
            SketchFor::Forge(o.id.clone()),
            "xy".into(),
            forge_math::Frame::default(),
            o.rows.clone(),
        );
        m.set_params(o.params.clone());
        m
    }

    fn apply(src: &str, ops: &[Op]) -> Result<String, forge_lang::error::Error> {
        forge_lang::edit::apply_all_checked(src, ops)
    }

    fn demo() -> String {
        forge_dirs::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/demo/part_tools.fg"
        ))
        .unwrap()
    }

    fn files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        for e in forge_dirs::fs::read_dir(dir).unwrap().flatten() {
            let p = e.path();
            if forge_dirs::fs::is_dir(&p) {
                files(&p, out);
            } else if p.extension().is_some_and(|x| x == "fg") {
                out.push(p);
            }
        }
    }

    #[test]
    fn a_components_sketch_reads_its_params_and_writes_inside_its_block() {
        let src = "shared plen = 30\ncomponent peg(d: 6, h: 30) {\n  sketch s(on: xy) {\n    c = circle((0, 0), d / 2)\n  }\n  shank = extrude(s, h)\n}\np = part(peg(h: plen))\n";
        let o = open(src, "peg.s", None).unwrap();
        assert!(o.read_only.is_none(), "{:?}", o.read_only);
        let m = mode(&o);
        assert!(m.error.is_none(), "{:?}", m.error);
        assert!(o.write_back(&m.rows, m.kept()).unwrap().is_empty());
    }

    #[test]
    fn every_example_sketch_opens_solves_and_finishes_unchanged_to_the_byte() {
        let mut paths = vec![];
        files(
            std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples")),
            &mut paths,
        );
        paths.sort();
        let (mut editable, mut looked, mut sketches) = (0, 0, 0);
        let mut bad = vec![];
        for p in &paths {
            let src = forge_dirs::fs::read_to_string(p).unwrap();
            let Ok(prog) = syntax::parse(&src) else { continue };
            // a component's sketches too, by their ids from outside (`peg.s`)
            let mut ids: Vec<String> = vec![];
            for s in &prog {
                match &s.kind {
                    StmtKind::Block {
                        kind: syntax::BlockKind::Sketch,
                        name,
                        ..
                    } => ids.push(name.clone()),
                    StmtKind::Block {
                        kind: syntax::BlockKind::Component,
                        name: c,
                        body,
                        ..
                    } => ids.extend(body.iter().filter_map(|b| match &b.kind {
                        StmtKind::Block {
                            kind: syntax::BlockKind::Sketch,
                            name,
                            ..
                        } => Some(format!("{c}.{name}")),
                        _ => None,
                    })),
                    _ => {}
                }
            }
            for name in &ids {
                sketches += 1;
                let o = match open(&src, name, p.parent().map(|d| d.to_path_buf())) {
                    Ok(o) => o,
                    Err(e) => {
                        bad.push(format!("{}: {name}: {}", p.display(), e.text()));
                        continue;
                    }
                };
                let m = mode(&o);
                if let Some(e) = &m.error {
                    bad.push(format!(
                        "{}: {name}: {e}\n{}",
                        p.display(),
                        o.rows
                            .iter()
                            .map(|r| r.to_string())
                            .collect::<Vec<_>>()
                            .join("\n")
                    ));
                }
                match o.write_back(&m.rows, m.kept()) {
                    Ok(ops) if ops.is_empty() => {}
                    got => bad.push(format!("{}: {name}: finished unchanged as {got:?}", p.display())),
                }
                if o.read_only.is_some() {
                    looked += 1;
                } else {
                    editable += 1;
                }
            }
        }
        assert!(bad.is_empty(), "{}", bad.join("\n\n"));
        // most of them are the sketcher's to edit; what isn't opens to be looked at
        assert!(
            sketches > 300 && editable * 4 > sketches * 3,
            "{editable} editable, {looked} read-only of {sketches}"
        );
    }

    #[test]
    fn a_moved_point_and_a_changed_dimension_set_their_params() {
        let src = demo();
        let o = open(&src, "sk2", None).unwrap();
        assert_eq!(
            o.rows,
            [
                Row::Circle(at("0", "0"), v("12")),
                Row::Dim(D::Rad, vec![rf("c1")], Some(v("r3"))),
                Row::Pt(at("20", "0"))
            ]
        );
        let mut rows = o.rows.clone();
        rows[2] = Row::Pt(at("25", "-3"));
        rows[1] = Row::Dim(D::Rad, vec![rf("c1")], Some(v("14")));
        let kept: Vec<usize> = (0..rows.len()).collect();
        let ops = o.write_back(&rows, &kept).unwrap();
        let text: Vec<String> = ops.iter().map(|op| format!("{:?}", op.json())).collect();
        assert_eq!(ops.len(), 3, "{text:?}");
        assert!(ops.iter().all(|op| matches!(op, Op::SetParam { .. })), "{text:?}");
        let after = apply(&src, &ops).unwrap();
        assert!(
            after.contains("param sk2.x4 = 25\n") && after.contains("param sk2.y5 = -3\n"),
            "{after}"
        );
        assert!(after.contains("param sk2.r3 = 14\n"), "{after}");
        // the rect's sides are its size params
        let o = open(&src, "sk1", None).unwrap();
        let k = o
            .rows
            .iter()
            .position(|r| matches!(r, Row::Dim(D::HDist, ..)))
            .unwrap();
        assert_eq!(
            o.rows[k],
            Row::Dim(D::HDist, vec![rf("l1.a"), rf("l1.b")], Some(v("w3")))
        );
        let mut rows = o.rows.clone();
        rows[k] = Row::Dim(D::HDist, vec![rf("l1.a"), rf("l1.b")], Some(v("90")));
        let ops = o.write_back(&rows, &(0..rows.len()).collect::<Vec<_>>()).unwrap();
        let after = apply(&src, &ops).unwrap();
        assert!(after.contains("param sk1.w3 = 90\n"), "{after}");
        assert_eq!(after.len(), src.len(), "only the value changed");
    }

    #[test]
    fn a_line_and_its_constraint_are_added_with_fresh_names_and_the_rest_kept() {
        let src = demo();
        let o = open(&src, "sk2", None).unwrap();
        let mut m = mode(&o);
        // a construction line: the sketch's profiles stay as the part reads them
        assert!(m.add(Row::Line(at("30", "5"), at("40", "5")).cons()));
        assert!(m.add(Row::Hor(vec![rf("l1")])));
        assert!(m.add(Row::Co(rf("l1.a"), rf("p2"))) || m.error.is_some());
        let rows: Vec<Row> = m
            .rows
            .iter()
            .filter(|r| !matches!(r, Row::Co(..)))
            .cloned()
            .collect();
        let ops = o.write_back(&rows, m.kept()).unwrap();
        let [Op::ReplaceFeature { id, text }] = &ops[..] else {
            panic!("{ops:?}")
        };
        assert_eq!(id, "sk2");
        assert!(
            text.contains("  c1 = circle(near(x1, y2), r3)\n  p1 = near(x4, y5)\n"),
            "{text}"
        );
        assert!(
            text.contains("  l1 = line(near(x6, y7), near(x8, y9), construction: true)\n  horizontal(l1)\n}"),
            "{text}"
        );
        assert!(text.contains("param sk2.x6 = 30\n"), "{text}");
        let after = apply(&src, &ops).unwrap();
        assert!(after.starts_with(&src[..src.find("param sk2.x1").unwrap()]));
        assert!(after.ends_with(&src[src.find("param extrude2.d").unwrap()..]));
    }

    #[test]
    fn deleting_takes_its_statement_out_and_what_a_later_feature_reads_is_refused_typed() {
        let src = "param sk.x1 = 0\nparam sk.w = 20\nsketch sk(on: xy) {\n  r1 = rect(corner: (x1, 0), size: (w, 10))\n  p1 = pt(5, 5)\n  p2 = near(8, 2)\n}\nbase = extrude(sk.r1, 2)\nhole1 = hole(at: sk.p2, d: 1, depth: thru)\n";
        let o = open(src, "sk", None).unwrap();
        let mut m = mode(&o);
        m.choose_at((5.0, 5.0), 0.5);
        assert!(m.delete_chosen());
        let ops = o.write_back(&m.rows, m.kept()).unwrap();
        let after = apply(src, &ops).unwrap();
        assert_eq!(after, src.replace("  p1 = pt(5, 5)\n", ""));
        // a later feature reads p2: taking it out is refused, typed, with the fix
        let mut m = mode(&o);
        m.choose_at((8.0, 2.0), 0.5);
        assert!(m.delete_chosen());
        let ops = o.write_back(&m.rows, m.kept()).unwrap();
        let e = apply(src, &ops).unwrap_err();
        assert_eq!(e.kind(), "EntityStillRead", "{}", e.text());
        assert!(e.text().contains("hole1 reads sk.p2"), "{}", e.text());
        assert!(e.fixes().contains(&Op::DeleteFeature { id: "hole1".into() }));
    }

    #[test]
    fn a_driven_value_is_refused_and_a_loop_opens_to_be_looked_at() {
        let src = "shared d = 10\nsketch sk(on: xy) {\n  c = circle((0, 0), d / 2)\n  for i in 0..2 {\n    q = pt(i * 3, 1)\n  }\n}\nbase = extrude(sk.c, 2)\n";
        let o = open(src, "sk", None).unwrap();
        assert!(
            o.read_only
                .as_ref()
                .is_some_and(|r| r.contains("`for` loop (line 4)")),
            "{:?}",
            o.read_only
        );
        // its centre written as coordinates is held free: no `fix` row
        assert_eq!(o.rows[1], Row::Dim(D::Rad, vec![rf("c1")], Some(v("(d / 2)"))));
        let m = mode(&o);
        assert!(m.error.is_none(), "{:?}", m.error);
        let mut rows = o.rows.clone();
        rows[1] = Row::Dim(D::Rad, vec![rf("c1")], Some(v("6")));
        let e = o.write_back(&rows, &[0, 1]).unwrap_err();
        assert_eq!(e.kind(), "Driven", "{}", e.text());
        // a typed expression is written as it is
        rows[1] = Row::Dim(D::Rad, vec![rf("c1")], Some(v("(d / 4)")));
        let ops = o.write_back(&rows, &[0, 1]).unwrap();
        let after = apply(src, &ops).unwrap();
        assert!(after.contains("circle((0, 0), d / 4)"), "{after}");
    }
}
