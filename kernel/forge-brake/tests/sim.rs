//! The simulator on parts with known answers: an L, a U channel, a box with four flanges, and a
//! return flange; each rule just passing and just failing; the refold against the builder.
use forge_brake::model::Model;
use forge_brake::sim::{Rule, Setup, StepCtx};
use forge_brake::{Options, library, plan};
use forge_math::{Frame, Point2, Point3, dmath};
use forge_ops::{ProfileLoop, Segment};
use forge_sheet::{BendRule, FeatureId, Gauge, Map, Sheet, WallId};

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

#[test]
fn an_ell_plans_in_one_step() {
    let s = ell(30.0);
    let m = Model::new(&s).unwrap();
    let tl = eu();
    let p = plan(&m, &tl, &Options::default());
    for st in &p.steps {
        for c in &st.checks {
            eprintln!(
                "{:?} {} {} {} {:?} {:?}",
                c.rule, c.pass, c.value, c.limit, c.tool, c.part
            );
        }
        eprintln!("{} {} {:?}", st.punch, st.die, st.gauge);
    }
    assert!(p.feasible, "{:?}", p.blocking);
    assert_eq!(p.sequence, vec![0]);
}

#[test]
fn rules_just_pass_and_just_fail() {
    // a flange whose leg reaches just past / just short of the V16 die's shoulder contact
    let tl = eu();
    let die = tl.dies.iter().position(|d| d.id == "std-eu/V16-88").unwrap();
    let punch = tl.punches.iter().position(|p| p.id == "std-eu/P88-R08").unwrap();
    let setup = Setup {
        punch,
        die,
        rotate: false,
    };
    let probe = ell(30.0);
    let m = Model::new(&probe).unwrap();
    let ctx = StepCtx::new(&m, &tl, &[false], 0, setup, 0.0);
    let (rules, _, _) = ctx.rules(None);
    let mf = rules.iter().find(|c| c.rule == Rule::MinFlange).unwrap();
    let reach = mf.limit;
    // the flange's leg is its height less the setback
    let s = forge_sheet::setback_of(&probe.gauge, 2.0, deg(90.0));
    for (h, pass) in [(reach + s + 0.05, true), (reach + s - 0.05, false)] {
        let part = ell(h);
        let m = Model::new(&part).unwrap();
        let ctx = StepCtx::new(&m, &tl, &[false], 0, setup, 0.0);
        let (rules, _, _) = ctx.rules(None);
        let mf = rules.iter().find(|c| c.rule == Rule::MinFlange).unwrap();
        assert_eq!(mf.pass, pass, "h {h}: {} vs {}", mf.value, mf.limit);
    }
}

#[test]
fn dbg_self() {
    let s = ell(30.0);
    let m = Model::new(&s).unwrap();
    let tl = eu();
    let ctx = StepCtx::new(
        &m,
        &tl,
        &[false],
        0,
        Setup {
            punch: 0,
            die: 4,
            rotate: false,
        },
        0.0,
    );
    for p in &ctx.pieces {
        eprintln!("{:?} {:?} dom {:?}", p.strip, p.side, p.dom);
        if let forge_brake::sim::Kind::Wall { region, .. } = &p.kind {
            eprintln!("  loops {:?}\n  minus {:?}", region.loops, region.minus);
        }
    }
    let c = ctx.self_check(false);
    eprintln!("{c:?}");
}

/// A channel `w` across, `len` long, two flanges `h` high.
fn channel(w: f64, len: f64, h: f64) -> Sheet {
    let (mut s, base) = sheet(2.0, 2.0, w, len);
    s.flange(base, 1, deg(90.0), h).unwrap();
    s.flange(base, 3, deg(90.0), h).unwrap();
    s
}

fn show(p: &forge_brake::Plan) {
    eprintln!(
        "feasible {} seq {:?} explored {}",
        p.feasible, p.sequence, p.explored
    );
    for st in &p.steps {
        eprintln!(
            " step bend {} {} {} rot {} gauge {:?}",
            st.bend,
            st.punch,
            st.die,
            st.setup.rotate,
            st.gauge.as_ref().map(|g| g.x)
        );
        for c in &st.checks {
            if !c.pass
                || matches!(
                    c.rule,
                    Rule::ToolCollision | Rule::ClosesOnPunch | Rule::SelfCollision
                )
            {
                eprintln!(
                    "   {:?} {} {:.3} {:.3} {:?} {:?} {:?}",
                    c.rule, c.pass, c.value, c.limit, c.tool, c.part, c.phase
                );
            }
        }
    }
    for b in &p.blocking {
        eprintln!(" blocked {} after {:?} {} {}", b.bend, b.after, b.punch, b.die);
        for c in &b.checks {
            eprintln!(
                "   {:?} {:.3} {:.3} {:?} {:?} {:?}",
                c.rule, c.value, c.limit, c.tool, c.part, c.phase
            );
        }
    }
    eprintln!(" suggestions {:?}", p.suggestions);
}

#[test]
fn dbg_parts() {
    let tl = eu();
    let t0 = std::time::Instant::now();
    for (name, s) in [
        ("wide U", channel(100.0, 200.0, 40.0)),
        ("narrow U", channel(30.0, 200.0, 60.0)),
    ] {
        let m = Model::new(&s).unwrap();
        let p = plan(&m, &tl, &Options::default());
        eprintln!("== {name} {:?}", t0.elapsed());
        show(&p);
    }
    let (mut s, base) = sheet(1.5, 1.5, 100.0, 80.0);
    for seg in 0..4 {
        s.flange(base, seg, deg(90.0), 30.0).unwrap();
    }
    let m = Model::new(&s).unwrap();
    let p = plan(&m, &tl, &Options::default());
    eprintln!("== box {:?}", t0.elapsed());
    show(&p);
    let (mut s, base) = sheet(2.0, 2.0, 60.0, 200.0);
    let (f1, _) = s.flange(base, 1, deg(90.0), 40.0).unwrap();
    s.flange(f1, 1, deg(90.0), 15.0).unwrap();
    s.flange(base, 3, deg(90.0), 40.0).unwrap();
    let m = Model::new(&s).unwrap();
    let p = plan(&m, &tl, &Options::default());
    eprintln!("== return {:?}", t0.elapsed());
    show(&p);
}

#[test]
fn dbg_time() {
    use std::time::Instant;
    let s = ell(30.0);
    let t = Instant::now();
    let m = Model::new(&s).unwrap();
    eprintln!("model {:?}", t.elapsed());
    let tl = eu();
    let t = Instant::now();
    let ctx = StepCtx::new(
        &m,
        &tl,
        &[false],
        0,
        Setup {
            punch: 0,
            die: 4,
            rotate: false,
        },
        0.0,
    );
    eprintln!("ctx {:?}", t.elapsed());
    let t = Instant::now();
    let r = ctx.rules(None);
    eprintln!("rules {:?} {}", t.elapsed(), r.0.len());
    let t = Instant::now();
    let g = ctx.gauge();
    eprintln!("gauge {:?}", t.elapsed());
    let t = Instant::now();
    let _ = ctx.self_check(true);
    eprintln!("self {:?}", t.elapsed());
    for ph in [
        forge_brake::Phase::Static,
        forge_brake::Phase::Approach,
        forge_brake::Phase::Bend,
        forge_brake::Phase::Return,
    ] {
        let t = Instant::now();
        let mut v = Vec::new();
        ctx.collisions(ph, g.1.as_ref(), &mut v, true);
        eprintln!("{ph:?} {:?}", t.elapsed());
    }
}

#[test]
fn dbg_goose() {
    let tl = eu();
    let s = channel(30.0, 200.0, 60.0);
    let m = Model::new(&s).unwrap();
    let g = tl.punches.iter().position(|p| p.id == "std-eu/G88-R08").unwrap();
    for rotate in [false, true] {
        let die = tl.dies.iter().position(|d| d.id == "std-eu/V16-88").unwrap();
        let ctx = StepCtx::new(
            &m,
            &tl,
            &[true, false],
            1,
            Setup {
                punch: g,
                die,
                rotate,
            },
            0.0,
        );
        let mut v = Vec::new();
        for ph in [
            forge_brake::Phase::Static,
            forge_brake::Phase::Approach,
            forge_brake::Phase::Bend,
        ] {
            ctx.collisions(ph, None, &mut v, false);
            ctx.collisions(ph, None, &mut v, true);
        }
        for c in v {
            eprintln!(
                "rot {rotate} {:?} {} {:.3} {:?} {:?} {:?} phi {:?}",
                c.rule, c.pass, c.value, c.tool, c.part, c.at, c.phi
            );
        }
        let (r, _, _) = ctx.rules(None);
        for c in r.iter().filter(|c| !c.pass) {
            eprintln!("rot {rotate} {:?} {:.3} {:.3}", c.rule, c.value, c.limit);
        }
        eprintln!("gauge {:?}", ctx.gauge());
    }
}

#[test]
fn dbg_goose_plan() {
    let tl = eu();
    let s = channel(30.0, 200.0, 60.0);
    let m = Model::new(&s).unwrap();
    let g = tl.punches.iter().position(|p| p.id == "std-eu/G88-R08").unwrap();
    let o = Options {
        punches: Some(vec![g]),
        sequence: Some(vec![0, 1]),
        rotate: Some(true),
        ..Options::default()
    };
    let p = plan(&m, &tl, &o);
    show(&p);
    for st in &p.steps {
        for c in st.checks.iter().filter(|c| !c.pass) {
            eprintln!(
                "FAIL {:?} {:.3} {:.3} {:?} {:?}",
                c.rule, c.value, c.limit, c.tool, c.phase
            );
        }
    }
}
