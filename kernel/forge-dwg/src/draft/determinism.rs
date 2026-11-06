//! The kernel gives the same bits on every target: a digest of many operations' results, pinned
//! (the wasm build runs the same test).

use super::entity::to_data;
use super::fillet::fillet;
use super::geom::{Affine, TAU, V2, v2};
use super::intersect::{Ext, intersect};
use super::offset::offset;
use super::ops::trim;
use super::{Curve, Ellipse, Nurbs};
use crate::types::Data;

struct Digest(u64);

impl Digest {
    fn f(&mut self, x: f64) {
        for b in x.to_bits().to_le_bytes() {
            self.0 = (self.0 ^ b as u64).wrapping_mul(0x100_0000_01b3);
        }
    }
    fn p(&mut self, p: V2) {
        self.f(p.x);
        self.f(p.y);
    }
    fn c(&mut self, c: &Curve) {
        let (a, b) = c.domain();
        for k in 0..=8 {
            self.p(c.point(a + (b - a) * k as f64 / 8.0));
        }
    }
}

fn digest() -> u64 {
    let mut d = Digest(0xcbf2_9ce4_8422_2325);
    let e = Curve::Ellipse(Ellipse {
        c: v2(1.0, 2.0),
        major: v2(7.0, 3.0),
        ratio: 0.45,
        start: 0.2,
        sweep: 5.0,
    });
    let s = Curve::Spline(Nurbs {
        degree: 3,
        knots: vec![0.0, 0.0, 0.0, 0.0, 1.0, 2.5, 3.0, 3.0, 3.0, 3.0],
        ctrl: vec![
            v2(-8.0, 0.0),
            v2(-4.0, 9.0),
            v2(0.0, -7.0),
            v2(3.0, 6.0),
            v2(6.0, -2.0),
            v2(9.0, 4.0),
        ],
        weights: Some(vec![1.0, 0.8, 1.3, 1.0, 0.7, 1.0]),
        closed: false,
    });
    let c = Curve::circle(v2(0.5, 0.3), 4.2);
    let l = Curve::line(v2(-12.0, -1.0), v2(11.0, 2.5));
    let curves = [e.clone(), s.clone(), c.clone(), l.clone()];
    for a in &curves {
        d.f(a.length());
        let (t, q) = a.closest(v2(2.0, 3.0));
        d.f(t);
        d.p(q);
        for b in &curves {
            for h in intersect(a, b, Ext::Both).points {
                d.f(h.ta);
                d.f(h.tb);
                d.p(h.p);
            }
        }
    }
    let r = e.transformed(&Affine::rotate(v2(1.0, 1.0), 0.8)).unwrap();
    d.c(&r);
    for p in trim(&c, &[l.clone(), s.clone()], v2(0.5, 4.5), false).unwrap() {
        d.c(&p);
    }
    let o = offset(&e, 0.6, v2(30.0, 0.0)).unwrap();
    if let Some(Data::Spline(sp)) = to_data(&o, 0.0) {
        sp.points.iter().for_each(|p| d.p(V2::from(*p)));
    }
    let f = fillet(&l, v2(10.0, 2.4), &c, v2(0.5, 4.5), 1.3).unwrap();
    d.c(&f.a);
    d.c(f.added.as_ref().unwrap());
    let full = Curve::Ellipse(Ellipse {
        sweep: TAU,
        ..match e {
            Curve::Ellipse(x) => x,
            _ => unreachable!(),
        }
    });
    d.f(full.length());
    d.0
}

#[test]
fn same_bits_on_every_target() {
    let got = digest();
    assert_eq!(got, digest());
    assert_eq!(got, PINNED, "{got:#x}");
}

const PINNED: u64 = 0x721b_4302_9106_80ed;
