//! forge-bool's `Index` (point membership) against the exact reference on flat bodies.
//!
//! The reference decides membership exactly (its `points` question: the body's pointwise set, a
//! ray perturbed symbolically); a point within LINEAR of a face's carrier, where the tolerance
//! rule decides instead, is not judged. Bodies are
//! the flat sub-programs of forge_lang::fuzz's extrude, pattern and work-plane programs. Points
//! are random in the body's box, and on purpose on the lines through the body's vertices and edge
//! midpoints along Index's first ray, so that ray meets an edge or a vertex and the witness must
//! be another.

use forge_exact_fuzz::{Reference, dump_solid, exact, guarded};
use forge_math::{Point3, Vec3, rng::Rng};
use forge_model::Combine;
use forge_topo::Body;

/// Index's first ray (forge-bool classify.rs `RAYS[0]`): points on a line along it through a
/// vertex cast that ray through the vertex.
const FIRST_RAY: [f64; 3] = [0.5773, 0.6071, 0.5463];

const CURVED: [&str; 12] = [
    "circle(", "arc(", "rounded", "revolve(", "ball(", "hole(", "fillet(", "chamfer(", "slot(", "spline(",
    "ellipse(", "coil(",
];

fn flat_kinds() -> u64 {
    let names = ["extrude", "pattern", "workplane", "param", "sketch"];
    forge_lang::fuzz::MAKERS
        .iter()
        .enumerate()
        .filter(|(_, m)| names.contains(&m.name))
        .fold(0u64, |k, (i, _)| k | 1 << i)
}

/// The flat body of language program `seed`, if it builds one.
fn flat_body(seed: u64) -> Option<Body> {
    guarded(|| {
        let run = forge_lang::fuzz::run_kinds(seed, 8, flat_kinds());
        let src: Vec<String> = forge_lang::fuzz::statements(&run.src)
            .into_iter()
            .filter(|s| !CURVED.iter().any(|c| s.contains(c)))
            .collect();
        let built = forge_lang::fuzz::build(&(src.join("\n") + "\n"));
        built.body().ok_or_else(|| "no body".into())
    })
    .inspect_err(|e| {
        if std::env::var_os("WHY").is_some() {
            eprintln!("lang/{seed}: {e}")
        }
    })
    .ok()
    .filter(|b| !b.faces.is_empty() && forge_topo::check(b).is_empty())
}

/// The reference's exact membership of each point (`points`: its pointwise boolean, the ray
/// perturbed symbolically), None for a point within LINEAR of a face's carrier, where the
/// tolerance rule decides instead.
fn exact_sides(reference: &mut Reference, body: &str, pts: &[Point3]) -> Vec<Option<bool>> {
    let ps: Vec<String> = pts
        .iter()
        .map(|p| format!("[{},{},{}]", exact(p.x), exact(p.y), exact(p.z)))
        .collect();
    let line = format!(
        "{{\"linear\":{},\"rule\":\"strict\",\"solids\":[{body}],\"points\":[{}]}}",
        exact(forge_math::tol::LINEAR),
        ps.join(",")
    );
    let out = reference.ask_raw(&line).expect("the reference answers");
    let bools = |key: &str| -> Vec<bool> {
        let at = out
            .find(&format!("\"{key}\":["))
            .unwrap_or_else(|| panic!("no {key} in {out}"));
        let rest = &out[at + key.len() + 4..];
        rest[..rest.find(']').expect("a list")]
            .split(',')
            .map(|w| w.trim() == "true")
            .collect()
    };
    let (inside, near) = (bools("inside"), bools("near"));
    assert_eq!(inside.len(), pts.len(), "{out}");
    inside
        .into_iter()
        .zip(near)
        .map(|(i, n)| (!n).then_some(i))
        .collect()
}

/// Random points, and points along the first ray from each vertex and edge midpoint.
fn points(body: &Body, rng: &mut Rng, n: usize) -> Vec<Point3> {
    let b = forge_bool::Index::new(body).bbox.grow(1.0);
    let d = Vec3::new(FIRST_RAY[0], FIRST_RAY[1], FIRST_RAY[2]);
    let d = d * (1.0 / d.length());
    let mut out: Vec<Point3> = (0..n)
        .map(|_| {
            Point3::new(
                rng.range(b.x.lo, b.x.hi),
                rng.range(b.y.lo, b.y.hi),
                rng.range(b.z.lo, b.z.hi),
            )
        })
        .collect();
    let mut on_rays: Vec<Point3> = body.vertices.iter().map(|v| v.point).collect();
    for e in body.edge_ids() {
        let e = body.edge(e);
        on_rays.push(forge_geom::CurveGeom::eval(&e.curve, e.range.mid()));
    }
    for (k, q) in on_rays.into_iter().enumerate() {
        if k % 3 == 0 {
            out.push(q - d * rng.range(0.5, 20.0));
        }
    }
    out
}

#[test]
#[cfg_attr(
    not(feature = "reference"),
    ignore = "asks the Lean reference: lake build in verify/forge-exact, then cargo test -p forge-exact-fuzz --features reference"
)]
fn index_agrees_with_the_exact_reference_on_flat_bodies() {
    let mut reference = Reference::start();
    let mut rng = Rng::new(31);
    let (mut judged, mut bodies, mut wrong) = (0usize, 0usize, Vec::new());
    for seed in 0..60 {
        let Some(body) = flat_body(seed) else { continue };
        let text = match dump_solid(&body, Combine::New) {
            Ok(t) => t,
            Err(e) => {
                if std::env::var_os("WHY").is_some() {
                    eprintln!("lang/{seed}: {e}");
                }
                continue;
            }
        };
        bodies += 1;
        let ix = forge_bool::Index::new(&body);
        let pts = points(&body, &mut rng, 200);
        for (p, side) in pts.iter().zip(exact_sides(&mut reference, &text, &pts)) {
            let Some(inside) = side else { continue };
            judged += 1;
            let got = ix.classify(*p);
            let want = if inside {
                forge_bool::Where::Inside
            } else {
                forge_bool::Where::Outside
            };
            if got != want {
                wrong.push(format!(
                    "lang/{seed} ({}, {}, {}): Index {got:?}, exact {want:?}",
                    p.x, p.y, p.z
                ));
            }
        }
    }
    println!("{bodies} bodies, {judged} points judged");
    assert!(
        bodies >= 20 && judged >= 4000,
        "too few cases: {bodies} bodies, {judged} points"
    );
    assert!(
        wrong.is_empty(),
        "{} of {judged} points wrong:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
}
