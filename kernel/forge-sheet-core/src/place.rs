//! A rigid motion of the developed plane, `p ↦ o + x p.x + x⊥ p.y` with `x⊥ = (−x.y, x.x)`: the
//! strip tree's development (`dev[b] = dev[a] · ja · shift(BA − 2s) · jb⁻¹`, build.rs), written for
//! Aeneas over the f64 bridge. Every operation is in build.rs `Rigid`'s order, so results are bit
//! for bit the same: `apply` is `(o + x·px) + x⊥·py` componentwise, `vec` is `x·vx + x⊥·vy`.

/// A rigid motion: origin and unit x axis, as plain doubles.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Place {
    pub ox: f64,
    pub oy: f64,
    pub xx: f64,
    pub xy: f64,
}

/// `p ↦ o + x p.x + x⊥ p.y`.
pub fn apply(m: Place, px: f64, py: f64) -> (f64, f64) {
    let ax = m.ox + m.xx * px;
    let ay = m.oy + m.xy * px;
    (ax + -m.xy * py, ay + m.xx * py)
}

/// `v ↦ x v.x + x⊥ v.y`.
pub fn vec(m: Place, vx: f64, vy: f64) -> (f64, f64) {
    (m.xx * vx + -m.xy * vy, m.xy * vx + m.xx * vy)
}

/// The motion's own coordinates of `p`.
pub fn local(m: Place, px: f64, py: f64) -> (f64, f64) {
    let dx = px - m.ox;
    let dy = py - m.oy;
    (dx * m.xx + dy * m.xy, dx * -m.xy + dy * m.xx)
}

/// `self ∘ inner`.
pub fn then(m: Place, inner: Place) -> Place {
    let (ox, oy) = apply(m, inner.ox, inner.oy);
    let (xx, xy) = vec(m, inner.xx, inner.xy);
    Place { ox, oy, xx, xy }
}

/// The inverse motion.
pub fn inverse(m: Place) -> Place {
    let (ox, oy) = local(m, 0.0, 0.0);
    Place {
        ox,
        oy,
        xx: m.xx,
        xy: -m.xy,
    }
}
