//! A workholding model compiled for the checker (forge_cam::fixture_model): a maker's STEP of a
//! device, its solids sorted into the device's bodies (`groups`), turned into the device's own
//! frame, each face the checker's:
//! - exact (forge_cam::lower: planes, cylinders, cones and tori with their trims);
//! - else its own surface over a parameter box that encloses it (forge_cam::enclose: proven
//!   from the face's box; a superset, conservative);
//! - else (procedural, NURBS) forge-mesh's triangles, each certified within the chord, as planar
//!   faces whose mu is the chord: conservative too. A face forge-mesh could not settle is refused
//!   (no box stands in for it).
//!
//! The device's frame: its turn of the model's axes (`rot`, a signed permutation, exact) and
//! its origin (in the model's own coordinates, mm, after the file's units).
use forge_cam::fixture_model::{self, Rot};
use forge_cam_geom::check::Face;

/// The compiler's version: a cache of compiled faces is named by it with its input's content,
/// so raise it whenever the same input would compile to other faces.
pub const VERSION: u32 = 1;

/// Which of the file's solids (in the order the reader gives them) make each of the device's
/// bodies.
#[derive(Clone, Debug)]
pub struct Spec {
    pub rot: Rot,
    pub origin: [f64; 3],
    pub groups: Vec<(String, Vec<usize>)>,
    /// forge-mesh's chord for faces left to the mesh (mm).
    pub chord: f64,
}

impl Spec {
    /// A model's spec (forge_workholding::model, data/models.toml).
    pub fn of(m: &forge_workholding::model::Model) -> Spec {
        Spec {
            rot: m.rot,
            origin: m.origin,
            groups: m
                .groups
                .iter()
                .map(|g| (g.name.clone(), g.solids.clone()))
                .collect(),
            chord: m.chord,
        }
    }
}

/// What came out, and how.
#[derive(Clone, Debug, Default)]
pub struct Compiled {
    pub text: String,
    /// Each group's faces' box in the device's frame: (name, min, max).
    pub boxes: Vec<(String, [f64; 3], [f64; 3])>,
    /// Each group's own box in the device's frame (its mesh grown by the chord: the body).
    pub tight: Vec<(String, [f64; 3], [f64; 3])>,
    /// Each group's triangles for drawing (the device's frame), as a binary STL.
    pub stl: Vec<(String, Vec<u8>)>,
    pub exact: usize,
    pub enclosed: usize,
    pub meshed: usize,
    pub triangles: usize,
    pub solids: usize,
}

fn stl(tris: &[[[f64; 3]; 3]], header: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(84 + 50 * tris.len());
    let mut h = [0u8; 80];
    for (d, s) in h.iter_mut().zip(header.bytes()) {
        *d = s;
    }
    out.extend_from_slice(&h);
    out.extend_from_slice(&(tris.len() as u32).to_le_bytes());
    for [a, b, c] in tris {
        let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        let n = [
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ];
        let l = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
        let n = if l > 0.0 { n.map(|x| x / l) } else { [0.0; 3] };
        for p in [n, *a, *b, *c] {
            for x in p {
                out.extend_from_slice(&(x as f32).to_le_bytes());
            }
        }
        out.extend_from_slice(&[0, 0]);
    }
    out
}

/// Compiles the STEP text by `spec`.
pub fn compile(step: &[u8], spec: &Spec) -> Result<Compiled, String> {
    let im = forge_step::import(step, &forge_step::Limits::default()).map_err(|e| format!("{e:?}"))?;
    if !im.refusals.is_empty() {
        return Err(format!(
            "the reader refused {} entities: {:?}",
            im.refusals.len(),
            im.refusals
        ));
    }
    let bodies: Vec<&forge_topo::Body> = im.solids.iter().map(|s| &s.body).collect();
    compile_bodies(&bodies, spec)
}

/// One solid compiled for the checker, in its own coordinates: each face exact
/// (forge_cam::lower), else under a proven enclosure (forge_cam::enclose), else forge-mesh's
/// triangles certified within `chord` (planar faces carrying mu = the chord plus the body's
/// rounding); and the whole mesh (every triangle's corners on the solid within the chord), for
/// drawing and for points on the solid. Shared by workholding and machine bodies (any model:
/// a `.fg` part's B-rep, a maker's or a user's STEP).
pub struct Solid {
    pub faces: Vec<Face>,
    pub mesh: Vec<[[f64; 3]; 3]>,
    pub exact: usize,
    pub enclosed: usize,
    pub meshed: usize,
    pub triangles: usize,
}

pub fn compile_solid(body: &forge_topo::Body, chord: f64) -> Result<Solid, String> {
    if chord <= 0.0 || chord.is_nan() {
        return Err("chord > 0".into());
    }
    let mut out = Solid {
        faces: Vec::new(),
        mesh: Vec::new(),
        exact: 0,
        enclosed: 0,
        meshed: 0,
        triangles: 0,
    };
    let mut left = std::collections::BTreeSet::new();
    for (k, f) in body.face_ids().enumerate() {
        match forge_cam::enclose::enclose_face(body, f, k, &mut out.faces) {
            Ok(forge_cam::enclose::How::Exact) => out.exact += 1,
            Ok(forge_cam::enclose::How::Enclosed) => out.enclosed += 1,
            Err(_) => {
                out.meshed += 1;
                left.insert(k as u32);
            }
        }
    }
    let q = forge_mesh::tessellate::Quality {
        chord,
        ..Default::default()
    };
    let m = forge_mesh::tessellate_tolerant(body, &q).map_err(|e| format!("{e:?}"))?;
    if let Some(u) = m.unsettled.iter().find(|f| left.contains(f)) {
        return Err(format!("face {u} is not settled by the mesh (no certified triangles)"));
    }
    let mu = forge_cam::lower::mu(body_scale(body)) + chord;
    for (t, f) in m.triangles.iter().zip(&m.face_of) {
        let [a, b, cc] = t.map(|i| {
            let p = m.positions[i as usize];
            [p.x, p.y, p.z]
        });
        out.mesh.push([a, b, cc]);
        if left.contains(f) {
            out.faces.push(fixture_model::triangle(a, b, cc, mu, 0));
            out.triangles += 1;
        }
    }
    Ok(out)
}

/// `compile` on solids already made (a `.fg` part built, or a file read): the same faces.
/// A spec with no groups makes one rigid body, `fixture`, of every solid.
pub fn compile_bodies(solids: &[&forge_topo::Body], spec: &Spec) -> Result<Compiled, String> {
    if !fixture_model::is_rot(&spec.rot) {
        return Err("rot: a proper rotation permuting the axes".into());
    }
    if spec.chord <= 0.0 || spec.chord.is_nan() {
        return Err("chord > 0".into());
    }
    if solids.is_empty() {
        return Err("no solid to hold with".into());
    }
    let all;
    let wanted: &[(String, Vec<usize>)] = if spec.groups.is_empty() {
        all = vec![("fixture".to_string(), (0..solids.len()).collect())];
        &all
    } else {
        &spec.groups
    };
    let rot = spec.rot;
    let r = |v: [f64; 3]| {
        [
            rot[0].1 * v[rot[0].0],
            rot[1].1 * v[rot[1].0],
            rot[2].1 * v[rot[2].0],
        ]
    };
    let ro = r(spec.origin);
    let shift = [-ro[0], -ro[1], -ro[2]];
    let mut c = Compiled {
        solids: solids.len(),
        ..Default::default()
    };
    let mut used = vec![false; solids.len()];
    let mut groups: Vec<(String, Vec<Face>, [[f64; 3]; 2])> = Vec::new();
    for (name, sol) in wanted {
        let mut faces = Vec::new();
        let mut draw = Vec::new();
        let (mut tlo, mut thi) = ([f64::INFINITY; 3], [f64::NEG_INFINITY; 3]);
        for &si in sol {
            let body: &forge_topo::Body = solids
                .get(si)
                .ok_or_else(|| format!("{name}: no solid {si} (the file has {})", solids.len()))?;
            if std::mem::replace(&mut used[si], true) {
                return Err(format!("solid {si} is in two groups"));
            }
            let one = compile_solid(body, spec.chord).map_err(|e| format!("solid {si}: {e}"))?;
            c.exact += one.exact;
            c.enclosed += one.enclosed;
            c.meshed += one.meshed;
            c.triangles += one.triangles;
            for t in &one.mesh {
                let t = t.map(|p| {
                    let w = r(p);
                    [w[0] + shift[0], w[1] + shift[1], w[2] + shift[2]]
                });
                // the body's own box: its mesh's corners, every one on the body within the
                // chord, grown by the chord so the box holds the body itself
                for v in t {
                    for k in 0..3 {
                        tlo[k] = tlo[k].min(v[k] - spec.chord);
                        thi[k] = thi[k].max(v[k] + spec.chord);
                    }
                }
                draw.push(t);
            }
            let fs = one.faces;
            faces.extend(fs.iter().map(|f| fixture_model::moved(f, &rot, shift, 0)));
        }
        for (i, f) in faces.iter_mut().enumerate() {
            f.id = i as u32;
        }
        let (mut lo, mut hi) = ([f64::INFINITY; 3], [f64::NEG_INFINITY; 3]);
        for f in &faces {
            for k in 0..3 {
                lo[k] = lo[k].min(f.bbox[k].lo);
                hi[k] = hi[k].max(f.bbox[k].hi);
            }
        }
        c.boxes.push((name.clone(), lo, hi));
        c.tight.push((name.clone(), tlo, thi));
        c.stl
            .push((name.clone(), stl(&draw, &format!("forgecad workholding {name}"))));
        groups.push((name.clone(), faces, [tlo, thi]));
    }
    if let Some(i) = used.iter().position(|u| !u) {
        return Err(format!("solid {i} is in no group"));
    }
    c.text = fixture_model::write(&groups);
    Ok(c)
}

fn body_scale(b: &forge_topo::Body) -> f64 {
    let mut s = 0.0f64;
    for f in b.face_ids() {
        let bb = b.face_box(f);
        for x in [bb.x.lo, bb.y.lo, bb.z.lo, bb.x.hi, bb.y.hi, bb.z.hi] {
            s = s.max(x.abs());
        }
    }
    s
}
