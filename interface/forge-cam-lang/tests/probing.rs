//! Probing ops: a touch probe as a tool (`probe`), `probe_wcs` and `inspect`; their paths posted
//! (G38.2 where the dialect cites it, refused elsewhere) and certified with the probe's rules:
//! every body keeps the clearance everywhere, the ball meets material only in a probe move's
//! contact window. The flange's stock here is an unmachined box: x, y ±43, z -3..20.
mod common;
use common::{dir, doc};

const PROBE: &str =
    "p = probe(\"renishaw-omp40-2\", t: 20, ball: 6, length: 50, stem: 3, body: [(20, 0, 69)])\n";

fn run(body: &str) -> forge_cam_lang::Cam {
    forge_cam_lang::run(&doc(&format!("{PROBE}{body}")), &dir())
}

fn op<'a>(cam: &'a forge_cam_lang::Cam, name: &str) -> &'a forge_cam_lang::OpOut {
    cam.ops.iter().find(|o| o.name == name).unwrap()
}

fn failing(o: &forge_cam_lang::OpOut) -> Vec<String> {
    o.checks
        .iter()
        .filter(|c| c.verdict != "pass")
        .map(|c| format!("{} {} {} at {:?}", c.rule, c.verdict, c.phase, c.at))
        .collect()
}

#[test]
fn a_surface_probe_on_the_stock_top_posts_g38_and_certifies() {
    let cam = run(
        "z0 = probe_wcs(op1, tool: p, kind: \"surface\", at: (25, 25, 20), wcs: 54)\nprog = program(op1, post: \"linuxcnc\", number: 10)\n",
    );
    assert!(cam.errors.is_empty(), "{:?}", cam.errors);
    let o = op(&cam, "z0");
    assert_eq!(o.kind, "probe_wcs");
    assert!(failing(o).is_empty(), "{:?}", failing(o));
    let pr = o.probe.as_ref().unwrap();
    assert_eq!(pr.measures, "z");
    assert_eq!(pr.points[0].dir, [0.0, 0.0, -1.0]);
    assert!(!pr.sets_wcs);
    let (txt, _) = cam.program_text().unwrap();
    assert!(txt.contains("G38.2"), "{txt}");
    // the probe is not spun
    assert!(!txt.lines().any(|l| l.contains("M3")), "{txt}");
    let j = cam.json().to_string();
    assert!(j.contains("\"probe\":{\"kind\":\"surface\""), "{j}");
}

#[test]
fn a_probe_that_meets_material_before_its_window_fails() {
    // the stock's top is at 20: probing for a surface at 12 meets it 8 mm early
    let cam = run(
        "z0 = probe_wcs(op1, tool: p, kind: \"surface\", at: (25, 25, 12))\nprog = program(op1, post: \"linuxcnc\", number: 10)\n",
    );
    let f = failing(op(&cam, "z0"));
    assert!(f.iter().any(|c| c.starts_with("probe_clear fail probe")), "{f:?}");
}

#[test]
fn a_web_and_a_corner_on_the_stock_outside() {
    let cam = run(
        "w = probe_wcs(op1, tool: p, kind: \"web\", at: (0, 0, 10), size: 86, axis: \"x\")\nc = probe_wcs(op1, tool: p, kind: \"corner\", at: (43, 43, 10), side: (1, 1))\nprog = program(op1, post: \"grbl\", number: 10)\n",
    );
    assert!(cam.errors.is_empty(), "{:?}", cam.errors);
    for name in ["w", "c"] {
        let o = op(&cam, name);
        assert!(failing(o).is_empty(), "{name}: {:?}", failing(o));
    }
    assert_eq!(op(&cam, "w").probe.as_ref().unwrap().points.len(), 2);
    assert_eq!(op(&cam, "c").probe.as_ref().unwrap().measures, "corner x y");
}

#[test]
fn a_control_with_no_cited_probe_move_refuses_the_program() {
    let cam = run(
        "z0 = probe_wcs(op1, tool: p, kind: \"surface\", at: (25, 25, 20))\nprog = program(op1, post: \"fanuc\", number: 10)\n",
    );
    let j = cam.json().to_string();
    assert!(j.contains("a probe move on fanuc"), "{j}");
}

#[test]
fn inspect_and_refusals() {
    let cam = run(
        "i = inspect(op1, tool: p, at: (43, 0, 10), normal: (1, 0, 0), nominal: 43, tol: 0.05)\nprog = program(op1, post: \"linuxcnc\", number: 10)\n",
    );
    let o = op(&cam, "i");
    assert!(failing(o).is_empty(), "{:?}", failing(o));
    let pr = o.probe.as_ref().unwrap();
    assert_eq!((pr.nominal, pr.tol), (Some(43.0), Some(0.05)));
    for (body, want) in [
        (
            "x = probe_wcs(op1, tool: t1, kind: \"surface\", at: (0, 0, 20))\n",
            "a probe",
        ),
        (
            "x = probe_wcs(op1, tool: p, kind: \"slot\", at: (0, 0, 20))\n",
            "not one of surface",
        ),
        (
            "x = probe_wcs(op1, tool: p, kind: \"bore\", at: (0, 0, 10))\n",
            "size",
        ),
        ("q = probe(\"x\", t: 21, ball: 6, length: 50, stem: 3)\n", "body"),
        (
            "q = probe(\"x\", t: 21, ball: 6, length: 50, stem: 8, body: [(20, 0, 60)])\n",
            "stem",
        ),
    ] {
        let cam = run(body);
        let all = format!("{:?} {}", cam.errors, cam.json());
        assert!(all.contains(want), "{body}: {all}");
    }
}

#[test]
fn an_overtravel_that_brings_the_stem_to_the_wall_fails() {
    // ball r 3, stem r 1.5: 2 mm past a wall's contact, the stem is within 0.1 of it
    let cam = run(
        "i = inspect(op1, tool: p, at: (43, 0, 10), normal: (1, 0, 0), nominal: 43, tol: 0.05, over: 2)\nprog = program(op1, post: \"linuxcnc\", number: 10)\n",
    );
    let f = failing(op(&cam, "i"));
    assert!(f.iter().any(|c| c.starts_with("probe_clear fail touch")), "{f:?}");
}

#[test]
fn a_probe_feed_past_the_machines_feed_limit_fails_g5() {
    let cam = run(
        "z0 = probe_wcs(op1, tool: p, kind: \"surface\", at: (25, 25, 20), feed: 1000000)\nprog = program(op1, post: \"linuxcnc\", number: 10)\n",
    );
    let o = op(&cam, "z0");
    assert!(
        o.checks
            .iter()
            .any(|c| c.verdict != "pass" && c.other.contains("feed limit")),
        "{:?}",
        o.checks
    );
}

#[test]
fn an_approach_no_longer_than_the_window_is_refused() {
    let cam =
        run("z0 = probe_wcs(op1, tool: p, kind: \"surface\", at: (25, 25, 20), approach: 1.05, entry: 1)\n");
    let o = op(&cam, "z0");
    assert_eq!(o.status, "refused");
    assert!(
        o.refusal.as_ref().unwrap().contains("checked approach"),
        "{:?}",
        o.refusal
    );
}
