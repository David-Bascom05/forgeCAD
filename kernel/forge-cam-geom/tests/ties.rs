//! An offset of a round-numbered region, whose pieces meet in exact ties, held to a bound on the
//! exact-real refinement's work (`Stats::refined_bits`): a tie among several square roots is
//! decided exactly in their tower (forge-num real.rs `tower_sign`), not by refinement to the
//! separation bound.
use forge_cam_geom::offset::offset;
use forge_cam_geom::region::{Loop, Region, Seg, p2};

/// Four quarter arcs of one circle, clockwise.
fn quarters(cx: f64, cy: f64, rad: f64) -> Loop {
    let c = p2(cx, cy);
    let (e, s, w, n) = (
        p2(cx + rad, cy),
        p2(cx, cy - rad),
        p2(cx - rad, cy),
        p2(cx, cy + rad),
    );
    Loop {
        segs: vec![
            Seg::Arc {
                a: e,
                b: s,
                c,
                ccw: false,
            },
            Seg::Arc {
                a: s,
                b: w,
                c,
                ccw: false,
            },
            Seg::Arc {
                a: w,
                b: n,
                c,
                ccw: false,
            },
            Seg::Arc {
                a: n,
                b: e,
                c,
                ccw: false,
            },
        ],
    }
}

/// From CAM's deep_boss_n_adaptive with a round stock box (min (-3, -3, 0), max (73, 73, 26)):
/// loop_band offsets this circle by 3, past its radius, and its pieces' ends tie exactly among
/// three square roots. By refinement alone that took 1,397,760 bits (11 s here, 17 s of the
/// program's certification); in the tower, 24,576 bits (the first precisions of the few values
/// that need any).
#[test]
fn a_tied_offset_stays_within_a_work_bound() {
    let rg = Region {
        loops: vec![quarters(44.25, 14.25, 2.642)],
    };
    let (out, st) = offset(&rg, 3.0).unwrap();
    assert_eq!(out.len(), 1, "{out:?}");
    assert!(st.refined_bits <= 100_000, "{st:?}");
}
