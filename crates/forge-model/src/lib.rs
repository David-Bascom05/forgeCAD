//! A part as a history of features, and regenerating it into a solid.

pub mod adversarial;
pub mod suite;
pub mod torture;

use forge_bool::{BoolError, Op, boolean_into};
use forge_math::{Frame, Point2, Vec2, dmath};
use forge_ops::{Axis2, ExtrudeError, Profile, ProfileLoop, RevolveError, Segment, extrude, revolve};
use forge_topo::Body;

/// One step of a part's history.
#[derive(Clone, Debug, PartialEq)]
pub enum Feature {
    /// Extrude a profile into a new solid, or add it to / cut it from the part.
    Extrude {
        profile: Profile,
        distance: f64,
        op: Combine,
    },
    /// Revolve a profile about an axis in its plane.
    Revolve {
        profile: Profile,
        axis: Axis2,
        angle: f64,
        op: Combine,
    },
    /// A ball (a shorthand for revolving a half disc).
    Ball { frame: Frame, radius: f64, op: Combine },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Combine {
    /// Start the part (only valid as the first feature).
    New,
    Add,
    Cut,
    Keep,
}

#[derive(Clone, Debug, PartialEq)]
pub enum RegenError {
    Extrude {
        feature: usize,
        error: ExtrudeError,
    },
    Revolve {
        feature: usize,
        error: RevolveError,
    },
    Boolean {
        feature: usize,
        error: BoolError,
    },
    /// A feature that combines with the part came before any part existed.
    NothingToCombine {
        feature: usize,
    },
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Part {
    pub features: Vec<Feature>,
}

impl Part {
    /// Rebuilds the solid from the first feature. Feature ids are their
    /// positions, so entity names stay stable when nothing upstream changes.
    pub fn regenerate(&self) -> Result<Body, RegenError> {
        let mut body: Option<Body> = None;
        for (i, f) in self.features.iter().enumerate() {
            let id = i as u64 + 1;
            let (tool, op) = match f {
                Feature::Extrude {
                    profile,
                    distance,
                    op,
                } => (
                    extrude(profile, *distance, id)
                        .map_err(|error| RegenError::Extrude { feature: i, error })?,
                    *op,
                ),
                Feature::Revolve {
                    profile,
                    axis,
                    angle,
                    op,
                } => (
                    revolve(profile, *axis, *angle, id)
                        .map_err(|error| RegenError::Revolve { feature: i, error })?,
                    *op,
                ),
                Feature::Ball { frame, radius, op } => {
                    let q = dmath::FRAC_PI_2;
                    let half = ProfileLoop::new(vec![
                        (
                            Segment::Arc {
                                center: Point2::ORIGIN,
                                radius: *radius,
                                start: -q,
                                sweep: 2.0 * q,
                            },
                            0,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(0.0, *radius),
                                b: Point2::new(0.0, -*radius),
                            },
                            1,
                        ),
                    ]);
                    let profile = Profile {
                        plane: *frame,
                        loops: vec![half],
                    };
                    let axis = Axis2 {
                        origin: Point2::ORIGIN,
                        dir: Vec2::Y,
                    };
                    (
                        revolve(&profile, axis, dmath::TAU, id)
                            .map_err(|error| RegenError::Revolve { feature: i, error })?,
                        *op,
                    )
                }
            };
            body = Some(match (body, op) {
                (_, Combine::New) => tool,
                (None, _) => return Err(RegenError::NothingToCombine { feature: i }),
                (Some(mut b), op) => {
                    let op = match op {
                        Combine::Add => Op::Union,
                        Combine::Cut => Op::Subtract,
                        _ => Op::Intersect,
                    };
                    // in place: only the faces near the tool change
                    boolean_into(&mut b, &tool, op, id + 1_000_000)
                        .map_err(|error| RegenError::Boolean { feature: i, error })?;
                    // tombstones are cheap until they outnumber the living
                    if b.dead > b.faces.len() / 2 {
                        b.compact()
                    } else {
                        b
                    }
                }
            });
        }
        body.map(|b| b.compact())
            .ok_or(RegenError::NothingToCombine { feature: 0 })
    }
}
