//! Project the same symbol geometry and sheet endpoints used by the compiler into drawing pages.
use super::*;
use forge_draw::{Align, Geom2, Page, Pen, Place, Stroke};
use forge_math::{Point2, Vec2, dmath};
use std::f64::consts::TAU;

fn pt(p: Point) -> Point2 {
    Point2::new(p[0], p[1])
}
fn stroke(page: &mut Page, geom: Geom2) {
    page.strokes.push(Stroke { pen: Pen::Thin, geom });
}
fn path(page: &mut Page, ps: Vec<Point2>) {
    stroke(page, Geom2::Poly(ps));
}
fn text(page: &mut Page, p: Point2, h: f64, s: &str) {
    forge_draw::sheet::text(page, p, h, Align::Left, s);
}
fn arc(page: &mut Page, a: Point2, m: Point2, b: Point2) {
    let x = m - a;
    let y = b - a;
    let d = 2.0 * (x.x * y.y - x.y * y.x);
    if d == 0.0 {
        path(page, vec![a, m, b]);
        return;
    }
    let offset = Vec2::new(
        (x.dot(x) * y.y - y.dot(y) * x.y) / d,
        (x.x * y.dot(y) - y.x * x.dot(x)) / d,
    );
    let center = a + offset;
    let start = dmath::atan2(a.y - center.y, a.x - center.x);
    let end = dmath::atan2(b.y - center.y, b.x - center.x);
    let (t0, t1) = if d > 0.0 {
        (start, start + (end - start).rem_euclid(TAU))
    } else {
        (end, end + (start - end).rem_euclid(TAU))
    };
    let r = offset.length();
    stroke(
        page,
        Geom2::Conic {
            c: center,
            u: Vec2::new(r, 0.),
            v: Vec2::new(0., r),
            t0,
            t1,
        },
    );
}
pub(super) fn pin_label(pin: &Pin, name: &str) -> (Point, String) {
    (
        super::graph::transform([0.8, 0.], pin.at, pin.angle, false),
        format!("{name} ({})", pin.number),
    )
}
fn glyph(page: &mut Page, s: &Symbol, unit: &str, at: Point, angle: f64, mirror: bool) {
    let f = |p: Point| pt(super::graph::transform(p, at, angle, mirror));
    for g in &s.graphics {
        if g.unit.as_deref().is_some_and(|u| u != unit) {
            continue;
        }
        match &g.shape {
            Shape::Line(a, b) => path(page, vec![f(*a), f(*b)]),
            Shape::Rect(a, b) => path(page, vec![f(*a), f([a[0], b[1]]), f(*b), f([b[0], a[1]]), f(*a)]),
            Shape::Circle(a, r) => stroke(
                page,
                Geom2::Conic {
                    c: f(*a),
                    u: Vec2::new(*r, 0.),
                    v: Vec2::new(0., *r),
                    t0: 0.,
                    t1: TAU,
                },
            ),
            Shape::Text(a, t) => text(page, f(*a), 1.2, t),
            Shape::Arc(a, m, b) => arc(page, f(*a), f(*m), f(*b)),
            Shape::Polyline(ps) => path(page, ps.iter().copied().map(f).collect()),
        }
    }
    for (name, p) in &s.pins {
        if p.unit == unit {
            let (tip, label) = pin_label(p, name);
            path(page, vec![f(p.at), f(tip)]);
            text(page, f(tip), 0.8, &label);
        }
    }
}
fn fit(mut p: Page) -> String {
    let (mut x0, mut y0, mut x1, mut y1) =
        (f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
    let mut include = |a: Point2, b: Point2| {
        x0 = x0.min(a.x);
        y0 = y0.min(a.y);
        x1 = x1.max(b.x);
        y1 = y1.max(b.y);
    };
    for s in &p.strokes {
        let (a, b) = s.geom.bounds();
        include(a, b);
    }
    for t in &p.texts {
        include(
            t.at,
            t.at + Vec2::new(forge_draw::page::text_width(&t.text, t.h), t.h),
        );
    }
    for (_, poly) in &p.fills {
        for v in poly {
            include(*v, *v);
        }
    }
    if !x0.is_finite() {
        (x0, y0, x1, y1) = (0., 0., 1., 1.);
    }
    p.w = x1 - x0 + 6.;
    p.h = y1 - y0 + 6.;
    let place = Place::new(Point2::new(3. - x0, 3. - y0), 1.);
    for s in &mut p.strokes {
        s.geom = s.geom.placed(&place);
    }
    for t in &mut p.texts {
        t.at = place.point(t.at);
    }
    for (_, poly) in &mut p.fills {
        for v in poly {
            *v = place.point(*v);
        }
    }
    forge_draw::svg::svg(&p)
}
pub(super) fn symbol_svg(d: &Document, id: &str, unit: &str) -> Result<String> {
    let s = d
        .symbols
        .get(id)
        .ok_or_else(|| Fault::new("UnknownSymbol", Span::default(), id))?;
    if !s.pins.is_empty() && !s.pins.values().any(|p| p.unit == unit) {
        return Err(Fault::new("UnknownUnit", Span::default(), unit));
    }
    let mut page = Page::default();
    glyph(&mut page, s, unit, [0., 0.], 0., false);
    Ok(fit(page))
}
pub(super) fn sheet_svg(d: &Document, c: &Circuit, scope: &str) -> Result<String> {
    let sheet = c
        .occurrences
        .get(scope)
        .ok_or_else(|| Fault::new("UnknownSheet", Span::default(), scope))?;
    let mut page = Page::default();
    let rows: Vec<_> = d.rows.iter().filter(|r| r.sheet == *sheet).collect();
    let placed: Map<_, _> = rows
        .iter()
        .filter_map(|r| {
            if let Item::Place { symbol, .. } = &r.item {
                Some((&r.id, symbol))
            } else {
                None
            }
        })
        .collect();
    for row in rows {
        let key = format!("{scope}/{}", row.id);
        match &row.item {
            Item::Place {
                symbol,
                unit,
                at,
                angle,
                mirror,
                value,
                ..
            } => {
                glyph(&mut page, &d.symbols[symbol], unit, *at, *angle, *mirror);
                text(
                    &mut page,
                    pt(*at) + Vec2::new(0., 1.5),
                    1.,
                    &format!("{} {value}", row.id),
                );
            }
            Item::Gate {
                of,
                unit,
                at,
                angle,
                mirror,
            } => {
                glyph(&mut page, &d.symbols[placed[of]], unit, *at, *angle, *mirror);
                text(
                    &mut page,
                    pt(*at) + Vec2::new(0., 1.5),
                    1.,
                    &format!("{of}.{unit}"),
                );
            }
            Item::Wire { .. } => {
                let (a, b, via) = &c.wires[&key];
                let mut ps = vec![pt(c.locations[a])];
                ps.extend(via.iter().copied().map(pt));
                ps.push(pt(c.locations[b]));
                path(&mut page, ps);
            }
            Item::Label { text: s, at, .. } | Item::Note(at, s) => text(&mut page, pt(*at), 1.2, s),
            Item::Junction(at) => {
                let a = pt(*at);
                page.fills.push((
                    Pen::Visible,
                    (0..12)
                        .map(|i| {
                            let (s, c) = dmath::sin_cos(TAU * i as f64 / 12.);
                            a + Vec2::new(c * 0.35, s * 0.35)
                        })
                        .collect(),
                ));
            }
            Item::Port(at) | Item::SheetInstance { at, .. } => text(&mut page, pt(*at), 1.2, &row.id),
            Item::NoConnect(p) => {
                let a = pt(c.locations[&format!("{scope}/{p}")]);
                for sign in [-1., 1.] {
                    let v = Vec2::new(0.35, sign * 0.35);
                    path(&mut page, vec![a - v, a + v]);
                }
            }
            _ => {}
        }
    }
    Ok(fit(page))
}

/// Use the same print coordinates and text widths for diagnostics and export.
pub(super) type TextBounds = [f64; 4];
pub(super) fn bounds(at: Point, s: &str, h: f64) -> TextBounds {
    [
        at[0],
        at[0] + forge_draw::page::text_width(s, h),
        at[1],
        at[1] + h,
    ]
}
pub(super) fn text_bounds(
    rows: &[&Row],
    scope: &str,
    pins: Vec<(String, Point, String)>,
) -> Map<String, TextBounds> {
    let mut boxes: Map<_, _> = pins
        .into_iter()
        .map(|(id, at, s)| (id, bounds(at, &s, 0.8)))
        .collect();
    for r in rows {
        if let Item::Note(at, s) | Item::Label { at, text: s, .. } = &r.item {
            boxes.insert(format!("{scope}/{}", r.id), bounds(*at, s, 1.2));
        }
    }
    boxes
}
pub(super) fn check_text(boxes: &Map<String, TextBounds>, issues: &mut Vec<super::graph::Issue>) {
    let mut ordered: Vec<_> = boxes.iter().collect();
    ordered.sort_by(|(_, a), (_, b)| a[0].total_cmp(&b[0]));
    for (i, (id, a)) in ordered.iter().enumerate() {
        for (other, b) in ordered[i + 1..].iter().take_while(|(_, b)| b[0] < a[1]) {
            if a[2] < b[3] && b[2] < a[3] {
                issues.push(super::graph::Issue {
                    kind: "TextBoundsOverlap",
                    objects: vec![(*id).clone(), (*other).clone()],
                    message: "printed pin name, label or note extents intersect".into(),
                });
            }
        }
    }
}
