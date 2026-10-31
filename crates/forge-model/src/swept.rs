//! Sweep, coil and loft features: their settings, and the tools they build.

use crate::{Combine, Feature, RegenError};
use forge_ops::{Axis2, PathSeg, Profile, coil, loft, sweep};
use forge_topo::Body;

/// Sweep a profile along a tangent-continuous path of lines and arcs, square to it.
#[derive(Clone, Debug, PartialEq)]
pub struct Sweep {
    pub profile: Profile,
    pub path: Vec<PathSeg>,
    pub op: Combine,
}

/// Wind a profile about an axis in its plane: `turns` turns rising `pitch` each, widening by the
/// angle `taper` as it rises.
#[derive(Clone, Debug, PartialEq)]
pub struct Coil {
    pub profile: Profile,
    pub axis: Axis2,
    pub pitch: f64,
    pub turns: f64,
    pub taper: f64,
    pub op: Combine,
}

/// Ruled walls through two or more profiles, matched loop by loop and segment by segment.
#[derive(Clone, Debug, PartialEq)]
pub struct Loft {
    pub profiles: Vec<Profile>,
    pub op: Combine,
}

/// How a sweep, coil or loft combines with the part; None for any other feature.
pub(crate) fn op(f: &Feature) -> Option<Combine> {
    match f {
        Feature::Sweep(s) => Some(s.op),
        Feature::Coil(c) => Some(c.op),
        Feature::Loft(l) => Some(l.op),
        _ => None,
    }
}

/// The solid sweep, coil or loft `i` makes.
pub(crate) fn build(i: usize, f: &Feature) -> Result<Body, RegenError> {
    let id = i as u64 + 1;
    match f {
        Feature::Sweep(s) => {
            sweep(&s.profile, &s.path, id).map_err(|error| RegenError::Sweep { feature: i, error })
        }
        Feature::Coil(c) => coil(&c.profile, c.axis, c.pitch, c.turns, c.taper, id)
            .map_err(|error| RegenError::Coil { feature: i, error }),
        Feature::Loft(l) => loft(&l.profiles, id).map_err(|error| RegenError::Loft { feature: i, error }),
        _ => unreachable!("only sweeps, coils and lofts"),
    }
}
