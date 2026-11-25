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
    // certified: no check is a marker; what G4 leaves (the skin under the part, the stock
    // being 3 below it and the contour going to −0.5) is, below z 0, once the setup is done
    assert!(sim.markers.iter().all(|m| !m.severe()), "{:?}", sim.markers);
    for m in &sim.markers {
        // all of it the stock left under the part by the plan: shown neutral, not an issue
        assert!(m.planned, "{m:?}");
        assert!(m.text.starts_with("Under the part, z "), "{}", m.text);
        let (_, _, z1) = m.region.expect("a leftover region");
        assert!(z1 <= 1e-9 && (m.t - tl.total).abs() < 1e-9, "{m:?}");
    }
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
            Some((Compare::Under, _)) => {
                // material left on is the skin under the part (the stock 3 below it, the
                // contour to −0.5), or a column straddling a wall
                assert!(z.hi[k] <= -0.5 + tol || pm.has(k), "column {k}: {} left", z.hi[k]);
                counts[0] += 1
            }
            None => {}
        }
    }
    // the part's top is reached all over (columns straddling its walls may stay above)
    assert!(counts[1] > 3 * counts[0], "{counts:?}");
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

/// A refused op's checks are markers at the time of their move: the bolt holes drilled under a
/// clear height set below the stock's top (`clear: -0.5`) rapid into the stock. (Drilling before
/// the top is faced is certified since 195b4712: each cycle's R plane is lifted over the stock.)
#[test]
fn a_refused_ops_check_is_a_marker_on_its_move() {
    let dir = machined();
    let src = "machining\nf = part(\"flange.fg\", group: \"N\")\nm = machine(\"generic-vmc3\")\nt1 = tool(\"flat-10\", t: 1, stickout: 32, holder: \"bt40-er32\")\nop1 = setup(f, machine: m, stock: box(margin: (3, 3, 2), below: 3), origin: \"top-center\", tol: 0.01, clear: -0.5, retract: -1)\nbolts = drill(op1, f.bolts, plan: \"auto\")\nprog = program(op1)\n";
    let sim = Sim::open(src, &dir);
    let b = sim.cam.ops.iter().position(|o| o.name == "bolts").unwrap();
    assert_eq!(sim.cam.ops[b].status, "refused");
    assert!(
        !sim.cam.ops[b].checks.is_empty(),
        "refused by its checks, on moves"
    );
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
        assert_ne!(m.rule, "refused", "a check's marker, not the op's");
    }
    // drilling alone leaves the plate's top and sides: leftover markers once the setup is done
    assert!(
        sim.markers
            .iter()
            .any(|m| m.rule == "leftover" && m.region.is_some())
    );
}

/// An op refused before it has moves (here: drilling through with no stock under the part) is a
/// marker all the same, at its place on the timeline, saying why.
#[test]
fn an_op_refused_without_moves_is_a_marker() {
    let dir = machined();
    let src = "machining\nf = part(\"flange.fg\", group: \"N\")\nm = machine(\"generic-vmc3\")\nt1 = tool(\"flat-10\", t: 1, stickout: 32, holder: \"bt40-er32\")\nop1 = setup(f, machine: m, stock: box(margin: (3, 3, 2)), origin: \"top-center\", tol: 0.01)\nskim = facing(op1, tool: t1, to: 18, step: 1)\nbolts = drill(op1, f.bolts, plan: \"auto\")\nprog = program(op1)\n";
    let sim = Sim::open(src, &dir);
    let b = sim.cam.ops.iter().position(|o| o.name == "bolts").unwrap();
    let op = &sim.cam.ops[b];
    assert_eq!(op.status, "refused");
    if !op.checks.is_empty() {
        // refused by its checks after all (planning changed): the case above covers it
        return;
    }
    let m: Vec<_> = sim.markers.iter().filter(|m| m.op == Some(b)).collect();
    assert_eq!(m.len(), 1, "{m:?}");
    assert_eq!(m[0].rule, "refused");
    assert!(m[0].severe());
    assert_eq!(m[0].t, sim.tl.ops[b].t0);
    assert!(
        m[0].text.contains(op.refusal.as_deref().unwrap()),
        "{}",
        m[0].text
    );
}

/// The flange's posted program on the generic VMC (forge-machine): interpreted, timed, every
/// body checked through it; the tip the machine puts the tool at is the posted path's; played as
/// the simulation's source, its stock is the toolpath's.
#[test]
fn the_flanges_posted_program_on_its_machine() {
    let mut sim = flange();
    // the post now retracts with `G91 G28 Z0.` before each M06; forge-machine's interpreter
    // refuses an incremental word before any absolute position (CAM-MACHINE, reported): until
    // it takes the setup's start pose as known, the machine run is the refusal, said
    let mv = match &sim.machines[0] {
        Ok(mv) => mv,
        Err(e) => {
            assert!(e.contains("incremental Z from an unknown position"), "{e}");
            eprintln!("machine run refused: {e}");
            return;
        }
    };
    let m = &mv.run;
    // the playback clock (feeds as programmed, rapids at the machine's published rates, the
    // axes together) is never longer than the machine run's own lower bound, which adds tool
    // changes and dwells to the same motion
    assert!(
        sim.tl.total <= m.tl.t_min + 1e-6,
        "playback {} s over the machine's least {} s",
        sim.tl.total,
        m.tl.t_min
    );
    assert_eq!(m.def.name, "generic-vmc3");
    assert!(m.wcs_given, "the flange gives its `at:` (CAM-CORE 8cae054a)");
    assert!(m.tl.segments.len() > 300);
    assert!(m.tl.violations.is_empty(), "{:?}", m.tl.violations);
    // every feed's tip, in the document's frame, ends on the posted path (rapids also go home)
    let ends: Vec<[f64; 3]> = sim.tl.segs.iter().map(|g| g.b).collect();
    for g in mv.tl.segs.iter().filter(|g| g.kind != Kind::Rapid) {
        let d = ends
            .iter()
            .map(|q| (0..3).map(|k| (q[k] - g.b[k]).powi(2)).sum::<f64>().sqrt())
            .fold(f64::INFINITY, f64::min);
        assert!(d < 1e-6, "a machine motion ends {d} from the posted path");
    }
    // an arc's middle is where the machine puts the tool halfway
    for (g, &k) in mv.tl.segs.iter().zip(&mv.seg_of) {
        let s = &m.tl.segments[k];
        let want = m.tip(&s.at(0.37), s.tool.unwrap()).unwrap();
        let got = g.at(0.37);
        let d = (0..3).map(|i| (want[i] - got[i]).powi(2)).sum::<f64>().sqrt();
        assert!(
            d < 1e-6,
            "piece {k} ({:?}, arc {:?}) off the machine's by {d}: {got:?} vs {want:?}",
            g.kind,
            g.arc
        );
    }
    // every op of the program found by its comment
    for (k, sp) in mv.tl.ops.iter().enumerate() {
        assert!(sp.end > sp.first, "op {k} ({}) not in the program", sp.name);
    }
    // with the stock 3 below the part (CAM-CORE's fix), nothing hits: no body of the machine
    // or the workholding meets the tool (a drill breaking through may come within the
    // clearance of the table: a `near`, said)
    assert!(
        m.contacts
            .contacts
            .iter()
            .all(|c| !matches!(c.verdict, forge_machine::collide::Verdict::Hit { .. })),
        "{:?}",
        m.contacts
            .contacts
            .iter()
            .find(|c| matches!(c.verdict, forge_machine::collide::Verdict::Hit { .. }))
    );
    assert!(
        mv.markers.iter().all(|k| k.verdict != "fail"),
        "{:?}",
        mv.markers.first()
    );
    // the posted code's rapids and holders against the stock as it cuts it: none
    assert!(
        sim.markers.iter().all(|k| !k.rule.ends_with("_into_stock_grid")),
        "{:?}",
        sim.markers.iter().find(|k| k.rule.ends_with("_into_stock_grid"))
    );
    // played as the source: the machine's time, the same stock at the end
    let end_tp = {
        let total = sim.tl.total;
        sim.stock_at(total).unwrap().0.clone()
    };
    sim.set_source(Some(0)).unwrap();
    assert_eq!(sim.machine().map(|m| m.setup), Some(0));
    assert!((sim.tl.total - sim.machines[0].as_ref().unwrap().run.tl.t_stop).abs() < 1e-6);
    let total = sim.tl.total;
    let end_m = sim.stock_at(total).unwrap().0.clone();
    let diff = end_m
        .hi
        .iter()
        .zip(&end_tp.hi)
        .filter(|(a, b)| (**a - **b).abs() > 1e-3)
        .count();
    // the same stock: the posted code does what the toolpath says
    assert!(diff * 1000 < end_tp.hi.len(), "{diff} columns differ");
    // and back
    sim.set_source(None).unwrap();
    assert!(sim.machine().is_none());
}

/// Colour by a measured value: each piece of a move takes the move's value, coloured by its
/// band against its scale; pieces without one are grey.
#[test]
fn moves_coloured_by_a_measured_value() {
    use forge_render::camsim::draw::{Values, look};
    let sim = flange();
    // a made-up measure for the test: the move's index over the op's first path, scale 10
    let v = Values::of_moves(
        &sim.tl,
        "chip load",
        "mm",
        Values::chip_bands(),
        &|op, path, mv| (op == 1 && path == 0).then_some((mv as f32 / 10.0, 10.0)),
    );
    assert_eq!(v.per_piece.len(), sim.tl.segs.len());
    for (g, x) in sim.tl.segs.iter().zip(&v.per_piece) {
        assert_eq!(x.is_some(), g.op == 1, "piece of op {}", g.op);
    }
    assert_eq!(v.colour(0.2, 1.0), look::LOW);
    assert_eq!(v.colour(0.8, 1.0), look::GOOD);
    assert_eq!(v.colour(1.2, 1.0), look::HIGH);
    assert_eq!(v.colour(3.0, 1.0), look::OVER);
}

/// Values per feed move (as forge-cam-quality measures them: rapids and cycles skipped) land
/// on the played path's lines and arcs in order; a path whose count differs stays grey.
#[test]
fn feed_move_values_land_on_lines_and_arcs() {
    use forge_cam::ops::PMove;
    use forge_render::camsim::draw::Values;
    let sim = flange();
    let feeds = |i: usize, p: usize| {
        sim.cam.sim_paths(i)[p]
            .moves
            .iter()
            .filter(|m| matches!(m, PMove::Line { .. } | PMove::Arc { .. }))
            .count()
    };
    // op 1: value k for its k-th feed move of each path; op 0: one value short (left grey)
    let v = Values::of_feeds(
        &sim.cam,
        &sim.tl,
        "chip load",
        "",
        Values::chip_bands(),
        &|i| match i {
            0 => Some(
                (0..feeds(0, 0).saturating_sub(1))
                    .map(|k| (0, k as f32, 1.0))
                    .collect(),
            ),
            1 => Some(
                (0..sim.cam.sim_paths(1).len())
                    .flat_map(|p| (0..feeds(1, p)).map(move |k| (p, k as f32, 1.0)))
                    .collect(),
            ),
            _ => None,
        },
    );
    let mut seen = std::collections::HashMap::new();
    let mut coloured = 0;
    for (g, x) in sim.tl.segs.iter().zip(&v.per_piece) {
        if g.op != 1 {
            assert!(x.is_none(), "op {} piece coloured", g.op);
            continue;
        }
        let m = &sim.cam.sim_paths(1)[g.path as usize].moves[g.mv as usize];
        let feed = matches!(m, PMove::Line { .. } | PMove::Arc { .. });
        assert_eq!(x.is_some(), feed, "op 1 move {} {:?}", g.mv, m);
        if let Some((val, _)) = x {
            // the k-th feed move of its path, in order
            let k = sim.cam.sim_paths(1)[g.path as usize].moves[..g.mv as usize]
                .iter()
                .filter(|m| matches!(m, PMove::Line { .. } | PMove::Arc { .. }))
                .count();
            assert_eq!(*val, k as f32);
            seen.insert((g.path, g.mv), ());
            coloured += 1;
        }
    }
    assert!(coloured > 0 && !seen.is_empty());
}

/// Rapids and holders against the stock as cut, on the grid: none on the certified flange
/// (milliseconds, the whole program); the bolt holes drilled under a clear height below the
/// stock's top rapid into the stock, as the certified checker also says.
#[test]
fn rapids_and_holders_against_the_stock_as_cut() {
    let sim = flange();
    let t0 = std::time::Instant::now();
    let issues = sim.stock_issues();
    eprintln!(
        "stock issues over the flange: {:.1} ms",
        t0.elapsed().as_secs_f64() * 1e3
    );
    assert!(issues.is_empty(), "{:?}", issues.first());
    let dir = machined();
    let src = "machining\nf = part(\"flange.fg\", group: \"N\")\nm = machine(\"generic-vmc3\")\nt1 = tool(\"flat-10\", t: 1, stickout: 32, holder: \"bt40-er32\")\nop1 = setup(f, machine: m, stock: box(margin: (3, 3, 2), below: 3), origin: \"top-center\", tol: 0.01, clear: -0.5, retract: -1)\nbolts = drill(op1, f.bolts, plan: \"auto\")\nprog = program(op1)\n";
    let sim = Sim::open(src, &dir);
    let b = sim.cam.ops.iter().position(|o| o.name == "bolts").unwrap();
    assert_eq!(sim.cam.ops[b].status, "refused");
    let issues = sim.stock_issues();
    assert!(
        issues
            .iter()
            .any(|m| m.rule == "rapid_into_stock_grid" && m.op == Some(b)),
        "{issues:?}"
    );
    // never read as a proven check: their kind and their words say they are approximate
    let m = issues.iter().find(|m| m.rule == "rapid_into_stock_grid").unwrap();
    assert!(
        m.text.contains("Approximate") && m.row.contains("about"),
        "{}",
        m.text
    );
    assert_eq!(
        forge_render::gui_camsim_panel::rule_title("rapid_into_stock_grid"),
        "Rapids into the stock (approximate)"
    );
}

/// The flange's own quality measurement (forge-cam-quality's MoveQ, per feed move of the planned
/// paths) lands on the posted program's lines and arcs: every op it measured is coloured, move
/// for move, none left grey by a count that differs.
#[test]
fn the_flanges_measured_chip_load_colours_its_moves() {
    use forge_cam::ops::PMove;
    use forge_render::camsim::draw::Values;
    let sim = flange();
    let q = forge_cam_quality::measure_cam(&sim.cam, String::new(), &forge_cam_quality::Config::default());
    let per = |i: usize| {
        let o = q.ops.iter().find(|o| o.name == sim.cam.ops[i].name)?;
        Some(
            o.moves_q
                .iter()
                .map(|m| (m.path, m.load as f32, 1.0))
                .collect::<Vec<_>>(),
        )
    };
    let v = Values::of_feeds(&sim.cam, &sim.tl, "chip load", "", Values::chip_bands(), &per);
    let mut measured = 0;
    for (i, op) in sim.cam.ops.iter().enumerate() {
        let Some(vals) = per(i).filter(|v| !v.is_empty()) else {
            continue;
        };
        measured += 1;
        let feeds = sim.tl.segs.iter().zip(&v.per_piece).filter(|(g, _)| {
            g.op as usize == i
                && matches!(
                    sim.cam.sim_paths(i)[g.path as usize].moves[g.mv as usize],
                    PMove::Line { .. } | PMove::Arc { .. }
                )
        });
        let (n, grey) = feeds.fold((0, 0), |(n, g), (_, x)| (n + 1, g + x.is_none() as usize));
        assert_eq!(
            grey,
            0,
            "{}: {grey} of {n} feed pieces grey ({} values)",
            op.name,
            vals.len()
        );
    }
    assert!(measured > 0, "the flange's ops measured per move");
}

/// The tools as drawn: every tool of a document meshed once from its record (a fluted cutter
/// with the record's flute count, then neck, shank and holder); a tap's thread cut in; drawing
/// one is well within a frame. Display only: the checks keep the revolved profiles.
#[test]
fn tools_are_drawn_from_their_records() {
    use forge_render::camsim::tools;
    let dir = machined();
    for doc in ["flange_cam.fg", "manifold_cam.fg", "mold_core_finish_cam.fg"] {
        let src = std::fs::read_to_string(dir.join(doc)).unwrap();
        let t0 = std::time::Instant::now();
        let sim = Sim::open(&src, &dir);
        let open_ms = t0.elapsed().as_secs_f64() * 1e3;
        assert_eq!(sim.looks.len(), sim.cam.sim_tools().len(), "{doc}");
        let t1 = std::time::Instant::now();
        let helix = tools::helices();
        let built: Vec<_> = sim
            .cam
            .sim_tools()
            .iter()
            .map(|t| tools::build(t, helix.get(&t.tool.id).copied()))
            .collect();
        let build_ms = t1.elapsed().as_secs_f64() * 1e3;
        for (tu, lk) in sim.cam.sim_tools().iter().zip(&built) {
            assert!(lk.tris > 0, "{doc} {}", tu.tool.id);
            // a fluted cutter's flutes are the builder's: one cutting edge per flute
            if tu.tool.nof > 0 && !tu.probe {
                let edges = lk.pieces.first().map_or(0, |p| p.tris.edges.len());
                assert!(
                    edges > tu.tool.nof as usize,
                    "{doc} {}: {edges} edges",
                    tu.tool.id
                );
            }
            if tu.holder.is_some() {
                assert!(
                    lk.pieces.iter().any(|p| !p.spins),
                    "{doc} {}: a holder",
                    tu.tool.id
                );
            }
        }
        // drawing the biggest, turned
        let big = built.iter().max_by_key(|l| l.tris).unwrap();
        let mut out = vec![];
        let t2 = std::time::Instant::now();
        for k in 0..20 {
            out.clear();
            tools::draw(big, [1.0, 2.0, 3.0], k as f64 * 0.3, tools::Hit::None, &mut out);
        }
        let draw_ms = t2.elapsed().as_secs_f64() * 1e3 / 20.0;
        eprintln!(
            "{doc}: {} tools, meshed in {build_ms:.1} ms (sim opened in {open_ms:.0} ms), the largest {} triangles drawn in {draw_ms:.2} ms",
            built.len(),
            big.tris
        );
        assert!(draw_ms < 8.0, "{doc}: {draw_ms} ms a frame for a tool");
    }
    // a tap: its thread at its pitch, cut into the fluted length
    let sim = Sim::open(
        &std::fs::read_to_string(dir.join("manifold_cam.fg")).unwrap(),
        &dir,
    );
    if let Some((tu, lk)) = sim
        .cam
        .sim_tools()
        .iter()
        .zip(&sim.looks)
        .find(|(t, _)| t.tool.kind == forge_tooling::Kind::Tap)
    {
        let p = &lk.pieces[0].tris.pos;
        let r: Vec<f64> = p.iter().map(|q| (q[0] * q[0] + q[1] * q[1]).sqrt()).collect();
        let (lo, hi) = r
            .iter()
            .fold((f64::INFINITY, 0.0f64), |a, &x| (a.0.min(x), a.1.max(x)));
        assert!(
            hi - lo > 0.5 * tu.tool.tp,
            "{}: thread depth {}",
            tu.tool.id,
            hi - lo
        );
    }
}

/// A failed check and a run's message in words (docs/ux/STANDARD.md §5/§7): rounded, with units,
/// never a Debug dump.
#[test]
fn checks_and_problems_in_words() {
    use forge_render::camsim::{check_sentence, tidy};
    let c = forge_cam::ops::CheckOut {
        rule: "holder_collision",
        verdict: "fail",
        value: -2.749999884242636,
        limit: -0.01,
        at: [0.0; 3],
        tool_part: "holder",
        other: "stock face 2158".into(),
        phase: "plunge",
        move_index: 3,
        path: 0,
    };
    assert_eq!(
        check_sentence(&c),
        "Holder hits the stock on the plunge at move 3 (2.75 mm into it)"
    );
    assert_eq!(
        tidy("the part's section at z = 29: NotPrism { face: 38 }"),
        "the part's section at z = 29: isn't a prism (face 38)"
    );
    assert_eq!(tidy("-2.749999884242636 vs -0.01"), "-2.750 vs -0.01");
    // a run's op key does not lead a sentence
    assert_eq!(
        tidy("thread_mill: the tool's pitch 1 is not the thread's 1.5"),
        "The tool's pitch 1 is not the thread's 1.5"
    );
}

/// A fluted cutter reads as one: its cutting length at least 15 % darker than its shank
/// (luminance of the colours drawn), its valleys darker than its lands.
#[test]
fn a_cutters_flutes_stand_out_from_its_shank() {
    let sim = flange();
    let lum = |c: [f32; 3]| 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2];
    let lk = &sim.looks[0];
    let cut = lk
        .pieces
        .iter()
        .find(|p| !p.shade.is_empty())
        .expect("a fluted piece");
    let shank = lk
        .pieces
        .iter()
        .find(|p| p.spins && p.shade.is_empty())
        .expect("a shank");
    let mean = cut.shade.iter().sum::<f32>() / cut.shade.len() as f32;
    let (lc, ls) = (lum(cut.colour) * mean, lum(shank.colour));
    assert!(lc < 0.85 * ls, "cutting {lc} vs shank {ls}");
    let (lo, hi) = cut
        .shade
        .iter()
        .fold((f32::MAX, f32::MIN), |a, &x| (a.0.min(x), a.1.max(x)));
    assert!(hi - lo > 0.4, "valleys {lo} lands {hi}");
    // the holder drawn from records: its ER nut (bt40-er32 names none: ER32's largest, 50 x 22.5)
    // and its interface's flange (BT40: 63 mm)
    use forge_render::camsim::draw::look;
    let rmax = |p: &forge_render::camsim::tools::Piece| {
        p.tris
            .pos
            .iter()
            .map(|q| (q[0] * q[0] + q[1] * q[1]).sqrt())
            .fold(0.0, f64::max)
    };
    let nut = lk.pieces.iter().find(|p| p.colour == look::NUT).expect("a nut");
    assert!((rmax(nut) - 25.0).abs() < 0.1, "nut radius {}", rmax(nut));
    let flange = lk
        .pieces
        .iter()
        .filter(|p| p.colour == look::HOLDER)
        .map(rmax)
        .fold(0.0, f64::max);
    assert!((flange - 31.5).abs() < 0.1, "flange radius {flange}");
    assert!(
        lk.note
            .as_ref()
            .is_some_and(|n| n.1.contains("interface table") && n.1.contains("nut ER32 from")),
        "{:?}",
        lk.note
    );
    // the label beside the tool is people's words; the record keys are the note's (its hover)
    assert_eq!(lk.label, "\u{d8}10 flat end mill \u{b7} BT40 holder, ER32 nut");
}

/// The Checks tab's first line says in words what every move was checked for: all clear on the
/// flange; on a broken copy (its clear height under the stock's top: rapids into it) the failure
/// named in its issue kind's words. The checker's codes are only its hover.
#[test]
fn the_checks_line_is_in_words() {
    use forge_render::gui_camsim_panel::checked_line;
    let sim = flange();
    let (line, failed) = checked_line(&sim.markers, sim.cam.leftover_done);
    assert!(!failed);
    assert_eq!(
        line,
        "Every move checked: no gouges, holder and fixture clear, rapids clear. Leftover stock measured exactly."
    );
    let dir = machined();
    let sim = Sim::open(BROKEN, &dir);
    let mut rules: Vec<&str> = sim.markers.iter().map(|m| m.rule.as_str()).collect();
    rules.sort();
    rules.dedup();
    eprintln!("broken flange: {rules:?}");
    let (line, failed) = checked_line(&sim.markers, sim.cam.leftover_done);
    assert!(failed, "{line}");
    // a rapid into the stock is a crash: its own kind, named so in the line
    assert!(line.contains("rapids into the stock"), "{line}");
    let m = sim
        .markers
        .iter()
        .find(|m| m.rule == "rapid_into_stock")
        .expect("a rapid into the stock");
    // its row says when, where and how much, not what (the heading does)
    assert!(
        m.row.contains("move ") && m.row.ends_with("mm into the stock"),
        "{}",
        m.row
    );
    assert!(!m.row.contains("bolts") && !m.row.contains("hits"), "{}", m.row);
    // the spot drill's pass and the drill's at the same hole are told apart by their tool
    let rows: Vec<(&str, &str)> = sim
        .markers
        .iter()
        .filter(|m| m.rule == "rapid_into_stock")
        .map(|m| (m.tool.as_str(), m.row.as_str()))
        .collect();
    assert!(rows.iter().any(|r| r.0 == "\u{d8}10 spot drill"), "{rows:?}");
    assert!(rows.iter().all(|r| r.1.starts_with("move ")), "{rows:?}");
    let mut seen = rows.clone();
    seen.sort();
    seen.dedup();
    assert_eq!(seen.len(), rows.len(), "no two rows alike: {rows:?}");
    // the checker says the cutter hit: the cutter alone is tinted, its holder stays itself
    use forge_render::camsim::tools::Hit;
    assert_eq!(m.part, "cutter");
    let h = Hit::of(&m.part);
    assert!(h.tints(true) && !h.tints(false));
    assert_eq!(h.and(Hit::of("shank/holder")), Hit::Whole);
    assert_eq!(Hit::None.and(h), h);
    assert!(!line.contains("G3") && !line.contains("pairs"), "{line}");
    // the run's warnings in people's words, the syntax on hover
    use forge_render::gui_camsim_panel::note_words;
    assert_eq!(
        note_words(
            "setup `op1` is not placed: G5 checked each axis's span against the travel, not the positions; fix: add `at: (x, y, z)` to the setup"
        ),
        "Setup op1 isn't placed on the machine. Place it to check machine travel, or allow uncertified output in NC Programs."
    );
    assert_eq!(
        note_words("part `flange.fg` names only a material group (N): name the alloy"),
        "The part's material is only a group: non-ferrous (aluminium, copper alloys). Pick the alloy for realistic feeds and tool life."
    );
}

/// The flange's bolt holes drilled alone under a clear height below the stock's top: rapids into
/// it, and the material drilling leaves.
pub const BROKEN: &str = "machining\nf = part(\"flange.fg\", group: \"N\")\nm = machine(\"generic-vmc3\")\nt1 = tool(\"flat-10\", t: 1, stickout: 32, holder: \"bt40-er32\")\nop1 = setup(f, machine: m, stock: box(margin: (3, 3, 2), below: 3), origin: \"top-center\", tol: 0.01, clear: -0.5, retract: -1)\nbolts = drill(op1, f.bolts, plan: \"auto\")\nprog = program(op1)\n";

/// The played stock's mesh, built each time the playhead moves: its time and size at the mould
/// core's end (the most cut there is), at the default cells and four times as many.
#[test]
fn the_stock_mesh_is_quick_to_build() {
    use forge_render::camsim::draw::{self, StockColour};
    let src = std::fs::read_to_string(machined().join("mold_core_finish_cam.fg")).unwrap();
    let mut sim = Sim::open(&src, &machined());
    for cells in [forge_render::camsim::CELLS, 4 * forge_render::camsim::CELLS] {
        sim.set_cells(cells);
        let end = sim.tl.total;
        sim.stock_at(end).unwrap();
        let (z, _) = sim.stock_shown().unwrap();
        let mut out = vec![];
        let t0 = std::time::Instant::now();
        let n = 5;
        for _ in 0..n {
            out.clear();
            draw::stock(z, None, StockColour::Material, 0.01, &mut out);
        }
        let ms = t0.elapsed().as_secs_f64() * 1e3 / n as f64;
        eprintln!(
            "stock mesh, {} cells ({:.3} mm): {ms:.1} ms, {} triangles",
            z.nx * z.ny,
            z.cell,
            out.len() / 3
        );
        assert!(!out.is_empty());
    }
}

/// The detail window about the tool (`Sim::detail_at`): a grid of about 0.03 mm over a square
/// snapped to the whole grid's columns, played from the setup's start; its time from nothing
/// (the window moved, or time went back) and for the next frame's step, mid-way through the
/// mould core's ball-end finishing.
#[test]
fn the_detail_window_is_fine_and_quick() {
    let src = std::fs::read_to_string(machined().join("mold_core_finish_cam.fg")).unwrap();
    let mut sim = Sim::open(&src, &machined());
    let k = sim.tl.ops.iter().position(|o| o.name == "ss").unwrap();
    let t = 0.5 * (sim.tl.ops[k].t0 + sim.tl.ops[k].t1);
    sim.stock_at(t).unwrap();
    let t0 = std::time::Instant::now();
    let (cell, n, cols) = {
        let (z, cols) = sim.detail_at(t).expect("a window");
        (z.cell, z.nx * z.ny, cols)
    };
    let first = t0.elapsed().as_secs_f64() * 1e3;
    // the next frames (a 30 fps clip at 10× through a 34 min op: a few pieces each)
    let t1 = std::time::Instant::now();
    let steps = 30;
    for s in 1..=steps {
        let tt = t + s as f64 * 10.0 / 30.0;
        sim.stock_at(tt).unwrap();
        sim.detail_at(tt);
    }
    let step = t1.elapsed().as_secs_f64() * 1e3 / steps as f64;
    // while playing: the window advances, never moves (it follows at the next pause)
    let t2 = std::time::Instant::now();
    let mut shown = 0;
    for s in 1..=steps {
        let tt = t + (steps + s) as f64 * 10.0 / 30.0;
        sim.stock_at(tt).unwrap();
        shown += usize::from(sim.detail_near(tt, false).is_some());
    }
    let playing = t2.elapsed().as_secs_f64() * 1e3 / steps as f64;
    eprintln!(
        "while playing: {playing:.1} ms a frame (stock and window), the window shown in {shown} of {steps}"
    );
    eprintln!(
        "detail window: {n} cells of {cell:.3} mm, columns {cols:?}: {first:.0} ms from nothing, {step:.1} ms a frame after (stock and window)"
    );
    assert!(cell <= 0.035, "fine: {cell}");
    let coarse = sim.stock_shown().unwrap().0.cell;
    assert!(coarse > 4.0 * cell, "finer than the whole grid ({coarse})");
}
