//! A thermoform's forming and simulated part kept on disk (`store`). Each is keyed by the bytes of
//! everything it reads: the forming by the process and spec, the tool's mesh and contact, the
//! trim and the plane; the simulated part by the forming's key and the modelled part as it reads
//! it (its tessellation and the exact normals at its corners). The entry holds those bytes and a
//! read compares them, so a hit is the same inputs, never two that only hash alike. Opening the
//! part again, or an edit elsewhere in it, reads them back bit for bit.

use crate::store::{self, In, Out};
use forge_math::Point3;
use forge_thermo::{Formed, MeshMass, PartMesh, Simulated, Spec, Step, Stretch};

/// The bytes of all the forming reads: `memo` is the feature's memo text (its spec, material and
/// process, printed exactly), `spec` the spec it forms with.
pub fn stretch_key(memo: &str, spec: &Spec, f: &Formed) -> Vec<u8> {
    let mut o = Out::default();
    o.str(memo);
    o.str(&format!("{spec:?}"));
    o.str(&format!("{:?}", f.frame));
    o.str(&format!("{:?}", f.report.trim_box));
    o.str(&format!("{:?}", f.trim));
    points(&mut o, &f.mesh.positions);
    tris(&mut o, &f.mesh.triangles);
    o.u64(f.contact.len() as u64);
    f.contact.iter().for_each(|&k| o.u64(k as u64));
    o.0
}

/// The bytes of all the simulated part reads besides the forming (`stretch`, its key).
pub fn sim_key(stretch: &[u8], walls: &str, t: f64, part: &PartMesh) -> Vec<u8> {
    let mut o = Out::default();
    o.bytes(stretch);
    o.str(walls);
    o.f64(t);
    points(&mut o, &part.pos);
    tris(&mut o, &part.tris);
    o.u64(part.faces.len() as u64);
    part.faces.iter().for_each(|&f| o.u64(f as u64));
    o.u64(part.normals.len() as u64);
    for n in part.normals.iter().flatten() {
        o.f64(n.x);
        o.f64(n.y);
        o.f64(n.z);
    }
    o.0
}

/// The entry's bytes after its key's, when the key it holds is `key`.
fn get(kind: &str, key: &[u8]) -> Option<Vec<u8>> {
    let b = store::get(kind, store::key(kind, |h| h.bytes(key)))?;
    opened(&b, key).map(<[u8]>::to_vec)
}

/// An entry's payload, when the key it was kept under is `key` byte for byte.
fn opened<'a>(entry: &'a [u8], key: &[u8]) -> Option<&'a [u8]> {
    let mut i = In(entry);
    (i.bytes()? == key).then_some(i.0)
}

fn put(kind: &str, key: &[u8], o: Out) {
    let mut e = Out::default();
    e.bytes(key);
    e.0.extend_from_slice(&o.0);
    store::put(kind, store::key(kind, |h| h.bytes(key)), &e.0);
}

/// A quick key: the whole document, the feature and the kernel, with the files the run had read
/// by then. Under it is kept the full key of the simulated part: the run being a function of those,
/// so is that key. Reopening an unchanged document finds its forming without working the full key
/// out (the part's tessellation and normals); the full key's own entry is still compared whole.
pub fn quick_key(src: &str, memo: &str) -> Option<(Vec<u8>, crate::deps::Deps)> {
    let read = crate::deps::so_far()?;
    let mut o = Out::default();
    o.str(src);
    o.str(memo);
    Some((o.0, read))
}

/// The full key and the sheet kept under a quick key, while every file it was made from holds what
/// it held.
pub fn get_quick(key: &[u8]) -> Option<(Vec<u8>, Simulated)> {
    let b = get("thermo-quick", key)?;
    let mut i = In(&b);
    let d = crate::deps::Deps::read(&mut i)?;
    let full = i.bytes()?.to_vec();
    let sheet = read_sim(&mut i)?;
    d.current().then_some((full, sheet))
}

/// Keeps the full key of the simulated part, and the model's sheet on the tool, under a quick key.
pub fn put_quick(key: &[u8], read: &crate::deps::Deps, full: &[u8], sheet: &Simulated) {
    let mut o = Out::default();
    read.write(&mut o);
    o.bytes(full);
    write_sim(&mut o, sheet);
    put("thermo-quick", key, o);
}

pub fn get_stretch(key: &[u8]) -> Option<Stretch> {
    read_stretch(&mut In(&get("thermo-stretch", key)?))
}

pub fn put_stretch(key: &[u8], s: &Stretch) {
    let mut o = Out::default();
    write_stretch(&mut o, s);
    put("thermo-stretch", key, o);
}

pub fn get_sim(key: &[u8]) -> Option<Simulated> {
    read_sim(&mut In(&get("thermo-sim", key)?))
}

pub fn put_sim(key: &[u8], s: &Simulated) {
    let mut o = Out::default();
    write_sim(&mut o, s);
    put("thermo-sim", key, o);
}

fn point(o: &mut Out, p: Point3) {
    o.f64(p.x);
    o.f64(p.y);
    o.f64(p.z);
}

fn points(o: &mut Out, v: &[Point3]) {
    o.u64(v.len() as u64);
    v.iter().for_each(|p| point(o, *p));
}

fn nums(o: &mut Out, v: &[f64]) {
    o.u64(v.len() as u64);
    v.iter().for_each(|x| o.f64(*x));
}

fn tris(o: &mut Out, v: &[[u32; 3]]) {
    o.u64(v.len() as u64);
    for t in v {
        t.iter().for_each(|&i| o.u64(i as u64));
    }
}

fn get_point(i: &mut In) -> Option<Point3> {
    Some(Point3::new(i.f64()?, i.f64()?, i.f64()?))
}

fn get_points(i: &mut In) -> Option<Vec<Point3>> {
    let n = i.count(24)?;
    (0..n).map(|_| get_point(i)).collect()
}

fn get_nums(i: &mut In) -> Option<Vec<f64>> {
    let n = i.count(8)?;
    (0..n).map(|_| i.f64()).collect()
}

fn get_u32(i: &mut In) -> Option<u32> {
    u32::try_from(i.u64()?).ok()
}

fn get_tris(i: &mut In) -> Option<Vec<[u32; 3]>> {
    let n = i.count(24)?;
    (0..n)
        .map(|_| Some([get_u32(i)?, get_u32(i)?, get_u32(i)?]))
        .collect()
}

fn f32s(o: &mut Out, v: impl ExactSizeIterator<Item = f32>) {
    o.u64(v.len() as u64);
    v.for_each(|x| o.u64(x.to_bits() as u64));
}

fn get_f32s(i: &mut In) -> Option<Vec<f32>> {
    let n = i.count(8)?;
    (0..n)
        .map(|_| Some(f32::from_bits(u32::try_from(i.u64()?).ok()?)))
        .collect()
}

const STAGES: [&str; 6] = ["sheet", "drape", "sag", "plug", "vacuum", "creep"];

fn write_stretch(o: &mut Out, s: &Stretch) {
    points(o, &s.nodes);
    o.u64(s.steps.len() as u64);
    for st in &s.steps {
        f32s(
            o,
            st.nodes.iter().flatten().copied().collect::<Vec<_>>().into_iter(),
        );
        f32s(o, st.ratio.iter().copied());
        o.str(st.stage);
        o.f64(st.pressure);
    }
    o.u64(s.grid.0 as u64);
    o.u64(s.grid.1 as u64);
    tris(o, &s.tris);
    nums(o, &s.ratio);
    nums(o, &s.node_ratio);
    o.u64(s.touched.len() as u64);
    s.touched.iter().for_each(|b| o.bool(*b));
    for (v, p) in [s.min, s.max] {
        o.f64(v);
        point(o, p);
    }
    o.f64(s.mean);
    points(o, &s.unformed);
    o.u64(s.iterations as u64);
}

fn read_stretch(i: &mut In) -> Option<Stretch> {
    let nodes = get_points(i)?;
    let n = i.count(24)?;
    let mut steps = Vec::with_capacity(n);
    for _ in 0..n {
        let flat = get_f32s(i)?;
        if flat.len() % 3 != 0 {
            return None;
        }
        let ratio = get_f32s(i)?;
        let name = i.str()?;
        let pressure = i.f64()?;
        steps.push(Step {
            nodes: flat.chunks(3).map(|c| [c[0], c[1], c[2]]).collect(),
            ratio,
            stage: STAGES.into_iter().find(|s| *s == name)?,
            pressure,
        });
    }
    let grid = (usize::try_from(i.u64()?).ok()?, usize::try_from(i.u64()?).ok()?);
    let tris = get_tris(i)?;
    let ratio = get_nums(i)?;
    let node_ratio = get_nums(i)?;
    let n = i.count(1)?;
    let touched = (0..n).map(|_| i.bool()).collect::<Option<Vec<_>>>()?;
    let min = (i.f64()?, get_point(i)?);
    let max = (i.f64()?, get_point(i)?);
    let mean = i.f64()?;
    let unformed = get_points(i)?;
    let iterations = usize::try_from(i.u64()?).ok()?;
    i.done().then_some(Stretch {
        nodes,
        steps,
        grid,
        tris,
        ratio,
        node_ratio,
        touched,
        min,
        max,
        mean,
        unformed,
        iterations,
    })
}

fn write_sim(o: &mut Out, s: &Simulated) {
    points(o, &s.mesh.positions);
    tris(o, &s.mesh.triangles);
    o.u64(s.mesh.face_of.len() as u64);
    s.mesh.face_of.iter().for_each(|f| o.u64(*f as u64));
    o.u64(s.mesh.edges.len() as u64);
    for e in &s.mesh.edges {
        o.u64(e.len() as u64);
        e.iter().for_each(|v| o.u64(*v as u64));
    }
    nums(o, &s.wall);
    points(o, &s.foot);
    nums(o, &s.raw);
    points(o, &s.sheet);
    points(o, &s.modelled);
    o.f64(s.area);
    o.f64(s.sheet_volume);
    o.f64(s.mass.volume);
    point(o, s.mass.centroid);
    s.mass.inertia.iter().flatten().for_each(|x| o.f64(*x));
}

fn read_sim(i: &mut In) -> Option<Simulated> {
    let positions = get_points(i)?;
    let triangles = get_tris(i)?;
    let n = i.count(8)?;
    let face_of = (0..n).map(|_| get_u32(i)).collect::<Option<Vec<_>>>()?;
    let n = i.count(8)?;
    let mut edges = Vec::with_capacity(n);
    for _ in 0..n {
        let m = i.count(8)?;
        edges.push((0..m).map(|_| get_u32(i)).collect::<Option<Vec<_>>>()?);
    }
    let wall = get_nums(i)?;
    let foot = get_points(i)?;
    let raw = get_nums(i)?;
    let sheet = get_points(i)?;
    let modelled = get_points(i)?;
    let (area, sheet_volume) = (i.f64()?, i.f64()?);
    let volume = i.f64()?;
    let centroid = get_point(i)?;
    let mut inertia = [[0.0; 3]; 3];
    for r in &mut inertia {
        for x in r {
            *x = i.f64()?;
        }
    }
    i.done().then_some(Simulated {
        mesh: forge_mesh::TriMesh {
            positions,
            triangles,
            face_of,
            edges,
        },
        wall,
        foot,
        raw,
        sheet,
        modelled,
        area,
        sheet_volume,
        mass: MeshMass {
            volume,
            centroid,
            inertia,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_entry_is_read_only_under_its_own_key() {
        let mut e = Out::default();
        e.bytes(b"the key");
        e.0.extend_from_slice(b"payload");
        assert_eq!(opened(&e.0, b"the key"), Some(&b"payload"[..]));
        // another key that hashed to the same file is a miss, never the other's forming
        assert_eq!(opened(&e.0, b"the kez"), None);
        assert_eq!(opened(&e.0, b"the key and more"), None);
        assert_eq!(opened(&e.0[..3], b"the key"), None);
    }
}
