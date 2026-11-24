//! An arc's box is its ends' box grown only to the circle's extremes its span may hold (decided
//! in interval signs, an undecided extreme taken), not its whole circle's. TOOLPATH-QUALITY's
//! repro (a pocket pass's region, validated in 6.1 s): corner arcs tangent to the boss's offset
//! circle met that circle's arcs in other quadrants, boxes overlapping by the whole circle, and
//! each near-tangent meet was decided at full precision (0.4 s); the arcs' own boxes are apart.
use forge_cam_geom::arr;
use forge_cam_geom::real::Ctx;
use forge_cam_geom::region::{self, Loop, Region, Seg, p2};

fn one(s: Seg) -> Loop {
    Loop { segs: vec![s] }
}

#[test]
fn far_arcs_of_tangent_circles_are_not_met() {
    let corner = Seg::Arc {
        a: p2(-31.13513513514772, -7.389408644521609),
        b: p2(-41.0, -8.544003745308146),
        c: p2(-36.0, -8.544003745308146),
        ccw: true,
    };
    let boss = Seg::Arc {
        a: p2(-31.13513513514772, 7.389408644521609),
        b: p2(-7.389408644521609, 31.13513513514772),
        c: p2(-0.0, -0.0),
        ccw: false,
    };
    let mut cx = Ctx::new();
    let ex = region::exact(
        &mut cx,
        &Region {
            loops: vec![one(corner), one(boss)],
        },
    )
    .unwrap();
    let (a, b) = (&ex.elems[0].piece, &ex.elems[1].piece);
    let before = cx.exact_evals;
    let (mut p, mut q) = (vec![], vec![]);
    arr::meet(&mut cx, a, b, &mut p, &mut q);
    assert!(p.is_empty() && q.is_empty());
    assert_eq!(cx.exact_evals, before, "decided by the boxes, no exact sign");
}

/// The box holds the arc: points along random arcs (either sense, spans up to nearly a whole
/// turn) inside it, to within the sampling's own rounding. Points by the rational map
/// u ↦ ((1 − u²), 2u)/(1 + u²), monotone in the angle on (−π, π).
#[test]
fn an_arcs_box_holds_the_arc() {
    let mut seed = 0x9e3779b97f4a7c15u64;
    let mut rnd = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        (seed >> 11) as f64 / (1u64 << 53) as f64
    };
    for _ in 0..400 {
        let (cx0, cy0, r) = (rnd() * 20.0 - 10.0, rnd() * 20.0 - 10.0, 0.5 + rnd() * 9.0);
        // u in (−30, 30): angles within about ±3.07 rad
        let (mut u0, mut u1) = (rnd() * 60.0 - 30.0, rnd() * 60.0 - 30.0);
        if u0 > u1 {
            std::mem::swap(&mut u0, &mut u1);
        }
        if u1 - u0 < 1e-3 {
            continue;
        }
        let ccw = rnd() < 0.5;
        let at = |u: f64| {
            let d = 1.0 + u * u;
            (cx0 + r * (1.0 - u * u) / d, cy0 + r * 2.0 * u / d)
        };
        // counter-clockwise from u0 to u1, or clockwise from u1 to u0: the same points
        let (a, b) = if ccw { (at(u0), at(u1)) } else { (at(u1), at(u0)) };
        let seg = Seg::Arc {
            a: p2(a.0, a.1),
            b: p2(b.0, b.1),
            c: p2(cx0, cy0),
            ccw,
        };
        let mut cx = Ctx::new();
        let ex = region::exact(
            &mut cx,
            &Region {
                loops: vec![one(seg)],
            },
        )
        .unwrap();
        let bx = arr::piece_box(&cx, &ex.elems[0].piece);
        for k in 0..=64 {
            let p = at(u0 + (u1 - u0) * k as f64 / 64.0);
            let eps = 1e-9 * (1.0 + r);
            assert!(
                p.0 >= bx.0 - eps && p.0 <= bx.2 + eps && p.1 >= bx.1 - eps && p.1 <= bx.3 + eps,
                "{p:?} outside {bx:?} (u {u0} to {u1}, ccw {ccw})"
            );
        }
    }
}
