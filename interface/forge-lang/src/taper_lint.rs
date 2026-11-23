//! The lint of a tapered extrude whose outline closes before its depth (forge-ops `closes_at`),
//! asked by the extrude builtin where it makes a taper and nowhere else: a part with no taper
//! runs none of it.
use crate::error::{Error, Literal, Warning};
use crate::eval::Dimension;
use crate::syntax::Span;
use crate::values;

/// The lint for a tapered extrude of `profile` by `distance` at `angle` (its arguments' `dims`), if
/// its outline closes before its depth: with the depth's param set to where it closes, or the
/// taper's to the largest angle that reaches the depth, where params drive them.
#[cold]
#[inline(never)]
pub(crate) fn closes(
    profile: &forge_ops::Profile,
    distance: f64,
    angle: f64,
    dims: &[Dimension],
    span: Span,
    id: &str,
) -> Option<Warning> {
    let at = forge_ops::taper::closes_at(profile, distance, angle)?;
    let depth = distance.abs();
    let driven = |what: &str| {
        dims.iter()
            .find(|d| d.what == what)
            .and_then(|d| match &d.num.src {
                values::Src::Driving(key) => Some(((**key).clone(), d.num.v)),
                _ => None,
            })
    };
    // three figures, rounded towards 0 so the walls meet at the depth or past it (a lesser
    // depth, or a shallower taper): the fixed part has no lint
    let down = |x: f64| {
        let p = 10f64.powi(2 - x.abs().log10().floor() as i32);
        (x.abs() * p).floor() / p * x.signum()
    };
    let depth_fix = driven("depth").map(|(key, v)| {
        Box::new((
            key,
            Literal {
                value: down(at) * v.signum(),
                dim: values::Dim::Length,
            },
        ))
    });
    // the walls close at the same offset whatever the angle: at tan a = depth tan a'
    let angle_fix = driven("taper").map(|(key, v)| {
        let (s, c) = forge_math::dmath::sin_cos(v.abs());
        let a = forge_math::dmath::atan2(at * s / c, depth);
        Box::new((
            key,
            Literal {
                value: down(a.to_degrees()) * v.signum(),
                dim: values::Dim::Angle,
            },
        ))
    });
    Some(Warning::Lint(Error::TaperCloses {
        span,
        id: id.into(),
        at,
        depth,
        fixes: [depth_fix, angle_fix],
    }))
}
