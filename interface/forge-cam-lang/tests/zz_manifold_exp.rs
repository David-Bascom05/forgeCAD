//! Experiment (not for merge): CAM-2D-FINISH's ops on the manifold shop part.
mod common;
use common::dir;

fn lib() -> forge_tooling::Library {
    let mut lib = forge_tooling::Library::builtin();
    lib.add(
        "project/cam/tools/shop.toml",
        "[[tool]]\nid = \"shop-threadmill-1.5\"\ntype = \"thread_mill\"\ndc = 14.0\ntp = 1.5\nnof = 3\nlcf = 20.0\ndcon = 14.0\noal = 75.0\nmaterial = \"carbide\"\nsource = \"shop\"\ntaken = \"2026-11-24\"\n\n[[tool]]\nid = \"shop-flat-10-long\"\ntype = \"flat\"\ndc = 10.0\nnof = 3\nlcf = 40.0\ndcon = 10.0\noal = 90.0\nmaterial = \"carbide\"\nsource = \"shop\"\ntaken = \"2026-11-24\"\n",
    )
    .unwrap();
    lib
}

fn run(name: &str, ops: &str) {
    let base = std::fs::read_to_string(dir().join("manifold_cam.fg")).unwrap();
    let head: String = base.lines().take_while(|l| !l.starts_with("skim")).collect::<Vec<_>>().join("\n");
    let src = format!("{head}\nt7 = tool(\"shop-threadmill-1.5\", t: 7, stickout: 40, holder: \"bt40-er32\")\nt8 = tool(\"spot-10-90\", t: 8, stickout: 30, holder: \"bt40-er32\")\nt9 = tool(\"shop-flat-10-long\", t: 9, stickout: 50, holder: \"bt40-er32\")\nskim  = facing(op1, tool: t6, to: 32, step: 1)\n{ops}\np1 = program(op1, post: \"fanuc\", number: 5001)\n");
    let t0 = std::time::Instant::now();
    let cam = forge_cam_lang::run_with_library(&src, &dir(), lib());
    eprintln!("=== {name} ({:.1} s) errors {:?}", t0.elapsed().as_secs_f64(), cam.errors);
    for p in &cam.programs {
        if p.error.is_some() {
            eprintln!("  program {}: {:?}", p.name, p.error);
        }
    }
    for o in &cam.ops {
        eprintln!("  {:6} {:9} {:?} {:?} cells {}", o.name, o.status, o.refusal, o.fix, o.cells);
        if std::env::var("MOVES").is_ok() {
            for (i, m) in o.paths.first().map(|p| p.moves.iter().take(16).collect::<Vec<_>>()).unwrap_or_default().iter().enumerate() {
                eprintln!("    {i} {m:?}");
            }
        }
    }
}

#[test]
#[ignore]
fn manifold_exp() {
    let v = std::env::var("EXP").unwrap_or_default();
    let all: &[(&str, &str)] = &[
        ("prof-plain", "prof = contour(op1, tool: t1, side: \"outside\", from: 26, to: 3, step: 5, finish: 2, finish_step: 0.3, spring: 1)"),
        ("prof-leads", "prof = contour(op1, tool: t1, side: \"outside\", from: 26, to: 3, step: 5, finish: 2, finish_step: 0.3, spring: 1, lead: \"arc\", lead_radius: 4, overlap: 0.5, entry: (0, -100))"),
        ("bore", "bh = bore(op1, f.bore, tool: t1, finish_leave: 0.2, spring: 1)"),
        ("bore-thr", "bh = bore(op1, f.bore, tool: t1, finish_leave: 0.2)\nthr = thread_mill(op1, f.bore, tool: t7, passes: 3, spring: 1, lead: \"arc\")"),
        ("prof-long", "prof = contour(op1, tool: t9, side: \"outside\", from: 26, to: 3, step: 5, finish: 2, finish_step: 0.3, spring: 1)"),
        ("prof-long-leads", "prof = contour(op1, tool: t9, side: \"outside\", from: 26, to: 3, step: 5, finish: 2, finish_step: 0.3, spring: 1, lead: \"arc\", lead_radius: 4, overlap: 0.5, entry: (0, -100))"),
        ("in-plain", "inn = contour(op1, tool: t2, side: \"inside\", from: 26, to: 24, step: 2)"),
        ("in-leads", "inn = contour(op1, tool: t2, side: \"inside\", from: 26, to: 24, step: 2, lead: \"arc\", lead_radius: 2)"),
        ("in-leads-small", "inn = contour(op1, tool: t2, side: \"inside\", from: 26, to: 24, step: 2, lead: \"arc\", lead_radius: 0.5)"),
        ("prof-tabs", "prof = contour(op1, tool: t9, side: \"outside\", from: 26, to: 3, step: 5, lead: \"arc\", lead_radius: 4, tabs: 4, tab_height: 2)"),
        ("brk", "brk = edge_break(op1, tool: t4, size: 0.3, tip_offset: 0.3)"),
        ("brk-spot", "brk = edge_break(op1, tool: t8, size: 0.3)"),
    ];
    for (n, o) in all {
        if v.is_empty() || v.split(',').any(|x| x == *n) {
            run(n, o);
        }
    }
}
