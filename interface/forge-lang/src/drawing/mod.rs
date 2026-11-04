//! Drawings: a `.fg` document whose first line is `drawing`, its views of a part or assembly
//! (forge-draw's exact hidden-line views), annotations attached to the model's persistent names,
//! and its sheets as PDF, DXF and SVG.

pub mod doc;
pub mod look;
pub mod ops;
pub mod regen;

use forge_math::Frame;
use std::sync::Arc;

/// A model's solids as a view draws them: a part's body once, an assembly's occurrences with
/// their poses and paths.
#[derive(Clone, Default)]
pub struct Model {
    pub bodies: Vec<(Arc<forge_topo::Body>, Frame)>,
    /// Each body's occurrence path (`post_fl.post`), empty for a part.
    pub paths: Vec<String>,
    /// Each body's part file, as the document wrote it.
    pub files: Vec<String>,
}

impl Model {
    /// The finished part or assembly of a run; None if it did not build.
    pub fn of(run: &crate::Run) -> Option<Model> {
        if run.asm.is_none() {
            let b = crate::export::folded(run)?;
            return Some(Model {
                bodies: vec![(Arc::new(b), Frame::WORLD)],
                paths: vec![String::new()],
                files: vec![String::new()],
            });
        }
        let a = run.asm.as_ref()?;
        let mut m = Model::default();
        for (k, (path, file, pose)) in a.occurrences.iter().enumerate() {
            if let Some(Some(p)) = a.occ_part.get(k) {
                m.bodies.push((a.parts[*p].clone(), *pose));
                m.paths.push(path.clone());
                m.files.push(file.clone());
            }
        }
        Some(m)
    }

    pub fn instances(&self) -> Vec<forge_draw::Instance<'_>> {
        self.bodies
            .iter()
            .map(|(b, pose)| forge_draw::Instance { body: b, pose: *pose })
            .collect()
    }
}

/// Whether a source is a drawing (its first line is the word `drawing`).
pub fn is_drawing(src: &str) -> bool {
    crate::syntax::parse(src).is_ok_and(|p| doc::declared(&p))
}
