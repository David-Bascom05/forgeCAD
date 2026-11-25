//! An n-ary boolean's helper threads (`forge_math::par`) spend its budget: what an op counts is the
//! same on one thread and on several, and a budget it runs past refuses it, `TooLong`, wherever the
//! work was. (Before, helpers ran with no budget: the app's booleans go through n-ary, and a runaway
//! on a helper was neither bounded nor stopped.)
#![cfg(not(target_family = "wasm"))]
use forge_bool::{BoolError, Op, nary};
use forge_math::{Frame, Point2, UnitVec3, Vec3};
use forge_step::{Limits, import};
use forge_topo::Body;

/// VR-14 #178820 (12 faces, from the Haas machine models) and four bores down through it, as
/// forge-stress's bore but spread about its middle: four pairs of its NURBS faces and a bore, which
/// an n-ary boolean's pair stage hands out to its threads.
fn part_and_bores() -> (Body, Vec<Body>) {
    let p = format!(
        "{}/tests/data/haas/vr14_bore_12faces.step",
        env!("CARGO_MANIFEST_DIR")
    );
    let imp = import(&std::fs::read(&p).unwrap(), &Limits::default()).unwrap();
    let b = imp.solids.into_iter().next().unwrap().body;
    let bx = b.faces_box();
    let small = bx.x.width().min(bx.y.width()).min(bx.z.width());
    let len = 3.0 * bx.diagonal();
    let bores = [(-0.15, -0.15), (0.15, -0.15), (-0.15, 0.15), (0.15, 0.15)]
        .iter()
        .enumerate()
        .map(|(k, &(dx, dy))| {
            let at = bx.center() - Vec3::from(UnitVec3::Z) * (0.5 * len)
                + Vec3::new(dx * bx.x.width(), dy * bx.y.width(), 0.0);
            let p = forge_ops::Profile {
                plane: Frame::from_z(at, UnitVec3::Z),
                loops: vec![forge_ops::Profile::circle(
                    Point2::ORIGIN,
                    0.06 * small,
                    7001 + 20 * k as u64,
                )],
            };
            forge_ops::extrude(&p, len, 7000 + 20 * k as u64).unwrap()
        })
        .collect();
    (b, bores)
}

fn bored(b: &Body, bores: &[Body], threads: usize) -> Result<Body, BoolError> {
    let tools: Vec<&Body> = std::iter::once(b).chain(bores).collect();
    nary(&tools, &vec![Op::Subtract; bores.len()], threads)
}

/// What `f` spends, under a budget it cannot run out of.
fn spent<T>(f: impl FnOnce() -> T) -> (u64, T) {
    let _all = forge_geom::budget::scoped(u64::MAX >> 1);
    let before = forge_geom::budget::left();
    let r = f();
    (before - forge_geom::budget::left(), r)
}

#[test]
fn an_nary_booleans_helpers_spend_its_budget() {
    // (each run on a part read afresh: a face keeps what a boolean worked out of it, and a second
    // boolean on it spends a thirtieth)
    let (b, bores) = part_and_bores();
    let (one, r1) = spent(|| bored(&b, &bores, 1));
    let r1 = r1.unwrap();
    assert!(one > 1 << 20, "spends {one}");
    // the helpers made to start at once: the same spend, counted whole wherever it was made
    forge_math::par::force_helpers(true);
    let (b, bores) = part_and_bores();
    let (four, r4) = spent(|| bored(&b, &bores, 4));
    let r4 = r4.unwrap();
    assert!(forge_math::par::helpers_started() > 0, "no helper started");
    assert_eq!(four, one);
    assert_eq!(r4.faces.len(), r1.faces.len());
    // a budget a little short of it: refused, on helpers as on one thread
    for threads in [1, 4] {
        let (b, bores) = part_and_bores();
        let _short = forge_geom::budget::scoped(one - one / 8);
        let r = bored(&b, &bores, threads);
        assert!(
            matches!(r, Err(BoolError::TooLong)),
            "{threads} threads: {:?}",
            r.map(|x| x.faces.len())
        );
    }
    forge_math::par::force_helpers(false);
}
