//! The selected feature's number on the model: an arrow along its real axis that drags in real
//! millimetres, its value beside it in a field, and a clear word if a value can't build.

use crate::shell::placed::Placed;
use super::theme::mono;
use super::{Action, Shell, icons, theme};
use crate::camera::{Camera, dot};
use egui::{Color32, Rect, Sense, Stroke, StrokeKind, pos2, vec2};
use forge_agent::handles::Handle;
use forge_agent::short;

/// How long the arrow's drawn, in points.
const ARROW_PT: f32 = 64.0;
const HEAD_PT: f32 = 11.0;
const FIELD_W: f32 = 92.0;

#[derive(Clone, Debug, PartialEq)]
pub struct DimEdit {
    pub handle: Handle,
    /// What the field holds.
    pub text: String,
    /// The value being shown: moves as a drag goes, before it's committed.
    pub value: f64,
    pub dragging: bool,
    pub error: Option<String>,
    /// The drag's start on the axis, in mm from the handle's origin.
    press: f64,
    start: f64,
}

impl DimEdit {
    pub fn new(handle: Handle) -> DimEdit {
        DimEdit::in_units(handle, "mm")
    }

    /// As the document reads: what's written, in its unit if it's a bare number.
    pub fn in_units(mut handle: Handle, units: &str) -> DimEdit {
        if let Some(kind) = super::panel::field_kind(handle.label) {
            handle.text = forge_doc::units::field_show(&handle.text, kind, units);
        }
        DimEdit {
            text: handle.text.clone(),
            value: handle.value,
            dragging: false,
            error: None,
            press: 0.0,
            start: handle.value,
            handle,
        }
    }

    /// Where the arrow starts while the value's moved from what it was.
    fn base(&self) -> [f64; 3] {
        let s = (self.value - self.handle.value) / self.handle.per_mm;
        [0, 1, 2].map(|k| self.handle.origin[k] + self.handle.dir[k] * s)
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

impl Shell {
    pub(super) fn dim_ui(&mut self, ctx: &egui::Context) {
        let t = self.theme;
        let Some((camera, size)) = self.camera else { return };
        // a feature's dialog holds its numbers while it's up
        if self.dialog.is_some() {
            return;
        }
        let Some(mut e) = self.dim.take() else { return };
        let ppp = ctx.pixels_per_point();
        let view = self.view_rect;
        let to_screen = |px: (f32, f32)| view.min + vec2(px.0, px.1) / ppp;
        let base = e.base();
        let p3 = base.map(|v| v as f32);
        let Some(at_px) = camera.pixel_of(p3, size.0, size.1) else {
            self.dim = Some(e);
            return;
        };
        // a millimetre on screen, here
        let dir = e.handle.dir;
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

        let mut actions = Vec::new();
        let mut typed = None;
        let mut hint = None;
        // before the arrow's response: egui ends a drag on Esc, and that mustn't read as a release
        if e.dragging && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            actions.push(Action::DimCancel);
            e.dragging = false;
            e.text = e.handle.text.clone();
        }
        let placed_before = self.places.len();
        let (_, sizing) = egui::Area::new(egui::Id::unique("dim-arrow"))
            .fixed_pos(view.min)
            .order(egui::Order::Middle)
            .interactable(true)
            .constrain(false)
            .show_sizing(ctx, |ui| {
                let p = ui.painter().clone();
                let side = vec2(-way.y, way.x);
                let usable = facing < 0.97 && e.handle.draggable;
                if usable {
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
                        e.press = along_axis(&camera, size, px_of(pos), e.handle.origin, dir).unwrap_or(0.0);
                        e.start = e.value;
                        e.dragging = true;
                    }
                    if r.dragged()
                        && e.dragging
                        && let Some(pos) = r.interact_pointer_pos()
                        && let Some(s) = along_axis(&camera, size, px_of(pos), e.handle.origin, dir)
                    {
                        let mm_per_px = 1.0 / (per_mm_pt * ppp) as f64;
                        let want = tidy(e.start + (s - e.press) * e.handle.per_mm, mm_per_px);
                        if want != e.value {
                            actions.push(Action::DimDrag(want));
                            e.text = match super::panel::field_kind(e.handle.label) {
                                Some(k) => forge_doc::units::number_in(want, k, self.units),
                                None => short(want),
                            };
                        }
                    }
                    if r.drag_stopped() && e.dragging {
                        actions.push(Action::DimCommit(e.value));
                    }
                }
            });
        // what it named unseen, sizing itself, is off
        if sizing {
            self.places.truncate(placed_before);
        }
        if let Some(c) = hint {
            ctx.set_cursor_icon(c);
        }

        // the value beside the arrow's head, in its way
        // clear of the arrow's head, with room for the word to the field's left
        let word_w = ctx
            .fonts_mut(|f| f.layout_no_wrap(e.handle.label.to_string(), theme::sans(12.0), t.ink))
            .size()
            .x
            + 18.0;
        let at = tip + way * 14.0 + vec2(if way.x < 0.0 { -FIELD_W } else { word_w }, -13.0);
        let at = pos2(
            at.x.clamp(view.left() + 4.0 + word_w, view.right() - FIELD_W - 60.0),
            at.y.clamp(view.top() + 4.0, view.bottom() - 60.0),
        );
        let placed_before = self.places.len();
        let (_, sizing) = egui::Area::new(egui::Id::unique("dim-field"))
            .fixed_pos(at)
            .order(egui::Order::Foreground)
            .constrain(false)
            .show_sizing(ctx, |ui| {
                let (row, _) = ui.allocate_exact_size(vec2(FIELD_W + 54.0, 26.0), Sense::hover());
                let field = Rect::from_min_size(row.min, vec2(FIELD_W, 26.0));
                ui.painter().rect_filled(field, 4.0, t.surface);
                let id = egui::Id::unique("dim-text");
                let resp = ui.put(
                    field,
                    egui::TextEdit::singleline(&mut e.text)
                        .id(id)
                        .font(mono(12.0))
                        .desired_width(FIELD_W - 14.0)
                        .margin(vec2(7.0, 6.0))
                        .align(egui::Align2::LEFT_CENTER)
                        .frame(egui::Frame::NONE),
                );
                self.places.push(("dim:field".to_string(), field));
                let edge = if e.error.is_some() {
                    t.error
                } else if resp.has_focus() {
                    t.focus
                } else {
                    t.line
                };
                ui.painter()
                    .rect_stroke(field, 4.0, Stroke::new(1.0, edge), StrokeKind::Inside);
                if let Some(k) = super::panel::field_kind(e.handle.label)
                    && forge_doc::units::is_bare(&e.text)
                {
                    ui.painter().text(
                        pos2(field.right() - 6.0, field.center().y),
                        egui::Align2::RIGHT_CENTER,
                        forge_doc::units::unit_of(k, self.units),
                        theme::mono(11.0),
                        t.ink2,
                    );
                }
                let changed = e.text != e.handle.text;
                if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) && changed {
                    typed = Some(self.dim_file(&e));
                }
                if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    e.text = e.handle.text.clone();
                    e.error = None;
                }
                if changed && !e.dragging {
                    let ok = Rect::from_min_size(pos2(field.right() + 2.0, field.top()), vec2(24.0, 26.0));
                    let no = Rect::from_min_size(pos2(ok.right() + 2.0, field.top()), vec2(24.0, 26.0));
                    if self.mini_button(ui, "dim:ok", ok, "finish", "OK").clicked() {
                        typed = Some(self.dim_file(&e));
                    }
                    if self
                        .mini_button(ui, "dim:cancel", no, "close", "Cancel")
                        .clicked()
                    {
                        e.text = e.handle.text.clone();
                        e.error = None;
                    }
                }
                // the word sits on its own chip: it may be over the part
                let word =
                    ui.fonts_mut(|f| f.layout_no_wrap(e.handle.label.to_string(), theme::sans(12.0), t.ink));
                let chip = Rect::from_min_size(
                    pos2(field.left() - word.size().x - 16.0, field.top() + 2.0),
                    vec2(word.size().x + 12.0, 22.0),
                );
                ui.painter().rect_filled(chip, 4.0, t.surface);
                ui.painter().galley(
                    pos2(chip.left() + 6.0, chip.center().y - word.size().y * 0.5),
                    word,
                    t.ink,
                );
                if let Some(why) = &e.error {
                    let g = ui.fonts_mut(|f| f.layout(why.clone(), theme::sans(12.0), Color32::WHITE, 280.0));
                    let bx =
                        Rect::from_min_size(field.left_bottom() + vec2(0.0, 4.0), g.size() + vec2(14.0, 8.0));
                    ui.painter().rect_filled(bx, 4.0, t.error);
                    ui.painter().galley(bx.min + vec2(7.0, 4.0), g, Color32::WHITE);
                    self.places.push(("dim:error".to_string(), bx));
                }
            });
        // what it named unseen, sizing itself, is off
        if sizing {
            self.places.truncate(placed_before);
        }
        self.out.extend(actions);
        if typed.is_some() {
            self.dim_typed = typed;
        }
        self.dim = Some(e);
    }

    fn dim_file(&self, e: &DimEdit) -> String {
        let Some(kind) = super::panel::field_kind(e.handle.label) else {
            return e.text.clone();
        };
        let is_param = |w: &str| self.is_param(w);
        forge_doc::units::field_file(&e.text, kind, self.units, &is_param)
    }

    /// Does the part have a parameter called this?
    pub fn is_param(&self, w: &str) -> bool {
        self.doc.as_ref().is_some_and(|d| {
            d.items
                .iter()
                .any(|i| matches!(&i.body, forge_doc::Body::Param { name, .. } if name == w))
        })
    }

    fn mini_button(
        &mut self,
        ui: &mut egui::Ui,
        name: &str,
        r: Rect,
        icon: &str,
        tip: &str,
    ) -> egui::Response {
        let t = self.theme;
        let resp = ui
            .interact(r, ui.scope_id().with(name), Sense::click())
            .on_hover_text(tip);
        resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, tip));
        ui.painter()
            .rect_filled(r, 4.0, if resp.hovered() { t.hover } else { t.surface });
        ui.painter()
            .rect_stroke(r, 4.0, Stroke::new(1.0, t.line), StrokeKind::Inside);
        icons::draw(
            ui.painter(),
            pos2(r.center().x - 7.0, r.center().y - 7.0),
            14.0,
            icon,
            1.5,
            t.ink,
        );
        self.places.push((name.to_string(), r));
        resp
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
        // straight down the axis it can't say
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
}
