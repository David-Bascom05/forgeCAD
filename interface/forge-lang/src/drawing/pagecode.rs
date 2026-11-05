//! A drawing's sheets as bytes, for keeping them between opens (`regen::shown`).

use crate::store::{In, Out};
use forge_draw::Geom2;
use forge_draw::page::{Align, Page, Pen, Stroke, Text};
use forge_math::{Point2, Vec2};

const PENS: [Pen; 10] = [
    Pen::Visible,
    Pen::Hidden,
    Pen::Tangent,
    Pen::Center,
    Pen::Phantom,
    Pen::Thin,
    Pen::Hatch,
    Pen::Cut,
    Pen::Border,
    Pen::Sick,
];
const ALIGNS: [Align; 3] = [Align::Left, Align::Centre, Align::Right];

fn pen(o: &mut Out, p: Pen) {
    o.u64(PENS.iter().position(|x| *x == p).unwrap_or(0) as u64);
}

fn read_pen(i: &mut In) -> Option<Pen> {
    PENS.get(usize::try_from(i.u64()?).ok()?).copied()
}

fn pt(o: &mut Out, p: Point2) {
    o.f64(p.x);
    o.f64(p.y);
}

fn read_pt(i: &mut In) -> Option<Point2> {
    Some(Point2::new(i.f64()?, i.f64()?))
}

fn geom(o: &mut Out, g: &Geom2) {
    match g {
        Geom2::Line { a, b } => {
            o.u64(0);
            pt(o, *a);
            pt(o, *b);
        }
        Geom2::Conic { c, u, v, t0, t1 } => {
            o.u64(1);
            pt(o, *c);
            o.f64(u.x);
            o.f64(u.y);
            o.f64(v.x);
            o.f64(v.y);
            o.f64(*t0);
            o.f64(*t1);
        }
        Geom2::Poly(ps) => {
            o.u64(2);
            o.u64(ps.len() as u64);
            for p in ps {
                pt(o, *p);
            }
        }
    }
}

fn read_geom(i: &mut In) -> Option<Geom2> {
    Some(match i.u64()? {
        0 => Geom2::Line {
            a: read_pt(i)?,
            b: read_pt(i)?,
        },
        1 => Geom2::Conic {
            c: read_pt(i)?,
            u: Vec2::new(i.f64()?, i.f64()?),
            v: Vec2::new(i.f64()?, i.f64()?),
            t0: i.f64()?,
            t1: i.f64()?,
        },
        2 => {
            let n = i.count(16)?;
            let mut ps = Vec::with_capacity(n);
            for _ in 0..n {
                ps.push(read_pt(i)?);
            }
            Geom2::Poly(ps)
        }
        _ => return None,
    })
}

pub fn write(pages: &[Page]) -> Vec<u8> {
    let mut o = Out::default();
    o.u64(pages.len() as u64);
    for p in pages {
        o.f64(p.w);
        o.f64(p.h);
        o.u64(p.strokes.len() as u64);
        for s in &p.strokes {
            pen(&mut o, s.pen);
            geom(&mut o, &s.geom);
        }
        o.u64(p.fills.len() as u64);
        for (pn, ps) in &p.fills {
            pen(&mut o, *pn);
            o.u64(ps.len() as u64);
            for q in ps {
                pt(&mut o, *q);
            }
        }
        o.u64(p.texts.len() as u64);
        for t in &p.texts {
            pt(&mut o, t.at);
            o.f64(t.h);
            o.f64(t.angle);
            o.u64(ALIGNS.iter().position(|a| *a == t.align).unwrap_or(0) as u64);
            o.str(&t.text);
            pen(&mut o, t.pen);
        }
    }
    o.0
}

pub fn read(b: &[u8]) -> Option<Vec<Page>> {
    let mut i = In(b);
    let n = i.count(40)?;
    let mut pages = Vec::with_capacity(n);
    for _ in 0..n {
        let (w, h) = (i.f64()?, i.f64()?);
        let mut page = Page {
            w,
            h,
            ..Page::default()
        };
        for _ in 0..i.count(16)? {
            let pen = read_pen(&mut i)?;
            page.strokes.push(Stroke {
                pen,
                geom: read_geom(&mut i)?,
            });
        }
        for _ in 0..i.count(16)? {
            let pn = read_pen(&mut i)?;
            let m = i.count(16)?;
            let mut ps = Vec::with_capacity(m);
            for _ in 0..m {
                ps.push(read_pt(&mut i)?);
            }
            page.fills.push((pn, ps));
        }
        for _ in 0..i.count(48)? {
            let at = read_pt(&mut i)?;
            let (h, angle) = (i.f64()?, i.f64()?);
            let align = *ALIGNS.get(usize::try_from(i.u64()?).ok()?)?;
            let text = i.str()?.to_string();
            let pen = read_pen(&mut i)?;
            page.texts.push(Text {
                at,
                h,
                angle,
                align,
                text,
                pen,
            });
        }
        pages.push(page);
    }
    i.done().then_some(pages)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sheets_read_back_as_written() {
        let page = Page {
            w: 420.0,
            h: 297.0,
            strokes: vec![
                Stroke {
                    pen: Pen::Hidden,
                    geom: Geom2::Line {
                        a: Point2::new(1.0, 2.0),
                        b: Point2::new(-0.0, 1e-300),
                    },
                },
                Stroke {
                    pen: Pen::Sick,
                    geom: Geom2::Conic {
                        c: Point2::new(0.0, 0.0),
                        u: Vec2::new(1.0, 0.0),
                        v: Vec2::new(0.0, 2.0),
                        t0: 0.1,
                        t1: 3.0,
                    },
                },
                Stroke {
                    pen: Pen::Visible,
                    geom: Geom2::Poly(vec![Point2::new(0.0, 0.0), Point2::new(1.0, 1.0)]),
                },
            ],
            fills: vec![(Pen::Thin, vec![Point2::new(3.0, 4.0)])],
            texts: vec![Text {
                at: Point2::new(5.0, 6.0),
                h: 3.5,
                angle: 90.0,
                align: Align::Right,
                text: "SECTION A-A".into(),
                pen: Pen::Thin,
            }],
        };
        let back = read(&write(std::slice::from_ref(&page))).expect("reads");
        assert_eq!(back, vec![page.clone()]);
        let w = write(&[page]);
        assert_eq!(read(&w[..w.len() - 2]), None);
    }
}
