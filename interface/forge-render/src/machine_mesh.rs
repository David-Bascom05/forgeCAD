//! A machine's bodies as triangles: the one place the app turns a machine definition's bodies
//! into geometry (the Machine Builder's view and the simulation's machine view, CAM-SIM).
//!
//! A body is its real `.fg` part where it has one and that part builds (a built-in machine's
//! parts are embedded; a user's or project's are beside its file), else its envelope
//! primitives (boxes and cylinders). Parts are built and tessellated once per (machine, body,
//! part, folder) and kept.
use crate::origin::WorldVert;
use forge_machine::def::{Body, MachineDef, Prim};
use forge_machine::kin;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

/// Triangles in the base frame at home: positions, a normal per vertex, and edge polylines.
#[derive(Clone, Debug, Default)]
pub struct Tris {
    pub pos: Vec<[f64; 3]>,
    pub nrm: Vec<[f64; 3]>,
    pub tri: Vec<[u32; 3]>,
    pub edges: Vec<Vec<[f64; 3]>>,
    /// One per primitive or part patch: (first triangle, count, first vertex, count).
    pub patches: Vec<(u32, u32, u32, u32)>,
}

impl Tris {
    fn quad(&mut self, p: [[f64; 3]; 4], n: [f64; 3]) {
        let b = self.pos.len() as u32;
        self.pos.extend_from_slice(&p);
        self.nrm.extend_from_slice(&[n; 4]);
        self.tri.push([b, b + 1, b + 2]);
        self.tri.push([b, b + 2, b + 3]);
    }
    fn patch(&mut self, t0: usize, v0: usize) {
        self.patches.push((
            t0 as u32,
            (self.tri.len() - t0) as u32,
            v0 as u32,
            (self.pos.len() - v0) as u32,
        ));
    }
}

/// Segments a cylinder's round is drawn with.
pub const ROUND: usize = 48;

/// A primitive's triangles (outward, counter-clockwise from outside) and edges.
pub fn prim_tris(p: &Prim, out: &mut Tris) {
    let (t0, v0) = (out.tri.len(), out.pos.len());
    match p {
        Prim::Box { min: l, max: h } => {
            let c = |i: usize| {
                [
                    if i & 1 == 0 { l[0] } else { h[0] },
                    if i & 2 == 0 { l[1] } else { h[1] },
                    if i & 4 == 0 { l[2] } else { h[2] },
                ]
            };
            out.quad([c(0), c(2), c(3), c(1)], [0.0, 0.0, -1.0]);
            out.quad([c(4), c(5), c(7), c(6)], [0.0, 0.0, 1.0]);
            out.quad([c(0), c(1), c(5), c(4)], [0.0, -1.0, 0.0]);
            out.quad([c(2), c(6), c(7), c(3)], [0.0, 1.0, 0.0]);
            out.quad([c(0), c(4), c(6), c(2)], [-1.0, 0.0, 0.0]);
            out.quad([c(1), c(3), c(7), c(5)], [1.0, 0.0, 0.0]);
            for (i, j) in [
                (0, 1),
                (2, 3),
                (4, 5),
                (6, 7),
                (0, 2),
                (1, 3),
                (4, 6),
                (5, 7),
                (0, 4),
                (1, 5),
                (2, 6),
                (3, 7),
            ] {
                out.edges.push(vec![c(i), c(j)]);
            }
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
            let at = |k: usize, z: f64| {
                let th = 2.0 * std::f64::consts::PI * (k % ROUND) as f64 / ROUND as f64;
                let (sn, cs) = forge_math::dmath::sin_cos(th);
                let d = kin::add(kin::scale(u, cs), kin::scale(w, sn));
                (kin::add(kin::add(*base, kin::scale(d, *r)), kin::scale(a, z)), d)
            };
            for k in 0..ROUND {
                let (p0, n0) = at(k, 0.0);
                let (p1, n1) = at(k + 1, 0.0);
                let (q0, _) = at(k, *h);
                let (q1, _) = at(k + 1, *h);
                let bi = out.pos.len() as u32;
                out.pos.extend_from_slice(&[p0, p1, q1, q0]);
                out.nrm.extend_from_slice(&[n0, n1, n1, n0]);
                out.tri.push([bi, bi + 1, bi + 2]);
                out.tri.push([bi, bi + 2, bi + 3]);
            }
            for z in [0.0, *h] {
                out.edges.push((0..=ROUND).map(|k| at(k, z).0).collect());
            }
            for (z, s) in [(0.0, -1.0), (*h, 1.0)] {
                let n = kin::scale(a, s);
                let c = out.pos.len() as u32;
                out.pos.push(kin::add(*base, kin::scale(a, z)));
                out.nrm.push(n);
                for k in 0..ROUND {
                    out.pos.push(at(k, z).0);
                    out.nrm.push(n);
                }
                for k in 0..ROUND as u32 {
                    let (i, j) = (c + 1 + k, c + 1 + (k + 1) % ROUND as u32);
                    out.tri.push(if s > 0.0 { [c, i, j] } else { [c, j, i] });
                }
            }
        }
    }
    out.patch(t0, v0);
}

/// A turned outline revolved about +z: (z, radius) points in order (an end on the axis closes
/// it), `ROUND` segments around; edges at every ring.
pub fn revolve(outline: &[[f64; 2]]) -> Tris {
    let mut t = Tris::default();
    let ring = |z: f64, r: f64, k: usize| {
        let th = 2.0 * std::f64::consts::PI * (k % ROUND) as f64 / ROUND as f64;
        let (sn, cs) = forge_math::dmath::sin_cos(th);
        ([r * cs, r * sn, z], [cs, sn])
    };
    for w in outline.windows(2) {
        let ([z0, r0], [z1, r1]) = (w[0], w[1]);
        let (dz, dr) = (z1 - z0, r1 - r0);
        let len = (dz * dz + dr * dr).sqrt();
        if len == 0.0 {
            continue;
        }
        // the outward normal of the generator in (radial, z): (dz, -dr) / len, facing out
        let (nr, nz) = (dz / len, -dr / len);
        for k in 0..ROUND {
            let (p0, c0) = ring(z0, r0, k);
            let (p1, c1) = ring(z0, r0, k + 1);
            let (q1, _) = ring(z1, r1, k + 1);
            let (q0, _) = ring(z1, r1, k);
            let n0 = [c0[0] * nr, c0[1] * nr, nz];
            let n1 = [c1[0] * nr, c1[1] * nr, nz];
            let b = t.pos.len() as u32;
            t.pos.extend_from_slice(&[p0, p1, q1, q0]);
            t.nrm.extend_from_slice(&[n0, n1, n1, n0]);
            t.tri.push([b, b + 1, b + 2]);
            t.tri.push([b, b + 2, b + 3]);
        }
        if r1 > 0.0 {
            t.edges.push((0..=ROUND).map(|k| ring(z1, r1, k).0).collect());
        }
    }
    let (nt, nv) = (t.tri.len(), t.pos.len());
    t.patches.push((0, nt as u32, 0, nv as u32));
    t
}

/// A body's primitives.
pub fn prims_tris(prims: &[Prim]) -> Tris {
    let mut t = Tris::default();
    for p in prims {
        prim_tris(p, &mut t);
    }
    t
}

/// A `.fg` part's triangles (its text, the folder its files are found in): built with forge-lang
/// and tessellated at 0.1 mm, each triangle flat-shaded.
pub fn part_tris(src: &str, dir: Option<&Path>) -> Result<Tris, String> {
    static BUILT: OnceLock<Built> = OnceLock::new();
    let key = (src.to_string(), dir.map(Path::to_path_buf));
    let c = BUILT.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(r) = c.lock().expect("no panics while held").get(&key) {
        return r.clone();
    }
    let r = build_part(src, dir);
    c.lock().expect("no panics while held").insert(key, r.clone());
    r
}

fn build_part(src: &str, dir: Option<&Path>) -> Result<Tris, String> {
    let opts = forge_lang::Options {
        dir: dir.map(Path::to_path_buf),
        ..forge_lang::Options::default()
    };
    let (run, _) = forge_lang::run(src, opts, forge_lang::build::Cache::default());
    if !run.ok() {
        return Err(format!("does not build: {:?}", run.errors.first()));
    }
    let body = run.body.ok_or("no solid")?;
    let q = forge_mesh::Quality {
        chord: 0.1,
        ..forge_mesh::Quality::default()
    };
    let m = forge_mesh::tessellate(&body, &q).map_err(|e| format!("{e:?}"))?;
    let mut t = Tris::default();
    for tri in &m.triangles {
        let n = m.triangle_normal(*tri);
        let n = n.normalize().map_or([0.0, 0.0, 1.0], |u| {
            let u = u.get();
            [u.x, u.y, u.z]
        });
        let b = t.pos.len() as u32;
        for v in tri {
            let p = m.positions[*v as usize];
            t.pos.push([p.x, p.y, p.z]);
            t.nrm.push(n);
        }
        t.tri.push([b, b + 1, b + 2]);
    }
    for e in &m.edges {
        if e.len() >= 2 {
            t.edges.push(
                e.iter()
                    .map(|v| {
                        let p = m.positions[*v as usize];
                        [p.x, p.y, p.z]
                    })
                    .collect(),
            );
        }
    }
    let (nt, nv) = (t.tri.len(), t.pos.len());
    t.patches.push((0, nt as u32, 0, nv as u32));
    Ok(t)
}

type Key = (String, String, Option<String>, Option<PathBuf>, String);
/// Parts built, by (text, folder).
type Built = Mutex<HashMap<(String, Option<PathBuf>), Result<Tris, String>>>;

fn cache() -> &'static Mutex<HashMap<Key, Arc<Tris>>> {
    static C: OnceLock<Mutex<HashMap<Key, Arc<Tris>>>> = OnceLock::new();
    C.get_or_init(|| Mutex::new(HashMap::new()))
}

/// A body's triangles: its part where it has one that builds (found in `dir`, or among the
/// built-in machines' embedded parts), else its primitives. Kept per body, part and folder.
pub fn body_tris(def: &MachineDef, b: &Body, dir: Option<&Path>) -> Arc<Tris> {
    let key: Key = (
        def.name.clone(),
        b.name.clone(),
        b.part.clone(),
        dir.map(Path::to_path_buf),
        format!("{:?}", b.prims),
    );
    if let Some(t) = cache().lock().expect("no panics while held").get(&key) {
        return t.clone();
    }
    let from_part = b.part.as_ref().and_then(|p| {
        let (src, d) = match dir {
            Some(d) => (
                forge_dirs::fs::read_to_string(d.join(p)).ok()?,
                d.join(p).parent().map(Path::to_path_buf),
            ),
            None => (forge_machine_builder::make::builtin_part(p)?.to_string(), None),
        };
        part_tris(&src, d.as_deref()).ok()
    });
    let t = Arc::new(from_part.unwrap_or_else(|| prims_tris(&b.prims)));
    cache()
        .lock()
        .expect("no panics while held")
        .insert(key, t.clone());
    t
}

/// Light baked into a colour: a key from over the viewer's right shoulder and a floor of ambient.
fn lit(c: [f32; 3], n: [f64; 3]) -> [f32; 4] {
    const L: [f64; 3] = [0.3714, 0.4642, 0.8040];
    let d = (n[0] * L[0] + n[1] * L[1] + n[2] * L[2]).max(0.0) as f32;
    let k = 0.38 + 0.62 * d;
    [c[0] * k, c[1] * k, c[2] * k, 1.0]
}

/// Every body as world triangles with light baked in (for `Overlay`): placed by `pose` (each
/// joint's link, kin::pose), then by `to_view` (base frame to the frame drawn in, e.g. the
/// part's), coloured by `colour` per body. Built-in machines' parts are found among the embedded
/// ones; `bodies_in` names the folder of a user's or project's machine.
pub fn bodies(
    def: &MachineDef,
    pose: &[kin::Rigid],
    to_view: &kin::Rigid,
    colour: &dyn Fn(&Body) -> [f32; 3],
    out: &mut Vec<WorldVert>,
) {
    bodies_in(def, None, pose, to_view, colour, out);
}

/// `bodies` for a machine whose part files are in `dir` (its file's folder).
pub fn bodies_in(
    def: &MachineDef,
    dir: Option<&Path>,
    pose: &[kin::Rigid],
    to_view: &kin::Rigid,
    colour: &dyn Fn(&Body) -> [f32; 3],
    out: &mut Vec<WorldVert>,
) {
    for b in &def.bodies {
        let t = body_tris(def, b, dir);
        let place = if pose.len() == def.joints.len() {
            to_view.then(kin::link(def, pose, &b.link))
        } else {
            *to_view
        };
        let c = colour(b);
        for tri in &t.tri {
            for &v in tri {
                let p = place.apply(t.pos[v as usize]);
                let n = place.dir(t.nrm[v as usize]);
                out.push(WorldVert {
                    at: [p[0] as f32, p[1] as f32, p[2] as f32],
                    colour: lit(c, n),
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_box_is_twelve_outward_triangles_and_a_cylinder_closed() {
        let t = prims_tris(&[Prim::Box {
            min: [0.0; 3],
            max: [1.0, 2.0, 3.0],
        }]);
        assert_eq!(t.tri.len(), 12);
        assert_eq!(t.edges.len(), 12);
        // outward: each triangle's winding agrees with its normal
        for tri in &t.tri {
            let [a, b, c] = tri.map(|i| t.pos[i as usize]);
            let n = kin::cross(kin::sub(b, a), kin::sub(c, a));
            assert!(kin::dot(n, t.nrm[tri[0] as usize]) > 0.0);
        }
        let c = prims_tris(&[Prim::Cyl {
            base: [0.0; 3],
            axis: [0.0, 0.0, 1.0],
            r: 1.0,
            h: 2.0,
        }]);
        assert_eq!(c.tri.len(), 4 * ROUND);
        for tri in &c.tri {
            let [a, b, cc] = tri.map(|i| c.pos[i as usize]);
            let n = kin::cross(kin::sub(b, a), kin::sub(cc, a));
            assert!(kin::dot(n, c.nrm[tri[0] as usize]) > -1e-12);
        }
    }

    #[test]
    fn a_revolved_outline_faces_out() {
        let t = revolve(&[
            [0.0, 0.0],
            [0.0, 5.0],
            [20.0, 5.0],
            [20.0, 3.0],
            [50.0, 3.0],
            [50.0, 0.0],
        ]);
        assert!(!t.tri.is_empty());
        for tri in &t.tri {
            let [a, b, c] = tri.map(|i| t.pos[i as usize]);
            let n = kin::cross(kin::sub(b, a), kin::sub(c, a));
            if kin::norm(n) > 1e-12 {
                assert!(kin::dot(n, t.nrm[tri[0] as usize]) > 0.0, "{tri:?}");
            }
        }
    }

    #[test]
    fn built_in_bodies_come_from_their_parts_posed_and_lit() {
        let def = forge_machine::library::builtin_named("generic-vmc3").unwrap();
        let mut q: Vec<f64> = def.joints.iter().map(|j| j.home).collect();
        let pose0 = kin::pose(&def, &q);
        let mut out = Vec::new();
        bodies(&def, &pose0, &kin::Rigid::ID, &|_| [0.5, 0.5, 0.5], &mut out);
        assert!(out.len() % 3 == 0 && !out.is_empty());
        // the head is its real part (more triangles than its envelope's box and cylinder)
        let head = def.bodies.iter().find(|b| b.name == "head").unwrap();
        let t = body_tris(&def, head, None);
        assert!(t.tri.len() > 12 + 4 * ROUND, "{} triangles", t.tri.len());
        // Z moves the head and nothing else
        let z = def.joint_index("Z").unwrap();
        q[z] = -100.0;
        let mut moved = Vec::new();
        bodies(
            &def,
            &kin::pose(&def, &q),
            &kin::Rigid::ID,
            &|_| [0.5, 0.5, 0.5],
            &mut moved,
        );
        assert_eq!(moved.len(), out.len());
        let changed = out.iter().zip(&moved).filter(|(a, b)| a.at != b.at).count();
        assert_eq!(changed, t.tri.len() * 3);
    }
}
