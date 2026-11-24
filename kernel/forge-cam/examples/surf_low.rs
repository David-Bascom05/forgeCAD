#[path = "surf_dome.rs"]
#[allow(dead_code)]
mod dome;
use forge_cam::ops::{self, Cut, Heights, PMove, ToolUse};
use forge_cam::surf::{self, Surfacer};
use forge_tooling::Library;
fn main() {
    let faces = dome::dome();
    let lib = Library::builtin();
    let tl = lib.tool("ball-6").unwrap().clone();
    let shape = tl.shape(tl.lcf + 15.0, Some(lib.holder("bt40-er32").unwrap())).unwrap();
    let ball = ToolUse { t: 1, tool: tl, holder: Some("bt40-er32".into()), stickout: 0.0, shape };
    let sf = Surfacer::new(&faces, &ball.shape.bodies[0], 0.0, 0.01, 0.0).unwrap();
    let c = Cut { tool: &ball, tidx: 0, h: Heights { clear: 40.0, retract: 25.0 }, feed: (1000.0, 300.0) };
    let t0 = std::time::Instant::now();
    let p = surf::parallel(&sf, c, [10.0, 10.0, 50.0, 50.0], 2.0, 30.0, 1.0).unwrap();
    println!("parallel {} moves {:?}", p.moves.len(), t0.elapsed());
    let t0 = std::time::Instant::now();
    let s = ops::certify_with(&p, &ball, &faces, &[(0, vec![])], &[], 0.0, 0.01, 0.1, true);
    println!("certify {} {:?}", s.clear, t0.elapsed());
    let mut low = p.clone();
    for m in low.moves.iter_mut() {
        if let PMove::Line { to, phase: ops::Phase::Cut, .. } = m { to[2] -= 0.05; }
    }
    let t0 = std::time::Instant::now();
    let s = ops::certify_with(&low, &ball, &faces, &[(0, vec![])], &[], 0.0, 0.01, 0.1, true);
    println!("low certify {} {:?} cells {} checks {}", s.clear, t0.elapsed(), s.cells, s.checks.len());
    for (name, r) in [("waterline", surf::waterline(&sf, c, [10.0, 10.0, 50.0, 50.0], &[2.0, 6.0], 1.0)), ("scallop", surf::scallop(&sf, c, [10.0, 10.0, 50.0, 50.0], 2.5, 1.0)), ("pencil", surf::pencil(&sf, c, [10.0, 10.0, 50.0, 50.0], 0.5, 0.02))] {
        let t0 = std::time::Instant::now();
        let p = r.unwrap();
        let s = ops::certify_with(&p, &ball, &faces, &[(0, vec![])], &[], 0.0, 0.01, 0.1, true);
        println!("{name} {} {:?}", s.clear, t0.elapsed());
    }
    let bull = lib.tool("bull-10-r1").unwrap().clone();
    let bs = bull.shape(bull.lcf + 15.0, None).unwrap();
    let sfb = Surfacer::new(&faces, &bs.bodies[0], 0.0, 0.01, 0.0).unwrap();
    let t0 = std::time::Instant::now();
    let p = surf::rest(&sf, &sfb, c, [10.0, 10.0, 50.0, 50.0], 2.0, 1.0, 1.0, 0.05).unwrap();
    println!("rest made {:?}", t0.elapsed());
    let t0 = std::time::Instant::now();
    let s = ops::certify_with(&p, &ball, &faces, &[(0, vec![])], &[], 0.0, 0.01, 0.1, true);
    println!("rest {} {:?}", s.clear, t0.elapsed());
}
