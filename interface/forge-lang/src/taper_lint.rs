//! The lint of a tapered extrude whose outline closes before its depth (forge-ops `closes_at`):
//! one pass over the built features after the run, out of eval.rs, which builds every feature,
//! as only a taper asks for it.
use crate::error::{Error, Literal, Warning};
use crate::eval::{Dimension, Eval, Status};
use crate::syntax::Span;
use crate::values;

/// The run's lints for its tapers that close before their depth.
pub(crate) fn lint(ev: &mut Eval) {
    let tapered = |f: &forge_model::Feature| {
        matches!(
            f,
            forge_model::Feature::Extrude {
                extent: forge_model::Extent::Tapered { .. },
                ..
            }
        )
    };
    let found: Vec<Warning> = ev
        .feats
        .iter()
        .filter(|f| f.status == Status::Ok && ev.b.features.get(f.idx).is_some_and(tapered))
        .filter_map(|f| closes(&ev.b.features[f.idx], &f.dims, f.span, &f.id))
        .collect();
    ev.warnings.extend(found);
}

/// The lint for `feature` (built, its arguments' `dims`), if its outline closes before its depth:
/// with the depth's param set to where it closes, or the taper's to the largest angle that
/// reaches the depth, where params drive them.
#[cold]
#[inline(never)]
fn closes(feature: &forge_model::Feature, dims: &[Dimension], span: Span, id: &str) -> Option<Warning> {
    let forge_model::Feature::Extrude {
        profile,
        extent: forge_model::Extent::Tapered { distance, angle },
        ..
    } = feature
    else {
        return None;
    };
    let at = forge_ops::taper::closes_at(profile, *distance, *angle)?;
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
        depth_fix,
        angle_fix,
    }))
}
