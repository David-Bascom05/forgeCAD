//! Edit Feature on a Forge part: a statement read back into the tool's form that writes it. The
//! form is found, not hand-written per kind: each kind's forms (its choices, flags, optional
//! picks) write a template with markers for picks, words and params, and the one whose
//! template matches the statement token for token gives the picks and values. What no form
//! writes is refused, typed, never half-read.

use super::fg::{parse_value, short};
use super::fgtool::{Choice, FgTool, KINDS, ToolError, choices_of, form_of};
use forge_lang::syntax::{Tok, TokKind, lex};

/// What a template token stands for.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Hole {
    Pick(usize, usize),
    /// A slot's picks in a row, a comma between each: one run, read as a list.
    Picks(usize),
    Word(usize),
    Param(usize),
}

fn refuse(why: String) -> ToolError {
    ToolError::NoForgeForm {
        tool: "Edit Feature",
        why,
    }
}

fn toks(text: &str) -> Option<Vec<Tok>> {
    let t = lex(text).ok()?;
    Some(
        t.into_iter()
            .filter(|t| !matches!(t.kind, TokKind::Nl | TokKind::Eof))
            .collect(),
    )
}

fn opens(t: &Tok) -> bool {
    matches!(t.kind, TokKind::Op("(" | "[" | "{"))
}

fn closes(t: &Tok) -> bool {
    matches!(t.kind, TokKind::Op(")" | "]" | "}"))
}

/// The text's own parts split where `sep` stands outside brackets.
fn split_top(text: &str, sep: &'static str) -> Vec<String> {
    let Some(ts) = toks(text) else {
        return vec![text.trim().to_string()];
    };
    let mut out = vec![];
    let (mut depth, mut from) = (0i32, 0usize);
    for t in &ts {
        if opens(t) {
            depth += 1;
        } else if closes(t) {
            depth -= 1;
        } else if depth == 0 && t.kind == TokKind::Op(sep) {
            out.push(text[from..t.span.lo as usize].trim().to_string());
            from = t.span.hi as usize;
        }
    }
    out.push(text[from..].trim().to_string());
    out
}

struct Matcher<'a> {
    holes: Vec<Option<Hole>>,
    t: &'a [Tok],
    o: &'a [Tok],
    src: &'a str,
    got: Vec<(Hole, String)>,
}

impl Matcher<'_> {
    /// Template from `i` against the original from `j`, every token used.
    fn go(&mut self, i: usize, j: usize) -> bool {
        if i == self.t.len() {
            return j == self.o.len();
        }
        let Some(h) = self.holes[i] else {
            return j < self.o.len() && same(&self.t[i], &self.o[j]) && self.go(i + 1, j + 1);
        };
        // a hole takes the shortest balanced run that lets the rest match; never a bare comma
        let mut depth = 0i32;
        for e in j..self.o.len() {
            let tok = &self.o[e];
            if opens(tok) {
                depth += 1;
            } else if closes(tok) {
                depth -= 1;
                if depth < 0 {
                    return false;
                }
            } else if depth == 0 && tok.kind == TokKind::Op(",") && !matches!(h, Hole::Picks(_)) {
                return false;
            }
            if depth == 0 {
                let n = self.got.len();
                let text = self.src[self.o[j].span.lo as usize..tok.span.hi as usize].to_string();
                self.got.push((h, text));
                if self.go(i + 1, e + 1) {
                    return true;
                }
                self.got.truncate(n);
            }
        }
        false
    }
}

fn same(a: &Tok, b: &Tok) -> bool {
    match (&a.kind, &b.kind) {
        (TokKind::Num(x, u), TokKind::Num(y, v)) => x == y && u == v,
        (x, y) => x == y,
    }
}

/// The holes of a template's tokens: markers for picks and words, and the tool's params where
/// they stand as values (not as a keyword's name).
fn holes(t: &[Tok], params: &[&str]) -> Vec<Option<Hole>> {
    (0..t.len())
        .map(|i| match &t[i].kind {
            TokKind::Id(w) => {
                if let Some(rest) = w.strip_prefix("zzs") {
                    let (s, k) = rest.split_once('_')?;
                    return Some(Hole::Pick(s.parse().ok()?, k.parse().ok()?));
                }
                let key = t.get(i + 1).is_some_and(|n| n.kind == TokKind::Op(":"));
                if key {
                    return None;
                }
                params.iter().position(|p| p == w).map(Hole::Param)
            }
            TokKind::Str(w) => w.strip_prefix("zzw")?.parse().ok().map(Hole::Word),
            _ => None,
        })
        .collect()
}

/// A slot's markers in a row (`a, b` of a loft's list) made one run, which takes any count.
fn runs(t: Vec<Tok>, holes: Vec<Option<Hole>>) -> (Vec<Tok>, Vec<Option<Hole>>) {
    let (mut ot, mut oh) = (vec![], vec![]);
    let mut i = 0;
    while i < t.len() {
        if let Some(Hole::Pick(k, 0)) = holes[i] {
            let mut e = i;
            while e + 2 < t.len()
                && t[e + 1].kind == TokKind::Op(",")
                && matches!(holes[e + 2], Some(Hole::Pick(x, _)) if x == k)
            {
                e += 2;
            }
            if e > i {
                ot.push(t[i].clone());
                oh.push(Some(Hole::Picks(k)));
                i = e + 1;
                continue;
            }
        }
        ot.push(t[i].clone());
        oh.push(holes[i]);
        i += 1;
    }
    (ot, oh)
}

/// A call's arguments matched: each name against the same name, the holes' texts.
fn match_call(
    tmpl: &[(String, String)],
    orig: &[(String, String)],
    params: &[&str],
) -> Option<Vec<(Hole, String)>> {
    if tmpl.len() != orig.len() {
        return None;
    }
    let mut got = vec![];
    for (name, tt) in tmpl {
        let (_, ot) = orig.iter().find(|o| o.0 == *name)?;
        let (t, o) = (toks(tt)?, toks(ot)?);
        let h = holes(&t, params);
        let (t, h) = runs(t, h);
        let mut m = Matcher {
            holes: h,
            t: &t,
            o: &o,
            src: ot,
            got: vec![],
        };
        if !m.go(0, 0) {
            return None;
        }
        got.extend(m.got);
    }
    Some(got)
}

/// Every way a kind's form can be: its choices' items, its flags, its optional picks given or
/// not, and its params at their defaults or not (a default a statement leaves out: no taper).
fn forms(kind: &'static str) -> Vec<FgTool> {
    let base = choices_of(kind);
    let mut combos: Vec<Vec<Choice>> = vec![base.clone()];
    for (k, c) in base.iter().enumerate() {
        combos = combos
            .into_iter()
            .flat_map(|cs| {
                (0..c.items.len()).map(move |on| {
                    let mut cs = cs.clone();
                    cs[k].on = on;
                    cs
                })
            })
            .collect();
    }
    let mut out = vec![];
    for choices in combos {
        let pick = |n: &str| choices.iter().find(|c| c.name == n).map_or("", |c| c.word());
        let Some((slots, params, words, flags)) = form_of(kind, &pick) else {
            continue;
        };
        let opt: Vec<usize> = (0..slots.len()).filter(|&k| slots[k].optional).collect();
        for fl in 0..(1usize << flags.len()) {
            for op in 0..(1usize << opt.len()) {
                for odd in [false, true] {
                    let mut t = FgTool::new(kind, &[]).expect("a kind with a form");
                    t.choices = choices.clone();
                    t.slots = slots.clone();
                    t.params = params.clone();
                    t.words = words.clone();
                    t.flags = flags.clone();
                    for (k, f) in t.flags.iter_mut().enumerate() {
                        f.1 = fl & (1 << k) != 0;
                    }
                    for (k, &s) in opt.iter().enumerate() {
                        if op & (1 << k) == 0 {
                            t.slots[s].picks = vec![];
                        } else {
                            t.slots[s].picks = vec!["x".into()];
                        }
                    }
                    if odd {
                        for p in &mut t.params {
                            p.text = "7.25".into();
                        }
                    }
                    out.push(t);
                }
            }
        }
    }
    out
}

/// The form with markers in it: its text, or none if no count of picks makes one.
fn template(t: &mut FgTool, id: &str) -> Option<String> {
    t.name = id.into();
    t.edit = Some(id.into());
    for (k, w) in t.words.iter_mut().enumerate() {
        w.1 = format!("zzw{k}");
    }
    let mut counts: Vec<usize> = t
        .slots
        .iter()
        .map(|s| usize::from(!s.picks.is_empty() || !s.optional))
        .collect();
    for _ in 0..12 {
        for (k, s) in t.slots.iter_mut().enumerate() {
            s.picks = (0..counts[k]).map(|j| format!("zzs{k}_{j}")).collect();
        }
        match t.text() {
            Ok(text) => return Some(text),
            Err(ToolError::TooFew { slot, want }) => {
                let k = t.slots.iter().position(|s| s.label == slot)?;
                if counts[k] >= want {
                    return None;
                }
                counts[k] = want;
            }
            Err(_) => return None,
        }
    }
    None
}

/// The right side of `param ID.NAME = ...` as the file writes it.
fn own_param<'s>(src: &'s str, id: &str, name: &str) -> Option<&'s str> {
    let head = format!("param {id}.{name}");
    src.lines().find_map(|l| {
        let rest = l.trim().strip_prefix(head.as_str())?.trim_start();
        Some(rest.strip_prefix('=')?.trim())
    })
}

/// A param's field from what the statement wrote: a number as itself, the feature's own param
/// by what it's set to, anything else kept as written. True: it's a plain number.
fn field(p: &mut super::fgtool::ToolParam, text: &str, id: &str, src: &str) -> bool {
    let text = text.trim();
    let written = own_param(src, id, text).unwrap_or(text);
    match parse_value(written, p.dim).filter(|l| l.dim == p.dim) {
        Some(l) => {
            p.text = short(l.value);
            p.expr = None;
            true
        }
        None => {
            p.text = written.to_string();
            p.expr = Some(written.to_string());
            false
        }
    }
}

impl FgTool {
    /// The tool's form for statement `id` as the part writes it, to edit it in its place.
    pub fn edit(id: &str, src: &str) -> Result<FgTool, ToolError> {
        let Some((func, args)) = super::sheet::call_args(src, id) else {
            return Err(refuse(format!(
                "{id} isn't written as one call, so no dialog holds it"
            )));
        };
        let mut seen = false;
        // of every reading, the one with the fewest values that aren't plain numbers: `sym(6)` is a
        // symmetric 6, not a distance written `sym(6)`
        let mut best: Option<(usize, FgTool)> = None;
        for kind in KINDS {
            for mut t in forms(kind) {
                let Some(text) = template(&mut t, id) else {
                    continue;
                };
                let Some((tf, targs)) = super::sheet::call_args(&text, id) else {
                    continue;
                };
                if tf != func {
                    continue;
                }
                seen = true;
                let names: Vec<&str> = t.params.iter().map(|p| p.name).collect();
                let Some(got) = match_call(&targs, &args, &names) else {
                    continue;
                };
                // the form found: its picks, words and values from the statement
                let defaults = forms(kind)
                    .into_iter()
                    .find(|d| d.choices == t.choices)
                    .map(|d| d.params)
                    .unwrap_or_default();
                for (k, s) in t.slots.iter_mut().enumerate() {
                    if let Some(g) = got.iter().find(|g| g.0 == Hole::Picks(k)) {
                        s.picks = split_top(&g.1, ",");
                        continue;
                    }
                    let mine: Vec<&String> = got
                        .iter()
                        .filter(|g| matches!(g.0, Hole::Pick(x, _) if x == k))
                        .map(|g| &g.1)
                        .collect();
                    s.picks = match mine.as_slice() {
                        [] => vec![],
                        [one] if s.kind.many() => {
                            let one = one.trim();
                            if one.starts_with('[') && one.ends_with(']') {
                                split_top(&one[1..one.len() - 1], ",")
                            } else {
                                split_top(one, "+")
                            }
                        }
                        many => many.iter().map(|m| m.trim().to_string()).collect(),
                    };
                }
                for (k, w) in t.words.iter_mut().enumerate() {
                    if let Some(g) = got.iter().find(|g| g.0 == Hole::Word(k)) {
                        w.1 = g.1.trim().trim_matches('"').to_string();
                    }
                }
                let mut odd = 0;
                for (k, p) in t.params.iter_mut().enumerate() {
                    match got.iter().find(|g| g.0 == Hole::Param(k)) {
                        Some(g) => odd += usize::from(!field(p, &g.1, id, src)),
                        None => {
                            if let Some(d) = defaults.iter().find(|d| d.name == p.name) {
                                p.text = d.text.clone();
                            }
                            p.expr = None;
                        }
                    }
                }
                t.active = t
                    .slots
                    .iter()
                    .position(|s| s.picks.is_empty() && !s.optional)
                    .unwrap_or(0);
                t.error = None;
                if best.as_ref().is_none_or(|b| odd < b.0) {
                    best = Some((odd, t));
                }
            }
        }
        if let Some((_, t)) = best {
            return Ok(t);
        }
        Err(refuse(if seen {
            format!(
                "{id} = {func}(..) is written in a form its dialog doesn't write (only the .fg file's text holds it)"
            )
        } else {
            format!("{func} has no dialog in the app yet")
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(src: &str, id: &str) -> FgTool {
        FgTool::edit(id, src).unwrap_or_else(|e| panic!("{id}: {}", e.text()))
    }

    #[test]
    fn a_tools_own_text_reads_back_into_its_form() {
        let mut t = FgTool::new("fillet", &[]).unwrap();
        t.take(Ok("edges(between: (a.end, a.side(s.c)))".into()));
        t.take(Ok("b.end.edges".into()));
        t.params[0].text = "3".into();
        let text = t.text().unwrap();
        let e = read(&text, "fillet1");
        assert_eq!(e.kind, "fillet");
        assert_eq!(e.slots[0].picks, t.slots[0].picks);
        assert_eq!(e.params[0].text, "3");
        assert_eq!(e.text().unwrap(), text);
    }

    #[test]
    fn a_hand_written_extrude_takes_its_literal_and_its_choice() {
        let e = read("plate = cut(plan.outline, sym(6))\n", "plate");
        assert_eq!(e.kind, "extrude");
        assert_eq!(e.choice("Extents"), "Symmetric");
        assert_eq!(e.choice("Output"), "Cut");
        assert_eq!(e.slots[0].picks, ["plan.outline"]);
        assert_eq!(e.params[0].text, "6");
        assert_eq!(
            e.op(e.text().unwrap()),
            forge_lang::error::Op::ReplaceFeature {
                id: "plate".into(),
                text: "param plate.d = 6\nplate = cut(plan.outline, sym(d))\n".into()
            }
        );
    }

    #[test]
    fn a_shared_param_is_kept_as_written() {
        let e = read("shared t = 4\nplate = extrude(plan, t)\n", "plate");
        assert_eq!(e.params[0].expr.as_deref(), Some("t"));
        assert_eq!(e.text().unwrap(), "param plate.d = t\nplate = extrude(plan, d)\n");
    }

    #[test]
    fn several_picks_and_a_left_out_default() {
        let e = read("e = fillet(a.edges + b.edges, 2, only: true)\n", "e");
        assert_eq!(e.slots[0].picks, ["a.edges", "b.edges"]);
        assert!(!e.flag("tangent chain"));
        let l = read("l = loft([p.a, p.b, p.c], smooth: true)\n", "l");
        assert_eq!(l.slots[0].picks, ["p.a", "p.b", "p.c"]);
        assert!(l.flag("smooth"));
    }

    #[test]
    fn what_no_dialog_writes_is_refused_typed() {
        let e = FgTool::edit("p", "p = extrude(plan, 6, wobble: 2)\n").unwrap_err();
        assert_eq!(e.kind(), "NoForgeForm");
        assert!(e.text().contains("form its dialog doesn't write"), "{}", e.text());
        let e = FgTool::edit("q", "q = frobnicate(plan)\n").unwrap_err();
        assert!(e.text().contains("no dialog"), "{}", e.text());
    }
}
