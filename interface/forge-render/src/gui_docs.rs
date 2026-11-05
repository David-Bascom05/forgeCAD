//! The open documents: each its own view, camera, build and undo; one active. The active one's
//! state is in `Gui`'s and `Shell`'s fields; an idle one's waits in its `DocState`.

use super::*;
use crate::shell::docs::{ShellDoc, per_doc};

per_doc!(
    #[derive(Default)]
    Gui,
    GuiDoc,
    [
        view: Option<View>,
        loading: Option<Receiver<Result<Loaded, String>>>,
        calls: Vec<Call>,
        view_file: Option<String>,
        tree_hover_was: Option<forge_doc::ItemId>,
        after_sketch: Option<&'static str>,
        sketch_for: Option<(String, SketchFor, Vec<String>)>,
        sketch_sources: std::collections::HashMap<String, Result<Vec<forge_doc::sketch::Refd>, String>>,
        fg: Option<crate::fg::Session>,
        fg_view: Option<Loaded>,
        fg_folded: Option<crate::mesh::PartMesh>,
        fg_flat: Option<crate::mesh::PartMesh>,
        fg_sketch: Option<crate::shell::fgsketch::Opened>,
        fg_sheets: Vec<crate::fg::SheetInfo>,
        fg_walls: Vec<crate::thermo_view::Gfx>,
        fg_anchors: Option<Arc<crate::fg::AnchorView>>,
        fg_asm: Option<AsmPlaced>,
        fg_asm_anchors: Option<Receiver<PartAnchors>>,
        fg_keep_camera: Option<Camera>,
        extras: gui_overlay::Extras,
        fg_asm_boxes: Vec<crate::explode::Aabb>,
        asm: gui_asm::AsmHost,
        explode: gui_explode::Explode,
        body_rows: Option<crate::scene::Rows>,
        inspector: gui_inspect::Inspector,
        fg_previewed: Option<String>,
    ],
    app [
        ctx, egui, shell, device, queue, adapter, waker, captured, over_view, pointer_px,
        pixels_per_point, warm_textures, target_px, mark_held, gpu_failed, settings_was, agent,
        apps,
    ]
);

/// An idle document's state, as the active one's is in `Gui` and `Shell`.
#[derive(Default)]
pub struct DocState {
    pub gui: GuiDoc,
    pub shell: ShellDoc,
}

impl Gui {
    /// The active document out to `d`, and `d`'s in.
    pub fn swap_state(&mut self, d: &mut DocState) {
        self.swap_doc(&mut d.gui);
        self.shell.swap_doc(&mut d.shell);
    }
}
