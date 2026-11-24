//! Exact dyadic geometry: the signs the decide-once boolean's ledger needs (a plane at the meet of
//! three others, a point against a polygon's edge) computed with no rounding at all and no
//! precondition on the inputs' range. The numbers are forge-num's ([`Dy`], a big integer times a
//! power of two); slow next to the floating filters of `predicates`, so callers filter first.

pub use forge_num::dy::{det2, ratio_f64};
pub use forge_num::{Big, Dy};

/// An exact 3-vector.
#[derive(Clone, Debug, Default)]
pub struct DyV3 {
    pub x: Dy,
    pub y: Dy,
    pub z: Dy,
}

impl DyV3 {
    pub fn new(x: Dy, y: Dy, z: Dy) -> DyV3 {
        DyV3 { x, y, z }
    }

    pub fn from_f64(x: f64, y: f64, z: f64) -> DyV3 {
        DyV3::new(Dy::from_f64(x), Dy::from_f64(y), Dy::from_f64(z))
    }

    pub fn dot(&self, o: &DyV3) -> Dy {
        self.x.mul(&o.x).add(&self.y.mul(&o.y)).add(&self.z.mul(&o.z))
    }

    pub fn cross(&self, o: &DyV3) -> DyV3 {
        DyV3::new(
            det2(&self.y, &self.z, &o.y, &o.z),
            det2(&self.z, &self.x, &o.z, &o.x),
            det2(&self.x, &self.y, &o.x, &o.y),
        )
    }

    pub fn scale(&self, k: &Dy) -> DyV3 {
        DyV3::new(self.x.mul(k), self.y.mul(k), self.z.mul(k))
    }

    pub fn add(&self, o: &DyV3) -> DyV3 {
        DyV3::new(self.x.add(&o.x), self.y.add(&o.y), self.z.add(&o.z))
    }

    pub fn sub(&self, o: &DyV3) -> DyV3 {
        DyV3::new(self.x.sub(&o.x), self.y.sub(&o.y), self.z.sub(&o.z))
    }

    pub fn is_zero(&self) -> bool {
        self.x.is_zero() && self.y.is_zero() && self.z.is_zero()
    }

    pub fn get(&self, axis: usize) -> &Dy {
        match axis {
            0 => &self.x,
            1 => &self.y,
            _ => &self.z,
        }
    }
}

/// A plane `n · x = c`, exactly.
#[derive(Clone, Debug, Default)]
pub struct DyPlane {
    pub n: DyV3,
    pub c: Dy,
}

/// A point in homogeneous form `(X, Y, Z) / W`, `W ≠ 0`: the exact meet of three planes, or a
/// combination of such meets.
#[derive(Clone, Debug, Default)]
pub struct HPoint {
    pub p: DyV3,
    pub w: Dy,
}

impl DyPlane {
    /// The plane through `o` with normal `n`, from the kernel's f64 frame, exactly.
    pub fn from_frame(o: [f64; 3], n: [f64; 3]) -> DyPlane {
        let n = DyV3::from_f64(n[0], n[1], n[2]);
        let o = DyV3::from_f64(o[0], o[1], o[2]);
        let c = n.dot(&o);
        DyPlane { n, c }
    }

    pub fn flipped(&self) -> DyPlane {
        DyPlane {
            n: DyV3::new(self.n.x.neg(), self.n.y.neg(), self.n.z.neg()),
            c: self.c.neg(),
        }
    }

    /// `n · x − c` at an exact point.
    pub fn eval(&self, x: &DyV3) -> Dy {
        self.n.dot(x).sub(&self.c)
    }

    /// The sign of the plane at a homogeneous point (`n · X − c W`, times the sign of `W`).
    pub fn side(&self, h: &HPoint) -> i32 {
        self.n.dot(&h.p).sub(&self.c.mul(&h.w)).signum() * h.w.signum()
    }

    /// The same point set (normals parallel, offsets in proportion).
    pub fn same_set(&self, o: &DyPlane) -> bool {
        if !self.n.cross(&o.n).is_zero() {
            return false;
        }
        // n₁ = k n₂: then c₁ = k c₂, i.e. c₁ n₂ = c₂ n₁ componentwise
        self.n.scale(&o.c).sub(&o.n.scale(&self.c)).is_zero()
    }
}

/// The point on three planes, if they meet in one (`meet3` of the reference).
pub fn meet3(a: &DyPlane, b: &DyPlane, c: &DyPlane) -> Option<HPoint> {
    let bc = b.n.cross(&c.n);
    let d = a.n.dot(&bc);
    if d.is_zero() {
        return None;
    }
    let p = bc
        .scale(&a.c)
        .add(&c.n.cross(&a.n).scale(&b.c))
        .add(&a.n.cross(&b.n).scale(&c.c));
    Some(HPoint { p, w: d })
}

impl HPoint {
    /// The point exactly (`W = 1`).
    pub fn from_dy(p: DyV3) -> HPoint {
        HPoint {
            p,
            w: Dy::from_i64(1),
        }
    }

    /// The nearest f64 point, or next to it (output only).
    pub fn to_f64(&self) -> [f64; 3] {
        [
            ratio_f64(&self.p.x, &self.w),
            ratio_f64(&self.p.y, &self.w),
            ratio_f64(&self.p.z, &self.w),
        ]
    }

    /// The sign of `self.axis − o.axis`.
    pub fn cmp_axis(&self, o: &HPoint, axis: usize) -> i32 {
        let a = self.p.get(axis).mul(&o.w);
        let b = o.p.get(axis).mul(&self.w);
        a.sub(&b).signum() * self.w.signum() * o.w.signum()
    }

    /// The same point.
    pub fn same(&self, o: &HPoint) -> bool {
        (0..3).all(|a| self.cmp_axis(o, a) == 0)
    }

    /// The mean of points.
    pub fn mean(ps: &[&HPoint]) -> HPoint {
        let mut w = Dy::from_i64(ps.len() as i64);
        for q in ps {
            w = w.mul(&q.w);
        }
        let mut p = DyV3::default();
        for (i, q) in ps.iter().enumerate() {
            let mut k = Dy::from_i64(1);
            for (j, r) in ps.iter().enumerate() {
                if i != j {
                    k = k.mul(&r.w);
                }
            }
            p = p.add(&q.p.scale(&k));
        }
        HPoint { p, w }
    }

    /// `self + t · d` (d a direction, t a dyadic).
    pub fn offset(&self, d: &DyV3, t: &Dy) -> HPoint {
        HPoint {
            p: self.p.add(&d.scale(&t.mul(&self.w))),
            w: self.w.clone(),
        }
    }
}

/// The sign of the 2D orientation of three homogeneous points projected to axes `(u, v)`:
/// positive counter-clockwise.
pub fn orient2(a: &HPoint, b: &HPoint, c: &HPoint, u: usize, v: usize) -> i32 {
    // det [[Au Av Aw] [Bu Bv Bw] [Cu Cv Cw]] · sign(Aw Bw Cw)
    let (au, av, aw) = (a.p.get(u), a.p.get(v), &a.w);
    let (bu, bv, bw) = (b.p.get(u), b.p.get(v), &b.w);
    let (cu, cv, cw) = (c.p.get(u), c.p.get(v), &c.w);
    let d = au
        .mul(&det2(bv, bw, cv, cw))
        .sub(&av.mul(&det2(bu, bw, cu, cw)))
        .add(&aw.mul(&det2(bu, bv, cu, cv)));
    d.signum() * aw.signum() * bw.signum() * cw.signum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::cmp::Ordering;

    #[test]
    fn dyadic_round_trips() {
        for x in [
            1.0,
            -0.1,
            3.5e-300,
            1e300,
            5e-324,
            core::f64::consts::FRAC_1_SQRT_2,
            12345.678,
        ] {
            assert_eq!(Dy::from_f64(x).to_f64_near(), x);
        }
    }

    #[test]
    fn arithmetic_is_exact() {
        // 0.1 + 0.2 - 0.3 is not zero in dyadics, and is exactly the f64 rounding error
        let s = Dy::from_f64(0.1).add(&Dy::from_f64(0.2)).sub(&Dy::from_f64(0.3));
        assert_eq!(s.signum(), 1);
        let a = Dy::from_f64(1e20);
        let b = Dy::from_f64(1e-20);
        assert_eq!(a.add(&b).sub(&a).cmp_dy(&b), Ordering::Equal);
        assert_eq!(a.mul(&b).signum(), 1);
    }

    /// Ring laws on numbers whose exponents span hundreds of bits, so sums and products cross
    /// between the inline and the limb forms both ways.
    #[test]
    fn inline_and_limbs_agree() {
        let mut r = crate::rng::Rng::new(7);
        let mut pick = || {
            let m = r.range(-1.0, 1.0);
            let e = (r.range(-200.0, 200.0)) as i64;
            Dy::from_f64(m).scale2(e)
        };
        for _ in 0..3000 {
            let (x, y, z) = (pick(), pick(), pick());
            assert_eq!(x.add(&y).sub(&y).cmp_dy(&x), Ordering::Equal);
            assert_eq!(x.mul(&y).mul(&z).cmp_dy(&x.mul(&y.mul(&z))), Ordering::Equal);
            assert_eq!(
                x.mul(&y.add(&z)).cmp_dy(&x.mul(&y).add(&x.mul(&z))),
                Ordering::Equal
            );
            assert_eq!(x.sub(&x).signum(), 0);
            let (fx, fy) = (x.to_f64_near(), y.to_f64_near());
            if fx.is_finite() && fy.is_finite() && (fx - fy).abs() > 1e-9 * (fx.abs() + fy.abs()) {
                assert_eq!(x.cmp_dy(&y), fx.partial_cmp(&fy).unwrap());
            }
            let big = x.mul(&x).mul(&x).mul(&x);
            assert_eq!(big.sub(&big).signum(), 0);
            assert_eq!(big.signum(), if x.is_zero() { 0 } else { 1 });
        }
    }

    #[test]
    fn three_planes_meet() {
        let x = DyPlane::from_frame([1.0, 0.0, 0.0], [1.0, 0.0, 0.0]);
        let y = DyPlane::from_frame([0.0, 2.0, 0.0], [0.0, 1.0, 0.0]);
        let z = DyPlane::from_frame([0.0, 0.0, 3.0], [0.0, 0.0, 1.0]);
        let p = meet3(&x, &y, &z).unwrap();
        assert_eq!(p.to_f64(), [1.0, 2.0, 3.0]);
        let tilt = DyPlane::from_frame([1.0, 2.0, 3.0], [0.6, 0.8, 0.0]);
        assert_eq!(tilt.side(&p), 0);
        let off = DyPlane::from_frame([1.0, 2.0, 3.0 + 1e-15], [0.0, 0.0, 1.0]);
        assert_eq!(off.side(&p), -1);
        assert!(x.same_set(&x.flipped()));
        assert!(!x.same_set(&y));
    }
}
