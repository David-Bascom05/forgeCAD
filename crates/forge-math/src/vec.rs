use core::ops::{Add, AddAssign, Div, Index, Mul, Neg, Sub, SubAssign};

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Vec2 {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Point2 {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Point3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

/// Length 1 by construction (within rounding of the normalising division).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UnitVec3(Vec3);

impl Vec2 {
    pub const ZERO: Vec2 = Vec2 { x: 0.0, y: 0.0 };
    pub const X: Vec2 = Vec2 { x: 1.0, y: 0.0 };
    pub const Y: Vec2 = Vec2 { x: 0.0, y: 1.0 };

    pub const fn new(x: f64, y: f64) -> Self {
        Vec2 { x, y }
    }
    pub fn dot(self, o: Vec2) -> f64 {
        self.x * o.x + self.y * o.y
    }
    /// z of the 3D cross product.
    pub fn cross(self, o: Vec2) -> f64 {
        self.x * o.y - self.y * o.x
    }
    pub fn length_sq(self) -> f64 {
        self.dot(self)
    }
    pub fn length(self) -> f64 {
        self.length_sq().sqrt()
    }
    pub fn perp(self) -> Vec2 {
        Vec2::new(-self.y, self.x)
    }
    pub fn normalize(self) -> Option<Vec2> {
        let l = self.length();
        (l > 0.0 && l.is_finite()).then(|| self / l)
    }
}

impl Point2 {
    pub const ORIGIN: Point2 = Point2 { x: 0.0, y: 0.0 };

    pub const fn new(x: f64, y: f64) -> Self {
        Point2 { x, y }
    }
    pub fn to_vec(self) -> Vec2 {
        Vec2::new(self.x, self.y)
    }
    pub fn distance(self, o: Point2) -> f64 {
        (self - o).length()
    }
    pub fn lerp(self, o: Point2, t: f64) -> Point2 {
        self + (o - self) * t
    }
}

impl Vec3 {
    pub const ZERO: Vec3 = Vec3 {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };
    pub const X: Vec3 = Vec3 {
        x: 1.0,
        y: 0.0,
        z: 0.0,
    };
    pub const Y: Vec3 = Vec3 {
        x: 0.0,
        y: 1.0,
        z: 0.0,
    };
    pub const Z: Vec3 = Vec3 {
        x: 0.0,
        y: 0.0,
        z: 1.0,
    };

    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Vec3 { x, y, z }
    }
    pub fn dot(self, o: Vec3) -> f64 {
        self.x * o.x + self.y * o.y + self.z * o.z
    }
    pub fn cross(self, o: Vec3) -> Vec3 {
        Vec3::new(
            self.y * o.z - self.z * o.y,
            self.z * o.x - self.x * o.z,
            self.x * o.y - self.y * o.x,
        )
    }
    pub fn length_sq(self) -> f64 {
        self.dot(self)
    }
    pub fn length(self) -> f64 {
        self.length_sq().sqrt()
    }
    pub fn normalize(self) -> Option<UnitVec3> {
        let l = self.length();
        (l > 0.0 && l.is_finite()).then(|| UnitVec3(self / l))
    }
    pub fn abs_max(self) -> f64 {
        self.x.abs().max(self.y.abs()).max(self.z.abs())
    }
    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.z.is_finite()
    }
}

impl Point3 {
    pub const ORIGIN: Point3 = Point3 {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };

    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Point3 { x, y, z }
    }
    pub fn to_vec(self) -> Vec3 {
        Vec3::new(self.x, self.y, self.z)
    }
    pub fn distance(self, o: Point3) -> f64 {
        (self - o).length()
    }
    pub fn distance_sq(self, o: Point3) -> f64 {
        (self - o).length_sq()
    }
    pub fn lerp(self, o: Point3, t: f64) -> Point3 {
        self + (o - self) * t
    }
    pub fn is_finite(self) -> bool {
        self.to_vec().is_finite()
    }
}

impl UnitVec3 {
    pub const X: UnitVec3 = UnitVec3(Vec3::X);
    pub const Y: UnitVec3 = UnitVec3(Vec3::Y);
    pub const Z: UnitVec3 = UnitVec3(Vec3::Z);

    /// Caller guarantees `v` has length 1.
    pub const fn new_unchecked(v: Vec3) -> Self {
        UnitVec3(v)
    }
    pub const fn get(self) -> Vec3 {
        self.0
    }
    pub fn dot(self, o: impl Into<Vec3>) -> f64 {
        self.0.dot(o.into())
    }
    pub fn cross(self, o: impl Into<Vec3>) -> Vec3 {
        self.0.cross(o.into())
    }
    /// A unit vector perpendicular to this one, chosen only from its components.
    pub fn any_perpendicular(self) -> UnitVec3 {
        let v = self.0;
        let (ax, ay, az) = (v.x.abs(), v.y.abs(), v.z.abs());
        let helper = if ax <= ay && ax <= az {
            Vec3::X
        } else if ay <= az {
            Vec3::Y
        } else {
            Vec3::Z
        };
        v.cross(helper)
            .normalize()
            .expect("unit vector has a perpendicular")
    }
}

impl From<UnitVec3> for Vec3 {
    fn from(u: UnitVec3) -> Vec3 {
        u.0
    }
}

impl Neg for UnitVec3 {
    type Output = UnitVec3;
    fn neg(self) -> UnitVec3 {
        UnitVec3(-self.0)
    }
}

impl Mul<f64> for UnitVec3 {
    type Output = Vec3;
    fn mul(self, s: f64) -> Vec3 {
        self.0 * s
    }
}

impl Index<usize> for Vec3 {
    type Output = f64;
    fn index(&self, i: usize) -> &f64 {
        match i {
            0 => &self.x,
            1 => &self.y,
            2 => &self.z,
            _ => panic!("Vec3 index {i}"),
        }
    }
}

impl Index<usize> for Point3 {
    type Output = f64;
    fn index(&self, i: usize) -> &f64 {
        match i {
            0 => &self.x,
            1 => &self.y,
            2 => &self.z,
            _ => panic!("Point3 index {i}"),
        }
    }
}

macro_rules! vec_ops {
    ($V:ident, $P:ident, $($f:ident),+) => {
        impl Add for $V { type Output = $V; fn add(self, o: $V) -> $V { $V { $($f: self.$f + o.$f),+ } } }
        impl Sub for $V { type Output = $V; fn sub(self, o: $V) -> $V { $V { $($f: self.$f - o.$f),+ } } }
        impl Neg for $V { type Output = $V; fn neg(self) -> $V { $V { $($f: -self.$f),+ } } }
        impl Mul<f64> for $V { type Output = $V; fn mul(self, s: f64) -> $V { $V { $($f: self.$f * s),+ } } }
        impl Mul<$V> for f64 { type Output = $V; fn mul(self, v: $V) -> $V { v * self } }
        impl Div<f64> for $V { type Output = $V; fn div(self, s: f64) -> $V { $V { $($f: self.$f / s),+ } } }
        impl AddAssign for $V { fn add_assign(&mut self, o: $V) { $(self.$f += o.$f;)+ } }
        impl SubAssign for $V { fn sub_assign(&mut self, o: $V) { $(self.$f -= o.$f;)+ } }
        impl Add<$V> for $P { type Output = $P; fn add(self, o: $V) -> $P { $P { $($f: self.$f + o.$f),+ } } }
        impl Sub<$V> for $P { type Output = $P; fn sub(self, o: $V) -> $P { $P { $($f: self.$f - o.$f),+ } } }
        impl Sub for $P { type Output = $V; fn sub(self, o: $P) -> $V { $V { $($f: self.$f - o.$f),+ } } }
        impl AddAssign<$V> for $P { fn add_assign(&mut self, o: $V) { $(self.$f += o.$f;)+ } }
    };
}
vec_ops!(Vec2, Point2, x, y);
vec_ops!(Vec3, Point3, x, y, z);

impl Add<UnitVec3> for Point3 {
    type Output = Point3;
    fn add(self, o: UnitVec3) -> Point3 {
        self + o.0
    }
}
