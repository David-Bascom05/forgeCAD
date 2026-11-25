//! The engagement the feeds are set from is never below the true one, wherever the stock's edges
//! fall on the model's grid: what the edge meets is read from a model a cell coarse, and a read
//! that errs low raises the feed over the chip (the cutter overloaded where it leaves the stock,
//! or meets a strip a cell narrower than it is). Checked against the exact engagement of a
//! rectangular stock (a first pass) and of that stock less a first pass's slot (a second pass
//! beside it), over sub-cell changes of the stock's extent and the passes' offsets.
use forge_cam::engage::{HeightMap, Profile};

const R: f64 = 5.0;
const EPS: f64 = 1e-3;

/// The true engagement's max cos θ over the front half edge at tip c moving +x (θ from the travel,
/// + left), material where `solid(x, y)`; None if the edge meets nothing. On a fine grid of θ.
fn truth(c: [f64; 2], solid: &dyn Fn(f64, f64) -> bool) -> Option<f64> {
    let n = 18_001;
    let mut best: Option<f64> = None;
    for k in 0..n {
        let th = -std::f64::consts::FRAC_PI_2 + std::f64::consts::PI * k as f64 / (n - 1) as f64;
        let (s, co) = forge_math::dmath::sin_cos(th);
        if solid(c[0] + R * co, c[1] + R * s) {
            best = Some(best.map_or(co, |b: f64| b.max(co)));
        }
    }
    best
}

/// The feed per tooth over fz that engagement allows (engage.rs `sample_feeds`'s chip rule):
/// 1 / max cos θ, at most the thinning cap; air (or an edge that only touches) the cap.
fn factor(cos_max: Option<f64>) -> f64 {
    let cap = forge_tooling::feeds::THIN_CAP;
    match cos_max {
        Some(c) if c * cap > 1.0 => 1.0 / c,
        _ => cap,
    }
}

/// Over every stock extent and tip place tried: the feed the model's estimate allows is never
/// above the one the true engagement allows (the chip it feeds for is never exceeded).
/// One case: the model, the tip, and where the true stock is.
type Case<'a> = &'a mut dyn FnMut(&HeightMap, [f64; 3], &dyn Fn(f64, f64) -> bool);

fn check(label: &str, h: f64, cases: &mut dyn FnMut(Case)) {
    let (mut n, mut engaged) = (0usize, 0usize);
    let mut worst = 0.0f64;
    let mut first_bad: Option<String> = None;
    cases(&mut |map, c, solid| {
        n += 1;
        let t = truth([c[0], c[1]], solid);
        let e = map.engage(c, [1.0, 0.0], R, EPS).map(|e| e.cos_max);
        if t.is_some() {
            engaged += 1;
        }
        let over = factor(e) / factor(t) - 1.0;
        if over > 1e-9 && over > worst {
            worst = over;
            first_bad = Some(format!("tip {c:?}: truth cos {t:?}, model {e:?}"));
        }
    });
    assert!(n > 500 && engaged > 100, "{label}: {n} cases, {engaged} engaged");
    assert!(
        first_bad.is_none(),
        "{label} (cell {h}): the model reads less engagement than there is: a feed up to {:.2} % over the chip; e.g. {}",
        100.0 * worst,
        first_bad.unwrap_or_default()
    );
}

#[test]
fn a_first_pass_leaving_the_stock_is_never_under_read() {
    for h in [10.0 / 30.0, 0.25, 0.2] {
        check("first pass", h, &mut |f| {
            for kl in 0..24 {
                // the stock's far edge at sub-cell offsets from the grid
                let l = 60.0 + kl as f64 * h / 24.0;
                let w = 30.0;
                let map = HeightMap::new([0.0, 0.0, 0.0], [l, w, 10.0], h);
                let solid = move |x: f64, y: f64| (0.0..=l).contains(&x) && (0.0..=w).contains(&y);
                for ky in 0..5 {
                    let y = [w * 0.5, 2.0, w - 2.0, 0.3, w - 0.1][ky];
                    for kx in 0..40 {
                        // the tip from inside the stock to past its far edge
                        let x = l - R - 1.0 + kx as f64 * (2.0 * R + 2.0) / 40.0;
                        f(&map, [x, y, 9.0], &solid);
                    }
                }
            }
        });
    }
}

#[test]
fn a_pass_beside_an_earlier_slot_is_never_under_read() {
    let prof = Profile::Bull { r: R, re: 0.0 };
    for h in [10.0 / 30.0, 0.25] {
        check("second pass", h, &mut |f| {
            for ko in 0..12 {
                // the first pass along y = y1 cut its slot through the stock; the second, a
                // stepover over, cuts beside it
                let y1 = 10.0 + ko as f64 * h / 12.0;
                let (l, w) = (40.0, 30.0);
                let mut map = HeightMap::new([0.0, 0.0, 0.0], [l, w, 10.0], h);
                map.sweep([-10.0, y1, 9.0], [l + 10.0, y1, 9.0], &prof);
                let solid = move |x: f64, y: f64| {
                    (0.0..=l).contains(&x) && (0.0..=w).contains(&y) && (y - y1).abs() > R
                };
                for &so in &[3.0, 5.0, 7.0, 8.5] {
                    for kx in 0..30 {
                        let x = 5.0 + kx as f64;
                        f(&map, [x, y1 + so, 9.0], &solid);
                    }
                }
            }
        });
    }
}

#[test]
fn a_pass_beside_a_slanted_slot_is_never_under_read() {
    let prof = Profile::Bull { r: R, re: 0.0 };
    // the first pass at a slant across the grid (the cut's edge at every angle to the cells); the
    // second, parallel to it a stepover over, read along its own direction
    for h in [10.0 / 30.0, 0.25] {
        for deg in [17.0f64, 30.0, 45.0, 62.0] {
            let (s, co) = forge_math::dmath::sin_cos(deg.to_radians());
            let (u, nrm) = ([co, s], [-s, co]);
            let (mut n, mut worst, mut bad) = (0usize, 0.0f64, None);
            for ko in 0..8 {
                let o = ko as f64 * h / 8.0;
                let (l, w) = (60.0, 60.0);
                let p0 = [10.0 + o, 10.0 + 0.37 * o];
                let mut map = HeightMap::new([0.0, 0.0, 0.0], [l, w, 10.0], h);
                let p1 = [p0[0] + 60.0 * u[0], p0[1] + 60.0 * u[1]];
                map.sweep([p0[0], p0[1], 9.0], [p1[0], p1[1], 9.0], &prof);
                let solid = move |x: f64, y: f64| {
                    if !((0.0..=l).contains(&x) && (0.0..=w).contains(&y)) {
                        return false;
                    }
                    // distance to the first pass's segment
                    let (dx, dy) = (x - p0[0], y - p0[1]);
                    let t = (dx * u[0] + dy * u[1]).clamp(0.0, 60.0);
                    let (qx, qy) = (p0[0] + t * u[0], p0[1] + t * u[1]);
                    ((x - qx).powi(2) + (y - qy).powi(2)).sqrt() > R
                };
                for &so in &[3.0, 5.0, 7.0, 8.5] {
                    for k in 0..20 {
                        let along = 8.0 + k as f64 * 1.3;
                        let c = [
                            p0[0] + along * u[0] + so * nrm[0],
                            p0[1] + along * u[1] + so * nrm[1],
                            9.0,
                        ];
                        n += 1;
                        // the truth along this direction
                        let mut tbest: Option<f64> = None;
                        let m = 18_001;
                        for kk in 0..m {
                            let th = -std::f64::consts::FRAC_PI_2
                                + std::f64::consts::PI * kk as f64 / (m - 1) as f64;
                            let (ts, tc) = forge_math::dmath::sin_cos(th);
                            let dx = u[0] * tc - u[1] * ts;
                            let dy = u[0] * ts + u[1] * tc;
                            if solid(c[0] + R * dx, c[1] + R * dy) {
                                tbest = Some(tbest.map_or(tc, |b: f64| b.max(tc)));
                            }
                        }
                        let e = map.engage(c, u, R, EPS).map(|e| e.cos_max);
                        let over = factor(e) / factor(tbest) - 1.0;
                        if over > 1e-9 && over > worst {
                            worst = over;
                            bad = Some(format!("{deg}°, tip {c:?}: truth {tbest:?}, model {e:?}"));
                        }
                    }
                }
            }
            assert!(n > 100);
            assert!(
                bad.is_none(),
                "cell {h}: a feed up to {:.2} % over the chip; {}",
                100.0 * worst,
                bad.unwrap_or_default()
            );
        }
    }
}
