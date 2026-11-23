//! The exact bits of every suite part scaled (Step::Scale, alone and between rigid steps): its
//! vertices, edge ranges and tolerances, and its exact volume. Native and wasm print the same lines.
//!   cargo run --release -p forge-model --example scale_bits
//!   cargo run --profile wasm -p forge-model --example scale_bits --target wasm32-wasip1
use forge_math::hash::BitHasher;
use forge_math::{Point3, Vec3};
use forge_model::{BodyFeature, Feature, Step, suite};

fn main() {
    let c = Point3::new(3.0, -7.0, 2.5);
    for (name, part) in suite::parts() {
        for (j, steps) in [
            vec![Step::Scale { about: c, k: 2.5 }],
            vec![Step::Scale { about: c, k: 0.375 }],
            vec![
                Step::Translate(Vec3::new(5.0, 0.0, -2.0)),
                Step::Scale { about: c, k: 1.7 },
                Step::Rotate {
                    origin: Point3::ORIGIN,
                    axis: Vec3::Z,
                    angle: 0.7,
                },
            ],
        ]
        .into_iter()
        .enumerate()
        {
            let mut p = part.clone();
            p.features
                .push(Feature::Body(Box::new(BodyFeature::Move { steps, copy: false })));
            let mut h = BitHasher::new();
            match p.regenerate() {
                Ok(b) => {
                    for v in &b.vertices {
                        h.p3(v.point);
                        h.f64(v.tol);
                    }
                    for e in &b.edges {
                        h.f64(e.range.lo);
                        h.f64(e.range.hi);
                        h.f64(e.tol);
                    }
                    h.u64(b.faces.len() as u64);
                    match forge_topo::mass::mass(&b) {
                        Ok(m) => {
                            h.f64(m.volume);
                            h.f64(m.area);
                        }
                        Err(_) => h.u64(u64::MAX - 1),
                    }
                }
                Err(_) => h.u64(u64::MAX),
            }
            println!("{name} {j} {:016x}", h.finish());
        }
    }
}
