//! Native = wasm, bit for bit: one digest over a turned part's half-profile, the ops' paths (the
//! exact nose-radius compensation included), the certification's verdicts and leftover, and the
//! posted text in every lathe dialect, pinned here and checked on both targets (cargo test
//! --target wasm32-wasip1 -p forge-turn --test bits).
mod common;
use forge_cam_geom::region::Seg;
use forge_turn::ops::{self, Step};
use forge_turn::post;

fn fnv(h: &mut u64, b: &[u8]) {
    for x in b {
        *h ^= *x as u64;
        *h = h.wrapping_mul(0x100000001b3);
    }
}

fn f(h: &mut u64, v: f64) {
    fnv(h, &v.to_bits().to_le_bytes());
}

fn segs(h: &mut u64, ss: &[Seg]) {
    for s in ss {
        let (a, b) = (s.start(), s.end());
        for v in [a.x, a.y, b.x, b.y] {
            f(h, v);
        }
        if let Seg::Arc { c, ccw, .. } = s {
            f(h, c.x);
            f(h, c.y);
            fnv(h, &[*ccw as u8]);
        }
    }
}

const PINNED: u64 = 0x412ecc512dfefddb;

#[test]
fn the_same_bits_everywhere() {
    let mut h = 0xcbf29ce484222325u64;
    let s = common::setup();
    for l in &s.part.region.loops {
        segs(&mut h, &l.segs);
    }
    f(&mut h, s.part.mu);
    let job = common::job(&s);
    for op in &job {
        for st in &op.steps {
            match *st {
                Step::Rapid(p) | Step::Feed(p) => {
                    f(&mut h, p[0]);
                    f(&mut h, p[1]);
                }
                Step::Arc { to, c, ccw } => {
                    for v in [to[0], to[1], c[0], c[1]] {
                        f(&mut h, v);
                    }
                    fnv(&mut h, &[ccw as u8]);
                }
                Step::Thread { to, pitch } => {
                    for v in [to[0], to[1], pitch] {
                        f(&mut h, v);
                    }
                }
            }
        }
    }
    let rep = ops::certify(&s, &job, None).unwrap();
    fnv(&mut h, &(rep.checks as u64).to_le_bytes());
    f(&mut h, rep.least);
    fnv(&mut h, &(rep.refusals.len() as u64).to_le_bytes());
    for l in &rep.leftover {
        for lp in &l.loops {
            segs(&mut h, &lp.segs);
        }
    }
    for (name, _) in post::DIALECTS {
        let d = post::Dialect::get(name).unwrap();
        let q = post::quantize(&job, 1, "shaft", &d).unwrap();
        let (text, _) = post::round_trip(&q, &d).unwrap();
        fnv(&mut h, text.as_bytes());
    }
    assert_eq!(h, PINNED, "digest {h:#018x}");
}
