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
