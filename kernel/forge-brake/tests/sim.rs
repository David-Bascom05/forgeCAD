//! The simulator on parts with known answers: an L, a U channel, a box with four flanges, and a
//! return flange; each rule just passing and just failing; the refold against the builder.
use forge_brake::model::Model;
use forge_brake::sim::{Check, Phase, Rule, Setup, StepCtx};
use forge_brake::{Options, PunchKind, Suggestion, library, plan};
use forge_math::{Frame, Point2, Point3, dmath};
use forge_ops::{ProfileLoop, Segment};
use forge_sheet::{BendRule, FeatureId, Gauge, Map, Sheet, Strip, WallId};

fn rect(w: f64, h: f64) -> ProfileLoop {
    let pts = [(0.0, 0.0), (w, 0.0), (w, h), (0.0, h)];
    ProfileLoop::new(
        (0..4)
            .map(|i| {
                let (a, b) = (pts[i], pts[(i + 1) % 4]);
                (
                    Segment::Line {
                        a: Point2::new(a.0, a.1),
                        b: Point2::new(b.0, b.1),
                    },
                    i as u64,
                )
            })
            .collect(),
    )
}

fn deg(d: f64) -> f64 {
    d * dmath::PI / 180.0
}

fn sheet(t: f64, r: f64, w: f64, h: f64) -> (Sheet, WallId) {
    Sheet::face(
        FeatureId(1),
        Gauge {
            thickness: t,
            radius: r,
            rule: BendRule::K(0.4),
        },
        Frame::WORLD,
        &[rect(w, h)],
    )
    .unwrap()
}

/// An L: a 120 x 60 base, one flange of `h` on its right edge.
fn ell(h: f64) -> Sheet {
    let (mut s, base) = sheet(2.0, 2.0, 120.0, 60.0);
    s.flange(base, 1, deg(90.0), h).unwrap();
    s
}

#[test]
fn full_refold_is_the_builders_part() {
    let mut s = ell(30.0);
    let (w1, _) = s.flange(WallId(1), 2, deg(-60.0), 20.0).unwrap();
    let _ = w1;
    let m = Model::new(&s).unwrap();
    let built = forge_sheet::build_with(&s, Map::Folded).unwrap();
    let phi: Vec<f64> = m.bends.iter().map(|b| b.theta).collect();
    let f = m.frames(&phi);
    for (wi, w) in m.walls.iter().enumerate() {
        let place = built.layout.walls.iter().find(|p| p.wall == w.id).unwrap();
        for p in [(0.0, 0.0, 0.0), (10.0, 5.0, -2.0), (3.0, 40.0, 0.0)] {
            let mine = f[wi].apply(forge_brake::geo::V3::new(p.0, p.1, p.2));
            let theirs = place.frame.point_to_world(Point3::new(p.0, p.1, p.2));
            let d = ((mine.x - theirs.x).powi(2) + (mine.y - theirs.y).powi(2) + (mine.z - theirs.z).powi(2))
                .sqrt();
            assert!(d < 1e-9, "wall {wi}: {d}");
        }
    }
    // and a partial state leaves the neutral layer's length: the zone's far tangent line moves
    // on a circle of the developed length about nothing; check BA is kept along the neutral layer
    let b = &m.bends[0];
    let k = b.rn - b.r; // K t, from the inside
    let w_n = if b.sigma > 0.0 { -k } else { -2.0 + k };
    for phi in [0.0, 0.3, 1.0, b.theta] {
        let n = 2000;
        let mut len = 0.0;
        let mut prev = b.fold(0.0, 0.0, w_n, phi);
        for i in 1..=n {
            let q = b.fold(b.ba * i as f64 / n as f64, 0.0, w_n, phi);
            len += q.minus(prev).norm();
            prev = q;
        }
        assert!((len - b.ba).abs() < 1e-6, "phi {phi}: {len} vs {}", b.ba);
    }
}

fn eu() -> forge_brake::Tooling {
    library::std_eu()
}

fn straight(tl: &forge_brake::Tooling) -> Vec<usize> {
    (0..tl.punches.len())
        .filter(|&k| tl.punches[k].kind == PunchKind::Straight)
        .collect()
}

fn kind_of(tl: &forge_brake::Tooling, id: &str) -> PunchKind {
    tl.punches.iter().find(|p| p.id == id).unwrap().kind
}

/// A channel `w` across, `len` long, two flanges `h` high.
fn channel(w: f64, len: f64, h: f64) -> Sheet {
    let (mut s, base) = sheet(2.0, 2.0, w, len);
    s.flange(base, 1, deg(90.0), h).unwrap();
    s.flange(base, 3, deg(90.0), h).unwrap();
    s
}

/// A box `w` x `d` with four flanges `h` high.
fn boxed(w: f64, d: f64, h: f64) -> Sheet {
    let (mut s, base) = sheet(1.5, 1.5, w, d);
    for seg in 0..4 {
        s.flange(base, seg, deg(90.0), h).unwrap();
    }
    s
}

#[test]
fn an_ell_plans_in_one_step() {
    let m = Model::new(&ell(30.0)).unwrap();
    let p = plan(&m, &eu(), &Options::default());
    assert!(p.feasible, "{:?}", p.blocking);
    assert_eq!(p.sequence, vec![0]);
    let st = &p.steps[0];
    assert!(st.pass());
    // every rule was checked, and the backgauge stops on the flange's tip
    for r in [
        Rule::DieAngle,
        Rule::PunchAngle,
        Rule::PunchRadius,
        Rule::VTooNarrow,
        Rule::MinFlange,
        Rule::VDepth,
        Rule::Stroke,
        Rule::OpenHeight,
        Rule::Segments,
        Rule::BedLength,
        Rule::BackgaugeReach,
        Rule::SelfCollision,
        Rule::ToolCollision,
        Rule::Withdraw,
    ] {
        assert!(st.checks.iter().any(|c| c.rule == r), "{r:?}");
    }
    let g = st.gauge.as_ref().unwrap();
    assert_eq!(g.part.segment, Some(1));
    // the flat leg: the zone's half allowance plus the flange less its setback
    let b = &m.bends[0];
    assert!((g.x - (0.5 * b.ba + 30.0 - 2.0)).abs() < 1e-9, "{}", g.x);
}

#[test]
fn u_channels() {
    let tl = eu();
    // wide: straight punches throughout
    let m = Model::new(&channel(100.0, 200.0, 40.0)).unwrap();
    let p = plan(&m, &tl, &Options::default());
    assert!(p.feasible);
    assert!(
        p.steps
            .iter()
            .all(|s| kind_of(&tl, &s.punch) == PunchKind::Straight)
    );
    // narrow and tall: the first flange closes on a straight punch; the gooseneck takes it
    let m = Model::new(&channel(30.0, 200.0, 60.0)).unwrap();
    let p = plan(
        &m,
        &tl,
        &Options {
            punches: Some(straight(&tl)),
            ..Options::default()
        },
    );
    assert!(!p.feasible);
    assert!(
        p.blocking
            .iter()
            .all(|b| b.checks.iter().any(|c| c.rule == Rule::ClosesOnPunch)),
        "{:?}",
        p.blocking
    );
    assert!(p.suggestions.iter().any(|s| matches!(
        s,
        Suggestion::NeedTool {
            rule: Rule::ClosesOnPunch,
            ..
        }
    )));
    let p = plan(&m, &tl, &Options::default());
    assert!(p.feasible, "{:?}", p.blocking);
    assert_eq!(kind_of(&tl, &p.steps[1].punch), PunchKind::Gooseneck);
    // the second flange's throat faces the first: turned end for end
    assert!(p.steps[1].setup.rotate);
}

#[test]
fn a_box_with_four_flanges() {
    let tl = eu();
    let m = Model::new(&boxed(120.0, 60.0, 25.0)).unwrap();
    let p = plan(&m, &tl, &Options::default());
    assert!(p.feasible, "{:?}", p.blocking);
    assert_eq!(p.steps.len(), 4);
    // a bend between two standing flanges takes a punch that fits between them
    for st in &p.steps[1..] {
        let len = m.bends[st.bend].len;
        let pl = st.punch_length.unwrap();
        assert!(
            pl <= len - 2.0 * tl.clearance + 1e-9 && pl >= len - 6.0 - 1e-9,
            "{pl} for {len}"
        );
    }
    // too deep for its width: the last long side closes on every punch
    let m = Model::new(&boxed(120.0, 40.0, 30.0)).unwrap();
    let p = plan(&m, &tl, &Options::default());
    assert!(!p.feasible);
    assert!(!p.blocking.is_empty());
    assert!(
        p.blocking
            .iter()
            .any(|b| b.checks.iter().any(|c| c.rule == Rule::ClosesOnPunch)),
        "{:?}",
        p.blocking
    );
}

#[test]
fn return_flanges() {
    let tl = eu();
    // one return lip: lip first, its flange, then the other flange over a gooseneck
    let (mut s, base) = sheet(2.0, 2.0, 60.0, 200.0);
    let (f1, _) = s.flange(base, 1, deg(90.0), 40.0).unwrap();
    s.flange(f1, 1, deg(90.0), 15.0).unwrap();
    s.flange(base, 3, deg(90.0), 40.0).unwrap();
    let m = Model::new(&s).unwrap();
    let p = plan(&m, &tl, &Options::default());
    assert!(p.feasible, "{:?}", p.blocking);
    let at = |k: usize| p.sequence.iter().position(|x| *x == k).unwrap();
    assert!(at(1) < at(0), "the lip before its flange: {:?}", p.sequence);
    let last = p.steps.last().unwrap();
    assert_eq!(kind_of(&tl, &last.punch), PunchKind::Gooseneck);
    // the lip closes on a straight punch: it is the colliding piece
    let p = plan(
        &m,
        &tl,
        &Options {
            punches: Some(straight(&tl)),
            sequence: Some(p.sequence.clone()),
            ..Options::default()
        },
    );
    assert!(!p.feasible);
    let b = p.blocking.last().unwrap();
    let c = b
        .checks
        .iter()
        .find(|c| c.rule == Rule::ClosesOnPunch)
        .expect("closes on the punch");
    let lip = m.walls[m.bends[1].b].id;
    let flange = m.walls[m.bends[0].b].id;
    let hit = match c.part.unwrap().strip {
        Strip::Wall(w) => w,
        Strip::Bend(_) => flange,
    };
    assert!(hit == lip || hit == flange, "{:?}", c.part);
    assert!(c.value < tl.clearance && c.at.is_some() && c.phi.is_some());
}

#[test]
fn the_search_is_deterministic() {
    let tl = eu();
    let m = Model::new(&channel(30.0, 200.0, 60.0)).unwrap();
    let a = plan(&m, &tl, &Options::default());
    let b = plan(&m, &tl, &Options::default());
    assert_eq!(a, b);
}

#[test]
fn a_given_sequence_is_simulated_as_given() {
    let tl = eu();
    let m = Model::new(&channel(100.0, 200.0, 40.0)).unwrap();
    let p = plan(
        &m,
        &tl,
        &Options {
            sequence: Some(vec![1, 0]),
            ..Options::default()
        },
    );
    assert!(p.feasible);
    assert_eq!(p.sequence, vec![1, 0]);
    assert_eq!(p.steps.iter().map(|s| s.bend).collect::<Vec<_>>(), vec![1, 0]);
}

/// The check of `rule` for bend 0 of `part` with `tl`.
fn rule_of(
    part: &Sheet,
    tl: &forge_brake::Tooling,
    setup: Setup,
    ob: f64,
    rule: Rule,
    uts: Option<f64>,
) -> Check {
    let m = Model::new(part).unwrap();
    let ctx = StepCtx::new(&m, tl, &vec![false; m.bends.len()], 0, setup, ob);
    let (rules, _, _) = ctx.rules(uts);
    let mut all = rules;
    all.push(ctx.gauge().0);
    all.into_iter()
        .find(|c| c.rule == rule)
        .unwrap_or_else(|| panic!("{rule:?}"))
}

fn setup(tl: &forge_brake::Tooling, punch: &str, die: &str) -> Setup {
    Setup {
        punch: tl.punches.iter().position(|p| p.id == punch).unwrap(),
        die: tl.dies.iter().position(|d| d.id == die).unwrap(),
        rotate: false,
    }
}

#[test]
fn rules_just_pass_and_just_fail() {
    let tl = eu();
    let su = setup(&tl, "std-eu/P88-R08", "std-eu/V16-88");
    let part = ell(30.0);
    let e = 1e-3;
    // the minimum flange: the leg is the height less the setback
    let reach = rule_of(&part, &tl, su, 0.0, Rule::MinFlange, None).limit;
    let s = forge_sheet::setback_of(&part.gauge, 2.0, deg(90.0));
    for (h, pass) in [(reach + s + 0.05, true), (reach + s - 0.05, false)] {
        assert_eq!(
            rule_of(&ell(h), &tl, su, 0.0, Rule::MinFlange, None).pass,
            pass,
            "min_flange at {h}"
        );
    }
    // the die's and the punch's angles against the overbent included angle (88 degrees each)
    for (ob, pass) in [(deg(2.0) - e, true), (deg(2.0) + e, false)] {
        assert_eq!(rule_of(&part, &tl, su, ob, Rule::DieAngle, None).pass, pass);
        assert_eq!(rule_of(&part, &tl, su, ob, Rule::PunchAngle, None).pass, pass);
    }
    // the tip radius (0.8) against the inside radius
    for (r, pass) in [(0.8 + e, true), (0.8 - e, false)] {
        let (mut s2, base) = sheet(2.0, r, 120.0, 60.0);
        s2.flange(base, 1, deg(90.0), 30.0).unwrap();
        assert_eq!(
            rule_of(&s2, &tl, su, 0.0, Rule::PunchRadius, None).pass,
            pass,
            "r {r}"
        );
    }
    // the V too narrow for a big radius: bisect the radius on a V6
    let v6 = setup(&tl, "std-eu/P88-R08", "std-eu/V6-88");
    let narrow = |r: f64| {
        let (mut s2, base) = sheet(2.0, r, 120.0, 60.0);
        s2.flange(base, 1, deg(90.0), 30.0).unwrap();
        rule_of(&s2, &tl, v6, 0.0, Rule::VTooNarrow, None)
    };
    let (mut lo, mut hi) = (0.5, 10.0);
    assert!(narrow(lo).pass && !narrow(hi).pass);
    for _ in 0..30 {
        let mid = 0.5 * (lo + hi);
        if narrow(mid).pass { lo = mid } else { hi = mid }
    }
    assert!(narrow(lo).value >= 0.0 && narrow(hi).value < 0.0 && hi - lo < 1e-6);
    // the press: stroke, open height, housing gap; the force with a strength
    let travel = |t: &forge_brake::Tooling| rule_of(&part, t, su, 0.0, Rule::Stroke, None);
    let need = travel(&tl).value;
    for (k, pass) in [(need + e, true), (need - e, false)] {
        let mut t = tl.clone();
        t.press.stroke = k;
        assert_eq!(travel(&t).pass, pass, "stroke {k}");
    }
    let oh = rule_of(&part, &tl, su, 0.0, Rule::OpenHeight, None);
    for (d, pass) in [(e, true), (-e, false)] {
        let mut t = tl.clone();
        t.press.open_height -= oh.value - oh.limit - d;
        assert_eq!(rule_of(&part, &t, su, 0.0, Rule::OpenHeight, None).pass, pass);
    }
    let len = rule_of(&part, &tl, su, 0.0, Rule::BedLength, None).value;
    assert!((len - 60.0).abs() < 1e-6, "{len}");
    for (g, pass) in [(len + e, true), (len - e, false)] {
        let mut t = tl.clone();
        t.press.housing_gap = g;
        assert_eq!(rule_of(&part, &t, su, 0.0, Rule::BedLength, None).pass, pass);
    }
    // the force: against the press alone (the tools unrated), then a tool's kN per metre
    let mut unrated = tl.clone();
    for p in &mut unrated.punches {
        p.max_load = None;
    }
    for d in &mut unrated.dies {
        d.max_load = None;
    }
    let f = rule_of(&part, &unrated, su, 0.0, Rule::Tonnage, Some(400.0));
    let want = 1.42 * 400.0 * 4.0 / 16.0 * 60.0 / 1000.0;
    assert!((f.value - want).abs() < 1e-9 && f.pass, "{f:?}");
    let per_m = rule_of(&part, &tl, su, 0.0, Rule::Tonnage, Some(400.0));
    assert!(
        (per_m.value - want / 0.06).abs() < 1e-9 && per_m.tool.is_some(),
        "{per_m:?}"
    );
    for (k, pass) in [(want + e, true), (want - e, false)] {
        let mut t = unrated.clone();
        t.press.max_force = Some(k);
        assert_eq!(rule_of(&part, &t, su, 0.0, Rule::Tonnage, Some(400.0)).pass, pass);
    }
    // segments: one piece just long enough for the bend less its ends (3 each), or not
    for (l, pass) in [(54.0, true), (53.9, false)] {
        let mut t = tl.clone();
        t.punches[su.punch].segments = vec![forge_brake::Segment { length: l, count: 1 }];
        assert_eq!(
            rule_of(&part, &t, su, 0.0, Rule::Segments, None).pass,
            pass,
            "segment {l}"
        );
    }
    // the backgauge's reach
    let x = rule_of(&part, &tl, su, 0.0, Rule::BackgaugeReach, None).value;
    for (hi, pass) in [(x + e, true), (x - e, false)] {
        let mut t = tl.clone();
        t.press.backgauge.x.1 = hi;
        assert_eq!(rule_of(&part, &t, su, 0.0, Rule::BackgaugeReach, None).pass, pass);
    }
}

#[test]
fn clearances_just_pass_and_just_fail() {
    // the measured least distance against the clearance asked for: certified below, met above
    let tl = eu();
    let su = setup(&tl, "std-eu/P88-R08", "std-eu/V16-88");
    let m = Model::new(&ell(30.0)).unwrap();
    let measure = |t: &forge_brake::Tooling| {
        let ctx = StepCtx::new(&m, t, &[false], 0, su, 0.0);
        let mut v = Vec::new();
        ctx.collisions(Phase::Bend, None, &mut v, false);
        v.into_iter().find(|c| c.rule == Rule::ToolCollision).unwrap()
    };
    let c = measure(&tl);
    assert!(c.pass);
    let (lower, upper) = (c.bound.unwrap(), c.value);
    assert!(tl.clearance - 0.125 <= lower && lower <= upper, "{lower} {upper}");
    let mut t = tl.clone();
    t.clearance = lower - 0.01;
    assert!(measure(&t).pass);
    t.clearance = upper + 0.01;
    let c = measure(&t);
    assert!(!c.pass && c.value < t.clearance && c.certified, "{c:?}");
    // the part against itself: the box's last flange meets its neighbours at the corners
    let tl = eu();
    let m = Model::new(&boxed(120.0, 60.0, 25.0)).unwrap();
    let selfc = |t: &forge_brake::Tooling| {
        let ctx = StepCtx::new(
            &m,
            t,
            &[true, true, true, false],
            3,
            Setup {
                punch: 0,
                die: 0,
                rotate: false,
            },
            0.0,
        );
        ctx.self_check(false)
    };
    let c = selfc(&tl);
    assert!(c.pass, "{c:?}");
    let (lower, upper) = (c.bound.unwrap(), c.value);
    let mut t = tl.clone();
    t.self_clearance = (lower - 0.03).max(0.0);
    assert!(selfc(&t).pass);
    t.self_clearance = upper + 0.03;
    let c = selfc(&t);
    assert!(!c.pass && c.part.is_some() && c.other.is_some(), "{c:?}");
}
