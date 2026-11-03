//! Direct edits of the part's faces as they stand: `delete` (faces removed, the gap closed by
//! their neighbours), `replace` (a face put on a work plane or another face's surface),
//! `moveface` (faces slid or turned, neighbours re-met).

use crate::builtins::{Args, not_in_sketch};
use crate::eval::{Eval, ExtentV, R, Value};
use crate::feat_error::{Because, FeatError};
use crate::syntax::{Arg, Span};
use crate::values::Ty;
use crate::wired::{W, feat};
use forge_model::{Edit, Feature, Motion, Step, Target};
use forge_topo::Name;

fn faces(a: &Args, k: &'static str) -> R<Vec<Name>> {
    match a.req(k)? {
        Value::Faces(f) => Ok(f.names.clone()),
        o => Err(crate::error::Error::WrongType {
            span: a.span(k),
            what: "faces",
            want: Ty::Faces,
            found: o.ty(),
        }),
    }
}

pub fn direct(ev: &mut Eval, name: &'static str, args: &[Arg], span: Span) -> R<Value> {
    not_in_sketch(ev, name, span)?;
    let id = ev.take_id(name, span)?;
    let names: &[&'static str] = match name {
        "delete" => &["faces", "bodies"],
        "replace" => &["face", "with", "bodies"],
        _ => &["faces", "by", "bodies"],
    };
    let a = Args::bind(ev, name, names, args, span)?;
    if a.poisoned() {
        return Ok(Value::Unresolved);
    }
    let edit = match name {
        "delete" => Edit::Delete {
            faces: faces(&a, "faces")?,
        },
        "replace" => {
            let face = match &faces(&a, "face")?[..] {
                [one] => *one,
                fs => {
                    return Err(FeatError::NotOne {
                        span: a.span("face"),
                        what: "the face replaced",
                        count: fs.len(),
                    }
                    .into());
                }
            };
            let with = match a.req("with")? {
                Value::Wired(w) if w.plane().is_some() && w.work_ref().is_some() => {
                    Target::Plane(w.work_ref().expect("checked").feature)
                }
                Value::Faces(f) => crate::holes::target_of(ev, &ExtentV::UpTo(f, 0.0), a.span("with"))?.0,
                o => {
                    return Err(FeatError::NotARef {
                        span: a.span("with"),
                        found: o.ty(),
                    }
                    .into());
                }
            };
            Edit::Replace { face, with }
        }
        _ => {
            let motion = match a.req("by")? {
                Value::Wired(w) => match *w {
                    W::Motion(Step::Translate(v)) => Motion::Translate(v),
                    W::Motion(Step::Rotate { origin, axis, angle }) => Motion::Rotate { origin, axis, angle },
                    W::Motion(Step::Frame(_)) => {
                        return Err(FeatError::NotApplicable {
                            span: a.span("by"),
                            func: name,
                            arg: "by",
                            because: Because::OtherForm,
                        }
                        .into());
                    }
                    ref o => {
                        return Err(crate::error::Error::WrongType {
                            span: a.span("by"),
                            what: "a motion (slide, turn)",
                            want: Ty::Motion,
                            found: o.ty(),
                        });
                    }
                },
                o => {
                    return Err(crate::error::Error::WrongType {
                        span: a.span("by"),
                        what: "a motion (slide, turn)",
                        want: Ty::Motion,
                        found: o.ty(),
                    });
                }
            };
            Edit::Move {
                faces: faces(&a, "faces")?,
                motion,
            }
        }
    };
    let f = crate::bodies::wrap_in(ev, &a, Feature::Edit(edit))?;
    let (idx, hid) = ev.push_feature(&id, name, f, span, vec![])?;
    Ok(feat(id, name, idx, hid, None))
}
