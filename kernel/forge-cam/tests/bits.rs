//! Native = wasm, bit for bit: one digest over exact offsets, booleans, checker verdicts and a
//! posted program's text, pinned here and checked on both targets (cargo test --target
//! wasm32-wasip1 -p forge-cam --test bits).
use forge_cam_geom::boolean::{Op, boolean};
use forge_cam_geom::check::{Face, Move, Surf, Verdict, certify};
use forge_cam_geom::ival::Ival;
use forge_cam_geom::offset::offset;
use forge_cam_geom::region::{Loop, Region, Seg, p2};
use forge_cam_geom::tool::{Cutter, assemble};

fn fnv(h: &mut u64, b: &[u8]) {
    for x in b {
        *h ^= *x as u64;
        *h = h.wrapping_mul(0x100000001b3);
    }
}

fn segs(h: &mut u64, ss: &[Seg]) {
    for s in ss {
        let (a, b) = (s.start(), s.end());
        for v in [a.x, a.y, b.x, b.y] {
            fnv(h, &v.to_bits().to_le_bytes());
        }
        if let Seg::Arc { c, ccw, .. } = s {
            fnv(h, &c.x.to_bits().to_le_bytes());
            fnv(h, &c.y.to_bits().to_le_bytes());
            fnv(h, &[*ccw as u8]);
        }
    }
}

const PINNED: u64 = 0xac6472cf64e5f871;

#[test]
fn the_same_bits_everywhere() {
    let mut h = 0xcbf29ce484222325u64;
    let l = Loop {
        segs: vec![
            Seg::Line {
                a: p2(0.0, 0.0),
                b: p2(31.7, 0.0),
            },
            Seg::Arc {
                a: p2(31.7, 0.0),
                b: p2(31.7, 13.1),
                c: p2(31.7, 6.55),
                ccw: true,
            },
            Seg::Line {
                a: p2(31.7, 13.1),
                b: p2(12.3, 21.9),
            },
            Seg::Line {
                a: p2(12.3, 21.9),
                b: p2(0.0, 0.0),
            },
        ],
    };
    let a0 = p2(19.0, 8.0);
    let isl = Loop {
        segs: vec![Seg::Arc {
            a: a0,
            b: a0,
            c: p2(17.0, 8.0),
            ccw: false,
        }],
    };
    let rg = Region { loops: vec![l, isl] };
    for r in [0.7, 1.9, 3.1, -2.2] {
        let (out, _) = offset(&rg, r).unwrap();
        for o in &out {
            segs(&mut h, &o.segs);
        }
    }
    let disc = Region {
        loops: vec![Loop {
            segs: vec![Seg::Arc {
                a: p2(34.0, 6.0),
                b: p2(34.0, 6.0),
                c: p2(30.0, 6.0),
                ccw: true,
            }],
        }],
    };
    for op in [Op::Union, Op::Inter, Op::Diff] {
        for o in boolean(&rg, &disc, op).unwrap() {
            segs(&mut h, &o.segs);
        }
    }
    let t = assemble(&Cutter::Bull { d: 8.0, re: 1.5 }, 20.0, 8.0, 30.0, None).unwrap();
    let f = Face::new(
        Surf::Cyl {
            o: [0.0, 0.0, -10.0],
            x: [1.0, 0.0, 0.0],
            y: [0.0, 1.0, 0.0],
            ax: [0.0, 0.0, 1.0],
            r: 7.3,
        },
        Ival::new(-0.9, 0.9),
        Ival::new(0.0, 10.0),
        None,
        0.0,
        0,
    );
    for rr in [11.29, 11.3, 11.31] {
        let mv = Move::Arc {
            c: [0.0, 0.0, -4.0],
            x: [1.0, 0.0, 0.0],
            y: [0.0, 1.0, 0.0],
            r: rr,
            t0: -0.4,
            t1: 0.4,
        };
        let (v, n) = certify(&t.bodies[0], &f, &mv, -0.01, 0.0025, 10_000);
        let x = match v {
            Verdict::Clear { lower } => lower,
            Verdict::Hit { d, .. } => d,
            Verdict::Undecided { lower, .. } => lower,
        };
        fnv(&mut h, &x.to_bits().to_le_bytes());
        fnv(&mut h, &(n as u64).to_le_bytes());
    }
    let p = forge_post::Program {
        number: 7,
        title: "bits".into(),
        recs: vec![
            forge_post::Rec::ToolChange {
                t: 1,
                h: 1,
                name: "x".into(),
            },
            forge_post::Rec::Spindle {
                rpm: 12345.6,
                cw: true,
            },
            forge_post::Rec::Rapid([1.0 / 3.0, 2.0 / 7.0, 5.0]),
            forge_post::Rec::Line([0.1 + 0.2, 0.7, -1.0005], 333.33),
            forge_post::Rec::Arc {
                to: [5.3, 0.7, -1.0005],
                c: [2.8, 0.7],
                cw: true,
                f: 400.0,
            },
            forge_post::Rec::SpindleStop,
        ],
    };
    for (d, _) in forge_post::DIALECTS {
        let dl = forge_post::Dialect::get(d).unwrap();
        let q = forge_post::quantize(&p, &dl).unwrap();
        let (txt, _) = forge_post::round_trip(&q, &dl).unwrap();
        fnv(&mut h, txt.as_bytes());
    }
    if PINNED == 0 {
        panic!("digest {h:#x}: pin it");
    }
    assert_eq!(h, PINNED, "digest {h:#x}");
}
