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
fn graphic(page: &mut Page, shape: &Shape, f: impl Fn(Point) -> Point2) {
    match shape {
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
/// Drawing primitives for hit testing or highlighting a single library graphic.
pub fn graphic_page(shape: &Shape) -> Page {
    let mut page = Page::default();
    graphic(&mut page, shape, pt);
    page
}
const PIN_HEIGHT: f64 = 0.95;
const REFERENCE_HEIGHT: f64 = 2.0;
const VALUE_HEIGHT: f64 = 1.55;

fn glyph(
    page: &mut Page,
    s: &Symbol,
    unit: &str,
    at: Point,
    angle: f64,
    mirror: bool,
    show_pins: bool,
) -> (Point2, Point2) {
    let f = |p| pt(super::graph::transform(p, at, angle, mirror));
    for g in &s.graphics {
        if g.unit.as_deref().is_none_or(|u| u == unit) {
            graphic(page, &g.shape, f);
        }
    }
    let mut lo = pt(at);
    let mut hi = pt(at);
    for stroke in &page.strokes {
        let (a, b) = stroke.geom.bounds();
        lo.x = lo.x.min(a.x);
        lo.y = lo.y.min(a.y);
        hi.x = hi.x.max(b.x);
        hi.y = hi.y.max(b.y);
    }
    for (name, p) in s.pins.iter().filter(|(_, p)| p.unit == unit) {
        let tip = f(p.at);
        lo.x = lo.x.min(tip.x);
        lo.y = lo.y.min(tip.y);
        hi.x = hi.x.max(tip.x);
        hi.y = hi.y.max(tip.y);
        if !show_pins || s.power_net.is_some() {
            continue;
        }
        let label = format!("{name} ({})", p.number);
        let dx = tip.x - at[0];
        let dy = tip.y - at[1];
        let left = if dx < -dy.abs() {
            tip.x - forge_draw::page::text_width(&label, PIN_HEIGHT) - 0.45
        } else {
            tip.x + 0.65
        };
        text(page, Point2::new(left, tip.y + 0.4), PIN_HEIGHT, &label);
    }
    (lo, hi)
}

fn component_fields(page: &mut Page, lo: Point2, hi: Point2, row: &Row, reference: &str, value: &str) {
    let (at, reference_offset, value_offset) = match &row.item {
        Item::Place {
            at,
            ref_offset,
            value_offset,
            ..
        } => (*at, *ref_offset, *value_offset),
        Item::Gate { at, .. } => (*at, None, None),
        _ => unreachable!(),
    };
    let w = |s, h| forge_draw::page::text_width(s, h);
    let vertical = hi.y - lo.y > hi.x - lo.x;
    let mid = (lo.x + hi.x) / 2.;
    let x = |s, h| if vertical { hi.x + 1.0 } else { mid - w(s, h) / 2. };
    let y = if vertical {
        (lo.y + hi.y) / 2. - VALUE_HEIGHT - 0.3
    } else {
        hi.y + 0.85
    };
    let default_ref = Point2::new(
        x(reference, REFERENCE_HEIGHT),
        y + if value.is_empty() { 0. } else { VALUE_HEIGHT + 0.65 },
    );
    let locate =
        |offset: Option<Point>, default| offset.map_or(default, |p| Point2::new(at[0] + p[0], at[1] + p[1]));
    text(
        page,
        locate(reference_offset, default_ref),
        REFERENCE_HEIGHT,
        reference,
    );
    if !value.is_empty() {
        text(
            page,
            locate(value_offset, Point2::new(x(value, VALUE_HEIGHT), y)),
            VALUE_HEIGHT,
            value,
        );
    }
}

fn fit_page(mut p: Page) -> Page {
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
    p
}
fn fit(p: Page) -> String {
    forge_draw::svg::svg(&fit_page(p))
}

pub fn sheet_pdf(d: &Document, c: &Circuit) -> Result<Vec<u8>> {
    if c.occurrences.is_empty() {
        return Err(Fault::new(
            "NoSheets",
            Span::default(),
            "the document has no sheets to print",
        ));
    }
    let pages = c
        .occurrences
        .keys()
        .map(|s| sheet_page(d, c, s).map(fit_page))
        .collect::<Result<Vec<_>>>()?;
    Ok(forge_draw::pdf::pdf(&pages))
}
pub fn symbol_page(d: &Document, id: &str, unit: &str) -> Result<Page> {
    let s = d
        .symbols
        .get(id)
        .ok_or_else(|| Fault::new("UnknownSymbol", Span::default(), id))?;
    if !s.pins.is_empty() && !s.pins.values().any(|p| p.unit == unit) {
        return Err(Fault::new("UnknownUnit", Span::default(), unit));
    }
    let mut page = Page::default();
    glyph(&mut page, s, unit, [0., 0.], 0., false, true);
    Ok(page)
}
pub fn sheet_page(d: &Document, c: &Circuit, scope: &str) -> Result<Page> {
    let mut page = sheet_selection_page(d, c, scope, |_, _| true)?;
    let sheet = &c.occurrences[scope];
    if let Some(template) = d.templates.get(sheet) {
        let mut frame = Page::default();
        for g in &d.symbols[template].graphics {
            graphic(&mut frame, &g.shape, pt);
        }
        let index = c.occurrences.keys().position(|s| s == scope).unwrap() + 1;
        for text in &mut frame.texts {
            text.text = text
                .text
                .replace("{page}", &index.to_string())
                .replace("{pages}", &c.occurrences.len().to_string())
                .replace("{sheet}", scope)
                .replace("{title}", &d.sheets[sheet]);
        }
        frame.texts.append(&mut page.texts);
        frame.strokes.append(&mut page.strokes);
        frame.fills.append(&mut page.fills);
        return Ok(frame);
    }
    Ok(page)
}
/// Project selected rows with the full sheet's gate and attachment context.
pub fn sheet_selection_page(
    d: &Document,
    c: &Circuit,
    scope: &str,
    include: impl Fn(&Row, &str) -> bool,
) -> Result<Page> {
    let sheet = c
        .occurrences
        .get(scope)
        .ok_or_else(|| Fault::new("UnknownSheet", Span::default(), scope))?;
    let rows = d.rows.iter().filter(|r| r.sheet == *sheet).collect::<Vec<_>>();
    let mut page = Page::default();
    visit_rows(d, c, scope, &rows, |row, key, mut ink, _| {
        if !include(row, &key) {
            return;
        }
        page.texts.append(&mut ink.texts);
        page.strokes.append(&mut ink.strokes);
        page.fills.append(&mut ink.fills);
    });
    Ok(page)
}
/// Foreground geometry for one pin. Keep component bodies and fields in context,
/// but dim the labels and authored leads of the other terminals on each gate.
pub fn pin_inspection_page(d: &Document, c: &Circuit, scope: &str, focus: &PinInspection) -> Result<Page> {
    let sheet = c
        .occurrences
        .get(scope)
        .ok_or_else(|| Fault::new("UnknownSheet", Span::default(), scope))?;
    let mut gates = Map::new();
    for id in &focus.pins {
        let Some((owner, _)) = id.rsplit_once('.') else {
            continue;
        };
        if owner.rsplit_once('/').is_some_and(|(s, _)| s == scope) {
            let pin = &c.terminals[id];
            let sym = &d.symbols[&pin.symbol];
            gates.insert(c.origins[id].lo, (owner, &sym.pins[&pin.pin].unit, sym));
        }
    }
    let mut endpoints = Set::new();
    for wire in &focus.wires {
        let (a, b, _) = &c.wires[wire];
        endpoints.extend([a, b]);
    }
    let rows: Vec<_> = d.rows.iter().filter(|r| r.sheet == *sheet).collect();
    let mut page = Page::default();
    visit_rows(d, c, scope, &rows, |row, key, mut ink, text_ids| {
        if focus.wires.contains(&key) {
            page.strokes.append(&mut ink.strokes);
            return;
        }
        if matches!(row.item, Item::Junction(_)) && endpoints.contains(&key) {
            page.fills.append(&mut ink.fills);
            return;
        }
        let Some(&(owner, unit, sym)) = gates.get(&row.span.lo) else {
            return;
        };
        // Symbol leads are authored graphics. Match their open endpoints against
        // this specific gate's transformed pin anchors, including rotated instances.
        let inactive: Vec<_> = sym
            .pins
            .iter()
            .filter(|(_, p)| &p.unit == unit)
            .map(|(name, _)| format!("{owner}.{name}"))
            .filter(|id| !focus.pins.contains(id))
            .filter_map(|id| c.locations.get(&id).copied().map(pt))
            .collect();
        ink.strokes.retain(|s| match &s.geom {
            Geom2::Poly(ps) if ps.first() != ps.last() => ![ps.first(), ps.last()]
                .into_iter()
                .flatten()
                .any(|p| inactive.contains(p)),
            Geom2::Line { a, b } => !inactive.contains(a) && !inactive.contains(b),
            _ => true,
        });
        page.strokes.append(&mut ink.strokes);
        page.fills.append(&mut ink.fills);
        page.texts.extend(
            ink.texts
                .into_iter()
                .zip(text_ids)
                .filter(|(_, id)| !c.terminals.contains_key(id) || focus.pins.contains(id))
                .map(|(text, _)| text),
        );
    });
    Ok(page)
}
// The editor, SVG export and readability checks consume the same positioned primitives.
fn visit_rows(
    d: &Document,
    c: &Circuit,
    scope: &str,
    rows: &[&Row],
    mut visit: impl FnMut(&Row, String, Page, Vec<String>),
) {
    let placed: Map<_, _> = rows
        .iter()
        .filter_map(|r| {
            if let Item::Place {
                symbol, reference, ..
            } = &r.item
            {
                Some((&r.id, (symbol, reference.as_ref().unwrap_or(&r.id))))
            } else {
                None
            }
        })
        .collect();
    let mut right_wires = Set::new();
    for row in rows.iter().filter(|r| matches!(r.item, Item::Wire { .. })) {
        let (a, b, via) = &c.wires[&format!("{scope}/{}", row.id)];
        if via.first().unwrap_or(&c.locations[b])[0] > c.locations[a][0] {
            right_wires.insert(a.as_str());
        }
        if via.last().unwrap_or(&c.locations[a])[0] > c.locations[b][0] {
            right_wires.insert(b.as_str());
        }
    }
    for &row in rows {
        let mut page = Page::default();
        let mut text_ids = Vec::new();
        let key = format!("{scope}/{}", row.id);
        match &row.item {
            Item::Place {
                symbol,
                unit,
                at,
                angle,
                mirror,
                value,
                reference,
                ..
            } => {
                let sym = &d.symbols[symbol];
                let show_pins =
                    !(sym.pins.len() == 2 && sym.pins.values().all(|p| p.electrical == Electrical::Passive));
                let (lo, hi) = glyph(&mut page, sym, unit, *at, *angle, *mirror, show_pins);
                glyph_ids(&mut text_ids, sym, unit, &key, show_pins);
                if let Some(net) = &sym.power_net {
                    let x = (lo.x + hi.x - forge_draw::page::text_width(net, VALUE_HEIGHT)) / 2.;
                    let y = if hi.y > at[1] {
                        hi.y + 0.55
                    } else {
                        lo.y - VALUE_HEIGHT - 0.55
                    };
                    text(&mut page, Point2::new(x, y), VALUE_HEIGHT, net);
                    text_ids.push(key.clone());
                } else {
                    component_fields(
                        &mut page,
                        lo,
                        hi,
                        row,
                        d.reference(&key, reference.as_ref().unwrap_or(&row.id)),
                        value,
                    );
                    text_ids.push(format!("{key}:reference"));
                    if !value.is_empty() {
                        text_ids.push(format!("{key}:value"));
                    }
                }
            }
            Item::Gate {
                of,
                unit,
                at,
                angle,
                mirror,
            } => {
                let sym = &d.symbols[placed[of].0];
                let (lo, hi) = glyph(&mut page, sym, unit, *at, *angle, *mirror, true);
                glyph_ids(&mut text_ids, sym, unit, &format!("{scope}/{of}"), true);
                component_fields(
                    &mut page,
                    lo,
                    hi,
                    row,
                    &format!("{}.{unit}", d.reference(&format!("{scope}/{of}"), placed[of].1)),
                    "",
                );
                text_ids.push(format!("{key}:reference"));
            }
            Item::Wire { .. } => {
                let (a, b, via) = &c.wires[&key];
                let mut ps = vec![pt(c.locations[a])];
                ps.extend(via.iter().copied().map(pt));
                ps.push(pt(c.locations[b]));
                path(&mut page, ps);
            }
            Item::Label {
                text: s,
                at,
                global,
                target,
            } => net_label(
                &mut page,
                *at,
                s,
                if *global { Tag::Global } else { Tag::Local },
                right_wires.contains(format!("{scope}/{target}").as_str()),
            ),
            Item::Note {
                at,
                text: s,
                size,
                bold,
            } => {
                text(&mut page, pt(*at), *size, s);
                if *bold && let Some(t) = page.texts.last_mut() {
                    t.pen = Pen::BoldText;
                }
            }
            Item::Drawing { shape, width } => {
                graphic(&mut page, shape, pt);
                for stroke in &mut page.strokes {
                    stroke.pen = Pen::Annotation((width * 1000.).round().max(1.) as u32);
                }
            }
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
            Item::Port(at) => net_label(
                &mut page,
                *at,
                &row.id,
                Tag::Port,
                right_wires.contains(key.as_str()),
            ),
            Item::SheetInstance { at, .. } => text(&mut page, pt(*at), VALUE_HEIGHT, &row.id),
            Item::NoConnect(p) => {
                let a = pt(c.locations[&format!("{scope}/{p}")]);
                for sign in [-1., 1.] {
                    let v = Vec2::new(0.35, sign * 0.35);
                    path(&mut page, vec![a - v, a + v]);
                }
            }
            _ => {}
        }
        text_ids.resize(page.texts.len(), key.clone());
        visit(row, key, page, text_ids);
    }
}
fn glyph_ids(ids: &mut Vec<String>, sym: &Symbol, unit: &str, owner: &str, show: bool) {
    for (i, _) in
        sym.graphics.iter().enumerate().filter(|(_, g)| {
            g.unit.as_deref().is_none_or(|u| u == unit) && matches!(g.shape, Shape::Text(..))
        })
    {
        ids.push(format!("{owner}:graphic{i}"));
    }
    if show && sym.power_net.is_none() {
        ids.extend(
            sym.pins
                .iter()
                .filter(|(_, p)| p.unit == unit)
                .map(|(n, _)| format!("{owner}.{n}")),
        );
    }
}
enum Tag {
    Local,
    Global,
    Port,
}
fn net_label(page: &mut Page, at: Point, name: &str, tag: Tag, left: bool) {
    let a = pt(at);
    let h = VALUE_HEIGHT;
    let w = forge_draw::page::text_width(name, h);
    let dir = if left { -1. } else { 1. };
    let p = |x, y| a + Vec2::new(dir * x, y);
    let pad = 0.48;
    let dy = h / 2. + pad;
    let x = match tag {
        Tag::Local => {
            text(
                page,
                Point2::new(a.x + if left { -w - 0.25 } else { 0.25 }, a.y + 0.4),
                h,
                name,
            );
            return;
        }
        Tag::Global => {
            let end = dy + w + 2. * pad;
            path(page, vec![a, p(dy, dy), p(end, dy), p(end, -dy), p(dy, -dy), a]);
            dy + pad
        }
        Tag::Port => {
            let end = w + 2. * pad;
            path(
                page,
                vec![
                    p(0., -dy),
                    p(0., dy),
                    p(end, dy),
                    p(end + dy, 0.),
                    p(end, -dy),
                    p(0., -dy),
                ],
            );
            pad
        }
    };
    text(
        page,
        Point2::new(a.x + if left { -x - w } else { x }, a.y - h / 2.),
        h,
        name,
    );
}
pub(super) fn symbol_svg(d: &Document, id: &str, unit: &str) -> Result<String> {
    symbol_page(d, id, unit).map(fit)
}
pub(super) fn sheet_svg(d: &Document, c: &Circuit, scope: &str) -> Result<String> {
    sheet_page(d, c, scope).map(fit)
}

#[derive(Clone, Copy)]
struct Extent {
    left: f64,
    right: f64,
    bottom: f64,
    top: f64,
}
impl Extent {
    fn overlaps(self, b: Self) -> bool {
        self.left < b.right && b.left < self.right && self.bottom < b.top && b.bottom < self.top
    }
    fn cuts(self, a: Point2, b: Point2) -> bool {
        // Clip the segment to the text rectangle, including horizontal and vertical strokes.
        let (mut enter, mut leave) = (0.0_f64, 1.0_f64);
        for (p, d, min, max) in [
            (a.x, b.x - a.x, self.left, self.right),
            (a.y, b.y - a.y, self.bottom, self.top),
        ] {
            if d == 0. {
                if p < min || p > max {
                    return false;
                }
            } else {
                let (u, v) = ((min - p) / d, (max - p) / d);
                enter = enter.max(u.min(v));
                leave = leave.min(u.max(v));
            }
        }
        enter <= leave
    }
}
struct PrintedItem {
    extent: Extent,
    owner: String,
    kind: Ink,
}
enum Ink {
    Text(String),
    Outline(Vec<Point2>, f64, bool),
}
fn inside(points: &[Point2], p: Point2) -> bool {
    if points.len() < 4 || (points[0] - points[points.len() - 1]).length() > 1e-7 {
        return false;
    }
    points.windows(2).fold(false, |odd, s| {
        let (a, b) = (s[0], s[1]);
        odd ^ ((a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x)
    })
}
pub(super) fn readability(d: &Document, c: &Circuit, scope: &str, rows: &[&Row]) -> Vec<super::graph::Issue> {
    let mut items = Vec::new();
    visit_rows(d, c, scope, rows, |_, owner, page, ids| {
        collect_ink(&mut items, owner, page, ids)
    });
    intersections(items)
}
pub(super) fn library_readability(d: &Document) -> Vec<super::graph::Issue> {
    let mut findings = vec![];
    for (id, s) in &d.symbols {
        let units: Set<_> = s
            .pins
            .values()
            .map(|p| p.unit.as_str())
            .chain(s.graphics.iter().filter_map(|g| g.unit.as_deref()))
            .chain(std::iter::once("A"))
            .collect();
        for unit in units {
            let owner = format!("{id}/{unit}");
            let mut page = Page::default();
            glyph(&mut page, s, unit, [0., 0.], 0., false, true);
            let mut ids = vec![];
            glyph_ids(&mut ids, s, unit, &owner, true);
            let mut items = vec![];
            collect_ink(&mut items, owner, page, ids);
            findings.extend(intersections(items));
        }
    }
    findings
}
fn collect_ink(items: &mut Vec<PrintedItem>, owner: String, page: Page, ids: Vec<String>) {
    for (t, id) in page.texts.into_iter().zip(ids) {
        let extent = Extent {
            left: t.at.x,
            right: t.at.x + forge_draw::page::text_width(&t.text, t.h),
            bottom: t.at.y,
            top: t.at.y + t.h,
        };
        items.push(PrintedItem {
            extent,
            owner: owner.clone(),
            kind: Ink::Text(id),
        });
    }
    let outlines = page
        .strokes
        .into_iter()
        .map(|s| {
            let graphic = matches!(s.pen, Pen::Annotation(_));
            let pad = if graphic { s.pen.width() / 2. } else { 0.08 };
            (s.geom, pad, graphic)
        })
        .chain(page.fills.into_iter().filter_map(|(_, mut points)| {
            points.push(*points.first()?);
            Some((Geom2::Poly(points), 0.08, false))
        }));
    for (geometry, pad, annotation) in outlines {
        let (a, b) = geometry.bounds();
        // Small stroke allowance also handles the zero-width bounds of a vertical wire.
        let extent = Extent {
            left: a.x - pad,
            right: b.x + pad,
            bottom: a.y - pad,
            top: b.y + pad,
        };
        items.push(PrintedItem {
            extent,
            owner: owner.clone(),
            kind: Ink::Outline(geometry.points(0.03), pad, annotation),
        });
    }
}
fn intersections(mut items: Vec<PrintedItem>) -> Vec<super::graph::Issue> {
    items.sort_by(|a, b| a.extent.left.total_cmp(&b.extent.left));
    let mut active_text = Vec::new();
    let mut active_outline = Vec::new();
    let mut pairs = Set::new();
    for (i, a) in items.iter().enumerate() {
        active_text.retain(|j: &usize| items[*j].extent.right >= a.extent.left);
        let is_text = matches!(a.kind, Ink::Text(_));
        if is_text {
            active_outline.retain(|j: &usize| items[*j].extent.right >= a.extent.left);
        }
        let others = active_text
            .iter()
            .chain(if is_text { active_outline.as_slice() } else { &[] });
        for &j in others {
            let b = &items[j];
            if !a.extent.overlaps(b.extent) {
                continue;
            }
            let (kind, first, second) = match (&a.kind, &b.kind) {
                (Ink::Text(x), Ink::Text(y)) => ("TextBoundsOverlap", x.min(y), x.max(y)),
                _ => {
                    let (txt, line) = if is_text { (a, b) } else { (b, a) };
                    let (Ink::Text(id), Ink::Outline(pts, pad, annotation)) = (&txt.kind, &line.kind) else {
                        continue;
                    };
                    let bounds = txt.extent;
                    let ink_bounds = Extent {
                        left: bounds.left - pad,
                        right: bounds.right + pad,
                        bottom: bounds.bottom - pad,
                        top: bounds.top + pad,
                    };
                    let crosses = pts.windows(2).any(|s| ink_bounds.cuts(s[0], s[1]));
                    let intentional =
                        txt.owner == line.owner && (id == &line.owner || id.contains(":graphic"));
                    if !crosses
                        && (*annotation
                            || intentional
                            || !inside(pts, Point2::new(bounds.left, bounds.bottom)))
                    {
                        continue;
                    }
                    ("TextGeometryOverlap", id, &line.owner)
                }
            };
            pairs.insert((kind, first.clone(), second.clone()));
        }
        if is_text {
            active_text.push(i);
        } else {
            active_outline.push(i);
        }
    }
    pairs
        .into_iter()
        .map(|(kind, a, b)| super::graph::Issue {
            unknown: false,
            kind,
            message: if kind == "TextBoundsOverlap" {
                format!("Printed text overlaps: {a} and {b}")
            } else {
                format!("Printed text {a} overlaps geometry of {b}")
            },
            objects: vec![a, b],
        })
        .collect()
}
