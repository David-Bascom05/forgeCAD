//! `collide::check_box`: a pair of bodies over a whole box of joint values.
use forge_machine::collide::{self, BoxVerdict};
use forge_machine::library::builtin;

fn builtin_named(n: &str) -> Option<forge_machine::MachineDef> {
    builtin().into_iter().find(|m| m.name == n)
}
use forge_math::interval::Interval;

fn travel(m: &forge_machine::MachineDef) -> Vec<Interval> {
    m.joints
        .iter()
        .map(|j| Interval::new(j.range[0], j.range[1]))
        .collect()
}

fn body<'a>(bs: &'a [collide::CBody], n: &str) -> &'a collide::CBody {
    bs.iter().find(|b| b.name == n).unwrap()
}

#[test]
fn vmc3_table_and_head_are_clear_through_all_travel() {
    let m = builtin_named("generic-vmc3").unwrap();
    let bs = collide::machine_bodies(&m);
    let (v, n) = collide::check_box(
        &m,
        body(&bs, "table"),
        body(&bs, "head"),
        &travel(&m),
        m.clearance,
        4096,
    );
    assert_eq!(v, BoxVerdict::Clear, "after {n} boxes");
}

#[test]
fn trunnion_head_reaches_its_cradle_inside_travel() {
    let m = builtin_named("generic-trunnion-bc").unwrap();
    let bs = collide::machine_bodies(&m);
    let (a, b) = (body(&bs, "cradle"), body(&bs, "head"));
    let (v, _) = collide::check_box(&m, a, b, &travel(&m), m.clearance, 4096);
    let BoxVerdict::Hit { q, .. } = v else {
        panic!("{v:?}")
    };
    for (x, j) in q.iter().zip(&m.joints) {
        assert!(j.range[0] <= *x && *x <= j.range[1], "{x} outside {:?}", j.range);
    }
    assert!(collide::dense_overlap(a, b, &forge_machine::kin::pose(&m, &q)));
}

/// Clear over a box is never contradicted by a sample inside it; a budget of one box that is not
/// clear says Undecided, never Clear.
#[test]
fn clear_boxes_are_never_contradicted_by_samples() {
    let mut x: u64 = 11;
    let mut rnd = || {
        x = x
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (x >> 11) as f64 / (1u64 << 53) as f64
    };
    let (mut clear, mut samples) = (0, 0);
    for m in builtin() {
        let bs = collide::machine_bodies(&m);
        for i in 0..bs.len() {
            for k in i + 1..bs.len() {
                let (a, b) = (&bs[i], &bs[k]);
                if a.link == b.link {
                    continue;
                }
                for _ in 0..6 {
                    // a random sub-box of the travel
                    let qb: Vec<Interval> = m
                        .joints
                        .iter()
                        .map(|j| {
                            let (u, v) = (rnd(), rnd());
                            let w = j.range[1] - j.range[0];
                            Interval::new(j.range[0] + u.min(v) * w, j.range[0] + u.max(v) * w)
                        })
                        .collect();
                    let (v, _) = collide::check_box(&m, a, b, &qb, 0.0, 512);
                    if v != BoxVerdict::Clear {
                        continue;
                    }
                    clear += 1;
                    for _ in 0..20 {
                        let q: Vec<f64> = qb.iter().map(|x| x.lo + rnd() * (x.hi - x.lo)).collect();
                        let pose = forge_machine::kin::pose(&m, &q);
                        assert!(
                            !collide::dense_overlap(a, b, &pose),
                            "{}: {} / {} cleared over {qb:?} but overlap at {q:?}",
                            m.name,
                            a.name,
                            b.name
                        );
                        samples += 1;
                    }
                }
            }
        }
    }
    assert!(
        clear > 20 && samples > 400,
        "{clear} clear boxes, {samples} samples"
    );
    let m = builtin_named("generic-trunnion-bc").unwrap();
    let bs = collide::machine_bodies(&m);
    let (v, n) = collide::check_box(
        &m,
        body(&bs, "cradle"),
        body(&bs, "head"),
        &travel(&m),
        m.clearance,
        1,
    );
    assert!(
        matches!(v, BoxVerdict::Undecided { .. } | BoxVerdict::Hit { .. }),
        "{v:?}"
    );
    assert_eq!(n, 1);
}
