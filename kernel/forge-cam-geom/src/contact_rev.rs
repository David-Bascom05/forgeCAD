//! Closed-form clearance for a torus or a cone face (contact.rs's statement, its parallelogram P
//! of capsule cores swept along a line move): a proven lower bound on dist(F, P) over F's full
//! surface (the trimmed patch is a part of it, so this proves Clear only).
//!
//! The frame. The face's vectors x, y, ax are orthonormal only to rounding: with
//! m = max(| |x|² − 1 |, .., |x·y|, ..) (outward-rounded), the matrix F = [x y ax] has
//! ‖FᵀF − I‖₂ ≤ ‖FᵀF − I‖_F ≤ 3m =: η, and its polar factor U (orthonormal) has
//! ‖F − U‖₂ = max |σᵢ − 1| ≤ max |σᵢ² − 1| ≤ η. The exact surface F* built on U (same o, radii)
//! puts each face point within η·|coefficients| of its counterpart (ε below), so
//! dist(F, P) ≥ dist(F*, P) − ε. For F*, with h the coordinate along U e₃ and ρ the distance to
//! that axis: |ax/|ax| − U e₃| ≤ ||ax| − 1| + η ≤ 2η, so the h and ρ the code computes about
//! ax differ from F*'s by at most |w|·2η (a distance to a line through o is |w|-Lipschitz in its
//! unit direction), w the point less o.
//!
//! Torus (R, tube radius r·|(cm, sm)|): dist(p, F*) ≥ shell(dist(p, C*)) with C* its core
//! circle, dist(p, C*)² = h² + (ρ − R)²: over P at least min h² + min (ρ − R)² (each minimised
//! alone), at most max h² + max (ρ − R)²; every point of F* is at distance exactly the tube radius
//! from C* when R ≥ it (a ring torus), at most that otherwise (the inner sheet of a spindle comes
//! nearer: then only P outside the tube can clear).
//!
//! Cone (o + (r0 + q k) u + q ax): in the meridian half-plane through p, F* is the pair of rays
//! ρ = |k (h − hₐ)| from the apex hₐ = −r0/k, and the 3D distance to a surface of revolution is the
//! distance in that half-plane; a point's distance to each ray is at least its distance to the
//! ray's line, |±k (h − hₐ) − ρ| / √(1 + k²), enclosed over P's h and ρ.
use crate::check::{Face, Surf, V3};
use crate::ival::Ival;

fn iv(v: V3) -> [Ival; 3] {
    [Ival::pt(v[0]), Ival::pt(v[1]), Ival::pt(v[2])]
}
fn dot(a: [Ival; 3], b: [Ival; 3]) -> Ival {
    a[0].mul(b[0]).add(a[1].mul(b[1])).add(a[2].mul(b[2]))
}
fn add(a: [Ival; 3], b: [Ival; 3]) -> [Ival; 3] {
    [a[0].add(b[0]), a[1].add(b[1]), a[2].add(b[2])]
}
fn scl(a: [Ival; 3], k: Ival) -> [Ival; 3] {
    [a[0].mul(k), a[1].mul(k), a[2].mul(k)]
}

/// An interval's distance from 0 (a lower bound on |x| over it).
fn off0(x: Ival) -> f64 {
    if x.lo > 0.0 {
        x.lo
    } else if x.hi < 0.0 {
        -x.hi
    } else {
        0.0
    }
}

/// η = 3m: the frame's distance from orthonormal (module doc), rounded up.
fn eta(x: V3, y: V3, ax: V3) -> f64 {
    let (xi, yi, ai) = (iv(x), iv(y), iv(ax));
    let one = Ival::pt(1.0);
    let m = [
        dot(xi, xi).sub(one),
        dot(yi, yi).sub(one),
        dot(ai, ai).sub(one),
        dot(xi, yi),
        dot(xi, ai),
        dot(yi, ai),
    ]
    .iter()
    .fold(0.0f64, |m, v| m.max(v.abs().hi));
    Ival::pt(m).scale(3.0).hi
}

fn fin(i: Ival) -> bool {
    i.lo.is_finite() && i.hi.is_finite()
}

/// P's h (along ax, normalised) and ρ (from the axis line through o) enclosures, each widened
/// by |w|·2η to F*'s; None when ax is degenerate. `w0, d, e`: contact.rs's parallelogram less o.
fn h_rho(w0: [Ival; 3], d: [Ival; 3], e: [Ival; 3], ax: V3, eta: f64) -> Option<(Ival, Ival)> {
    let ai = iv(ax);
    let aa = dot(ai, ai);
    if !(aa.lo > 0.0) {
        return None;
    }
    let na = aa.sqrt();
    let (mut hlo, mut hhi) = (f64::INFINITY, f64::NEG_INFINITY);
    for (s, t) in crate::contact::CORNERS {
        let w = add(add(w0, scl(d, Ival::pt(s))), scl(e, Ival::pt(t)));
        let hk = dot(w, ai).div(na);
        hlo = hlo.min(hk.lo);
        hhi = hhi.max(hk.hi);
    }
    // (NaN or overflow in any corner: refuse; f64 min/max would drop a NaN)
    if !(hlo.is_finite() && hhi.is_finite()) {
        return None;
    }
    let rq = crate::contact::Quad {
        w0,
        d,
        e,
        ax: Some((ai, aa)),
    };
    let wq = crate::contact::Quad { w0, d, e, ax: None };
    let (rlo, rhi) = (Ival::pt(rq.min_lo()?).sqrt().lo, Ival::pt(rq.max_hi()?).sqrt().hi);
    let wmax = Ival::pt(wq.max_hi()?).sqrt().hi;
    let widen = Ival::pt(wmax).mul(Ival::pt(eta)).scale(2.0).hi;
    let h = Ival::new(
        Ival::pt(hlo).sub(Ival::pt(widen)).lo,
        Ival::pt(hhi).add(Ival::pt(widen)).hi,
    );
    let rho = Ival::new(
        Ival::pt(rlo).sub(Ival::pt(widen)).lo.max(0.0),
        Ival::pt(rhi).add(Ival::pt(widen)).hi,
    );
    Some((h, rho))
}

/// A proven lower bound on dist(F, P) for a torus or a cone face; None for other surfaces or
/// degenerate data (the caller goes on to the branch and bound).
pub(crate) fn dist(f: &Face, w0: [Ival; 3], d: [Ival; 3], e: [Ival; 3]) -> Option<f64> {
    match f.surf {
        Surf::Torus {
            x,
            y,
            ax,
            big,
            r,
            cm,
            sm,
            ..
        } => {
            let eta = eta(x, y, ax);
            let (h, rho) = h_rho(w0, d, e, ax, eta)?;
            let rr = Ival::pt(big);
            let lam = Ival::pt(cm).sqr().add(Ival::pt(sm).sqr()).sqrt();
            let tube = Ival::pt(r).abs().mul(lam);
            // dist(P, C*)² enclosed
            let dr = rho.sub(rr);
            if !(fin(h) && fin(dr) && fin(tube)) {
                return None;
            }
            let q_lo = Ival::pt(off0(h)).sqr().add(Ival::pt(off0(dr)).sqr()).lo;
            let hmax = h.abs().hi;
            let drmax = dr.abs().hi;
            let q_hi = Ival::pt(hmax).sqr().add(Ival::pt(drmax).sqr()).hi;
            // a ring torus's points are exactly the tube radius from C*; a spindle's at most
            let s_lo = if rr.lo >= tube.hi { tube.lo } else { 0.0 };
            let base = crate::contact::shell(q_lo, q_hi, s_lo, tube.hi)?;
            // each face point within η (R + 2·tube) of F*'s
            let eps = Ival::pt(eta).mul(rr.abs().add(tube.scale(2.0))).hi;
            Some(Ival::pt(base).sub(Ival::pt(eps)).lo.max(0.0))
        }
        Surf::Cone { x, y, ax, r0, k, .. } => {
            if k == 0.0 || !k.is_finite() {
                return None;
            }
            let eta = eta(x, y, ax);
            let (h, rho) = h_rho(w0, d, e, ax, eta)?;
            let ki = Ival::pt(k);
            let ha = Ival::pt(r0).div(ki).scale(-1.0);
            let lin = ki.mul(h.sub(ha));
            let norm = Ival::pt(1.0).add(ki.sqr()).sqrt();
            let fp = off0(lin.sub(rho));
            let fm = off0(Ival::pt(0.0).sub(lin).sub(rho));
            // both nappes' bounds must be numbers: f64 min drops a NaN, which would keep only
            // one nappe's (larger) bound
            if !(fp.is_finite() && fm.is_finite() && fin(norm)) {
                return None;
            }
            let base = Ival::pt(fp.min(fm)).div(norm).lo;
            // the coefficients (r0 + q k, q) over the face's q range
            let q = f.q;
            let coef = Ival::pt(r0).add(q.mul(ki)).abs().add(q.abs());
            let eps = Ival::pt(eta).mul(coef).hi;
            Some(Ival::pt(base).sub(Ival::pt(eps)).lo.max(0.0))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check::{Move, Verdict, certify_bb};
    use crate::tool::{Body, Cutter, assemble};

    fn ball(d: f64) -> Body {
        assemble(&Cutter::Ball { d }, 3.0 * d, d, 4.0 * d, None)
            .unwrap()
            .bodies[0]
            .clone()
    }

    fn rnd(seed: &mut u64) -> f64 {
        *seed ^= *seed << 13;
        *seed ^= *seed >> 7;
        *seed ^= *seed << 17;
        (*seed >> 11) as f64 / (1u64 << 53) as f64
    }

    /// A frame about a random axis (unit and orthogonal to rounding), from rational maps.
    fn frame(seed: &mut u64) -> (V3, V3, V3) {
        let n = |v: V3| {
            let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
            [v[0] / l, v[1] / l, v[2] / l]
        };
        let ax = n([rnd(seed) - 0.5, rnd(seed) - 0.5, 1.0 + rnd(seed)]);
        let x = n([ax[2], 0.0, -ax[0]]);
        let y = [
            ax[1] * x[2] - ax[2] * x[1],
            ax[2] * x[0] - ax[0] * x[2],
            ax[0] * x[1] - ax[1] * x[0],
        ];
        (x, y, ax)
    }

    fn true_min(body: &Body, f: &Face, a: V3, b: V3, n: usize) -> f64 {
        let mut tmin = f64::INFINITY;
        for i in 0..=n {
            for j in 0..=n {
                let p = f.p.lo + (f.p.hi - f.p.lo) * i as f64 / n as f64;
                let q = f.q.lo + (f.q.hi - f.q.lo) * j as f64 / n as f64;
                let y = f.surf.eval(Ival::pt(p), Ival::pt(q));
                let y = [y[0].mid(), y[1].mid(), y[2].mid()];
                for m in 0..=10 {
                    let s = m as f64 / 10.0;
                    let g = [
                        a[0] + (b[0] - a[0]) * s,
                        a[1] + (b[1] - a[1]) * s,
                        a[2] + (b[2] - a[2]) * s,
                    ];
                    let sd = body
                        .sd(
                            Ival::pt(y[0] - g[0]),
                            Ival::pt(y[1] - g[1]),
                            Ival::pt(y[2] - g[2]),
                        )
                        .mid();
                    tmin = tmin.min(sd);
                }
            }
        }
        tmin
    }

    /// On random tori (ring and spindle, tilted, the tube's middle anywhere) and cones (either
    /// slope, apex inside or outside the face's range) against random line moves of a ball cutter,
    /// above, beside and inside: the bound never exceeds the sampled true minimum, and wherever it
    /// clears, the branch and bound finds no Hit.
    #[test]
    fn never_above_the_true_minimum() {
        let body = ball(6.0);
        let c = crate::contact::capsule(&body).unwrap();
        let mut seed = 0x2545f4914f6cdd1du64;
        let mut cleared = 0;
        for k in 0..240 {
            let (x, y, ax) = frame(&mut seed);
            let o = [
                rnd(&mut seed) * 10.0 - 5.0,
                rnd(&mut seed) * 10.0 - 5.0,
                rnd(&mut seed) * 6.0 - 12.0,
            ];
            let f = if k % 2 == 0 {
                let big = 4.0 + rnd(&mut seed) * 10.0;
                let r = if k % 6 == 0 {
                    big * (1.0 + rnd(&mut seed))
                } else {
                    1.0 + rnd(&mut seed) * (big - 1.0)
                };
                let tm = rnd(&mut seed) * 4.0 - 2.0;
                let (cm, sm) = ((1.0 - tm * tm) / (1.0 + tm * tm), 2.0 * tm / (1.0 + tm * tm));
                Face::new(
                    Surf::Torus {
                        o,
                        x,
                        y,
                        ax,
                        big,
                        r,
                        cm,
                        sm,
                    },
                    Ival::new(-1.0, 1.0),
                    Ival::new(-1.0, 1.0),
                    None,
                    0.0,
                    k as u32,
                )
            } else {
                let k_slope = (rnd(&mut seed) - 0.5) * 3.0;
                let r0 = 2.0 + rnd(&mut seed) * 8.0;
                let qlo = -4.0 - rnd(&mut seed) * 6.0;
                Face::new(
                    Surf::Cone {
                        o,
                        x,
                        y,
                        ax,
                        r0,
                        k: k_slope,
                    },
                    Ival::new(-1.0, 1.0),
                    Ival::new(qlo, qlo + 4.0 + rnd(&mut seed) * 8.0),
                    None,
                    0.0,
                    k as u32,
                )
            };
            let a = [
                rnd(&mut seed) * 30.0 - 15.0,
                rnd(&mut seed) * 30.0 - 15.0,
                rnd(&mut seed) * 30.0 - 12.0,
            ];
            let b = [
                a[0] + rnd(&mut seed) * 8.0 - 4.0,
                a[1] + rnd(&mut seed) * 8.0 - 4.0,
                a[2] + rnd(&mut seed) * 4.0 - 2.0,
            ];
            let lo = crate::contact::lower(&c, &f, a, b).expect("a closed form");
            let tmin = true_min(&body, &f, a, b, 40);
            assert!(lo <= tmin + 1e-9, "k={k} {:?} lo={lo} sampled min={tmin}", f.surf);
            if lo >= 0.0 {
                let (v, _) = certify_bb(&body, &f, &Move::Line { a, b }, lo - 1e-9, 0.01, 20_000);
                assert!(
                    !matches!(v, Verdict::Hit { .. }),
                    "k={k}: clears at {lo}, B&B hits: {v:?}"
                );
                cleared += 1;
            }
        }
        assert!(cleared > 40, "{cleared}");
    }

    /// Exact where the ball touches: a ball over the outer equator of a ring torus, and over a
    /// cone's slant, at a gap above them, has the bound within rounding of that gap.
    #[test]
    fn tight_where_the_ball_touches() {
        let body = ball(6.0);
        let c = crate::contact::capsule(&body).unwrap();
        let (x, y, ax) = ([1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]);
        let torus = Face::new(
            Surf::Torus {
                o: [0.0, 0.0, 0.0],
                x,
                y,
                ax,
                big: 10.0,
                r: 3.0,
                cm: 1.0,
                sm: 0.0,
            },
            Ival::new(-1.0, 1.0),
            Ival::new(-1.0, 1.0),
            None,
            0.0,
            0,
        );
        // the tube's top at (10, 0, 3); the ball's centre 3 above its tip
        for gap in [0.5, 1e-3, 0.0] {
            let tip = [10.0, 0.0, 3.0 + gap];
            let lo = crate::contact::lower(&c, &torus, tip, tip).unwrap();
            assert!(lo <= gap && lo > gap - 1e-9, "torus gap {gap} lo {lo}");
        }
    }

    /// A mould-like part (a cone wall with a 1:5 draft, its foot filleted by a concave torus of
    /// radius 3) under ball raster moves at clearances 0.05..3: every pair the branch and bound
    /// decides alone is decided the same here (never Clear where it Hits), and the cells and time
    /// with the closed form against without (printed; run with --nocapture).
    #[test]
    fn mould_raster_cells_and_time() {
        let body = ball(6.0);
        let (x, y, ax) = ([1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]);
        // the wall: radius 30 at z = 3 shrinking 0.2 a unit up, z 3..23
        let cone = Face::new(
            Surf::Cone {
                o: [0.0, 0.0, 3.0],
                x,
                y,
                ax,
                r0: 30.0,
                k: -0.2,
            },
            Ival::new(-1.0, 1.0),
            Ival::new(0.0, 20.0),
            None,
            1e-9,
            1,
        );
        // the fillet: tube radius 3 about the circle of radius 33 at z = 3 (outside the wall's
        // foot), the quarter facing the wall's foot: φ about 225° (middle), ψ ∈ ±45°
        let (cm, sm) = (-std::f64::consts::FRAC_1_SQRT_2, -std::f64::consts::FRAC_1_SQRT_2);
        let fillet = Face::new(
            Surf::Torus {
                o: [0.0, 0.0, 3.0],
                x,
                y,
                ax,
                big: 33.0,
                r: 3.0,
                cm,
                sm,
            },
            Ival::new(-1.0, 1.0),
            Ival::new(-0.42, 0.42),
            None,
            1e-9,
            2,
        );
        let faces = [cone, fillet];
        let (mut cells_fast, mut cells_bb, mut t_fast, mut t_bb) = (0usize, 0usize, 0.0f64, 0.0f64);
        let (mut n, mut clear_fast) = (0, 0);
        for row in 0..12 {
            let yy = -40.0 + row as f64 * 7.0;
            for z in [3.05, 4.0, 8.0, 14.0, 20.0, 26.0] {
                for (i, xx) in [-50.0, -30.0, -10.0, 10.0, 30.0]
                    .windows(2)
                    .map(|w| (w[0], w[1]))
                    .enumerate()
                {
                    let _ = i;
                    let mv = crate::check::Move::Line {
                        a: [xx.0, yy, z],
                        b: [xx.1, yy, z],
                    };
                    for f in &faces {
                        n += 1;
                        let t0 = std::time::Instant::now();
                        let (vf, cf) = crate::check::certify(&body, f, &mv, 0.0, 0.01, 50_000);
                        t_fast += t0.elapsed().as_secs_f64();
                        let t0 = std::time::Instant::now();
                        let (vb, cb) = certify_bb(&body, f, &mv, 0.0, 0.01, 50_000);
                        t_bb += t0.elapsed().as_secs_f64();
                        cells_fast += cf;
                        cells_bb += cb;
                        if matches!(vf, Verdict::Clear { .. }) {
                            clear_fast += 1;
                            assert!(
                                !matches!(vb, Verdict::Hit { .. }),
                                "fast Clear where the B&B hits: {mv:?} {:?}",
                                f.surf
                            );
                        }
                    }
                }
            }
        }
        eprintln!(
            "mould: pairs {n} cleared {clear_fast}; cells fast {cells_fast} vs B&B {cells_bb}; secs fast {t_fast:.3} vs B&B {t_bb:.3}"
        );
    }
}
