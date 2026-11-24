//! Refusals read for a person (a subject, units, where), with the typed code beside them for a
//! program (`OpOut::code`, the reply's "code"): a check's rule, or the refusal's own code. No raw
//! keys, Debug output or bare numbers in the words.
mod common;
use common::dir;

fn plain(words: &str) {
    for raw in [
        "{",
        "}",
        "Some(",
        " vs ",
        "fail at move",
        "NotPrism",
        "NeedTool",
        "_",
    ] {
        assert!(!words.contains(raw), "`{raw}` in {words:?}");
    }
    assert!(words.ends_with('.'), "{words:?}");
}

#[test]
fn the_manifolds_refusals_read_for_a_person() {
    let src = std::fs::read_to_string(dir().join("manifold_cam.fg")).unwrap();
    let cam = forge_cam_lang::run(&src, &dir());
    let op = |n: &str| cam.ops.iter().find(|o| o.name == n).unwrap();
    let thr = op("thr");
    assert_eq!(thr.code.as_deref(), Some("WrongPitch"));
    assert_eq!(
        thr.refusal.as_deref(),
        Some("Thread mill `threadmill-m6-m12-1` cuts a 1 mm pitch; the thread needs 1.5 mm.")
    );
    let dwl = op("dwl");
    assert_eq!(dwl.code.as_deref(), Some("NeedTool"));
    assert_eq!(
        dwl.refusal.as_deref(),
        Some("No 6 mm reamer for hole `dowels` (H7 fit) in the tool library.")
    );
    for o in cam.ops.iter().filter(|o| o.status == "refused") {
        assert!(o.code.is_some(), "{}: no code", o.name);
        let w = o.refusal.as_deref().unwrap();
        // hole and tool names are the user's own and may hold `_`: checked without them
        let bare: String = w.split('`').step_by(2).collect();
        plain(&bare);
    }
    // the reply carries both
    let j = cam.json().to_string();
    assert!(j.contains("\"code\":\"WrongPitch\""), "{j}");
    // a 2.5D op over a face that is not a wall: the statement's error names the face and the way
    // out, its code first
    assert!(
        cam.errors.iter().any(|e| e.contains(
            "NotPrism: At z = 29 mm, the part's face 38 is not straight-sided (a wall along z) there, so a 2.5D op cannot cut it: use a 3D op (zlevel) for that height."
        )),
        "{:?}",
        cam.errors
    );
}

#[test]
fn a_check_refusal_says_what_meets_what_by_how_much_and_where() {
    // a clear height under the stock's top: the drill's rapids enter the stock
    let src = "machining\nf = part(\"flange.fg\", group: \"N\")\nm = machine(\"generic-vmc3\")\nop1 = setup(f, machine: m, stock: box(margin: (3, 3, 2), below: 3), origin: \"top-center\", tol: 0.01, clear: -0.5, retract: -1)\nbolts = drill(op1, f.bolts, plan: \"auto\")\nprog = program(op1)\n";
    let cam = forge_cam_lang::run(src, &dir());
    let b = cam.ops.iter().find(|o| o.name == "bolts").unwrap();
    assert_eq!(b.status, "refused");
    assert_eq!(b.code.as_deref(), Some("rapid_clear"));
    let w = b.refusal.as_deref().unwrap();
    assert!(
        w.starts_with("On a rapid, the cutter would ")
            && w.contains(" mm ")
            && w.contains("the stock")
            && w.contains("on move 0 (a rapid) at (")
            && w.ends_with("; it must stay at least 0.1 mm clear."),
        "{w}"
    );
    plain(w);
    // the check itself stays typed
    assert_eq!(b.checks[0].rule, "rapid_clear");
}

#[test]
fn a_tool_too_short_says_which_lengths() {
    let src = "machining\nf = part(\"flange.fg\", group: \"N\")\nm = machine(\"generic-vmc3\")\nt1 = tool(\"flat-10\", t: 1, stickout: 22, holder: \"bt40-er32\")\n";
    let cam = forge_cam_lang::run(src, &dir());
    assert!(
        cam.errors.iter().any(|e| e.contains(
            "BadTool: Tool `flat-10`: its stick-out (22 mm) must be longer than its flute length (25 mm)."
        )),
        "{:?}",
        cam.errors
    );
}
