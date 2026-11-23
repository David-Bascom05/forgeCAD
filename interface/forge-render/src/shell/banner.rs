//! A part that didn't build says so in the view, not only in the status bar's chip: the first
//! error in words, and the way to all of them.

use super::Shell;
use super::fg::Severity;
use super::theme::{self, S1, S2, S3};
use egui::{Align2, Rect, Sense, Stroke, StrokeKind, pos2, vec2};

impl Shell {
    /// Over the view's top: "Build failed", the first error and its line, and Show issues. Gone
    /// while the Issues window is up, in a sketch (the mode bar's place), and in an assembly
    /// (whose errors are its relationships', listed in its own browser).
    pub(super) fn build_banner(&mut self, ui: &mut egui::Ui) {
        if self.sketch.is_some() || self.sketch_pick || self.is_asm() {
            return;
        }
        let t = self.theme;
        let v = self.view_rect;
        let Some(fg) = &self.fg else { return };
        if fg.issues_list || fg.tool.is_some() {
            return;
        }
        let errors: Vec<_> = fg
            .issues
            .iter()
            .filter(|i| i.severity == Severity::Error && i.asm.is_none())
            .collect();
        let Some(first) = errors.first() else { return };
        let more = errors.len() - 1;
        let what = if more > 0 {
            format!("{} · line {} (and {more} more)", first.text, first.line)
        } else {
            format!("{} · line {}", first.text, first.line)
        };
        let max_w = (v.width() - 2.0 * 120.0).clamp(240.0, 720.0);
        let head = ui.fonts_mut(|f| f.layout_no_wrap("Build failed".into(), theme::medium(13.0), t.error));
        let button_w = 96.0;
        let room = max_w - head.size().x - button_w - 18.0 - 4.0 * S2 - 2.0 * S3;
        let body = ui.fonts_mut(|f| {
            let mut job = egui::text::LayoutJob::simple_singleline(what, theme::small(), t.ink);
            job.wrap = egui::text::TextWrapping::truncate_at_width(room.max(40.0));
            f.layout_job(job)
        });
        let h = theme::CONTROL_H + 2.0 * S1 + 2.0;
        let w = 2.0 * S3 + 18.0 + S2 + head.size().x + S2 + body.size().x + 2.0 * S2 + button_w;
        let r = Rect::from_min_size(pos2(v.center().x - w * 0.5, v.top() + 10.0), vec2(w, h));
        let p = ui.painter().clone();
        // the panel's own shadow, by hand: this is painted, not a window
        p.rect_filled(r.translate(vec2(0.0, 2.0)).expand(1.0), theme::R2, t.shadow);
        p.rect_filled(r, theme::R2, t.surface);
        p.rect_stroke(r, theme::R2, Stroke::new(1.0, t.error), StrokeKind::Inside);
        let mut x = r.left() + S3;
        super::icons::draw(&p, pos2(x, r.center().y - 9.0), 18.0, "warning", 1.5, t.error);
        x += 18.0 + S2;
        let hw = head.size();
        p.galley(pos2(x, r.center().y - hw.y * 0.5), head, t.error);
        x += hw.x + S2;
        let bw = body.size();
        p.galley(pos2(x, r.center().y - bw.y * 0.5), body, t.ink);
        let b = Rect::from_min_size(
            pos2(
                r.right() - S1 - 2.0 - button_w,
                r.center().y - theme::CONTROL_H * 0.5,
            ),
            vec2(button_w, theme::CONTROL_H),
        );
        // the whole of the first error, where it's cut short
        ui.interact(r, ui.scope_id().with("build-banner"), Sense::hover())
            .on_hover_text(&first.text);
        let resp = ui.interact(b, ui.scope_id().with("build-banner:issues"), Sense::click());
        resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, "Show issues"));
        p.rect_filled(b, theme::R1, if resp.hovered() { t.hover } else { t.raised });
        p.rect_stroke(b, theme::R1, Stroke::new(1.0, t.line), StrokeKind::Inside);
        p.text(
            b.center(),
            Align2::CENTER_CENTER,
            "Show issues",
            theme::small(),
            t.ink,
        );
        self.view_blocks.push(r);
        self.note("build-banner", r);
        self.note("build-banner:issues", b);
        if resp.clicked()
            && let Some(fg) = &mut self.fg
        {
            fg.issues_list = true;
        }
    }
}
