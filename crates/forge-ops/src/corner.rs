//! Corner patches where several blends meet. A stand-in until the corner
//! solver lands: it handles nothing, so fillet falls back to its own ball.

use crate::fillet::{CornerInput, CornerPatch, FaceTrim, HoleSide};

/// Where each blend stops (spine parameters, in `c.blends` order), or None to leave the corner.
pub fn setbacks(_c: &CornerInput) -> Option<Vec<f64>> {
    None
}

/// Each face's trim at the corner for those setbacks.
pub fn face_curves(_c: &CornerInput, _setbacks: &[f64]) -> Result<Vec<FaceTrim>, String> {
    Err("no corner solver".to_string())
}

/// The faces closing the hole, or None when this corner is not handled.
pub fn patch(_c: &CornerInput, _hole: &[HoleSide]) -> Option<Result<CornerPatch, String>> {
    None
}
