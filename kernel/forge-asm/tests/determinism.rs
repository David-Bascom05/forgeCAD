//! Same bits every time (prior-art §5.4, §11.5, §11.6): across thread counts, across the order the
//! document lists its lines in, and across targets (this digest is checked natively and as wasm).

mod common;
use common::*;
use forge_asm::*;
use forge_math::Frame;
use forge_math::hash::BitHasher;
use forge_math::rng::Rng;

fn corpus() -> Vec<Assembly> {
    let mut out = Vec::new();
    for i in 0..6 {
        let t = 0.3 + 0.4 * i as f64;
        let mut fb = four_bar(40.0, 120.0, 80.0, 100.0, 0.6);
        fb.joints[0] = fb.joints[0].clone().driven(Freedom::Rz, t);
        out.push(fb);
        let mut sc = slider_crank(30.0, 100.0, 0.4);
        sc.joints[0] = sc.joints[0].clone().driven(Freedom::Rz, t);
        out.push(sc);
    }
    // many small mechanisms in one document: many partitions for the threads
    let mut big = Assembly::default();
    big.instances.push(inst(1, Frame::WORLD).grounded());
    for m in 0..40u32 {
        let fb = four_bar(40.0 + m as f64, 120.0, 80.0, 100.0, 0.6);
        let o = at(0.0, 300.0 * m as f64, 0.0);
        let id = |i: u32| if i == 1 { 1 } else { 10 * m + i + 10 };
        for x in &fb.instances[1..] {
            big.instances.push(inst(id(x.id.0), o.compose(&x.pose)));
        }
        for j in &fb.joints {
            let e = |e: &End| {
                let AnchorRef::Local(r) = &e.anchor else {
                    unreachable!()
                };
                let f = if e.path[0].0 == 1 {
                    o.compose(&r.frame)
                } else {
                    r.frame
                };
                End::local(&[id(e.path[0].0)], f)
            };
            let mut nj = Joint::new(10 * m + j.id.0, j.kind, e(&j.a), e(&j.b));
            if j.id.0 == 1 {
                nj = nj.driven(Freedom::Rz, 0.2 + 0.05 * m as f64);
            }
            big.joints.push(nj);
        }
    }
    out.push(big);
    out
}

fn digest(threads: usize) -> u64 {
    let mut h = BitHasher::new();
    let parts = catalog();
    for asm in corpus() {
        let sol = solve(&asm, &parts, &Options { threads });
        for (id, f) in &sol.poses {
            h.u64(id.0 as u64);
            h.p3(f.origin);
            h.v3(f.x().get());
            h.v3(f.z().get());
        }
        h.u64(sol.mobility as u64);
        h.u64(sol.diagnostics.len() as u64);
    }
    h.finish()
}

#[test]
fn bits_are_the_same_on_every_target_and_thread_count() {
    let one = digest(1);
    println!("digest {one:#x}");
    for t in [2, 4, 8] {
        assert_eq!(digest(t), one, "{t} threads");
    }
    assert_eq!(one, EXPECTED);
}

const EXPECTED: u64 = 0x76b6_fb1b_6623_c86d;

#[test]
fn the_order_lines_are_written_in_changes_no_bit() {
    let parts = catalog();
    let mut r = Rng::new(0x5_4ff1e);
    for asm in corpus() {
        let base = solve(&asm, &parts, &Options { threads: 1 });
        for _ in 0..3 {
            let mut s = asm.clone();
            for i in (1..s.instances.len()).rev() {
                s.instances.swap(i, r.below(i + 1));
            }
            for i in (1..s.joints.len()).rev() {
                s.joints.swap(i, r.below(i + 1));
            }
            let sol = solve(&s, &parts, &Options { threads: 1 });
            assert_eq!(sol.poses, base.poses);
            assert_eq!(sol.diagnostics, base.diagnostics);
        }
    }
}
