//! Tool life over a job: the cited Taylor row is found for the tool's substrate, coating and
//! the part's material; a tool with no row is "not estimated"; a worn tool gets a warning with
//! a change point, never an error.
use forge_tooling::life::{self, Limit};
use std::collections::BTreeMap;
use std::path::Path;

fn doc(material: &str) -> String {
    format!(
        "machining\nf = part(\"flange.fg\", group: \"S\"{material})\nm = machine(\"generic-vmc3\")\nt1 = tool(\"lakeshore-LC206014X\", t: 1, stickout: 30, holder: \"bt40-er32\", rpm: 4000)\nt2 = tool(\"flat-10\", t: 2, stickout: 32, holder: \"bt40-er32\")\nop1 = setup(f, machine: m, stock: box(margin: (3, 3, 2), below: 3), origin: \"top-center\", tol: 0.01)\nskim = facing(op1, tool: t1, to: 18.5, step: 1)\nskim2 = facing(op1, tool: t2, to: 18, step: 1)\nprog = program(op1, post: \"fanuc\", number: 1)\n"
    )
}

#[test]
fn ti64_altin_carbide_is_estimated_and_warns_when_worn() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/machined");
    let cam = forge_cam_lang::run(&doc(", material: \"Ti-6Al-4V\""), &dir);
    assert!(
        cam.ok,
        "{:?} {:?}",
        cam.errors,
        cam.ops.iter().map(|o| (&o.name, &o.refusal)).collect::<Vec<_>>()
    );
    let rows = life::builtin();
    let out = cam.tool_life(&rows, &[], &BTreeMap::new());
    let t1 = out.iter().find(|o| o.t == 1).unwrap();
    assert_eq!(
        t1.life.row.as_deref(),
        Some("ti64-carbide-alti-n-highfeed-vb03"),
        "{}",
        t1.life.note
    );
    assert!(t1.life.minutes_in_cut > 0.0);
    let used = t1.life.used.unwrap();
    assert!(used > 0.0 && used.is_finite());
    // the sum is per sample: equal to Σ minutes / T(vc) for this V-T row
    let t2 = out.iter().find(|o| o.t == 2).unwrap();
    assert_eq!(t2.life.used, None);
    assert!(t2.life.note.starts_with("not estimated"), "{}", t2.life.note);
    assert!(t2.warning.is_none());
    // a tool with almost no life left: a warning, with the change point inside the program
    let mut lim = BTreeMap::new();
    lim.insert(
        1,
        Limit {
            at: 1.0,
            start: 1.0 - 0.5 * used,
        },
    );
    let out = cam.tool_life(&rows, &[], &lim);
    let w = out.iter().find(|o| o.t == 1).unwrap().warning.clone().unwrap();
    let at = w.change_at.unwrap();
    assert!(at.minutes > 0.0 && at.minutes < t1.life.minutes_in_cut);
    assert!(w.why.contains("change it in op 1"), "{}", w.why);
    // by name, for the UI
    let rep = forge_cam_lang::life::estimate(&cam, &rows, &lim);
    let v = rep.tools.iter().find(|t| t.t == 1).unwrap();
    assert_eq!(v.wear_limit_at.as_ref().unwrap().op, "skim");
    assert!(v.warning.is_some() && v.source.contains("work condition"));
    assert!(rep.ops.iter().any(|o| o.op == "skim2" && o.fraction.is_none()));
}

#[test]
fn no_material_is_not_estimated() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/machined");
    let cam = forge_cam_lang::run(&doc(""), &dir);
    assert!(
        cam.ok,
        "{:?} {:?}",
        cam.errors,
        cam.ops.iter().map(|o| (&o.name, &o.refusal)).collect::<Vec<_>>()
    );
    let out = cam.tool_life(&life::builtin(), &[], &BTreeMap::new());
    assert!(
        out.iter()
            .all(|o| o.life.used.is_none() && o.life.note.contains("names no material"))
    );
}
