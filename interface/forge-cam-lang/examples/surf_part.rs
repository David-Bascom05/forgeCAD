//! Times 3-axis surfacing on a .fg part: forge-cam-lang's example harness (args: part.fg).
use forge_cam::surf::{self, Surfacer};
use forge_tooling::Library;
fn main() {
    let path = std::env::args().nth(1).expect("part.fg");
    let src = std::fs::read_to_string(&path).unwrap();
    let (run, _) = forge_lang::run(&src, forge_lang::Options::default(), Default::default());
    let body = run.body.unwrap();
    let t0 = std::time::Instant::now();
    let (faces, whole) = forge_cam::lower3d::lower_surfacing(&body).unwrap();
    println!(
        "lowered {} faces ({} whole) in {:?}",
        faces.len(),
        whole.len(),
        t0.elapsed()
    );
    for (i, f) in faces.iter().enumerate() {
        println!(
            "  {i} id {} {:?} p {:?} q {:?} trim {}",
            f.id,
            std::mem::discriminant(&f.surf),
            f.p,
            f.q,
            f.trim.is_some()
        );
    }
    let lib = Library::builtin();
    let tl = lib.tool("ball-6").unwrap().clone();
    let shape = tl.shape(40.0, Some(lib.holder("bt40-er32").unwrap())).unwrap();
    let sf = Surfacer::new(&faces, &shape.bodies[0], 0.0, 0.01, 10.0).unwrap();
    let t0 = std::time::Instant::now();
    let mut worst = (0usize, 0.0, 0.0);
    let mut cells = 0;
    for iy in 0..25 {
        for ix in 0..31 {
            let (x, y) = (-30.0 + 2.0 * ix as f64, -24.0 + 2.0 * iy as f64);
            let c = sf.cl(x, y);
            cells += c.cells;
            if c.cells > worst.0 {
                worst = (c.cells, x, y);
            }
        }
    }
    println!("775 drops {:?}, cells {cells}, worst {worst:?}", t0.elapsed());
    let tool = forge_cam::ops::ToolUse {
        t: 1,
        tool: tl,
        holder: None,
        stickout: 40.0,
        shape,
    };
    let c = forge_cam::ops::Cut {
        tool: &tool,
        tidx: 0,
        h: forge_cam::ops::Heights {
            clear: 50.0,
            retract: 40.0,
        },
        feed: (1000.0, 300.0),
        ramp_feed: None,
        lead_feed: None,
    };
    let t0 = std::time::Instant::now();
    let r = surf::parallel(&sf, c, [-30.0, -24.0, 30.0, 24.0], 4.0, 0.0, 0.5);
    println!(
        "parallel (4 mm rows): {:?} {:?}",
        r.as_ref().map(|p| p.moves.len()),
        t0.elapsed()
    );
}
