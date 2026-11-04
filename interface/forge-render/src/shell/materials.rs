//! The Material Browser, Inventor's: the libraries (project, mine, ForgeCAD read-only), their
//! materials, and an editor whose tabs other tools add to (`EXTRA_TABS`).

use super::theme::Theme;
use forge_material::Material;

/// An editor tab another tool adds: its title, whether a material has anything for it, and its
/// contents, which may change the material (true when they did).
pub struct ExtraTab {
    pub title: &'static str,
    pub applies: fn(&Material) -> bool,
    pub ui: fn(&mut egui::Ui, &Theme, &mut Material) -> bool,
}

/// The tabs after Identity, Physical and Appearance, before Sources: thermoforming adds its own
/// here (its values are `thermo.*` keys of the material).
pub const EXTRA_TABS: &[ExtraTab] = &[];
