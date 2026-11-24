//! A probing op end to end: planned in the document, posted for linuxcnc (G38.2 then the
//! back-off), read by forge-machine's interpreter, simulated on the setup's machine, and the
//! moves after each skip move checked for every place the probe can stop (collide's
//! check_after_skip): the skip moves come out proven.
mod common;
use common::{dir, doc};
use forge_machine::sim::Happening;

#[test]
fn a_posted_probe_op_runs_on_the_machine_and_its_skips_are_proven() {
    let src = doc(
        "p = probe(\"renishaw-omp40-2\", t: 20, ball: 6, length: 50, stem: 3, body: [(20, 0, 69)])\nz0 = probe_wcs(op1, tool: p, kind: \"surface\", at: (25, 25, 20))\nw = probe_wcs(op1, tool: p, kind: \"web\", at: (0, 0, 10), size: 86, axis: \"x\")\nprog = program(op1, post: \"linuxcnc\", number: 10)\n",
    );
    let cam = forge_cam_lang::run(&src, &dir());
    assert!(cam.errors.is_empty(), "{:?}", cam.errors);
    let run = cam.machine_run(0).unwrap();
    assert!(run.text.contains("G38.2"), "{}", run.text);
    // the probe is a probe in the simulation too
    let probe = run.tools.iter().find(|t| t.t == 20).unwrap();
    assert!(matches!(probe.cut, forge_machine::sim::Cut::Probe { .. }));
    // three skip moves (one surface, two web sides), each followed by its back-off
    let skips: Vec<usize> = run
        .tl
        .marks
        .iter()
        .filter(|m| matches!(m.what, Happening::Skip { .. }))
        .map(|m| m.line)
        .collect();
    assert_eq!(skips.len(), 3, "{}", run.text);
    assert!(run.tl.skip_unmodelled.is_empty(), "{:?}", run.tl.skip_unmodelled);
    for l in &skips {
        assert!(
            run.contacts.skips_proven.contains(l),
            "skip at line {l} not proven: {:?}",
            run.contacts
        );
    }
    // and skip.end narrows to the material removal after them
    let mut un = forge_machine::sim::unchecked(&run.def, &run.tl);
    forge_machine::collide::narrow_skip_unchecked(&mut un, &run.tl, &run.contacts);
    let se = un.iter().find(|u| u.path == "skip.end").unwrap();
    assert!(se.check.contains("collisions proven for every stop"), "{se:?}");
}
