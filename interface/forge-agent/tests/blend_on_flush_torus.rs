//! A rounded boss on a part of a revolve, joined to the whole revolve: the part's curved face lies on
//! the whole's, and the round only touches that surface, along its rim.
use forge_agent::model::Model;
use forge_bool::{Op, boolean};
use forge_math::dmath;
use forge_topo::Body;

const PROFILE: &str = "loop line 0 30 0 -100 line 0 -100 50 -100 arc 150 -100 100 180 -90 line 150 0 150 30 line 150 30 0 30";

fn body(src: &str) -> (Body, f64) {
    let mut m = Model::new(src);
    assert!(m.error.is_none(), "{src}: {:?}", m.error.map(|e| e.msg));
    let v = m.mass().expect("mass").volume;
    (m.body().expect("a body").clone(), v)
}

#[test]
fn a_round_touches_the_flush_torus_and_does_not_cut_it() {
    let (ring, whole) = body(&format!("w = 1\nh = 2\nrevolve xz axis y {PROFILE}\n"));
    let copy = |k: usize| {
        let (s, c) = dmath::sin_cos((120.0 * k as f64).to_radians());
        let part = format!("revolve 60 z {s} {} 0 x {c} {s} 0 at 0 0 0 axis y {PROFILE}\n", -c);
        let box_ = format!("extrude 80 z 0 0 1 x {c} {s} 0 at 0 0 -70 rect 95 45 20 6\n");
        // a body's names come from its features' lines: copies with lines of their own
        let lines: String = (0..10 * (k + 1)).map(|i| format!("param{i} = 1\n")).collect();
        (part.clone(), format!("{lines}{part}{box_}fillet 2 (edges concave)\n"))
    };
    let (_, wedge) = body(&copy(0).0);
    assert!((wedge - whole / 6.0).abs() < 1e-3, "{wedge} {whole}");
    let added = body(&copy(0).1).1 - wedge;
    assert!(added > 1000.0, "{added}");

    let mut out = ring;
    for k in 0..3 {
        // the boss stands out of the torus inside its wedge, away from the wedge's ends
        let boss = body(&copy(k).1).0;
        out = boolean(&out, &boss, Op::Union, 7 + k as u64).expect("the union");
        let problems = forge_topo::check(&out);
        assert!(problems.is_empty(), "copy {k}: {problems:?}");
        let v = forge_topo::mass::mass(&out).expect("mass").volume;
        let want = whole + (k + 1) as f64 * added;
        assert!((v - want).abs() < 1e-2, "copy {k}: {v} against {want}");
    }
}
