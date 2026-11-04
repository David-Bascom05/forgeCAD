//! A sketch drawn with the app's sketch tools, written as a Forge sketch block. The tools write
//! rows in the .fc sketch form (`line P P`, `rect X Y W H`, `dist l1 20`); this turns them into
//! Forge's (`l1 = line(near(x1, y2), near(x3, y4))`, `distance(l1, d5)`): points drawn are free
//! guesses (`near`), as in the drawing; every number (guess, size, dimension) is a param of the
//! sketch. A row with no Forge form is refused, typed.

use super::fgtool::ToolError;
use forge_doc::text::tokens;
use std::collections::HashMap;

struct Out {
    name: String,
    params: Vec<String>,
    rows: Vec<String>,
    /// What each .fc entity name (`l1`, `c2`, `p1`) is in Forge.
    names: HashMap<String, String>,
    counts: HashMap<char, usize>,
    /// Lines that are a rect's sides: (rect, side), for dimensions a fixed rect already holds.
    rect_sides: HashMap<String, (usize, usize)>,
    /// Each rect's size params, set by an `hdist`/`vdist` on its sides.
    rects: Vec<[String; 2]>,
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

impl Out {
    fn param(&mut self, base: &str, v: f64) -> String {
        let k = self.params.len() + 1;
        let n = format!("{base}{k}");
        self.params
            .push(format!("param {}.{n} = {}\n", self.name, fmt(v)));
        n
    }

    fn next(&mut self, kind: char) -> String {
        let c = self.counts.entry(kind).or_default();
        *c += 1;
        format!("{kind}{c}")
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
    let mut o = Out {
        name: name.to_string(),
        params: vec![],
        rows: vec![],
        names: HashMap::new(),
        counts: HashMap::new(),
        rect_sides: HashMap::new(),
        rects: vec![],
    };
    // a rect's own width and height: the dimensions on its sides, else as drawn
    let mut held: HashMap<(usize, usize), f64> = HashMap::new();
    let mut lines_seen = 0usize;
    // a circle's own radius: a dimension on it, else as drawn
    let mut radii: HashMap<String, f64> = HashMap::new();
    for r in rows {
        let t = tokens(r);
        match t.first().map(String::as_str) {
            Some(w @ ("rad" | "dia")) => {
                if let (Some(c), Some(v)) = (t.get(1), t.get(2))
                    && c.starts_with('c')
                    && !c.contains('.')
                {
                    let v = num(v, r)?;
                    radii.insert(c.clone(), if w == "dia" { v / 2.0 } else { v });
                }
            }
            Some("line") => lines_seen += 1,
            Some("poly") => {
                let open = t.last().is_some_and(|w| w == "open");
                let (mut k, mut pts) = (1, 0);
                while k < t.len() - usize::from(open) {
                    k += if t[k].parse::<f64>().is_ok() { 2 } else { 1 };
                    pts += 1;
                }
                lines_seen += pts - usize::from(open);
            }
            Some("rect") => {
                let first = lines_seen + 1;
                let rect = o.rects.len();
                for side in 0..4 {
                    o.rect_sides.insert(format!("l{}", first + side), (rect, side));
                }
                lines_seen += 4;
                o.rects.push([String::new(), String::new()]);
            }
            Some(w @ ("hdist" | "vdist")) => {
                let side = |p: &str| p.split_once('.').map(|x| x.0.to_string());
                if let (Some(a), Some(b), Some(v)) = (
                    t.get(1).and_then(|x| side(x)),
                    t.get(2).and_then(|x| side(x)),
                    t.get(3),
                ) && a == b
                    && let Some(&(rect, s)) = o.rect_sides.get(&a)
                {
                    let k = usize::from(w == "vdist" || s % 2 == 1);
                    held.insert((rect, k), num(v, r)?);
                }
            }
            _ => {}
        }
    }
    let mut rect_no = 0usize;
    for r in rows {
        let mut t = tokens(r);
        // a construction line is written as one; other construction geometry has no Forge form
        let cons = t.last().is_some_and(|w| w == "cons");
        if cons {
            t.pop();
        }
        let Some((w, rest)) = t.split_first() else {
            continue;
        };
        if cons && w != "line" {
            return Err(no_form(r));
        }
        match w.as_str() {
            "pt" => {
                let (p, _) = o.point(rest, r)?;
                let n = o.next('p');
                o.names.insert(n.clone(), n.clone());
                o.rows.push(format!("{n} = {p}"));
            }
            "line" => {
                let (a, k) = o.point(rest, r)?;
                let (b, _) = o.point(&rest[k..], r)?;
                let n = o.next('l');
                o.names.insert(n.clone(), n.clone());
                let c = if cons { ", construction: true" } else { "" };
                o.rows.push(format!("{n} = line({a}, {b}{c})"));
            }
            "rect" => {
                let v: Vec<f64> = rest.iter().map(|x| num(x, r)).collect::<Result<_, _>>()?;
                let [x, y, w0, h0] = v[..] else {
                    return Err(no_form(r));
                };
                let w = held.get(&(rect_no, 0)).copied().unwrap_or(w0);
                let h = held.get(&(rect_no, 1)).copied().unwrap_or(h0);
                let (px, py) = (o.param("x", x), o.param("y", y));
                let (pw, ph) = (o.param("w", w), o.param("h", h));
                o.rects[rect_no] = [pw.clone(), ph.clone()];
                let n = format!("r{}", rect_no + 1);
                o.rows
                    .push(format!("{n} = rect(corner: ({px}, {py}), size: ({pw}, {ph}))"));
                let first = o.counts.get(&'l').copied().unwrap_or(0) + 1;
                for side in 0..4 {
                    o.names
                        .insert(format!("l{}", first + side), format!("{n}[{side}]"));
                }
                *o.counts.entry('l').or_default() += 4;
                rect_no += 1;
            }
            "circle" => {
                let (c, k) = o.point(rest, r)?;
                let n = o.next('c');
                let rr = match radii.get(&n) {
                    Some(&v) => v,
                    None => num(rest.get(k).ok_or(no_form(r))?, r)?,
                };
                let pr = o.param("r", rr);
                o.names.insert(n.clone(), n.clone());
                o.rows.push(format!("{n} = circle({c}, {pr})"));
            }
            "arc" => {
                let (c, k1) = o.point(rest, r)?;
                let (s, k2) = o.point(&rest[k1..], r)?;
                let (e, k3) = o.point(&rest[k1 + k2..], r)?;
                let cw = rest.get(k1 + k2 + k3).is_some_and(|w| w == "cw");
                let n = o.next('a');
                o.names.insert(n.clone(), n.clone());
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
                let n = o.next('e');
                o.names.insert(n.clone(), n.clone());
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
                let n = o.next('s');
                o.names.insert(n.clone(), n.clone());
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
                let n = format!("o{}", o.counts.get(&'o').copied().unwrap_or(0) + 1);
                *o.counts.entry('o').or_default() += 1;
                let first = o.counts.get(&'l').copied().unwrap_or(0) + 1;
                let sides = ps.len() - usize::from(open);
                for side in 0..sides {
                    o.names
                        .insert(format!("l{}", first + side), format!("{n}[{side}]"));
                }
                *o.counts.entry('l').or_default() += sides;
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
                    let d = o.param("d", num(v, r)?);
                    o.rows.push(format!("distance({e}, {d})"));
                }
                [a, b, v] => {
                    let (a, b) = (o.reference(a, true, r)?, o.reference(b, false, r)?);
                    let d = o.param("d", num(v, r)?);
                    o.rows.push(format!("distance({a}, {b}, {d})"));
                }
                _ => return Err(no_form(r)),
            },
            // a circle holds its own radius
            "rad" | "dia" if rest.first().is_some_and(|c| radii.contains_key(c)) => {}
            "rad" | "dia" => {
                let e = o.reference(rest.first().ok_or(no_form(r))?, false, r)?;
                let v = num(rest.get(1).ok_or(no_form(r))?, r)?;
                let d = o.param("r", if w == "dia" { v / 2.0 } else { v });
                o.rows.push(format!("radius({e}, {d})"));
            }
            "ang" => {
                let a = o.reference(rest.first().ok_or(no_form(r))?, false, r)?;
                let b = o.reference(rest.get(1).ok_or(no_form(r))?, false, r)?;
                let v = num(rest.get(2).ok_or(no_form(r))?, r)?;
                let k = o.params.len() + 1;
                let n = format!("ang{k}");
                o.params.push(format!("param {}.{n} = {}deg\n", o.name, fmt(v)));
                o.rows.push(format!("angle({a}, {b}, {n})"));
            }
            _ => return Err(no_form(r)),
        }
    }
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
