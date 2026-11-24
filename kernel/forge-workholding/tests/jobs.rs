//! Sample jobs: workholding bodies through forge-cam's certified checker (G2) and the lathe
//! envelopes, each a pair: the path that clears, and the one that does not, naming the body.
use forge_cam::ops::{self, OpPath, PMove, Phase, ToolUse};
use forge_workholding::{Box3, Library, Pose, Workholding, lathe, mill};

fn tool() -> ToolUse {
    let l = forge_tooling::Library::builtin();
    let t = l.tool("flat-6").unwrap().clone();
    let shape = t
        .shape(t.lcf + 5.0, Some(l.holder("bt40-er32").unwrap()))
        .unwrap();
    ToolUse {
        t: 1,
        tool: t,
        holder: Some("bt40-er32".into()),
        stickout: 0.0,
        shape,
        probe: false,
    }
}

/// A straight cut from `a` to `b` (tip positions), entered by a plunge from above.
fn pass(a: [f64; 3], b: [f64; 3], up: f64) -> OpPath {
    OpPath {
        tool: 0,
        start: [a[0], a[1], up],
        moves: vec![
            PMove::Line {
                to: a,
                f: 300.0,
                phase: Phase::Plunge,
            },
            PMove::Line {
                to: b,
                f: 600.0,
                phase: Phase::Cut,
            },
            PMove::Rapid { to: [b[0], b[1], up] },
        ],
        rpm: 8000.0,
        coolant: forge_post::Coolant::Flood,
    }
}

/// The bodies the path met within the clearance (by name), none if clear.
fn hits(w: &Workholding, p: &OpPath, clear: f64) -> Vec<String> {
    let faces = w.faces(1e-9);
    let s = ops::certify_fixtured(p, &tool(), &[], &[(0, vec![])], &faces, clear, 0.01, 0.5);
    let mut v: Vec<String> = s
        .checks
        .iter()
        .filter(|c| c.rule == "fixture_collision")
        .filter_map(|c| c.other.strip_prefix("fixture face ").and_then(|x| x.parse().ok()))
        .filter_map(|id| w.body_of(id).map(|(d, b)| format!("{d}/{b}")))
        .collect();
    v.sort();
    v.dedup();
    assert_eq!(s.clear, v.is_empty(), "{:?}", s.checks);
    v
}

#[test]
fn a_vise_on_parallels_a_cut_too_deep_hits_a_parallel() {
    let l = Library::builtin();
    let stock = Box3 {
        min: [0.0; 3],
        max: [100.0, 60.0, 40.0],
    };
    // gripping 20 of a 50 mm jaw: parallels 30 high
    let v = mill::vise(
        "vise",
        l.get("standin-vise-150").unwrap(),
        &stock,
        0,
        20.0,
        Some(l.get("standin-parallels").unwrap()),
        None,
    )
    .unwrap();
    assert!(v.bodies.iter().filter(|b| b.name.starts_with("parallel")).count() == 2);
    assert_eq!(v.bodies[0].part.as_deref(), Some("standin-vise-150/base.fg"));
    let w = Workholding { placed: vec![v] };
    // a slot along y over the parallel at x 0..3 (the tool spans x 3..9): 2 above the seat
    // clears it by 2; through the stock's bottom it meets the parallel
    assert!(hits(&w, &pass([6.0, -20.0, 2.0], [6.0, 80.0, 2.0], 60.0), 1.0).is_empty());
    assert_eq!(
        hits(&w, &pass([6.0, -20.0, -1.0], [6.0, 80.0, -1.0], 60.0), 1.0),
        vec!["vise/parallel 1"]
    );
    // a parallel 25 high is not in the set: refused, with the clamps that are
    let e = mill::vise(
        "vise",
        l.get("standin-vise-150").unwrap(),
        &stock,
        0,
        25.0,
        Some(l.get("kurt-dx6").unwrap()),
        None,
    );
    assert!(e.is_err());
    let e = mill::vise(
        "vise",
        l.get("standin-vise-150").unwrap(),
        &stock,
        0,
        25.0,
        None,
        None,
    )
    .unwrap_err();
    assert!(e.to_string().contains("parallels 25.00"), "{e}");
    // the leftover under the jaws' top is held
    assert_eq!(w.held([0.0, 0.0, 0.0], [100.0, 60.0, 19.0], 1.0), Some("vise"));
    assert_eq!(w.held([0.0, 0.0, 0.0], [100.0, 60.0, 25.0], 1.0), None);
}

#[test]
fn a_strap_clamp_the_path_beside_it_clears_the_one_under_it_does_not() {
    let l = Library::builtin();
    let stock = Box3 {
        min: [0.0; 3],
        max: [100.0, 60.0, 30.0],
    };
    let c = mill::strap_clamp(
        "clamp",
        l.get("standin-strap-m12").unwrap(),
        l.get("standin-step-block-25").unwrap(),
        &stock,
        0.0,
        [50.0, 60.0],
        [0.0, -1.0],
        10.0,
        40.0,
    )
    .unwrap();
    // the heel on step 2 (25 high, the stock's top 30): the strap tilts
    assert!(c.notes[0].contains("step 2"), "{:?}", c.notes);
    let w = Workholding { placed: vec![c] };
    // a facing pass 1 below the top along y = 20 (the nose ends at y = 50): clear
    assert!(hits(&w, &pass([-10.0, 20.0, 29.0], [110.0, 20.0, 29.0], 80.0), 1.0).is_empty());
    // the same pass at y = 55 runs under the strap's nose
    let h = hits(&w, &pass([-10.0, 55.0, 29.0], [110.0, 55.0, 29.0], 80.0), 1.0);
    assert!(h.contains(&"clamp/strap".to_string()), "{h:?}");
    // the reach report: beside the strap, a holder 25 in radius needs the tool out past its top
    let r = w.reach(&[[50.0, 45.0, 29.0]], 25.0, 1.0).unwrap();
    // the strap's top is at 30 + 16: 46 + 1 − 29
    assert!(r.body == "clamp/strap" && r.stickout == 18.0, "{r:?}");
    // a strap whose heel would land on the stock is refused
    assert!(
        mill::strap_clamp(
            "c",
            l.get("standin-strap-m12").unwrap(),
            l.get("standin-step-block-25").unwrap(),
            &stock,
            0.0,
            [50.0, 60.0],
            [0.0, -1.0],
            90.0,
            95.0
        )
        .is_err()
    );
}

#[test]
fn a_dovetail_job_five_sides_open_above_the_grip() {
    let l = Library::builtin();
    let stock = Box3 {
        min: [0.0; 3],
        max: [40.0, 40.0, 40.0],
    };
    let d = mill::dovetail("dovetail", l.get("standin-dovetail-60").unwrap(), &stock, 0, 30.0).unwrap();
    assert!(d.notes[0].contains("tenon 30.00"), "{:?}", d.notes);
    let w = Workholding { placed: vec![d] };
    let c = stock.mid();
    // each side's tool axis (3+2): a facing pass 0.5 into the side, at height h; the frame in
    // which the tool comes down −z is the pose taking its axis to +z
    type Side = fn(f64, f64) -> [f64; 3];
    let sides: [([f64; 3], Side); 4] = [
        ([1.0, 0.0, 0.0], |t, h| [39.5, t, h]),
        ([-1.0, 0.0, 0.0], |t, h| [0.5, t, h]),
        ([0.0, 1.0, 0.0], |t, h| [t, 39.5, h]),
        ([0.0, -1.0, 0.0], |t, h| [t, 0.5, h]),
    ];
    for (dir, side) in sides {
        let p = Pose::tool_along(dir, c).unwrap();
        let posed = Workholding {
            placed: w
                .placed
                .iter()
                .map(|x| {
                    let mut x = x.clone();
                    for b in &mut x.bodies {
                        b.prims = b.prims.iter().map(|q| q.posed(&p)).collect();
                    }
                    x
                })
                .collect(),
        };
        let run = |h: f64| {
            let (a, b) = (p.point(side(-10.0, h)), p.point(side(50.0, h)));
            hits(&posed, &pass(a, b, a[2] + 60.0), 1.0)
        };
        // the jaws stand 3 above the seat: a tool 3 in radius at height 8 clears them by 2
        assert!(run(8.0).is_empty(), "{dir:?}: {:?}", run(8.0));
        assert!(!run(5.0).is_empty(), "{dir:?}");
    }
    // a tenon outside the fixture's range is refused
    assert!(mill::dovetail("d", l.get("standin-dovetail-60").unwrap(), &stock, 0, 10.0).is_err());
    // a maker's dovetail whose body sizes are not published: refused, naming the field
    let e = mill::dovetail("d", l.get("5thaxis-dm25").unwrap(), &stock, 0, 30.0).unwrap_err();
    assert!(e.to_string().contains("not published"), "{e}");
}

#[test]
fn a_lathe_chuck_job_and_its_stick_out() {
    let l = Library::builtin();
    let ch = lathe::chuck("chuck", l.get("standin-chuck-200").unwrap(), 60.0, 0.0, 25.0).unwrap();
    let (nose, clear) = (0.8, 1.0);
    // skimming the bar's OD (nose centre at ρ 30.3) toward the chuck
    let path = |zmin: f64| vec![[30.3, 81.0], [30.3, zmin]];
    let ok = lathe::check_path(std::slice::from_ref(&ch), &path(2.0), nose, clear, 0.1);
    assert!(ok.hit.is_none(), "{ok:?}");
    let bad = lathe::check_path(std::slice::from_ref(&ch), &path(0.5), nose, clear, 0.1);
    assert_eq!(bad.hit.as_ref().map(|h| h.0.as_str()), Some("chuck/jaws"));
    // the stick-out: 80 out of the jaws; the pass to z 2 needs 79.8, to 0.5 needs 81.3
    let (need, have) = lathe::stick_out(0.0, 80.0, 2.0, nose, clear);
    assert!(need <= have);
    let (need, have) = lathe::stick_out(0.0, 80.0, 0.5, nose, clear);
    assert!((need - have - 1.3).abs() < 1e-9);
    // 200 out on ⌀60 is over the 3 × guideline unless the far end is supported
    assert!(lathe::overhang(200.0, 60.0, false).is_some());
    assert!(lathe::overhang(200.0, 60.0, true).is_none());
    assert!(lathe::overhang(80.0, 60.0, false).is_none());
    // a tailstock centre at the far end is one more body of revolution
    let ce = lathe::centre("tail", l.get("standin-live-centre").unwrap(), 200.0, 2.0, 50.0).unwrap();
    let far = lathe::check_path(
        &[ch.clone(), ce],
        &[[30.3, 199.0], [30.3, 150.0]],
        nose,
        clear,
        0.1,
    );
    assert!(far.hit.is_none(), "{far:?}");
    let face = lathe::check_path(
        &[
            ch,
            lathe::centre("tail", l.get("standin-live-centre").unwrap(), 200.0, 2.0, 50.0).unwrap(),
        ],
        &[[10.0, 201.0], [0.0, 201.0]],
        nose,
        clear,
        0.1,
    );
    assert_eq!(face.hit.map(|h| h.0), Some("tail/centre".to_string()));
    // a bar the jaws cannot close on
    assert!(lathe::chuck("c", l.get("standin-chuck-200").unwrap(), 10.0, 0.0, 10.0).is_err());
    // a collet: the ER32's range
    assert!(lathe::collet("c", l.get("er32-rego-fix").unwrap(), 25.0, 0.0).is_err());
    assert!(lathe::collet("c", l.get("er32-rego-fix").unwrap(), 20.0, 0.0).is_ok());
}

#[test]
fn vacuum_hold_down_both_ways() {
    let l = Library::builtin();
    let r = l.get("standin-vacuum-300").unwrap();
    let big = Box3 {
        min: [0.0; 3],
        max: [200.0, 150.0, 10.0],
    };
    // 190 × 140 sealed at 80 kPa: 2128 N; μ 0.25 resists 532 N
    let m = mill::hold_down(r, &big, 0.25, 2.0, 200.0).unwrap();
    assert!((m - 0.25 * 80e-3 * 190.0 * 140.0 / 400.0).abs() < 1e-9);
    let small = Box3 {
        min: [0.0; 3],
        max: [40.0, 30.0, 10.0],
    };
    let e = mill::hold_down(r, &small, 0.25, 2.0, 200.0).unwrap_err();
    assert!(e.to_string().contains("at least 20000 mm²"), "{e}");
    assert!(mill::vacuum("vac", r, &big).is_ok());
    // a maker's plate: the vacuum is the pump's, unknown until given
    assert!(mill::hold_down(l.get("witte-r3-600x400").unwrap(), &big, 0.25, 2.0, 200.0).is_err());
}

#[test]
fn plates_zero_point_patterns_and_custom() {
    let l = Library::builtin();
    let stock = Box3 {
        min: [50.0, 50.0, 0.0],
        max: [150.0, 110.0, 30.0],
    };
    // dowels: hole (0, 0) is at (25, 25): clear of the stock; hole (1, 1) at (75, 75) is under it
    let p = mill::fixture_plate(
        "plate",
        l.get("standin-plate-50").unwrap(),
        &stock,
        [0.0, 0.0],
        &[(0, 1)],
    )
    .unwrap();
    assert_eq!(p.bodies.len(), 2);
    assert!(
        mill::fixture_plate(
            "plate",
            l.get("standin-plate-50").unwrap(),
            &stock,
            [0.0, 0.0],
            &[(1, 1)]
        )
        .is_err()
    );
    // a dowel touching the stock's x− side locates it: hole (0, 1) at (25, 75), r 6: stock at x 31
    let s2 = Box3 {
        min: [31.0, 50.0, 0.0],
        max: [131.0, 110.0, 30.0],
    };
    let p2 = mill::fixture_plate(
        "plate",
        l.get("standin-plate-50").unwrap(),
        &s2,
        [0.0, 0.0],
        &[(0, 1)],
    )
    .unwrap();
    assert!(p2.notes.iter().any(|n| n.contains("locates")), "{:?}", p2.notes);
    // a self-centring vise on a riser on a 52 mm zero-point module
    let s = Box3 {
        min: [-30.0, -20.0, 0.0],
        max: [30.0, 20.0, 25.0],
    };
    let mut v = mill::vise(
        "vise",
        l.get("standin-centring-vise-80").unwrap(),
        &s,
        0,
        4.0,
        Some(l.get("standin-parallels").unwrap()),
        None,
    )
    .unwrap();
    let z = mill::zero_point(
        "zp",
        l.get("standin-zero-point-52").unwrap(),
        Some(l.get("standin-riser-100").unwrap()),
        &mut v,
        [0.0, 0.0],
        &[(0, 0)],
    )
    .unwrap();
    assert!(matches!(v.mount, forge_workholding::Mount::On { .. }));
    let w = Workholding { placed: vec![v, z] };
    assert!(!w.faces(1e-9).is_empty() && w.prims().len() >= 6);
    // a real product needing an unpublished size refuses: the Kurt DX6's jaw thickness
    // a real vise whose jaw thickness and height are not published: a conservative envelope
    // from its published overall sizes, which holds the exact bodies for any measured values
    let s6 = Box3 {
        min: [0.0; 3],
        max: [100.0, 60.0, 44.07],
    };
    let env = mill::vise("v", l.get("kurt-dx6").unwrap(), &s6, 0, 44.07, None, None).unwrap();
    assert!(
        env.notes.iter().any(|n| n.contains("conservative envelope")),
        "{:?}",
        env.notes
    );
    for (t, h) in [(15.0, 117.1), (22.0, 111.1), (190.0, 100.0)] {
        let m = l
            .get("kurt-dx6")
            .unwrap()
            .with("jaw_thickness", t)
            .unwrap()
            .with("height", h)
            .unwrap();
        let exact = mill::vise("v", &m, &s6, 0, 44.07, None, None).unwrap();
        for b in &exact.bodies {
            let (lo, hi) = b.bounds();
            let inside = env.bodies.iter().any(|e| {
                let (a, c) = e.bounds();
                (0..3).all(|k| lo[k] >= a[k] - 1e-9 && hi[k] <= c[k] + 1e-9)
            });
            assert!(inside, "t {t} h {h}: {} {lo:?} {hi:?}", b.name);
        }
    }
    // a value that is merely missing (not listed unknown) is still refused
    let e = mill::vise(
        "v",
        l.get("tormach-vise-35285").unwrap(),
        &s6,
        0,
        30.0,
        None,
        None,
    )
    .unwrap_err();
    assert!(e.to_string().contains("body_width"), "{e}");
    // two vises side by side: a pattern
    let one = mill::vise(
        "v",
        l.get("standin-vise-150").unwrap(),
        &Box3 {
            min: [0.0; 3],
            max: [60.0, 40.0, 50.0],
        },
        0,
        50.0,
        None,
        None,
    )
    .unwrap();
    let two = mill::pattern(&one, &[Pose::ID, Pose::shift([0.0, 150.0, 0.0])]);
    assert_eq!(two[1].name, "v[2]");
    assert!(two[1].held[0].0[1] == 150.0);
    // a custom angle plate turned a quarter
    let c = mill::custom(
        "ap",
        l.get("standin-angle-plate").unwrap(),
        &Pose::quarter_z(1, [0.0; 3]),
        vec![],
    )
    .unwrap();
    assert_eq!(c.bodies.len(), 2);
    // an edge clamp against the stock's side
    let e = mill::side_clamp(
        "mb",
        l.get("standin-edge-clamp-m8").unwrap(),
        &stock,
        0.0,
        [50.0, 80.0],
        [1.0, 0.0],
    )
    .unwrap();
    assert_eq!(e.bodies[0].bounds().1[0], 50.0);
}
