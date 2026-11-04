//! A Forge sketch block opened in the sketcher, and written back in place. Each statement of the
//! block reads as .fc rows (a fixed point gets its `fix`, a circle's radius and a rect's sides
//! their dimensions), and each number in them remembers where it is written: a param
//! (`SetParam`), a number in the text (put back there), or an expression (driven: refused). At
//! Finish only what changed is written: kept statements keep their bytes and their names, gone
//! ones are taken out, new rows are added as `block` writes them, in one ReplaceFeature.

use super::super::fgtool::ToolError;
use super::{Out, fmt, made};
use forge_doc::text::tokens;
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
    /// The Forge names of what its row makes, in the order the row's .fc names are.
    ents: Vec<String>,
}

#[derive(Clone, Debug)]
struct RowOf {
    st: usize,
    /// Its statement's own row (the others are its fixes and dimensions).
    main: bool,
    /// Each word's number's place in the file, where it has one.
    slots: Vec<Option<Slot>>,
}

/// A sketch block as the sketcher's rows, with the way back.
#[derive(Clone, Debug)]
pub struct Opened {
    pub id: String,
    pub rows: Vec<String>,
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
}

struct Rd<'a> {
    src: &'a str,
    view: &'a forge_lang::SketchView,
    /// The params a bare name reads in the block: the block's own, then shared ones.
    params: &'a HashMap<String, (ParamKey, Dim)>,
    env: &'a [(String, String)],
    /// Forge's name of an entity (`l1`, `r[2]`) to the .fc row word.
    words: HashMap<String, String>,
    fc: HashMap<char, usize>,
    rows: Vec<String>,
    of: Vec<RowOf>,
}

/// A point argument: where it is, and how it's written.
enum Pt {
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
                let env = |name: &str| {
                    let v = self.env.iter().find(|p| p.0 == name)?.1.parse::<f64>().ok()?;
                    Some((v, forge_model::Unit::None))
                };
                forge_doc::expr::eval(t, &env).ok()?;
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

    fn point(&self, e: &Expr) -> Option<Pt> {
        match &e.kind {
            ExprKind::Tuple(xs) if xs.len() == 2 => Some(Pt::Fixed(Some(self.slot(&xs[0])), Some(self.slot(&xs[1])))),
            _ => match call(e) {
                Some(("near", a)) if a.len() == 2 => {
                    Some(Pt::Free(Some(self.slot(&a[0].value)), Some(self.slot(&a[1].value))))
                }
                Some(("pt", a)) if a.len() == 2 => {
                    Some(Pt::Fixed(Some(self.slot(&a[0].value)), Some(self.slot(&a[1].value))))
                }
                _ => self.word(e).map(Pt::Word),
            },
        }
    }

    fn at(&self, p: EntityId) -> (f64, f64) {
        let q = self.view.s.point(p);
        (q.x, q.y)
    }

    fn next(&mut self, kind: char) -> String {
        let c = self.fc.entry(kind).or_default();
        *c += 1;
        format!("{kind}{c}")
    }

    fn push(&mut self, st: usize, main: bool, words: Vec<(String, Option<Slot>)>) {
        let (w, s): (Vec<String>, Vec<Option<Slot>>) = words.into_iter().unzip();
        self.rows.push(w.join(" "));
        self.of.push(RowOf { st, main, slots: s });
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
        let id = name.and_then(|n| self.view.named.iter().find(|x| x.0 == n).map(|x| (x.1, x.2.clone())));
        let ent = || -> Result<(EntityId, Vec<EntityId>), String> {
            if name.is_none() {
                return Err(format!("an unnamed `{f}`"));
            }
            id.clone().ok_or_else(|| format!("`{f}` the sketch didn't make"))
        };
        let num = |v: f64| (fmt(v), None);
        let w = |t: &str| (t.to_string(), None);
        let unsupported = || Err(format!("a `{f}` reference the sketcher can't name"));
        let st = stmts.len();
        let mut ents = vec![];
        let mut fixes: Vec<String> = vec![];
        let mut dims: Vec<Vec<(String, Option<Slot>)>> = vec![];
        let mut row: Vec<(String, Option<Slot>)> = vec![];
        // a point's words in the row: its numbers (with their slots) or the word it shares
        let put = |row: &mut Vec<(String, Option<Slot>)>, fixes: &mut Vec<String>, p: Option<Pt>, at: (f64, f64), me: String| {
            match p {
                Some(Pt::Word(w)) => row.push((w, None)),
                Some(Pt::Fixed(x, y)) => {
                    row.extend([(exact(at.0), x), (exact(at.1), y)]);
                    fixes.push(me);
                }
                Some(Pt::Free(x, y)) => row.extend([(exact(at.0), x), (exact(at.1), y)]),
                None => return false,
            }
            true
        };
        match f {
            "pt" | "near" => {
                let (e, _) = ent()?;
                let n = self.next('p');
                row.push(w("pt"));
                let p = Pt::Free(Some(self.slot(&args[0].value)), args.get(1).map(|a| self.slot(&a.value)));
                let p = if f == "pt" {
                    match p {
                        Pt::Free(x, y) => Pt::Fixed(x, y),
                        p => p,
                    }
                } else {
                    p
                };
                put(&mut row, &mut fixes, Some(p), self.at(e), n.clone());
                ents.push((n, name.unwrap_or("").to_string()));
            }
            "line" => {
                let (e, _) = ent()?;
                let Some(Entity::Line { a, b }) = self.view.s.entity(e) else {
                    return unsupported();
                };
                let n = self.next('l');
                row.push(w("line"));
                let pa = arg(args, 0, "a").and_then(|x| self.point(x));
                let pb = arg(args, 1, "b").and_then(|x| self.point(x));
                if !put(&mut row, &mut fixes, pa, self.at(a), format!("{n}.a"))
                    || !put(&mut row, &mut fixes, pb, self.at(b), format!("{n}.b"))
                {
                    return unsupported();
                }
                if arg(args, 9, "construction").is_some_and(|c| matches!(c.kind, ExprKind::Bool(true))) {
                    row.push(w("cons"));
                }
                ents.push((n, name.unwrap_or("").to_string()));
            }
            "circle" => {
                let (e, _) = ent()?;
                let Some(Entity::Circle { center, .. }) = self.view.s.entity(e) else {
                    return unsupported();
                };
                let n = self.next('c');
                row.push(w("circle"));
                let pc = arg(args, 0, "center").and_then(|x| self.point(x));
                if !put(&mut row, &mut fixes, pc, self.at(center), format!("{n}.c")) {
                    return unsupported();
                }
                row.push(num(self.view.s.radius(e)));
                if let Some(r) = arg(args, 1, "r") {
                    let s = self.slot(r);
                    dims.push(match self.value(&s) {
                        Some(v) => vec![w("rad"), w(&n), (v, Some(s))],
                        None => vec![w("ref"), w("rad"), w(&n)],
                    });
                }
                ents.push((n, name.unwrap_or("").to_string()));
            }
            "arc" => {
                let (e, _) = ent()?;
                let Some(Entity::Arc { center, start, end, ccw }) = self.view.s.entity(e) else {
                    return unsupported();
                };
                let n = self.next('a');
                row.push(w("arc"));
                for (key, p, part) in [("center", center, "c"), ("from", start, "a"), ("to", end, "b")] {
                    let pp = arg(args, 9, key).and_then(|x| self.point(x));
                    if !put(&mut row, &mut fixes, pp, self.at(p), format!("{n}.{part}")) {
                        return unsupported();
                    }
                }
                if !ccw {
                    row.push(w("cw"));
                }
                ents.push((n, name.unwrap_or("").to_string()));
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
                    Some(ExprKind::Tuple(xs)) if xs.len() == 2 => Some((self.slot(&xs[0]), self.slot(&xs[1]))),
                    _ => None,
                };
                let (cx, cy) = pair(corner).map_or((None, None), |(x, y)| (Some(x), Some(y)));
                row.extend([w("rect"), (exact(p0.0), cx), (exact(p0.1), cy)]);
                row.extend([num(p1.0 - p0.0), num(p2.1 - p1.1)]);
                fixes.push(format!("{}.a", ns[0]));
                match pair(size).filter(|_| corner.is_some()) {
                    Some((sw, sh)) => {
                        for (dim, l, s) in [("hdist", &ns[0], sw), ("vdist", &ns[1], sh)] {
                            dims.push(match self.value(&s) {
                                Some(v) => vec![w(dim), w(&format!("{l}.a")), w(&format!("{l}.b")), (v, Some(s))],
                                None => vec![w("ref"), w(dim), w(&format!("{l}.a")), w(&format!("{l}.b"))],
                            });
                        }
                    }
                    // by its centre, or sized otherwise: held where it is
                    None => fixes.push(format!("{}.a", ns[2])),
                }
                let base = name.unwrap_or("");
                for (k, l) in ns.into_iter().enumerate() {
                    ents.push((l, format!("{base}[{k}]")));
                }
            }
            "poly" | "spline" => {
                let (e, segs) = ent()?;
                let Some(ExprKind::List(ps)) = arg(args, 0, "points").map(|x| &x.kind) else {
                    return what("a list of points built from other values");
                };
                let closed = !arg(args, 9, "closed").is_some_and(|c| matches!(c.kind, ExprKind::Bool(false)));
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
                    self.view.s.spline_points(e).map(<[EntityId]>::to_vec).unwrap_or_default()
                };
                if at.len() != ps.len() {
                    return unsupported();
                }
                let first = self.fc.get(&'l').copied().unwrap_or(0) + 1;
                let sides = ps.len() - usize::from(!closed);
                let sn = if f == "spline" { Some(self.next('s')) } else { None };
                row.push(w(f));
                for (k, p) in ps.iter().enumerate() {
                    let me = match &sn {
                        Some(s) => format!("{s}.{k}"),
                        None if k < sides => format!("l{}.a", first + k),
                        None => format!("l{}.b", first + k - 1),
                    };
                    let pp = self.point(p);
                    if !put(&mut row, &mut fixes, pp, self.at(at[k]), me) {
                        return unsupported();
                    }
                }
                if !closed {
                    row.push(w("open"));
                }
                let base = name.unwrap_or("");
                match sn {
                    Some(s) => ents.push((s, base.to_string())),
                    None => {
                        for k in 0..sides {
                            let l = self.next('l');
                            ents.push((l, format!("{base}[{k}]")));
                        }
                    }
                }
            }
            "ellipse" => {
                let (e, _) = ent()?;
                let Some(Entity::Ellipse { center, major, b, ends: None }) = self.view.s.entity(e) else {
                    return unsupported();
                };
                let n = self.next('e');
                row.push(w("ellipse"));
                for (key, p, part) in [("center", center, "c"), ("major", major, "m")] {
                    let pp = arg(args, 9, key).and_then(|x| self.point(x));
                    if !put(&mut row, &mut fixes, pp, self.at(p), format!("{n}.{part}")) {
                        return unsupported();
                    }
                }
                row.push(num(self.view.s.params()[b]));
                if let Some(bx) = arg(args, 9, "b") {
                    let s = self.slot(bx);
                    dims.push(match self.value(&s) {
                        Some(v) => vec![w("rad"), w(&n), (v, Some(s))],
                        None => vec![w("ref"), w("rad"), w(&n)],
                    });
                }
                ents.push((n, name.unwrap_or("").to_string()));
            }
            "tangent" | "coincident" | "horizontal" | "vertical" | "parallel" | "perpendicular" | "equal"
            | "midpoint" | "concentric" | "collinear" | "symmetric" | "fix" | "distance" | "radius" | "angle"
                if name.is_none() =>
            {
                let kw = match f {
                    "tangent" => "tan",
                    "coincident" => "co",
                    "horizontal" => "hor",
                    "vertical" => "ver",
                    "parallel" => "par",
                    "perpendicular" => "perp",
                    "equal" => "eq",
                    "midpoint" => "mid",
                    "concentric" => "conc",
                    "collinear" => "colin",
                    "symmetric" => "sym",
                    "fix" => "fix",
                    "distance" => "dist",
                    "radius" => "rad",
                    _ => "ang",
                };
                let valued = matches!(f, "distance" | "radius" | "angle");
                let n = args.len() - usize::from(valued);
                let mut ws = vec![];
                for a in &args[..n] {
                    match self.word(&a.value) {
                        Some(x) => ws.push(w(&x)),
                        None => return unsupported(),
                    }
                }
                if valued {
                    let s = self.slot(&args[n].value);
                    match self.value(&s) {
                        Some(v) => {
                            row.push(w(kw));
                            row.extend(ws);
                            row.push((v, Some(s)));
                        }
                        None => {
                            row.extend([w("ref"), w(kw)]);
                            row.extend(ws);
                        }
                    }
                } else {
                    row.push(w(kw));
                    row.extend(ws);
                }
            }
            "project" => return what("a `project`"),
            f => return Err(format!("`{f}`")),
        }
        stmts.push(St {
            span: s.span,
            name: name.map(str::to_string),
            ents: ents.iter().map(|e| e.1.clone()).collect(),
        });
        for (fc, fg) in ents {
            // a loop's sides are named by index; the loop itself has no single word
            self.words.insert(fg, fc);
        }
        self.push(st, true, row);
        for x in fixes {
            self.push(st, false, vec![w("fix"), w(&x)]);
        }
        for d in dims {
            self.push(st, false, d);
        }
        Ok(())
    }
}

/// The program's sketch statement `id`, and the first of the params written just above it.
fn find(prog: &[Stmt], id: &str) -> Option<(usize, usize)> {
    let k = prog
        .iter()
        .position(|s| matches!(&s.kind, StmtKind::Block { kind: syntax::BlockKind::Sketch, name, .. } if name == id))?;
    let mut j = k;
    while j > 0
        && matches!(&prog[j - 1].kind, StmtKind::Param { owner, .. } if owner.first().is_some_and(|o| o.name == id))
    {
        j -= 1;
    }
    Some((j, k))
}

/// Opens sketch `id` of `src` (fonts and used files from `dir`).
pub fn open(src: &str, id: &str, dir: Option<std::path::PathBuf>) -> Result<Opened, ToolError> {
    let no = |why: String| ToolError::NoForgeForm { tool: "Edit Sketch", why };
    let prog = syntax::parse(src).map_err(|e| no(e.text()))?;
    let (_, k) = find(&prog, id).ok_or_else(|| no(format!("{id} is not a sketch of this part")))?;
    let opts = forge_lang::Options {
        build: false,
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
        .ok_or_else(|| no(format!("{id} didn't evaluate: fix what the issues list says first")))?,
    };
    let (run, _) = forge_lang::run(src, opts, Default::default());
    let mut params: HashMap<String, (ParamKey, Dim)> = HashMap::new();
    let mut env = vec![];
    let own = |p: &ParamKey| p.owner.len() == 1 && p.owner[0].name == id && p.owner[0].index.is_empty();
    // shared first, so the block's own of the same name wins
    for p in run.shared.iter().chain(run.locals.iter().filter(|p| own(&p.key))) {
        if let forge_lang::eval::Value::Num(n) = &p.value {
            let v = if n.dim == Dim::Angle { n.v.to_degrees() } else { n.v };
            params.insert(p.key.name.clone(), (p.key.clone(), n.dim));
            env.retain(|e: &(String, String)| e.0 != p.key.name);
            env.push((p.key.name.clone(), fmt(v)));
        }
    }
    let StmtKind::Block { body, .. } = &prog[k].kind else {
        unreachable!("a sketch block")
    };
    let mut rd = Rd {
        src,
        view: &view,
        params: &params,
        env: &env,
        words: HashMap::new(),
        fc: HashMap::new(),
        rows: vec![],
        of: vec![],
    };
    let mut stmts = vec![];
    let mut read_only = None;
    for s in body {
        if let Err(what) = rd.stmt(s, &mut stmts)
            && read_only.is_none()
        {
            let line = s.span.lines(src).0;
            read_only = Some(format!("{what} (line {line}) has no form in the sketcher yet; edit it in the source"));
        }
    }
    let (rows, of) = (rd.rows, rd.of);
    let mut taken: HashSet<String> = forge_lang::edit::bound(&prog).into_iter().collect();
    taken.extend(forge_lang::edit::bound(body));
    taken.extend(params.keys().cloned());
    let mine: Vec<String> = run.locals.iter().filter(|p| own(&p.key)).map(|p| p.key.name.clone()).collect();
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
    })
}

/// A row with its entity words read as Forge names, numbers as they are: what's compared.
fn canon(row: &str, names: &HashMap<String, String>) -> Vec<String> {
    tokens(row)
        .into_iter()
        .map(|t| {
            let (base, part) = t.split_once('.').map_or((t.as_str(), ""), |(b, p)| (b, p));
            match names.get(base) {
                Some(n) if part.is_empty() => n.clone(),
                Some(n) => format!("{n}.{part}"),
                None => t.clone(),
            }
        })
        .collect()
}

impl Opened {
    /// The .fc name to Forge name of every entity the rows make, row `k < kept.len()` being
    /// opened row `kept[k]`; new rows' entities have none yet.
    fn names(&self, rows: &[String], kept: &[usize]) -> (HashMap<String, String>, Vec<Vec<String>>) {
        let mut out = HashMap::new();
        let mut per = vec![];
        let mut fc: HashMap<char, usize> = HashMap::new();
        for (k, r) in rows.iter().enumerate() {
            let of = kept.get(k).map(|&o| &self.of[o]);
            let ents: Vec<String> = match of {
                Some(o) if o.main => self.stmts[o.st].ents.clone(),
                _ => vec![],
            };
            for (i, kind) in made(&tokens(r)).into_iter().enumerate() {
                let c = fc.entry(kind).or_default();
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
    pub fn write_back(&self, rows: &[String], kept: &[usize]) -> Result<Vec<Op>, ToolError> {
        let opened = self.names(&self.rows, &(0..self.rows.len()).collect::<Vec<_>>()).0;
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
            // the same words, only values changed: each is put back where it's written
            let same = a.len() == b.len() && a.iter().zip(&b).enumerate().all(|(i, (x, y))| x == y || of.slots[i].is_some());
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
            for (i, (x, y)) in a.iter().zip(&b).enumerate() {
                if x == y {
                    continue;
                }
                let s = of.slots[i].as_ref().expect("a slot");
                let Ok(v) = y.parse::<f64>() else {
                    let t = y.strip_prefix('(').and_then(|t| t.strip_suffix(')')).unwrap_or(y);
                    splices.push((s.span, t.to_string()));
                    continue;
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
        let mut o = Out::new(&self.id, self.taken.clone());
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
            let hi = src[s.hi as usize..].find('\n').map_or(src.len(), |n| s.hi as usize + n + 1);
            splices.push((Span::new(lo, hi), String::new()));
        }
        if splices.is_empty() && fresh.is_empty() && o.params.is_empty() {
            return Ok(sets);
        }
        let prog = syntax::parse(src).map_err(|e| ToolError::NoForgeForm {
            tool: "sketch",
            why: e.text(),
        })?;
        let (j, k) = find(&prog, &self.id).expect("opened from this text");
        let (lo, hi) = (prog[j].span.lo as usize, prog[k].span.hi as usize);
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
            Span::new(prog[k].span.lo as usize, prog[k].span.lo as usize),
            o.params.concat(),
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
    use crate::shell::sketching::{SketchFor, SketchMode};

    fn mode(o: &Opened) -> SketchMode {
        let mut m = SketchMode::new(
            SketchFor::Forge(o.id.clone()),
            "xy".into(),
            forge_math::Frame::default(),
            o.rows.clone(),
            None,
        );
        m.set_params(o.params.clone());
        m
    }

    fn apply(src: &str, ops: &[Op]) -> Result<String, forge_lang::error::Error> {
        forge_lang::edit::apply_all_checked(src, ops)
    }

    fn demo() -> String {
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/demo/part_tools.fg")).unwrap()
    }

    fn files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                files(&p, out);
            } else if p.extension().is_some_and(|x| x == "fg") {
                out.push(p);
            }
        }
    }

    #[test]
    fn every_example_sketch_opens_solves_and_finishes_unchanged_to_the_byte() {
        let mut paths = vec![];
        files(std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples")), &mut paths);
        paths.sort();
        let (mut editable, mut looked, mut sketches) = (0, 0, 0);
        let mut bad = vec![];
        for p in &paths {
            let src = std::fs::read_to_string(p).unwrap();
            let Ok(prog) = syntax::parse(&src) else { continue };
            for s in &prog {
                let StmtKind::Block {
                    kind: syntax::BlockKind::Sketch,
                    name,
                    ..
                } = &s.kind
                else {
                    continue;
                };
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
                    bad.push(format!("{}: {name}: {e}\n{}", p.display(), o.rows.join("\n")));
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
        assert!(sketches > 300 && editable * 4 > sketches * 3, "{editable} editable, {looked} read-only of {sketches}");
    }

    #[test]
    fn a_moved_point_and_a_changed_dimension_set_their_params() {
        let src = demo();
        let o = open(&src, "sk2", None).unwrap();
        assert_eq!(o.rows, ["circle 0 0 12", "rad c1 r3", "pt 20 0"].map(String::from));
        let mut rows = o.rows.clone();
        rows[2] = "pt 25 -3".into();
        rows[1] = "rad c1 14".into();
        let kept: Vec<usize> = (0..rows.len()).collect();
        let ops = o.write_back(&rows, &kept).unwrap();
        let text: Vec<String> = ops.iter().map(|op| format!("{:?}", op.json())).collect();
        assert_eq!(ops.len(), 3, "{text:?}");
        assert!(ops.iter().all(|op| matches!(op, Op::SetParam { .. })), "{text:?}");
        let after = apply(&src, &ops).unwrap();
        assert!(after.contains("param sk2.x4 = 25\n") && after.contains("param sk2.y5 = -3\n"), "{after}");
        assert!(after.contains("param sk2.r3 = 14\n"), "{after}");
        // the rect's sides are its size params
        let o = open(&src, "sk1", None).unwrap();
        let k = o.rows.iter().position(|r| r.starts_with("hdist")).unwrap();
        assert_eq!(o.rows[k], "hdist l1.a l1.b w3");
        let mut rows = o.rows.clone();
        rows[k] = "hdist l1.a l1.b 90".into();
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
        assert!(m.add("line 30 5 40 5 cons".into()));
        assert!(m.add("hor l1".into()));
        assert!(m.add("co l1.a p2".into()) || m.error.is_some());
        let rows: Vec<String> = m.rows.iter().filter(|r| !r.starts_with("co ")).cloned().collect();
        let ops = o.write_back(&rows, m.kept()).unwrap();
        let [Op::ReplaceFeature { id, text }] = &ops[..] else {
            panic!("{ops:?}")
        };
        assert_eq!(id, "sk2");
        assert!(text.contains("  c1 = circle(near(x1, y2), r3)\n  p1 = near(x4, y5)\n"), "{text}");
        assert!(text.contains("  l1 = line(near(x6, y7), near(x8, y9), construction: true)\n  horizontal(l1)\n}"), "{text}");
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
        assert!(o.read_only.as_ref().is_some_and(|r| r.contains("`for` loop (line 4)")), "{:?}", o.read_only);
        assert_eq!(o.rows[2], "rad c1 (d / 2)");
        let m = mode(&o);
        assert!(m.error.is_none(), "{:?}", m.error);
        let mut rows = o.rows.clone();
        rows[2] = "rad c1 6".into();
        let e = o.write_back(&rows, &[0, 1, 2]).unwrap_err();
        assert_eq!(e.kind(), "Driven", "{}", e.text());
        // a typed expression is written as it is
        rows[2] = "rad c1 (d / 4)".into();
        let ops = o.write_back(&rows, &[0, 1, 2]).unwrap();
        let after = apply(src, &ops).unwrap();
        assert!(after.contains("circle((0, 0), d / 4)"), "{after}");
    }
}
