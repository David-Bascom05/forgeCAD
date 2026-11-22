//! Shared by the `kernel_*` regression tests: a Forge part built, its final solids and body, and
//! the kernel's features to regenerate on their own (n-ary, by stage).
#![allow(dead_code)]

use crate::common;
use forge_model::{Feature, Part, RegenError, Solid};
use forge_topo::Body;

/// A part built without error: its run's final solids, and the kernel features it sent.
pub struct Built {
    pub run: forge_lang::Run,
    pub solids: Vec<Solid>,
    /// The solids as one, faces on one surface joined: what the part shows.
    pub body: Body,
    pub features: Vec<Feature>,
    pub ids: Vec<u64>,
}

/// Builds the part; panics on any error.
pub fn built(src: &str) -> Built {
    let (run, cache) = common::run(src);
    assert!(run.errors.is_empty(), "{}", forge_lang::report::text(&run, src));
    let stage = cache.measured.last().expect("measured").0.clone();
    let (f, ids) = cache.history();
    Built {
        solids: stage.solids.clone(),
        body: stage.view().expect("a body").clone(),
        features: f.to_vec(),
        ids: ids.to_vec(),
        run,
    }
}

impl Built {
    pub fn part(&self) -> Part {
        Part {
            features: self.features.clone(),
        }
    }

    /// The kernel's regeneration in sequence.
    pub fn seq(&self) -> Result<Vec<Solid>, RegenError> {
        self.part().regenerate_solids_ids(Some(&self.ids))
    }

    /// The kernel's regeneration in n-ary booleans on `threads` threads.
    pub fn nary(&self, threads: usize) -> Result<Vec<Solid>, RegenError> {
        self.part().regenerate_nary_solids_ids(threads, Some(&self.ids))
    }

    /// The solids after each kernel feature, in sequence.
    pub fn stages(&self) -> Vec<Vec<Solid>> {
        let h = forge_model::History::new(&self.features, Some(&self.ids));
        let mut solids = Vec::new();
        let mut out = Vec::new();
        for i in 0..self.features.len() {
            solids = forge_model::step_solids(h, i, solids).expect("builds");
            out.push(solids.clone());
        }
        out
    }

    /// The body's exact volume.
    pub fn volume(&self) -> f64 {
        forge_topo::mass::mass(&self.body).expect("a mass").volume
    }
}

/// The solids as one body.
pub fn joined(solids: &[Solid]) -> Body {
    match solids {
        [one] => one.body.clone(),
        many => {
            let mut all = Body::new();
            for s in many {
                all.append(&s.body.compact());
            }
            all
        }
    }
}

/// The volume of the body's mesh at this chord.
pub fn mesh_volume(b: &Body, chord: f64) -> f64 {
    let q = forge_mesh::Quality {
        chord,
        ..forge_mesh::Quality::default()
    };
    forge_mesh::tessellate(&b.compact(), &q).expect("meshes").volume()
}

/// The part's first error, which must be the kernel's refusal of a feature: that refusal.
pub fn refusal(src: &str) -> String {
    let (run, _) = common::run(src);
    match run.errors.first() {
        Some(forge_lang::error::Error::Build { error, .. }) => format!("{error:?}"),
        other => panic!("expected the kernel's refusal, got {other:?}"),
    }
}

/// Builds valid in sequence and n-ary (one thread), the two the same volume by meshes at
/// `chord`; that volume.
pub fn builds_alike(src: &str, chord: f64) -> f64 {
    let b = built(src);
    let p = forge_topo::check(&b.body);
    assert!(p.is_empty(), "{p:?}");
    let n = joined(&b.nary(1).expect("n-ary"));
    let p = forge_topo::check(&n);
    assert!(p.is_empty(), "n-ary: {p:?}");
    let (vs, vn) = (mesh_volume(&b.body, chord), mesh_volume(&n, chord));
    assert!((vs - vn).abs() < 1e-3 * vs, "seq {vs}, n-ary {vn}");
    vs
}
