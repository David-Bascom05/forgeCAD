//! A thermoform's forming and simulated part kept on disk (`store`), keyed by everything they are
//! made from: the shape's whole body, the feature's text and id, the material's law and the
//! process. Opening the part again, or an edit elsewhere in it, reads them back bit for bit.

use crate::store::{self, In, Out};
use forge_math::Point3;
use forge_thermo::{MeshMass, Simulated, Step, Stretch};

/// The forming's key: `memo` is the feature's in-run memo key (its text, material, process).
pub fn key(kind: &str, memo: &str, id: &str, shape: &forge_topo::Body) -> u128 {
    store::key(kind, |h| {
        h.str(memo);
        h.str(id);
        h.str(&format!("{shape:?}"));
    })
}

pub fn get_stretch(key: u128) -> Option<Stretch> {
    let b = store::get("thermo-stretch", key)?;
    read_stretch(&mut In(&b))
}

pub fn put_stretch(key: u128, s: &Stretch) {
    let mut o = Out::default();
    write_stretch(&mut o, s);
    store::put("thermo-stretch", key, &o.0);
}

pub fn get_sim(key: u128) -> Option<Simulated> {
    let b = store::get("thermo-sim", key)?;
    read_sim(&mut In(&b))
}

pub fn put_sim(key: u128, s: &Simulated) {
    let mut o = Out::default();
    write_sim(&mut o, s);
    store::put("thermo-sim", key, &o.0);
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

const STAGES: [&str; 4] = ["sheet", "drape", "vacuum", "creep"];

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
        steps.push(Step {
            nodes: flat.chunks(3).map(|c| [c[0], c[1], c[2]]).collect(),
            ratio,
            stage: STAGES.into_iter().find(|s| *s == name)?,
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
