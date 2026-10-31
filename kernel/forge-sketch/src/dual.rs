//! Forward-mode dual numbers over the (at most eight) parameters one
//! constraint touches, so every residual comes with its exact gradient.

use core::ops::{Add, Div, Mul, Neg, Sub};

pub(crate) const N: usize = 8;

#[derive(Clone, Copy, Debug)]
pub(crate) struct D {
    pub v: f64,
    pub g: [f64; N],
}

impl D {
    pub fn c(v: f64) -> D {
        D { v, g: [0.0; N] }
    }
    pub fn sqrt(self) -> D {
        let s = self.v.max(0.0).sqrt();
        let k = if s > 0.0 { 0.5 / s } else { 0.0 };
        D {
            v: s,
            g: self.g.map(|x| x * k),
        }
    }
}

impl Add for D {
    type Output = D;
    fn add(self, o: D) -> D {
        let mut g = self.g;
        for (a, b) in g.iter_mut().zip(o.g) {
            *a += b;
        }
        D { v: self.v + o.v, g }
    }
}

impl Sub for D {
    type Output = D;
    fn sub(self, o: D) -> D {
        self + (-o)
    }
}

impl Neg for D {
    type Output = D;
    fn neg(self) -> D {
        D {
            v: -self.v,
            g: self.g.map(|x| -x),
        }
    }
}

impl Mul for D {
    type Output = D;
    fn mul(self, o: D) -> D {
        let mut g = [0.0; N];
        for (k, gk) in g.iter_mut().enumerate() {
            *gk = self.g[k] * o.v + o.g[k] * self.v;
        }
        D { v: self.v * o.v, g }
    }
}

impl Mul<f64> for D {
    type Output = D;
    fn mul(self, s: f64) -> D {
        D {
            v: self.v * s,
            g: self.g.map(|x| x * s),
        }
    }
}

impl Div for D {
    type Output = D;
    fn div(self, o: D) -> D {
        let inv = 1.0 / o.v;
        let mut g = [0.0; N];
        for (k, gk) in g.iter_mut().enumerate() {
            *gk = (self.g[k] * o.v - o.g[k] * self.v) * inv * inv;
        }
        D { v: self.v * inv, g }
    }
}

/// A 2D vector of duals.
#[derive(Clone, Copy, Debug)]
pub(crate) struct V {
    pub x: D,
    pub y: D,
}

impl V {
    pub fn sub(self, o: V) -> V {
        V {
            x: self.x - o.x,
            y: self.y - o.y,
        }
    }
    pub fn add(self, o: V) -> V {
        V {
            x: self.x + o.x,
            y: self.y + o.y,
        }
    }
    pub fn scale(self, s: f64) -> V {
        V {
            x: self.x * s,
            y: self.y * s,
        }
    }
    pub fn dot(self, o: V) -> D {
        self.x * o.x + self.y * o.y
    }
    pub fn cross(self, o: V) -> D {
        self.x * o.y - self.y * o.x
    }
    pub fn len(self) -> D {
        self.dot(self).sqrt()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gradients() {
        let mut x = D::c(3.0);
        x.g[0] = 1.0;
        let mut y = D::c(4.0);
        y.g[1] = 1.0;
        let r = (x * x + y * y).sqrt();
        assert_eq!(r.v, 5.0);
        assert!((r.g[0] - 0.6).abs() < 1e-15 && (r.g[1] - 0.8).abs() < 1e-15);
        let q = x / y;
        assert!((q.g[0] - 0.25).abs() < 1e-15 && (q.g[1] + 3.0 / 16.0).abs() < 1e-15);
    }
}
