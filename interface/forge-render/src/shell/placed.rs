//! A window or area shown so what it names for clicks by name is where it's drawn: egui lays a
//! new one out once unseen to find its size (the sizing pass), and what it names then is off.

use egui::{Context, InnerResponse, Rect, Ui};

pub(crate) type Places = Vec<(String, Rect)>;

pub(crate) trait Placed: Sized {
    type Out<R>;

    /// Shown, and whether this was its unseen sizing pass.
    fn show_sizing<R>(self, ctx: &Context, add: impl FnOnce(&mut Ui) -> R) -> (Self::Out<R>, bool);

    /// Shown, keeping what it names in `places` only when it's drawn where it's seen.
    fn show_placed<R>(
        self,
        ctx: &Context,
        places: &mut Places,
        add: impl FnOnce(&mut Ui, &mut Places) -> R,
    ) -> Self::Out<R> {
        let before = places.len();
        let (r, sizing) = self.show_sizing(ctx, |ui| add(ui, places));
        if sizing {
            places.truncate(before);
        }
        r
    }
}

impl Placed for egui::Window<'_> {
    type Out<R> = Option<InnerResponse<Option<R>>>;

    fn show_sizing<R>(self, ctx: &Context, add: impl FnOnce(&mut Ui) -> R) -> (Self::Out<R>, bool) {
        let mut sizing = false;
        let r = self.show(ctx, |ui| {
            sizing = ui.is_sizing_pass();
            add(ui)
        });
        (r, sizing)
    }
}

impl Placed for egui::Area {
    type Out<R> = InnerResponse<R>;

    fn show_sizing<R>(self, ctx: &Context, add: impl FnOnce(&mut Ui) -> R) -> (Self::Out<R>, bool) {
        let mut sizing = false;
        let r = self.show(ctx, |ui| {
            sizing = ui.is_sizing_pass();
            add(ui)
        });
        (r, sizing)
    }
}

/// Where a dialog is shown: floating over the view, or docked in a rect of the environment's
/// properties column (docs/ux/STANDARD.md: docked panels, not floating windows). The same body
/// draws in either; docked, it scrolls when taller than its rect.
pub(crate) enum Host<'a> {
    Float(Box<egui::Window<'a>>),
    Dock(egui::Id, Rect, egui::Frame),
}

impl Placed for Host<'_> {
    type Out<R> = Option<InnerResponse<Option<R>>>;

    fn show_sizing<R>(self, ctx: &Context, add: impl FnOnce(&mut Ui) -> R) -> (Self::Out<R>, bool) {
        match self {
            Host::Float(w) => (*w).show_sizing(ctx, add),
            Host::Dock(id, r, frame) => {
                let mut sizing = false;
                let ir = egui::Area::new(id)
                    .fixed_pos(r.min)
                    .order(egui::Order::Middle)
                    .fade_in(false)
                    .show(ctx, |ui| {
                        ui.set_width(r.width());
                        ui.set_max_height(r.height());
                        // nothing past the rect (what is under it, a section below, keeps its clicks)
                        ui.set_clip_rect(r);
                        // the frame's own margins and stroke are inside the rect too
                        let m = frame.total_margin().sum().y;
                        frame
                            .show(ui, |ui| {
                                egui::ScrollArea::vertical()
                                    .id_salt(id.with("scroll"))
                                    .max_height((r.height() - m - 2.0).max(40.0))
                                    .auto_shrink([false, true])
                                    .show(ui, |ui| {
                                        sizing = ui.is_sizing_pass();
                                        add(ui)
                                    })
                                    .inner
                            })
                            .inner
                    });
                (Some(InnerResponse::new(Some(ir.inner), ir.response)), sizing)
            }
        }
    }
}
