//! A sketch drawn with the app's sketch tools, written as a Forge sketch block. The tools write
//! rows in the .fc sketch form (`line P P`, `rect X Y W H`, `dist l1 20`); this turns them into
//! Forge's (`l1 = line(near(x1, y2), near(x3, y4))`, `distance(l1, d5)`): points drawn are free
//! guesses (`near`), as in the drawing; every number (guess, size, dimension) is a param of the
//! sketch. A row with no Forge form is refused, typed.

mod edit;

pub use edit::{Opened, open};

use super::fgtool::ToolError;
use forge_doc::text::tokens;
use std::collections::{HashMap, HashSet};

struct Out {
    name: String,
    params: Vec<String>,
    rows: Vec<String>,
    /// What each .fc entity name (`l1`, `c2`, `p1`) is in Forge.
    names: HashMap<String, String>,
    /// The .fc names made so far, by kind: `l3` is the third line.
    fc: HashMap<char, usize>,
    /// Forge names in use: a fresh one is the next free `{kind}{k}`.
    taken: HashSet<String>,
    /// Names a rewritten statement keeps, taken before fresh ones.
    keep: Vec<String>,
    /// How many params the block has already: new ones are numbered on from there.
    base: usize,
    /// Lines that are a new rect's sides: (its row, side), for dimensions a fixed rect holds.
    rect_sides: HashMap<String, (usize, usize)>,
}

fn num(t: &str, row: &str) -> Result<f64, ToolError> {
    t.parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
        .ok_or(ToolError::BadNumber {
            param: "sketch value",
            text: format!("{t} (in `{row}`)"),
        })
}

fn fmt(v: f64) -> String {
    super::fg::short(v)
}

/// The kinds of the .fc names a row makes, in order (`rect`: four lines).
pub(super) fn made(t: &[String]) -> Vec<char> {
    match t.first().map(String::as_str) {
        Some("pt") => vec!['p'],
        Some("line") => vec!['l'],
        Some("circle") => vec!['c'],
        Some("arc") => vec!['a'],
        Some("spline" | "bspline") => vec!['s'],
        Some("ellipse" | "earc") => vec!['e'],
        Some("rect") => vec!['l'; 4],
        Some("poly") => {
            let open = t.last().is_some_and(|w| w == "open");
            let (mut k, mut pts) = (1, 0);
            while k < t.len() - usize::from(open) {
                k += if t[k].parse::<f64>().is_ok() { 2 } else { 1 };
                pts += 1;
            }
            vec!['l'; pts - usize::from(open)]
        }
        _ => vec![],
    }
}

impl Out {
    fn new(name: &str, taken: HashSet<String>) -> Out {
        Out {
            name: name.to_string(),
            params: vec![],
            rows: vec![],
            names: HashMap::new(),
            fc: HashMap::new(),
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

    fn fc_next(&mut self, kind: char) -> String {
        let c = self.fc.entry(kind).or_default();
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

    /// An entity row's own name: its .fc name, and the Forge one it's written as.
    fn entity(&mut self, kind: char) -> String {
        let f = self.fc_next(kind);
        // its .fc name, where Forge has no such name yet
        let n = if self.keep.is_empty() && self.taken.insert(f.clone()) {
            f.clone()
        } else {
            self.fresh(kind)
        };
        self.names.insert(f, n.clone());
        n
    }

    /// A point: `X Y` (two tokens, a free point there) or a name; returns the text and how many
    /// tokens it took.
    fn point(&mut self, t: &[String], row: &str) -> Result<(String, usize), ToolError> {
        let w = t.first().ok_or(no_form(row))?;
        if let Ok(x) = w.parse::<f64>() {
            let y = num(t.get(1).ok_or(no_form(row))?, row)?;
            // where it was drawn: a guess the solver starts from, kept as params like any number
            let (px, py) = (self.param("x", x), self.param("y", y));
            return Ok((format!("near({px}, {py})"), 2));
        }
        Ok((self.reference(w, true, row)?, 1))
    }

    /// An entity or a point by its .fc name: `l1`, `l1.a`, `c1` (as a point: its centre), `o`.
    fn reference(&self, w: &str, as_point: bool, row: &str) -> Result<String, ToolError> {
        if w == "o" {
            return Ok("near(0, 0)".into());
        }
        let (base, part) = match w.split_once('.') {
            Some((b, p)) => (b, Some(p)),
            None => (w, None),
        };
        let e = self.names.get(base).ok_or(no_form(row))?;
        Ok(match part {
            Some("a") => format!("{e}.a"),
            Some("b") => format!("{e}.b"),
            Some("c") => format!("{e}.center"),
            Some(_) => return Err(no_form(row)),
            None if as_point && (base.starts_with('c') || base.starts_with('a')) => format!("{e}.center"),
            None => e.clone(),
        })
    }

    /// A dimension's value: a number becomes a param, a name or expression is written as it is.
    fn value(&mut self, base: &str, v: &str, row: &str, unit: &str) -> Result<String, ToolError> {
        if v.parse::<f64>().is_ok() {
            return Ok(self.param_in(base, &format!("{}{unit}", fmt(num(v, row)?))));
        }
        Ok(v.strip_prefix('(')
            .and_then(|x| x.strip_suffix(')'))
            .unwrap_or(v)
            .to_string())
    }

    /// Rows `from` on written as Forge rows (after `kept`, the Forge names of what each earlier
    /// row makes).
    fn rows_from(&mut self, rows: &[String], from: usize, kept: &[Vec<String>]) -> Result<(), ToolError> {
        // a new rect's own width and height: the dimensions on its sides, else as drawn
        let mut held: HashMap<(usize, usize), f64> = HashMap::new();
        // a new circle's own radius: a dimension on it, else as drawn
        let mut radii: HashMap<String, f64> = HashMap::new();
        let mut counts: HashMap<char, usize> = HashMap::new();
        let mut circles = vec![];
        for (k, r) in rows.iter().enumerate() {
            let t = tokens(r);
            let kinds = made(&t);
            for (side, kind) in kinds.iter().enumerate() {
                let c = counts.entry(*kind).or_default();
                *c += 1;
                if k >= from && t[0] == "rect" {
                    self.rect_sides.insert(format!("l{c}"), (k, side));
                }
                if k >= from && t[0] == "circle" {
                    circles.push(format!("c{c}"));
                }
            }
            match t.first().map(String::as_str) {
                Some(w @ ("rad" | "dia")) => {
                    if let (Some(c), Some(v)) = (t.get(1), t.get(2))
                        && circles.contains(c)
                    {
                        let v = num(v, r)?;
                        radii.insert(c.clone(), if w == "dia" { v / 2.0 } else { v });
                    }
                }
                Some(w @ ("hdist" | "vdist")) => {
                    let side = |p: &str| p.split_once('.').map(|x| x.0.to_string());
                    if let (Some(a), Some(b), Some(v)) = (
                        t.get(1).and_then(|x| side(x)),
                        t.get(2).and_then(|x| side(x)),
                        t.get(3),
                    ) && a == b
                        && let Some(&(rect, s)) = self.rect_sides.get(&a)
                    {
                        let k = usize::from(w == "vdist" || s % 2 == 1);
                        held.insert((rect, k), num(v, r)?);
                    }
                }
                _ => {}
            }
        }
        for (k, r) in rows.iter().enumerate() {
            if k < from {
                let t = tokens(r);
                let mut names = kept.get(k).into_iter().flatten();
                for kind in made(&t) {
                    let f = self.fc_next(kind);
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
        r: &str,
        held: &HashMap<(usize, usize), f64>,
        radii: &HashMap<String, f64>,
    ) -> Result<(), ToolError> {
        let o = self;
        let mut t = tokens(r);
        // a construction line is written as one; other construction geometry has no Forge form
        let cons = t.last().is_some_and(|w| w == "cons");
        if cons {
            t.pop();
        }
        let Some((w, rest)) = t.split_first() else {
            return Ok(());
        };
        if cons && w != "line" {
            return Err(no_form(r));
        }
        match w.as_str() {
            "pt" => {
                let (p, _) = o.point(rest, r)?;
                let n = o.entity('p');
                o.rows.push(format!("{n} = {p}"));
            }
            "line" => {
                let (a, k) = o.point(rest, r)?;
                let (b, _) = o.point(&rest[k..], r)?;
                let n = o.entity('l');
                let c = if cons { ", construction: true" } else { "" };
                o.rows.push(format!("{n} = line({a}, {b}{c})"));
            }
            "rect" => {
                let v: Vec<f64> = rest.iter().map(|x| num(x, r)).collect::<Result<_, _>>()?;
                let [x, y, w0, h0] = v[..] else {
                    return Err(no_form(r));
                };
                let w = held.get(&(k, 0)).copied().unwrap_or(w0);
                let h = held.get(&(k, 1)).copied().unwrap_or(h0);
                let (px, py) = (o.param("x", x), o.param("y", y));
                let (pw, ph) = (o.param("w", w), o.param("h", h));
                let n = o.fresh('r');
                o.rows
                    .push(format!("{n} = rect(corner: ({px}, {py}), size: ({pw}, {ph}))"));
                for side in 0..4 {
                    let f = o.fc_next('l');
                    o.names.insert(f, format!("{n}[{side}]"));
                }
            }
            "circle" => {
                let (c, at) = o.point(rest, r)?;
                let f = format!("c{}", o.fc.get(&'c').copied().unwrap_or(0) + 1);
                let rr = match radii.get(&f) {
                    Some(&v) => v,
                    None => num(rest.get(at).ok_or(no_form(r))?, r)?,
                };
                let pr = o.param("r", rr);
                let n = o.entity('c');
                o.rows.push(format!("{n} = circle({c}, {pr})"));
            }
            "arc" => {
                let (c, k1) = o.point(rest, r)?;
                let (s, k2) = o.point(&rest[k1..], r)?;
                let (e, k3) = o.point(&rest[k1 + k2..], r)?;
                let cw = rest.get(k1 + k2 + k3).is_some_and(|w| w == "cw");
                let n = o.entity('a');
                o.rows.push(format!(
                    "{n} = arc(center: {c}, from: {s}, to: {e}{})",
                    if cw { ", cw: true" } else { "" }
                ));
            }
            "ellipse" => {
                let (c, k1) = o.point(rest, r)?;
                let (m, k2) = o.point(&rest[k1..], r)?;
                let b = num(rest.get(k1 + k2).ok_or(no_form(r))?, r)?;
                let pb = o.param("b", b);
                let n = o.entity('e');
                o.rows
                    .push(format!("{n} = ellipse(center: {c}, major: {m}, b: {pb})"));
            }
            "spline" => {
                let open = rest.last().is_some_and(|w| w == "open");
                let pts = if open { &rest[..rest.len() - 1] } else { rest };
                let mut ps = vec![];
                let mut k = 0;
                while k < pts.len() {
                    let (p, used) = o.point(&pts[k..], r)?;
                    ps.push(p);
                    k += used;
                }
                let n = o.entity('s');
                o.rows.push(format!(
                    "{n} = spline([{}], closed: {})",
                    ps.join(", "),
                    if open { "false" } else { "true" }
                ));
            }
            // a fixed rect holds its own corners square and its sides' lengths
            "hdist" | "vdist"
                if rest
                    .first()
                    .and_then(|x| x.split_once('.'))
                    .is_some_and(|x| o.rect_sides.contains_key(x.0)) => {}
            "co" | "par" | "perp" | "tan" | "eq" | "conc" | "colin" => {
                let f = match w.as_str() {
                    "co" => "coincident",
                    "par" => "parallel",
                    "perp" => "perpendicular",
                    "tan" => "tangent",
                    "conc" => "concentric",
                    "colin" => "collinear",
                    _ => "equal",
                };
                let point = w == "co";
                let a = o.reference(rest.first().ok_or(no_form(r))?, point, r)?;
                let b = o.reference(rest.get(1).ok_or(no_form(r))?, point, r)?;
                o.rows.push(format!("{f}({a}, {b})"));
            }
            "hor" | "ver" | "fix" => {
                let f = match w.as_str() {
                    "hor" => "horizontal",
                    "ver" => "vertical",
                    _ => "fix",
                };
                for e in rest {
                    let x = o.reference(e, w == "fix", r)?;
                    o.rows.push(format!("{f}({x})"));
                }
            }
            "sym" => {
                let p = o.reference(rest.first().ok_or(no_form(r))?, true, r)?;
                let q = o.reference(rest.get(1).ok_or(no_form(r))?, true, r)?;
                let l = o.reference(rest.get(2).ok_or(no_form(r))?, false, r)?;
                o.rows.push(format!("symmetric({p}, {q}, {l})"));
            }
            "poly" => {
                let open = rest.last().is_some_and(|w| w == "open");
                let pts = if open { &rest[..rest.len() - 1] } else { rest };
                let mut ps = vec![];
                let mut k = 0;
                while k < pts.len() {
                    let (p, used) = o.point(&pts[k..], r)?;
                    ps.push(p);
                    k += used;
                }
                let n = o.fresh('o');
                for side in 0..ps.len() - usize::from(open) {
                    let f = o.fc_next('l');
                    o.names.insert(f, format!("{n}[{side}]"));
                }
                o.rows.push(format!(
                    "{n} = poly([{}], closed: {})",
                    ps.join(", "),
                    if open { "false" } else { "true" }
                ));
            }
            "mid" => {
                let p = o.reference(rest.first().ok_or(no_form(r))?, true, r)?;
                let l = o.reference(rest.get(1).ok_or(no_form(r))?, false, r)?;
                o.rows.push(format!("midpoint({p}, {l})"));
            }
            "dist" => match rest {
                [l, v] => {
                    let e = o.reference(l, false, r)?;
                    let d = o.value("d", v, r, "")?;
                    o.rows.push(format!("distance({e}, {d})"));
                }
                [a, b, v] => {
                    let (a, b) = (o.reference(a, true, r)?, o.reference(b, false, r)?);
                    let d = o.value("d", v, r, "")?;
                    o.rows.push(format!("distance({a}, {b}, {d})"));
                }
                _ => return Err(no_form(r)),
            },
            // a circle holds its own radius
            "rad" | "dia" if rest.first().is_some_and(|c| radii.contains_key(c)) => {}
            "rad" | "dia" => {
                let e = o.reference(rest.first().ok_or(no_form(r))?, false, r)?;
                let v = rest.get(1).ok_or(no_form(r))?;
                let v = match v.parse::<f64>() {
                    Ok(x) if w == "dia" => (x / 2.0).to_string(),
                    _ if w == "dia" => return Err(no_form(r)),
                    _ => v.clone(),
                };
                let d = o.value("r", &v, r, "")?;
                o.rows.push(format!("radius({e}, {d})"));
            }
            "ang" => {
                let a = o.reference(rest.first().ok_or(no_form(r))?, false, r)?;
                let b = o.reference(rest.get(1).ok_or(no_form(r))?, false, r)?;
                let v = rest.get(2).ok_or(no_form(r))?;
                let n = o.value("ang", v, r, "deg")?;
                o.rows.push(format!("angle({a}, {b}, {n})"));
            }
            _ => return Err(no_form(r)),
        }
        Ok(())
    }
}

fn no_form(row: &str) -> ToolError {
    ToolError::NoForgeForm {
        tool: "sketch",
        why: format!("the row `{row}` has no Forge form here; write it in the source"),
    }
}

/// The Forge block for a sketch named `name` on `on` (an anchor, a sketch, or `xy`/`xz`/`yz`):
/// its params, then the block.
pub fn block(name: &str, on: &str, rows: &[String]) -> Result<String, ToolError> {
    let mut o = Out::new(name, HashSet::new());
    o.rows_from(rows, 0, &[])?;
    let mut out: String = o.params.concat();
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

    fn check(src: &str) -> forge_lang::Run {
        let opts = forge_lang::Options {
            build: false,
            ..forge_lang::Options::default()
        };
        forge_lang::run(src, opts, Default::default()).0
    }

    #[test]
    fn drawn_rows_become_a_forge_sketch_with_its_sizes_as_params() {
        let rows: Vec<String> = [
            "rect 0 0 40 30",
            "hdist l1.a l1.b 42",
            "circle 20 15 4",
            "dia c1 10",
            "line 50 0 60 10",
            "hor l5",
            "dist l5 12",
            "pt 5 5",
        ]
        .map(String::from)
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
        let t = block("sk1", "xy", &["rect 0 0 4 3".into(), "hor l2".into()]).unwrap();
        assert!(t.contains("horizontal(r1[1])"), "{t}");
        // a row with no Forge form is refused, typed
        let e = block("sk1", "xy", &["project @e12".into()]).unwrap_err();
        assert_eq!(e.kind(), "NoForgeForm");
    }

    #[test]
    fn concentric_collinear_symmetric_and_construction_rows_become_forge() {
        let rows: Vec<String> = [
            "circle 0 0 10",
            "circle 1 1 4",
            "conc c1 c2",
            "line 0 -20 0 20 cons",
            "line 20 0 30 0",
            "line -22 0 -31 1",
            "sym l2.a l3.a l1",
            "sym l2.b l3.b l1",
            "line 40 0 50 0",
            "colin l2 l4",
            "poly 60 0 70 0 65 8",
            "ellipse 100 0 110 0 4",
        ]
        .map(String::from)
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
        let rows: Vec<String> = [
            "line 0 0 20 0",
            "line 20 0 20 15",
            "co l1.b l2.a",
            "perp l1 l2",
            "hor l1",
            "arc 30 30 35 30 30 35",
            "spline 40 0 45 5 50 0 open",
            "pt 5 20",
            "dist l1 20",
            "ang l1 l2 90",
        ]
        .map(String::from)
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
