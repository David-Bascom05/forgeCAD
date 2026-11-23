//! The core against brute force: classes as connected components, chains as walks, owners.
use forge_math::rng::Rng;
use forge_sheet_core::{ChainError, chain, classes, owner};

fn components(n: usize, pairs: &[(u32, u32)]) -> Vec<usize> {
    // the least member of each point's component, by repeated relaxation
    let mut least: Vec<usize> = (0..n).collect();
    loop {
        let mut changed = false;
        for &(a, b) in pairs {
            let m = least[a as usize].min(least[b as usize]);
            for x in [a as usize, b as usize] {
                if least[x] != m {
                    least[x] = m;
                    changed = true;
                }
            }
        }
        if !changed {
            return least;
        }
    }
}

#[test]
fn classes_are_components_numbered_by_least_member() {
    let mut rng = Rng::new(7);
    for _ in 0..500 {
        let n = 1 + rng.below(40);
        let pairs: Vec<(u32, u32)> = (0..rng.below(2 * n))
            .map(|_| (rng.below(n) as u32, rng.below(n) as u32))
            .collect();
        let c = classes(n, &pairs);
        let least = components(n, &pairs);
        let mut roots: Vec<usize> = least.clone();
        roots.sort();
        roots.dedup();
        for i in 0..n {
            assert_eq!(c[i] as usize, roots.iter().position(|&r| r == least[i]).unwrap());
        }
    }
}

#[test]
fn chains() {
    // two squares
    let from = [0, 1, 2, 3, 4, 5, 6, 7];
    let to = [1, 2, 3, 0, 5, 6, 7, 4];
    let l = chain(8, &from, &to).unwrap();
    assert_eq!(l.starts, vec![0, 4, 8]);
    assert_eq!(l.pieces, vec![0, 1, 2, 3, 4, 5, 6, 7]);
    // a vertex left twice and reached once
    assert!(matches!(
        chain(3, &[0, 0, 1], &[1, 2, 0]),
        Err(ChainError::Unchained { vertex: 0, .. })
    ));
    // a figure eight: vertex 0 twice in and out
    match chain(3, &[0, 1, 0, 2], &[1, 0, 2, 0]) {
        Err(ChainError::Pinched { pinches }) => assert_eq!(pinches, vec![(0, 1, 3)]),
        r => panic!("{r:?}"),
    }
}

#[test]
fn owners() {
    // 0 holds 1 holds the hole: the owner is 1
    let holds = [true, true];
    let nest = [false, false, true, false]; // nest[o*2+p]: p holds o's vertex
    assert_eq!(owner(2, &holds, &nest), Some(1));
    assert_eq!(owner(2, &[false, false], &nest), None);
}

#[test]
fn place_then_inverse() {
    use forge_sheet_core::place::{Place, apply, inverse, then};
    let (s, c) = forge_math::dmath::sin_cos(0.7);
    let a = Place {
        ox: 3.0,
        oy: -2.0,
        xx: c,
        xy: s,
    };
    let b = Place {
        ox: -1.5,
        oy: 4.25,
        xx: 0.6,
        xy: 0.8,
    };
    let ab = then(a, b);
    // (a ∘ b)(p) = a(b(p))
    let p = (1.25, -0.5);
    let (bx, by) = apply(b, p.0, p.1);
    let (x1, y1) = apply(a, bx, by);
    let (x2, y2) = apply(ab, p.0, p.1);
    assert!((x1 - x2).abs() < 1e-12 && (y1 - y2).abs() < 1e-12);
    // a⁻¹ ∘ a = id
    let id = then(inverse(a), a);
    assert!(id.ox.abs() < 1e-12 && id.oy.abs() < 1e-12 && (id.xx - 1.0).abs() < 1e-12 && id.xy.abs() < 1e-12);
}

#[test]
fn joint_frame() {
    use forge_sheet_core::place::joint;
    let (j, la) = joint(1.0, 2.0, 4.0, 6.0);
    assert_eq!(la, 5.0);
    // right((3, 4) / 5) = (0.8, -0.6)
    assert!((j.xx - 0.8).abs() < 1e-15 && (j.xy + 0.6).abs() < 1e-15);
    assert_eq!((j.ox, j.oy), (1.0, 2.0));
}

#[test]
fn setback_is_shops() {
    use forge_math::dmath;
    use forge_sheet_core::fold::setback;
    let mut x: u64 = 0x5eed;
    for _ in 0..10000 {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        let theta = (x >> 11) as f64 / (1u64 << 53) as f64 * 3.1;
        let rho = 0.1 + (x % 1000) as f64 / 37.0;
        let (s, c) = dmath::sin_cos(theta);
        let shop = rho * s / (1.0 + c);
        assert_eq!(setback(rho, s, c).to_bits(), shop.to_bits());
        let (sh, ch) = dmath::sin_cos(theta / 2.0);
        assert!((setback(rho, s, c) - rho * sh / ch).abs() <= 1e-12 * (1.0 + rho / (1.0 + c)));
    }
}

#[test]
fn round_is_builds() {
    use forge_math::dmath;
    use forge_sheet_core::fold::{phi, round_local};
    let mut x: u64 = 0x2a2a;
    let mut next = || {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        (x >> 11) as f64 / (1u64 << 53) as f64
    };
    for _ in 0..10000 {
        let (theta, rho0, t) = (0.1 + 2.9 * next(), 0.5 + 9.0 * next(), 0.5 + 3.0 * next());
        let sigma = if next() < 0.5 { 1.0 } else { -1.0 };
        let rn = rho0 + 0.4 * t;
        let ba = rn * theta;
        let (u, y, w) = (
            if next() < 0.1 { ba } else { ba * next() },
            50.0 * next(),
            t * next(),
        );
        // build.rs, as written there
        let a = if u == ba { theta } else { u / rn };
        let (s, c) = dmath::sin_cos(a);
        let r = rho0 + sigma * w;
        let want = (r * c, r * s, -sigma * y);
        assert_eq!(phi(u, rn, theta, ba).to_bits(), a.to_bits());
        let (s2, c2) = dmath::sin_cos(phi(u, rn, theta, ba));
        let got = round_local(rho0, sigma, s2, c2, y, w);
        assert_eq!(
            (got.0.to_bits(), got.1.to_bits(), got.2.to_bits()),
            (want.0.to_bits(), want.1.to_bits(), want.2.to_bits())
        );
    }
}

/// fix-sheet's kernel/forge-sheet/src/exact.rs (51967c1b), verbatim but for its tests, as the
/// reference the Aeneas-style port must match bit for bit.
#[allow(clippy::all, dead_code)]
mod reference {
    // Exact signs for region.rs's three squared comparisons (verify/sheet's Region.lean: `near`,
    // `within`, `inDisc`): a float filter where its error bound proves the sign, else the value
    // recomputed exactly with floating-point expansions (Shewchuk 1997: two-sum, Dekker's
    // two-product, grow-expansion). Every input is a double, so a dyadic rational, and every
    // expansion here is exact; the sign of a nonoverlapping expansion is its largest component's.
    //
    // The filters: each value is a sum of products of rounded differences of doubles, `v`, with an
    // absolute error at most `FILTER · m`, `m` the sum of the products' magnitudes (each difference
    // rounds once, relatively, each product and sum once more: well under 8 ε of `m`; 16 ε is
    // taken). Where `|v| > FILTER · m` the float sign is the exact one; else the exact path decides.
    // Like predicates.rs, this assumes no product underflows inexactly (inputs within its windows).

    const EPS: f64 = f64::EPSILON * 0.5;
    /// The filters' relative bound.
    const FILTER: f64 = 16.0 * EPS;
    const SPLITTER: f64 = 134_217_729.0; // 2^27 + 1

    fn two_sum(a: f64, b: f64) -> (f64, f64) {
        let x = a + b;
        let bv = x - a;
        let av = x - bv;
        (x, (a - av) + (b - bv))
    }

    fn split(a: f64) -> (f64, f64) {
        let c = SPLITTER * a;
        let hi = c - (c - a);
        (hi, a - hi)
    }

    fn two_product(a: f64, b: f64) -> (f64, f64) {
        let x = a * b;
        let (ah, al) = split(a);
        let (bh, bl) = split(b);
        let err = x - ah * bh - al * bh - ah * bl;
        (x, al * bl - err)
    }

    /// An exact value: a sum of doubles, nonoverlapping, increasing in magnitude (zeros dropped).
    #[derive(Clone, Debug, Default)]
    struct Exp(Vec<f64>);

    impl Exp {
        fn of(x: f64) -> Exp {
            Exp(if x == 0.0 { vec![] } else { vec![x] })
        }
        /// `a - b` exactly.
        fn diff(a: f64, b: f64) -> Exp {
            let mut e = Exp::of(a);
            e.grow(-b);
            e
        }
        /// Adds `b` exactly (grow-expansion).
        fn grow(&mut self, b: f64) {
            let mut q = b;
            let mut out = Vec::with_capacity(self.0.len() + 1);
            for &e in &self.0 {
                let (s, lo) = two_sum(q, e);
                if lo != 0.0 {
                    out.push(lo);
                }
                q = s;
            }
            if q != 0.0 {
                out.push(q);
            }
            self.0 = out;
        }
        fn add(&mut self, o: &Exp) {
            for &x in &o.0 {
                self.grow(x);
            }
        }
        fn neg(&self) -> Exp {
            Exp(self.0.iter().map(|x| -x).collect())
        }
        /// The product exactly: every pair's two-product, grown in.
        fn mul(&self, o: &Exp) -> Exp {
            let mut out = Exp::default();
            for &a in &self.0 {
                for &b in &o.0 {
                    let (x, y) = two_product(a, b);
                    out.grow(y);
                    out.grow(x);
                }
            }
            out
        }
        /// The sign: the largest component's.
        fn sign(&self) -> f64 {
            self.0
                .iter()
                .rev()
                .find(|x| **x != 0.0)
                .map_or(0.0, |x| x.signum())
        }
    }

    /// The sign of |p − c|² − r², exactly.
    pub fn disc(px: f64, py: f64, cx: f64, cy: f64, r: f64) -> f64 {
        let (dx, dy) = (px - cx, py - cy);
        let v = dx * dx + dy * dy - r * r;
        if v.abs() > FILTER * (dx * dx + dy * dy + r * r) {
            return v.signum();
        }
        let (ex, ey, er) = (Exp::diff(px, cx), Exp::diff(py, cy), Exp::of(r));
        let mut s = ex.mul(&ex);
        s.add(&ey.mul(&ey));
        s.add(&er.mul(&er).neg());
        s.sign()
    }

    /// The sign of orient(a, b, c)² − l² |b − a|², exactly.
    pub fn line_gap(ax: f64, ay: f64, bx: f64, by: f64, cx: f64, cy: f64, l: f64) -> f64 {
        let (ux, uy, vx, vy) = (bx - ax, by - ay, cx - ax, cy - ay);
        let o = ux * vy - uy * vx;
        let n = ux * ux + uy * uy;
        let v = o * o - l * l * n;
        let m = (ux * vy).abs() + (uy * vx).abs();
        if v.abs() > FILTER * (m * m + l * l * n) {
            return v.signum();
        }
        let (eux, euy, evx, evy) = (
            Exp::diff(bx, ax),
            Exp::diff(by, ay),
            Exp::diff(cx, ax),
            Exp::diff(cy, ay),
        );
        let mut o = eux.mul(&evy);
        o.add(&euy.mul(&evx).neg());
        let mut n = eux.mul(&eux);
        n.add(&euy.mul(&euy));
        let el = Exp::of(l);
        let mut s = o.mul(&o);
        s.add(&el.mul(&el).mul(&n).neg());
        s.sign()
    }
}

#[test]
fn exact_is_fix_sheets() {
    use forge_sheet_core::exact::{disc, line_gap};
    let mut x: u64 = 0xe7ac7;
    let mut next = || {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        (x >> 11) as f64 / (1u64 << 53) as f64
    };
    let mut ties = 0;
    for k in 0..20000 {
        // random, then near-ties: points on and near circles and lines by construction
        let (px, py, cx, cy) = (100.0 * next(), 100.0 * next(), 100.0 * next(), 100.0 * next());
        let r = if k % 2 == 0 {
            ((px - cx).powi(2) + (py - cy).powi(2)).sqrt()
        } else {
            50.0 * next()
        };
        let a = disc(px, py, cx, cy, r);
        assert_eq!(
            a.to_bits(),
            reference::disc(px, py, cx, cy, r).to_bits(),
            "{px} {py} {cx} {cy} {r}"
        );
        let (ax, ay, bx, by) = (10.0 * next(), 10.0 * next(), 10.0 * next(), 10.0 * next());
        let t = next();
        let (qx, qy) = (ax + t * (bx - ax) + 1e-6 * (next() - 0.5), ay + t * (by - ay));
        let l = if k % 3 == 0 { 1e-5 } else { 1e-6 * next() };
        let b = line_gap(ax, ay, bx, by, qx, qy, l);
        assert_eq!(
            b.to_bits(),
            reference::line_gap(ax, ay, bx, by, qx, qy, l).to_bits()
        );
        if a == 0.0 || b == 0.0 {
            ties += 1;
        }
    }
    // exact ties and near-ties by construction: Pythagorean triples at dyadic scales and
    // offsets (on the circle exactly, or one ulp off), points exactly l off a line
    for k in 0..4000u64 {
        let (a, b, c) = [(3.0, 4.0, 5.0), (5.0, 12.0, 13.0), (8.0, 15.0, 17.0)][(k % 3) as usize];
        let sc = [0.25, 1.0, 1024.0, 1.0 / 1024.0][(k / 3 % 4) as usize];
        let (ox, oy) = ((k % 97) as f64 * 0.5, (k % 89) as f64 * 0.125);
        let (sx, sy) = if k % 2 == 0 { (1.0, 1.0) } else { (-1.0, 1.0) };
        let px = ox + sx * a * sc;
        let py = oy + sy * b * sc;
        for r in [
            c * sc,
            f64::from_bits((c * sc).to_bits() + 1),
            f64::from_bits((c * sc).to_bits() - 1),
        ] {
            let v = disc(px, py, ox, oy, r);
            assert_eq!(v.to_bits(), reference::disc(px, py, ox, oy, r).to_bits());
            if v == 0.0 {
                ties += 1;
            }
        }
        let l = [1e-5, 0.5, 2.0][(k % 3) as usize];
        let len = [1.0, 3.0, 0.75][(k / 3 % 3) as usize];
        let (ax, ay) = (ox, oy);
        for (bx, by, qx, qy) in [
            (ax + len, ay, ax + 0.3, ay + l),
            (ax, ay + len, ax - l, ay + 0.6),
            (ax + len, ay, ax + 0.3, f64::from_bits((ay + l).to_bits() + 1)),
        ] {
            let v = line_gap(ax, ay, bx, by, qx, qy, l);
            assert_eq!(
                v.to_bits(),
                reference::line_gap(ax, ay, bx, by, qx, qy, l).to_bits()
            );
            if v == 0.0 {
                ties += 1;
            }
        }
    }
    // the reference's own pinned cases
    assert_eq!(disc(3.0, 4.0, 0.0, 0.0, 5.0), 0.0);
    assert_eq!(disc(0.1 + 0.2, 0.0, 0.1, 0.0, 0.2), 1.0);
    assert_eq!(line_gap(0.0, 0.0, 1.0, 0.0, 5.0, 2.0, 2.0), 0.0);
    assert_eq!(line_gap(0.0, 0.0, 3.0, 0.0, 1.0, 1e-5, 1e-5), 0.0);
    eprintln!("exact ties: {ties}");
    assert!(ties > 100, "too few exact ties: {ties}");
}
