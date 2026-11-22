//! A sketch drawn with the app's sketch tools, written as a Forge sketch block. The tools write
//! the sketcher's typed rows (`Row::Line`, `Row::Rect`, a `Row::Dim` distance); this writes each as
//! Forge (`l1 = line(near(x1, y2), near(x3, y4))`, `distance(l1, d5)`): points drawn are free
//! guesses (`near`), as in the drawing; every number (guess, size, dimension) is a param of the
//! sketch. A row with no Forge form is refused, typed.

mod edit;
#[cfg(test)]
mod variants;

pub use edit::{Opened, open};

use super::fgtool::ToolError;
use super::sketching::rows::{D, Part, Pt, Ref, Row, V};
use std::collections::{HashMap, HashSet};

struct Out {
    name: String,
    params: Vec<String>,
    /// What `project` rows read, bound above the block (a selection is no sketch row's to make).
    binds: Vec<String>,
    rows: Vec<String>,
    /// What each row name (`l1`, `c2`, `p1`) is in Forge.
    names: HashMap<String, String>,
    /// The row names made so far, by kind: `l3` is the third line.
    named: HashMap<char, usize>,
    /// Forge names in use: a fresh one is the next free `{kind}{k}`.
    taken: HashSet<String>,
    /// Names a rewritten statement keeps, taken before fresh ones.
    keep: Vec<String>,
    /// How many params the block has already: new ones are numbered on from there.
    base: usize,
    /// Lines that are a new rect's sides: (its row, side), for dimensions a fixed rect holds.
    rect_sides: HashMap<String, (usize, usize)>,
}

fn num(v: &V, row: &Row) -> Result<f64, ToolError> {
    v.num().filter(|v| v.is_finite()).ok_or(ToolError::BadNumber {
        param: "sketch value",
        text: format!("{v} (in `{row}`)"),
    })
}

fn fmt(v: f64) -> String {
    super::fg::short(v)
}

impl Out {
    fn new(name: &str, taken: HashSet<String>) -> Out {
        Out {
            name: name.to_string(),
            params: vec![],
            binds: vec![],
            rows: vec![],
            names: HashMap::new(),
            named: HashMap::new(),
            taken,
            keep: vec![],
            base: 0,
            rect_sides: HashMap::new(),
        }
    }

    fn param(&mut self, base: &str, v: f64) -> String {
        self.param_in(base, &fmt(v))
    }

    fn param_in(&mut self, base: &str, text: &str) -> String {
        let mut k = self.base + self.params.len() + 1;
        while self.taken.contains(&format!("{base}{k}")) {
            k += 1;
        }
        let n = format!("{base}{k}");
        self.taken.insert(n.clone());
        self.params.push(format!("param {}.{n} = {text}\n", self.name));
        n
    }

    /// A name above the block for the part's geometry `text` (a name already is one).
    fn bind(&mut self, text: &str) -> String {
        let plain = text
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
            && text.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
        if plain {
            return text.to_string();
        }
        let n = (1..)
            .map(|k| format!("{}_src{k}", self.name))
            .find(|n| !self.taken.contains(n))
            .expect("a free name");
        self.taken.insert(n.clone());
        self.binds.push(format!("{n} = {text}\n"));
        n
    }

    fn row_next(&mut self, kind: char) -> String {
        let c = self.named.entry(kind).or_default();
        *c += 1;
        format!("{kind}{c}")
    }

    /// A Forge name for a new entity: the kept one, else the next free `{kind}{k}`.
    fn fresh(&mut self, kind: char) -> String {
        if !self.keep.is_empty() {
            return self.keep.remove(0);
        }
        let n = (1..)
            .map(|k| format!("{kind}{k}"))
            .find(|n| !self.taken.contains(n))
            .expect("a free name");
        self.taken.insert(n.clone());
        n
    }

    /// An entity row's own name: its row name, and the Forge one it's written as.
    fn entity(&mut self, kind: char) -> String {
        let f = self.row_next(kind);
        // its row name, where Forge has no such name yet
        let n = if self.keep.is_empty() && self.taken.insert(f.clone()) {
            f.clone()
        } else {
            self.fresh(kind)
        };
        self.names.insert(f, n.clone());
        n
    }

    /// A point: a free one (a guess the solver starts from, its numbers params) or the sketch's.
    fn point(&mut self, p: &Pt, row: &Row) -> Result<String, ToolError> {
        match p {
            Pt::At(x, y) => {
                let (x, y) = (num(x, row)?, num(y, row)?);
                // where it was drawn: a guess the solver starts from, kept as params like any number
                let (px, py) = (self.param("x", x), self.param("y", y));
                Ok(format!("near({px}, {py})"))
            }
            Pt::R(r) => self.reference(r, true, row),
        }
    }

    /// An entity or a point by its row name: `l1`, `l1.a`, `c1` (as a point: its centre), `o`.
    fn reference(&self, w: &Ref, as_point: bool, row: &Row) -> Result<String, ToolError> {
        if w.base == "o" {
            return Ok("near(0, 0)".into());
        }
        let base = w.base.as_str();
        let e = self.names.get(base).ok_or(no_form(row))?;
        Ok(match w.part {
            Part::A => format!("{e}.a"),
            Part::B => format!("{e}.b"),
            Part::C => format!("{e}.center"),
            Part::M | Part::K(_) => return Err(no_form(row)),
            Part::Whole if as_point && (base.starts_with('c') || base.starts_with('a')) => {
                format!("{e}.center")
            }
            Part::Whole => e.clone(),
        })
    }

    /// A dimension's value: a number becomes a param, a name or expression is written as it is.
    fn value(&mut self, base: &str, v: &V, unit: &str) -> String {
        match v {
            V::N(x) => self.param_in(base, &format!("{}{unit}", fmt(*x))),
            V::X(t) => t
                .strip_prefix('(')
                .and_then(|x| x.strip_suffix(')'))
                .unwrap_or(t)
                .to_string(),
        }
    }

    /// Rows `from` on written as Forge rows (after `kept`, the Forge names of what each earlier
    /// row makes).
    fn rows_from(&mut self, rows: &[Row], from: usize, kept: &[Vec<String>]) -> Result<(), ToolError> {
        // a new rect's own width and height: the dimensions on its sides, else as drawn
        let mut held: HashMap<(usize, usize), f64> = HashMap::new();
        // a new circle's own radius (an ellipse's minor axis): a dimension on it, else as drawn
        let mut radii: HashMap<String, f64> = HashMap::new();
        let mut counts: HashMap<char, usize> = HashMap::new();
        let mut circles = vec![];
        for (k, r) in rows.iter().enumerate() {
            for (side, kind) in r.made().iter().enumerate() {
                let c = counts.entry(*kind).or_default();
                *c += 1;
                if k >= from && matches!(r, Row::Rect(..)) {
                    self.rect_sides.insert(format!("l{c}"), (k, side));
                }
                if k >= from && matches!(r, Row::Circle(..)) {
                    circles.push(format!("c{c}"));
                }
                if k >= from && matches!(r, Row::Ellipse(..)) {
                    circles.push(format!("e{c}"));
                }
            }
            match r {
                Row::Dim(d @ (D::Rad | D::Dia), on, Some(v)) => {
                    if let [c] = &on[..]
                        && !c.is_point()
                        && circles.contains(&c.base)
                    {
                        let v = num(v, r)?;
                        radii.insert(c.base.clone(), if *d == D::Dia { v / 2.0 } else { v });
                    }
                }
                Row::Dim(d @ (D::HDist | D::VDist), on, Some(v)) => {
                    if let [a, b] = &on[..]
                        && a.is_point()
                        && b.is_point()
                        && a.base == b.base
                        && let Some(&(rect, s)) = self.rect_sides.get(&a.base)
                    {
                        let k = usize::from(*d == D::VDist || s % 2 == 1);
                        held.insert((rect, k), num(v, r)?);
                    }
                }
                _ => {}
            }
        }
        for (k, r) in rows.iter().enumerate() {
            if k < from {
                let mut names = kept.get(k).into_iter().flatten();
                for kind in r.made() {
                    let f = self.row_next(kind);
                    if let Some(n) = names.next() {
                        self.names.insert(f, n.clone());
                    }
                }
                continue;
            }
            self.row(k, r, &held, &radii)?;
        }
        Ok(())
    }

    fn row(
        &mut self,
        k: usize,
        r: &Row,
        held: &HashMap<(usize, usize), f64>,
        radii: &HashMap<String, f64>,
    ) -> Result<(), ToolError> {
        let o = self;
        // a construction line is written as one; other construction geometry has no Forge form
        let cons = r.is_cons();
        let row = r.plain();
        if cons && !matches!(row, Row::Line(..)) {
            return Err(no_form(r));
        }
        let list = |ps: Vec<String>| ps.join(", ");
        match row {
            Row::Pt(p) => {
                let p = o.point(p, r)?;
                let n = o.entity('p');
                o.rows.push(format!("{n} = {p}"));
            }
            Row::Line(a, b) => {
                let (a, b) = (o.point(a, r)?, o.point(b, r)?);
                let n = o.entity('l');
                let c = if cons { ", construction: true" } else { "" };
                o.rows.push(format!("{n} = line({a}, {b}{c})"));
            }
            Row::Rect(x, y, w0, h0) => {
                let (x, y, w0, h0) = (num(x, r)?, num(y, r)?, num(w0, r)?, num(h0, r)?);
                let w = held.get(&(k, 0)).copied().unwrap_or(w0);
                let h = held.get(&(k, 1)).copied().unwrap_or(h0);
                let (px, py) = (o.param("x", x), o.param("y", y));
                let (pw, ph) = (o.param("w", w), o.param("h", h));
                let n = o.fresh('r');
                o.rows
                    .push(format!("{n} = rect(corner: ({px}, {py}), size: ({pw}, {ph}))"));
                for side in 0..4 {
                    let f = o.row_next('l');
                    o.names.insert(f, format!("{n}[{side}]"));
                }
            }
            Row::Circle(c, rad) => {
                let c = o.point(c, r)?;
                let f = format!("c{}", o.named.get(&'c').copied().unwrap_or(0) + 1);
                let rr = match radii.get(&f) {
                    Some(&v) => v,
                    None => num(rad, r)?,
                };
                let pr = o.param("r", rr);
                let n = o.entity('c');
                o.rows.push(format!("{n} = circle({c}, {pr})"));
            }
            Row::Arc(c, s, e, cw) => {
                let (c, s, e) = (o.point(c, r)?, o.point(s, r)?, o.point(e, r)?);
                let n = o.entity('a');
                o.rows.push(format!(
                    "{n} = arc(center: {c}, from: {s}, to: {e}{})",
                    if *cw { ", cw: true" } else { "" }
                ));
            }
            Row::Ellipse(c, m, b) => {
                let (c, m) = (o.point(c, r)?, o.point(m, r)?);
                // its minor axis: a dimension on it, else as drawn
                let f = format!("e{}", o.named.get(&'e').copied().unwrap_or(0) + 1);
                let b = match radii.get(&f) {
                    Some(&v) => v,
                    None => num(b, r)?,
                };
                let pb = o.param("b", b);
                let n = o.entity('e');
                o.rows
                    .push(format!("{n} = ellipse(center: {c}, major: {m}, b: {pb})"));
            }
            Row::Spline(pts, open) | Row::BSpline(_, pts, open) => {
                // control vertices: Forge's are cubic, or as cubic as few points allow
                let degree = match row {
                    Row::BSpline(d, ..) => Some(num(d, r)?),
                    _ => None,
                };
                let ps = pts.iter().map(|p| o.point(p, r)).collect::<Result<Vec<_>, _>>()?;
                if let Some(d) = degree
                    && d != 3.0_f64.min(ps.len() as f64 - 1.0 - f64::from(u8::from(!open)))
                {
                    return Err(no_form(r));
                }
                let n = o.entity('s');
                o.rows.push(format!(
                    "{n} = spline([{}], closed: {}{})",
                    list(ps),
                    if *open { "false" } else { "true" },
                    if degree.is_some() { ", cv: true" } else { "" }
                ));
            }
            // a fixed rect holds its own corners square and its sides' lengths
            Row::Dim(D::HDist | D::VDist, on, Some(_))
                if on
                    .first()
                    .filter(|x| x.is_point())
                    .is_some_and(|x| o.rect_sides.contains_key(&x.base)) => {}
            Row::Co(a, b)
            | Row::Par(a, b)
            | Row::Perp(a, b)
            | Row::Tan(a, b)
            | Row::Conc(a, b)
            | Row::Colin(a, b) => {
                let f = match row {
                    Row::Co(..) => "coincident",
                    Row::Par(..) => "parallel",
                    Row::Perp(..) => "perpendicular",
                    Row::Tan(..) => "tangent",
                    Row::Conc(..) => "concentric",
                    _ => "collinear",
                };
                let point = matches!(row, Row::Co(..));
                let (a, b) = (o.reference(a, point, r)?, o.reference(b, point, r)?);
                o.rows.push(format!("{f}({a}, {b})"));
            }
            Row::Eq(es) => {
                let [a, rest @ ..] = &es[..] else {
                    return Err(no_form(r));
                };
                if rest.is_empty() {
                    return Err(no_form(r));
                }
                let (a, b) = (o.reference(a, false, r)?, o.reference(&rest[0], false, r)?);
                o.rows.push(format!("equal({a}, {b})"));
            }
            Row::Hor(es) | Row::Ver(es) | Row::Fix(es) => {
                let f = match row {
                    Row::Hor(_) => "horizontal",
                    Row::Ver(_) => "vertical",
                    _ => "fix",
                };
                let fix = matches!(row, Row::Fix(_));
                for e in es {
                    let x = o.reference(e, fix, r)?;
                    o.rows.push(format!("{f}({x})"));
                }
            }
            Row::Sym(p, q, l) => {
                let (p, q) = (o.reference(p, true, r)?, o.reference(q, true, r)?);
                let l = o.reference(l, false, r)?;
                o.rows.push(format!("symmetric({p}, {q}, {l})"));
            }
            Row::Poly(pts, open) => {
                let ps = pts.iter().map(|p| o.point(p, r)).collect::<Result<Vec<_>, _>>()?;
                let n = o.fresh('o');
                for side in 0..ps.len() - usize::from(*open) {
                    let f = o.row_next('l');
                    o.names.insert(f, format!("{n}[{side}]"));
                }
                o.rows.push(format!(
                    "{n} = poly([{}], closed: {})",
                    list(ps),
                    if *open { "false" } else { "true" }
                ));
            }
            // the part's edges or a face's boundary, laid on the plane: read afresh each rebuild
            Row::Project { of, many, approx } => {
                let of = o.bind(of);
                let tail = match approx {
                    Some(v) => format!(", approx: {}", o.param("tol", num(v, r)?)),
                    None => String::new(),
                };
                let n = if *many { o.fresh('j') } else { o.entity('j') };
                o.rows.push(format!("{n} = project({of}{tail})"));
            }
            Row::Mid(p, l) => {
                let p = o.reference(p, true, r)?;
                let l = o.reference(l, false, r)?;
                o.rows.push(format!("midpoint({p}, {l})"));
            }
            Row::Dim(D::Dist, on, Some(v)) => match &on[..] {
                [l] => {
                    let e = o.reference(l, false, r)?;
                    let d = o.value("d", v, "");
                    o.rows.push(format!("distance({e}, {d})"));
                }
                [a, b] => {
                    let (a, b) = (o.reference(a, true, r)?, o.reference(b, false, r)?);
                    let d = o.value("d", v, "");
                    o.rows.push(format!("distance({a}, {b}, {d})"));
                }
                _ => return Err(no_form(r)),
            },
            // a circle holds its own radius, an ellipse its minor axis
            Row::Dim(D::Rad | D::Dia, on, Some(_))
                if on
                    .first()
                    .is_some_and(|c| !c.is_point() && radii.contains_key(&c.base)) => {}
            Row::Dim(d @ (D::Rad | D::Dia), on, Some(v)) => {
                let [c] = &on[..] else {
                    return Err(no_form(r));
                };
                let e = o.reference(c, false, r)?;
                let v = match (d, v) {
                    (D::Dia, V::N(x)) => V::N(x / 2.0),
                    (D::Dia, V::X(_)) => return Err(no_form(r)),
                    (_, v) => v.clone(),
                };
                let x = o.value("r", &v, "");
                o.rows.push(format!("radius({e}, {x})"));
            }
            Row::Dim(D::Ang, on, Some(v)) => {
                let [a, b] = &on[..] else {
                    return Err(no_form(r));
                };
                let (a, b) = (o.reference(a, false, r)?, o.reference(b, false, r)?);
                let x = o.value("ang", v, "deg");
                o.rows.push(format!("angle({a}, {b}, {x})"));
            }
            _ => return Err(no_form(r)),
        }
        Ok(())
    }
}

fn no_form(row: &Row) -> ToolError {
    ToolError::NoForgeForm {
        tool: "sketch",
        why: format!("the row `{row}` has no Forge form here; write it in the source"),
    }
}

/// The rows Forge sketch statements (as a block holds them, on XY) read as.
pub fn read(statements: &str) -> Result<Vec<Row>, ToolError> {
    let src = format!("sketch drawn(on: xy) {{\n{statements}\n}}\n");
    let o = open(&src, "drawn", None)?;
    match o.read_only {
        Some(why) => Err(ToolError::NoForgeForm { tool: "sketch", why }),
        None => Ok(o.rows),
    }
}

/// The Forge block for a sketch named `name` on `on` (an anchor, a sketch, or `xy`/`xz`/`yz`):
/// its params, then the block.
pub fn block(name: &str, on: &str, rows: &[Row]) -> Result<String, ToolError> {
    let mut o = Out::new(name, HashSet::new());
    o.rows_from(rows, 0, &[])?;
    let mut out: String = o.params.concat();
    out.push_str(&o.binds.concat());
    out.push_str(&format!("sketch {name}(on: {on}) {{\n"));
    for r in &o.rows {
        out.push_str(&format!("  {r}\n"));
    }
    out.push_str("}\n");
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::sketching::rows::dsl::*;

    fn check(src: &str) -> forge_lang::Run {
        let opts = forge_lang::Options {
            build: false,
            ..forge_lang::Options::default()
        };
        forge_lang::run(src, opts, Default::default()).0
    }

    #[test]
    fn drawn_rows_become_a_forge_sketch_with_its_sizes_as_params() {
        let rows: Vec<Row> = [
            Row::Rect(v("0"), v("0"), v("40"), v("30")),
            Row::Dim(D::HDist, vec![rf("l1.a"), rf("l1.b")], Some(v("42"))),
            Row::Circle(at("20", "15"), v("4")),
            Row::Dim(D::Dia, vec![rf("c1")], Some(v("10"))),
            Row::Line(at("50", "0"), at("60", "10")),
            Row::Hor(vec![rf("l5")]),
            Row::Dim(D::Dist, vec![rf("l5")], Some(v("12"))),
            Row::Pt(at("5", "5")),
        ]
        .to_vec();
        let text = block("sk1", "xy", &rows).unwrap();
        assert!(text.contains("param sk1.w3 = 42\n"), "{text}");
        assert!(
            text.contains("r1 = rect(corner: (x1, y2), size: (w3, h4))"),
            "{text}"
        );
        assert!(text.contains("c1 = circle(near(x5, y6), r7)"), "{text}");
        assert!(text.contains("param sk1.r7 = 5\n"), "{text}");
        assert!(text.contains("horizontal(l5)"), "{text}");
        assert!(text.contains("distance(l5, d12)"), "{text}");
        let r = check(&text);
        assert!(r.errors.is_empty(), "{text}\n{:?}", r.errors);
        // nothing loose: sizes and dimensions are params, drawn places free guesses
        let lints: Vec<_> = r
            .warnings
            .iter()
            .filter(|w| matches!(w, forge_lang::error::Warning::Lint(_)))
            .collect();
        assert!(lints.is_empty(), "{text}\n{lints:?}");
        // a rect side is its loop's side
        let t = block(
            "sk1",
            "xy",
            &[
                Row::Rect(v("0"), v("0"), v("4"), v("3")),
                Row::Hor(vec![rf("l2")]),
            ],
        )
        .unwrap();
        assert!(t.contains("horizontal(r1[1])"), "{t}");
        // a row with no Forge form is refused, typed
        let e = block("sk1", "xy", &[Row::Dim(D::Dist, vec![rf("l1")], None)]).unwrap_err();
        assert_eq!(e.kind(), "NoForgeForm");
    }

    #[test]
    fn projected_edges_are_bound_above_the_block_and_open_again_as_they_were() {
        use crate::shell::sketching::{SketchFor, SketchMode};
        let part =
            "sketch s(on: xy) {\n  r = rect(corner: (0, 0), size: (40, 30))\n}\nbase = extrude(s, 10)\n";
        let rows: Vec<Row> = [
            Row::Project {
                of: "edges(between: (base.end, base.side(s.r[0])))".into(),
                many: false,
                approx: None,
            },
            Row::Line(pn("j1.b"), at("50", "20")),
            Row::Project {
                of: "base.start".into(),
                many: true,
                approx: None,
            },
            Row::Project {
                of: "q".into(),
                many: false,
                approx: Some(v("0.002")),
            },
        ]
        .to_vec();
        let text = block("sk1", "base.end", &rows[..3]).unwrap();
        assert!(
            text.contains(
                "sk1_src1 = edges(between: (base.end, base.side(s.r[0])))\nsk1_src2 = base.start\nsketch sk1(on: base.end) {\n"
            ),
            "{text}"
        );
        assert!(text.contains("  j1 = project(sk1_src1)\n"), "{text}");
        assert!(text.contains("  l1 = line(j1.b, near(x1, y2))\n"), "{text}");
        // a face's several curves are a loop no row names
        assert!(text.contains("  j2 = project(sk1_src2)\n"), "{text}");
        let t4 = block("sk1", "base.end", &rows[3..]).unwrap();
        assert!(t4.contains("param sk1.tol1 = 0.002\n"), "{t4}");
        assert!(t4.contains("j1 = project(q, approx: tol1)"), "{t4}");
        let src = format!("{part}{text}");
        let (r, _) = forge_lang::run(&src, forge_lang::Options::default(), Default::default());
        assert!(r.errors.is_empty(), "{src}\n{:?}", r.errors);
        // opened again: the rows it was written from, what they project as the part has it
        let o = edit::open(&src, "sk1", None).unwrap();
        assert!(o.read_only.is_none(), "{:?}", o.read_only);
        assert_eq!(
            o.rows[0],
            Row::Project {
                of: "sk1_src1".into(),
                many: false,
                approx: None
            },
            "{:?}",
            o.rows
        );
        assert_eq!(
            o.rows[2],
            Row::Project {
                of: "sk1_src2".into(),
                many: true,
                approx: None
            },
            "{:?}",
            o.rows
        );
        let mut m = SketchMode::new(
            SketchFor::Forge("sk1".into()),
            "base.end".into(),
            forge_math::Frame::default(),
            o.rows.clone(),
        );
        m.set_params(o.params.clone());
        m.take_sources(o.sources.clone());
        assert!(m.error.is_none(), "{:?}", m.error);
        let s = m.solved.as_ref().unwrap();
        let (a, b) = crate::shell::sketching::rows::line(s, "j1").unwrap();
        assert!((a.distance(b) - 40.0).abs() < 1e-9, "{a:?} {b:?}");
        assert!(o.write_back(&m.rows, m.kept()).unwrap().is_empty());
        // a line added from the projected edge's end is written after it, by its Forge name
        assert!(m.add(Row::Line(pn("j1.a"), at("0", "20"))), "{:?}", m.error);
        let ops = o.write_back(&m.rows, m.kept()).unwrap();
        let after = forge_lang::edit::apply_all_checked(&src, &ops).unwrap();
        assert!(after.contains("= line(j1.a, near("), "{after}");
    }

    #[test]
    fn concentric_collinear_symmetric_and_construction_rows_become_forge() {
        let rows: Vec<Row> = [
            Row::Circle(at("0", "0"), v("10")),
            Row::Circle(at("1", "1"), v("4")),
            Row::Conc(rf("c1"), rf("c2")),
            Row::Line(at("0", "-20"), at("0", "20")).cons(),
            Row::Line(at("20", "0"), at("30", "0")),
            Row::Line(at("-22", "0"), at("-31", "1")),
            Row::Sym(rf("l2.a"), rf("l3.a"), rf("l1")),
            Row::Sym(rf("l2.b"), rf("l3.b"), rf("l1")),
            Row::Line(at("40", "0"), at("50", "0")),
            Row::Colin(rf("l2"), rf("l4")),
            Row::Poly(vec![at("60", "0"), at("70", "0"), at("65", "8")], false),
            Row::Ellipse(at("100", "0"), at("110", "0"), v("4")),
        ]
        .to_vec();
        let text = block("sk3", "xy", &rows).unwrap();
        assert!(text.contains("concentric(c1, c2)"), "{text}");
        assert!(text.contains("construction: true)"), "{text}");
        assert!(text.contains("symmetric(l2.a, l3.a, l1)"), "{text}");
        assert!(text.contains("collinear(l2, l4)"), "{text}");
        assert!(text.contains("o1 = poly(["), "{text}");
        assert!(text.contains("e1 = ellipse(center: near("), "{text}");
        let r = check(&text);
        assert!(r.errors.is_empty(), "{text}\n{:?}", r.errors);
    }

    #[test]
    fn lines_arcs_splines_and_their_constraints_check() {
        let rows: Vec<Row> = [
            Row::Line(at("0", "0"), at("20", "0")),
            Row::Line(at("20", "0"), at("20", "15")),
            Row::Co(rf("l1.b"), rf("l2.a")),
            Row::Perp(rf("l1"), rf("l2")),
            Row::Hor(vec![rf("l1")]),
            Row::Arc(at("30", "30"), at("35", "30"), at("30", "35"), false),
            Row::Spline(vec![at("40", "0"), at("45", "5"), at("50", "0")], true),
            Row::Pt(at("5", "20")),
            Row::Dim(D::Dist, vec![rf("l1")], Some(v("20"))),
            Row::Dim(D::Ang, vec![rf("l1"), rf("l2")], Some(v("90"))),
        ]
        .to_vec();
        let text = block("sk2", "xy", &rows).unwrap();
        assert!(text.contains("coincident(l1.b, l2.a)"), "{text}");
        assert!(text.contains("perpendicular(l1, l2)"), "{text}");
        assert!(text.contains("a1 = arc(center: near("), "{text}");
        assert!(text.contains("closed: false"), "{text}");
        assert!(text.contains("deg\n"), "{text}");
        let r = check(&text);
        let kinds: Vec<&str> = r.errors.iter().map(|e| e.kind()).collect();
        // the drawing's own conflicts are the solver's to report; the syntax and types hold
        assert!(
            kinds
                .iter()
                .all(|k| *k == "SketchConflict" || *k == "SketchNotSolved"),
            "{text}\n{:?}",
            r.errors
        );
    }
}
