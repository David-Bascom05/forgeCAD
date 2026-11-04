//! A stretch law fitted to a material's own test: stress against stretch from a uniaxial or an
//! equibiaxial (bubble) test at forming temperature, an incompressible Ogden law of one to three
//! terms (neo-Hookean and Mooney-Rivlin as its cases) found by Nelder-Mead from fixed starts, and
//! checked for stability (Drucker: stress rising with stretch, both ways of loading) over the
//! stretches a formed part reaches.
use crate::stretch::Law;
use forge_math::dmath;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Test {
    Uniaxial,
    Equibiaxial,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Model {
    NeoHookean,
    MooneyRivlin,
    Ogden(usize),
}

/// The fit: the law, its RMS error over the points (a fraction of each point's stress), and the
/// stretch up to which it is stable (both ways of loading), up to `reach`.
#[derive(Clone, Debug, PartialEq)]
pub struct Fitted {
    pub law: Law,
    pub rms: f64,
    pub stable_to: f64,
}

fn pw(x: f64, a: f64) -> f64 {
    dmath::powf(x, a)
}

/// The true stress (MPa) a law gives at stretch `l` in a test.
pub fn stress(law: &Law, test: Test, l: f64) -> f64 {
    let lateral = match test {
        Test::Uniaxial => -0.5,
        Test::Equibiaxial => -2.0,
    };
    law.terms
        .iter()
        .map(|&(mu, a)| mu * (pw(l, a) - pw(l, lateral * a)))
        .sum()
}

/// Reads a CSV of stretch and stress (MPa): lines of two numbers; others (a header) skipped.
pub fn read_csv(text: &str) -> Vec<(f64, f64)> {
    text.lines()
        .filter_map(|l| {
            let mut it = l.split([',', ';', '\t']).map(str::trim);
            let a = it.next()?.parse::<f64>().ok()?;
            let b = it.next()?.parse::<f64>().ok()?;
            (a.is_finite() && b.is_finite() && a > 1.0 && b >= 0.0).then_some((a, b))
        })
        .collect()
}

fn law_of(model: Model, p: &[f64]) -> Law {
    Law {
        terms: match model {
            Model::NeoHookean => vec![(p[0], 2.0)],
            Model::MooneyRivlin => vec![(p[0], 2.0), (-p[1], -2.0)],
            Model::Ogden(_) => p.chunks(2).map(|c| (c[0], c[1])).collect(),
        },
    }
}

fn error(law: &Law, test: Test, pts: &[(f64, f64)]) -> f64 {
    let e: f64 = pts
        .iter()
        .map(|&(l, s)| {
            let r = (stress(law, test, l) - s) / s.abs().max(1e-3);
            r * r
        })
        .sum();
    (e / pts.len().max(1) as f64).sqrt()
}

/// Nelder-Mead from `x0` with steps `step`, on f.
fn simplex(f: &dyn Fn(&[f64]) -> f64, x0: &[f64], step: &[f64], iters: usize) -> (Vec<f64>, f64) {
    let n = x0.len();
    let mut pts: Vec<Vec<f64>> = vec![x0.to_vec()];
    for k in 0..n {
        let mut p = x0.to_vec();
        p[k] += step[k];
        pts.push(p);
    }
    let mut vals: Vec<f64> = pts.iter().map(|p| f(p)).collect();
    for _ in 0..iters {
        let mut order: Vec<usize> = (0..=n).collect();
        order.sort_by(|&a, &b| vals[a].total_cmp(&vals[b]).then(a.cmp(&b)));
        let (best, worst, second) = (order[0], order[n], order[n - 1]);
        if (vals[worst] - vals[best]).abs() < 1e-12 {
            break;
        }
        let centre: Vec<f64> = (0..n)
            .map(|k| order[..n].iter().map(|&i| pts[i][k]).sum::<f64>() / n as f64)
            .collect();
        let at = |t: f64| -> Vec<f64> {
            (0..n)
                .map(|k| centre[k] + t * (pts[worst][k] - centre[k]))
                .collect()
        };
        let r = at(-1.0);
        let fr = f(&r);
        if fr < vals[best] {
            let e = at(-2.0);
            let fe = f(&e);
            (pts[worst], vals[worst]) = if fe < fr { (e, fe) } else { (r, fr) };
        } else if fr < vals[second] {
            (pts[worst], vals[worst]) = (r, fr);
        } else {
            let c = at(0.5);
            let fc = f(&c);
            if fc < vals[worst] {
                (pts[worst], vals[worst]) = (c, fc);
            } else {
                for &i in &order[1..] {
                    pts[i] = (0..n)
                        .map(|k| pts[best][k] + 0.5 * (pts[i][k] - pts[best][k]))
                        .collect();
                    vals[i] = f(&pts[i]);
                }
            }
        }
    }
    let b = (0..=n)
        .min_by(|&a, &b| vals[a].total_cmp(&vals[b]))
        .expect("a point");
    (pts[b].clone(), vals[b])
}

/// The stretch up to which the law's stress rises with stretch in both tests, up to `reach`.
pub fn stable_to(law: &Law, reach: f64) -> f64 {
    let n = 200;
    let mut prev = [0.0; 2];
    for k in 1..=n {
        let l = 1.0 + (reach - 1.0) * k as f64 / n as f64;
        for (j, t) in [Test::Uniaxial, Test::Equibiaxial].into_iter().enumerate() {
            let s = stress(law, t, l);
            if k > 1 && s <= prev[j] {
                return 1.0 + (reach - 1.0) * (k - 1) as f64 / n as f64;
            }
            prev[j] = s;
        }
    }
    reach
}

/// The best law of `model` for the points of `test`, checked to stretch `reach`.
pub fn fit(pts: &[(f64, f64)], test: Test, model: Model, reach: f64) -> Option<Fitted> {
    if pts.len() < 2 {
        return None;
    }
    // a small-strain modulus from the first points: 3 G (lambda - 1) uniaxially, 6 G biaxially
    let (l, s) = pts[0];
    let g = (s / ((l - 1.0).max(1e-3) * if test == Test::Uniaxial { 3.0 } else { 6.0 })).max(1e-3);
    let starts: Vec<(Vec<f64>, Vec<f64>)> = match model {
        Model::NeoHookean => vec![(vec![g], vec![0.5 * g])],
        Model::MooneyRivlin => vec![(vec![0.8 * g, 0.1 * g], vec![0.4 * g, 0.1 * g])],
        Model::Ogden(n) => {
            let n = n.clamp(1, 3);
            [[2.0, 5.0, -2.0], [1.5, 3.0, -1.0], [2.5, 6.0, -3.0]]
                .iter()
                .map(|alphas| {
                    let x: Vec<f64> = (0..n)
                        .flat_map(|k| [2.0 * g / alphas[k] / n as f64, alphas[k]])
                        .collect();
                    let s: Vec<f64> = (0..n)
                        .flat_map(|k| [g / n as f64, 0.5 * alphas[k].abs()])
                        .collect();
                    (x, s)
                })
                .collect()
        }
    };
    let f = |p: &[f64]| {
        let law = law_of(model, p);
        let e = error(&law, test, pts);
        if e.is_finite() { e } else { 1e30 }
    };
    let (best, _) = starts
        .iter()
        .map(|(x, s)| {
            let (p, _) = simplex(&f, x, s, 4000);
            // restart once from where it stopped: the simplex shrinks on long valleys
            let step: Vec<f64> = p.iter().map(|v| 0.1 * v.abs().max(1e-3)).collect();
            simplex(&f, &p, &step, 4000)
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))?;
    let law = law_of(model, &best);
    Some(Fitted {
        rms: error(&law, test, pts),
        stable_to: stable_to(&law, reach),
        law,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_law_is_found_again_from_its_own_points() {
        let want = Law {
            terms: vec![(0.3, 1.8), (0.02, 5.0)],
        };
        let pts: Vec<(f64, f64)> = (0..30)
            .map(|k| {
                let l = 1.1 + 0.13 * k as f64;
                (l, stress(&want, Test::Equibiaxial, l))
            })
            .collect();
        let got = fit(&pts, Test::Equibiaxial, Model::Ogden(2), 6.0).unwrap();
        assert!(got.rms < 0.01, "{got:?}");
        assert!(got.stable_to > 5.9);
        // a neo-Hookean fit to the same points is worse, not wrong
        let nh = fit(&pts, Test::Equibiaxial, Model::NeoHookean, 6.0).unwrap();
        assert!(nh.rms > got.rms);
        assert_eq!(
            read_csv("stretch,stress\n1.5, 0.4\nx\n2;0.7"),
            vec![(1.5, 0.4), (2.0, 0.7)]
        );
    }
}
