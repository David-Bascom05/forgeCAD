//! A Forge part's selected feature's number on the model: an arrow along its real axis that drags
//! in real millimetres, the part rebuilt as it goes (a preview: the file's text kept), and let go
//! the value is written, one undo step. A driving param is set (`SetParam`); a number written in
//! the feature's call is lifted to a param of the feature first (`LiftLiteral`, then `SetParam`),
//! as its lint's fix would; a driven one (an expression) shows, and doesn't drag.

use super::fg::{DimRow, FeatRow, FgAct, From, Issue, ParamRow};
use super::theme::{mono, sans};
use super::{Shell, theme};
use crate::camera::{Camera, dot};
use crate::mesh::{Lod, PartMesh, VERTEX};
use egui::{Rect, Sense, Stroke, StrokeKind, pos2, vec2};
use forge_lang::error::{Literal, Op, ParamKey, Scope};
use forge_lang::syntax::Seg;
use forge_lang::values::Dim;

/// How long the arrow's drawn, in points.
const ARROW_PT: f32 = 64.0;
const HEAD_PT: f32 = 11.0;
const FIELD_W: f32 = 92.0;

/// What a value of the number writes.
#[derive(Clone, Debug, PartialEq)]
pub enum Target {
    /// A param read by its bare name: `SetParam`.
    Param(ParamKey),
    /// A number written in the call: lifted to this param by this op, then set.
    Literal { lift: Op, key: ParamKey },
    /// An expression of params: no handle of its own.
    Driven(Vec<ParamKey>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct FgDim {
    /// The feature it is of, and the build it was found on.
    pub feat: String,
    pub key: u64,
    /// The number's word (`depth`, `radius`).
    pub what: String,
    /// The value as built, and as shown (it moves as a drag goes, before it's written).
    pub was: f64,
    pub value: f64,
    pub target: Target,
    /// Where its arrow starts, which way it points (unit), and how much the value changes per
    /// millimetre along it.
    pub origin: [f64; 3],
    pub dir: [f64; 3],
    pub per_mm: f64,
    pub text: String,
    pub dragging: bool,
    pub error: Option<String>,
    press: f64,
    start: f64,
}

impl FgDim {
    pub fn draggable(&self) -> bool {
        !matches!(self.target, Target::Driven(_))
    }

    /// The ops that make the number this value: one undo step.
    pub fn ops(&self, v: f64) -> Vec<Op> {
        let set = |key: &ParamKey| Op::SetParam {
            param: key.clone(),
            value: Literal {
                value: v,
                dim: Dim::Length,
            },
        };
        match &self.target {
            Target::Param(k) => vec![set(k)],
            Target::Literal { lift, key } => vec![lift.clone(), set(key)],
            Target::Driven(_) => vec![],
        }
    }

    /// Where the arrow starts while the value's moved from what it was.
    fn base(&self) -> [f64; 3] {
        let s = (self.value - self.was) / self.per_mm;
        [0, 1, 2].map(|k| self.origin[k] + self.dir[k] * s)
    }
}

/// Along the axis through `at` in direction `dir`, the point nearest the ray under this pixel,
/// in mm from `at`. None when the axis looks straight at the eye.
pub fn along_axis(c: &Camera, size: (u32, u32), px: (f32, f32), at: [f64; 3], dir: [f64; 3]) -> Option<f64> {
    let eye = c.eye();
    let through = c.point_at(px, c.dist, size.0, size.1);
    let d = [0, 1, 2].map(|k| (through[k] - eye[k]) as f64);
    let l = d.iter().map(|v| v * v).sum::<f64>().sqrt();
    let d = d.map(|v| v / l);
    let w = [0, 1, 2].map(|k| at[k] - eye[k] as f64);
    let (a, b) = (
        dir.iter().zip(&d).map(|(x, y)| x * y).sum::<f64>(),
        dir.iter().zip(&w).map(|(x, y)| x * y).sum::<f64>(),
    );
    let e = d.iter().zip(&w).map(|(x, y)| x * y).sum::<f64>();
    let denom = 1.0 - a * a;
    if denom < 1e-6 {
        return None;
    }
    Some((a * e - b) / denom)
}

/// Drags land on tidy values, as fine as a pixel can tell.
pub fn tidy(v: f64, mm_per_px: f64) -> f64 {
    let step = 10f64.powi(mm_per_px.max(1e-6).log10().ceil() as i32);
    (v / step).round() * step
}

/// A face's triangles as the finest level has them: its area, area-weighted middle and normal, and
/// how its normals spread apart moving along it (a bulge, not a hollow; zero if flat), and the
/// triangle nearest its middle (where on the surface an arrow stands, and its normal there).
#[derive(Clone, Copy, Debug)]
struct Patch {
    area: f64,
    bulge: f64,
    on: [f64; 3],
    normal: [f64; 3],
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn dot3(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn unit(a: [f64; 3]) -> [f64; 3] {
    let l = dot3(a, a).sqrt();
    if l > 0.0 { a.map(|v| v / l) } else { [0.0; 3] }
}

/// An octahedral i16x2 normal back to a unit vector.
fn unoct(x: i16, y: i16) -> [f64; 3] {
    let (mut x, mut y) = (x as f64 / 32767.0, y as f64 / 32767.0);
    let z = 1.0 - x.abs() - y.abs();
    if z < 0.0 {
        (x, y) = ((1.0 - y.abs()) * x.signum(), (1.0 - x.abs()) * y.signum());
    }
    unit([x, y, z])
}

fn vertex(l: &Lod, i: usize) -> Option<([f64; 3], [f64; 3])> {
    let b = l.vertices.get(i * VERTEX..(i + 1) * VERTEX)?;
    let f = |k: usize| f32::from_le_bytes([b[k], b[k + 1], b[k + 2], b[k + 3]]) as f64;
    let s = |k: usize| i16::from_le_bytes([b[k], b[k + 1]]);
    Some(([f(0), f(4), f(8)], unoct(s(12), s(14))))
}

/// A point of face `face` near its middle, and its normal there.
pub(crate) fn face_point(l: &Lod, face: u32) -> Option<([f64; 3], [f64; 3])> {
    patch(l, face).map(|p| (p.on, p.normal))
}

fn patch(l: &Lod, face: u32) -> Option<Patch> {
    let r = l.face(face)?;
    let mut tris = Vec::with_capacity(r.tris as usize);
    for t in r.first_tri..r.first_tri + r.tris {
        let at = t as usize * 12;
        let idx = |k: usize| {
            l.indices
                .get(at + k * 4..at + k * 4 + 4)
                .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize)
        };
        let (Some(a), Some(b), Some(c)) = (idx(0), idx(1), idx(2)) else {
            continue;
        };
        let (Some(a), Some(b), Some(c)) = (vertex(l, a), vertex(l, b), vertex(l, c)) else {
            continue;
        };
        let twice = dot3(
            cross(sub(b.0, a.0), sub(c.0, a.0)),
            cross(sub(b.0, a.0), sub(c.0, a.0)),
        )
        .sqrt();
        let mid = [0, 1, 2].map(|k| (a.0[k] + b.0[k] + c.0[k]) / 3.0);
        let n = unit([0, 1, 2].map(|k| a.1[k] + b.1[k] + c.1[k]));
        tris.push((mid, n, twice));
    }
    let area: f64 = tris.iter().map(|t| t.2).sum();
    if area <= 0.0 {
        return None;
    }
    let centre = [0, 1, 2].map(|k| tris.iter().map(|t| t.0[k] * t.2).sum::<f64>() / area);
    let mut bulge = 0.0;
    let step = (tris.len() / 16).max(1);
    for p in tris.iter().step_by(step) {
        for q in tris.iter().step_by(step) {
            bulge += dot3(sub(p.1, q.1), sub(p.0, q.0));
        }
    }
    let near = tris.iter().min_by(|a, b| {
        dot3(sub(a.0, centre), sub(a.0, centre)).total_cmp(&dot3(sub(b.0, centre), sub(b.0, centre)))
    })?;
    Some(Patch {
        area: area / 2.0,
        bulge,
        on: near.0,
        normal: near.1,
    })
}

/// The number of a feature's kind an arrow drags, its faces' role, and which way it grows: along
/// the face's outward normal (+), against it (-), or out if the face hollows and in if it bulges
/// (None); and how much per mm.
fn rule(kind: &str, statement: &str) -> Option<(&'static [&'static str], &'static str, Option<f64>)> {
    Some(match kind {
        // a cut's end is its floor: deeper is against the floor's normal
        "extrude" if statement.contains("= cut(") => (&["depth", "forward"], "end", Some(-1.0)),
        "extrude" if statement.contains("sym(") => (&["depth"], "end", Some(2.0)),
        "extrude" => (&["depth", "forward"], "end", Some(1.0)),
        // a hole's wall faces its axis: wider is against its normal, both sides
        "hole" => (&["diameter"], "wall", Some(-2.0)),
        "fillet" => (&["radius"], "faces", None),
        "chamfer" => (&["size"], "faces", Some(-1.0)),
        "shell" => (&["wall"], "inner", Some(1.0)),
        _ => return None,
    })
}

/// A name for the param a number in the call is lifted to: its word, or that and a count.
fn fresh(what: &str, owner: &[Seg], params: &[ParamRow]) -> String {
    let base: String = what
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    let taken = |n: &str| params.iter().any(|p| p.key.owner == owner && p.key.name == n);
    if !taken(&base) {
        return base;
    }
    (2..)
        .map(|k| format!("{base}_{k}"))
        .find(|n| !taken(n))
        .expect("a free name")
}

/// The number's handle: where its value is written.
fn target_of(feat: &FeatRow, d: &DimRow, src: &str, issues: &[Issue], params: &[ParamRow]) -> Option<Target> {
    match &d.from {
        From::Driving(k) => Some(Target::Param(k.clone())),
        From::Driven(ks) => Some(Target::Driven(ks.clone())),
        From::Const => {
            // the loose number its lint names, written in this feature's statement
            // the feature's own param, whatever scope the lint's fixes offer
            let own = forge_lang::error::parse_path(&feat.id)?;
            let lift = issues
                .iter()
                .filter(|i| i.feature.as_deref() == Some(feat.id.as_str()))
                .flat_map(|i| i.fixes.iter())
                .find_map(|op| match op {
                    Op::LiftLiteral { span, scope, .. } => {
                        let t = src.get(span.lo as usize..span.hi as usize)?;
                        let v: f64 = t.trim_end_matches(char::is_alphabetic).parse().ok()?;
                        let owner = match scope {
                            Scope::Local(o) => o.clone(),
                            Scope::Shared => own.clone(),
                        };
                        ((v - d.value).abs() < 1e-9).then_some((*span, owner))
                    }
                    _ => None,
                })?;
            let (span, owner) = lift;
            let name = fresh(&d.what, &owner, params);
            Some(Target::Literal {
                lift: Op::LiftLiteral {
                    span,
                    name: name.clone(),
                    scope: Scope::Local(owner.clone()),
                },
                key: ParamKey { owner, name },
            })
        }
    }
}

/// The selected feature's number on the part as built (`key`: the build), if its kind has one an
/// arrow can stand for: on its own faces, as the anchors name them.
pub fn handle(
    feat: &FeatRow,
    src: &str,
    issues: &[Issue],
    params: &[ParamRow],
    anchors: &crate::fg::AnchorView,
    mesh: &PartMesh,
    key: u64,
) -> Option<FgDim> {
    let line = src.lines().nth(feat.line.saturating_sub(1)).unwrap_or("");
    let (whats, role, grows) = rule(&feat.kind, line)?;
    let d = feat
        .dims
        .iter()
        // a number written bare in the call may not carry its dimension: a length by its word
        .find(|d| !matches!(d.dim, Dim::Angle | Dim::Count) && whats.contains(&d.what.as_str()))?;
    let target = target_of(feat, d, src, issues, params)?;
    let faces = anchors.role_faces.get(&format!("{}.{role}", feat.id))?;
    let lod = mesh.lods.first()?;
    let p = faces
        .iter()
        .filter_map(|&f| patch(lod, f))
        .max_by(|a, b| a.area.total_cmp(&b.area))?;
    let per_mm = grows.unwrap_or(if p.bulge < 0.0 { 1.0 } else { -1.0 });
    Some(FgDim {
        feat: feat.id.clone(),
        key,
        what: d.what.clone(),
        was: d.value,
        value: d.value,
        target,
        origin: p.on,
        dir: p.normal,
        per_mm,
        text: super::fg::short(d.value),
        dragging: false,
        error: None,
        press: 0.0,
        start: d.value,
    })
}

impl Shell {
    /// The arrow on the model and its value's field: dragged, the part follows; let go or typed,
    /// it's written.
    pub(super) fn fg_dim_ui(&mut self, ctx: &egui::Context) {
        let t = self.theme;
        let Some((camera, size)) = self.camera else { return };
        let Some(mut e) = self.fg_dim.take() else { return };
        let ppp = ctx.pixels_per_point();
        let view = self.view_rect;
        let to_screen = |px: (f32, f32)| view.min + vec2(px.0, px.1) / ppp;
        let base = e.base();
        let p3 = base.map(|v| v as f32);
        let Some(at_px) = camera.pixel_of(p3, size.0, size.1) else {
            self.fg_dim = Some(e);
            return;
        };
        // a millimetre on screen, here
        let dir = e.dir;
        let one = [0, 1, 2].map(|k| p3[k] + dir[k] as f32);
        let next = camera.pixel_of(one, size.0, size.1).unwrap_or(at_px);
        let along = vec2(next.0 - at_px.0, next.1 - at_px.1) / ppp;
        let (f, _, _) = camera.axes();
        let facing = dot(f, dir.map(|v| v as f32)).abs();
        let per_mm_pt = along.length().max(1e-4);
        let way = along / per_mm_pt;
        let start = to_screen(at_px);
        let tip = start + way * ARROW_PT;
        let ink = if e.error.is_some() { t.error } else { t.focus };
        let mut acts: Vec<FgAct> = Vec::new();
        // before the arrow's response: egui ends a drag on Esc, and that mustn't read as a release
        if e.dragging && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            acts.push(FgAct::DimCancel);
            e.dragging = false;
            e.value = e.was;
            e.text = super::fg::short(e.was);
        }
        let mut hint = None;
        egui::Area::new(egui::Id::unique("fg-dim-arrow"))
            .fixed_pos(view.min)
            .order(egui::Order::Middle)
            .interactable(true)
            .constrain(false)
            .show(ctx, |ui| {
                let p = ui.painter().clone();
                let side = vec2(-way.y, way.x);
                if facing < 0.97 && e.draggable() {
                    let grip = Rect::from_center_size(tip - way * HEAD_PT * 0.5, vec2(26.0, 26.0));
                    let r = ui
                        .interact(grip, ui.scope_id().with("grip"), Sense::drag())
                        .on_hover_cursor(egui::CursorIcon::Grab);
                    self.places.push(("dim:arrow".to_string(), grip));
                    self.view_blocks.push(grip);
                    let wide = if r.hovered() || r.dragged() { 3.0 } else { 2.0 };
                    p.line_segment([start, tip - way * HEAD_PT], Stroke::new(wide, ink));
                    p.add(egui::Shape::convex_polygon(
                        vec![
                            tip,
                            tip - way * HEAD_PT + side * 5.0,
                            tip - way * HEAD_PT - side * 5.0,
                        ],
                        ink,
                        Stroke::NONE,
                    ));
                    p.circle_filled(start, 2.5, ink);
                    if r.hovered() || r.dragged() {
                        hint = Some(egui::CursorIcon::Grabbing);
                    }
                    let px_of = |pos: egui::Pos2| ((pos.x - view.min.x) * ppp, (pos.y - view.min.y) * ppp);
                    // from where the button went down, not where the drag was first noticed
                    if r.drag_started()
                        && let Some(pos) = ui.input(|i| i.pointer.press_origin())
                    {
                        e.press = along_axis(&camera, size, px_of(pos), e.origin, dir).unwrap_or(0.0);
                        e.start = e.value;
                        e.dragging = true;
                    }
                    if r.dragged()
                        && e.dragging
                        && let Some(pos) = r.interact_pointer_pos()
                        && let Some(s) = along_axis(&camera, size, px_of(pos), e.origin, dir)
                    {
                        let mm_per_px = 1.0 / (per_mm_pt * ppp) as f64;
                        let want = tidy(e.start + (s - e.press) * e.per_mm, mm_per_px);
                        if want != e.value && want > 0.0 {
                            e.value = want;
                            e.text = super::fg::short(want);
                            acts.push(FgAct::DimPreview(e.ops(want)));
                        }
                    }
                    if r.drag_stopped() && e.dragging {
                        e.dragging = false;
                        if e.value != e.was {
                            acts.push(FgAct::DimCommit(e.ops(e.value)));
                        } else {
                            acts.push(FgAct::DimCancel);
                        }
                    }
                } else {
                    p.line_segment([start, tip], Stroke::new(1.5, t.ink2));
                    p.circle_filled(start, 2.5, t.ink2);
                }
            });
        if let Some(c) = hint {
            ctx.set_cursor_icon(c);
        }
        // the value beside the arrow's head, its word before it
        let word = format!("{} {}", e.feat, e.what);
        let word_w = ctx
            .fonts_mut(|f| f.layout_no_wrap(word.clone(), sans(12.0), t.ink))
            .size()
            .x
            + 18.0;
        let at = tip + way * 14.0 + vec2(if way.x < 0.0 { -FIELD_W } else { word_w }, -13.0);
        let at = pos2(
            at.x.clamp(
                view.left() + 4.0 + word_w,
                (view.right() - FIELD_W - 60.0).max(view.left()),
            ),
            at.y.clamp(view.top() + 4.0, (view.bottom() - 60.0).max(view.top())),
        );
        egui::Area::new(egui::Id::unique("fg-dim-field"))
            .fixed_pos(at)
            .order(egui::Order::Foreground)
            .constrain(false)
            .show(ctx, |ui| {
                let (row, _) = ui.allocate_exact_size(vec2(FIELD_W + 54.0, 26.0), Sense::hover());
                let field = Rect::from_min_size(row.min, vec2(FIELD_W, 26.0));
                ui.painter().rect_filled(field, 4.0, t.surface);
                let id = egui::Id::unique("fg-dim-text");
                let draggable = e.draggable();
                let resp = ui.put(
                    field,
                    egui::TextEdit::singleline(&mut e.text)
                        .id(id)
                        .font(mono(12.0))
                        .desired_width(FIELD_W - 14.0)
                        .margin(vec2(7.0, 6.0))
                        .interactive(draggable)
                        .frame(egui::Frame::NONE),
                );
                self.places.push(("dim:field".to_string(), field));
                self.view_blocks.push(row);
                let edge = if e.error.is_some() {
                    t.error
                } else if resp.has_focus() {
                    t.focus
                } else {
                    t.line
                };
                ui.painter()
                    .rect_stroke(field, 4.0, Stroke::new(1.0, edge), StrokeKind::Inside);
                if let Target::Driven(ks) = &e.target {
                    let by: Vec<String> = ks.iter().map(ToString::to_string).collect();
                    resp.clone()
                        .on_hover_text(format!("driven by {}: edit those", by.join(", ")));
                }
                let enter = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                if enter && e.text.trim() != super::fg::short(e.was) {
                    match super::fg::parse_value(&e.text, Dim::Length) {
                        Some(v) if v.dim == Dim::Length && v.value > 0.0 => {
                            e.error = None;
                            e.value = v.value;
                            acts.push(FgAct::DimCommit(e.ops(v.value)));
                        }
                        _ => e.error = Some(format!("{:?} is not a length", e.text)),
                    }
                }
                // the word sits on its own chip: it may be over the part
                let g = ui.fonts_mut(|f| f.layout_no_wrap(word.clone(), sans(12.0), t.ink));
                let chip = Rect::from_min_size(
                    pos2(field.left() - g.size().x - 16.0, field.top() + 2.0),
                    vec2(g.size().x + 12.0, 22.0),
                );
                ui.painter().rect_filled(chip, 4.0, t.surface);
                ui.painter().galley(
                    pos2(chip.left() + 6.0, chip.center().y - g.size().y * 0.5),
                    g,
                    t.ink,
                );
                if let Some(why) = &e.error {
                    let g = ui
                        .fonts_mut(|f| f.layout(why.clone(), theme::sans(12.0), egui::Color32::WHITE, 280.0));
                    let bx =
                        Rect::from_min_size(field.left_bottom() + vec2(0.0, 4.0), g.size() + vec2(14.0, 8.0));
                    ui.painter().rect_filled(bx, 4.0, t.error);
                    ui.painter()
                        .galley(bx.min + vec2(7.0, 4.0), g, egui::Color32::WHITE);
                    self.places.push(("dim:error".to_string(), bx));
                }
            });
        if let Some(p) = &mut self.fg {
            p.out.extend(acts);
        }
        self.fg_dim = Some(e);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cursor_on_the_axis_reads_its_real_distance() {
        let mut c = Camera::fit([0.0; 3], 50.0);
        c.yaw = 0.7;
        c.pitch = 0.4;
        let size = (1000, 800);
        let (at, dir) = ([3.0f64, -2.0, 1.0], [0.0f64, 0.0, 1.0]);
        for s in [-20.0f32, 0.0, 7.5, 30.0] {
            let p = [at[0] as f32, at[1] as f32, at[2] as f32 + s];
            let px = c.pixel_of(p, size.0, size.1).unwrap();
            let got = along_axis(&c, size, px, at, dir).unwrap();
            assert!((got - s as f64).abs() < 1e-2, "{s}: {got}");
        }
        c.pitch = core::f32::consts::FRAC_PI_2;
        c.target = [3.0, -2.0, 1.0];
        assert_eq!(along_axis(&c, size, (500.0, 400.0), at, dir), None);
    }

    #[test]
    fn drags_land_on_tidy_values() {
        assert!((tidy(6.13, 0.05) - 6.1).abs() < 1e-9);
        assert!((tidy(6.13, 0.5) - 6.0).abs() < 1e-9);
        assert!((tidy(6.137, 0.002) - 6.14).abs() < 1e-9);
    }

    #[test]
    fn a_normal_goes_round_its_octahedral_form_and_back() {
        for n in [
            [0.0, 0.0, 1.0],
            [0.0, 0.0, -1.0],
            [0.6, -0.8, 0.0],
            [0.3, 0.4, -0.866],
        ] {
            let n = unit(n);
            let s = n[0].abs() + n[1].abs() + n[2].abs();
            let (mut x, mut y) = (n[0] / s, n[1] / s);
            if n[2] < 0.0 {
                (x, y) = ((1.0 - y.abs()) * x.signum(), (1.0 - x.abs()) * y.signum());
            }
            let back = unoct((x * 32767.0).round() as i16, (y * 32767.0).round() as i16);
            assert!(dot3(back, n) > 0.9999, "{n:?} {back:?}");
        }
    }
}
