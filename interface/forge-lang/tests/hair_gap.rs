//! Pinned in the kernel-stress corpus (research/kernel-stress/pinned.tsv, owner FALLBACK): the
//! demo part's counterbored hole1 with a tapped M5 hole and its thread where it is, its diameter
//! swept through the M5 major diameter. A bore a hair inside it (4.9999, 4.99997) was refused,
//! InconsistentClassification, where 4.999, 5 and 5.00002 built (CURVED-SITES). A guard: each
//! diameter builds a valid part or is refused, never a wrong one; the window it is refused in is
//! printed (`--nocapture`), to map it. On 12c7d280: built at 4.999 and from 5.00002; refused from
//! 4.9995 to 4.99997 (InconsistentClassification), at 4.99999 (a split crossed itself) and at 5 and
//! 5.00001 (NoClosedForm on the cylinder): no wrong answer.

use forge_lang::{Options, run};

/// The diameters about 5; 5 itself, the pinned document as it stands, is its own test (its
/// refusal takes 4 s).
const DIAMETERS: [f64; 11] = [
    4.999, 4.9995, 4.9998, 4.9999, 4.99995, 4.99997, 4.99999, 5.00001, 5.00002, 5.00005, 5.0001,
];

/// One build: "built" (and it checks clean) or "refused: why".
fn verdict(d: f64) -> String {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/demo");
    let opts = Options {
        dir: Some(dir),
        ..Options::default()
    };
    let (r, _) = run(&part(d), opts, Default::default());
    match r.errors.iter().find(|e| e.kind() == "Build") {
        Some(e) => format!("refused: {}", e.text().chars().take(100).collect::<String>()),
        None => {
            let b = r.body.as_ref().expect("a body");
            let p = forge_topo::check(b);
            assert!(p.is_empty(), "d {d}: an Ok that fails check: {p:?}");
            "built".into()
        }
    }
}

fn part(d: f64) -> String {
    let base = include_str!("../../../examples/demo/part_tools.fg");
    assert!(base.contains("param hole1.d = 5\n"));
    format!(
        "{}hole2 = hole(at: sk2.p1, screw: \"M5\", fit: \"tap\", depth: thru)\nhole2_thread = thread(hole: hole2, length: \"full\")\n",
        base.replace("param hole1.d = 5\n", &format!("param hole1.d = {d}\n"))
    )
}

#[test]
fn a_bore_a_hair_inside_a_tapped_m5_builds_right_or_is_refused() {
    // each diameter on a thread of its own: twelve builds one after another were 6 s
    let map: Vec<String> = std::thread::scope(|s| {
        let all: Vec<_> = DIAMETERS
            .into_iter()
            .map(|d| s.spawn(move || format!("{d:<8} {}", verdict(d))))
            .collect();
        all.into_iter().map(|t| t.join().unwrap()).collect()
    });
    eprintln!("{}", map.join("\n"));
}

/// The pinned document itself (the bore at the M5 major diameter, 5): built right or refused.
#[test]
fn corpus_guard_part_tools_tapped_m5() {
    eprintln!("5        {}", verdict(5.0));
}
