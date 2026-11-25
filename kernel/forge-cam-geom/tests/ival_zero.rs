//! Adversarial programs in interval derivatives (`dual::D`) against the strict interval rules
//! (forge-num `Ival::add_strict` / `Ival::mul_strict`, no exact-zero rule): every value and
//! derivative enclosure the used rules give lies inside the strict rules' (each used rule is the
//! strict one or an exact result inside it, and the strict rules are inclusion-isotone), and the
//! strict enclosure at sampled points of the box meets it (soundness: the true value is in both).
//! Exact zeros, zero-width boxes, sign-crossing boxes, huge and tiny magnitudes, and overflow to
//! infinity are all drawn on purpose.
use forge_cam_geom::dual::{D, Num};
use forge_cam_geom::ival::Ival;

/// The strict rules, as a `Num`.
#[derive(Clone, Copy, Debug)]
struct Si(Ival);

impl Num for Si {
    fn c(x: f64) -> Si {
        Si(Ival::pt(x))
    }
    fn add(self, o: Si) -> Si {
        Si(self.0.add_strict(o.0))
    }
    fn sub(self, o: Si) -> Si {
        Si(self.0.add_strict(o.0.neg()))
    }
    fn mul(self, o: Si) -> Si {
        Si(self.0.mul_strict(o.0))
    }
    fn div(self, o: Si) -> Si {
        Si(self.0.div(o.0))
    }
    fn sqr(self) -> Si {
        Si(self.0.sqr())
    }
}

/// `dual::D`'s formulas over the strict rules.
#[derive(Clone, Copy, Debug)]
struct Sd {
    v: Si,
    g: [Si; 3],
}

const SZ: Si = Si(Ival { lo: 0.0, hi: 0.0 });

impl Sd {
    fn var(x: Ival, k: usize) -> Sd {
        let mut g = [SZ; 3];
        g[k] = Si::c(1.0);
        Sd { v: Si(x), g }
    }
}

impl Num for Sd {
    fn c(x: f64) -> Sd {
        Sd {
            v: Si::c(x),
            g: [SZ; 3],
        }
    }
    fn add(self, o: Sd) -> Sd {
        Sd {
            v: self.v.add(o.v),
            g: [0, 1, 2].map(|k| self.g[k].add(o.g[k])),
        }
    }
    fn sub(self, o: Sd) -> Sd {
        Sd {
            v: self.v.sub(o.v),
            g: [0, 1, 2].map(|k| self.g[k].sub(o.g[k])),
        }
    }
    fn mul(self, o: Sd) -> Sd {
        Sd {
            v: self.v.mul(o.v),
            g: [0, 1, 2].map(|k| self.g[k].mul(o.v).add(self.v.mul(o.g[k]))),
        }
    }
    fn div(self, o: Sd) -> Sd {
        let q = self.v.div(o.v);
        Sd {
            v: q,
            g: [0, 1, 2].map(|k| self.g[k].sub(q.mul(o.g[k])).div(o.v)),
        }
    }
    fn sqr(self) -> Sd {
        let two = self.v.mul(Si::c(2.0));
        Sd {
            v: self.v.sqr(),
            g: [0, 1, 2].map(|k| two.mul(self.g[k])),
        }
    }
}

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
    fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }
    /// A float that provokes: exact zeros of both signs, small integers, ulp-near values, and
    /// magnitudes that underflow or overflow when multiplied.
    fn f(&mut self) -> f64 {
        match self.below(10) {
            0 => 0.0,
            1 => -0.0,
            2 => self.below(7) as f64 - 3.0,
            3 => (self.below(2) as f64 * 2.0 - 1.0) * 1e300 * (1.0 + self.unit()),
            4 => (self.below(2) as f64 * 2.0 - 1.0) * 1e-300 * (1.0 + self.unit()),
            5 => 1.0f64.next_up(),
            _ => (self.unit() - 0.5) * 20.0,
        }
    }
    fn ival(&mut self) -> Ival {
        let a = self.f();
        match self.below(4) {
            0 => Ival::pt(a),
            1 => Ival::new(-a.abs(), a.abs()),
            _ => Ival::new(a, self.f()),
        }
    }
}

#[derive(Clone, Copy)]
enum Op {
    Var(usize),
    Const(f64),
    Add(usize, usize),
    Sub(usize, usize),
    Mul(usize, usize),
    Div(usize, usize),
    Sqr(usize),
}

fn program(r: &mut Rng) -> Vec<Op> {
    let mut p = vec![Op::Var(0), Op::Var(1), Op::Var(2)];
    let n = 3 + r.below(25) as usize;
    for _ in 0..n {
        let m = p.len() as u64;
        let (i, j) = (r.below(m) as usize, r.below(m) as usize);
        p.push(match r.below(8) {
            0 => Op::Const(r.f()),
            1 => Op::Add(i, j),
            2 => Op::Sub(i, j),
            3 | 4 => Op::Mul(i, j),
            5 => Op::Div(i, j),
            6 => Op::Sqr(i),
            _ => Op::Mul(i, p.len() - 1),
        });
    }
    p
}

fn run<N: Num>(p: &[Op], var: impl Fn(usize) -> N) -> Vec<N> {
    let mut s: Vec<N> = Vec::with_capacity(p.len());
    for op in p {
        let v = match *op {
            Op::Var(k) => var(k),
            Op::Const(x) => N::c(x),
            Op::Add(i, j) => s[i].add(s[j]),
            Op::Sub(i, j) => s[i].sub(s[j]),
            Op::Mul(i, j) => s[i].mul(s[j]),
            Op::Div(i, j) => s[i].div(s[j]),
            Op::Sqr(i) => s[i].sqr(),
        };
        s.push(v);
    }
    s
}

fn inside(a: Ival, b: Ival) -> bool {
    b.lo <= a.lo && a.hi <= b.hi
}
fn meets(a: Ival, b: Ival) -> bool {
    a.lo <= b.hi && b.lo <= a.hi
}

/// A point of the box: an end, the midpoint, or a random interior point (finite only).
fn point(r: &mut Rng, x: Ival) -> f64 {
    let t = match r.below(4) {
        0 => x.lo,
        1 => x.hi,
        2 => x.lo * 0.5 + x.hi * 0.5,
        _ => x.lo + (x.hi - x.lo) * r.unit(),
    };
    t.clamp(x.lo, x.hi)
}

#[test]
fn derivatives_under_the_exact_zero_rule_stay_inside_the_strict_rule_and_sound() {
    let mut r = Rng(0x9E37_79B9_7F4A_7C15);
    let (mut slots, mut narrower) = (0u64, 0u64);
    for _ in 0..20_000 {
        let p = program(&mut r);
        let x = [r.ival(), r.ival(), r.ival()];
        let used = run(&p, |k| D::var(x[k], k));
        let strict = run(&p, |k| Sd::var(x[k], k));
        for (u, s) in used.iter().zip(&strict) {
            slots += 1;
            let pairs = [
                (u.v, s.v.0),
                (u.g[0], s.g[0].0),
                (u.g[1], s.g[1].0),
                (u.g[2], s.g[2].0),
            ];
            for (a, b) in pairs {
                assert!(inside(a, b), "used {a:?} not inside strict {b:?}");
                if a != b {
                    narrower += 1;
                }
            }
        }
        // soundness: at points of the box, the strict point enclosure (which holds the true
        // value and derivatives) meets every enclosure over the box
        for _ in 0..4 {
            let t = [0, 1, 2].map(|k| point(&mut r, x[k]));
            if t.iter().any(|v| !v.is_finite()) {
                continue;
            }
            let at = run(&p, |k| Sd::var(Ival::pt(t[k]), k));
            for (u, s) in used.iter().zip(&at) {
                assert!(
                    meets(u.v, s.v.0),
                    "value {:?} misses the point's {:?}",
                    u.v,
                    s.v.0
                );
                for k in 0..3 {
                    assert!(meets(u.g[k], s.g[k].0), "∂{k} {:?} misses {:?}", u.g[k], s.g[k].0);
                }
            }
        }
    }
    assert!(slots > 200_000, "{slots}");
    // the rule must actually be exercised (in the default build)
    if cfg!(not(feature = "ival-strict")) {
        assert!(narrower > slots / 10, "{narrower} of {slots}");
    } else {
        assert_eq!(narrower, 0);
    }
}

/// A derivative in a parameter an expression does not depend on stays the exact point 0 through
/// sums, products and squares (the strict rule widens it by an ulp each step), and a value times an
/// exact-zero constant is exactly zero.
#[test]
#[cfg(not(feature = "ival-strict"))]
fn an_absent_parameter_keeps_an_exactly_zero_derivative() {
    let x = D::var(Ival::new(-1.5, 2.25), 0);
    let y = D::var(Ival::new(0.5, 0.75), 1);
    let f = x.mul(x).add(x.mul(D::c(3.0))).sqr().sub(x);
    for k in [1, 2] {
        assert_eq!(f.g[k], Ival { lo: 0.0, hi: 0.0 }, "{f:?}");
    }
    let h = f.mul(y).mul(D::c(0.0));
    assert_eq!(h.v, Ival { lo: 0.0, hi: 0.0 });
    assert_eq!(h.g, [Ival { lo: 0.0, hi: 0.0 }; 3]);
    // an unbounded operand keeps the general rule: [0, 0] · [−∞, ∞] is not hidden as 0
    let w = Ival::new(f64::NEG_INFINITY, 1.0).mul(Ival::pt(0.0));
    assert_eq!((w.lo, w.hi), (f64::NEG_INFINITY, f64::INFINITY), "{w:?}");
}
