//! Open documents, Inventor's way: one active, each with its own browser, view, selection and
//! undo. The active one's state lives in `Shell`'s (and `Gui`'s) own fields, so nothing that reads
//! them changes; an idle one's is swapped out into a `ShellDoc`.

use super::Shell;

/// Splits a struct's fields into the document's and the app's. The document's go into `$doc`,
/// swapped in and out whole; the app's stay. Every field must be named in one list or the other,
/// so a field added later fails to build until it's placed.
macro_rules! per_doc {
    ($(#[$m:meta])* $host:ident, $doc:ident, [$($f:ident: $t:ty),* $(,)?], app [$($a:ident),* $(,)?]) => {
        $(#[$m])*
        pub struct $doc {
            $(pub(crate) $f: $t,)*
        }

        impl $host {
            /// This document's state in, the one that was in out to `d`.
            pub fn swap_doc(&mut self, d: &mut $doc) {
                $(std::mem::swap(&mut self.$f, &mut d.$f);)*
            }

            #[allow(dead_code)]
            fn every_field_placed(&self) {
                let $host { $($f: _,)* $($a: _,)* } = self;
            }
        }
    };
}
pub(crate) use per_doc;

per_doc!(
    Shell,
    ShellDoc,
    [
        tab: usize,
        name: String,
        doc: Option<forge_doc::Document>,
        rows: Vec<super::Row>,
        failed: Option<(forge_doc::ItemId, String)>,
        selected: Option<forge_doc::ItemId>,
        version: u64,
        status: String,
        view_tex: Option<egui::TextureId>,
        cube_labels: Vec<crate::viewcube::Label>,
        cube_place: Option<crate::viewcube::Place>,
        camera: Option<(crate::camera::Camera, (u32, u32))>,
        origin_open: bool,
        origin_shown: [bool; 7],
        origin_hover: Option<crate::origin::Item>,
        origin_tree_hover: Option<crate::origin::Item>,
        origin_selected: Option<crate::origin::Item>,
        tree_hover: Option<forge_doc::ItemId>,
        end_at: Option<forge_doc::ItemId>,
        renaming: Option<(forge_doc::ItemId, String)>,
        renamed: Option<(forge_doc::ItemId, String)>,
        sketch_shapes: Vec<forge_agent::handles::SketchShape>,
        sketch_eye: std::collections::HashMap<String, bool>,
        sketch_hover: Option<forge_doc::ItemId>,
        unshown: bool,
        flags: Vec<forge_agent::live::Flag>,
        checking_since: Option<f64>,
        units: &'static str,
        part_middle: [f32; 3],
        sketch: Option<super::sketching::SketchMode>,
        sketch_pick: bool,
        open_rows: std::collections::HashSet<forge_doc::ItemId>,
        then_feature: Option<&'static str>,
        dialog: Option<super::dialog::Dialog>,
        dim: Option<super::dim::DimEdit>,
        content: super::content::ContentCenter,
        dim_typed: Option<String>,
        fg: Option<super::fg::FgPanel>,
        sheet: super::sheet::SheetUi,
        inspect: super::inspect::Inspect,
        drawing: Option<super::drawing::DrawingEnv>,
        new_drawing: Option<super::drawing::NewDrawing>,
    ],
    app [
        theme, view, out, view_rect, places, cube_menu, cube_menu_fresh, cube_region, marking,
        marking_row, window_box, view_blocks, side_px, overlays, file_menu, style_set, settings,
        prefs, agent,
    ]
);

impl Default for ShellDoc {
    /// A document with nothing in it: as a new shell has it.
    fn default() -> ShellDoc {
        let mut d = ShellDoc::blank();
        Shell::default().swap_doc(&mut d);
        d
    }
}

impl ShellDoc {
    /// Placeholders, swapped out at once.
    fn blank() -> ShellDoc {
        ShellDoc {
            tab: 0,
            name: String::new(),
            doc: None,
            rows: vec![],
            failed: None,
            selected: None,
            version: 0,
            status: String::new(),
            view_tex: None,
            cube_labels: vec![],
            cube_place: None,
            camera: None,
            origin_open: false,
            origin_shown: [false; 7],
            origin_hover: None,
            origin_tree_hover: None,
            origin_selected: None,
            tree_hover: None,
            end_at: None,
            renaming: None,
            renamed: None,
            sketch_shapes: vec![],
            sketch_eye: Default::default(),
            sketch_hover: None,
            unshown: false,
            flags: vec![],
            checking_since: None,
            units: "mm",
            part_middle: [0.0; 3],
            sketch: None,
            sketch_pick: false,
            open_rows: Default::default(),
            then_feature: None,
            dialog: None,
            dim: None,
            content: Default::default(),
            dim_typed: None,
            fg: None,
            sheet: super::sheet::SheetUi::new(),
            inspect: Default::default(),
            drawing: None,
            new_drawing: None,
        }
    }
}
