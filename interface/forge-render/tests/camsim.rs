//! The toolpath and stock simulation, headless, on real machining documents
//! (examples/machined): the timeline, the display stock, and the certified checks as markers.

use forge_render::camsim::timeline::{Kind, Timeline, push_path};
use forge_render::camsim::zmap::{Compare, Cutter, ZMap, compare};
use forge_render::camsim::{Sim, draw};
use std::path::{Path, PathBuf};

fn machined() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/machined")
}

fn flange() -> Sim {
    let dir = machined();
    let src = std::fs::read_to_string(dir.join("flange_cam.fg")).unwrap();
    Sim::open(&src, &dir)
}

#[test]
fn the_flange_plays_as_posted() {
    let sim = flange();
    assert!(sim.cam.ok, "{:?}", sim.cam.errors);
    let tl = &sim.tl;
    assert_eq!(tl.ops.len(), 5);
    // every op played from its posted text (the paths the checks ran on)
    for (i, op) in sim.cam.ops.iter().enumerate() {
        assert_eq!(op.posted.len(), op.paths.len(), "{}", op.name);
        assert!(tl.ops[i].end > tl.ops[i].first, "{}", op.name);
    }
    // in order, back to back, and continuous within each path
    let mut t = 0.0;
    for (k, g) in tl.segs.iter().enumerate() {
        assert!((g.t0 - t).abs() < 1e-9, "piece {k} starts at {} not {t}", g.t0);
        t = g.t1();
        if let Some(n) = tl.segs.get(k + 1)
            && n.op == g.op
            && n.path == g.path
        {
            let d = (0..3).map(|i| (n.a[i] - g.b[i]).abs()).fold(0.0, f64::max);
            assert!(d < 1e-9, "a jump of {d} between pieces {k} and {}", k + 1);
        }
        // an arc's ends are where the move says
        let e = (0..3).map(|i| (g.at(1.0)[i] - g.b[i]).abs()).fold(0.0, f64::max);
        assert!(e < 1e-6, "piece {k} ends {e} from its end");
    }
    assert!((tl.total - t).abs() < 1e-9);
    assert!(tl.segs.iter().any(|g| g.kind == Kind::Rapid));
    assert!(
        tl.segs.iter().any(|g| g.arc.is_some()),
        "the bore and contour have arcs"
    );
    // the tool's place at a time, and a move's time
    let (p, i) = tl.pos(0.0).unwrap();
    assert_eq!(i, 0);
    assert_eq!(p, tl.segs[0].a);
    let mid = tl.ops[2].t0 + 0.5 * (tl.ops[2].t1 - tl.ops[2].t0);
    assert_eq!(tl.op_at(mid), Some(2));
    let g = &tl.segs[tl.ops[3].first + 5];
    let at = tl.locate(3, g.path as usize, g.mv as usize, 0.0).unwrap();
    assert!(at <= g.t0 + 1e-9 && tl.seg_at(at + 1e-9).is_some());
    // stepping visits every piece's start
    assert_eq!(tl.step(0.0, false), tl.segs[1].t0);
    assert_eq!(tl.step(tl.segs[1].t0, true), 0.0);
    // certified, nothing left over: no markers
    assert!(sim.markers.is_empty(), "{:?}", sim.markers);
}

#[test]
fn the_flanges_stock_never_shows_a_gouge_and_ends_at_the_part() {
    let mut sim = flange();
    assert!(!sim.part_tris[0].is_empty(), "the part meshed");
    let tol = sim.setups[0].tol as f32;
    let total = sim.tl.total;
    let start_volume = {
        let (z, _) = sim.stock_at(0.0).unwrap();
        z.volume()
    };
    let (z, s) = sim.stock_at(total).unwrap();
    let z = z.clone();
    assert_eq!(s, 0);
    assert!(
        z.volume() < start_volume * 0.8,
        "{} of {start_volume}",
        z.volume()
    );
    let pm = sim.part_maps[0].as_ref().unwrap();
    let mut counts = [0usize; 3];
    for k in 0..z.nx * z.ny {
        match compare(&z, pm, k, tol) {
            Some((Compare::Gouge, d)) => panic!(
                "column {k} at {:?} below the part by {d}: stock {}..{} part {}..{}",
                z.centre(k % z.nx, k / z.nx),
                z.lo[k],
                z.hi[k],
                pm.bottom[k],
                pm.top[k]
            ),
            Some((Compare::InTol, _)) => counts[1] += 1,
            Some((Compare::Under, _)) => counts[0] += 1,
            None => {}
        }
    }
    // the part's top is reached all over (columns straddling its walls may stay above)
    assert!(counts[1] > 10 * counts[0], "{counts:?}");
    // scrubbing back and forward again gives the same stock
    let (a, _) = sim.stock_at(total * 0.37).unwrap();
    let a = a.clone();
    sim.stock_at(total * 0.9).unwrap();
    let (b, _) = sim.stock_at(total * 0.37).unwrap();
    assert_eq!(&a, b);
    let (c, _) = sim.stock_at(total).unwrap();
    assert_eq!(c, &z);
}

#[test]
fn scrubbing_takes_milliseconds() {
    let mut sim = flange();
    let total = sim.tl.total;
    sim.stock_at(total).unwrap();
    let t0 = std::time::Instant::now();
    let n = 40;
    for k in 0..n {
        // back and forth across the whole program
        let t = total * ((k * 7919) % n) as f64 / n as f64;
        sim.stock_at(t).unwrap();
    }
    let per = t0.elapsed().as_secs_f64() * 1e3 / n as f64;
    eprintln!("stock at a random time: {per:.2} ms");
    let (z, s) = sim.stock_at(total * 0.5).unwrap();
    let z = z.clone();
    let t0 = std::time::Instant::now();
    let mut out = vec![];
    draw::stock(
        &z,
        sim.part_maps[s].as_ref(),
        draw::StockColour::Compare,
        0.01,
        &mut out,
    );
    eprintln!(
        "stock mesh: {:.2} ms, {} triangles",
        t0.elapsed().as_secs_f64() * 1e3,
        out.len() / 3
    );
    // a debug build's bound, generous; the release figures are in the PR
    assert!(per < 500.0, "{per} ms");
}

/// A straight slot with a flat end mill: the depth exact, the width to a cell.
#[test]
fn a_flat_slot_is_cut_to_depth_and_width() {
    let mut z = ZMap::new([0.0, 0.0, 0.0], [100.0, 40.0, 10.0], 40_000);
    let c = Cutter::flat(5.0);
    let mut segs = vec![];
    let p = forge_cam::ops::OpPath {
        tool: 0,
        start: [10.0, 20.0, 6.0],
        moves: vec![forge_cam::ops::PMove::Line {
            to: [90.0, 20.0, 6.0],
            f: 600.0,
            phase: forge_cam::ops::Phase::Cut,
        }],
        rpm: 1000.0,
        coolant: forge_post::Coolant::Off,
    };
    let t = push_path(0, 0, &p, &mut segs, 0.0);
    assert!((t - 80.0 / 600.0 * 60.0).abs() < 1e-9);
    let before = z.volume();
    assert!(z.cut(&segs[0], 0.0, 1.0, &c));
    let removed = before - z.volume();
    // the slot: 80 x 10 plus the two half discs, 4 deep
    let exact = (80.0 * 10.0 + std::f64::consts::PI * 25.0) * 4.0;
    let err = (removed - exact).abs() / exact;
    assert!(err < 0.03, "removed {removed} vs {exact}");
    let (i, j) = z.cell_of([50.0, 20.0]).unwrap();
    assert_eq!(z.hi[j * z.nx + i], 6.0);
    let (i, j) = z.cell_of([50.0, 30.0]).unwrap();
    assert_eq!(z.hi[j * z.nx + i], 10.0);
}

/// A ball end mill's groove along a full circle, and a plunge through: the groove's section is
/// the ball's, the plunge empties its columns.
#[test]
fn a_ball_groove_and_a_through_plunge() {
    let mut z = ZMap::new([-30.0, -30.0, 0.0], [30.0, 30.0, 10.0], 90_000);
    let ball = Cutter::ball(3.0);
    let mut segs = vec![];
    let p = forge_cam::ops::OpPath {
        tool: 0,
        start: [20.0, 0.0, 8.0],
        moves: vec![
            forge_cam::ops::PMove::Arc {
                to: [20.0, 0.0, 8.0],
                c: [0.0, 0.0],
                cw: false,
                f: 500.0,
            },
            forge_cam::ops::PMove::Rapid { to: [0.0, 0.0, 12.0] },
            forge_cam::ops::PMove::Line {
                to: [0.0, 0.0, -1.0],
                f: 200.0,
                phase: forge_cam::ops::Phase::Plunge,
            },
        ],
        rpm: 1000.0,
        coolant: forge_post::Coolant::Off,
    };
    push_path(0, 0, &p, &mut segs, 0.0);
    assert_eq!(segs.len(), 3);
    assert!(
        (segs[0].len - std::f64::consts::TAU * 20.0).abs() < 1e-9,
        "a whole turn"
    );
    for g in &segs {
        z.cut(g, 0.0, 1.0, &ball);
    }
    // on the circle the ball's bottom; 2 mm off it, the ball's side
    let h = |p: [f64; 2]| {
        let (i, j) = z.cell_of(p).unwrap();
        z.hi[j * z.nx + i] as f64
    };
    let ring = h([0.0, 20.0]);
    assert!((ring - 8.0).abs() < 0.05, "{ring}");
    let side = h([0.0, 22.0]);
    let want = 8.0 + 3.0 - (9.0f64 - 4.0).sqrt();
    assert!((side - want).abs() < 0.15, "{side} vs {want}");
    // the plunge went through the bottom: its column is empty
    let (i, j) = z.cell_of([0.0, 0.0]).unwrap();
    let k = j * z.nx + i;
    assert!(z.hi[k] <= z.lo[k]);
}

/// The tool's envelope from its real profile (forge-tooling's library through the CAM run).
#[test]
fn cutters_from_the_tools_profiles() {
    let sim = flange();
    let c = sim.cutters[0].as_ref().expect("flat-10 has a cutting body");
    assert!((c.r - 5.0).abs() < 1e-9, "{}", c.r);
    assert_eq!(c.at(0.0), Some(0.0));
    assert_eq!(c.at(4.9), Some(0.0));
    assert_eq!(c.at(5.1), None);
    assert!(Timeline::of(&sim.cam).total > 0.0);
}

/// A refused op's checks are markers at the time of their move.
#[test]
fn a_refused_ops_check_is_a_marker_on_its_move() {
    let dir = machined();
    let src = "machining\nf = part(\"flange.fg\", group: \"N\")\nm = machine(\"generic-vmc3\")\nt1 = tool(\"flat-10\", t: 1, stickout: 32, holder: \"bt40-er32\")\nop1 = setup(f, machine: m, stock: box(margin: (3, 3, 2)), origin: \"top-center\", tol: 0.01)\nskim = facing(op1, tool: t1, to: 18, step: 1)\nbolts = drill(op1, f.bolts, plan: \"auto\")\nprog = program(op1)\n";
    let sim = Sim::open(src, &dir);
    let b = sim.cam.ops.iter().position(|o| o.name == "bolts").unwrap();
    assert_eq!(sim.cam.ops[b].status, "refused");
    let m: Vec<_> = sim.markers.iter().filter(|m| m.op == Some(b)).collect();
    assert!(!m.is_empty());
    let sp = &sim.tl.ops[b];
    for m in &m {
        assert!(
            m.t >= sp.t0 && m.t <= sp.t1,
            "{} outside {}..{}",
            m.t,
            sp.t0,
            sp.t1
        );
        assert!(m.severe());
    }
    // facing alone leaves the plate's sides: leftover markers once the setup is done
    assert!(
        sim.markers
            .iter()
            .any(|m| m.rule == "leftover" && m.region.is_some())
    );
}
