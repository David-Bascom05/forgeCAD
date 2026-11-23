//! File > Export STL / 3MF / OBJ: the writer's options before the host asks where. Chord and
//! angle tolerance; STL's ASCII or binary; OBJ's exact normals and a group per face; 3MF's flat
//! (each occurrence an object where it is). Export hands the host `Action::ExportMesh`, the
//! options kept here for it (`Shell::mesh_opts`); the result (watertight, manifold, pinches)
//! comes back in the status bar.

use super::theme::{self, S1, S2};
use super::{Action, Shell};
use egui::{Rect, pos2, vec2};
use forge_lang::meshfile::{Format, Options};

const W: f32 = 300.0;

impl Shell {
    /// File > Export's item: the options dialog for that format, as last set.
    pub fn mesh_export_open(&mut self, f: Format) {
        let mut o = self.mesh_opts.clone().unwrap_or_else(|| Options::new(f));
        o.format = f;
        self.mesh_dialog = Some(o);
    }

    pub(super) fn mesh_export_ui(&mut self, ctx: &egui::Context) {
        let Some(mut o) = self.mesh_dialog.take() else {
            return;
        };
        let t = self.theme;
        let v = self.view_rect;
        let mut places: Vec<(String, Rect)> = vec![];
        let (mut export, mut cancel, mut close) = (false, false, false);
        let title = format!("Export {}", o.format.ext().to_uppercase());
        let asm = self.is_asm();
        let shown = egui::Window::new("Export Mesh")
            .id(egui::Id::unique("mesh-export-dialog"))
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .movable(true)
            .fade_in(false)
            .default_pos(pos2(v.center().x - W * 0.5, v.top() + 40.0))
            .fixed_size(vec2(W, 0.0))
            .frame(theme::panel_frame(&t))
            .constrain_to(super::dialog_area(ctx))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                close = super::fgasm::dialog_head(ui, &t, "export", &title, "mesh", &mut places);
                let row =
                    |ui: &mut egui::Ui, label: &str, add: &mut dyn FnMut(&mut egui::Ui) -> egui::Response| {
                        ui.horizontal(|ui| {
                            let (r, _) =
                                ui.allocate_exact_size(vec2(120.0, theme::CONTROL_H), egui::Sense::hover());
                            ui.painter().text(
                                pos2(r.left(), r.center().y),
                                egui::Align2::LEFT_CENTER,
                                label,
                                theme::body(),
                                t.ink,
                            );
                            add(ui)
                        })
                        .inner
                    };
                let r = row(ui, "Chord tolerance", &mut |ui| {
                    ui.add(
                        egui::DragValue::new(&mut o.tol)
                            .speed(0.001)
                            .range(0.001..=10.0)
                            .max_decimals(3)
                            .suffix(" mm"),
                    )
                    .on_hover_text(
                        "How far a facet may stand off the true surface: smaller is finer and larger",
                    )
                });
                places.push(("mesh:tol".into(), r.rect));
                let r = row(ui, "Angle tolerance", &mut |ui| {
                    ui.add(
                        egui::DragValue::new(&mut o.angle)
                            .speed(0.1)
                            .range(0.5..=45.0)
                            .max_decimals(1)
                            .suffix("°"),
                    )
                    .on_hover_text("How far neighbouring facets' normals may turn")
                });
                places.push(("mesh:angle".into(), r.rect));
                ui.add_space(S1);
                match o.format {
                    Format::Stl => {
                        let words = [("Binary", !o.ascii), ("ASCII", o.ascii)];
                        if let Some(k) = super::dialog::text_toggles(ui, &t, &words, "mesh:stl", &mut places)
                        {
                            o.ascii = k == 1;
                        }
                    }
                    Format::Obj => {
                        let r = ui.checkbox(
                            &mut o.normals,
                            egui::RichText::new("Exact normals at each corner").font(theme::body()),
                        );
                        places.push(("mesh:normals".into(), r.rect));
                        let r = ui.checkbox(
                            &mut o.faces,
                            egui::RichText::new("A group per face").font(theme::body()),
                        );
                        places.push(("mesh:faces".into(), r.rect));
                    }
                    Format::ThreeMf => {
                        let r = ui.add_enabled(
                            asm,
                            egui::Checkbox::new(
                                &mut o.flat,
                                egui::RichText::new("Flat: each occurrence an object where it is")
                                    .font(theme::body()),
                            ),
                        );
                        let r = if asm {
                            r
                        } else {
                            r.on_disabled_hover_text("Only an assembly has occurrences to place")
                        };
                        places.push(("mesh:flat".into(), r.rect));
                    }
                }
                ui.add_space(S2);
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let b = ui.add(
                            egui::Button::new(
                                egui::RichText::new("Export…")
                                    .font(theme::medium(13.0))
                                    .color(egui::Color32::WHITE),
                            )
                            .fill(t.accent)
                            .min_size(vec2(84.0, theme::CONTROL_H)),
                        );
                        places.push(("mesh:export".into(), b.rect));
                        export = b.clicked();
                        let b = ui.add(
                            egui::Button::new(egui::RichText::new("Cancel").font(theme::body()))
                                .min_size(vec2(74.0, theme::CONTROL_H)),
                        );
                        places.push(("mesh:cancel".into(), b.rect));
                        cancel = b.clicked();
                    });
                });
            });
        if let Some(r) = shown {
            self.view_blocks.push(r.response.rect);
            places.push(("mesh:dialog".into(), r.response.rect));
        }
        self.places.extend(places);
        let esc = ctx.input(|i| i.key_pressed(egui::Key::Escape));
        if export {
            self.out.push(Action::ExportMesh(o.format));
            self.mesh_opts = Some(o);
        } else if !(cancel || close || esc) {
            self.mesh_dialog = Some(o);
        }
    }
}

/// The status bar's word on an export: closed and manifold, or what isn't.
pub fn said(file: &str, e: &forge_lang::meshfile::Exported) -> String {
    let objects = e.checks.len();
    if e.watertight() && e.manifold() {
        return format!(
            "Exported {file}: {objects} object{}, {} triangles, watertight and manifold",
            if objects == 1 { "" } else { "s" },
            e.triangles()
        );
    }
    let open: usize = e.checks.iter().map(|c| c.open_edges).sum();
    let pinched: usize = e.checks.iter().map(|c| c.pinched).sum();
    let mut why = vec![];
    if open > 0 {
        why.push(format!("{open} open edge{}", if open == 1 { "" } else { "s" }));
    }
    if pinched > 0 {
        why.push(format!(
            "{pinched} pinched vertex{}",
            if pinched == 1 { "" } else { "es" }
        ));
    }
    if why.is_empty() {
        why.push("not closed".into());
    }
    format!(
        "Exported {file}: {} triangles, but {}: check it before printing",
        e.triangles(),
        why.join(" and ")
    )
}
