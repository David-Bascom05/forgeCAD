use crate::rebuild::RebuildError;
use forge_topo::{Body, Name, Problem};

/// How far a chamfer cuts back from its edges.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ChamferSize {
    /// The same distance back along both faces.
    Equal(f64),
    /// `d1` back along `face`, `d2` along the other face of each edge.
    Distances { face: Name, d1: f64, d2: f64 },
    /// `d` back along `face`, the cut leaving it at `angle`.
    DistanceAngle { face: Name, d: f64, angle: f64 },
}

#[derive(Clone, Debug, PartialEq)]
pub enum ChamferError {
    NoSuchEdge {
        edge: Name,
    },
    /// A distance is no more than the tolerance, or the angle is not between 0 and the faces' angle.
    BadSize,
    /// The reference face of a two-sided size does not bound this edge.
    NotOnFace {
        edge: Name,
    },
    Unsupported {
        edge: Name,
        what: &'static str,
    },
    Rebuild(RebuildError),
    Invalid(Vec<Problem>),
}

/// Bevels `edges` all at once.
pub fn chamfer(
    _body: &Body,
    edges: &[Name],
    _size: ChamferSize,
    _feature: u64,
) -> Result<Body, ChamferError> {
    Err(ChamferError::Unsupported {
        edge: edges.first().copied().unwrap_or(Name::NONE),
        what: "not yet",
    })
}
