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
        .map(|j| Interval::new(j.range.unwrap()[0], j.range.unwrap()[1]))
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
        4096,
        false,
    );
    assert_eq!(v, BoxVerdict::Clear, "after {n} boxes");
}

/// Every pair of every built-in machine over all its travel outside its zones: proven Clear,
/// except the pairs listed with why they are not decided (UNKNOWN: neither proven clear nor
/// found to touch within the budget).
#[test]
fn builtin_pairs_are_proven_clear_over_all_travel_outside_their_zones() {
    // (machine, a, b): not decided within 200 000 boxes (nor 1 000 000): the mill-turn's B head
    // against the bodies its zones keep it from, over six or seven joints, with the zones' faces
    // (travel) about 1 mm from contact. Each is UNKNOWN, not clear; CAM-MACHINE has the list and
    // is sampling the faces.
    const UNDECIDED: &[(&str, &str, &str)] = &[
        ("generic-mill-turn", "headstock", "bhead"),
        ("generic-mill-turn", "chuck", "bhead"),
        ("generic-mill-turn", "subchuck", "bhead"),
        ("generic-mill-turn", "subhead", "bhead"),
    ];
    let mut proven = 0;
    for m in builtin() {
        let bs = collide::machine_bodies(&m);
        for i in 0..bs.len() {
            for k in i + 1..bs.len() {
                let (a, b) = (&bs[i], &bs[k]);
                let listed = UNDECIDED.iter().any(|(mm, x, y)| {
                    *mm == m.name && ((*x == a.name && *y == b.name) || (*x == b.name && *y == a.name))
                });
                // a listed pair gets a small budget: it must not turn out to touch
                let budget = if listed { 2_000 } else { 200_000 };
                let (v, n) = collide::check_box(&m, a, b, &travel(&m), budget, true);
                match v {
                    BoxVerdict::NotChecked => {}
                    BoxVerdict::Clear => {
                        assert!(
                            !listed,
                            "{} {} / {} is now Clear: take it off the list",
                            m.name, a.name, b.name
                        );
                        proven += 1;
                    }
                    BoxVerdict::Undecided { .. } if listed => {}
                    v => panic!("{} {} / {}: {v:?} after {n} boxes", m.name, a.name, b.name),
                }
            }
        }
    }
    assert!(proven >= 15, "{proven} pairs proven");
}

#[test]
fn trunnion_head_is_clear_of_its_cradle_over_all_travel() {
    let m = builtin_named("generic-trunnion-bc").unwrap();
    let bs = collide::machine_bodies(&m);
    let (v, n) = collide::check_box(
        &m,
        body(&bs, "cradle"),
        body(&bs, "head"),
        &travel(&m),
        200_000,
        true,
    );
    assert_eq!(v, BoxVerdict::Clear, "after {n} boxes");
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
                            let w = j.range.unwrap()[1] - j.range.unwrap()[0];
                            Interval::new(
                                j.range.unwrap()[0] + u.min(v) * w,
                                j.range.unwrap()[0] + u.max(v) * w,
                            )
                        })
                        .collect();
                    let (v, _) = collide::check_box(&m, a, b, &qb, 512, false);
                    if v == BoxVerdict::NotChecked {
                        break;
                    }
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
    let (v, n) = collide::check_box(&m, body(&bs, "cradle"), body(&bs, "head"), &travel(&m), 1, false);
    assert!(
        matches!(v, BoxVerdict::Undecided { .. } | BoxVerdict::Hit { .. }),
        "{v:?}"
    );
    assert_eq!(n, 1);
}

/// A box wholly inside a zone is skipped with `outside_zones` (the control forbids it), and a
/// pair `pair_rule` never checks is NotChecked.
#[test]
fn zones_and_unchecked_pairs() {
    let m = builtin_named("generic-mill-turn").unwrap();
    let z = m.zones.first().expect("the generic mill-turn has zones");
    let bs = collide::machine_bodies(&m);
    // every joint at home, the zone's joints strictly inside it
    let mut q: Vec<Interval> = m.joints.iter().map(|j| Interval::new(j.home, j.home)).collect();
    for (j, (lo, hi)) in z.joints.iter().zip(z.min.iter().zip(&z.max)) {
        let i = m.joint_index(j).unwrap();
        let (lo, hi) = (
            lo.max(m.joints[i].range.unwrap()[0]),
            hi.min(m.joints[i].range.unwrap()[1]),
        );
        let w = hi - lo;
        q[i] = Interval::new(lo + 0.25 * w, hi - 0.25 * w);
    }
    let mut skipped = 0;
    for i in 0..bs.len() {
        for k in i + 1..bs.len() {
            let (v, n) = collide::check_box(&m, &bs[i], &bs[k], &q, 4096, true);
            match v {
                BoxVerdict::NotChecked => assert_eq!(n, 0),
                v => {
                    assert_eq!(v, BoxVerdict::Clear, "{} / {}", bs[i].name, bs[k].name);
                    assert_eq!(n, 1, "skipped at once");
                    skipped += 1;
                }
            }
        }
    }
    assert!(skipped > 0);
    // same-link bodies are never checked
    let same = bs
        .iter()
        .enumerate()
        .find_map(|(i, a)| bs[i + 1..].iter().find(|b| b.link == a.link).map(|b| (a, b)));
    if let Some((a, b)) = same {
        assert_eq!(
            collide::check_box(&m, a, b, &travel(&m), 16, false),
            (BoxVerdict::NotChecked, 0)
        );
    }
}
