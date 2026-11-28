//! Standard overlap checks use the same printed strokes and text as the canvas.
use super::*;
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
pub(in crate::schematic) fn readability(d: &Document, c: &Circuit, scope: &str, rows: &[&Row]) -> Vec<Issue> {
    let mut items = Vec::new();
    visit_rows(d, c, scope, rows, |_, owner, page, ids| {
        collect_ink(&mut items, owner, page, ids)
    });
    intersections(items)
}
pub(in crate::schematic) fn library_readability(d: &Document) -> Vec<Issue> {
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
fn intersections(mut items: Vec<PrintedItem>) -> Vec<Issue> {
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
        .map(|(kind, a, b)| Issue {
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
