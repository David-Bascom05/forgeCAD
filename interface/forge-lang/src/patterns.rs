//! Patterns as features: copies of earlier features (or the whole part) recorded and edited as
//! one feature, laid out by a layout value (`lin`, `circ`, `mirror`, `follow`, `places`). The
//! copies' faces are named from the pattern's id and each copy's index, so ids hold when the
//! count changes (the surviving copies keep theirs).

use crate::builtins::{ANG, Args, LEN, axis_of, dim, dir_of, not_in_sketch, plane_of, point_ent, point_of};
use crate::eval::{Dimension, Eval, R, Value};
use crate::syntax::{Arg, Span};
use crate::values::Ty;
use crate::wired::{W, conflict, count, feat, one_form, wired};
use forge_math::dmath;
use forge_model::{At, Feature, Layout, Row, Source};

pub fn pattern(ev: &mut Eval, args: &[Arg], span: Span) -> R<Value> {
    not_in_sketch(ev, "pattern", span)?;
    let id = ev.take_id("pattern", span)?;
    let a = Args::bind(ev, "pattern", &["of", "layout", "whole", "bodies"], args, span)?;
    if a.poisoned() {
        return Ok(Value::Unresolved);
    }
    let form = one_form(&a, "pattern", &["of", "whole"], span)?;
    let of = if form == "whole" {
        if !a.flag("whole")? {
            return Err(crate::error::Error::BadValue {
                span: a.span("whole"),
                what: "whole (true, or leave it out and give of:)",
                value: 0.0,
            });
        }
        Source::Body
    } else {
        match a.req("of")? {
            // sheet cutouts: copied below, once the layout is read
            Value::Sheet(_) => Source::Body,
            v => {
                let fs = crate::builtins::feats_of(v, a.span("of"))?;
                Source::Features(fs.iter().map(|f| f.idx).collect())
            }
        }
    };
    let (layout, dims) = match a.req("layout")? {
        Value::Wired(w) => match &*w {
            W::Layout(l, d) => (l.clone(), d.clone()),
            o => {
                return Err(crate::error::Error::WrongType {
                    span: a.span("layout"),
                    what: "a layout (lin, circ, mirror, follow, places)",
                    want: Ty::Layout,
                    found: o.ty(),
                });
            }
        },
        o => {
            return Err(crate::error::Error::WrongType {
                span: a.span("layout"),
                what: "a layout (lin, circ, mirror, follow, places)",
                want: Ty::Layout,
                found: o.ty(),
            });
        }
    };
    // copies of sheet cutouts are cuts in their walls (sheet.rs)
    if let Some(v @ Value::Sheet(_)) = a.opt("of") {
        return crate::sheet::pattern(ev, id, v, a.span("of"), &layout, dims, span);
    }
    let f = crate::bodies::wrap_in(ev, &a, Feature::Pattern { of, layout })?;
    let (idx, hid) = ev.push_feature(&id, "pattern", f, span, dims)?;
    Ok(feat(id, "pattern", idx, hid, None))
}

pub fn layout(ev: &mut Eval, name: &'static str, args: &[Arg], span: Span) -> R<Value> {
    let names: &[&'static str] = match name {
        "lin" => &["along", "n", "step", "along2", "n2", "step2"],
        "circ" => &["about", "n", "over", "each"],
        "mirror" => &["on"],
        "follow" => &["curve", "n", "step", "fixed", "reverse"],
        _ => &["at"],
    };
    let a = Args::bind(ev, name, names, args, span)?;
    if a.poisoned() {
        return Ok(Value::Unresolved);
    }
    let mut dims: Vec<Dimension> = vec![];
    let req_count = |a: &Args, k: &'static str, dims: &mut Vec<Dimension>| -> R<usize> {
        match count(a, k)? {
            Some((n, num)) => {
                dims.push(dim("count", &num));
                Ok(n)
            }
            None => Err(crate::error::Error::MissingArg {
                span,
                func: name,
                arg: k,
            }),
        }
    };
    let layout = match name {
        "lin" => {
            let mut row = |ev: &mut Eval, d: &'static str, n: &'static str, s: &'static str| -> R<Row> {
                let v = a.req(d)?;
                ev.world_check(matches!(v, Value::WorldDir(_)), a.span(d))?;
                let dir = dir_of(v, a.span(d))?
                    .normalize()
                    .ok_or(crate::error::Error::BadValue {
                        span: a.span(d),
                        what: "a zero direction",
                        value: 0.0,
                    })?
                    .get();
                let count = req_count(&a, n, &mut dims)?;
                let step = a.req_num(s, LEN)?;
                dims.push(dim("spacing", &step));
                Ok(Row {
                    dir,
                    count,
                    spacing: step.v,
                })
            };
            let first = row(ev, "along", "n", "step")?;
            let any2 = ["along2", "n2", "step2"].iter().any(|k| a.opt(k).is_some());
            let second = if any2 {
                Some(row(ev, "along2", "n2", "step2")?)
            } else {
                None
            };
            Layout::Linear { first, second }
        }
        "circ" => {
            conflict(&a, name, "over", "each")?;
            let (o, d) = axis_of(ev, a.req("about")?, a.span("about"))?;
            let count = req_count(&a, "n", &mut dims)?;
            let (angle, incremental) = match (a.num("over", ANG)?, a.num("each", ANG)?) {
                (Some(t), _) => {
                    dims.push(dim("angle", &t));
                    (t.v, false)
                }
                (None, Some(t)) => {
                    dims.push(dim("step angle", &t));
                    (t.v, true)
                }
                (None, None) => (dmath::TAU, false),
            };
            Layout::Circular {
                origin: o,
                axis: d.get(),
                count,
                angle,
                incremental,
            }
        }
        "mirror" => Layout::Mirror {
            plane: plane_of(ev, a.req("on")?, a.span("on"), "on")?,
        },
        "follow" => {
            let along = match a.req("curve")? {
                Value::Edges(es) => {
                    let mut out = vec![];
                    for &e in es.iter() {
                        let r = crate::work::owner(ev, e, false, a.span("curve"))?;
                        if !out.contains(&r) {
                            out.push(r);
                        }
                    }
                    out
                }
                v => vec![crate::work::to_ref(ev, v, a.span("curve"))?],
            };
            let count = req_count(&a, "n", &mut dims)?;
            let spacing = match a.num("step", LEN)? {
                Some(s) => {
                    dims.push(dim("spacing", &s));
                    Some(s.v)
                }
                None => None,
            };
            Layout::Path {
                along,
                count,
                spacing,
                turn: !a.flag("fixed")?,
                reverse: a.flag("reverse")?,
            }
        }
        _ => {
            let pts: Vec<Value> = crate::wired::items(a.req("at")?);
            let mut at: Vec<At> = vec![];
            for p in pts {
                match &p {
                    Value::Ent(e) => {
                        let id = point_ent(e, a.span("at"))?;
                        let s = e.sk.borrow();
                        let w = s.world(s.pos(id));
                        let n = crate::work::point_name(&s, id).unwrap_or_default();
                        // points of one sketch in a row are one entry
                        match at.last_mut() {
                            Some(At::Sketch {
                                feature,
                                points,
                                at: ps,
                                ..
                            }) if *feature == s.idx => {
                                points.push(n);
                                ps.push(w);
                            }
                            _ => at.push(At::Sketch {
                                feature: s.idx,
                                points: vec![n],
                                centers: false,
                                at: vec![w],
                            }),
                        }
                    }
                    Value::Wired(w) if w.work_ref().is_some() && w.point().is_some() => {
                        at.push(At::Ref(w.work_ref().expect("checked")))
                    }
                    v => at.push(At::Xyz(point_of(ev, v.clone(), a.span("at"))?)),
                }
            }
            Layout::Points(at)
        }
    };
    Ok(wired(W::Layout(layout, dims)))
}
