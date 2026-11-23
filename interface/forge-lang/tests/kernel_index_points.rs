//! forge-bool's `Index` (point membership) on curved bodies, against a winding number of a fine
//! tessellation and, for the repros, against OpenCascade's classifier as well.
//!
//! The repros are the fillet census's 14 pipe parts (research/fillets/census.md) where `Index`
//! answered wrong, each cut after its fillet (tests/index/*.fg: a bar, an oblique boss, the
//! concave edge between them rounded), with the census's points. Two faults made them:
//! - a fillet face's pipe is swept along the whole spine, which runs on past the face; where the
//!   spine comes back within the radius, the balls of the stretch beyond the face cover the face's
//!   own sheet, so a ray's crossing of the face was not found, or was found on the other
//!   stretch's balls and placed off the face (p00006 and 12 more);
//! - a cylinder face whose boundary has a traced edge was placed by its loops drawn in (u, v),
//!   not by the exact ruling parity, and near its seam the drawing put a point in (p00007).
//!
//! The fuzz: random bar-and-boss parts, the boss's foot rounded, random points in their boxes,
//! each farther from the tessellation than its chord error, held against the winding number.

use forge_bool::{Index, Where};
use forge_math::{Point3, rng::Rng};
use forge_mesh::mesh::TriMesh;
use forge_mesh::tessellate::{Quality, tessellate, tessellate_tolerant};
use forge_topo::Body;

/// The part built with no folder beside it (wasi has no temp_dir; these parts read no files).
fn run(src: &str) -> (forge_lang::Run, forge_lang::build::Cache) {
    forge_lang::run(src, forge_lang::Options::default(), Default::default())
}

/// The part's finished body.
fn body(src: &str) -> Body {
    let (r, _) = run(src);
    assert!(r.errors.is_empty(), "{}", forge_lang::report::text(&r, src));
    r.body.expect("a body")
}

/// A fine tessellation, at the finest chord that meshes.
fn mesh(b: &Body) -> Option<(TriMesh, f64)> {
    [1e-3, 3e-3, 1e-2].iter().find_map(|&chord| {
        let q = Quality {
            chord,
            max_angle: 0.05,
        };
        tessellate(b, &q)
            .or_else(|_| tessellate_tolerant(b, &q))
            .ok()
            .filter(|m| !m.triangles.is_empty() && m.open_edges().is_empty())
            .map(|m| (m, chord))
    })
}

/// Generalised winding number of a closed triangle mesh at p (van Oosterom–Strackee).
fn winding(m: &TriMesh, p: Point3) -> f64 {
    let mut w = 0.0;
    for t in &m.triangles {
        let [a, b, c] = t.map(|i| m.positions[i as usize] - p);
        let (la, lb, lc) = (a.length(), b.length(), c.length());
        let num = a.dot(b.cross(c));
        let den = la * lb * lc + a.dot(b) * lc + b.dot(c) * la + c.dot(a) * lb;
        w += 2.0 * forge_math::dmath::atan2(num, den);
    }
    w / (4.0 * std::f64::consts::PI)
}

/// The distance from p to the mesh, at least as far as `near` (stops early once nearer).
fn mesh_distance(m: &TriMesh, p: Point3, near: f64) -> f64 {
    let mut best = f64::INFINITY;
    for t in &m.triangles {
        let [a, b, c] = t.map(|i| m.positions[i as usize]);
        // a triangle's points lie within its longest edge of each corner
        let reach = a.distance(b).max(b.distance(c)).max(c.distance(a));
        if a.distance(p) - reach > best.min(near) {
            continue;
        }
        best = best.min(tri_dist(a, b, c, p));
        if best < near {
            return best;
        }
    }
    best
}

fn tri_dist(a: Point3, b: Point3, c: Point3, p: Point3) -> f64 {
    // closest point on a triangle (Ericson)
    let (ab, ac, ap) = (b - a, c - a, p - a);
    let (d1, d2) = (ab.dot(ap), ac.dot(ap));
    if d1 <= 0.0 && d2 <= 0.0 {
        return (p - a).length();
    }
    let bp = p - b;
    let (d3, d4) = (ab.dot(bp), ac.dot(bp));
    if d3 >= 0.0 && d4 <= d3 {
        return (p - b).length();
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        return (p - (a + ab * (d1 / (d1 - d3)))).length();
    }
    let cp = p - c;
    let (d5, d6) = (ab.dot(cp), ac.dot(cp));
    if d6 >= 0.0 && d5 <= d6 {
        return (p - c).length();
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        return (p - (a + ac * (d2 / (d2 - d6)))).length();
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        let w = (d4 - d3) / ((d4 - d3) + (d5 - d6));
        return (p - (b + (c - b) * w)).length();
    }
    let den = 1.0 / (va + vb + vc);
    let (v, w) = (vb * den, vc * den);
    (p - (a + ab * v + ac * w)).length()
}

fn side(inside: bool) -> Where {
    if inside { Where::Inside } else { Where::Outside }
}

/// The census's points of each repro part, with their side as OpenCascade's classifier (on the
/// part's STEP export) and, where the part meshes, the winding number both give it.
const REPROS: [(&str, [f64; 3], bool); 14] = [
    ("p00006", [-13.0476, 20.9518, 4.5435], false),
    ("p00007", [12.8085, -0.7532, 11.6154], true),
    ("p00023", [22.4246, 14.795, 45.2695], true),
    ("p00056", [-27.8605, -5.5231, 54.472], true),
    ("p00071", [14.204, 3.0975, 40.9171], true),
    ("p00080", [9.4838, -15.3947, 15.976], false),
    ("p00094", [-18.7166, 0.684, -1.3624], false),
    ("p00128", [15.4861, 17.0251, -0.4471], false),
    ("p00137", [8.0146, 7.9926, 30.6836], true),
    ("p00167", [8.3894, -2.0433, 31.06], true),
    ("p00168", [7.9429, 16.0593, -1.8177], false),
    ("p00170", [-19.3673, 14.5452, 8.0738], false),
    ("p00190", [3.9316, 4.1423, 13.659], true),
    ("p00199", [-7.8466, -15.2716, -2.1315], false),
];

/// A repro part's source (tests/index), compiled in so wasm runs it too.
fn part(id: &str) -> &'static str {
    match id {
        "p00006" => include_str!("index/p00006.fg"),
        "p00007" => include_str!("index/p00007.fg"),
        "p00023" => include_str!("index/p00023.fg"),
        "p00056" => include_str!("index/p00056.fg"),
        "p00071" => include_str!("index/p00071.fg"),
        "p00080" => include_str!("index/p00080.fg"),
        "p00094" => include_str!("index/p00094.fg"),
        "p00128" => include_str!("index/p00128.fg"),
        "p00137" => include_str!("index/p00137.fg"),
        "p00167" => include_str!("index/p00167.fg"),
        "p00168" => include_str!("index/p00168.fg"),
        "p00170" => include_str!("index/p00170.fg"),
        "p00190" => include_str!("index/p00190.fg"),
        "p00199" => include_str!("index/p00199.fg"),
        _ => panic!("no repro {id}"),
    }
}

#[test]
fn the_census_repros_are_classified_right() {
    let mut wrong = Vec::new();
    for (id, [x, y, z], inside) in REPROS {
        let b = body(part(id));
        let got = Index::new(&b).classify(Point3::new(x, y, z));
        if got != side(inside) {
            wrong.push(format!(
                "{id} ({x}, {y}, {z}): Index {got:?}, want {:?}",
                side(inside)
            ));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// A bar or block with an oblique round boss leaving its side, the concave edge between them
/// rounded (research/pipe-speed/fuzz/gen.py's parts, without their cuts).
fn bar_and_boss(seed: u64) -> String {
    let mut r = Rng::new(seed);
    let unit = |v: [f64; 3]| {
        let n = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        [v[0] / n, v[1] / n, v[2] / n]
    };
    let cross = |a: [f64; 3], b: [f64; 3]| {
        [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ]
    };
    let big = r.range(8.0, 25.0);
    let len = r.range(30.0, 80.0);
    let round_bar = r.range(0.0, 1.0) < 0.7;
    let th = r.range(0.0, std::f64::consts::TAU);
    let lean = r.range(-0.7, 0.7);
    let z = unit([forge_math::dmath::cos(th), forge_math::dmath::sin(th), lean]);
    let x = unit(cross(
        z,
        if z[0].abs() < 0.9 {
            [1.0, 0.0, 0.0]
        } else {
            [0.0, 1.0, 0.0]
        },
    ));
    let y = cross(z, x);
    let rb = big * r.range(0.2, 0.75);
    let h = r.range(0.3, 0.7) * len;
    let off = (big - rb) * r.range(-0.6, 0.6);
    let at = [y[0] * off, y[1] * off, h + y[2] * off];
    let len_b = big + r.range(5.0, 30.0);
    let rf = rb * r.range(0.08, 0.4);
    let outline = if round_bar {
        format!("circle((0, 0), {big})")
    } else {
        format!(
            "rect(corner: ({}, {}), size: ({}, {}))",
            -big,
            -big,
            2.0 * big,
            2.0 * big
        )
    };
    // the boss's plane: at `at`, its normal and x the directions from the origin to points
    format!(
        "sketch q1(on: xy) {{\n  q = pt(0, 0)\n}}\n\
         sketch s1(on: plane(at: q1.q, normal: Z, x: X)) {{\n  o = {outline}\n}}\n\
         body = extrude(s1, {len})\n\
         sketch q2(on: offset(xy, {})) {{\n  q = pt({}, {})\n}}\n\
         sketch q3(on: offset(xy, {})) {{\n  q = pt({}, {})\n}}\n\
         sketch q4(on: offset(xy, {})) {{\n  q = pt({}, {})\n}}\n\
         sketch s2(on: plane(at: q2.q, normal: axis(q1.q, q3.q), x: axis(q1.q, q4.q))) {{\n  o = circle((0, 0), {rb})\n}}\n\
         boss = extrude(s2, {len_b})\n\
         round = fillet(edges(between: (body, boss), crease: \"concave\"), {rf})\n",
        at[2], at[0], at[1], z[2], z[0], z[1], x[2], x[0], x[1]
    )
}

#[test]
fn index_agrees_with_the_winding_number_on_rounded_bodies() {
    let (mut parts, mut judged, mut wrong) = (0usize, 0usize, Vec::new());
    // every answer folded in, printed: a wasm run must print the same
    let mut digest = 0xcbf2_9ce4_8422_2325u64;
    for seed in 0..12u64 {
        let src = bar_and_boss(seed);
        let (r, _) = run(&src);
        let Some(b) = r.body.filter(|_| r.errors.is_empty()) else {
            continue;
        };
        let Some((mesh, chord)) = mesh(&b) else { continue };
        parts += 1;
        let ix = Index::new(&b);
        let bx = ix.bbox;
        let mut rng = Rng::new(seed + 100);
        for _ in 0..400 {
            let p = Point3::new(
                rng.range(bx.x.lo, bx.x.hi),
                rng.range(bx.y.lo, bx.y.hi),
                rng.range(bx.z.lo, bx.z.hi),
            );
            // nearer the mesh than its chord error, the winding number cannot judge
            if mesh_distance(&mesh, p, 20.0 * chord) < 20.0 * chord {
                continue;
            }
            judged += 1;
            let w = winding(&mesh, p);
            let got = ix.classify(p);
            digest = (digest ^ got as u64 ^ p.x.to_bits()).wrapping_mul(0x100_0000_01b3);
            if got != side(w > 0.5) {
                wrong.push(format!(
                    "seed {seed} ({}, {}, {}): Index {got:?}, winding {w:.4}",
                    p.x, p.y, p.z
                ));
            }
        }
    }
    println!("{parts} parts, {judged} points, answers' digest {digest:016x}");
    assert!(
        parts >= 8 && judged >= 2000,
        "too few cases: {parts} parts, {judged} points"
    );
    assert!(
        wrong.is_empty(),
        "{} of {judged} wrong:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
}
