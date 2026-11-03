//! What each face and edge of a built sheet is, by role: the skins of each wall and bend zone,
//! the side faces and edges each trim made. A host resolves anchors through these, never by
//! re-deriving names, and the names are the same folded and flat.
use crate::error::{Strip, TrimRef};
use forge_topo::Name;

/// Which skin: the reference skin (the face the walls' outlines are drawn on) or the back one,
/// a thickness behind it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Skin {
    Reference,
    Back,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Roles {
    /// Each strip's skin faces (a strip cut in two has two of each).
    pub skins: Vec<(Strip, Skin, Name)>,
    /// The side faces carrying a piece of each trim edge. Side faces merge across tangent lines,
    /// so one face may carry pieces of several trims.
    pub sides: Vec<(TrimRef, Name)>,
    /// The edges each trim edge made on each skin (pieces where a trim was cut).
    pub edges: Vec<(TrimRef, Skin, Name)>,
}

impl Roles {
    pub fn skin(&self, strip: Strip, skin: Skin) -> Vec<Name> {
        self.skins
            .iter()
            .filter(|s| s.0 == strip && s.1 == skin)
            .map(|s| s.2)
            .collect()
    }

    pub fn side(&self, trim: TrimRef) -> Vec<Name> {
        self.sides.iter().filter(|s| s.0 == trim).map(|s| s.1).collect()
    }

    pub fn edge(&self, trim: TrimRef, skin: Skin) -> Vec<Name> {
        self.edges
            .iter()
            .filter(|s| s.0 == trim && s.1 == skin)
            .map(|s| s.2)
            .collect()
    }
}
