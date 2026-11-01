//! A solid from another system: a body the host read from a file, whole. The kernel reads no
//! files: the host finds what the feature names and hands it over, as for a text's font.

use crate::{Combine, Feature, History, RegenError, Tool};
use forge_bool::Shape;
use forge_topo::{Body, Name};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct Import {
    /// The file, as the part file names it.
    pub file: String,
    pub op: Combine,
    /// What the host read there; None until it has looked.
    pub body: Option<Arc<Body>>,
}

/// Two imports are equal when they name the same file the same way.
impl PartialEq for Import {
    fn eq(&self, o: &Import) -> bool {
        (&self.file, self.op) == (&o.file, o.op)
    }
}

/// The body as the feature's tool, its names its own: the same file twice names nothing twice.
pub(crate) fn tools(features: History<'_>, i: usize) -> Result<Vec<Tool>, RegenError> {
    let Feature::Import(t) = &features[i] else {
        unreachable!("only import")
    };
    let Some(b) = &t.body else {
        return Err(RegenError::Import {
            feature: i,
            file: t.file.clone(),
        });
    };
    let salt = Name::feature(features.id(i)).with_str("import");
    let mut body = (**b).clone();
    body.vertices
        .iter_mut()
        .for_each(|v| v.name = v.name.salted(salt));
    body.edges.iter_mut().for_each(|e| e.name = e.name.salted(salt));
    body.faces.iter_mut().for_each(|f| f.name = f.name.salted(salt));
    let bbox = forge_bool::Index::new(&body).bbox;
    Ok(vec![(body, t.op, Shape::Brep(bbox))])
}
