//! Sheets (surface bodies, `Body::sheet`): chains of a sketch extruded or revolved into a surface with
//! no material. A sheet is always a body of its own, named by its feature; a feature that makes or
//! changes material never takes one as the body it acts on (`targets` passes over them), and one that
//! names a sheet's faces or edges to change them is `BodyError::OnSheet`.

use crate::{RegenError, Tool};
use forge_math::Frame;
use forge_ops::{Axis2, Segment, extrude_sheet, revolve_sheet};
use forge_topo::Body;

#[derive(Clone, Debug, PartialEq)]
pub struct Sheet {
    /// The sketch plane the chains lie in.
    pub plane: Frame,
    /// Segments end to end, each chain open or closed.
    pub chains: Vec<Vec<(Segment, u64)>>,
    pub how: SheetHow,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SheetHow {
    /// Along the plane's normal from `lo` to `hi` (`lo` < `hi`).
    Extrude { lo: f64, hi: f64 },
    /// About `axis` (in the plane) from `from` through `angle` (positive, at most a turn).
    Revolve { axis: Axis2, from: f64, angle: f64 },
}

/// The sheet, as the one tool its feature gives: a new body.
pub(crate) fn tool(i: usize, id: u64, s: &Sheet, build: bool) -> Result<Tool, RegenError> {
    let body = if build { make(i, id, s)? } else { Body::new() };
    let shape = crate::part_shape(&body);
    Ok((body, crate::Combine::New, shape))
}

fn make(i: usize, id: u64, s: &Sheet) -> Result<Body, RegenError> {
    match s.how {
        SheetHow::Extrude { lo, hi } => {
            let plane = s.plane.with_origin(s.plane.origin + s.plane.z() * lo);
            extrude_sheet(&plane, &s.chains, hi - lo, id)
                .map_err(|error| RegenError::Extrude { feature: i, error })
        }
        SheetHow::Revolve { axis, from, angle } => {
            // starting at `from`: the plane turned that far about the axis first
            let plane = if from == 0.0 {
                s.plane
            } else {
                let o = s.plane.plane_point(axis.origin);
                let d = s.plane.x() * axis.dir.x + s.plane.y() * axis.dir.y;
                match d.normalize() {
                    Some(z) => {
                        let at = Frame::from_z(o, z);
                        at.rotated_about_z(from).compose(&at.inverse()).compose(&s.plane)
                    }
                    None => s.plane,
                }
            };
            revolve_sheet(&plane, &s.chains, axis, angle, id)
                .map_err(|error| RegenError::Revolve { feature: i, error })
        }
    }
}
