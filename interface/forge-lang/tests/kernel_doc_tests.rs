//! Naming as the kernel does it, on Forge parts: a part turned whole keeps every name, and two
//! features cutting one edge name their vertices apart.
mod common;

use forge_model::Part;

/// The part's history as the kernel takes it: its features and their ids.
fn history(src: &str) -> (Part, Vec<u64>) {
    let (r, cache) = common::run(src);
    assert!(r.errors.is_empty(), "{}", forge_lang::report::text(&r, src));
    let (f, ids) = cache.history();
    (Part { features: f.to_vec() }, ids.to_vec())
}

/// Planes for anywhere in a part, before its first feature.
const PLANES: &str = "XZ = plane(at: xy.origin, normal: xz.normal, x: yz.normal)\n";

/// A sketch `NAME` holding the point `p` as `p`.
fn point(name: &str, p: [f64; 3]) -> String {
    format!(
        "sketch {name}(on: offset(XZ, {:?})) {{\n  p = pt({:?}, {:?})\n}}\n",
        -p[1], p[0], p[2]
    )
}

/// A rod along `plane`'s normal, an arm on its foot along `(ax, ay)` in the plane, the arm
/// patterned five round the rod.
fn spider(plane: &str) -> String {
    let (ax, ay) = (0.808086874008685, 0.5890633277117765);
    format!(
        "sketch sp(on: {plane}) {{
  c = circle((0, 0), 21)
  ray = line((0, 0), ({:?}, {:?}), construction: true)
}}
rod = extrude(sp.c, sym(30), new: true)
sketch ak(on: plane(at: rod.start.plane.origin, normal: sp.plane.normal, x: sp.ray.dir)) {{
  o = hull(disc((6, -2), 6), disc((56, -2), 6), disc((56, 2), 6), disc((6, 2), 6))
}}
arm = extrude(ak.o, 6)
arms = pattern(of: arm, layout: circ(about: rod.axis, n: count(5)))
",
        10.0 * ax,
        10.0 * ay
    )
}

/// A circular pattern of arms on a rod, and the same part turned and moved by an inexact
/// rotation: every name the same (ties between branches and cuts are not broken by world
/// coordinates or by which of several coincident edges a cut found).
#[test]
fn turning_the_whole_part_renames_nothing() {
    let names = |src: &str, nary: bool| {
        let (part, ids) = history(src);
        let solids = if nary {
            part.regenerate_nary_solids_ids(1, Some(&ids))
        } else {
            part.regenerate_solids_ids(Some(&ids))
        }
        .expect("builds");
        let mut out: Vec<u64> = vec![];
        for s in &solids {
            let b = &s.body;
            out.extend(b.face_ids().map(|f| b.face(f).name.0));
            out.extend(b.edge_ids().map(|e| b.edge(e).name.0));
            out.extend(b.vertex_ids().map(|v| b.vertex(v).name.0));
        }
        out.sort();
        out
    };
    let flat = format!("{PLANES}{}", spider("xy"));
    // the turned frame's origin and its x and z axes
    let o = [-37.0, -53.0, 73.0];
    let x = [0.23076923076923078, 0.3076923076923077, -0.9230769230769231];
    let z = [-0.03692307692307689, 0.9507692307692308, 0.3076923076923077];
    let along = |d: [f64; 3]| [o[0] + 10.0 * d[0], o[1] + 10.0 * d[1], o[2] + 10.0 * d[2]];
    let turned = format!(
        "{PLANES}{}{}{}{}",
        point("po", o),
        point("pz", along(z)),
        point("px", along(x)),
        spider("plane(at: po.p, normal: axis(po.p, pz.p), x: axis(po.p, px.p))")
    );
    for nary in [false, true] {
        assert_eq!(names(&flat, nary), names(&turned, nary), "n-ary {nary}");
    }
}

/// A volute loft, a flange disc through it, then the core cut: a vertex each of the two booleans
/// makes on one loft edge where no new edge ends were named alike (seq DuplicateName, R32).
#[test]
fn vertices_cut_on_one_edge_by_two_features_are_named_apart() {
    let src = r#"shared ang = [60deg, 110deg, 160deg, 210deg, 260deg, 310deg, 360deg]
shared rc = [87.17, 89.81, 92.44, 95.08, 97.72, 100.4, 103]
shared ro = [16.67, 18.89, 21.11, 23.33, 25.56, 27.78, 30]
shared ri = [11.67, 13.89, 16.11, 18.33, 20.56, 22.78, 25]
shared n = count(7)
shared dx = 103
shared dy = [55, 110]
shared dro = [33.5, 37]
shared dri = [28.5, 32]
shared dyc = 125
shared drc = 33
shared tick = 10
shared fl = 14
sketch datum(on: xy) {
  for i in 0..n {
    c = pt(rc[i] * cos(ang[i]), rc[i] * sin(ang[i]))
    radial = line(c, ((rc[i] + tick) * cos(ang[i]), (rc[i] + tick) * sin(ang[i])), construction: true)
  }
  for k in 0..2 {
    diff = line((dx, dy[k]), (dx + tick, dy[k]), construction: true)
  }
  core_end = line((dx, dyc), (dx + tick, dyc), construction: true)
  flange_face = line((dx, dy[1] - fl), (dx + tick, dy[1] - fl), construction: true)
}
for i in 0..n {
  sketch vo(on: rotate(datum.plane, about: datum.radial[i], by: -90deg)) {
    c = circle((0, 0), ro[i])
  }
  sketch vi(on: rotate(datum.plane, about: datum.radial[i], by: -90deg)) {
    c = circle((0, 0), ri[i])
  }
}
for k in 0..2 {
  sketch dob(on: rotate(datum.plane, about: datum.diff[k], by: -90deg)) {
    c = circle((0, 0), dro[k])
  }
  sketch dib(on: rotate(datum.plane, about: datum.diff[k], by: -90deg)) {
    c = circle((0, 0), dri[k])
  }
}
sketch dic(on: rotate(datum.plane, about: datum.core_end, by: -90deg)) {
  c = circle((0, 0), drc)
}
volute = loft([vo[i].c for i in 0..n] + [dob[k].c for k in 0..2], smooth: true)
sketch cfs(on: offset(datum.plane, 10)) {
  c = circle((0, 0), 104)
}
cflange = extrude(cfs.c, 12)
core = loft([vi[i].c for i in 0..n] + [dib[k].c for k in 0..2] + [dic.c], smooth: true, cut: true)
sketch bolt(on: rotate(datum.plane, about: datum.flange_face, by: 90deg)) {
  p = pt(45, 0)
}
dbolt = hole(at: bolt.p, d: 14, depth: thru)
"#;
    let (part, ids) = history(src);
    let b = part.regenerate_ids(Some(&ids)).expect("builds");
    let problems = forge_topo::check(&b);
    assert!(problems.is_empty(), "{problems:?}");
}
