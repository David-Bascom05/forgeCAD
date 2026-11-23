//! The Machine Builder's host side (research/cam/machine-builder.md §4): the machine's bodies in
//! the view, one placement each, posed by the jogged joint values and tinted by link (red where
//! the live check finds a hit, amber where it is near). The part's renderer and scene are kept
//! while the builder is open and put back when it closes (gui_versions.rs's way).
use super::{Gui, Loaded};
use crate::mesh::{FaceRange, Lod, PartMesh};
use crate::render::Renderer;
use crate::scene::{Placement, Rows, Scene};
use forge_machine::def::{MachineDef, Prim};
use forge_machine::kin;

#[derive(Default)]
pub struct MachHost {
    /// The part's renderer and scene, while the machine is shown in their place.
    taken: Option<(Renderer, Scene)>,
    /// No view was open: the builder made one, and closing drops it.
    made_view: bool,
    geom: u64,
    pose: u64,
    fit: u64,
    /// Each placement's body name and link name.
    bodies: Vec<(String, String)>,
}

fn oct(n: [f32; 3]) -> [i16; 2] {
    let s = n[0].abs() + n[1].abs() + n[2].abs();
    let (mut x, mut y) = (n[0] / s, n[1] / s);
    if n[2] < 0.0 {
        (x, y) = ((1.0 - y.abs()) * x.signum(), (1.0 - x.abs()) * y.signum());
    }
    [(x * 32767.0).round() as i16, (y * 32767.0).round() as i16]
}

#[derive(Default)]
struct Build {
    pos: Vec<[f32; 3]>,
    nrm: Vec<[f32; 3]>,
    tri: Vec<[u32; 3]>,
    faces: Vec<FaceRange>,
}

impl Build {
    fn quad(&mut self, p: [[f32; 3]; 4], n: [f32; 3]) {
        let b = self.pos.len() as u32;
        self.pos.extend_from_slice(&p);
        self.nrm.extend_from_slice(&[n; 4]);
        self.tri.push([b, b + 1, b + 2]);
        self.tri.push([b, b + 2, b + 3]);
    }
    fn face(&mut self, first_tri: usize, first_vertex: usize) {
        let k = self.faces.len() as u32;
        self.faces.push(FaceRange {
            face: k,
            first_tri: first_tri as u32,
            tris: (self.tri.len() - first_tri) as u32,
            first_vertex: first_vertex as u32,
            vertices: (self.pos.len() - first_vertex) as u32,
        });
    }
}

fn f3(a: [f64; 3]) -> [f32; 3] {
    [a[0] as f32, a[1] as f32, a[2] as f32]
}

fn add_prim(b: &mut Build, p: &Prim) {
    let (t0, v0) = (b.tri.len(), b.pos.len());
    match p {
        Prim::Box { min, max } => {
            let (l, h) = (f3(*min), f3(*max));
            let c = |i: usize| {
                [
                    if i & 1 == 0 { l[0] } else { h[0] },
                    if i & 2 == 0 { l[1] } else { h[1] },
                    if i & 4 == 0 { l[2] } else { h[2] },
                ]
            };
            // each face counter-clockwise seen from outside
            b.quad([c(0), c(2), c(3), c(1)], [0.0, 0.0, -1.0]);
            b.quad([c(4), c(5), c(7), c(6)], [0.0, 0.0, 1.0]);
            b.quad([c(0), c(1), c(5), c(4)], [0.0, -1.0, 0.0]);
            b.quad([c(2), c(6), c(7), c(3)], [0.0, 1.0, 0.0]);
            b.quad([c(0), c(4), c(6), c(2)], [-1.0, 0.0, 0.0]);
            b.quad([c(1), c(3), c(7), c(5)], [1.0, 0.0, 0.0]);
        }
        Prim::Cyl { base, axis, r, h } => {
            let a = kin::scale(*axis, 1.0 / kin::norm(*axis));
            let u0 = if a[0].abs() < 0.9 {
                [1.0, 0.0, 0.0]
            } else {
                [0.0, 1.0, 0.0]
            };
            let u = kin::cross(a, u0);
            let u = kin::scale(u, 1.0 / kin::norm(u));
            let w = kin::cross(a, u);
            const N: usize = 48;
            let at = |k: usize, z: f64| {
                let th = 2.0 * std::f64::consts::PI * (k % N) as f64 / N as f64;
                let d = {
                    let (sn, cs) = forge_math::dmath::sin_cos(th);
                    kin::add(kin::scale(u, cs), kin::scale(w, sn))
                };
                (
                    f3(kin::add(kin::add(*base, kin::scale(d, *r)), kin::scale(a, z))),
                    f3(d),
                )
            };
            for k in 0..N {
                let (p0, n0) = at(k, 0.0);
                let (p1, n1) = at(k + 1, 0.0);
                let (q0, _) = at(k, *h);
                let (q1, _) = at(k + 1, *h);
                let bi = b.pos.len() as u32;
                b.pos.extend_from_slice(&[p0, p1, q1, q0]);
                b.nrm.extend_from_slice(&[n0, n1, n1, n0]);
                b.tri.push([bi, bi + 1, bi + 2]);
                b.tri.push([bi, bi + 2, bi + 3]);
            }
            for (z, s) in [(0.0, -1.0), (*h, 1.0)] {
                let n = f3(kin::scale(a, s));
                let c = b.pos.len() as u32;
                b.pos.push(f3(kin::add(*base, kin::scale(a, z))));
                b.nrm.push(n);
                for k in 0..N {
                    b.pos.push(at(k, z).0);
                    b.nrm.push(n);
                }
                for k in 0..N as u32 {
                    let (i, j) = (c + 1 + k, c + 1 + (k + 1) % N as u32);
                    b.tri.push(if s > 0.0 { [c, i, j] } else { [c, j, i] });
                }
            }
        }
    }
    b.face(t0, v0);
}

/// A body's primitives as a mesh, in the base frame at home.
pub fn body_mesh(prims: &[Prim]) -> PartMesh {
    let mut b = Build::default();
    for p in prims {
        add_prim(&mut b, p);
    }
    let mut lod = Lod {
        chord: 0.05,
        ..Lod::default()
    };
    for (p, n) in b.pos.iter().zip(&b.nrm) {
        for v in p {
            lod.vertices.extend_from_slice(&v.to_le_bytes());
        }
        let o = oct(*n);
        lod.vertices.extend_from_slice(&o[0].to_le_bytes());
        lod.vertices.extend_from_slice(&o[1].to_le_bytes());
    }
    for t in &b.tri {
        for v in t {
            lod.indices.extend_from_slice(&v.to_le_bytes());
        }
    }
    lod.faces = b.faces;
    let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
    for p in &b.pos {
        for k in 0..3 {
            lo[k] = lo[k].min(p[k]);
            hi[k] = hi[k].max(p[k]);
        }
    }
    let center = [0, 1, 2].map(|k| (lo[k] + hi[k]) * 0.5);
    let radius = b
        .pos
        .iter()
        .map(|p| (0..3).map(|k| (p[k] - center[k]).powi(2)).sum::<f32>())
        .fold(0.0f32, f32::max)
        .sqrt();
    PartMesh {
        center,
        radius,
        lods: vec![lod],
    }
}

fn rows_of(r: &kin::Rigid) -> Rows {
    let m = r.r;
    [0, 1, 2].map(|i| [m[i][0] as f32, m[i][1] as f32, m[i][2] as f32, r.p[i] as f32])
}

/// A link's colour: the base grey, each joint's link a pastel by its place in the tree.
fn link_tint(def: &MachineDef, link: &str) -> [f32; 4] {
    match def.joint_index(link) {
        None => crate::palette::GREY,
        Some(i) => {
            let c = crate::palette::PASTEL[i % crate::palette::PASTEL.len()];
            [c[0], c[1], c[2], 0.0]
        }
    }
}

const HIT: [f32; 4] = [0.93, 0.25, 0.22, 0.0];
const NEAR: [f32; 4] = [0.98, 0.66, 0.18, 0.0];

impl Gui {
    /// The Manage tab's Machine Builder: opens the workspace on the open part's folder (its
    /// project), or closes it.
    pub(crate) fn machine_toggle(&mut self) {
        if self.shell.machine.take().is_some() {
            return;
        }
        let project = self
            .shell
            .fg
            .as_ref()
            .and_then(|f| std::path::Path::new(&f.path).parent().map(|p| p.to_path_buf()));
        self.shell.machine = Some(crate::shell::machine::MachineUi::new(project));
    }

    /// Each frame: the scene follows the workspace (built when the bodies change, posed and tinted
    /// when the joints move), and the part comes back when it closes.
    pub(crate) fn machine_frame(&mut self) {
        let Some(m) = &self.shell.machine else {
            if self.mach.made_view {
                self.mach.made_view = false;
                self.view = None;
                self.shell.view_tex = None;
            } else if let (Some((r, s)), Some(v)) = (self.mach.taken.take(), &mut self.view) {
                v.renderer = r;
                v.scene = s;
                v.renderer.resize(v.size);
                v.dirty = true;
            }
            self.mach.taken = None;
            self.mach.geom = 0;
            self.mach.pose = 0;
            self.mach.fit = 0;
            return;
        };
        let Some(def) = m.def.clone() else { return };
        if m.geom != self.mach.geom {
            self.mach.geom = m.geom;
            self.mach.pose = 0;
            let mut meshes: Vec<PartMesh> = def.bodies.iter().map(|b| body_mesh(&b.prims)).collect();
            self.mach.bodies = def
                .bodies
                .iter()
                .map(|b| (b.name.clone(), b.link.clone()))
                .collect();
            if meshes.is_empty() {
                // a scene needs a part: an empty mesh at the origin
                meshes.push(body_mesh(&[]));
            }
            let info = crate::render::part_info(&meshes);
            let placements: Vec<Placement> = (0..meshes.len())
                .map(|k| Placement {
                    tint: def
                        .bodies
                        .get(k)
                        .map_or(crate::palette::GREY, |b| link_tint(&def, &b.link)),
                    ..Placement::at(k as u32, [0.0; 3])
                })
                .collect();
            let mut scene = Scene::new(&info, placements);
            scene.set_explode(0.0);
            let renderer = Renderer::new(
                self.device.clone(),
                self.queue.clone(),
                self.adapter.clone(),
                crate::render::OFFSCREEN,
                (16, 16),
                &meshes,
                &scene,
            );
            match &mut self.view {
                Some(v) => {
                    let mut renderer = renderer;
                    renderer.resize(v.size);
                    renderer.style = v.renderer.style;
                    renderer.look = v.renderer.look.clone();
                    renderer.back_colour = v.renderer.back_colour;
                    renderer.set_tints(&scene);
                    let fresh = self.mach.taken.is_none() && !self.mach.made_view;
                    let old_r = std::mem::replace(&mut v.renderer, renderer);
                    let old_s = std::mem::replace(&mut v.scene, scene);
                    if fresh {
                        self.mach.taken = Some((old_r, old_s));
                    }
                    if fresh || m.fit != self.mach.fit {
                        self.mach.fit = m.fit;
                        let (c, r) = v.scene.bounds();
                        v.nav.camera = super::view_camera(c, r, self.shell.view.parallel, v.size);
                    }
                    v.dirty = true;
                }
                None => {
                    self.mach.made_view = true;
                    let status = std::mem::take(&mut self.shell.status);
                    self.show_loaded(Loaded {
                        file: None,
                        name: def.name.clone(),
                        empty: false,
                        scene,
                        renderer,
                    });
                    self.shell.status = status;
                }
            }
        }
        let Some(m) = &self.shell.machine else { return };
        if m.pose != self.mach.pose
            && let Some(v) = &mut self.view
        {
            self.mach.pose = m.pose;
            let valid = m.problem.is_none();
            let pose = if valid { kin::pose(&def, &m.q) } else { Vec::new() };
            for (i, (name, link)) in self.mach.bodies.iter().enumerate() {
                if i >= v.scene.placements.len() {
                    break;
                }
                let rows = if valid {
                    rows_of(kin::link(&def, &pose, link))
                } else {
                    rows_of(&kin::Rigid::ID)
                };
                v.scene.set_rows(i, &rows);
                let hit = m.live.iter().find(|(a, b, _)| a == name || b == name);
                v.scene.placements[i].tint = match hit {
                    Some((_, _, true)) => HIT,
                    Some((_, _, false)) => NEAR,
                    None => link_tint(&def, link),
                };
            }
            v.renderer.set_tints(&v.scene);
            v.dirty = true;
        }
    }
}
