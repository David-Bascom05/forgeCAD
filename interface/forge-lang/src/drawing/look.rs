//! `fg view`: a model's hidden-line views on their own, for checking: lengths by class and
//! visibility, the pieces as points, one page per view.

use super::Model;
use forge_agent::json::Json;
use forge_draw::{Class, Geom2, Options, Page, Pen, Piece, Place, Stroke, Text, View};
use forge_math::Point2;

pub struct Look {
    pub name: String,
    pub pieces: Vec<forge_draw::Piece>,
    pub ms: f64,
}

pub fn views(model: &Model, names: &[&str], opts: &Options) -> Result<Vec<Look>, String> {
    let insts = model.instances();
    names
        .iter()
        .map(|name| {
            let view = View::named(name)
                .ok_or_else(|| format!("no view {name}: {}", forge_draw::view::NAMED.join(" ")))?;
            let t = std::time::Instant::now();
            let pieces = forge_draw::hlr(&insts, &view, opts);
            Ok(Look {
                name: name.to_string(),
                pieces,
                ms: t.elapsed().as_secs_f64() * 1e3,
            })
        })
        .collect()
}

fn class_word(c: Class) -> &'static str {
    match c {
        Class::Sharp => "sharp",
        Class::Tangent => "tangent",
        Class::Outline => "outline",
    }
}

/// Seen and hidden length by class, and with `points` every piece as points within `chord`.
pub fn json(looks: &[Look], points: Option<f64>) -> Json {
    Json::Arr(
        looks
            .iter()
            .map(|l| {
                let mut f = vec![
                    ("view", Json::from(l.name.as_str())),
                    ("pieces", l.pieces.len().into()),
                    ("ms", l.ms.into()),
                ];
                for vis in [true, false] {
                    for c in [Class::Sharp, Class::Tangent, Class::Outline] {
                        let len: f64 = l
                            .pieces
                            .iter()
                            .filter(|p| p.visible == vis && p.class == c)
                            .map(|p| p.geom.length())
                            .sum();
                        let key: &'static str = match (vis, c) {
                            (true, Class::Sharp) => "seen_sharp",
                            (true, Class::Tangent) => "seen_tangent",
                            (true, Class::Outline) => "seen_outline",
                            (false, Class::Sharp) => "hidden_sharp",
                            (false, Class::Tangent) => "hidden_tangent",
                            (false, Class::Outline) => "hidden_outline",
                        };
                        f.push((key, len.into()));
                    }
                }
                if let Some(chord) = points {
                    let ps = l
                        .pieces
                        .iter()
                        .map(|p| {
                            Json::obj(vec![
                                ("visible", p.visible.into()),
                                ("class", class_word(p.class).into()),
                                (
                                    "points",
                                    Json::Arr(
                                        p.geom
                                            .points(chord)
                                            .iter()
                                            .map(|q| Json::Arr(vec![q.x.into(), q.y.into()]))
                                            .collect(),
                                    ),
                                ),
                            ])
                        })
                        .collect();
                    f.push(("polylines", Json::Arr(ps)));
                }
                Json::obj(f)
            })
            .collect(),
    )
}

/// Each view on its own A3 page, fitted, its name below it.
pub fn pages(looks: &[Look]) -> Vec<Page> {
    looks
        .iter()
        .map(|l| {
            let (w, h) = (420.0, 297.0);
            let mut lo = Point2::new(f64::INFINITY, f64::INFINITY);
            let mut hi = Point2::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
            for p in &l.pieces {
                let (a, b) = p.geom.bounds();
                lo = Point2::new(lo.x.min(a.x), lo.y.min(a.y));
                hi = Point2::new(hi.x.max(b.x), hi.y.max(b.y));
            }
            let s = ((w - 40.0) / (hi.x - lo.x).max(1e-9)).min((h - 50.0) / (hi.y - lo.y).max(1e-9));
            let mid = lo.lerp(hi, 0.5);
            let place = Place::new(Point2::new(w / 2.0 - s * mid.x, h / 2.0 + 5.0 - s * mid.y), s);
            let strokes = l
                .pieces
                .iter()
                .map(|p: &Piece| Stroke {
                    pen: match (p.visible, p.class) {
                        (false, _) => Pen::Hidden,
                        (true, Class::Tangent) => Pen::Tangent,
                        _ => Pen::Visible,
                    },
                    geom: p.geom.placed(&place),
                })
                .collect();
            let border = [
                (10.0, 10.0),
                (w - 10.0, 10.0),
                (w - 10.0, h - 10.0),
                (10.0, h - 10.0),
                (10.0, 10.0),
            ];
            let mut strokes: Vec<Stroke> = strokes;
            strokes.push(Stroke {
                pen: Pen::Border,
                geom: Geom2::Poly(border.iter().map(|&(x, y)| Point2::new(x, y)).collect()),
            });
            Page {
                w,
                h,
                strokes,
                fills: vec![],
                texts: vec![Text {
                    at: Point2::new(w / 2.0, 16.0),
                    h: 5.0,
                    angle: 0.0,
                    align: forge_draw::Align::Centre,
                    text: format!("{} ({:.3}:1)", l.name.to_uppercase(), s),
                    pen: Pen::Thin,
                }],
            }
        })
        .collect()
}
