//! What of the origin the view draws: a Forge file's planes and axes as the view's switches have
//! them, an assembly's Origin rows' eyes, and what's hovered or picked of it.

use super::Shell;
use crate::origin::{self, Item};

impl Shell {
    /// What of the origin's drawn, each and how strongly.
    pub fn origin_drawn(&self) -> Vec<(Item, bool)> {
        let o = self.overlays();
        origin::ITEMS
            .into_iter()
            .enumerate()
            .filter_map(|(k, i)| {
                let strong = self.origin_hover == Some(i) || self.origin_selected == Some(i);
                // an assembly's Origin rows have their own eyes
                let on = if k < 3 { o.planes } else { o.axes } || (self.is_asm() && self.origin_shown[k]);
                // a sketch being started shows the planes it can go on, as Inventor does
                let on = on || (k < 3 && self.sketch_pick);
                (on || strong).then_some((i, strong))
            })
            .collect()
    }
}
