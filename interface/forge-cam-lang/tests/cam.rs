//! The machining document end to end: built, certified on the posted path, and the refusals
//! typed with fixes.
use std::path::Path;

fn doc(body: &str) -> String {
    format!(
        "machining\nf = part(\"flange.fg\", group: \"N\")\nm = machine(\"generic-vmc3\")\nt1 = tool(\"flat-10\", t: 1, stickout: 32, holder: \"bt40-er32\")\nt2 = tool(\"flat-6\", t: 2, stickout: 26, holder: \"bt40-er32\")\nop1 = setup(f, machine: m, stock: box(margin: (3, 3, 2)), origin: \"top-center\", tol: 0.01)\n{body}"
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
    // G4: every band decided, nothing left over
    assert!(cam.leftover_done && cam.leftover.is_empty(), "{:?}", cam.leftover);
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
    let c = o.checks.iter().find(|c| c.rule == "fixture_collision").expect("a fixture check");
    assert!(c.other.contains("jaw"), "{}", c.other);
    let fix = o.fix.as_deref().unwrap();
    assert!(fix.starts_with("SetArg clamp: grip at most 11.00"), "{fix}");
    assert!(cam.program_text().is_none());
    // the facing passes overhang the stock: with the jaws gripping its full height, they would
    // cut the jaws
    let cam = forge_cam_lang::run(&vised(20.0, "skim = facing(op1, tool: t1, to: 18, step: 1)\n"), &dir);
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
    assert!(cam.errors.iter().any(|e| e.contains("clamp depth")), "{:?}", cam.errors);
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
        cam.ops[0].checks.iter().any(|c| c.rule == "fixture_collision" && c.phase == "rapid"),
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
    assert!(cam.ok, "{:?}", cam.ops.iter().map(|o| (&o.name, &o.refusal)).collect::<Vec<_>>());
    assert!(cam.leftover_done);
    assert!(cam.leftover.iter().any(|l| l.why == "held by fixture" && l.z1 <= 5.0));
    assert!(cam.leftover.iter().all(|l| l.why == "uncut" || l.z1 <= 5.0));
    let mut j = String::new();
    cam.json().write(&mut j);
    assert!(j.contains("\"why\":\"held by fixture\"") && j.contains("\"jaw_top\":5"), "{j}");
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
            assert!(b[0] > 27.0 && b[2] < 43.0 && b[1] > -49.0 && b[3] < -37.0, "{b:?}");
        }
    }
}

#[test]
fn soft_jaws_follow_the_part() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/machined");
    // a wider notch in a copy of the part: the jaws' pocket is cut to the new outline
    let tmp = std::env::temp_dir().join(format!("forge-cam-jaws-{}", std::process::id()));
    std::fs::create_dir_all(&tmp).unwrap();
    let part = std::fs::read_to_string(dir.join("bracket.fg")).unwrap();
    std::fs::write(tmp.join("bracket.fg"), part.replace("shared notch = 14", "shared notch = 20")).unwrap();
    std::fs::copy(dir.join("bracket_cam.fg"), tmp.join("bracket_cam.fg")).unwrap();
    let before = bracket(&dir, &|s| s);
    let after = bracket(&tmp, &|s| s);
    let _ = std::fs::remove_dir_all(&tmp);
    assert!(after.ok, "{:?} {:?}", after.errors, after.ops.iter().map(|o| (&o.name, &o.refusal)).collect::<Vec<_>>());
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
        cam.errors.iter().any(|e| e.contains("does not seat") && e.contains("relief")),
        "{:?}",
        cam.errors
    );
    // a grip as deep as the part
    let cam = bracket(&dir, &|s| s.replace("grip: 6", "grip: 14"));
    assert!(cam.errors.iter().any(|e| e.contains("grip depth")), "{:?}", cam.errors);
    // a hole is drilled from the side it opens on, not from the turned setup
    let cam = bracket(&dir, &|s| s + "late = drill(op2, f.bore_h, plan: \"auto\")\n");
    let late = cam.ops.iter().find(|o| o.name == "late").unwrap();
    assert_eq!(late.status, "refused");
    assert!(late.refusal.as_deref().unwrap().contains("side it opens on"), "{:?}", late.refusal);
}

#[test]
fn soft_jaws_need_the_top_faced() {
    // op1 not faced to the part's top: the stock above it would sit in the jaws below the seat
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/machined");
    let cam = bracket(&dir, &|s| s.replace("skim = facing(op1, tool: t1, to: 14, step: 1)\n", ""));
    assert!(
        cam.errors.iter().any(|e| e.contains("above the part's top") && e.contains("facing")),
        "{:?}",
        cam.errors
    );
}

#[test]
fn helical_ramps_are_posted_and_certified() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/machined");
    let cam = bracket(&dir, &|s| s);
    assert!(cam.ok, "{:?}", cam.ops.iter().map(|o| (&o.name, &o.refusal)).collect::<Vec<_>>());
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
    let txt = cam.programs.iter().find(|p| p.name == "p1").unwrap().text.clone().unwrap();
    assert!(txt.lines().any(|l| l.contains("G3") && l.contains(" Z")), "{txt}");
}

#[test]
fn engraving_follows_the_sketch_into_the_modelled_groove() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/machined");
    let src = std::fs::read_to_string(dir.join("plate_mark_cam.fg")).unwrap();
    let cam = forge_cam_lang::run(&src, &dir);
    assert!(cam.ok, "{:?} {:?}", cam.errors, cam.ops.iter().map(|o| (&o.name, &o.refusal)).collect::<Vec<_>>());
    let o = cam.ops.iter().find(|o| o.name == "mark").unwrap();
    assert_eq!((o.kind, o.status), ("engrave", "built"));
    // two levels along the line, down to 0.3 below the top
    let low = o.paths[0]
        .moves
        .iter()
        .filter_map(|m| if let forge_cam::ops::PMove::Line { to, .. } = m { Some(to[2]) } else { None })
        .fold(f64::INFINITY, f64::min);
    assert!((low - 4.7).abs() < 1e-9, "{low}");
    // the line to the groove's square ends: the V tool's width at depth cuts the end walls
    let tmp = std::env::temp_dir().join(format!("forge-cam-mark-{}", std::process::id()));
    std::fs::create_dir_all(&tmp).unwrap();
    let part = std::fs::read_to_string(dir.join("plate_mark.fg")).unwrap();
    std::fs::write(
        tmp.join("plate_mark.fg"),
        part.replace("line((m + groove_w / 2, W / 2), (L - m - groove_w / 2, W / 2))", "line((m, W / 2), (L - m, W / 2))"),
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
    assert!(cam.ok, "{:?} {:?}", cam.errors, cam.ops.iter().map(|o| (&o.name, &o.refusal)).collect::<Vec<_>>());
    // the drill plan leaves the thread: no tap
    let d = cam.ops.iter().find(|o| o.name == "tapd").unwrap();
    assert!(d.plan.iter().all(|p| !p.starts_with("tap")), "{:?}", d.plan);
    // the thread mill declares its Δ and climbs a helix of the pitch
    let t = cam.ops.iter().find(|o| o.name == "thr").unwrap();
    assert_eq!(t.removals.len(), 1);
    assert!(t.removals[0].starts_with("cosmetic_thread M6x1"), "{}", t.removals[0]);
    let txt = cam.program_text().unwrap().0;
    assert!(txt.lines().any(|l| l.contains("G3") && l.contains(" Z")), "{txt}");
    let mut j = String::new();
    cam.json().write(&mut j);
    assert!(j.contains("\"removals\":[\"cosmetic_thread M6x1"), "{j}");
    // a hole that records no thread: nothing to declare, so no thread milling
    let tmp = std::env::temp_dir().join(format!("forge-cam-m6-{}", std::process::id()));
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
    assert!(t.refusal.as_deref().unwrap().contains("records no thread"), "{:?}", t.refusal);
}
