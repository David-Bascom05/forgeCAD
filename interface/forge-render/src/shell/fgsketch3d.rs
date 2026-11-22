//! A Forge part's sketches in the view while a tool's selection takes sketch geometry: each
//! sketch drawn on its plane where the part has it, what the selection can take lit under the
//! pointer, and a click taking it (a profile by its loop or by pointing inside it, a line for an
//! axis or a path, a point for a hole), as `.fc`'s sketches in the view were picked.

use super::Shell;
use super::fgtool::SlotKind as K;
use egui::{Pos2, Rect, Stroke, vec2};

/// How near a curve or point the pointer takes it, in points.
const NEAR_PT: f32 = 7.0;

fn to_line_gap(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let (ab, ap) = (b - a, p - a);
    let t = (ap.dot(ab) / ab.length_sq().max(1e-6)).clamp(0.0, 1.0);
    (a + ab * t).distance(p)
}

/// Is this place inside the outline?
fn inside(l: &[Pos2], p: Pos2) -> bool {
    let mut odd = false;
    for k in 0..l.len() {
        let (a, b) = (l[k], l[(k + 1) % l.len()]);
        if (a.y > p.y) != (b.y > p.y) && p.x < a.x + (p.y - a.y) / (b.y - a.y) * (b.x - a.x) {
            odd = !odd;
        }
    }
    odd
}

fn area(l: &[Pos2]) -> f32 {
    let mut s = 0.0;
    for k in 0..l.len() {
        let (a, b) = (l[k], l[(k + 1) % l.len()]);
        s += a.x * b.y - b.x * a.y;
    }
    s.abs() * 0.5
}

/// What a selection of this kind takes from a sketch, by text: closed entities and loops for a
/// profile, lines for an axis, any curve for a path, named points for a hole.
pub fn takes(sk: &forge_lang::anchors::SketchRefs, kind: K) -> Vec<String> {
    match kind {
        K::Profile | K::Profiles => sk.closed.clone(),
        K::Line | K::Axis | K::RevAxis => sk.lines.clone(),
        K::Dir => sk.lines.iter().map(|l| format!("{l}.dir")).collect(),
        K::Path => sk
            .lines
            .iter()
            .chain(&sk.curves)
            .chain(&sk.closed)
            .cloned()
            .collect(),
        K::Points | K::Point => sk
            .points
            .iter()
            .filter(|p| !p.ends_with(".points"))
            .cloned()
            .collect(),
        _ => vec![],
    }
}

/// Where a taken text lies: its own shape, or its loop's segments' end to end.
fn shape_of<'a>(sk: &'a forge_lang::anchors::SketchRefs, text: &str) -> Vec<&'a [[f64; 3]]> {
    let text = text.strip_suffix(".dir").unwrap_or(text);
    if let Some((_, s)) = sk.shapes.iter().find(|s| s.0 == text) {
        return vec![s.as_slice()];
    }
    let seg = format!("{text}[");
    sk.shapes
        .iter()
        .filter(|s| s.0.starts_with(&seg))
        .map(|s| s.1.as_slice())
        .collect()
}

impl Shell {
    /// The slot taking picks now takes sketch geometry: which kind.
    fn sketch_slot(&self) -> Option<K> {
        let t = self.fg.as_ref()?.tool.as_ref()?;
        let k = t.slots.get(t.active)?.kind;
        matches!(
            k,
            K::Profile
                | K::Profiles
                | K::Line
                | K::Axis
                | K::RevAxis
                | K::Dir
                | K::Path
                | K::Points
                | K::Point
        )
        .then_some(k)
    }

    pub(super) fn fg_sketches_ui(&mut self, ctx: &egui::Context) {
        let Some(kind) = self.sketch_slot() else { return };
        let Some((camera, size)) = self.camera else { return };
        let (view, ppp, t) = (self.view_rect, ctx.pixels_per_point(), self.theme);
        let screen = |w: [f64; 3]| {
            camera
                .pixel_of([w[0] as f32, w[1] as f32, w[2] as f32], size.0, size.1)
                .map(|px| view.min + vec2(px.0, px.1) / ppp)
        };
        let up = |l: &[[f64; 3]]| -> Option<Vec<Pos2>> { l.iter().map(|w| screen(*w)).collect() };
        let pointer = ctx.pointer_hover_pos().filter(|p| {
            view.contains(*p)
                && !self.view_blocks.iter().any(|r| r.contains(*p))
                && ctx
                    .layer_id_at(*p)
                    .is_none_or(|l| l.order == egui::Order::Background)
        });
        let Some(fg) = self.fg.as_ref() else { return };
        let picked: Vec<String> = fg
            .tool
            .as_ref()
            .map(|t| t.slots[t.active].picks.clone())
            .unwrap_or_default();
        let p = ctx
            .layer_painter(egui::LayerId::background())
            .with_clip_rect(view);
        // the nearest thing it takes to the pointer: (gap, text); inside a profile counts as near
        // its outline, the smallest holding the pointer first
        let mut best: Option<(f32, String, Vec<Vec<Pos2>>)> = None;
        for sk in &fg.sketches {
            for text in takes(sk, kind) {
                let pieces: Vec<Vec<Pos2>> = shape_of(sk, &text).into_iter().filter_map(up).collect();
                if pieces.is_empty() {
                    continue;
                }
                let on = picked.contains(&text);
                for pts in &pieces {
                    if pts.len() == 1 {
                        p.circle_filled(
                            pts[0],
                            if on { 5.0 } else { 3.5 },
                            if on { t.focus } else { t.sketch },
                        );
                    } else {
                        p.add(egui::Shape::line(
                            pts.clone(),
                            Stroke::new(if on { 2.5 } else { 1.5 }, if on { t.focus } else { t.sketch }),
                        ));
                    }
                }
                let Some(at) = pointer else { continue };
                let mut gap = pieces
                    .iter()
                    .map(|pts| match pts.len() {
                        1 => at.distance(pts[0]),
                        _ => pts
                            .windows(2)
                            .map(|w| to_line_gap(at, w[0], w[1]))
                            .fold(f32::MAX, f32::min),
                    })
                    .fold(f32::MAX, f32::min);
                if matches!(kind, K::Profile | K::Profiles) {
                    let outline: Vec<Pos2> = pieces.iter().flatten().copied().collect();
                    if outline.len() > 2 && inside(&outline, at) {
                        // inside: as near as its size says, so the innermost wins
                        gap = gap.min(NEAR_PT * area(&outline) / (area(&outline) + 1.0e5));
                    }
                }
                if gap < NEAR_PT && best.as_ref().is_none_or(|b| gap < b.0) {
                    best = Some((gap, text, pieces));
                }
            }
        }
        let Some((_, text, pieces)) = best else { return };
        for pts in &pieces {
            if pts.len() == 1 {
                p.circle_stroke(pts[0], 7.0, Stroke::new(2.0, t.focus));
            } else {
                p.add(egui::Shape::line(pts.clone(), Stroke::new(3.0, t.focus)));
            }
        }
        let at = pointer.expect("near the pointer");
        // the press is this overlay's, not the view's: no face under it is picked too
        self.view_blocks
            .push(Rect::from_center_size(at, vec2(2.0 * NEAR_PT, 2.0 * NEAR_PT)));
        self.places.push((
            format!("fg:sketch3d:{text}"),
            Rect::from_center_size(at, vec2(4.0, 4.0)),
        ));
        self.status = format!("{}: {text}", kind.prompt());
        if ctx.input(|i| i.pointer.primary_clicked())
            && let Some(tool) = self.fg.as_mut().and_then(|f| f.tool.as_mut())
        {
            tool.take(Ok(text));
        }
    }
}
