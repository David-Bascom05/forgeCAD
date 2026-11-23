//! What a run holds, read-only, for the app's toolpath and stock simulation (forge-render's
//! camsim): each setup's part, stock, frame and workholding, and each tool's shape. Nothing here
//! decides anything: the verdicts stay the certified checks' (`OpOut::checks`, `Cam::leftover`).

use crate::{Cam, OpPath};
use forge_cam::ops::{Stock, ToolUse};
use std::path::PathBuf;

/// One setup as the simulation draws it.
#[derive(Clone, Debug)]
pub struct SimSetup {
    pub name: String,
    /// The part's source and its directory (to build and mesh it for display).
    pub part_src: String,
    pub part_dir: PathBuf,
    pub part_file: String,
    /// The part turned over about x for this setup: its frame maps the part's (x, y, z) to
    /// (x, −y, h − z) with this h. None: the part as drawn.
    pub turned: Option<f64>,
    /// The part's box in this setup's frame.
    pub part_box: ([f64; 3], [f64; 3]),
    pub stock: Stock,
    /// The WCS origin in the part's frame (the posted text's zero).
    pub origin: [f64; 3],
    pub tol: f64,
    /// The workholding's pieces as boxes (name, min, max), in this setup's frame.
    pub fixture: Vec<(String, [f64; 3], [f64; 3])>,
    /// Its name, if it has one.
    pub fixture_name: Option<String>,
    /// The setup this one continues, turned over.
    pub from: Option<usize>,
}

impl Cam {
    /// Every setup, in order.
    pub fn sim_setups(&self) -> Vec<SimSetup> {
        self.setups
            .iter()
            .map(|s| {
                let p = &self.parts[s.part];
                let (src_part, turned) = match p.turn_of {
                    Some(k) => (&self.parts[k], Some(self.parts[k].h())),
                    None => (p, None),
                };
                SimSetup {
                    name: s.name.clone(),
                    part_src: src_part.src.clone(),
                    part_dir: src_part.pdir.clone(),
                    part_file: src_part.file.clone(),
                    turned,
                    part_box: p.bbox,
                    stock: s.stock,
                    origin: s.origin,
                    tol: s.tol,
                    fixture: s.fixture.as_ref().map(|f| f.boxes()).unwrap_or_default(),
                    fixture_name: s.fixture.as_ref().map(|f| f.name.clone()),
                    from: s.from,
                }
            })
            .collect()
    }

    /// The tools the document holds, by the index an op's paths name.
    pub fn sim_tools(&self) -> &[ToolUse] {
        &self.tools
    }

    /// The paths of op `i` the simulation plays: as posted where a program posted it (what the
    /// checks were run on and their `move_index` counts), else as planned.
    pub fn sim_paths(&self, i: usize) -> &[OpPath] {
        let op = &self.ops[i];
        if op.posted.len() == op.paths.len() && !op.posted.is_empty() {
            &op.posted
        } else {
            &op.paths
        }
    }
}
