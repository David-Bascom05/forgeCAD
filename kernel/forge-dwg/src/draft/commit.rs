//! Putting results into a drawing through the editing core: an entity replaced by what an
//! operation made of it (in place when the type holds it, else erased and new entities added
//! with its layer, colour, line type and the rest), and a space's curves and snap candidates.

use super::entity::{Flat, curve_of};
use super::geom::V2;
use super::snap::{SnapGeom, SnapItem};
use crate::bits::{R, bad};
use crate::edit::History;
use crate::file::Drawing;
use crate::types::Data;

/// Adds an entity to the block `like` is in, with `like`'s properties (layer, colour, line
/// type and scale, lineweight, plot style, material, visibility). Its handle.
pub fn add_like(d: &mut Drawing, hist: &mut History, like: u32, data: Data) -> R<u32> {
    let Some(block) = d.block_of(like) else {
        return bad(format!("{like:#x} is not an entity"));
    };
    let Some(src) = d.objects.get(&like).and_then(|o| o.ent.clone()) else {
        return bad(format!("{like:#x} is not an entity"));
    };
    let h = d.add_entity(hist, block, src.layer.handle, data)?;
    d.modify(hist, h, |o| {
        if let Some(e) = o.ent.as_mut() {
            let mut c = src.clone();
            c.graphic = None;
            c.entmode = e.entmode;
            c.nolinks = e.nolinks;
            c.prev = e.prev;
            c.next = e.next;
            *e = c;
        }
    })?;
    Ok(h)
}

/// Replaces entity `h` by `new` (what TRIM, BREAK, OFFSET, .. made of it): the first piece in
/// place when it is the entity's type (the handle kept), the rest added beside it like it;
/// otherwise the entity is erased and all are added. No pieces erases it. The handles, in
/// `new`'s order.
pub fn replace(d: &mut Drawing, hist: &mut History, h: u32, new: Vec<Data>) -> R<Vec<u32>> {
    let Some(o) = d.objects.get(&h) else {
        return bad(format!("no object {h:#x}"));
    };
    let same = new
        .first()
        .is_some_and(|n| std::mem::discriminant(n) == std::mem::discriminant(&o.data));
    let mut out = vec![];
    let mut rest = new.into_iter();
    if same {
        let first = rest.next().unwrap_or(Data::Raw(0));
        d.modify(hist, h, |o| o.data = first)?;
        out.push(h);
        for n in rest {
            out.push(add_like(d, hist, h, n)?);
        }
        return Ok(out);
    }
    for n in rest {
        out.push(add_like(d, hist, h, n)?);
    }
    d.erase(hist, h)?;
    Ok(out)
}

/// The entities directly in a block record (model space, a layout, a block definition).
pub fn entities_in(d: &Drawing, block: u32) -> Vec<u32> {
    d.objects
        .iter()
        .filter(|(h, o)| {
            o.ent.is_some()
                && d.block_of(**h) == Some(block)
                && !matches!(
                    o.data,
                    Data::Vertex2d(_)
                        | Data::Vertex3d(_)
                        | Data::VertexMesh(_)
                        | Data::VertexPface(_)
                        | Data::VertexPfaceFace(_)
                        | Data::SeqEnd(_)
                        | Data::Attrib(_)
                        | Data::Block(_)
                        | Data::EndBlk(_)
                )
        })
        .map(|(h, _)| *h)
        .collect()
}

/// The editable curves in a block record.
pub fn curves_in(d: &Drawing, block: u32) -> Vec<(u32, Flat)> {
    entities_in(d, block)
        .into_iter()
        .filter_map(|h| curve_of(d, h).ok().map(|f| (h, f)))
        .collect()
}

/// Snap candidates for entities (the caller's spatial query picks which): curves, POINTs as
/// nodes, text, block references and attributes by their insertion points.
pub fn snap_items(d: &Drawing, handles: &[u32]) -> Vec<SnapItem> {
    let mut out = vec![];
    for &h in handles {
        let Some(o) = d.objects.get(&h) else { continue };
        let geom = match &o.data {
            Data::Point(p) => SnapGeom::Node(V2::from(p.at)),
            Data::Text(t) => SnapGeom::Insertion(V2::from(t.insert)),
            Data::Attrib(a) => SnapGeom::Insertion(V2::from(a.text.insert)),
            Data::MText(t) => SnapGeom::Insertion(V2::from(t.insert)),
            Data::Insert(i) => SnapGeom::Insertion(V2::from(i.at)),
            Data::MInsert(i) => SnapGeom::Insertion(V2::from(i.insert.at)),
            _ => match curve_of(d, h) {
                Ok(f) => SnapGeom::Curve(f.curve),
                Err(_) => continue,
            },
        };
        out.push(SnapItem { handle: h, geom });
    }
    out
}
