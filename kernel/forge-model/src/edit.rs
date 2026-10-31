//! Features that work on the part's faces as they stand: ribs, offset and thickened faces, and
//! direct edits (delete, replace, move).

use crate::rib::{Rib, RibError, rib};
use crate::{Combine, Feature, History, RegenError, Target, op_of};
use forge_bool::{BoolError, boolean};
use forge_math::{Frame, Point3, Vec3};
use forge_ops::{
    DirectError, Rounded, ThickenError, delete_faces, move_faces, offset_faces, replace_face, thicken,
};
use forge_topo::{Body, Name};

#[derive(Clone, Debug, PartialEq)]
pub enum Edit {
    Rib(Box<Rib>),
    /// Move faces `distance` along their outward normals, the neighbours re-intersected.
    Offset {
        faces: Vec<Name>,
        distance: f64,
    },
    /// A solid `thickness` thick on faces, outward from them (`Pos`), inward, or both ways,
    /// combined with the part (`New` replaces it).
    Thicken {
        faces: Vec<Name>,
        thickness: f64,
        side: Side,
        op: Combine,
    },
    /// Remove faces, the gap closed by their neighbours.
    Delete {
        faces: Vec<Name>,
    },
    /// Put a face on a work plane or on the surface of an earlier feature's face.
    Replace {
        face: Name,
        with: Target,
    },
    Move {
        faces: Vec<Name>,
        motion: Motion,
    },
}

/// Which way a thickness goes from the faces or path it is measured from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Both,
    Pos,
    Neg,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Motion {
    Translate(Vec3),
    /// By `angle` about the line through `origin` along `axis`.
    Rotate {
        origin: Point3,
        axis: Vec3,
        angle: f64,
    },
}

impl Motion {
    /// The motion as a frame (its local coordinates to world), or None for a zero axis.
    pub fn frame(&self) -> Option<Frame> {
        match *self {
            Motion::Translate(d) => Some(Frame::WORLD.with_origin(Point3::ORIGIN + d)),
            Motion::Rotate { origin, axis, angle } => {
                let a = Frame::from_z(origin, axis.normalize()?);
                Some(a.rotated_about_z(angle).compose(&a.inverse()))
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum EditError {
    /// A zero rotation axis, or a size that is not finite.
    BadSize,
    /// Nothing would be left (a thickened slab kept where it only touches the part).
    Empty,
    Rib(RibError),
    Direct(DirectError),
    Thicken(ThickenError),
    Boolean(BoolError),
}

/// Edit feature `i` applied to `body`.
pub(crate) fn apply(features: History<'_>, i: usize, edit: &Edit, body: &Body) -> Result<Body, RegenError> {
    let id = features.id(i);
    let err = |error| RegenError::Edit { feature: i, error };
    let direct = |e| err(EditError::Direct(e));
    let sizes = match *edit {
        Edit::Offset { distance, .. } => vec![distance],
        Edit::Thicken { thickness, .. } => vec![thickness],
        Edit::Move {
            motion: Motion::Translate(d),
            ..
        } => vec![d.x, d.y, d.z],
        Edit::Move {
            motion: Motion::Rotate { origin, axis, angle },
            ..
        } => vec![origin.x, origin.y, origin.z, axis.x, axis.y, axis.z, angle],
        _ => vec![],
    };
    if !sizes.iter().all(|x| x.is_finite()) {
        return Err(err(EditError::BadSize));
    }
    match edit {
        Edit::Rib(r) => rib(body, r, id).map_err(|e| err(EditError::Rib(e))),
        Edit::Offset { faces, distance } => {
            offset_faces(body, faces, *distance, &rounded(features, i)).map_err(direct)
        }
        Edit::Thicken {
            faces,
            thickness,
            side,
            op,
        } => {
            let t = *thickness;
            let (lo, hi) = match side {
                Side::Both => (-t / 2.0, t / 2.0),
                Side::Pos => (0.0, t),
                Side::Neg => (-t, 0.0),
            };
            let slab = thicken(body, faces, lo, hi, id).map_err(|e| err(EditError::Thicken(e)))?;
            if *op == Combine::New {
                return Ok(slab);
            }
            let b =
                boolean(body, &slab, op_of(*op), id + 1_000_000).map_err(|e| err(EditError::Boolean(e)))?;
            if b.faces.is_empty() {
                return Err(err(EditError::Empty));
            }
            Ok(b)
        }
        Edit::Delete { faces } => delete_faces(body, faces).map_err(direct),
        Edit::Replace { face, with } => {
            let s = crate::extent::surface(features, i, with)?;
            replace_face(body, *face, &s, &rounded(features, i)).map_err(direct)
        }
        Edit::Move { faces, motion } => {
            let m = motion.frame().ok_or(err(EditError::BadSize))?;
            move_faces(body, faces, &m, &rounded(features, i)).map_err(direct)
        }
    }
}

/// The blends the fillets before feature `i` made, by the names they gave their faces.
fn rounded(features: History<'_>, i: usize) -> Vec<Rounded> {
    let mut out = Vec::new();
    for (j, f) in features[..i].iter().enumerate() {
        if let Feature::Fillet { edges, .. } = f {
            let feature = features.id(j);
            for (edge, radius) in edges {
                out.push(Rounded {
                    face: Name::feature(feature).with_str("fillet").with_name(*edge),
                    edge: *edge,
                    radius: radius.clone(),
                    feature,
                });
            }
        }
    }
    out
}
