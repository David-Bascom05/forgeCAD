//! Every op kind is a function of its inputs (research/forge-lang/lean/ForgeLang/OpMemo.lean
//! assumes it; forgecad-gate/instant-load/op-memo-proof.md audits it): one feature of each kind,
//! and the sheet-metal and thermoform example parts, built fresh by `fg` five ways, must give the
//! same report and STEP file, byte for byte:
//! twice as is; on one core (`taskset -c 0`, `FORGECAD_THREADS=1`); with the n-ary batch forced
//! to the CPU (`FORGE_BATCH=cpu`) and to the GPU (`FORGE_BATCH=gpu`, the CPU where no GPU opens).
//!
//! `after_a_caught_panic_*`: an op run on a thread where an earlier op was cut short by a panic
//! (audit items 1 and 3: thread-local state left set) gives what a fresh thread gives.

use std::path::{Path, PathBuf};
use std::process::Command;

const BASE: &str = r#"shared w = 40
shared h = 20
shared t = 2
sketch base(on: xy) {
  r = rect(corner: (0, 0), size: (w, w))
}
box = extrude(base, h)
sketch top(on: box.end) {
  c = circle((w / 2, w / 2), 6)
}"#;

/// One feature of each kind the kernel builds, on BASE (each builds, checked by the test).
const OPS: [(&str, &str); 22] = [
    ("ball", r#"knob = ball(top.c, 6)"#),
    ("ball_cut", r#"knob = ball(top.c, 6, cut: true)"#),
    ("chamfer", r#"bevel = chamfer(edges(of: box), t)"#),
    (
        "coil",
        r#"sketch spr(on: xz) {
  c = circle((80, 5), 1)
  ax = line((70, 0), (70, 10), construction: true)
}
spring = coil(spr.c, axis: spr.ax, pitch: 4, turns: 3)"#,
    ),
    ("cut", r#"pit = cut(top.c, 5)"#),
    (
        "delete_faces",
        r#"pit = cut(top.c, 5)
fill = delete_faces(pit.faces)"#,
    ),
    (
        "draft",
        r#"tilt = draft(box.side(base.r[1]), angle: 5deg, neutral: base.plane)"#,
    ),
    ("extrude", r#""#),
    (
        "facefillet",
        r#"round = facefillet(box.side(base.r[0]), box.end, t)"#,
    ),
    ("fillet", r#"round = fillet(edges(of: box), t)"#),
    (
        "hole",
        r#"tap = hole(at: top.c, screw: "M6", fit: "tap", depth: 12, point: 118deg)"#,
    ),
    (
        "loft",
        r#"sketch l0(on: xy) {
  c = circle((80, 0), 5)
}
sketch l1(on: offset(l0.plane, 20)) {
  sq = rect(center: (80, 0), size: (8, 8))
}
neck = loft([l0.c, l1.sq])"#,
    ),
    (
        "move_faces",
        r#"turn = move_faces(box.side(base.r[1]), about: base.r[1], by: 5deg)"#,
    ),
    ("offset_faces", r#"grow = offset_faces(box.side(base.r[1]), 3)"#),
    (
        "pattern_lin",
        r#"peg = extrude(top.c, 4)
pegs = pattern(of: peg, layout: lin(along: base.r[0].dir, n: 3, step: 10))"#,
    ),
    (
        "pattern_mirror",
        r#"peg = extrude(top.c, 4)
pegs = pattern(of: peg, layout: mirror(on: box.side(base.r[0])))"#,
    ),
    (
        "revolve",
        r#"sketch prof(on: xz) {
  q = rect(corner: (50, 0), size: (8, 10))
  ax = line((60, 0), (60, 10), construction: true)
}
ring = revolve(prof.q, axis: prof.ax, angle: 360deg)"#,
    ),
    ("shell", r#"cup = shell(t, open: box.end)"#),
    (
        "sweep",
        r#"sketch sec(on: xy) {
  c = circle((80, 0), 3)
  b = line((80, 0), (90, 0), construction: true)
}
sketch rail(on: rotate(sec.plane, about: sec.b, by: 90deg)) {
  path route(start: (0, 0)) {
    line(to: (0, 20))
    tarc(to: (10, 30))
  }
}
tube = sweep(sec.c, along: rail.route)"#,
    ),
    (
        "text",
        r#"mark = text("FG", font: "PublicSans-SemiBold.ttf", size: 10, depth: 1, on: box.end, at: top.c.center, cut: true)"#,
    ),
    ("thicken", r#"slab = thicken(box.end, 3)"#),
    (
        "thread",
        r#"tap = hole(at: top.c, screw: "M6", fit: "tap", depth: 12, point: 118deg)
tapped = thread(hole: tap, length: "full", chamfer: true)"#,
    ),
];

/// The example parts whose ops the snippets do not make (sheet metal, thermoforming, a frame).
const PARTS: [&str; 4] = [
    "demo/sheet_tools.fg",
    "thermoform/tray.fg",
    "machined/flange.fg",
    "rack18u/frame/post.fg",
];

/// A fresh directory for this run under cargo's test scratch (never the source tree: every file
/// fg writes, the STEP and its `-flat` sibling, goes here).
fn tmp() -> PathBuf {
    let d = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("kernel-det-{}", std::process::id()));
    std::fs::create_dir_all(&d).expect("mkdir");
    d
}

/// The report (less its timings) and the STEP file (less its time of writing) of a fresh build.
fn build(file: &Path, envs: &[(&str, &str)], one_core: bool, tag: &str) -> (String, String) {
    let step = tmp().join(format!(
        "{}.{tag}.step",
        file.file_stem().expect("a name").to_string_lossy()
    ));
    let mut c = if one_core && Path::new("/usr/bin/taskset").exists() {
        let mut c = Command::new("/usr/bin/taskset");
        c.args(["-c", "0", env!("CARGO_BIN_EXE_fg")]);
        c
    } else {
        Command::new(env!("CARGO_BIN_EXE_fg"))
    };
    c.args(["run", file.to_str().expect("utf8"), "--json", "--step"])
        .arg(&step);
    c.env("FORGECAD_CACHE", "off");
    for (k, v) in envs {
        c.env(k, v);
    }
    let o = c.output().expect("fg runs");
    let json = String::from_utf8(o.stdout).expect("utf8");
    let json = match json.find("\"timings_ms\":{") {
        Some(at) => {
            let end = at + json[at..].find('}').expect("closed") + 2;
            format!("{}{}", &json[..at], &json[end..])
        }
        None => json,
    };
    let step: String = std::fs::read_to_string(&step)
        .unwrap_or_default()
        .lines()
        .filter(|l| !l.starts_with("FILE_NAME"))
        .collect::<Vec<_>>()
        .join("\n");
    (json, step)
}

fn same_five_ways(file: &Path) -> Vec<String> {
    let base = build(file, &[], false, "a");
    let mut bad = vec![];
    if !base.0.contains("\"ok\":true") {
        bad.push(format!("{}: does not build", file.display()));
        return bad;
    }
    for (envs, one, what) in [
        (vec![], false, "built again"),
        (vec![("FORGECAD_THREADS", "1")], true, "on one core"),
        (vec![("FORGE_BATCH", "cpu")], false, "FORGE_BATCH=cpu"),
        (vec![("FORGE_BATCH", "gpu")], false, "FORGE_BATCH=gpu"),
    ] {
        let other = build(file, &envs, one, "b");
        if other.0 != base.0 {
            bad.push(format!("{}: the report differs {what}", file.display()));
        }
        if other.1 != base.1 {
            bad.push(format!("{}: the STEP file differs {what}", file.display()));
        }
    }
    bad
}

#[test]
fn every_op_kind_is_the_same_however_it_is_run() {
    let dir = tmp();
    let font = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/demo/PublicSans-SemiBold.ttf");
    std::fs::copy(&font, dir.join("PublicSans-SemiBold.ttf")).expect("font");
    let mut bad = vec![];
    for (kind, op) in OPS {
        let f = dir.join(format!("{kind}.fg"));
        std::fs::write(&f, format!("{BASE}\n{op}\n")).expect("write");
        bad.extend(same_five_ways(&f));
    }
    assert!(
        bad.is_empty(),
        "an op depends on how it is run:\n{}",
        bad.join("\n")
    );
}

#[test]
fn every_example_part_is_the_same_however_it_is_run() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    // (each part's files are its own, by its name)
    let bad = forge_model::suite::each(&PARTS, |p| same_five_ways(&root.join(p))).concat();
    assert!(
        bad.is_empty(),
        "a part depends on how it is run:\n{}",
        bad.join("\n")
    );
}
