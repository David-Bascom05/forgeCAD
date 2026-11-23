//! Text in a sketch: `text("FORGE", at: (x, y), size: s, font:, bold:, italic:, align:, angle:)`
//! (or a list of lines, `text(["FORGE", "CAD"], ..)`) sets the string in the font (`forge_font::layout`) and draws each glyph's contours as fixed
//! sketch geometry, exactly as the font holds them: a line stays a line, a TrueType quadratic is
//! a degree-2 control spline on its three points, a CFF cubic a degree-3 one on its four. Its
//! regions are what any sketch's are (`forge_sketch::profiles`): each outside contour with the
//! counters inside it as holes; contours that cross are refused, never joined.
//!
//! The value is a list with one loop per drawn character, in string order (blanks draw
//! nothing): `extrude(sk.label, d)` raises every glyph, `sk.label[k]` is the k-th drawn one.
//!
//! Names: a named text's segment `s` of contour `c` of the `n`-th occurrence of character `ch` in
//! the string is `label.ch#n.c.s`, so its faces keep their names while other characters are put
//! in or taken out, the text is moved, turned or resized, or its alignment changes; reordering
//! the same character, or another font or style, renames what it changes.

use crate::builtins::{ANG, Args, LEN, in_sketch, num_of, pending_name, positive};
use crate::error::{EntKind, Error};
use crate::eval::{Ent, Eval, R, Value};
use crate::feat_error::{FeatError, TextProblem};
use crate::syntax::{Arg, Span};
use crate::values::Ty;
use forge_font::Seg;
use forge_font::layout::{Align, LayoutError, lay_out};
use forge_math::{Point2, dmath};
use forge_sketch::EntityId;
use std::rc::Rc;

pub fn text(ev: &mut Eval, args: &[Arg], span: Span) -> R<Value> {
    let sk = in_sketch(ev, "text", span)?;
    let a = Args::bind(
        ev,
        "text",
        &[
            "text", "at", "size", "font", "bold", "italic", "align", "angle", "spacing",
        ],
        args,
        span,
    )?;
    if a.poisoned() {
        return Ok(Value::Unresolved);
    }
    // one line, or a list of lines (a string holds no line break)
    let string = match a.req("text")? {
        Value::List(xs) => {
            let mut lines = Vec::with_capacity(xs.len());
            for x in xs.iter() {
                match x {
                    Value::Str(l) => lines.push(l.to_string()),
                    o => {
                        return Err(Error::WrongType {
                            span: a.span("text"),
                            what: "a text's lines",
                            want: Ty::Str,
                            found: o.ty(),
                        });
                    }
                }
            }
            lines.join("\n")
        }
        _ => crate::media::string(&a, "text")?,
    };
    let size = a.req_num("size", LEN)?;
    positive(&size, a.span("size"), "a text's size")?;
    // where the baseline starts: coordinates, which the text is fixed to (a solved point would
    // move after the glyphs were set)
    let at = match a.opt("at") {
        None => Point2::new(0.0, 0.0),
        Some(Value::Tuple(t)) if t.len() == 2 => {
            let x = num_of(t[0].clone(), LEN, a.span("at"), "x")?;
            let y = num_of(t[1].clone(), LEN, a.span("at"), "y")?;
            Point2::new(x.v, y.v)
        }
        Some(o) => {
            return Err(Error::WrongType {
                span: a.span("at"),
                what: "a text's start (x, y)",
                want: Ty::Vec2,
                found: o.ty(),
            });
        }
    };
    let angle = a.num("angle", ANG)?.map_or(0.0, |n| n.v);
    // a length added after each character (but a line's last), beyond the font's advance
    let spacing = a.num("spacing", LEN)?.map_or(0.0, |n| n.v);
    let family = match a.opt("font") {
        None => crate::fonts::DEFAULT.to_string(),
        Some(_) => crate::media::string(&a, "font")?,
    };
    let (bold, italic) = (a.flag("bold")?, a.flag("italic")?);
    let align = match crate::wired::word(&a, "align", &["left", "center", "right"])? {
        None | Some("left") => Align::Left,
        Some("center") => Align::Center,
        _ => Align::Right,
    };
    let font = crate::media::font_for(
        ev.dir.as_deref(),
        ev.root.as_deref(),
        &family,
        bold,
        italic,
        a.span("font"),
    )?;
    let fail = |problem| -> Error { FeatError::Text { span, problem }.into() };
    let placed =
        lay_out(&font, &string, align, spacing / size.v * font.units_per_em).map_err(|e| match e {
            LayoutError::Font(f) => fail(TextProblem::Font(format!("{f:?}"))),
            LayoutError::NoGlyph { index, ch } => FeatError::Text {
                span: a.span("text"),
                problem: TextProblem::NoGlyph { index, ch },
            }
            .into(),
            LayoutError::NoLineSpacing => fail(TextProblem::NoLineSpacing),
        })?;
    if placed.is_empty() {
        return Err(fail(TextProblem::Empty));
    }
    // font units to the sketch: scaled to the size (the em), turned, moved to the start
    let k = size.v / font.units_per_em;
    let (s, c) = if angle == 0.0 {
        (0.0, 1.0)
    } else {
        dmath::sin_cos(angle)
    };
    let to = |p: Point2| {
        let (x, y) = (k * p.x, k * p.y);
        if angle == 0.0 {
            Point2::new(at.x + x, at.y + y)
        } else {
            Point2::new(at.x + (c * x - s * y), at.y + (s * x + c * y))
        }
    };
    let name = pending_name(ev, span);
    let mut out = Vec::with_capacity(placed.len());
    let mut sk_ref = sk.borrow_mut();
    for g in &placed {
        let mut segs: Vec<EntityId> = Vec::new();
        for (ci, (contour, &part)) in g.contours.iter().zip(&g.parts).enumerate() {
            let Some(first) = contour.first() else { continue };
            let start = sk_ref.point(to(first.start()), true, None, span)?;
            let mut at_pt = start;
            for (si, seg) in contour.iter().enumerate() {
                let end = if si + 1 == contour.len() && seg.end() == first.start() {
                    start
                } else {
                    sk_ref.point(to(seg.end()), true, None, span)?
                };
                let nm = name
                    .as_ref()
                    .map(|n| format!("{n}.{}#{}.{ci}.{si}", g.ch, g.occurrence));
                let e = match *seg {
                    Seg::Line(..) => sk_ref.line(at_pt, end, false, nm.as_deref(), span)?,
                    Seg::Quad(_, m, _) => {
                        let m = sk_ref.point(to(m), true, None, span)?;
                        sk_ref.bspline_on(&[at_pt, m, end], 2, None, None, nm.as_deref(), span)?
                    }
                    Seg::Cubic(_, m, n, _) => {
                        let m = sk_ref.point(to(m), true, None, span)?;
                        let n = sk_ref.point(to(n), true, None, span)?;
                        sk_ref.bspline_on(&[at_pt, m, n, end], 3, None, None, nm.as_deref(), span)?
                    }
                };
                segs.push(e);
                sk_ref.text_segs.push((e, g.index, part));
                at_pt = end;
            }
        }
        if segs.is_empty() {
            continue;
        }
        out.push(Value::Ent(Ent {
            sk: sk.clone(),
            id: segs[0],
            kind: EntKind::Loop,
            loop_segs: Rc::new(segs),
        }));
    }
    Ok(Value::List(Rc::new(out)))
}
