use forge_cam_geom::check::*;
use forge_cam_geom::ival::Ival;
use forge_cam_geom::tool::{Cutter, Holder, assemble};
fn main() {
    let holder = Holder {
        profile: vec![[0.0, 25.0], [20.0, 25.0], [25.0, 40.0], [60.0, 40.0]],
    };
    let t = assemble(&Cutter::Flat { d: 6.0 }, 25.0, 6.0, 35.0, Some(&holder)).unwrap();
    let mu = 1e-5;
    let f = Face::new(
        Surf::Plane {
            o: [-40.0, 40.0, 0.0],
            u: [0.0, -1.0, 0.0],
            v: [0.0, 0.0, 1.0],
        },
        Ival::new(-mu, 80.0 + mu),
        Ival::new(-mu, 12.0 + mu),
        None,
        mu,
        4,
    );
    let m = Move::Arc {
        c: [-40.0, -40.0, 2.0],
        x: [-0.9998072404820648, -0.01963369246062829, 0.0],
        y: [0.01963369246062829, -0.9998072404820648, 0.0],
        r: 3.0,
        t0: -0.009817792466785748,
        t1: 0.009817792466785748,
    };
    let (v, n) = certify(&t.bodies[0], &f, &m, -0.01, 0.0025, 200_000);
    println!("{v:?} {n}");
}
