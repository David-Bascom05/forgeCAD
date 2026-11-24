//! The machining document end to end: built, certified on the posted path, and the refusals
//! typed with fixes.
use std::path::Path;

fn doc(body: &str) -> String {
    format!(
        "machining\nf = part(\"flange.fg\", group: \"N\")\nm = machine(\"generic-vmc3\")\nt1 = tool(\"flat-10\", t: 1, stickout: 32, holder: \"bt40-er32\")\nt2 = tool(\"flat-6\", t: 2, stickout: 26, holder: \"bt40-er32\")\nop1 = setup(f, machine: m, stock: box(margin: (3, 3, 2), below: 3), origin: \"top-center\", tol: 0.01)\n{body}"
    )
}

#[test]
fn the_flange_program_is_certified() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/machined");
    let src = std::fs::read_to_string(dir.join("flange_cam.fg")).unwrap();
    let cam = forge_cam_lang::run(&src, &dir);
    assert!(
        cam.ok,
        "{:?} {:?}",
        cam.errors,
        cam.ops.iter().map(|o| (&o.name, &o.refusal)).collect::<Vec<_>>()
    );
    assert_eq!(cam.ops.len(), 5);
    assert!(cam.ops.iter().all(|o| o.status == "built" && o.pairs > 0));
    let (txt, post) = cam.program_text().unwrap();
    assert_eq!(post, "fanuc");
    assert!(
        txt.contains("G98 G83") || txt.contains("G98 G81"),
        "the bolt holes are a canned cycle"
    );
    let mut j = String::new();
    cam.json().write(&mut j);
    assert!(j.contains("\"schema\":\"forgecad.cam/1\""));
    assert!(cam.sheet().contains("certified"));
    // G4: every band decided; nothing left over but the stock the part rests on (below z = 0:
    // a second setup faces it off)
    assert!(cam.leftover_done, "{:?}", cam.leftover);
    assert!(cam.leftover.iter().all(|l| l.z1 <= 0.0), "{:?}", cam.leftover);
    // placed on the machine (`at:`): every position within the travel
    assert_eq!(cam.certification(), "full");
    // not placed: released, but not called fully certified, and the warning names the fix
    let cam = forge_cam_lang::run(
        &doc(
            "skim = facing(op1, tool: t1, to: 18, step: 1)\nprog = program(op1, post: \"fanuc\", number: 1)\n",
        ),
        &dir,
    );
    assert_eq!(cam.certification(), "except G5");
    let mut j = String::new();
    cam.json().write(&mut j);
    assert!(
        j.contains("\"certified\":\"except G5 (unchecked: no `at:`)\""),
        "{j}"
    );
    assert!(
        cam.warnings
            .iter()
            .any(|w| w.contains("setup `op1` is not placed")
                && w.contains("fix: add `at: (x, y, z)`")
                && w.contains("G54")),
        "{:?}",
        cam.warnings
    );
}

/// With no workholding declared, whatever holds the stock is under it: a path below the
/// stock's bottom is refused (G2, the support), with a fix. CAM-SIM's machine simulation found
/// the flange profile (to z -0.5, stock bottom z 0) cutting into the table.
#[test]
fn nothing_cuts_below_the_stock_without_workholding() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/machined");
    let src = std::fs::read_to_string(dir.join("flange_cam.fg")).unwrap();
    let cam = forge_cam_lang::run(&src.replace(", below: 3)", ")"), &dir);
    assert!(!cam.ok && cam.program_text().is_none());
    let o = cam.ops.iter().find(|o| o.name == "prof").unwrap();
    let c = o
        .checks
        .iter()
        .find(|c| c.rule == "fixture_collision")
        .expect("the support check");
    assert!(c.other.contains("support under the stock"), "{c:?}");
    assert!(
        o.fix.as_deref().is_some_and(|f| f.contains("below the stock")),
        "{:?}",
        o.fix
    );
    // with stock under the part, released
    assert!(forge_cam_lang::run(&src, &dir).ok);
}

#[test]
fn leftover_is_listed() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/machined");
    // facing alone: the plate's sides, the bore and the holes are all still stock
    let cam = forge_cam_lang::run(&doc("skim = facing(op1, tool: t1, to: 18, step: 1)\n"), &dir);
    assert!(cam.leftover_done);
    assert!(!cam.leftover.is_empty());
    assert!(
        cam.leftover.iter().any(|l| l.z0 < 12.0 && l.z1 > 0.0),
        "{:?}",
        cam.leftover
    );
}

/// G4's reasons: below every tool tip (the facing's floor), and tool radius (the notch's inside
/// corners, which no disk of the chain's smallest radius reaches); the rest says "uncut".
#[test]
fn leftover_says_why() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/machined");
    let cam = forge_cam_lang::run(
        &doc(
            "skim = facing(op1, tool: t1, to: 18, step: 1)\nprog = program(op1, post: \"fanuc\", number: 1)\n",
        ),
        &dir,
    );
    assert!(cam.leftover_done && !cam.leftover.is_empty());
    // the facing's lowest tip is its floor, 18: everything under it is below every tool tip
    for l in &cam.leftover {
        // (the read-back tip: the posted 18 back in the part's frame, to f64)
        let d = l.below.expect("the lowest tip");
        assert!(
            l.why == "below every tool tip" && (d - 18.0).abs() < 1e-9 && l.z1 <= d,
            "{l:?}"
        );
    }
    // unposted: no tip is known, so no such claim
    let cam = forge_cam_lang::run(&doc("skim = facing(op1, tool: t1, to: 18, step: 1)\n"), &dir);
    assert!(
        cam.leftover.iter().all(|l| l.below.is_none()),
        "{:?}",
        cam.leftover
    );
    // and the bolt holes (6.6 mm) and the bore take no disk of the facing tool's radius, 5
    assert!(
        cam.leftover
            .iter()
            .any(|l| l.why == "tool radius" && l.r == Some(5.0)),
        "{:?}",
        cam.leftover
    );
    // the bracket: op2 (turned over) clears with the 10 mm tool; the chain's smallest is the
    // 6 mm (op1's bore). The notch's corners (x 28 and 42, y −38 turned) are out of reach of a
    // radius-3 disk; the rest of the notch's leftover a 6 mm tool could reach: uncut
    let cam = bracket(&dir, &|s| s);
    assert!(cam.ok && cam.leftover_done);
    let tr: Vec<_> = cam.leftover.iter().filter(|l| l.why == "tool radius").collect();
    assert!(!tr.is_empty(), "{:?}", cam.leftover);
    for l in &tr {
        assert_eq!(l.r, Some(3.0));
        assert!(l.z0 >= 6.0 - 1e-9, "{l:?}");
        for b in &l.boxes {
            // each corner's box (its arc's circle box, a superset) touches a notch corner
            let near = |x: f64, y: f64| b[0] <= x && x <= b[2] && b[1] <= y && y <= b[3];
            assert!(near(28.0, -38.0) || near(42.0, -38.0), "{b:?}");
        }
    }
    assert!(
        cam.leftover
            .iter()
            .any(|l| l.why == "uncut" && l.z0 >= 6.0 - 1e-9)
    );
    let mut j = String::new();
    cam.json().write(&mut j);
    assert!(j.contains("\"why\":\"tool radius\",\"r\":3"), "{j}");
}

#[test]
fn refusals_name_the_check_and_a_fix() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/machined");
    // the bore with a stick-out so short the holder's nose dips below the stock's top
    let cam = forge_cam_lang::run(
        &doc(
            "t3 = tool(\"flat-6\", t: 3, stickout: 19, holder: \"bt40-er32\")\nbore = pocket(op1, f.bore_h, tool: t3, stepover: 2.5, step: 3)\nprog = program(op1, post: \"haas\")\n",
        ),
        &dir,
    );
    assert!(!cam.ok);
    let o = &cam.ops[0];
    assert_eq!(o.status, "refused");
    assert!(
        o.checks.iter().any(|c| c.rule == "holder_collision"),
        "{:?}",
        o.checks
    );
    assert!(o.fix.is_some());
    assert!(
        cam.program_text().is_none(),
        "no program is released with a refused op"
    );
    // the bolt holes before the plate top is cleared: the rapids would enter stock
    let cam = forge_cam_lang::run(
        &doc(
            "skim = facing(op1, tool: t1, to: 18, step: 1)\nbolts = drill(op1, f.bolts, plan: \"auto\")\nprog = program(op1)\n",
        ),
        &dir,
    );
    let b = cam.ops.iter().find(|o| o.name == "bolts").unwrap();
    assert_eq!(b.status, "refused", "{:?}", b.checks);
    assert!(b.checks.iter().any(|c| c.rule == "rapid_clear"));
    // a typed error for an unknown name
    let cam = forge_cam_lang::run(&doc("x = drill(op1, f.nope)\n"), &dir);
    assert!(
        cam.errors.iter().any(|e| e.contains("no hole feature `nope`")),
        "{:?}",
        cam.errors
    );
}

#[test]
fn fillets_and_chamfers_are_lowered_and_a_gouge_into_a_fillet_is_refused() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/machined");
    let src = "machining\nf = part(\"boss_fillet.fg\", group: \"N\")\nt1 = tool(\"flat-6\", t: 1, stickout: 26, holder: \"bt40-er32\")\nop1 = setup(f, stock: box(margin: (2, 2, 1)), origin: \"top-center\")\nskim = facing(op1, tool: t1, to: 18, step: 1)\nboss = contour(op1, tool: t1, side: \"outside\", from: 18, to: 10.5, step: 4)\nprog = program(op1, post: \"linuxcnc\")\n";
    let cam = forge_cam_lang::run(src, &dir);
    assert!(cam.errors.is_empty(), "{:?}", cam.errors);
    let b = cam.ops.iter().find(|o| o.name == "boss").unwrap();
    // the contour is made from the section above the fillet: at its foot the tool meets the
    // fillet (a torus face), a real gouge
    assert_eq!(b.status, "refused", "{:?}", b.checks);
    assert!(b.checks.iter().any(|c| c.rule == "gouge"), "{:?}", b.checks);
}

#[test]
fn a_pocket_of_every_hole_of_the_section() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/machined");
    let cam = forge_cam_lang::run(
        &doc(
            "skim = facing(op1, tool: t1, to: 18, step: 1)\ntop = clear(op1, tool: t1, from: 18, to: 12, step: 3, stepover: 4)\nholes = pocket(op1, tool: t2, from: 12, to: -0.5, stepover: 2, step: 4)\nprog = program(op1)\n",
        ),
        &dir,
    );
    let o = cam.ops.iter().find(|o| o.name == "holes").unwrap();
    // the bore and the four bolt holes, each its own pocket
    assert_eq!(o.paths.len(), 5, "{:?}", o.refusal);
    assert_eq!(o.status, "built", "{:?}", o.checks);
}

fn vised(clamp: f64, body: &str) -> String {
    format!(
        "machining\nf = part(\"flange.fg\", group: \"N\")\nm = machine(\"generic-vmc3\")\nt1 = tool(\"flat-10\", t: 1, stickout: 32, holder: \"bt40-er32\")\nop1 = setup(f, machine: m, stock: box(margin: (3, 3, 2)), origin: \"top-center\", tol: 0.01, fixture: vise(\"generic-vise-150\", across: \"x\", clamp: {clamp}, parallel: 6, clearance: 1))\n{body}prog = program(op1, post: \"fanuc\")\n"
    )
}

#[test]
fn the_vise_is_checked_by_its_clearance() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/machined");
    // clearing the plate's top down to z 12 runs out to the stock's faces, where the jaws are:
    // jaws to z 10.9 keep the 1 mm clearance under the cutter's tip, jaws to 11.1 do not
    let body = "skim = facing(op1, tool: t1, to: 18, step: 1)\ntop = clear(op1, tool: t1, from: 18, to: 12, step: 3, stepover: 4)\n";
    let cam = forge_cam_lang::run(&vised(10.9, body), &dir);
    assert!(
        cam.ok,
        "{:?} {:?}",
        cam.errors,
        cam.ops.iter().map(|o| (&o.name, &o.refusal)).collect::<Vec<_>>()
    );
    let cam = forge_cam_lang::run(&vised(11.1, body), &dir);
    assert!(!cam.ok);
    let o = cam.ops.iter().find(|o| o.name == "top").unwrap();
    assert_eq!(o.status, "refused");
    let c = o
        .checks
        .iter()
        .find(|c| c.rule == "fixture_collision")
        .expect("a fixture check");
    assert!(c.other.contains("jaw"), "{}", c.other);
    let fix = o.fix.as_deref().unwrap();
    assert!(fix.starts_with("SetArg clamp: grip at most 11.00"), "{fix}");
    assert!(cam.program_text().is_none());
    // the facing passes overhang the stock: with the jaws gripping its full height, they would
    // cut the jaws
    let cam = forge_cam_lang::run(
        &vised(20.0, "skim = facing(op1, tool: t1, to: 18, step: 1)\n"),
        &dir,
    );
    assert!(!cam.ok);
    assert!(cam.ops[0].checks.iter().any(|c| c.rule == "fixture_collision"));
    // the profile goes through the plate: the jaws hold what it would cut; split the setup
    let cam = forge_cam_lang::run(
        &vised(
            5.0,
            "skim = facing(op1, tool: t1, to: 18, step: 1)\ntop = clear(op1, tool: t1, from: 18, to: 12, step: 3, stepover: 4)\nprof = contour(op1, tool: t1, side: \"outside\", from: 12, to: -0.5, step: 4)\n",
        ),
        &dir,
    );
    let o = cam.ops.iter().find(|o| o.name == "prof").unwrap();
    assert_eq!(o.status, "refused");
    assert!(o.fix.as_deref().unwrap().contains("second setup"), "{:?}", o.fix);
    // a clamp deeper than the stock is refused at the setup
    let cam = forge_cam_lang::run(&vised(25.0, ""), &dir);
    assert!(
        cam.errors.iter().any(|e| e.contains("clamp depth")),
        "{:?}",
        cam.errors
    );
}

#[test]
fn rapids_across_the_jaws_keep_the_clearance() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/machined");
    // jaws flush with the stock's top; the bore's rapids at 0.5 above it come within 1 mm
    let doc = |retract: f64| {
        format!(
            "machining\nf = part(\"flange.fg\", group: \"N\")\nm = machine(\"generic-vmc3\")\nt1 = tool(\"flat-10\", t: 1, stickout: 32, holder: \"bt40-er32\")\nop1 = setup(f, machine: m, stock: box(margin: (3, 3, 2)), origin: \"top-center\", tol: 0.01, clear: {retract}, retract: {retract}, fixture: vise(\"generic-vise-150\", across: \"x\", clamp: 20, clearance: 1))\nskim = facing(op1, tool: t1, to: 19.95, step: 1)\nprog = program(op1, post: \"fanuc\")\n"
        )
    };
    let cam = forge_cam_lang::run(&doc(0.5), &dir);
    assert!(!cam.ok);
    assert!(
        cam.ops[0]
            .checks
            .iter()
            .any(|c| c.rule == "fixture_collision" && c.phase == "rapid"),
        "{:?}",
        cam.ops[0].checks
    );
}

#[test]
fn what_the_jaws_hold_is_left_over_by_name() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/machined");
    let cam = forge_cam_lang::run(
        &vised(
            5.0,
            "skim = facing(op1, tool: t1, to: 18, step: 1)\ntop = clear(op1, tool: t1, from: 18, to: 12, step: 3, stepover: 4)\nprof = contour(op1, tool: t1, side: \"outside\", from: 12, to: 6.5, step: 3)\n",
        ),
        &dir,
    );
    assert!(
        cam.ok,
        "{:?}",
        cam.ops.iter().map(|o| (&o.name, &o.refusal)).collect::<Vec<_>>()
    );
    assert!(cam.leftover_done);
    assert!(
        cam.leftover
            .iter()
            .any(|l| l.why == "held by fixture" && l.z1 <= 5.0)
    );
    assert!(
        cam.leftover
            .iter()
            .all(|l| l.why != "held by fixture" || l.z1 <= 5.0)
    );
    let mut j = String::new();
    cam.json().write(&mut j);
    assert!(
        j.contains("\"why\":\"held by fixture\"") && j.contains("\"jaw_top\":5"),
        "{j}"
    );
    assert!(cam.sheet().contains("Held in the vise"));
}

fn bracket(dir: &Path, edit: &dyn Fn(String) -> String) -> forge_cam_lang::Cam {
    let src = std::fs::read_to_string(dir.join("bracket_cam.fg")).unwrap();
    forge_cam_lang::run(&edit(src), dir)
}

fn fixture_json(cam: &forge_cam_lang::Cam, setup: &str) -> String {
    let mut j = String::new();
    cam.json().write(&mut j);
    let at = j.find(&format!("\"name\":\"{setup}\"")).unwrap();
    let f = at + j[at..].find("\"fixture\"").unwrap();
    j[f..f + j[f..].find("\"heights\"").unwrap()].to_string()
}

#[test]
fn soft_jaws_hold_the_part_turned_over() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/machined");
    let cam = bracket(&dir, &|s| s);
    assert!(
        cam.ok,
        "{:?} {:?}",
        cam.errors,
        cam.ops.iter().map(|o| (&o.name, &o.refusal)).collect::<Vec<_>>()
    );
    assert_eq!(cam.programs.len(), 3);
    assert!(cam.programs.iter().all(|p| p.error.is_none() && p.round_trip));
    // every setup placed: G5 on every position, certified in full
    assert_eq!(cam.certification(), "full");
    // the jaws are cut in their own setup, and op2's ops are checked against them
    let rest = cam.ops.iter().find(|o| o.name == "rest").unwrap();
    assert!(rest.status == "built" && rest.pairs > 0);
    assert!(fixture_json(&cam, "op2").contains("soft jaw"));
    // G4 at the chain's end: op1 is continued by op2; the jaws are cut clean; what is left is
    // the notch's two sharp inside corners, which no end mill reaches
    assert!(cam.leftover_done);
    // setups in order: op1, opj, op2
    assert!(cam.leftover.iter().all(|l| l.setup == 2), "{:?}", cam.leftover);
    for l in &cam.leftover {
        assert_eq!(l.boxes.len(), 2);
        for b in &l.boxes {
            // in the notch (x 28..42, y −45..−38 turned over), each about a corner
            assert!(
                b[0] > 27.0 && b[2] < 43.0 && b[1] > -49.0 && b[3] < -37.0,
                "{b:?}"
            );
        }
    }
}

#[test]
fn soft_jaws_follow_the_part() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/machined");
    // a wider notch in a copy of the part: the jaws' pocket is cut to the new outline
    let tmp = forge_dirs::temp_dir().join(format!("forge-cam-jaws-{}", forge_dirs::process_id()));
    std::fs::create_dir_all(&tmp).unwrap();
    let part = std::fs::read_to_string(dir.join("bracket.fg")).unwrap();
    std::fs::write(
        tmp.join("bracket.fg"),
        part.replace("shared notch = 14", "shared notch = 20"),
    )
    .unwrap();
    std::fs::copy(dir.join("bracket_cam.fg"), tmp.join("bracket_cam.fg")).unwrap();
    let before = bracket(&dir, &|s| s);
    let after = bracket(&tmp, &|s| s);
    let _ = std::fs::remove_dir_all(&tmp);
    assert!(
        after.ok,
        "{:?} {:?}",
        after.errors,
        after
            .ops
            .iter()
            .map(|o| (&o.name, &o.refusal))
            .collect::<Vec<_>>()
    );
    // the jaws' pocket follows the outline: the op that cuts it changes
    let cut = |c: &forge_cam_lang::Cam| c.ops.iter().find(|o| o.name == "cutj").unwrap().length_cut;
    assert!((cut(&before) - cut(&after)).abs() > 1.0);
}

#[test]
fn soft_jaws_refuse_what_cannot_seat() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/machined");
    // no reliefs: the jaw tool's radius rounds the pocket's corners, and the part's sharp ones
    // do not go in
    let cam = bracket(&dir, &|s| s.replace(", relief: \"corners\"", ""));
    assert!(
        cam.errors
            .iter()
            .any(|e| e.contains("does not seat") && e.contains("relief")),
        "{:?}",
        cam.errors
    );
    // a grip as deep as the part
    let cam = bracket(&dir, &|s| s.replace("grip: 6", "grip: 14"));
    assert!(
        cam.errors.iter().any(|e| e.contains("grip depth")),
        "{:?}",
        cam.errors
    );
    // a hole is drilled from the side it opens on, not from the turned setup
    let cam = bracket(&dir, &|s| s + "late = drill(op2, f.bore_h, plan: \"auto\")\n");
    let late = cam.ops.iter().find(|o| o.name == "late").unwrap();
    assert_eq!(late.status, "refused");
    assert!(
        late.refusal.as_deref().unwrap().contains("side it opens on"),
        "{:?}",
        late.refusal
    );
}

#[test]
fn soft_jaws_need_the_top_faced() {
    // op1 not faced to the part's top: the stock above it would sit in the jaws below the seat
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/machined");
    let cam = bracket(&dir, &|s| {
        s.replace("skim = facing(op1, tool: t1, to: 14, step: 1)\n", "")
    });
    assert!(
        cam.errors
            .iter()
            .any(|e| e.contains("above the part's top") && e.contains("facing")),
        "{:?}",
        cam.errors
    );
}

#[test]
fn helical_ramps_are_posted_and_certified() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/machined");
    let cam = bracket(&dir, &|s| s);
    assert!(
        cam.ok,
        "{:?}",
        cam.ops.iter().map(|o| (&o.name, &o.refusal)).collect::<Vec<_>>()
    );
    for name in ["bore", "cutj"] {
        let o = cam.ops.iter().find(|o| o.name == name).unwrap();
        assert_eq!(o.status, "built");
        // helical pieces: arcs whose height changes
        let mut at = o.paths[0].start;
        let mut helical = 0;
        for m in &o.paths[0].moves {
            match *m {
                forge_cam::ops::PMove::Arc { to, .. } => {
                    if to[2] != at[2] {
                        helical += 1;
                    }
                    at = to;
                }
                forge_cam::ops::PMove::Rapid { to } | forge_cam::ops::PMove::Line { to, .. } => at = to,
                _ => {}
            }
        }
        assert!(helical > 0, "{name}");
    }
    // in the program text: G3 blocks with a Z word
    let txt = cam
        .programs
        .iter()
        .find(|p| p.name == "p1")
        .unwrap()
        .text
        .clone()
        .unwrap();
    assert!(txt.lines().any(|l| l.contains("G3") && l.contains(" Z")), "{txt}");
}

#[test]
fn engraving_follows_the_sketch_into_the_modelled_groove() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/machined");
    let src = std::fs::read_to_string(dir.join("plate_mark_cam.fg")).unwrap();
    let cam = forge_cam_lang::run(&src, &dir);
    assert!(
        cam.ok,
        "{:?} {:?}",
        cam.errors,
        cam.ops.iter().map(|o| (&o.name, &o.refusal)).collect::<Vec<_>>()
    );
    let o = cam.ops.iter().find(|o| o.name == "mark").unwrap();
    assert_eq!((o.kind, o.status), ("engrave", "built"));
    // two levels along the line, down to 0.3 below the top
    let low = o.paths[0]
        .moves
        .iter()
        .filter_map(|m| {
            if let forge_cam::ops::PMove::Line { to, .. } = m {
                Some(to[2])
            } else {
                None
            }
        })
        .fold(f64::INFINITY, f64::min);
    assert!((low - 4.7).abs() < 1e-9, "{low}");
    // the line to the groove's square ends: the V tool's width at depth cuts the end walls
    let tmp = forge_dirs::temp_dir().join(format!("forge-cam-mark-{}", forge_dirs::process_id()));
    std::fs::create_dir_all(&tmp).unwrap();
    let part = std::fs::read_to_string(dir.join("plate_mark.fg")).unwrap();
    std::fs::write(
        tmp.join("plate_mark.fg"),
        part.replace(
            "line((m + groove_w / 2, W / 2), (L - m - groove_w / 2, W / 2))",
            "line((m, W / 2), (L - m, W / 2))",
        ),
    )
    .unwrap();
    let cam = forge_cam_lang::run(&src, &tmp);
    let _ = std::fs::remove_dir_all(&tmp);
    let o = cam.ops.iter().find(|o| o.name == "mark").unwrap();
    assert_eq!(o.status, "refused");
    assert!(o.checks.iter().any(|c| c.rule == "gouge"), "{:?}", o.checks);
}

#[test]
fn a_thread_milled_as_a_declared_cosmetic_thread() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/machined");
    let src = std::fs::read_to_string(dir.join("block_m6_cam.fg")).unwrap();
    let cam = forge_cam_lang::run(&src, &dir);
    assert!(
        cam.ok,
        "{:?} {:?}",
        cam.errors,
        cam.ops.iter().map(|o| (&o.name, &o.refusal)).collect::<Vec<_>>()
    );
    // the drill plan leaves the thread: no tap
    let d = cam.ops.iter().find(|o| o.name == "tapd").unwrap();
    assert!(d.plan.iter().all(|p| !p.starts_with("tap")), "{:?}", d.plan);
    // the thread mill declares its Δ and climbs a helix of the pitch
    let t = cam.ops.iter().find(|o| o.name == "thr").unwrap();
    assert_eq!(t.removals.len(), 1);
    assert!(
        t.removals[0].starts_with("cosmetic_thread M6x1"),
        "{}",
        t.removals[0]
    );
    let txt = cam.program_text().unwrap().0;
    assert!(txt.lines().any(|l| l.contains("G3") && l.contains(" Z")), "{txt}");
    let mut j = String::new();
    cam.json().write(&mut j);
    assert!(j.contains("\"removals\":[\"cosmetic_thread M6x1"), "{j}");
    // a hole that records no thread: nothing to declare, so no thread milling
    let tmp = forge_dirs::temp_dir().join(format!("forge-cam-m6-{}", forge_dirs::process_id()));
    std::fs::create_dir_all(&tmp).unwrap();
    let part = std::fs::read_to_string(dir.join("block_m6.fg")).unwrap();
    std::fs::write(
        tmp.join("block_m6.fg"),
        part.replace("screw: \"M6\", fit: \"tap\"", "d: 5"),
    )
    .unwrap();
    let cam = forge_cam_lang::run(&src, &tmp);
    let _ = std::fs::remove_dir_all(&tmp);
    let t = cam.ops.iter().find(|o| o.name == "thr").unwrap();
    assert_eq!(t.status, "refused");
    assert!(
        t.refusal.as_deref().unwrap().contains("records no thread"),
        "{:?}",
        t.refusal
    );
}

#[test]
fn every_posted_position_is_within_the_machines_travel() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/machined");
    let src = std::fs::read_to_string(dir.join("flange_cam.fg")).unwrap();
    let placed = |at: &str| {
        // (the example is placed at (0, 0, -250): moved for each case)
        src.replace(
            "origin: \"top-center\", tol: tol, at: (0, 0, -250))",
            &format!("origin: \"top-center\", tol: tol, at: {at})"),
        )
    };
    // generic-vmc3: X −300..300, Y −200..200, spindle Z −400..0. A work offset in the middle of
    // the table, its Z low enough that the tools' lengths stay under 0
    let cam = forge_cam_lang::run(&placed("(0, 0, -250)"), &dir);
    assert!(
        cam.ok,
        "{:?}",
        cam.ops.iter().map(|o| (&o.name, &o.refusal)).collect::<Vec<_>>()
    );
    assert!(
        cam.warnings.iter().all(|w| !w.contains("not placed")),
        "{:?}",
        cam.warnings
    );
    // near the end of X: the profile's far side runs off the travel
    let cam = forge_cam_lang::run(&placed("(280, 0, -250)"), &dir);
    assert!(!cam.ok);
    let o = cam
        .ops
        .iter()
        .find(|o| o.checks.iter().any(|c| c.rule == "axis_limit"))
        .expect("an axis limit");
    assert!(
        o.checks.iter().any(|c| c.other.contains("joint X")),
        "{:?}",
        o.checks
    );
    assert!(cam.program_text().is_none());
    // the spindle above its top: Z offset too high for the tool's length
    let cam = forge_cam_lang::run(&placed("(0, 0, -20)"), &dir);
    assert!(cam.ops.iter().any(|o| {
        o.checks
            .iter()
            .any(|c| c.rule == "axis_limit" && c.other.contains("joint Z"))
    }));
    // unplaced: the spans are checked, and the reply says the positions were not
    let cam = forge_cam_lang::run(&src.replace(", at: (0, 0, -250))", ")"), &dir);
    assert!(cam.ok);
    assert!(
        cam.warnings.iter().any(|w| w.contains("not placed")),
        "{:?}",
        cam.warnings
    );
}

#[test]
fn edge_break_certifies_the_chamfer_and_refuses_what_it_cannot_cut() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/machined");
    let src = std::fs::read_to_string(dir.join("plate_break_cam.fg")).unwrap();
    let t0 = std::time::Instant::now();
    let cam = forge_cam_lang::run(&src, &dir);
    let dt = t0.elapsed().as_secs_f64();
    assert!(
        cam.ok,
        "{:?} {:?}",
        cam.errors,
        cam.ops.iter().map(|o| (&o.name, &o.refusal)).collect::<Vec<_>>()
    );
    // the 0.3 mm break with chamfer-10-90: declared, certified, within budget (the closed forms
    // by symmetry: 105,228 cells and 0.5 s for the whole program where the branch and bound
    // alone took 644,450 cells and 113 s for the break)
    let b = cam.ops.iter().find(|o| o.name == "brk").unwrap();
    assert_eq!(b.status, "built");
    assert_eq!(b.removals.len(), 1);
    assert!(
        b.removals[0].starts_with("edge_break 0.3 x 45°"),
        "{}",
        b.removals[0]
    );
    assert!(b.cells < 200_000, "{} cells", b.cells);
    assert!(dt < 30.0, "{dt} s");
    // refused: a break the tool's flank cannot reach, and a tool that is not a 90° chamfer mill
    for (edit, why) in [
        ("tool: t7, size: 0.3)", "tool: t7, size: 4.5)"),
        ("tool: t7, size: 0.3)", "tool: t1, size: 0.3)"),
    ] {
        let s = src.replace(edit, why);
        let cam = forge_cam_lang::run(&s, &dir);
        let b = cam.ops.iter().find(|o| o.name == "brk").unwrap();
        assert_eq!(b.status, "refused", "{why}");
        assert!(b.fix.is_some(), "{why}: {:?}", b.refusal);
    }
}

/// A pocket of the section keeps the islands standing in it: well_island.fg's well has a boss in it, to
/// the plate's top. The pocket's region is the well less the part's section, so the passes go
/// round the boss (certified: no gouge), and G4 leaves only the well's four corners (a 6 mm
/// tool) besides the stock outside the plate, which nothing here cuts.
#[test]
fn a_pocket_keeps_its_islands() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/machined");
    let src = std::fs::read_to_string(dir.join("well_island_cam.fg")).unwrap();
    let cam = forge_cam_lang::run(&src, &dir);
    assert!(
        cam.ok,
        "{:?}",
        cam.ops.iter().map(|o| (&o.name, &o.refusal)).collect::<Vec<_>>()
    );
    let well = cam.ops.iter().find(|o| o.name == "well").unwrap();
    assert!(well.status == "built" && well.pairs > 0);
    // the passes run on both sides of the boss (x 25 to 35) at its height
    let xs: Vec<f64> = well.paths[0]
        .moves
        .iter()
        .filter_map(|m| match m {
            forge_cam::ops::PMove::Line { to, .. } | forge_cam::ops::PMove::Arc { to, .. }
                if (to[1] - 20.0).abs() < 4.0 && to[2] < 12.0 =>
            {
                Some(to[0])
            }
            _ => None,
        })
        .collect();
    assert!(
        xs.iter().any(|x| *x < 25.0) && xs.iter().any(|x| *x > 35.0),
        "{xs:?}"
    );
    assert!(cam.leftover_done);
    let corners = [[8.0, 8.0], [52.0, 8.0], [52.0, 32.0], [8.0, 32.0]];
    for l in cam.leftover.iter().filter(|l| l.z0 >= 6.0 - 1e-9) {
        for b in &l.boxes {
            // the stock's ring outside the plate, or a corner of the well (a box an arc's
            // circle gives: of radius 3, or 3 + 1e-6 for G4's tool-radius piece)
            let ring = b[0] <= 0.0 && b[2] >= 60.0;
            let corner = corners.iter().any(|c| {
                (b[0] - c[0]).abs().min((b[2] - c[0]).abs()) < 1e-5
                    && (b[1] - c[1]).abs().min((b[3] - c[1]).abs()) < 1e-5
                    && b[2] - b[0] <= 6.0 + 1e-5
            });
            assert!(ring || corner, "{b:?}");
        }
    }
}

/// `slot`: an end mill along a sketch's centrelines, the modelled groove its width, ramped in
/// along each line; certified (the cutter tangent to both walls within tol), and G4 finds the
/// grooves cleared. A pointed tool is refused (that is engraving).
#[test]
fn slots_follow_their_centrelines() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/machined");
    let src = std::fs::read_to_string(dir.join("slot_plate_cam.fg")).unwrap();
    let cam = forge_cam_lang::run(&src, &dir);
    assert!(
        cam.ok,
        "{:?}",
        cam.ops.iter().map(|o| (&o.name, &o.refusal)).collect::<Vec<_>>()
    );
    let op = cam.ops.iter().find(|o| o.name == "slots").unwrap();
    assert_eq!(op.kind, "slot");
    // ramped: moves that descend while they travel along the line
    let mut at = op.paths[0].start;
    let mut ramps = 0;
    for m in &op.paths[0].moves {
        if let forge_cam::ops::PMove::Line { to, .. } = m {
            if to[2] < at[2] && (to[0] != at[0] || to[1] != at[1]) {
                ramps += 1;
            }
            at = *to;
        } else if let forge_cam::ops::PMove::Rapid { to } | forge_cam::ops::PMove::Arc { to, .. } = m {
            at = *to;
        }
    }
    assert!(ramps >= 4, "{ramps}");
    // the grooves are cleared: in their band nothing is left inside the plate
    assert!(cam.leftover_done);
    for l in cam.leftover.iter().filter(|l| l.z1 > 6.0 + 1e-9) {
        for b in &l.boxes {
            assert!(b[0] <= 0.0 && b[2] >= 60.0, "inside the plate: {b:?} in {l:?}");
        }
    }
    // a pointed tool engraves; a slot takes an end mill
    let cam = forge_cam_lang::run(&src.replace("tool(\"flat-6\"", "tool(\"chamfer-10-90\""), &dir);
    let op = cam.ops.iter().find(|o| o.name == "slots").unwrap();
    assert!(
        op.refusal.as_deref().is_some_and(|r| r.contains("an end mill")),
        "{:?}",
        op.refusal
    );
}

/// G5's runtime half: the posted S and F, as read back, within the machine's spindle range and
/// feed limit (a machine of the project's own library, slower than the generic one).
#[test]
fn posted_speeds_and_feeds_are_within_the_machine() {
    let src_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/machined");
    let dir = forge_dirs::temp_dir().join(format!("forge-cam-g5rt-{}", forge_dirs::process_id()));
    std::fs::create_dir_all(dir.join("cam")).unwrap();
    std::fs::copy(src_dir.join("flange.fg"), dir.join("flange.fg")).unwrap();
    let machine = |rpm: &str, feed: &str| {
        let m = std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../kernel/forge-tooling/data/machines.toml"),
        )
        .unwrap();
        // the generic VMC's record, renamed, with its spindle range and feed limit replaced
        let vmc = m.split("[[machine]]").nth(1).unwrap();
        let rec = vmc
            .replace("\"generic-vmc3\"", "\"slow-vmc\"")
            .replace("rpm = [100, 12000]", &format!("rpm = {rpm}"))
            .replace("max_feed = 10000", &format!("max_feed = {feed}"));
        std::fs::write(dir.join("cam/slow.toml"), format!("[[machine]]{rec}")).unwrap();
    };
    let src = std::fs::read_to_string(src_dir.join("flange_cam.fg"))
        .unwrap()
        .replace("generic-vmc3", "slow-vmc");
    // the same limits as the generic VMC: released
    machine("[100, 12000]", "10000");
    let cam = forge_cam_lang::run(&src, &dir);
    assert!(
        cam.ok,
        "{:?} {:?}",
        cam.errors,
        cam.ops.iter().map(|o| &o.refusal).collect::<Vec<_>>()
    );
    // a feed limit under the profile's feed: refused by the posted F, with the limit
    machine("[100, 12000]", "1000");
    let cam = forge_cam_lang::run(&src, &dir);
    assert!(!cam.ok && cam.program_text().is_none());
    let c = cam
        .ops
        .iter()
        .flat_map(|o| o.checks.iter())
        .find(|c| c.rule == "machine_limit")
        .expect("a feed over the limit");
    assert!(c.value > 1000.0 && c.limit == 1000.0, "{c:?}");
    // a material group with no cutting data: the ops fall back to S3000, under this
    // spindle's least speed: refused by the posted S
    machine("[5000, 24000]", "10000");
    let cam = forge_cam_lang::run(&src.replace("group: \"N\"", "group: \"X\""), &dir);
    assert!(!cam.ok && cam.program_text().is_none());
    let c = cam
        .ops
        .iter()
        .flat_map(|o| o.checks.iter())
        .find(|c| c.rule == "machine_limit")
        .expect("a spindle speed under the range");
    assert!(
        c.value == 3000.0 && c.limit == 5000.0 && c.other.starts_with("spindle"),
        "{c:?}"
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// Cycle time counts arcs: each op's time is its moves' lengths at their feeds (rapids at
/// 10 000 mm/min), arcs by the turn they sweep. Worked out here from the planned path, its own
/// way; the posted path differs only by the dialect's rounding.
#[test]
fn cycle_time_counts_arcs() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/machined");
    let src = std::fs::read_to_string(dir.join("flange_cam.fg")).unwrap();
    let cam = forge_cam_lang::run(&src, &dir);
    assert!(cam.ok);
    let mut arcs_seen = 0;
    for op in cam.ops.iter().filter(|o| o.kind != "drill") {
        let (mut t, mut len) = (0.0, 0.0);
        for p in &op.paths {
            let mut at = p.start;
            for m in &p.moves {
                match *m {
                    forge_cam::ops::PMove::Rapid { to } => {
                        let l = ((to[0] - at[0]).powi(2) + (to[1] - at[1]).powi(2) + (to[2] - at[2]).powi(2))
                            .sqrt();
                        t += l / 10_000.0 * 60.0;
                        at = to;
                    }
                    forge_cam::ops::PMove::Line { to, f, .. } => {
                        let l = ((to[0] - at[0]).powi(2) + (to[1] - at[1]).powi(2) + (to[2] - at[2]).powi(2))
                            .sqrt();
                        t += l / f * 60.0;
                        len += l;
                        at = to;
                    }
                    forge_cam::ops::PMove::Arc { to, c, cw, f } => {
                        arcs_seen += 1;
                        let (u, v) = ([at[0] - c[0], at[1] - c[1]], [to[0] - c[0], to[1] - c[1]]);
                        let r = (u[0] * u[0] + u[1] * u[1]).sqrt();
                        // the turn from u to v in the arc's sense (a whole one if they agree)
                        let mut a =
                            forge_math::dmath::atan2(u[0] * v[1] - u[1] * v[0], u[0] * v[0] + u[1] * v[1]);
                        if cw {
                            a = -a;
                        }
                        if a <= 1e-12 {
                            a += std::f64::consts::TAU;
                        }
                        let l = ((r * a).powi(2) + (to[2] - at[2]).powi(2)).sqrt();
                        t += l / f * 60.0;
                        len += l;
                        at = to;
                    }
                    forge_cam::ops::PMove::Cycle { .. } => {}
                }
            }
        }
        assert!(
            (op.time_s - t).abs() <= 0.01 * t + 0.05,
            "{}: {} vs {t}",
            op.name,
            op.time_s
        );
        assert!(
            (op.length_cut - len).abs() <= 0.01 * len + 0.05,
            "{}: {} vs {len}",
            op.name,
            op.length_cut
        );
    }
    assert!(arcs_seen > 0);
}

/// Every tool change is preceded by Z to the machine's safe position (the dialect's
/// retract_home): an M06 need not move Z, and a longer tool would otherwise come down into the
/// stock where the last one stood (CAM-SIM's machine simulation of the flange, T12 at N3890).
#[test]
fn z_retracts_before_every_tool_change() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/machined");
    let src = std::fs::read_to_string(dir.join("flange_cam.fg")).unwrap();
    for (post, retract) in [("fanuc", "G28 Z0"), ("haas", "G28 Z0"), ("linuxcnc", "G53 G0 Z0")] {
        let cam = forge_cam_lang::run(
            &src.replace("post: \"fanuc\"", &format!("post: \"{post}\"")),
            &dir,
        );
        assert!(cam.ok, "{post}");
        let (txt, _) = cam.program_text().unwrap();
        let lines: Vec<&str> = txt.lines().collect();
        let mut changes = 0;
        for (i, l) in lines.iter().enumerate() {
            if l.contains(" M06") || l.ends_with(" M6") || l.contains(" M6 ") {
                changes += 1;
                // the retract within the two lines before (G28's G90 restore follows it)
                assert!(
                    lines[i.saturating_sub(2)..i].iter().any(|p| p.contains(retract)),
                    "{post}: no retract before {l}: {:?}",
                    &lines[i.saturating_sub(3)..i]
                );
            }
        }
        assert_eq!(changes, 3, "{post}: T1, T11, T12");
    }
}

/// GRBL changes tools by hand and has no machine-safe Z: before each change the tool is at the
/// setup's clearance height in the work coordinates (Z10 for the flange, top-centre origin), and
/// the program tells the operator to set Z for the new tool. The program posts (it did not:
/// the machine-home retract is refused on GRBL).
#[test]
fn grbl_tool_changes_stand_at_the_clearance_height() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/machined");
    let src = std::fs::read_to_string(dir.join("flange_cam.fg"))
        .unwrap()
        .replace("post: \"fanuc\"", "post: \"grbl\"");
    let cam = forge_cam_lang::run(&src, &dir);
    assert!(
        cam.ok,
        "{:?}",
        cam.programs.iter().map(|p| &p.error).collect::<Vec<_>>()
    );
    let (txt, _) = cam.program_text().unwrap();
    let lines: Vec<&str> = txt.lines().collect();
    let changes: Vec<usize> = (0..lines.len())
        .filter(|&i| lines[i].starts_with("(TOOL"))
        .collect();
    assert_eq!(changes.len(), 3);
    for &i in &changes[1..] {
        assert!(
            lines[..i]
                .iter()
                .rev()
                .take(4)
                .any(|l| l.contains("set Z zero for the new tool"))
        );
        // the last Z word before it: the clearance height
        let z = lines[..i]
            .iter()
            .rev()
            .filter(|l| !l.starts_with('('))
            .find_map(|l| l.split_whitespace().find_map(|w| w.strip_prefix('Z')))
            .unwrap();
        assert_eq!(
            z.parse::<f64>().unwrap(),
            10.0,
            "{:?}",
            &lines[i.saturating_sub(6)..=i]
        );
    }
}

/// The on-demand check runs the full checker over what was posted, without generating again:
/// every op of the flange clear, with the staged stock built for every path; one op by name.
#[test]
fn the_posted_paths_check_again_on_demand() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/machined");
    let src = std::fs::read_to_string(dir.join("flange_cam.fg")).unwrap();
    let cam = forge_cam_lang::run(&src, &dir);
    assert!(cam.ok);
    let all = cam.check(None);
    assert_eq!(all.len(), cam.ops.len());
    for (oc, o) in all.iter().zip(&cam.ops) {
        assert!(oc.clear && oc.checks.is_empty(), "{oc:?}");
        // at least the pairs the generate checked (the stock is never skipped here)
        assert!(oc.pairs >= o.pairs, "{} {} {}", oc.op, oc.pairs, o.pairs);
    }
    let one = cam.check(Some("bolts"));
    assert_eq!(one.len(), 1);
    assert!(one[0].clear);
    assert!(cam.check(Some("no-such-op")).is_empty());
}

/// A bare call after the header did nothing and reported nothing (LANG-PROOFS): it is refused,
/// with the named form as the fix.
#[test]
fn a_bare_operation_is_refused_with_its_named_form() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/machined");
    let cam = forge_cam_lang::run(&doc("facing(op1, tool: t1, to: 14, step: 1)\n"), &dir);
    assert!(!cam.ok);
    assert!(
        cam.errors
            .iter()
            .any(|e| e.contains("line 7") && e.contains("an operation needs a name: `facing1 = facing(..)`")),
        "{:?}",
        cam.errors
    );
    // the named form is taken
    let cam = forge_cam_lang::run(&doc("face1 = facing(op1, tool: t1, to: 14, step: 1)\n"), &dir);
    assert!(cam.errors.is_empty(), "{:?}", cam.errors);
    assert_eq!(cam.ops.len(), 1);
}

/// A tool made in the builder (a project record, not built in) reaches CAM through the library
/// the levels make: `run_with_library` takes it; `run` (the built-in library) does not know it.
#[test]
fn a_project_tool_is_used_through_the_leveled_library() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/machined");
    let body = "t9 = tool(\"shop-flat-8\", t: 9, stickout: 30, holder: \"bt40-er32\")\nface1 = facing(op1, tool: t9, to: 18, step: 1)\nprog = program(op1, post: \"fanuc\", number: 1)\n";
    let cam = forge_cam_lang::run(&doc(body), &dir);
    assert!(!cam.ok);
    assert!(
        cam.errors.iter().any(|e| e.contains("shop-flat-8")),
        "{:?}",
        cam.errors
    );
    // the library forge-tooling-builder's `library(project, user_cam)` returns: the built-in
    // records with the project's added
    let mut lib = forge_tooling::Library::builtin();
    lib.add(
        "project/cam/tools/shop.toml",
        "[[tool]]\nid = \"shop-flat-8\"\ntype = \"flat\"\ndc = 8.0\nnof = 3\nlcf = 20.0\ndcon = 8.0\noal = 63.0\nmaterial = \"carbide\"\nsource = \"shop\"\ntaken = \"2026-11-24\"\n",
    )
    .unwrap();
    let cam = forge_cam_lang::run_with_library(&doc(body), &dir, lib);
    assert!(cam.errors.is_empty(), "{:?}", cam.errors);
    assert_eq!(cam.ops.len(), 1);
    assert_eq!(cam.ops[0].status, "built", "{:?}", cam.ops[0].refusal);
    let (txt, _) = cam.program_text().unwrap();
    assert!(txt.contains("T9 M06"), "{txt}");
}

/// Axial stock to leave raises each floor by it (Fusion's "axial stock to leave"); `leave` stays
/// the radial stock, on walls. G4 grows the part by the larger of the two. An op without floors
/// refuses it.
#[test]
fn axial_stock_to_leave_raises_the_floors() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/machined");
    let deepest = |extra: &str| {
        let body = format!(
            "skim = facing(op1, tool: t1, to: 18, step: 1)\ntop = clear(op1, tool: t1, from: 18, to: 12, step: 3, stepover: 4{extra})\nprog = program(op1, post: \"fanuc\", number: 1)\n"
        );
        let cam = forge_cam_lang::run(&doc(&body), &dir);
        assert!(cam.errors.is_empty(), "{:?}", cam.errors);
        let o = cam.ops.iter().find(|o| o.name == "top").unwrap();
        assert_eq!(o.status, "built", "{:?}", o.refusal);
        (o.deepest, o.leave)
    };
    let (d0, l0) = deepest("");
    let (d1, l1) = deepest(", leave: 0.2, leave_axial: 0.5");
    assert!((d1 - d0 - 0.5).abs() < 1e-9, "{d0} {d1}");
    assert_eq!((l0, l1), (0.0, 0.5));
    let cam = forge_cam_lang::run(
        &doc("bolts = drill(op1, f.bolts, plan: \"auto\", leave_axial: 0.5)\n"),
        &dir,
    );
    assert!(
        cam.errors
            .iter()
            .any(|e| e.contains("drill does not take `leave_axial:`")),
        "{:?}",
        cam.errors
    );
}

/// A contour's direction and passes (Fusion parity): with M03 an outside contour is climb by
/// default, clockwise round the part (G2 at its corners); `direction: "conventional"` runs it
/// counter-clockwise (G3). `finish: n` radial passes `finish_step` apart, the last at the
/// contour's own offset, and `spring: n` repeats the last: each a loop at every level, all
/// certified.
#[test]
fn a_contour_takes_its_direction_and_passes() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/machined");
    let run = |extra: &str| {
        let body = format!(
            "skim = facing(op1, tool: t1, to: 18, step: 1)\ntop = clear(op1, tool: t1, from: 18, to: 12, step: 3, stepover: 4)\nprof = contour(op1, tool: t1, side: \"outside\", from: 12, to: -0.5, step: 4{extra})\nprog = program(op1, post: \"fanuc\", number: 1)\n"
        );
        let cam = forge_cam_lang::run(&doc(&body), &dir);
        assert!(cam.errors.is_empty(), "{:?}", cam.errors);
        let o = cam.ops.iter().find(|o| o.name == "prof").unwrap();
        assert_eq!(o.status, "built", "{:?}", o.refusal);
        let (txt, _) = cam.program_text().unwrap();
        let prof: String = txt.split("(prof contour)").nth(1).unwrap().to_string();
        let plunges = o.paths[0]
            .moves
            .iter()
            .filter(|m| {
                matches!(
                    m,
                    forge_cam::ops::PMove::Line {
                        phase: forge_cam::ops::Phase::Plunge,
                        ..
                    }
                )
            })
            .count();
        (prof.contains("G3 "), prof.contains("G2 "), plunges)
    };
    let (g3, g2, n0) = run("");
    assert!(g2 && !g3, "climb by default: clockwise round the part");
    let (g3, g2, n1) = run(", direction: \"climb\"");
    assert!(g2 && !g3);
    assert_eq!(n1, n0);
    let (g3, g2, _) = run(", direction: \"conventional\"");
    assert!(
        g3 && !g2,
        "conventional outside: counter-clockwise round the part"
    );
    let (_, _, n2) = run(", direction: \"conventional\", finish: 2, finish_step: 0.5, spring: 1");
    assert_eq!(n2, 3 * n0, "two radial passes and a spring pass at each level");
    let cam = forge_cam_lang::run(
        &doc("top = clear(op1, tool: t1, from: 18, to: 12, spring: 1)\n"),
        &dir,
    );
    assert!(
        cam.errors
            .iter()
            .any(|e| e.contains("clear does not take `spring:`")),
        "{:?}",
        cam.errors
    );
}

/// Heights as model references (Fusion parity): `from:`/`to:` take a horizontal face of the part
/// (`f.boss.end`, `f.plate.end`) with an offset (`+ 0.5`), read in the setup's frame; the program
/// is the one the same heights as numbers give. A reference that is not horizontal, or one in an
/// argument that is not a height, is refused.
#[test]
fn heights_are_model_references() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/machined");
    let text = |body: &str| {
        let cam = forge_cam_lang::run(&doc(body), &dir);
        assert!(cam.errors.is_empty(), "{:?}", cam.errors);
        assert!(
            cam.ops.iter().all(|o| o.status == "built"),
            "{:?}",
            cam.ops.iter().map(|o| &o.refusal).collect::<Vec<_>>()
        );
        cam.program_text().unwrap().0.to_string()
    };
    let prog = "prog = program(op1, post: \"fanuc\", number: 1)\n";
    let by_number = text(&format!(
        "skim = facing(op1, tool: t1, to: 18, step: 1)\ntop = clear(op1, tool: t1, from: 18, to: 12.5, step: 3, stepover: 4)\n{prog}"
    ));
    let by_model = text(&format!(
        "skim = facing(op1, tool: t1, to: f.boss.end, step: 1)\ntop = clear(op1, tool: t1, from: f.boss.end, to: f.plate.end + 0.5, step: 3, stepover: 4)\n{prog}"
    ));
    assert_eq!(by_number, by_model);
    // turned over (`from:`): the mounting face (z = 0 in the part) is the top, h − z = 18
    let turned = |to: &str| {
        let body = format!(
            "skim = facing(op1, tool: t1, to: 18, step: 1)\nprog = program(op1, post: \"fanuc\", number: 1)\nop2 = setup(f, from: op1, machine: m, origin: \"top-center\", tol: 0.01)\nback = facing(op2, tool: t1, to: {to}, step: 1)\nprog2 = program(op2, post: \"fanuc\", number: 2)\n"
        );
        let cam = forge_cam_lang::run(&doc(&body), &dir);
        assert!(cam.errors.is_empty(), "{:?}", cam.errors);
        let o = cam.ops.iter().find(|o| o.name == "back").unwrap();
        (
            o.status,
            format!("{:?}", o.paths.iter().map(|p| &p.moves).collect::<Vec<_>>()),
        )
    };
    let (st, a) = turned("18");
    assert_eq!(st, "built");
    let (_, b) = turned("f.plate.start");
    assert_eq!(a, b);
    for (body, why) in [
        (
            "skim = facing(op1, tool: t1, to: f.plan.outline, step: 1)\n",
            "not",
        ),
        (
            "skim = facing(op1, tool: t1, to: 18, step: f.plate.end)\n",
            "`step` takes a number, not a model height",
        ),
        (
            "skim = facing(op1, tool: t1, to: f.plate.end * 2, step: 1)\n",
            "a model height takes only + or − a length",
        ),
    ] {
        let cam = forge_cam_lang::run(&doc(body), &dir);
        assert!(
            cam.errors.iter().any(|e| e.contains(why)),
            "{body}: {:?}",
            cam.errors
        );
    }
}

fn stocked(stock: &str, dir: &Path) -> forge_cam_lang::Cam {
    stocked_to(stock, dir, -0.5)
}

fn stocked_to(stock: &str, dir: &Path, bottom: f64) -> forge_cam_lang::Cam {
    let src = format!(
        "machining\nf = part(\"flange.fg\", group: \"N\")\nm = machine(\"generic-vmc3\")\nt1 = tool(\"flat-10\", t: 1, stickout: 32, holder: \"bt40-er32\")\nop1 = setup(f, machine: m, stock: {stock}, origin: \"top-center\", tol: 0.01)\nskim = facing(op1, tool: t1, to: 18, step: 1)\ntop = clear(op1, tool: t1, from: 18, to: 12, step: 3, stepover: 4)\nprof = contour(op1, tool: t1, side: \"outside\", from: 12, to: {bottom}, step: 4)\nprog = program(op1, post: \"fanuc\", number: 1)\n"
    );
    forge_cam_lang::run(&src, dir)
}

/// Stock shapes (Fusion parity): a bar (`cylinder`) or a tube about the part's centre, or the
/// stock as its own part file (`solid`, a prism along z). The in-process stock is the stock's
/// own shape (what lies outside it cleared from its bottom up), and each is shown to hold the
/// part band by band (exact booleans): a bar too thin, or a tube whose bore cuts the part, is
/// refused with where.
#[test]
fn stock_is_a_bar_a_tube_or_a_modelled_solid() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/machined");
    let ok = |cam: &forge_cam_lang::Cam, kind: &str| {
        assert!(cam.errors.is_empty(), "{:?}", cam.errors);
        assert!(
            cam.ops.iter().all(|o| o.status == "built"),
            "{:?}",
            cam.ops.iter().map(|o| &o.refusal).collect::<Vec<_>>()
        );
        let mut j = String::new();
        cam.json().write(&mut j);
        assert!(j.contains(&format!("\"kind\":\"{kind}\"")), "{j}");
    };
    // the plate's corners are 40√2 from the centre: the bar is that plus 2 across
    ok(&stocked("cylinder(margin: (2, 1), below: 3)", &dir), "cylinder");
    // the bore is 20: a tube's bore of 18 is inside it at every height
    ok(
        &stocked("tube(d: 120, id: 18, margin: (0, 1), below: 3)", &dir),
        "tube",
    );
    let cam = stocked("cylinder(d: 100, margin: (0, 1))", &dir);
    assert!(
        cam.errors
            .iter()
            .any(|e| e.contains("the part is outside the stock")),
        "{:?}",
        cam.errors
    );
    let cam = stocked("tube(d: 120, id: 22)", &dir);
    assert!(
        cam.errors
            .iter()
            .any(|e| e.contains("the part is outside the stock")),
        "{:?}",
        cam.errors
    );
    // the stock modelled: a 90 × 90 plate to z = 19, in the part's frame
    let tmp = forge_dirs::temp_dir().join(format!("forgecad-stock-{}", forge_dirs::process_id()));
    std::fs::create_dir_all(&tmp).unwrap();
    std::fs::copy(dir.join("flange.fg"), tmp.join("flange.fg")).unwrap();
    std::fs::write(
        tmp.join("blank.fg"),
        "sketch s(on: xy) {\n  o = rect(center: (0, 0), size: (90, 90))\n}\nb = extrude(s, 19)\n",
    )
    .unwrap();
    // (flush with the part's bottom: the profile stops above it, or it would cut what holds
    // the stock)
    ok(&stocked_to("solid(\"blank.fg\")", &tmp, 0.5), "solid");
    let cam = stocked("solid(\"blank.fg\")", &tmp);
    assert!(cam.ops.iter().any(|o| {
        o.refusal
            .as_deref()
            .is_some_and(|r| r.contains("support under the stock"))
    }));
    // a blank whose section changes with height is not taken
    std::fs::write(
        tmp.join("stepped.fg"),
        "sketch s(on: xy) {\n  o = rect(center: (0, 0), size: (90, 90))\n}\nb = extrude(s, 10)\nsketch t(on: b.end) {\n  o = rect(center: (0, 0), size: (86, 86))\n}\nc = extrude(t.o, 9)\n",
    )
    .unwrap();
    let cam = stocked_to("solid(\"stepped.fg\")", &tmp, 0.5);
    assert!(
        cam.errors.iter().any(|e| e.contains("is not a prism along z")),
        "{:?}",
        cam.errors
    );
    let _ = std::fs::remove_dir_all(&tmp);
}
