//! The Bend Simulation environment in the view: the press, its tools and the part at the instant
//! shown (forge-brake's scene), drawn by the view's own renderer as parts, so they are lit and
//! shadowed as any part is. The design's renderer and scene are put aside while it is in and put
//! back when it leaves.

use super::Gui;
use crate::mesh::{FaceRange, Lod, PartMesh};
use crate::render::Renderer;
use crate::scene::{Placement, Scene};
use crate::shell::env::EnvId;
use forge_brake::scene::{Body, Look, Mesh};

struct Taken {
    renderer: Renderer,
    scene: Scene,
}

#[derive(Default)]
pub struct BendView {
    taken: Option<Taken>,
    /// What is drawn: (simulation's identity, step, instant) and the bodies' names.
    shown: Option<(usize, usize, u64)>,
    names: Vec<String>,
    /// The view's renderer's geometry counter after our last upload: another value means a
    /// build put its own part in the view since.
    ours: Option<u64>,
    /// The camera framed on the press once per entry.
    framed: bool,
}

/// Unit normal to octahedral i16x2 (as tess.rs packs them).
fn oct(n: [f32; 3]) -> [i16; 2] {
    let s = (n[0].abs() + n[1].abs() + n[2].abs()).max(1e-20);
    let (mut x, mut y) = (n[0] / s, n[1] / s);
    if n[2] < 0.0 {
        (x, y) = ((1.0 - y.abs()) * x.signum(), (1.0 - x.abs()) * y.signum());
    }
    [(x * 32767.0).round() as i16, (y * 32767.0).round() as i16]
}

/// A scene body's triangles as a part: one level, one face, no edges.
pub fn part_of(m: &Mesh) -> PartMesh {
    let mut lod = Lod {
        chord: 0.05,
        ..Lod::default()
    };
    for (p, n) in m.pos.iter().zip(&m.nrm) {
        for v in p {
            lod.vertices.extend_from_slice(&v.to_le_bytes());
        }
        for v in oct(*n) {
            lod.vertices.extend_from_slice(&v.to_le_bytes());
        }
    }
    for t in &m.tri {
        for v in t {
            lod.indices.extend_from_slice(&v.to_le_bytes());
        }
    }
    lod.faces.push(FaceRange {
        face: 0,
        first_tri: 0,
        tris: m.tri.len() as u32,
        first_vertex: 0,
        vertices: m.pos.len() as u32,
    });
    let (lo, hi) = m.bounds().unwrap_or(([0.0; 3], [0.0; 3]));
    let center = [0, 1, 2].map(|k| (lo[k] + hi[k]) * 0.5);
    let radius = (0..3).map(|k| (hi[k] - lo[k]).powi(2)).sum::<f32>().sqrt() * 0.5;
    PartMesh {
        center,
        radius: radius.max(1e-3),
        lods: vec![lod],
    }
}

/// A body's colour (linear-ish rgb) and how metal, until RENDER-STUDIO's appearances land:
/// ground tool steel, painted frame, the sheet by its material's family.
pub fn tint(b: &Body, material: Option<&str>) -> [f32; 4] {
    if b.hit {
        return [0.86, 0.16, 0.12, 0.0];
    }
    match b.look {
        Look::Tool => [0.66, 0.67, 0.69, 1.0],
        // a maker's livery is not in the data: a neutral machine grey
        Look::Frame => [0.46, 0.48, 0.50, 0.15],
        Look::Gauge => [0.16, 0.16, 0.17, 0.6],
        Look::Floor => [0.30, 0.30, 0.29, 0.0],
        Look::Witness => [0.95, 0.10, 0.08, 0.0],
        Look::Sheet => {
            // the part in the panel's part blue, set apart from the steel tools; half metal
            // when its material is a metal
            let m = material.unwrap_or("").to_ascii_lowercase();
            let metal = if m.is_empty()
                || m.contains("steel")
                || m.contains("alumin")
                || m.contains("copper")
                || m.contains("brass")
            {
                0.7
            } else {
                0.0
            };
            // the panel's part blue (shell/bendsim.rs look::PART), linear
            [0.07, 0.33, 0.81, metal * 0.5]
        }
    }
}

impl BendView {
    /// The view holds the simulation's bodies, not the design.
    pub fn showing(&self) -> bool {
        self.taken.is_some()
    }
}

impl Gui {
    /// Each frame: the simulation's instant in the view while the environment is in; the
    /// design's view back once it is not.
    pub(super) fn bendsim_frame(&mut self) {
        let on = self.shell.env == Some(EnvId::BendSim)
            && self.shell.sheet.bend_sim.as_ref().is_some_and(|b| b.sim.is_ok());
        if !on {
            self.bendsim_put_back();
            return;
        }
        let Some(v) = &mut self.view else { return };
        // a build put its own part in the view: that is the design's now
        if let Some(g) = self.bend.ours
            && v.renderer.geometry() != g
        {
            self.bend.taken = None;
            self.bend.shown = None;
            self.bend.names.clear();
            self.bend.ours = None;
        }
        let b = self.shell.sheet.bend_sim.as_ref().unwrap();
        let sim = b.sim.as_ref().unwrap();
        let n = sim.plan.steps.len();
        if n == 0 {
            return;
        }
        let step = b.step.min(n - 1);
        let key = (sim as *const _ as usize, step, b.t.to_bits());
        if self.bend.shown == Some(key) {
            return;
        }
        let mut sc = forge_brake::scene::Scene::new(&sim.model, &sim.tooling, &sim.plan);
        if let Some(h) = b.table_height {
            sc.table_height = h;
        }
        let bodies = sc.bodies(step, b.t);
        let parts: Vec<PartMesh> = bodies.iter().map(|x| part_of(&x.mesh)).collect();
        let material = b.material.as_deref();
        let placements: Vec<Placement> = bodies
            .iter()
            .enumerate()
            .map(|(k, x)| Placement {
                tint: tint(x, material),
                ..Placement::at(k as u32, [0.0; 3])
            })
            .collect();
        let info: Vec<([f32; 3], f32, Vec<f32>)> = parts
            .iter()
            .map(|m| (m.center, m.radius, m.lods.iter().map(|x| x.chord).collect()))
            .collect();
        let names: Vec<String> = bodies.iter().map(|x| x.name.clone()).collect();
        let mut scene = Scene::new(&info, placements);
        scene.set_explode(0.0);
        if names == self.bend.names && self.bend.taken.is_some() {
            // the same bodies: new triangles, the same renderer
            v.renderer.set_parts(&parts);
            v.renderer.set_tints(&scene);
            v.scene = scene;
        } else {
            let mut renderer = Renderer::new(
                self.device.clone(),
                self.queue.clone(),
                self.adapter.clone(),
                crate::render::OFFSCREEN,
                (16, 16),
                &parts,
                &scene,
            );
            renderer.resize(v.size);
            renderer.style = v.renderer.style;
            renderer.look = v.renderer.look.clone();
            renderer.back_colour = v.renderer.back_colour;
            renderer.set_tints(&scene);
            let old_r = std::mem::replace(&mut v.renderer, renderer);
            let old_s = std::mem::replace(&mut v.scene, scene);
            if self.bend.taken.is_none() {
                self.bend.taken = Some(Taken {
                    renderer: old_r,
                    scene: old_s,
                });
            }
            self.bend.names = names;
        }
        if !self.bend.framed {
            // the part on the working station, seen along the bed from its near end and a
            // little in front (the operator's side) and above: the punch, the V and the part's
            // section in view, the ram and frame behind
            let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
            for x in bodies.iter().filter(|x| x.look == Look::Sheet) {
                if let Some((a, c)) = x.mesh.bounds() {
                    for k in 0..3 {
                        lo[k] = lo[k].min(a[k]);
                        hi[k] = hi[k].max(c[k]);
                    }
                }
            }
            if lo[0] <= hi[0] {
                let center = [0.5 * (lo[0] + hi[0]), 0.5 * (lo[1] + hi[1]), 40.0];
                let reach = (hi[0] - lo[0]).max(hi[1] - lo[1]).max(hi[2] - lo[2]);
                // a perspective eye of its own (the design's may be parallel)
                let mut cam = crate::camera::Camera::fit(center, reach);
                cam.yaw = 262f32.to_radians();
                cam.pitch = 12f32.to_radians();
                cam.dist = (2.6 * reach).max(450.0);
                v.nav.jump_to(cam);
            }
            self.bend.framed = true;
        }
        self.bend.ours = Some(v.renderer.geometry());
        self.bend.shown = Some(key);
        v.dirty = true;
    }

    fn bendsim_put_back(&mut self) {
        self.bend.framed = false;
        self.bend.shown = None;
        self.bend.names.clear();
        self.bend.ours = None;
        if let (Some(t), Some(v)) = (self.bend.taken.take(), &mut self.view) {
            v.renderer = t.renderer;
            v.scene = t.scene;
            v.renderer.resize(v.size);
            v.nav.fit_all(&v.scene);
            v.dirty = true;
        }
    }
}
