//! An L-shaped vertical wall (a vertical plane whose trim is not its parameter box: the side of
//! an open step flush with the outline) is checked first as the wall over its box, a prism that
//! holds it (slab.rs `Prism::hull_wall`). Every Clear `certify_shared` gives is below dense
//! samples of the true face; a gouge of the true face is never cleared; the hull is formed only
//! for a vertical plane with such a trim; and a corner arc tangent to the wall's end edge is
//! decided in few cells where the branch and bound took about 200,000.
use forge_cam_geom::check::{Face, Move, Surf, TSeg, Trim, Verdict, certify_bb, certify_shared};
use forge_cam_geom::ival::Ival;
use forge_cam_geom::slab::Prism;
use forge_cam_geom::tool::{Body, Cutter, assemble};

struct Rng(u64);
impl Rng {
    fn f(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 11) as f64) / ((1u64 << 53) as f64)
    }
    fn r(&mut self, a: f64, b: f64) -> f64 {
        a + (b - a) * self.f()
    }
}

fn line(a: [f64; 2], b: [f64; 2]) -> TSeg {
    TSeg::Line { a, b }
}

/// The wall y = 50 (its points o + p u + q v, u = −x, v = +z, o = (80, 50, 0)), over p in
/// [0, 80] and q in [0, 20], less the corner p > 40, q > 10: an L, counter-clockwise in (p, q).
fn l_wall() -> Face {
    let pts = [
        [0.0, 0.0],
        [80.0, 0.0],
        [80.0, 20.0],
        [40.0, 20.0],
        [40.0, 10.0],
        [0.0, 10.0],
    ];
    // the L: everything but the notch p in (0, 40), q in (10, 20)
    let pts = [pts[0], pts[1], pts[2], pts[3], pts[4], pts[5]];
    let segs = (0..pts.len())
        .map(|i| line(pts[i], pts[(i + 1) % pts.len()]))
        .collect();
    Face::new(
        Surf::Plane {
            o: [80.0, 50.0, 0.0],
            u: [-1.0, 0.0, 0.0],
            v: [0.0, 0.0, 1.0],
        },
        Ival::new(0.0, 80.0),
        Ival::new(0.0, 20.0),
        Some(Trim { segs }),
        1e-9,
        0,
    )
}

/// Whether (p, q) is in the L.
fn in_l(p: f64, q: f64) -> bool {
    (0.0..=80.0).contains(&p) && (0.0..=20.0).contains(&q) && !(p < 40.0 && q > 10.0)
}

/// The least sampled signed distance of the moved body to the true (L-shaped) face.
fn sampled(body: &Body, mv: &Move) -> f64 {
    let mut least = f64::INFINITY;
    for i in 0..=80 {
        let p = 80.0 * i as f64 / 80.0;
        for j in 0..=20 {
            let q = 20.0 * j as f64 / 20.0;
            if !in_l(p, q) {
                continue;
            }
            let y = [80.0 - p, 50.0, q];
            for k in 0..=8 {
                let a = mv.at(Ival::pt(k as f64 / 8.0));
                let a = [a[0].mid(), a[1].mid(), a[2].mid()];
                let rho = ((y[0] - a[0]).powi(2) + (y[1] - a[1]).powi(2)).sqrt();
                least = least.min(body.sd_rz(Ival::pt(rho), Ival::pt(y[2] - a[2])).mid());
            }
        }
    }
    least
}

fn flat() -> Body {
    assemble(&Cutter::Flat { d: 10.0 }, 32.0, 10.0, 40.0, None)
        .unwrap()
        .bodies[0]
        .clone()
}

#[test]
fn the_hull_is_formed_only_for_a_vertical_plane_whose_trim_is_not_its_box() {
    let f = l_wall();
    assert!(f.prism.is_none(), "an L is not a prism");
    // the box's heights, rounded outward
    let Some(Prism::Wall { z0, z1, .. }) = Prism::hull_wall(&f) else {
        panic!("an L wall's hull");
    };
    assert!(
        z0 <= 0.0 && z0 > -1e-9 && z1 >= 20.0 && z1 < 20.0 + 1e-9,
        "{z0} {z1}"
    );
    // a horizontal plane, a sloped plane: none
    let mut h = l_wall();
    h.surf = Surf::Plane {
        o: [0.0, 0.0, 5.0],
        u: [1.0, 0.0, 0.0],
        v: [0.0, 1.0, 0.0],
    };
    assert!(Prism::hull_wall(&h).is_none());
    let mut s = l_wall();
    s.surf = Surf::Plane {
        o: [80.0, 50.0, 0.0],
        u: [-1.0, 0.0, 0.0],
        v: [0.0, 0.6, 0.8],
    };
    assert!(Prism::hull_wall(&s).is_none());
}

/// Random lines and arcs along, past and into the wall, in the notch and beside it, at heights
/// through the wall's: a Clear is below every sample of the true face; where the branch and bound
/// finds a gouge of the true face, no Clear.
#[test]
fn every_clear_is_below_the_samples_and_no_gouge_is_cleared() {
    let mut g = Rng(5);
    let body = flat();
    let f = l_wall();
    let (mut clear, mut other, mut hits) = (0, 0, 0);
    for case in 0..64 {
        let z = g.r(-2.0, 24.0);
        let y = 50.0 + g.r(-6.0, 6.0);
        let mv = if case % 2 == 0 {
            Move::Line {
                a: [g.r(-10.0, 90.0), y, z],
                b: [g.r(-10.0, 90.0), 50.0 + g.r(-6.0, 6.0), z],
            }
        } else {
            let (ax, ay) = loop {
                let (vx, vy) = (g.r(-1.0, 1.0), g.r(-1.0, 1.0));
                let m = (vx * vx + vy * vy).sqrt();
                if m > 0.1 && m <= 1.0 {
                    break (vx / m, vy / m);
                }
            };
            Move::Arc {
                c: [g.r(0.0, 80.0), y, z],
                x: [ax, ay, 0.0],
                y: [-ay, ax, 0.0],
                r: g.r(1.0, 8.0),
                t0: -g.r(0.05, 0.4),
                t1: g.r(0.05, 0.4),
            }
        };
        let least = sampled(&body, &mv);
        for lim in [least - 0.5, least - 0.01, least + 0.01] {
            let (v, _) = certify_shared(&body, &f, &mv, lim, 0.01 / 4.0, 200_000, &mut None);
            match v {
                Verdict::Clear { lower } => {
                    clear += 1;
                    assert!(
                        lower <= least + 1e-6,
                        "case {case}: Clear {lower} over the sampled least {least}"
                    );
                    if lim > least
                        && let (Verdict::Hit { .. }, _) = certify_bb(&body, &f, &mv, lim, 0.01 / 4.0, 200_000)
                    {
                        panic!("case {case}: a gouge of the true face cleared");
                    }
                }
                Verdict::Hit { .. } => hits += 1,
                Verdict::Undecided { .. } => other += 1,
            }
        }
    }
    assert!(
        clear > 60 && hits > 20,
        "clear {clear}, hits {hits}, other {other}"
    );
}

/// The open step's corner: the tool's centre on an arc of its own radius about the wall's end
/// edge (x = 80), its side touching the edge all along. The branch and bound took ~200,000 cells
/// per such arc; the wall over the box clears it in few.
#[test]
fn a_corner_arc_about_the_walls_end_is_cleared_in_few_cells() {
    let body = flat();
    let f = l_wall();
    let h = std::f64::consts::FRAC_1_SQRT_2;
    let mv = Move::Arc {
        c: [80.0, 50.0, 12.5],
        x: [h, h, 0.0],
        y: [h, -h, 0.0],
        r: 5.0,
        t0: -0.4142135623730951,
        t1: 0.4142135623730951,
    };
    let (v, n) = certify_shared(&body, &f, &mv, -0.01, 0.01 / 4.0, 200_000, &mut None);
    assert!(
        matches!(v, Verdict::Clear { .. }) && n < 2000,
        "{v:?} in {n} cells"
    );
}
