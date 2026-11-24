//! Times this crate's pieces on the mould core through SurfDrop (cargo run --release -p
//! forge-cam-3d --example mould_time).
use forge_cam::surf::Surfacer;
use forge_cam_3d::drape::{Drape, drape};
use forge_cam_3d::drop::Band;
use forge_cam_3d::drop::DropCutter;
use forge_cam_3d::surfdrop::SurfDrop;
use forge_cam_3d::{pattern, steep, waterline};
use forge_tooling::Library;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;
struct Count<'a> {
    i: &'a dyn DropCutter,
    n: [AtomicU64; 3],
    t: [AtomicU64; 3],
}
impl Count<'_> {
    fn tick<T>(&self, k: usize, f: impl FnOnce() -> T) -> T {
        let t0 = Instant::now();
        let r = f();
        self.n[k].fetch_add(1, Ordering::Relaxed);
        self.t[k].fetch_add(t0.elapsed().as_nanos() as u64, Ordering::Relaxed);
        r
    }
}
impl DropCutter for Count<'_> {
    fn drop(&self, x: f64, y: f64) -> Option<Band> {
        self.tick(0, || self.i.drop(x, y))
    }
    fn drop_seg(&self, a: [f64; 2], b: [f64; 2]) -> Option<Band> {
        self.i.drop_seg(a, b)
    }
    fn seg_below(&self, a: [f64; 2], b: [f64; 2], z: f64) -> bool {
        self.tick(1, || self.i.seg_below(a, b, z))
    }
    fn seg3_below(&self, a: [f64; 3], b: [f64; 3], s: f64) -> bool {
        let t0 = Instant::now();
        let r = self.tick(2, || self.i.seg3_below(a, b, s));
        let dt = t0.elapsed().as_secs_f64();
        if dt > 5.0 {
            println!("  slow seg3 {dt:.2} s {a:?} -> {b:?} = {r}");
        }
        r
    }
    fn extent(&self) -> [[f64; 2]; 2] {
        self.i.extent()
    }
}
fn main() {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/machined/mold_core.fg");
    let src = std::fs::read_to_string(p).unwrap();
    let (run, _) = forge_lang::run(&src, forge_lang::Options::default(), Default::default());
    let faces = forge_cam::lower3d::lower_surfacing(&run.body.unwrap()).unwrap().0;
    let l = Library::builtin();
    let t = l.tool("ball-6").unwrap().clone();
    let shape = t.shape(40.0, Some(l.holder("bt40-er32").unwrap())).unwrap();
    let sf = Surfacer::new(&faces, &shape.bodies[0], 0.0, 0.01, 10.0).unwrap();
    let dc = SurfDrop::new(&sf).unwrap();
    println!("extent {:?}", dc.extent());
    let t0 = Instant::now();
    let mut n = 0;
    for i in 0..100 {
        let _ = dc.drop(-30.0 + 0.6 * i as f64, 3.0);
        n += 1;
    }
    println!("drop: {:.3} ms each", t0.elapsed().as_secs_f64() * 1e3 / n as f64);
    let t0 = Instant::now();
    let mut ok = 0;
    for i in 0..100 {
        let x = -30.0 + 0.6 * i as f64;
        if dc.seg_below([x, 3.0], [x + 0.5, 3.0], 27.0) {
            ok += 1;
        }
    }
    println!(
        "seg_below: {:.3} ms each ({ok} clear)",
        t0.elapsed().as_secs_f64() * 1e3 / 100.0
    );
    let t0 = Instant::now();
    let mut ok = 0;
    for i in 0..100 {
        let x = -30.0 + 0.6 * i as f64;
        let a = dc.drop(x, 3.0).unwrap().hi;
        let b = dc.drop(x + 0.5, 3.0).unwrap().hi;
        if dc.seg3_below([x, 3.0, a], [x + 0.5, 3.0, b], 0.004) {
            ok += 1;
        }
    }
    println!(
        "seg3_below (incl 2 drops): {:.3} ms each ({ok} clear)",
        t0.elapsed().as_secs_f64() * 1e3 / 100.0
    );
    let t0 = Instant::now();
    let sm = steep::slope_map(&dc, 1.0);
    println!(
        "slope map ({} nodes): {:.1} s",
        sm.tan.len(),
        t0.elapsed().as_secs_f64()
    );
    let t0 = Instant::now();
    let wc = Count {
        i: &dc,
        n: Default::default(),
        t: Default::default(),
    };
    let ls = waterline::level(
        &wc,
        20.0,
        waterline::Params {
            grid: 1.0,
            tol_xy: 0.01,
        },
    )
    .unwrap();
    println!(
        "waterline z=20: {} loops, {} pts, {:.1} s",
        ls.len(),
        ls.iter().map(|l| l.pts.len()).sum::<usize>(),
        t0.elapsed().as_secs_f64()
    );
    println!(
        "  waterline calls drop/seg_below/seg3: {:?} secs {:?}",
        wc.n.iter().map(|c| c.load(Ordering::Relaxed)).collect::<Vec<_>>(),
        wc.t.iter()
            .map(|c| c.load(Ordering::Relaxed) as f64 * 1e-9)
            .collect::<Vec<_>>()
    );
    let t0f = Instant::now();
    let fc = Count {
        i: &dc,
        n: Default::default(),
        t: Default::default(),
    };
    let fl = forge_cam_3d::strategy::flat(
        &fc,
        &[10.0],
        0.0,
        0.49,
        0.12,
        0.01,
        &forge_cam_3d::contain::Containment::default(),
    );
    println!(
        "flat at 10: {} passes {:.1} s; calls drop/seg_below/seg3: {:?} secs {:?}",
        fl.len(),
        t0f.elapsed().as_secs_f64(),
        fc.n.iter().map(|c| c.load(Ordering::Relaxed)).collect::<Vec<_>>(),
        fc.t.iter()
            .map(|c| c.load(Ordering::Relaxed) as f64 * 1e-9)
            .collect::<Vec<_>>()
    );
    // the chords of that line at 2 mm, each check timed
    let mut worst = Vec::new();
    let mut x = -43.0;
    while x < 43.0 {
        let a = [x, 2.5, dc.drop(x, 2.5).unwrap().hi];
        let b = [x + 2.0, 2.5, dc.drop(x + 2.0, 2.5).unwrap().hi];
        let t0 = Instant::now();
        let ok = dc.seg3_below(a, b, 0.01);
        let t1 = Instant::now();
        let m = [x + 1.0, 2.5];
        let _ = dc.drop(m[0], m[1]);
        let t2 = Instant::now();
        worst.push((
            (t1 - t0).as_secs_f64(),
            (t2 - t1).as_secs_f64(),
            x,
            ok,
            a[2],
            b[2],
        ));
        x += 2.0;
    }
    worst.sort_by(|p, q| q.0.total_cmp(&p.0));
    for w in worst.iter().take(6) {
        println!(
            "seg3 {:.3} s drop {:.4} s at x {} ok {} z {:.3}->{:.3}",
            w.0, w.1, w.2, w.3, w.4, w.5
        );
    }
    let t0 = Instant::now();
    let d = Drape {
        step: 2.0,
        slack: 0.01,
        leave: 0.01,
        zmin: 10.0,
    };
    let mut pts = 0;
    let cc = Count {
        i: &dc,
        n: Default::default(),
        t: Default::default(),
    };
    for line in pattern::raster([-43.0, 2.0], [43.0, 3.0], 0.0, 2.0, true) {
        for ps in drape(&cc, &line, d).unwrap() {
            pts += ps.len();
        }
    }
    println!(
        "one raster line draped: {pts} pts, {:.1} s",
        t0.elapsed().as_secs_f64()
    );
    println!(
        "  calls drop/seg_below/seg3: {:?} secs {:?}",
        cc.n.iter().map(|c| c.load(Ordering::Relaxed)).collect::<Vec<_>>(),
        cc.t.iter()
            .map(|c| c.load(Ordering::Relaxed) as f64 * 1e-9)
            .collect::<Vec<_>>()
    );
}
