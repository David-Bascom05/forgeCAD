//! What the view shows with no document open: where to start, and the keys for it.

use super::theme::{self, S1, S2, S3, S5};
use super::{Action, Shell, sheet};
use egui::{Align2, Rect, Sense, Stroke, StrokeKind, pos2, vec2};

const W: f32 = 360.0;
const BUTTON_H: f32 = 34.0;

impl Shell {
    /// No part, no drawing: a heading, a sentence, New part as the primary, New sheet metal part
    /// and Open beside it, and the shortcuts that do the same.
    pub(super) fn empty_state(&mut self, ui: &mut egui::Ui) {
        let t = self.theme;
        let v = self.view_rect;
        if v.width() < W + 2.0 * S5 || v.height() < 320.0 {
            return;
        }
        let p = ui.painter().clone();
        let mut y = v.center().y - 120.0;
        let x = v.center().x - W * 0.5;
        p.text(
            pos2(v.center().x, y),
            Align2::CENTER_TOP,
            "Start a part",
            theme::heading(),
            t.ink,
        );
        y += 30.0;
        p.text(
            pos2(v.center().x, y),
            Align2::CENTER_TOP,
            "Make a new part, or open a Forge file (.fg) or a DWG drawing.",
            theme::body(),
            t.ink2,
        );
        y += 20.0 + S5;
        let buttons: [(&str, &str, &str, Action, bool); 3] = [
            ("empty:new", "new", "New part", Action::New, true),
            (
                "empty:sheet",
                "sheet",
                "New sheet metal part",
                Action::Sheet(sheet::SheetCmd::NewPart),
                false,
            ),
            ("empty:open", "open", "Open\u{2026}", Action::Open, false),
        ];
        for (name, icon, word, action, primary) in buttons {
            let r = Rect::from_min_size(pos2(x, y), vec2(W, BUTTON_H));
            let resp = ui.interact(r, ui.scope_id().with(name), Sense::click());
            resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, word));
            let r1 = theme::R1;
            let (fill, ink) = if primary {
                let f = if resp.hovered() {
                    t.accent.gamma_multiply(1.15)
                } else {
                    t.accent
                };
                (f, egui::Color32::WHITE)
            } else {
                (if resp.hovered() { t.hover } else { t.surface }, t.ink)
            };
            p.rect_filled(r, r1, fill);
            if !primary {
                p.rect_stroke(r, r1, Stroke::new(1.0, t.line), StrokeKind::Inside);
            }
            super::icons::draw(&p, pos2(r.left() + S3, r.center().y - 9.0), 18.0, icon, 1.5, ink);
            p.text(
                pos2(r.left() + S3 + 18.0 + S2, r.center().y),
                Align2::LEFT_CENTER,
                word,
                if primary {
                    theme::medium(13.0)
                } else {
                    theme::body()
                },
                ink,
            );
            if resp.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            self.note(name, r);
            self.view_blocks.push(r);
            if resp.clicked() {
                self.out.push(action);
            }
            y += BUTTON_H + S2;
        }
        // the keys, as Settings has them now
        y += S3;
        let key = |id: &str| {
            self.prefs
                .cmds
                .iter()
                .find(|c| c.id == id)
                .and_then(|c| self.settings.keys.get(c))
                .map(|k| k.to_string())
        };
        let hints: Vec<(String, &str)> = [("open", "Open a file"), ("undo", "Undo")]
            .into_iter()
            .filter_map(|(id, what)| key(id).map(|k| (k, what)))
            .collect();
        for (k, what) in hints {
            let g = ui.fonts_mut(|f| f.layout_no_wrap(k.clone(), theme::code(), t.ink2));
            let chip = Rect::from_min_size(pos2(x, y), vec2(g.size().x + 2.0 * (S1 + 2.0), g.size().y + S1));
            p.rect_filled(chip, theme::R1, t.raised);
            p.rect_stroke(chip, theme::R1, Stroke::new(1.0, t.line), StrokeKind::Inside);
            p.galley(chip.min + vec2(S1 + 2.0, S1 * 0.5), g, t.ink2);
            p.text(
                pos2(chip.right() + S2, chip.center().y),
                Align2::LEFT_CENTER,
                what,
                theme::small(),
                t.ink2,
            );
            y += chip.height() + S1;
        }
    }
}
