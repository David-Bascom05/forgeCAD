//! The lookahead time (lookahead.rs: grbl v1.1h's planner restated) against closed forms on the
//! 3-axis mill (X and Y at 3000 mm/s²): a lone move is its trapezoid, a split straight move is one
//! trapezoid, a corner's speed is the junction deviation's, a short window slows a run of short
//! moves to what it can stop within, and t_min ≤ t_la ≤ t_stop on lines.
use forge_machine::gcode::Control;
use forge_machine::library::builtin_named;
use forge_machine::lookahead::{self, LookAhead};
use forge_machine::sim::{self, Cut, Setup, SimTool, Timeline};

fn run(prog: &str) -> (forge_machine::MachineDef, Timeline) {
    let m = builtin_named("generic-vmc3").unwrap();
    let tl = {
        let mut s = Setup::new(&m, Control::for_dialect(&m, "fanuc", 1).unwrap()).unwrap();
        s.wcs = vec![(54, vec![0.0, 0.0, -300.0])];
        s.h = vec![(1, 100.0)];
        s.tools = vec![SimTool {
            t: 1,
            name: "e10".into(),
            tip: [0.0, 0.0, -100.0],
            body: vec![(5.0, 0.0, 25.0), (12.0, 25.0, 80.0)],
            cut: Cut::Mill { r: 5.0, corner: 0.0 },
        }];
        s.start_tool = Some(1);
        sim::simulate(&format!("G43 H1\nG0 X0 Y0 Z10.\nG4 P100\n{prog}"), &s).unwrap()
    };
    (m, tl)
}

/// The la time of the segments on program lines >= 4 (after the dwell: from rest).
fn after(tl: &Timeline) -> f64 {
    tl.segments
        .iter()
        .zip(&tl.seg_la)
        .filter(|(g, _)| g.line >= 4)
        .map(|(_, t)| t)
        .sum()
}

/// Closed form: length l from v0 to v1, cruise vn, acceleration a.
fn trapezoid(l: f64, v0: f64, v1: f64, vn: f64, a: f64) -> f64 {
    let (da, dd) = ((vn * vn - v0 * v0) / (2.0 * a), (vn * vn - v1 * v1) / (2.0 * a));
    if da + dd <= l {
        (vn - v0) / a + (vn - v1) / a + (l - da - dd) / vn
    } else {
        let vp = ((2.0 * a * l + v0 * v0 + v1 * v1) / 2.0).sqrt();
        (vp - v0) / a + (vp - v1) / a
    }
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-9 * b.abs().max(1.0)
}

#[test]
fn a_lone_move_is_its_trapezoid_and_a_split_one_is_one_trapezoid() {
    // 100 mm at 6000 mm/min (100 mm/s), a = 3000: 1 + 1/30 s; its t_stop is the same trapezoid
    let (_, tl) = run("G1 X100. F6000.\n");
    let want = trapezoid(100.0, 0.0, 0.0, 100.0, 3000.0);
    assert!(close(after(&tl), want), "{} vs {want}", after(&tl));
    let g = tl.segments.iter().position(|g| g.line == 4).unwrap();
    assert!(close(tl.seg_la[g], tl.segments[g].t_stop));
    // in two collinear halves: no junction limit, one trapezoid over both
    let (_, tl) = run("G1 X50. F6000.\nX100.\n");
    assert!(close(after(&tl), want), "{} vs {want}", after(&tl));
}

#[test]
fn a_corner_is_taken_at_the_junction_deviations_speed() {
    let (_, tl) = run("G1 X100. F6000.\nY100.\n");
    // 90°: cos θ = 0, sin(θ/2) = √½; along (−1, 1)/√2 the limit is 3000 √2; δ = 0.01
    let s = 0.5f64.sqrt();
    let aj = 3000.0 * 2f64.sqrt();
    let vj = (aj * 0.01 * s / (1.0 - s)).sqrt();
    let want = 2.0 * trapezoid(100.0, 0.0, vj, 100.0, 3000.0);
    assert!(close(after(&tl), want), "{} vs {want}", after(&tl));
    // a larger deviation: a faster corner
    let la = LookAhead {
        junction_deviation: 0.1,
        ..Default::default()
    };
    let (m, tl2) = run("G1 X100. F6000.\nY100.\n");
    let (per, _) = lookahead::plan(&m, &tl2, &la);
    let t2: f64 = tl2
        .segments
        .iter()
        .zip(&per)
        .filter(|(g, _)| g.line >= 4)
        .map(|(_, t)| t)
        .sum();
    assert!(t2 < after(&tl));
}

#[test]
fn a_short_window_slows_short_moves_to_what_it_can_stop_within() {
    // 100 blocks of 1 mm along X at 100 mm/s with a = 30 (scale 0.01): with a window as long as
    // the run, one triangle over 100 mm; with one block, a stop after each; with grbl's 15, the
    // block being run can only reach what stops within the 14 behind it
    let mut p = String::from("G1 X1. F6000.\n");
    for i in 2..=100 {
        p.push_str(&format!("X{i}.\n"));
    }
    let (m, tl) = run(&p);
    let t = |blocks: usize| {
        let la = LookAhead {
            accel_scale: 0.01,
            blocks,
            ..Default::default()
        };
        let (per, _) = lookahead::plan(&m, &tl, &la);
        tl.segments
            .iter()
            .zip(&per)
            .filter(|(g, _)| g.line >= 4)
            .map(|(_, t)| t)
            .sum::<f64>()
    };
    let (all, one, grbl) = (t(1000), t(1), t(15));
    assert!(close(all, trapezoid(100.0, 0.0, 0.0, 100.0, 30.0)), "{all}");
    assert!(close(one, 100.0 * trapezoid(1.0, 0.0, 0.0, 100.0, 30.0)), "{one}");
    assert!(all < grbl && grbl < one, "{all} {grbl} {one}");
    // the cruise with 15: at most √(2·30·14) mm/s, so at least 100 mm at that, less the ends
    let vmax = (2.0 * 30.0 * 14.0f64).sqrt();
    assert!(grbl >= (100.0 - 2.0 * 14.0) / vmax, "{grbl}");
}

#[test]
fn bounds_and_the_acceleration_scale() {
    // a pocket's zig-zag with arcs
    let mut p = String::from("G1 Z-2. F500.\n");
    let mut y = -40.0;
    let mut dir = 1.0;
    while y <= 40.0 {
        p.push_str(&format!(
            "G1 X{:.1} Y{y:.1} F3000.\nG1 Y{:.1}\n",
            40.0 * dir,
            y + 6.0
        ));
        y += 6.0;
        dir = -dir;
    }
    p.push_str("G0 X30. Y0\nG2 X30. Y0 I-30. J0 F2000.\nG0 Z10.\n");
    let (m, tl) = run(&p);
    assert!(tl.t_min <= tl.t_la * (1.0 + 1e-12), "{} {}", tl.t_min, tl.t_la);
    // the cycle-time estimate (jerk-limited, the default assumption set) never below the bound
    assert!(
        tl.t_min <= tl.t_scurve * (1.0 + 1e-12),
        "{} {}",
        tl.t_min,
        tl.t_scurve
    );
    for (g, t) in tl.segments.iter().zip(&tl.seg_scurve) {
        assert!(g.t_min <= t * (1.0 + 1e-9), "line {}: {} > {t}", g.line, g.t_min);
    }
    for (g, t) in tl.segments.iter().zip(&tl.seg_la) {
        assert!(g.t_min <= t * (1.0 + 1e-9), "line {}: {} > {t}", g.line, g.t_min);
        if matches!(g.path, sim::Path::Line) {
            assert!(
                *t <= g.t_stop * (1.0 + 1e-9),
                "line {}: {t} > {}",
                g.line,
                g.t_stop
            );
        }
    }
    let at = |k: f64| {
        lookahead::plan(
            &m,
            &tl,
            &LookAhead {
                accel_scale: k,
                ..Default::default()
            },
        )
        .1
    };
    let (h, one, two) = (at(0.5), at(1.0), at(2.0));
    assert!(h >= one && one >= two, "{h} {one} {two}");
    assert!(close(one, tl.t_la));
    println!(
        "t_min {:.3}  t_la {:.3} (x0.5 {h:.3}, x2 {two:.3})  t_stop {:.3}",
        tl.t_min, tl.t_la, tl.t_stop
    );
}

// ---- the jerk-limited planner (lookahead::plan_scurve) against closed forms

/// The 3-axis mill with no `accel` in its record: the planner's own values apply.
fn no_accel(mut m: forge_machine::MachineDef) -> forge_machine::MachineDef {
    for j in m.joints.iter_mut() {
        j.accel = None;
    }
    m
}

fn scurve_after(prog: &str, a: f64, j: f64) -> f64 {
    let (m, tl) = run(prog);
    let m = no_accel(m);
    let p = lookahead::SCurve {
        accel: [a, 3600.0],
        jerk: [j, 1e6],
        tolerance: 0.02,
    };
    let (per, _) = lookahead::plan_scurve(&m, &tl, &p);
    tl.segments
        .iter()
        .zip(&per)
        .filter(|(g, _)| g.line >= 4)
        .map(|(_, t)| t)
        .sum()
}

#[test]
fn a_lone_move_is_its_s_curve_and_unlimited_jerk_is_the_trapezoid() {
    // 100 mm at 100 mm/s, a = 3000: with jerk 1e4 the acceleration never reaches a (a²/j = 900 >
    // 100): each ramp takes 2 √(Δv/j) = 0.2 s over 10 mm; 80 mm cruise: 1.2 s
    let t = scurve_after("G1 X100. F6000.\n", 3000.0, 1e4);
    assert!(close(t, 1.2), "{t}");
    // jerk unbounded in effect: the trapezoid, 1 + 1/30 s
    let t = scurve_after("G1 X100. F6000.\n", 3000.0, 1e12);
    assert!(
        (t - trapezoid(100.0, 0.0, 0.0, 100.0, 3000.0)).abs() < 1e-6,
        "{t}"
    );
    // reaching a: Δv ≥ a²/j; a = 500, j = 1e4 (a²/j = 25 < 100): each ramp Δv/a + a/j = 0.25 s
    // over 12.5 mm; 75 mm cruise: 1.25 s
    let t = scurve_after("G1 X100. F6000.\n", 500.0, 1e4);
    assert!(close(t, 1.25), "{t}");
}

#[test]
fn an_arc_is_held_to_its_centripetal_limits() {
    // a full circle of radius 10 at 100 mm/s asked: √(a r) = √(30 × 10) ≈ 17.3 mm/s at a = 30 binds
    let prog = "G1 X10. F6000.\nG3 X10. Y0 I-10. J0\n";
    let (m, tl) = run(prog);
    let m = no_accel(m);
    let p = lookahead::SCurve {
        accel: [30.0, 3600.0],
        jerk: [1e9, 1e6],
        tolerance: 0.02,
    };
    let (per, _) = lookahead::plan_scurve(&m, &tl, &p);
    let g = tl
        .segments
        .iter()
        .position(|g| matches!(g.path, sim::Path::Arc { .. }))
        .unwrap();
    let len = 2.0 * std::f64::consts::PI * 10.0;
    assert!(
        per[g] >= len / (30.0f64 * 10.0).sqrt() * (1.0 - 1e-6),
        "{} vs {}",
        per[g],
        len / (300.0f64).sqrt()
    );
    // and the jerk limit: (j r²)^(1/3) with j = 1e3: 46.4 mm/s binds before √(a r) at a = 1e4
    let p = lookahead::SCurve {
        accel: [1e4, 3600.0],
        jerk: [1e3, 1e6],
        tolerance: 0.02,
    };
    let (per, _) = lookahead::plan_scurve(&m, &tl, &p);
    assert!(
        per[g] >= len / forge_math::dmath::cbrt(1e3 * 100.0) * (1.0 - 1e-6),
        "{}",
        per[g]
    );
}

#[test]
fn a_records_acceleration_takes_precedence_over_the_assumption_set() {
    // generic-vmc3's X: 3000 mm/s² in its record; the set's 500 is not used for it
    let (m, tl) = run("G1 X100. F6000.\n");
    let p = lookahead::SCurve {
        accel: [500.0, 3600.0],
        jerk: [1e12, 1e6],
        tolerance: 0.02,
    };
    let (per, _) = lookahead::plan_scurve(&m, &tl, &p);
    let t: f64 = tl
        .segments
        .iter()
        .zip(&per)
        .filter(|(g, _)| g.line >= 4)
        .map(|(_, t)| t)
        .sum();
    assert!(
        (t - trapezoid(100.0, 0.0, 0.0, 100.0, 3000.0)).abs() < 1e-6,
        "{t}"
    );
}
