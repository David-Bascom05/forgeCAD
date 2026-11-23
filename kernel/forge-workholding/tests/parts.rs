//! Every built-in body backed by a real part: the part builds (forge-lang) and lies in the body's
//! envelope, which it fills a good part of. A part is drawn in its body's frame: the envelope's
//! least corner at the origin (lathe bodies: the spindle along z, from z = 0).
use forge_mesh::{Quality, tessellate};
use forge_workholding::{Box3, Library, Prim, lathe, mill};
use std::path::Path;

fn build(file: &str) -> (Vec<[f64; 3]>, f64) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("parts").join(file);
    let src = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let opts = forge_lang::Options {
        dir: path.parent().map(Path::to_path_buf),
        ..forge_lang::Options::default()
    };
    let (run, _) = forge_lang::run(&src, opts, forge_lang::build::Cache::default());
    assert!(run.ok(), "{file}: {:?}", run.errors);
    let body = run.body.expect("a body");
    let q = Quality {
        chord: 0.01f64.min(Quality::default().chord),
        ..Quality::default()
    };
    let mesh = tessellate(&body, &q).unwrap_or_else(|e| panic!("{file}: {e:?}"));
    let v = run.measure.as_ref().map(|m| m.volume).unwrap_or(f64::NAN);
    (mesh.positions.iter().map(|p| [p.x, p.y, p.z]).collect(), v)
}

fn check(file: &str, env: &[Prim]) {
    let (pts, vol) = build(file);
    let worst = pts
        .iter()
        .map(|x| env.iter().map(|p| p.outside(*x)).fold(f64::INFINITY, f64::min))
        .fold(0.0, f64::max);
    assert!(worst <= 0.01, "{file}: a vertex {worst} outside its envelope");
    let ev: f64 = env.iter().map(Prim::volume).sum();
    assert!(vol > 0.0 && vol <= ev * (1.0 + 1e-9), "{file}: volume {vol}, envelope {ev}");
    assert!(vol >= 0.25 * ev, "{file}: volume {vol} of envelope {ev}");
}

#[test]
fn builtin_parts_lie_in_their_envelopes() {
    let l = Library::builtin();
    let mut n = 0;
    let stock = Box3 {
        min: [0.0, 0.0, 0.0],
        max: [100.0, 60.0, 80.0],
    };
    for (r, _) in &l.records {
        if r.parts.is_empty() {
            continue;
        }
        if r.kind.is_lathe() {
            let s = lathe::chuck("c", r, 60.0, 0.0, 10.0).unwrap();
            for ring in &s.rings {
                let Some(f) = r.parts.get(&ring.name) else { continue };
                // the ring's outer radius and z span, as a cylinder from z = 0
                let rr = ring.poly.iter().map(|p| p[0]).fold(0.0, f64::max);
                let z0 = ring.poly.iter().map(|p| p[1]).fold(f64::INFINITY, f64::min);
                let z1 = ring.poly.iter().map(|p| p[1]).fold(f64::NEG_INFINITY, f64::max);
                check(f, &[Prim::post(0.0, 0.0, rr, 0.0, z1 - z0)]);
                n += 1;
            }
            continue;
        }
        let p = mill::vise("v", r, &stock, 0, r.get("jaw_height").unwrap(), None, None).unwrap();
        for b in &p.bodies {
            let Some(f) = r.parts.get(&b.name) else { continue };
            let (lo, _) = b.bounds();
            let env: Vec<Prim> = b
                .prims
                .iter()
                .map(|x| x.posed(&forge_workholding::Pose::shift([-lo[0], -lo[1], -lo[2]])))
                .collect();
            check(f, &env);
            n += 1;
        }
    }
    assert_eq!(n, 4);
}
