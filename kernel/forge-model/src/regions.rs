//! Several regions of one sketch extruded alike in one feature (a sketch text's glyphs, say):
//! each region its own extrude, every one combined with the part the same way.
//!
//! Region `k`'s solid is named as an extrude whose feature id is `region_id(id, key)`, its key
//! the least segment id of its outside loop: segment ids are the sketch entities' stable names,
//! so a region keeps its faces' names while regions elsewhere in the list come and go.

use crate::extent::{self, Resolved};
use crate::{Combine, Extent, Feature, History, RegenError, Tool};
use forge_bool::{Op, Shape};
use forge_ops::Profile;
use forge_topo::{Body, Name};

#[derive(Clone, Debug, PartialEq)]
pub struct Regions {
    pub profiles: Vec<Profile>,
    pub extent: Extent,
    pub op: Combine,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegionsError {
    /// No region, or one with no outside loop.
    Empty,
    /// An extent that reaches a face of the part (To, ToNext): each region would end on its own.
    Extent,
    /// Two regions with the same key: one outside loop's least segment id shared.
    SameKey,
}

/// What names region `key`'s solid of feature `id`: the id its extrude is built with.
pub fn region_id(id: u64, key: u64) -> u64 {
    Name::feature(id).with_str("region").with(key).0
}

/// A region's key: the least segment id of its outside loop.
pub fn key(p: &Profile) -> Option<u64> {
    p.loops.first()?.segments.iter().map(|s| s.1).min()
}

/// A region's plain extrude: what the feature builds for it.
pub fn region(r: &Regions, profile: &Profile) -> Feature {
    Feature::Extrude {
        profile: profile.clone(),
        extent: r.extent,
        op: r.op,
    }
}

/// The solids feature `i` (Regions) combines with the part: one per region, in order; a new
/// body's joined into one.
pub(crate) fn tools(features: History<'_>, i: usize, build: bool) -> Result<Vec<Tool>, RegenError> {
    let Feature::Regions(r) = &features[i] else {
        unreachable!("only regions")
    };
    let e = |error| RegenError::Regions { feature: i, error };
    if matches!(r.extent, Extent::To { .. } | Extent::ToNext { .. }) {
        return Err(e(RegionsError::Extent));
    }
    let keys: Vec<u64> = r
        .profiles
        .iter()
        .map(key)
        .collect::<Option<_>>()
        .ok_or(e(RegionsError::Empty))?;
    if keys.is_empty() {
        return Err(e(RegionsError::Empty));
    }
    let mut sorted = keys.clone();
    sorted.sort_unstable();
    if sorted.windows(2).any(|w| w[0] == w[1]) {
        return Err(e(RegionsError::SameKey));
    }
    let id = features.id(i);
    let mut out = Vec::with_capacity(keys.len());
    for (p, &key) in r.profiles.iter().zip(&keys) {
        let f = region(r, p);
        // (a plain extent comes to a plain extrude: To and ToNext are refused above)
        let Resolved::Plain(plain, _) = extent::resolve_as(features, i, &f)? else {
            return Err(e(RegionsError::Extent));
        };
        out.push(crate::tool(i, region_id(id, key), &plain, build)?);
    }
    // a new body: the regions' solids joined into it, one tool (as a part's first feature takes)
    if r.op == Combine::New && out.len() > 1 {
        let shape = Shape::Csg {
            first: Box::new(out[0].2.clone()),
            rest: out[1..].iter().map(|t| (Op::Union, t.2.clone())).collect(),
        };
        let body = if build {
            let bodies: Vec<&Body> = out.iter().map(|t| &t.0).collect();
            let ops = vec![Op::Union; bodies.len() - 1];
            forge_bool::nary(&bodies, &ops, 1).map_err(|error| RegenError::Boolean { feature: i, error })?
        } else {
            Body::new()
        };
        return Ok(vec![(body, Combine::New, shape)]);
    }
    Ok(out)
}
