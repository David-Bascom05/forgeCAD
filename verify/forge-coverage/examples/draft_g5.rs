//! G5 (research/kernel-audit/coverage.md) as a real drafted part, through forge-model's regen.
//! A lid B (10 x 10 x 5) sits on a block A (10 x 10 x 10) with the same footprint. B's bottom face
//! is drafted by `angle` about its middle line (pull +y, neutral plane y = 5), so it dips into A
//! towards y = 0; B's end faces are drafted by `angle` about A's top (pull +z, neutral plane
//! z = 10). B's dipped corner then sits 5 theta^2 off A's side face y = 0: 1.52 LINEAR at a 0.1
//! degree draft, inside G5a's (LINEAR, phi LINEAR]. Then A is combined with the drafted lid.
//! cargo run --release -p forge-coverage --example draft_g5 -- [degrees ...]   (default 0.1)
use forge_coverage::draft::lid;
use forge_coverage::{Outcome, block, kernel_on};
use forge_math::Point3;
use forge_model::{Combine, Part};
use forge_topo::Body;

fn verdict(r: Result<Body, forge_model::RegenError>) -> String {
    match r {
        Err(e) => format!("error {}", format!("{e:?}").chars().take(90).collect::<String>()),
        Ok(b) => {
            let pr = forge_topo::check(&b);
            if let Some(p) = pr.first() {
                format!("invalid {p:?}")
            } else {
                let q = forge_mesh::Quality {
                    chord: 0.02,
                    max_angle: 0.5,
                };
                match forge_mesh::tessellate(&b, &q) {
                    Ok(m) => format!("ok {:.6}", m.volume()),
                    Err(e) => format!("mesh {e:?}"),
                }
            }
        }
    }
}

fn main() {
    let degs: Vec<f64> = {
        let d: Vec<f64> = std::env::args().skip(1).filter_map(|a| a.parse().ok()).collect();
        if d.is_empty() { vec![0.1] } else { d }
    };
    for deg in degs {
        let angle = deg.to_radians();
        let drafted = Part { features: lid(angle) }
            .regenerate()
            .expect("the drafted lid");
        // where the dipped corner is, against A's side y = 0
        let corner = drafted
            .vertex_ids()
            .map(|v| drafted.vertex(v).point)
            .filter(|p| p.x == 0.0 && p.z < 10.0)
            .fold(None, |m: Option<Point3>, p| {
                Some(m.map_or(p, |q| if p.y.abs() < q.y.abs() { p } else { q }))
            });
        println!(
            "draft {deg} deg ({angle:.6e} rad): dipped corner {corner:?} (y / LINEAR = {:.3})",
            corner.map_or(f64::NAN, |c| c.y / forge_math::tol::LINEAR)
        );
        println!(
            "  expected: add {:.6}  cut {:.6}  keep {:.6}",
            forge_coverage::draft::expected(angle, Combine::Add),
            forge_coverage::draft::expected(angle, Combine::Cut),
            forge_coverage::draft::expected(angle, Combine::Keep)
        );
        for (name, op) in [
            ("add", Combine::Add),
            ("cut", Combine::Cut),
            ("keep", Combine::Keep),
        ] {
            let mut f = lid(angle);
            f.push(forge_coverage::draft::up_block(
                [0.0, 0.0, 0.0],
                10.0,
                10.0,
                10.0,
                op,
            ));
            let p = Part { features: f };
            println!(
                "  lid {name} block: sequential {}  nary {}",
                verdict(p.regenerate()),
                verdict(p.regenerate_nary(1))
            );
        }
        // the bare boolean, A first (A minus the lid is the cap fixture's subtract)
        let a = block([[0.0, 10.0], [0.0, 10.0], [0.0, 10.0]], 9);
        for (name, op) in [
            ("union", forge_bool::Op::Union),
            ("subtract", forge_bool::Op::Subtract),
            ("intersect", forge_bool::Op::Intersect),
        ] {
            let got: Outcome = kernel_on(a.clone(), drafted.clone(), op);
            println!("  block {name} lid (bare boolean): {}", got.word());
        }
    }
}
