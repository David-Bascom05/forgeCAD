use forge_cam_geom::check::{Face, Move, Surf};
use forge_cam_geom::drop::{Column, drop, gap_move};
use forge_cam_geom::ival::Ival;
use forge_cam_geom::tool::{Cutter, assemble};
fn main() {
    let sph = Face::new(Surf::Sphere { o: [0.0; 3], x: [1.0, 0.0, 0.0], y: [0.0, 1.0, 0.0], ax: [0.0, 0.0, 1.0], r: 10.0 },
        Ival::new(-1.0, 1.0), Ival::new(0.0, 1.0), None, 1e-7, 0);
    let tor = Face::new(Surf::Torus { o: [0.0; 3], x: [1.0, 0.0, 0.0], y: [0.0, 1.0, 0.0], ax: [0.0, 0.0, 1.0], big: 20.0, r: 5.0, cm: 0.0, sm: 1.0 },
        Ival::new(-1.0, 1.0), Ival::new(-1.0, 1.0), None, 1e-7, 1);
    for (name, c) in [("ball6", Cutter::Ball { d: 6.0 }), ("flat6", Cutter::Flat { d: 6.0 }), ("bull8r1", Cutter::Bull { d: 8.0, re: 1.0 })] {
        let b = assemble(&c, 15.0, 2.0 * c.radius(), 30.0, None).unwrap().bodies[0].clone();
        let col = Column::of(&b).unwrap();
        for (fname, f) in [("sphere", &sph), ("torus", &tor)] {
            let t = std::time::Instant::now();
            let mut cells = 0; let mut n = 0; let mut wmax: f64 = 0.0;
            let mut pts = vec![];
            for i in 0..50 { let x = -9.0 + 18.0 * i as f64 / 49.0; let y = if fname == "torus" { 20.0 } else { 0.3 };
                let cl = drop(&col, std::slice::from_ref(f), &[0], x, y, 1e-3, 1_000_000); cells += cl.cells; n += 1; wmax = wmax.max(cl.hi - cl.lo); pts.push([x, y, cl.hi]); }
            let dt = t.elapsed();
            // moves between consecutive CL points, lifted by 0
            let t2 = std::time::Instant::now(); let mut mc = 0; let mut ok = 0;
            for w in pts.windows(2) { if !w[0][2].is_finite() || !w[1][2].is_finite() { continue; } let mv = Move::Line { a: w[0], b: w[1] };
                let (g, c) = gap_move(&col, f, &mv, -0.01, 1e-4, 1_000_000); mc += c; if matches!(g, forge_cam_geom::drop::Gap::Clear{..}) { ok += 1; } else if false { let mut mx = f64::NEG_INFINITY; for j in 0..=200 { let t = j as f64/200.0; let px = w[0][0]+t*(w[1][0]-w[0][0]); let pz = w[0][2]+t*(w[1][2]-w[0][2]); let cl = drop(&col, std::slice::from_ref(f), &[0], px, w[0][1], 1e-6, 1_000_000); mx = mx.max(cl.hi - pz); } println!("  {fname} {g:?} {c} a={:?} b={:?} max u sampled {mx:.3e}", w[0], w[1]); } }
            println!("{name} {fname}: drop {n} pts {cells} cells {:?} maxw {wmax:.2e}; moves ok {ok}/{} cells {mc} {:?}", dt, pts.len()-1, t2.elapsed());
        }
    }
}
#[allow(dead_code)]
fn dbg() {}
