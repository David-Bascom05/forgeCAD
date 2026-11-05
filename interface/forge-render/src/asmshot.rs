//! An assembly (.fg) for `forge-render --shot`: solved, each part meshed once and placed at every
//! occurrence, with the app's exploded-view plan, so a picture can be taken at any explode amount
//! and from any side of the view cube.

use crate::explode::{Aabb, Item, Plan};
use crate::mesh::PartMesh;
use crate::scene::{Placement, Rows, Scene};
use crate::shell::fgasm::{AsmTree, OccRow};

pub struct AsmStage {
    pub parts: Vec<PartMesh>,
    pub scene: Scene,
    pub plan: Plan,
    /// Each placement's pose and its mesh's box: where its trail starts.
    pub homes: Vec<(Rows, Aabb)>,
}

pub fn stage(path: &str) -> Result<AsmStage, String> {
    let src = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    let dir = std::fs::canonicalize(path)
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()));
    let opts = forge_lang::Options {
        dir: dir.clone(),
        render: true,
        ..forge_lang::Options::default()
    };
    let (run, _) = forge_lang::run(&src, opts, Default::default());
    if let Some(e) = run.errors.first() {
        return Err(format!("{path}: {}: {}", e.kind(), e.text()));
    }
    let out = run.asm.ok_or(format!("{path}: not an assembly"))?;
    // each part once, the finest the app makes
    let at = std::sync::atomic::AtomicUsize::new(0);
    let parts: Vec<PartMesh> = std::thread::scope(|s| {
        let workers: Vec<_> = (0..4)
            .map(|_| {
                s.spawn(|| {
                    let mut got = vec![];
                    loop {
                        let k = at.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        let Some(b) = out.parts.get(k) else { return got };
                        got.push((
                            k,
                            crate::tess::build_part(b).unwrap_or_else(|_| PartMesh::empty()),
                        ));
                    }
                })
            })
            .collect();
        let mut all: Vec<(usize, PartMesh)> = workers
            .into_iter()
            .flat_map(|w| w.join().expect("a mesher panicked"))
            .collect();
        all.sort_by_key(|x| x.0);
        all.into_iter().map(|x| x.1).collect()
    });
    let boxes: Vec<Aabb> = parts.iter().map(crate::explode::mesh_box).collect();
    let rows: Vec<OccRow> = out
        .occurrences
        .iter()
        .zip(&out.occ_part)
        .zip(out.occ_tooling.iter().chain(std::iter::repeat(&None)))
        .map(|(((p, f, _), part), tool)| OccRow {
            path: p.clone(),
            file: f.clone(),
            drawn: part.is_some(),
            tooling: tool.is_some(),
        })
        .collect();
    let tree = AsmTree::build(&rows, dir.as_deref());
    let mut placements = vec![];
    let mut items = vec![];
    let mut homes = vec![];
    for (k, ((_, _, pose), part)) in out.occurrences.iter().zip(&out.occ_part).enumerate() {
        let Some(m) = part else { continue };
        let r = crate::fg::rows_of(pose);
        placements.push(Placement {
            part: *m as u32,
            rows: r,
            tint: crate::fg::PART_TINT,
        });
        items.push(Item {
            rows: r,
            local: boxes[*m],
            group: tree.group[k],
            sub: tree.sub[k],
            ground: out
                .grounded
                .iter()
                .any(|g| crate::shell::fgasm::under(&out.occurrences[k].0, g)),
            axis: tree.axis[k],
        });
        homes.push((r, boxes[*m]));
    }
    if placements.is_empty() {
        return Err(format!("{path}: nothing to draw"));
    }
    let mut scene = Scene::new(&crate::render::part_info(&parts), placements);
    scene.set_part_boxes(boxes);
    Ok(AsmStage {
        parts,
        scene,
        plan: crate::explode::plan(&items),
        homes,
    })
}

impl AsmStage {
    /// Moves the parts `t` of the way apart.
    pub fn explode(&mut self, t: f32) {
        self.scene.set_offsets(&crate::explode::at(&self.plan, t));
    }

    /// Each moved part's trail, home to where it is.
    pub fn trails(
        &self,
        camera: &crate::camera::Camera,
        size: (u32, u32),
        px: f32,
        out: &mut Vec<crate::origin::WorldVert>,
    ) {
        let rgb = crate::viewcube::rgb(0xE0A21B);
        for (pl, (r, b)) in self.homes.iter().enumerate() {
            let d = self.scene.offset_of(pl);
            if d == [0.0; 3] {
                continue;
            }
            let c = [0, 1, 2].map(|k| (b.0[k] + b.1[k]) * 0.5);
            let home = [0, 1, 2].map(|i| r[i][0] * c[0] + r[i][1] * c[1] + r[i][2] * c[2] + r[i][3]);
            let to = [home[0] + d[0], home[1] + d[1], home[2] + d[2]];
            crate::overlay::trail(home, to, camera, size, px, rgb, out);
        }
    }
}

/// A view cube region by name: front, back, left, right, top, bottom, iso (front right top).
pub fn region(name: &str) -> Option<[i8; 3]> {
    Some(match name {
        "front" => [0, -1, 0],
        "back" => [0, 1, 0],
        "left" => [-1, 0, 0],
        "right" => [1, 0, 0],
        "top" => [0, 0, 1],
        "bottom" => [0, 0, -1],
        "iso" => crate::nav::HOME_CORNER,
        _ => return None,
    })
}
