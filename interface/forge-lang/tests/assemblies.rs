//! Assembly documents in Forge: the textbook mechanisms, a bolted assembly under one fit rule,
//! conflicts and limits with their typed fixes, ids under edits, `expect .. at`, and the
//! 5,000-instance targets measured through `run`.

use forge_lang::edit::apply;
use forge_lang::error::{Error, Op, ParamKey, Warning};
use forge_lang::{Options, Run};
use forge_math::dmath;
use std::path::{Path, PathBuf};

/// A fresh directory for one test's files.
fn dir(name: &str) -> PathBuf {
    let d = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("asm-{name}"));
    let _ = std::fs::create_dir_all(&d);
    d
}

fn write(d: &Path, file: &str, text: &str) {
    std::fs::write(d.join(file), text).expect("write a fixture");
}

/// A bar `len` long, anchors `a` at its start and `b` at its end, z up.
fn link(len: f64) -> String {
    format!(
        "shared ln = {len}
shared w = 8
shared t = 4
sketch plan(on: xy) {{
  outline = rect(corner: (-w / 2, -w / 2), size: (ln + w, w))
  p0 = pt(0, 0)
  p1 = pt(ln, 0)
}}
bar = extrude(plan, t)
a = anchor(plane(at: plan.p0, normal: plan.plane.normal, x: plan.outline[0].dir))
b = anchor(plane(at: plan.p1, normal: plan.plane.normal, x: plan.outline[0].dir))
"
    )
}

/// A ground plate with pivots `o2` at the origin and `o4` at (d, 0), and a track along x.
fn base(d: f64) -> String {
    format!(
        "shared d = {d}
shared w = 20
shared t = 3
sketch plan(on: xy) {{
  outline = rect(corner: (-w, -w - 300), size: (d + w + w, w + w))
  g0 = pt(0, 0)
  g1 = pt(d, 0)
}}
plate = extrude(plan, t)
o2 = anchor(plane(at: plan.g0, normal: plan.plane.normal, x: plan.outline[0].dir))
o4 = anchor(plane(at: plan.g1, normal: plan.plane.normal, x: plan.outline[0].dir))
track = anchor(plane(at: plan.g0, normal: plan.outline[0].dir, x: plan.plane.normal))
"
    )
}

/// A slider block: its pin z up, and its slide along its x.
const BLOCK: &str = "shared s = 6
sketch plan(on: xy) {
  outline = rect(corner: (-s, -s), size: (s + s, s + s))
  o = pt(0, 0)
}
body = extrude(plan, s)
pin = anchor(plane(at: plan.o, normal: plan.plane.normal, x: plan.outline[0].dir))
glide = anchor(plane(at: plan.o, normal: plan.outline[0].dir, x: plan.plane.normal))
";

fn open_four_bar(a: f64, b: f64, c: f64, d: f64, t2: f64) -> (f64, f64) {
    let (s2, c2) = dmath::sin_cos(t2);
    let (px, py) = (a * c2, a * s2);
    let (ex, ey) = (d - px, -py);
    let l = (ex * ex + ey * ey).sqrt();
    let x = (b * b - c * c + l * l) / (2.0 * l);
    let h = (b * b - x * x).max(0.0).sqrt();
    let (ux, uy) = (ex / l, ey / l);
    let (qx, qy) = (px + ux * x - uy * h, py + uy * x + ux * h);
    (dmath::atan2(qy - py, qx - px), dmath::atan2(qy, qx - d))
}

const A: f64 = 40.0;
const B: f64 = 120.0;
const C: f64 = 80.0;
const D: f64 = 100.0;

/// The four-bar as a Forge assembly, written at crank angle `t2` (degrees), driven by `j1.turn`.
fn four_bar_doc(t2: f64) -> String {
    let r = t2.to_radians();
    let (t3, t4) = open_four_bar(A, B, C, D, r);
    let (px, py) = (A * dmath::cos(r), A * dmath::sin(r));
    format!(
        "assembly

param j1.turn = {t2}deg

base = part(\"base.fg\", ground: true)
crank = part(\"crank.fg\", at: pose(rz: {t2}deg))
coupler = part(\"coupler.fg\", at: pose(at: ({px}, {py}, 0), rz: {}deg))
rocker = part(\"rocker.fg\", at: pose(at: ({D}, 0, 0), rz: {}deg))
j1 = revolute(crank.a, base.o2, turn: turn)
j2 = revolute(coupler.a, crank.b)
j3 = revolute(rocker.b, coupler.b)
j4 = revolute(rocker.a, base.o4)
",
        t3.to_degrees(),
        t4.to_degrees()
    )
}

fn mech_dir(name: &str) -> PathBuf {
    let d = dir(name);
    write(&d, "base.fg", &base(D));
    write(&d, "crank.fg", &link(A));
    write(&d, "coupler.fg", &link(B));
    write(&d, "rocker.fg", &link(C));
    write(&d, "rod.fg", &link(100.0));
    write(&d, "block.fg", BLOCK);
    d
}

fn run_in(d: &Path, src: &str) -> Run {
    let opts = Options {
        dir: Some(d.to_path_buf()),
        ..Options::default()
    };
    forge_lang::run(src, opts, Default::default()).0
}

fn errors(r: &Run) -> Vec<String> {
    r.errors
        .iter()
        .map(|e| format!("{} {}", e.kind(), e.text()))
        .collect()
}

fn pose(r: &Run, id: &str) -> forge_math::Frame {
    r.asm
        .as_ref()
        .expect("an assembly")
        .poses
        .iter()
        .find(|p| p.0 == id)
        .expect("an instance")
        .1
}

fn warnings_of(r: &Run, kind: &str) -> usize {
    r.warnings
        .iter()
        .filter(|w| matches!(w, Warning::Asm(n) if n.kind() == kind))
        .count()
}

#[test]
fn a_four_bar_follows_its_drive_param_and_keeps_three_redundant_rows() {
    let d = mech_dir("fourbar");
    let src = four_bar_doc(30.0);
    let r = run_in(&d, &src);
    assert!(r.ok(), "{:?}", errors(&r));
    let a = r.asm.as_ref().unwrap();
    assert_eq!(a.mobility, 0);
    assert_eq!(warnings_of(&r, "Redundant"), 2, "{:?}", r.warnings.len());
    // the drive is a param: SetParam is the motion, and the solve is incremental
    let mut cache = forge_lang::build::Cache::default();
    let opts = Options {
        dir: Some(d.clone()),
        ..Options::default()
    };
    let (_, c) = forge_lang::run(&src, opts.clone(), cache);
    cache = c;
    for t2 in [45.0, 60.0, 90.0, 135.0, 200.0] {
        let text = apply(
            &src,
            &Op::SetParam {
                param: ParamKey {
                    owner: vec![forge_lang::edit::seg("j1")],
                    name: "turn".into(),
                },
                value: forge_lang::error::Literal {
                    value: t2,
                    dim: forge_lang::values::Dim::Angle,
                },
            },
        )
        .unwrap();
        let (r, c) = forge_lang::run(&text, opts.clone(), cache);
        cache = c;
        assert!(r.ok(), "{:?}", errors(&r));
        let a = r.asm.as_ref().unwrap();
        assert!(a.timings.incremental, "a drive edit re-solves in place");
        let (_, t4) = open_four_bar(A, B, C, D, f64::to_radians(t2));
        let x = pose(&r, "rocker").x().get();
        assert!(
            (x.x - dmath::cos(t4)).abs() < 1e-9 && (x.y - dmath::sin(t4)).abs() < 1e-9,
            "{t2}: {x:?}"
        );
    }
}

#[test]
fn textbook_mobility_from_forge() {
    let d = mech_dir("mobility");
    // undriven four-bar: 1
    let free = four_bar_doc(30.0).replace(
        "revolute(crank.a, base.o2, turn: turn)",
        "revolute(crank.a, base.o2)",
    );
    let r = run_in(&d, &free.replace("param j1.turn = 30deg\n", ""));
    assert!(r.ok(), "{:?}", errors(&r));
    assert_eq!(r.asm.as_ref().unwrap().mobility, 1);
    // slider-crank: 1
    let t: f64 = 0.4;
    let (rr, l) = (A, 100.0);
    let (px, py) = (rr * dmath::cos(t), rr * dmath::sin(t));
    let bx = px + (l * l - py * py).sqrt();
    let t3 = dmath::atan2(-py, bx - px).to_degrees();
    let sc = format!(
        "assembly

base = part(\"base.fg\", ground: true)
crank = part(\"crank.fg\", at: pose(rz: {}deg))
rod = part(\"rod.fg\", at: pose(at: ({px}, {py}, 0), rz: {t3}deg))
block = part(\"block.fg\", at: pose(at: ({bx}, 0, 0)))
j1 = revolute(crank.a, base.o2)
j2 = revolute(rod.a, crank.b)
j3 = revolute(block.pin, rod.b)
j4 = slider(block.glide, base.track)
expect mobility = 1
",
        t.to_degrees()
    );
    let r = run_in(&d, &sc);
    assert!(r.ok(), "{:?}", errors(&r));
    assert_eq!(r.asm.as_ref().unwrap().mobility, 1);
    // a gear pair: 1, and its ratio
    let gears = "assembly

shared k = ratio(0.5)
param g.turn = 40deg

base = part(\"base.fg\", ground: true)
w1 = part(\"crank.fg\")
w2 = part(\"crank.fg\", at: pose(at: (100, 0, 0)))
g = revolute(w1.a, base.o2, turn: turn)
h = revolute(w2.a, base.o4)
mesh = gear(g, h, ratio: k)
expect mobility = 0
expect value(h.turn) = -20deg +- 0.000001deg
";
    let r = run_in(&d, gears);
    assert!(r.ok(), "{:?}", errors(&r));
    let free = gears
        .replace(", turn: turn)", ")")
        .replace("param g.turn = 40deg\n", "");
    let free = free.replace(
        "expect mobility = 0\nexpect value(h.turn) = -20deg +- 0.000001deg\n",
        "expect mobility = 1\n",
    );
    let r = run_in(&d, &free);
    assert!(r.ok(), "{:?}", errors(&r));
}

#[test]
fn expect_at_holds_a_param_for_one_check() {
    let d = mech_dir("expectat");
    let (_, t4) = open_four_bar(A, B, C, D, 60f64.to_radians());
    let src = format!(
        "{}expect value(j4.turn) = {}deg +- 0.000001deg at j1.turn = 60deg\nexpect value(j4.turn) = 0deg at j1.turn = 60deg\n",
        four_bar_doc(30.0),
        t4.to_degrees()
    );
    let r = run_in(&d, &src);
    let failed: Vec<&Error> = r.errors.iter().filter(|e| e.kind() == "ExpectFailed").collect();
    assert_eq!(failed.len(), 1, "{:?}", errors(&r));
    // the failing one is the second line
    let line = failed[0].span().unwrap().lines(&src).0;
    assert!(
        src.lines()
            .nth(line - 1)
            .unwrap()
            .starts_with("expect value(j4.turn) = 0deg at")
    );
}

#[test]
fn a_conflicting_pivot_is_named_and_its_fix_removes_it() {
    let d = mech_dir("conflict");
    let src = format!("{}j5 = revolute(crank.b, base.o4)\n", four_bar_doc(30.0));
    let r = run_in(&d, &src);
    let conflicts: Vec<&Error> = r.errors.iter().filter(|e| e.kind() == "Conflict").collect();
    assert_eq!(conflicts.len(), 1, "{:?}", errors(&r));
    let text = conflicts[0].text();
    assert!(
        text.contains("j1 (coincident)") && text.contains("j5 (coincident)"),
        "{text}"
    );
    assert_eq!(conflicts[0].fixes(), vec![Op::DeleteFeature { id: "j5".into() }]);
    let fixed = apply(&src, &conflicts[0].fixes()[0]).unwrap();
    let r = run_in(&d, &fixed);
    assert!(r.ok(), "{:?}", errors(&r));
}

#[test]
fn a_drive_past_its_limit_is_fixed_by_setting_its_param() {
    let d = mech_dir("limit");
    let src = "assembly

param g.turn = 50deg
param g.lo = -30deg
param g.hi = 30deg

base = part(\"base.fg\", ground: true)
w1 = part(\"crank.fg\")
g = revolute(w1.a, base.o2, turn: turn, limit: (lo, hi))
";
    let r = run_in(&d, src);
    let lim: Vec<&Error> = r.errors.iter().filter(|e| e.kind() == "LimitViolated").collect();
    assert_eq!(lim.len(), 1, "{:?}", errors(&r));
    let fix = lim[0].fixes();
    assert_eq!(fix.len(), 1);
    assert!(
        matches!(&fix[0], Op::SetParam { param, value } if param.to_string() == "g.turn" && (value.value - 30.0).abs() < 1e-9)
    );
    let fixed = apply(src, &fix[0]).unwrap();
    assert!(run_in(&d, &fixed).ok());
}

#[test]
fn ids_and_poses_hold_under_edits() {
    let d = mech_dir("ids");
    let src = four_bar_doc(30.0);
    let r0 = run_in(&d, &src);
    assert!(r0.ok());
    let ids0: Vec<String> = r0.feats.iter().map(|f| f.id.clone()).collect();
    // a new part above the joints, and a joint moved: every other id and pose the same bits
    let added = apply(
        &src,
        &Op::AddFeature {
            before: Some("j1".into()),
            text: "spare = part(\"crank.fg\", at: pose(at: (0, 500, 0)))".into(),
        },
    )
    .unwrap();
    let moved = apply(
        &added,
        &Op::MoveFeature {
            id: "j4".into(),
            before: Some("j2".into()),
        },
    )
    .unwrap();
    let r1 = run_in(&d, &moved);
    assert!(r1.ok(), "{:?}", errors(&r1));
    let ids1: Vec<String> = r1.feats.iter().map(|f| f.id.clone()).collect();
    for id in &ids0 {
        assert!(ids1.contains(id), "{id} kept");
    }
    for id in ["base", "crank", "coupler", "rocker"] {
        assert_eq!(pose(&r0, id), pose(&r1, id), "{id}");
    }
}

/// A plate with M6 seats `tap1` and `tap2`, and bolts with 12 or 3 of thread.
/// One directory per test: tests run at once and must not write each other's files.
fn bolted_dir(name: &str) -> PathBuf {
    let d = dir(name);
    write(
        &d,
        "plate.fg",
        "shared L = 60
shared W = 40
shared t = 10
shared m = 6
shared sx = 15
shared sy = 20
sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (L, W))
  s1 = pt(sx, sy)
  s2 = pt(L - sx, sy)
}
plate = extrude(plan, t)
tap1 = anchor(plane(at: plan.s1, normal: plan.plane.normal, x: plan.outline[0].dir), kind: internal_thread, size: m, depth: t)
tap2 = anchor(plane(at: plan.s2, normal: plan.plane.normal, x: plan.outline[0].dir), kind: internal_thread, size: m, depth: t)
",
    );
    let bolt = |tl: f64| {
        format!(
            "shared r = 3
shared ln = 20
shared m = 6
shared tl = {tl}
sketch plan(on: xy) {{
  c = circle((0, 0), r)
  o = pt(0, 0)
  ax = line((0, 0), (r, 0), construction: true)
}}
shank = extrude(plan.c, ln)
thd = anchor(plane(at: plan.o, normal: plan.plane.normal, x: plan.ax.dir), kind: external_thread, size: m, depth: tl)
"
        )
    };
    write(&d, "bolt.fg", &bolt(12.0));
    write(&d, "stub.fg", &bolt(3.0));
    d
}

#[test]
fn a_bolted_plate_under_one_fit_rule() {
    let d = bolted_dir("bolted-1");
    let src = "assembly

shared m = 6

base = part(\"plate.fg\", ground: true)
b1 = part(\"bolt.fg\")
b2 = part(\"stub.fg\")
f1 = fastened(b1.thd, base.tap1)
f2 = fastened(b2.thd, base.tap2)
m6 = fit_rule(external_thread, internal_thread, size: m)
expect interferences = 0
";
    let r = run_in(&d, src);
    assert!(r.ok(), "{:?}", errors(&r));
    let a = r.asm.as_ref().unwrap();
    assert_eq!(a.rules, vec![("m6".to_string(), 2)]);
    assert_eq!(a.intended, 2);
    // the stub's 3 of thread are under the shop's one diameter (6)
    assert_eq!(warnings_of(&r, "ShallowEngagement"), 1);
    assert_eq!(warnings_of(&r, "IntendedContact"), 2);
    // without the rule both bolts interfere: warnings, and the fix declares a fit
    let bare = src.replace("m6 = fit_rule(external_thread, internal_thread, size: m)\n", "");
    let r = run_in(&d, &bare);
    assert_eq!(warnings_of(&r, "Interference"), 2);
    let Some(Warning::Asm(n)) = r
        .warnings
        .iter()
        .find(|w| matches!(w, Warning::Asm(n) if n.kind() == "Interference"))
    else {
        panic!("an interference")
    };
    let fixed = apply(&bare, &n.fixes()[0]).unwrap();
    let r = run_in(&d, &fixed);
    assert_eq!(warnings_of(&r, "Interference"), 1, "one declared fit removes one");
    // a kind no part declares is refused
    let unknown = src.replace(
        "fit_rule(external_thread, internal_thread",
        "fit_rule(kind(\"dowel\"), internal_thread",
    );
    let r = run_in(&d, &unknown);
    assert!(
        r.errors.iter().any(|e| e.kind() == "UnknownKind"),
        "{:?}",
        errors(&r)
    );
}

#[test]
fn part_built_ins_are_refused_in_an_assembly_and_back() {
    let d = mech_dir("wrongdoc");
    let r = run_in(
        &d,
        "assembly\nshared t = 4\nsketch plan(on: xy) {\n  c = circle((0, 0), t)\n}\nx = extrude(plan, t)\n",
    );
    assert!(
        r.errors.iter().any(|e| e.kind() == "WrongDocument"),
        "{:?}",
        errors(&r)
    );
    let r = run_in(&d, "j = revolute(a, b)\n");
    assert!(!r.ok());
}

/// 230 plates of 20 bolts each under one fit rule, 50 four-bars, a sliding table: through `run`
/// as `fg run` and `fg serve` do it.
#[test]
#[cfg(not(target_family = "wasm"))]
fn five_thousand_instances_through_forge() {
    let d = mech_dir("scale");
    write(
        &d,
        "seats.fg",
        "shared L = 100
shared t = 5
shared m = 6
shared xs = [10, 30, 50, 70, 90, 10, 30, 50, 70, 90, 10, 30, 50, 70, 90, 10, 30, 50, 70, 90]
shared ys = [10, 10, 10, 10, 10, 35, 35, 35, 35, 35, 60, 60, 60, 60, 60, 85, 85, 85, 85, 85]
sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (L, L))
  for k in 0..20 {
    s = pt(xs[k], ys[k])
  }
}
plate = extrude(plan, t)
for k in 0..20 {
  seat = anchor(plane(at: plan.s[k], normal: plan.plane.normal, x: plan.outline[0].dir), kind: internal_thread, size: m, depth: t)
}
",
    );
    write(
        &d,
        "m6.fg",
        "shared r = 3
shared ln = 12
shared m = 6
shared tl = 10
sketch plan(on: xy) {
  c = circle((0, 0), r)
  o = pt(0, 0)
  ax = line((0, 0), (r, 0), construction: true)
}
shank = extrude(plan.c, ln)
thd = anchor(plane(at: plan.o, normal: plan.plane.normal, x: plan.ax.dir), kind: external_thread, size: m, depth: tl)
",
    );
    let mut src = String::from(
        "assembly

shared m = 6
shared pitch = 110
param table.along = 0
param j1.turn = 30deg
param j1[7].turn = 30deg

base = part(\"base.fg\", ground: true)
rule = fit_rule(external_thread, internal_thread, size: m)
for r in 0..15 {
  for c in 0..16 {
    plate = part(\"seats.fg\", at: pose(at: (c * pitch, r * pitch + 1000, 0)))
    hold = fastened(plate.a, base.o2)
  }
}
",
    );
    // the plates are anchored by their own frame (a reference to the instance itself) where written
    src = src.replace("hold = fastened(plate.a, base.o2)", "hold = lock(plate, base)");
    let _ = &mut src;
    // the plates are fastened where they are written: lock keeps the written relative pose? no:
    // lock joins frames, so each plate sits on the base frame; place them by a pose anchor instead
    let mut src = String::from(
        "assembly

shared m = 6
shared spacing = 400
param table.along = 0
param j1.turn = 30deg
param j1[7].turn = 30deg

base = part(\"base.fg\", ground: true)
rule = fit_rule(external_thread, internal_thread, size: m)
",
    );
    for r in 0..15 {
        for c in 0..16 {
            if r * 16 + c >= 230 {
                break;
            }
            let k = r * 16 + c;
            src.push_str(&format!(
                "p{k} = part(\"seats.fg\", at: pose(at: ({}, {}, 0)), ground: true)\n",
                c as f64 * 110.0,
                r as f64 * 110.0 + 1000.0
            ));
            src.push_str(&format!(
                "for k in 0..20 {{\n  bolt{k} = part(\"m6.fg\")\n  hold{k} = fastened(bolt{k}.thd, p{k}.seat[k])\n}}\n"
            ));
        }
    }
    // a sliding table: plate p0 on a slider instead of ground
    let src = src.replacen(
        "p0 = part(\"seats.fg\", at: pose(at: (0, 1000, 0)), ground: true)",
        "p0 = part(\"seats.fg\", at: pose(at: (0, 1000, 0)))\ntable = slider(p0, base.track, along: along)",
        1,
    );
    let mut src = src;
    src.push_str("for k in 0..50 {\n");
    let r = 30f64.to_radians();
    let (t3, t4) = open_four_bar(A, B, C, D, r);
    let (px, py) = (A * dmath::cos(r), A * dmath::sin(r));
    src.push_str(&format!(
        "  ground = part(\"base.fg\", at: pose(at: (-1000, k * spacing, 0)), ground: true)
  crank = part(\"crank.fg\", at: pose(at: (-1000, k * spacing, 0), rz: 30deg))
  coupler = part(\"coupler.fg\", at: pose(at: ({px} - 1000, {py} + k * spacing, 0), rz: {}deg))
  rocker = part(\"rocker.fg\", at: pose(at: ({D} - 1000, k * spacing, 0), rz: {}deg))
  j1 = revolute(crank.a, ground.o2, turn: turn)
  j2 = revolute(coupler.a, crank.b)
  j3 = revolute(rocker.b, coupler.b)
  j4 = revolute(rocker.a, ground.o4)
  pin1 = fit(crank, coupler)
  pin2 = fit(coupler, rocker)
}}
",
        t3.to_degrees(),
        t4.to_degrees()
    ));
    let opts = Options {
        dir: Some(d.clone()),
        ..Options::default()
    };
    // cold three times, each from nothing; the best of each time
    let mut cold = std::time::Duration::MAX;
    let (mut anchors, mut solve0, mut overlap0, mut load) = (f64::MAX, f64::MAX, f64::MAX, f64::MAX);
    let mut first = None;
    for _ in 0..3 {
        let t0 = std::time::Instant::now();
        let (run, cache) = forge_lang::run(&src, opts.clone(), Default::default());
        cold = cold.min(t0.elapsed());
        assert!(
            run.ok(),
            "{:?}",
            errors(&run).into_iter().take(3).collect::<Vec<_>>()
        );
        let a = run.asm.as_ref().unwrap();
        anchors = anchors.min(a.timings.anchors_ms);
        solve0 = solve0.min(a.timings.solve_ms);
        overlap0 = overlap0.min(a.timings.overlap_ms);
        load = load.min(a.timings.load_ms);
        first = Some((run, cache));
    }
    let (run, mut cache) = first.unwrap();
    let a = run.asm.as_ref().unwrap();
    let n = a.poses.len();
    assert!(n >= 5000, "{n} instances");
    assert_eq!(a.interferences, 0);
    assert_eq!(a.rules, vec![("rule".to_string(), 4600)]);

    let set = |text: &str, owner: Vec<forge_lang::syntax::Seg>, name: &str, v: f64, dim| {
        apply(
            text,
            &Op::SetParam {
                param: ParamKey {
                    owner,
                    name: name.into(),
                },
                value: forge_lang::error::Literal { value: v, dim },
            },
        )
        .unwrap()
    };
    let j17 = vec![forge_lang::syntax::Seg {
        name: "j1".into(),
        index: vec![forge_lang::syntax::Key::Ix(7)],
    }];
    // one joint edit, three times: j1[7]'s drive
    let (mut recheck, mut edit_total) = (f64::MAX, std::time::Duration::MAX);
    let mut text = src.clone();
    for v in [50.0, 60.0, 70.0] {
        text = set(&src, j17.clone(), "turn", v, forge_lang::values::Dim::Angle);
        let t1 = std::time::Instant::now();
        let (run2, c) = forge_lang::run(&text, opts.clone(), cache);
        edit_total = edit_total.min(t1.elapsed());
        cache = c;
        let a2 = run2.asm.as_ref().unwrap();
        assert!(run2.ok());
        assert!(a2.timings.incremental);
        assert_eq!(a2.stats.partitions_solved, 1, "{:?}", a2.stats);
        assert_eq!(a2.stats.blocks_solved, 1);
        recheck = recheck.min(a2.timings.solve_ms);
    }
    // a drag, three times: the table slides
    let (mut drag_overlap, mut drag_total) = (f64::MAX, std::time::Duration::MAX);
    let mut o = None;
    for x in [1.0, 2.0, 3.0] {
        let dragged = set(
            &text,
            vec![forge_lang::edit::seg("table")],
            "along",
            x,
            forge_lang::values::Dim::Length,
        );
        let t2 = std::time::Instant::now();
        let (run3, c) = forge_lang::run(&dragged, opts.clone(), cache);
        drag_total = drag_total.min(t2.elapsed());
        cache = c;
        let a3 = run3.asm.as_ref().unwrap();
        assert!(
            run3.ok(),
            "{:?}",
            errors(&run3).into_iter().take(3).collect::<Vec<_>>()
        );
        assert!(a3.timings.incremental);
        drag_overlap = drag_overlap.min(a3.timings.overlap_ms);
        o = a3.overlap.clone();
    }
    let o = o.unwrap();
    println!(
        "forge: {n} instances; cold run {cold:?} (load {load:.1} ms, anchors {anchors:.2} ms, solve {solve0:.2} ms, overlap {overlap0:.1} ms); \
         joint edit: run {edit_total:?}, re-check {recheck:.2} ms; drag: run {drag_total:?}, overlap {drag_overlap:.2} ms ({} candidates, {} cached)",
        o.candidates, o.cache_hits
    );
    assert!(anchors < 10.0, "anchor check {anchors} ms");
    assert!(recheck < 10.0, "re-check {recheck} ms");
    assert!(overlap0 < 1000.0, "overlap cold {overlap0} ms");
    assert!(drag_overlap < 50.0, "overlap after a drag {drag_overlap} ms");
}

#[test]
fn a_name_at_is_refused_typed_and_renamed_by_its_fix() {
    let d = mech_dir("reserved");
    let part = "shared at = 5
sketch k(on: xy) {
  o = rect(corner: (0, 0), size: (at, at))
}
f = extrude(k, at)
";
    let r = run_in(&d, part);
    assert_eq!(r.errors.len(), 1);
    let e = &r.errors[0];
    assert_eq!(e.kind(), "ReservedWord");
    assert_eq!(e.span().unwrap().lo, 7);
    let fix = e.fixes();
    assert_eq!(
        fix,
        vec![Op::Rename {
            from: "at".into(),
            to: "at_1".into()
        }]
    );
    let fixed = apply(part, &fix[0]).unwrap();
    assert!(fixed.contains("size: (at_1, at_1)"));
    assert!(run_in(&d, &fixed).ok());
    // in an assembly, `at:` arguments and an expect's clause stay keywords
    let asm = "assembly

shared at = 30deg

base = part(\"base.fg\", ground: true)
w1 = part(\"crank.fg\", at: pose(rz: 10deg))
g = revolute(w1.a, base.o2, turn: at)
expect value(g.turn) = 45deg at at = 45deg
";
    let r = run_in(&d, asm);
    assert_eq!(r.errors[0].kind(), "ReservedWord");
    let fixed = apply(asm, &r.errors[0].fixes()[0]).unwrap();
    assert!(
        fixed.contains("part(\"crank.fg\", at: pose") && fixed.contains("at at_1 = 45deg"),
        "{fixed}"
    );
    let r = run_in(&d, &fixed);
    assert!(r.ok(), "{:?}", errors(&r));
}

/// Files an assembly names are found as media files are: from the document's directory, a real
/// `.fg` file of at most 64 MB under the session's root. Every way out is refused alike.
#[test]
#[cfg(unix)]
fn an_assembly_reads_nothing_outside_its_root() {
    let top = dir("security");
    let root = top.join("root");
    let _ = std::fs::create_dir_all(&root);
    write(&root, "base.fg", &base(D));
    write(&root, "crank.fg", &link(A));
    write(&top, "outside.fg", &link(A));
    write(&root, "notes.txt", &link(A));
    let _ = std::fs::remove_file(root.join("link.fg"));
    std::os::unix::fs::symlink(top.join("outside.fg"), root.join("link.fg")).unwrap();
    let big = std::fs::File::create(root.join("big.fg")).unwrap();
    big.set_len((64 << 20) + 1).unwrap();
    let opts = Options {
        dir: Some(root.clone()),
        root: Some(root.clone()),
        ..Options::default()
    };
    let outside = top.join("outside.fg").to_string_lossy().to_string();
    for file in [
        outside.as_str(),
        "../outside.fg",
        "link.fg",
        "notes.txt",
        "big.fg",
        "nowhere.fg",
    ] {
        for call in ["part", "subassembly"] {
            let src = format!("assembly\n\nbase = part(\"base.fg\", ground: true)\nx = {call}(\"{file}\")\n");
            let r = forge_lang::run(&src, opts.clone(), Default::default()).0;
            let texts: Vec<String> = r.errors.iter().map(|e| e.text()).collect();
            assert_eq!(texts, vec![format!("{file}: no such file")], "{call}({file})");
            assert!(r.errors.iter().all(|e| e.kind() == "PartFile"));
        }
    }
    // the same parts inside the root are read
    let ok = "assembly\n\nbase = part(\"base.fg\", ground: true)\nx = part(\"crank.fg\")\n";
    let r = forge_lang::run(ok, opts, Default::default()).0;
    assert!(r.ok(), "{:?}", errors(&r));
}

/// The root reaches the parts an assembly loads: a part under it cannot import a STEP file or
/// read a font outside it, refused with the same typed Missing as any file not found.
#[test]
fn a_part_loaded_by_an_assembly_reads_nothing_outside_the_root() {
    let top = dir("security-nested");
    let root = top.join("root");
    let sub = root.join("sub");
    let _ = std::fs::create_dir_all(&sub);
    // real files outside: a STEP block and a font, which a part with no root reads cleanly
    let block = forge_ops::extrude(
        &forge_ops::Profile {
            plane: forge_math::Frame::WORLD,
            loops: vec![forge_ops::Profile::rectangle(
                forge_math::Point2::new(0.0, 0.0),
                10.0,
                10.0,
                1,
            )],
        },
        10.0,
        1,
    )
    .unwrap();
    let step = forge_step::export(&block, &forge_step::Options::default())
        .unwrap()
        .text;
    write(&top, "outside.step", &step);
    std::fs::copy(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../forge-render/fonts/PublicSans-Regular.ttf"
        ),
        top.join("outside.ttf"),
    )
    .unwrap();
    write(&root, "base.fg", &base(D));
    write(
        &sub,
        "marked.fg",
        "shared t = 4
shared size = 5
sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (t * 10, t * 5))
  o = pt(t, t)
}
body = extrude(plan, t)
other = import(\"../../outside.step\")
mark = text(\"A\", font: \"../../outside.ttf\", size: size, depth: t, on: body.end, at: plan.o)
",
    );
    let part_src = std::fs::read_to_string(sub.join("marked.fg")).unwrap();
    let file_errors = |root: Option<PathBuf>| -> Vec<String> {
        let opts = Options {
            dir: Some(sub.clone()),
            root,
            ..Options::default()
        };
        forge_lang::run(&part_src, opts, Default::default())
            .0
            .errors
            .iter()
            .filter(|e| e.kind() == "File")
            .map(|e| e.text())
            .collect()
    };
    // under the root: both refused, the same way as a file that is not there
    assert_eq!(
        file_errors(Some(root.clone())),
        vec![
            "../../outside.step: not found beside the part file".to_string(),
            "../../outside.ttf: not found beside the part file".to_string(),
        ]
    );
    // with no root both are read: the fence is what refuses
    assert_eq!(file_errors(None), Vec::<String>::new());
    // through an assembly under the root, the part's own reads are fenced the same way
    let opts = Options {
        dir: Some(root.clone()),
        root: Some(root.clone()),
        ..Options::default()
    };
    let src = "assembly\n\nbase = part(\"base.fg\", ground: true)\nm = part(\"sub/marked.fg\")\n";
    let r = forge_lang::run(src, opts.clone(), Default::default()).0;
    let texts: Vec<String> = r.errors.iter().map(|e| e.text()).collect();
    assert_eq!(
        texts,
        vec!["sub/marked.fg: 2 errors, the first File".to_string()],
        "{texts:?}"
    );
    // and the same assembly with no root loads the part cleanly
    let open = Options { root: None, ..opts };
    let r = forge_lang::run(src, open, Default::default()).0;
    assert!(r.ok(), "{:?}", errors(&r));
}

#[test]
fn expects_measure_anchors_and_role_names_and_none_is_skipped() {
    let d = mech_dir("expect-anchors");
    let (_, t4) = open_four_bar(A, B, C, D, 30f64.to_radians());
    let (bx, by) = (D + C * dmath::cos(t4), C * dmath::sin(t4));
    let src = format!(
        "{}expect x(rocker.a) = {D} +- 0.000001
expect y(rocker.b) = {by} +- 0.000001
expect x(rocker.b) = {bx} +- 0.000001
expect distance(crank.b, coupler.a) = 0 +- 0.000001
expect z(crank.bar.end) = 4 +- 0.000001
expect distance(base.o2, base.o4) = {D} +- 0.000001
expect x(rocker.nothing) = 0
expect x(42) = 0
expect value(rocker) = 0
expect x() = 0
expect x(rocker.a)
",
        four_bar_doc(30.0)
    );
    let r = run_in(&d, &src);
    let a = r.asm.as_ref().expect("an assembly");
    // every expect is listed: six measured and passing, four that cannot be measured, one with
    // no limit (measured and listed)
    assert_eq!(a.expects.len(), 11, "{:?}", a.expects);
    for e in &a.expects[..6] {
        assert!(e.ok && e.measured.is_some(), "{e:?}");
    }
    for e in &a.expects[6..10] {
        assert!(!e.ok && e.measured.is_none(), "{e:?}");
    }
    assert!(a.expects[10].ok && (a.expects[10].measured.unwrap() - D).abs() < 1e-9);
    // each one that cannot be measured is a typed error on its own line
    let unevaluable: Vec<(usize, String)> = r
        .errors
        .iter()
        .filter(|e| e.kind() == "ExpectUnevaluable")
        .map(|e| (e.span().unwrap().lines(&src).0, e.text()))
        .collect();
    assert_eq!(unevaluable.len(), 4, "{:?}", errors(&r));
    assert!(
        unevaluable[0].1.contains("rocker.nothing is not a reference"),
        "{unevaluable:?}"
    );
    assert!(unevaluable[1].1.contains("not an instance or a reference"));
    assert!(unevaluable[2].1.contains("takes a joint value"));
    assert!(unevaluable[3].1.contains("argument 1 is missing"));
    // and the JSON lists every expect with its value and verdict
    let json = forge_lang::report::json(&r, &src);
    let listed = json.get("assembly").and_then(|x| x.get("expects")).unwrap();
    let forge_agent::json::Json::Arr(items) = listed else {
        panic!("a list")
    };
    assert_eq!(items.len(), 11);
    assert_eq!(items[0].get("ok"), Some(&forge_agent::json::Json::Bool(true)));
    assert_eq!(items[6].get("measured"), Some(&forge_agent::json::Json::Null));
}

#[test]
fn every_threaded_fit_reports_its_engagement_and_full_means_full() {
    let d = bolted_dir("bolted-2");
    let src = "assembly

shared m = 6

base = part(\"plate.fg\", ground: true)
b1 = part(\"bolt.fg\")
b2 = part(\"stub.fg\")
f1 = fastened(b1.thd, base.tap1)
f2 = fastened(b2.thd, base.tap2)
m6 = fit_rule(external_thread, internal_thread, size: m)
";
    let r = run_in(&d, src);
    let e: Vec<(String, f64, Option<f64>, bool)> = r
        .asm
        .as_ref()
        .unwrap()
        .engagements
        .iter()
        .map(|x| (x.a.clone() + "/" + &x.b, x.engaged, x.min, x.short))
        .collect();
    // the bolt engages the whole 10 of the seat; the stub its 3, short of the shop's 6
    assert_eq!(
        e,
        vec![
            ("b1/base".to_string(), 10.0, Some(6.0), false),
            ("b2/base".to_string(), 3.0, Some(6.0), true)
        ]
    );
    // full: the screw must fill the seat's whole thread (10)
    let full = src.replace("size: m)", "size: m, min_engagement: full)");
    let r = run_in(&d, &full);
    let e: Vec<(f64, Option<f64>, bool)> = r
        .asm
        .as_ref()
        .unwrap()
        .engagements
        .iter()
        .map(|x| (x.engaged, x.min, x.short))
        .collect();
    assert_eq!(e, vec![(10.0, Some(10.0), false), (3.0, Some(10.0), true)]);
    assert_eq!(warnings_of(&r, "ShallowEngagement"), 1);
    let json = forge_lang::report::json(&r, &full);
    let forge_agent::json::Json::Arr(listed) = json
        .get("assembly")
        .and_then(|x| x.get("engagements"))
        .unwrap()
        .clone()
    else {
        panic!("a list")
    };
    assert_eq!(listed.len(), 2);
}

#[test]
fn clearance_and_interference_measure_subassemblies_and_fitted_pairs() {
    let d = mech_dir("sub-measure");
    write(
        &d,
        "one_block.fg",
        "assembly\n\nblk = part(\"block.fg\", ground: true)\n",
    );
    write(&d, "nothing.fg", "assembly\n");
    // blocks are 12 x 12 x 6 about their origins: 30 apart, 18 clear; 6 apart, 6 x 12 x 6 shared
    let src = "assembly

shared gap = 30
shared lap = 6

a = part(\"block.fg\", ground: true)
s = subassembly(\"one_block.fg\", at: pose(at: (gap, 0, 0)))
b = part(\"block.fg\", ground: true, at: pose(at: (lap, 0, 0)))
f = fit(a, b)
e = subassembly(\"nothing.fg\")
expect clearance(s, a) = 18 +- 0.000001
expect clearance(s.blk, a) = 18 +- 0.000001
expect interference(a, b) = 432 +- 0.000001
expect interference(s, a) = 0
expect interferences = 0
expect clearance(e, a) = 0
";
    let r = run_in(&d, src);
    let a = r.asm.as_ref().unwrap();
    let got: Vec<(Option<f64>, bool)> = a.expects.iter().map(|e| (e.measured, e.ok)).collect();
    assert_eq!(got[..5].iter().filter(|x| x.1).count(), 5, "{:?}", a.expects);
    // a subassembly with nothing in it has no solid: typed, not infinite and passing
    assert_eq!(got[5], (None, false));
    let errs: Vec<String> = r.errors.iter().map(|e| e.text()).collect();
    assert_eq!(errs.len(), 1, "{errs:?}");
    assert!(errs[0].contains("e has no solid to measure"), "{errs:?}");
}

#[test]
fn a_loop_index_divides_into_rows_and_columns() {
    let d = mech_dir("divmod");
    let src = "shared xs = [10, 30, 50]
shared ys = [10, 40]
shared w = 80
sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (w, w))
  for k in 0..6 {
    s = pt(xs[mod(k, 3)], ys[div(k, 3)])
  }
}
body = extrude(plan, w)
";
    let r = run_in(&d, src);
    assert!(r.ok(), "{:?}", errors(&r));
    for bad in ["div(k, 0)", "div(k, w)"] {
        let r = run_in(&d, &src.replace("div(k, 3)", bad));
        assert!(!r.ok(), "{bad}");
        assert!(
            r.errors
                .iter()
                .all(|e| matches!(e.kind(), "BadValue" | "WrongDim")),
            "{:?}",
            errors(&r)
        );
    }
}
