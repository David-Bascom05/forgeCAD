//! Putting results into a drawing through the editing core: an entity replaced by what an
//! operation made of it (in place when the type holds it, else erased and new entities added
//! with its layer, colour, line type and the rest), and a space's curves and snap candidates.

use super::entity::{Flat, curve_of, transform_data, write_curve};
use super::geom::Affine;
use crate::io::Href;
use crate::version::Version;
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

/// The block a dimension draws from, when it has one of its own.
pub fn dim_block(d: &Drawing, h: u32) -> Option<u32> {
    let b = match &d.objects.get(&h)?.data {
        Data::DimLinear(x) => x.dim.block.handle,
        Data::DimAligned(x) => x.dim.block.handle,
        Data::DimRadius(x) => x.dim.block.handle,
        Data::DimDiameter(x) => x.dim.block.handle,
        Data::DimOrdinate(x) => x.dim.block.handle,
        Data::DimAng2Ln(x) => x.dim.block.handle,
        Data::DimAng3Pt(x) => x.dim.block.handle,
        _ => return None,
    };
    matches!(d.objects.get(&b).map(|o| &o.data), Some(Data::BlockHeader(_))).then_some(b)
}

/// A block record's entities, in order.
pub fn block_entities(d: &Drawing, block: u32) -> Vec<u32> {
    match d.objects.get(&block).map(|o| &o.data) {
        Some(Data::BlockHeader(bh)) if !bh.owned.is_empty() => bh.owned.iter().map(|x| x.handle).collect(),
        Some(Data::BlockHeader(bh)) if bh.first.handle != 0 => {
            // R2000: the chain from first to last
            let mut out = vec![];
            let mut x = bh.first.handle;
            while x != 0 && out.len() < 10_000_000 {
                out.push(x);
                if x == bh.last.handle {
                    break;
                }
                x = d.links(x).1;
            }
            out
        }
        _ => vec![],
    }
}

fn set_dim_block(data: &mut Data, block: u32) {
    let dc = match data {
        Data::DimLinear(x) => &mut x.dim,
        Data::DimAligned(x) => &mut x.dim,
        Data::DimRadius(x) => &mut x.dim,
        Data::DimDiameter(x) => &mut x.dim,
        Data::DimOrdinate(x) => &mut x.dim,
        Data::DimAng2Ln(x) => &mut x.dim,
        Data::DimAng3Pt(x) => &mut x.dim,
        _ => return,
    };
    dc.block = Href::new(5, block);
}

fn owned_mut(data: &mut Data) -> Option<&mut crate::types::Owned> {
    match data {
        Data::Insert(i) if i.has_attribs => Some(&mut i.attribs),
        Data::MInsert(i) if i.insert.has_attribs => Some(&mut i.insert.attribs),
        Data::Polyline2d(p) => Some(&mut p.vertices),
        Data::Polyline3d(p) => Some(&mut p.vertices),
        Data::PolylinePface(p) => Some(&mut p.vertices),
        Data::PolylineMesh(p) => Some(&mut p.vertices),
        _ => None,
    }
}

/// A copy of entity `h` beside it (COPY, ARRAY, MIRROR keeping the source): with copies of what
/// it owns (attributes, vertices) and of a dimension's block. Its handle.
pub fn copy_entity(d: &mut Drawing, hist: &mut History, h: u32) -> R<u32> {
    let Some(o) = d.objects.get(&h).cloned() else {
        return bad(format!("no object {h:#x}"));
    };
    let subs = d.owned_by(h);
    let mut data = o.data.clone();
    if let Some(b) = dim_block(d, h) {
        let nb = copy_block(d, hist, b)?;
        set_dim_block(&mut data, nb);
    }
    let n = add_like(d, hist, h, data)?;
    if subs.is_empty() {
        return Ok(n);
    }
    let ver = d.ver();
    let mut new_subs = vec![];
    for _ in &subs {
        new_subs.push(d.new_handle(hist));
    }
    let has_seq = subs
        .last()
        .is_some_and(|x| matches!(d.objects.get(x).map(|o| &o.data), Some(Data::SeqEnd(_))));
    let chain: Vec<u32> = if has_seq {
        new_subs[..new_subs.len() - 1].to_vec()
    } else {
        new_subs.clone()
    };
    for (&old, &new) in subs.iter().zip(&new_subs) {
        let Some(mut so) = d.objects.get(&old).cloned() else { continue };
        so.handle = new;
        so.owner = Href::new(4, n);
        so.raw = None;
        if let Some(e) = so.ent.as_mut()
            && ver < Version::R2004
        {
            e.nolinks = false;
            let pos = chain.iter().position(|x| *x == new);
            e.prev = Href::new(4, pos.and_then(|p| p.checked_sub(1)).map_or(0, |p| chain[p]));
            e.next = Href::new(4, pos.and_then(|p| chain.get(p + 1)).copied().unwrap_or(0));
        }
        hist.save(d, new);
        d.objects.insert(new, so);
    }
    d.modify(hist, n, |o| {
        if let Some(ow) = owned_mut(&mut o.data) {
            if ver >= Version::R2004 {
                ow.items = chain.iter().map(|x| Href::new(3, *x)).collect();
            } else {
                ow.first = Href::new(4, chain.first().copied().unwrap_or(0));
                ow.last = Href::new(4, chain.last().copied().unwrap_or(0));
            }
            if has_seq {
                ow.seqend = Href::new(3, new_subs.last().copied().unwrap_or(0));
            }
        }
    })?;
    Ok(n)
}

/// The next free number for an anonymous block of a kind ("*D", "*U").
pub fn next_anonymous(d: &Drawing, kind: &str) -> u32 {
    d.objects
        .values()
        .filter_map(|o| match &o.data {
            Data::Block(x) => x.name.text().strip_prefix(kind).and_then(|k| k.parse::<u32>().ok()),
            _ => None,
        })
        .max()
        .unwrap_or(0)
        + 1
}

/// A copy of a block definition (a dimension's): a new anonymous block with copies of its
/// entities. Its block record.
fn copy_block(d: &mut Drawing, hist: &mut History, b: u32) -> R<u32> {
    let name = match d.objects.get(&b).map(|o| &o.data) {
        Some(Data::BlockHeader(bh)) => bh.entry.name.text(),
        _ => return bad("not a block"),
    };
    let kind: String = name.chars().take(2).collect();
    let n = next_anonymous(d, &kind);
    let nb = d.add_block(hist, &format!("{kind}{n}"), true)?;
    for e in block_entities(d, b) {
        let Some(o) = d.objects.get(&e).cloned() else { continue };
        let Some(ent) = o.ent.clone() else { continue };
        if matches!(o.data, Data::Block(_) | Data::EndBlk(_)) {
            continue;
        }
        let h = d.add_entity(hist, nb, ent.layer.handle, o.data.clone())?;
        set_ent(d, hist, h, ent)?;
    }
    Ok(nb)
}

/// Gives a new entity another's common properties (its own place in its block kept).
fn set_ent(d: &mut Drawing, hist: &mut History, h: u32, ent: crate::object::EntCommon) -> R<()> {
    d.modify(hist, h, |x| {
        if let Some(e) = x.ent.as_mut() {
            let mut c = ent;
            c.graphic = None;
            c.entmode = e.entmode;
            c.nolinks = e.nolinks;
            c.prev = e.prev;
            c.next = e.next;
            *e = c;
        }
    })
}

/// Entity data under a map, or None: what `transform_data` keeps, and curves it can't keep
/// as their type (a circle under a non-uniform scale) as the type that holds them.
pub fn mapped(d: &Drawing, h: u32, m: &Affine, mirrtext: bool) -> Option<Data> {
    let o = d.objects.get(&h)?;
    if let Some(x) = transform_data(&o.data, m, mirrtext) {
        return Some(x);
    }
    let f = curve_of(d, h).ok()?;
    let c = f.curve.transformed(m)?;
    write_curve(&c, &o.data)
}

/// MOVE, ROTATE, SCALE, MIRROR of entity `h` in place: its data, what it owns and a
/// dimension's block. False when the map can't apply to it (left as it was).
pub fn transform(d: &mut Drawing, hist: &mut History, h: u32, m: &Affine, mirrtext: bool) -> R<bool> {
    let Some(new) = mapped(d, h, m, mirrtext) else {
        return Ok(false);
    };
    let mut subs: Vec<(u32, Data)> = vec![];
    for s in d.owned_by(h) {
        let Some(o) = d.objects.get(&s) else { continue };
        if matches!(o.data, Data::SeqEnd(_)) {
            continue;
        }
        match transform_data(&o.data, m, mirrtext) {
            Some(x) => subs.push((s, x)),
            None => return Ok(false),
        }
    }
    if let Some(b) = dim_block(d, h) {
        for e in block_entities(d, b) {
            if let Some(x) = d.objects.get(&e).and_then(|o| transform_data(&o.data, m, false)) {
                subs.push((e, x));
            }
        }
    }
    let same = d
        .objects
        .get(&h)
        .is_some_and(|o| std::mem::discriminant(&o.data) == std::mem::discriminant(&new));
    if !same {
        // a circle become an ellipse: a new entity in its place
        replace(d, hist, h, vec![new])?;
        return Ok(true);
    }
    d.modify(hist, h, |o| o.data = new)?;
    for (s, x) in subs {
        d.modify(hist, s, |o| o.data = x)?;
    }
    Ok(true)
}

/// EXPLODE: a polyline into its lines and arcs, a block reference or a dimension into its
/// block's entities (where the reference puts them; layer 0 and by-block properties take the
/// reference's), and the source erased. The new handles; none when it doesn't explode.
pub fn explode_entity(d: &mut Drawing, hist: &mut History, h: u32) -> R<Vec<u32>> {
    let Some(o) = d.objects.get(&h).cloned() else {
        return Ok(vec![]);
    };
    let Some(space) = d.block_of(h) else { return Ok(vec![]) };
    let mut out = vec![];
    match &o.data {
        Data::LwPolyline(_) | Data::Polyline2d(_) => {
            let Ok(f) = curve_of(d, h) else { return Ok(vec![]) };
            for c in super::entity::explode(&f.curve) {
                if let Some(x) = super::entity::to_data(&c, f.z) {
                    out.push(add_like(d, hist, h, x)?);
                }
            }
        }
        Data::Insert(i) => {
            let up = i.extrusion[0].abs() < 1e-12 && i.extrusion[1].abs() < 1e-12 && i.extrusion[2] > 0.0;
            let base = match d.objects.get(&i.block.handle).map(|o| &o.data) {
                Some(Data::BlockHeader(bh)) if up => V2::from(bh.base),
                _ => return Ok(vec![]),
            };
            let m = Affine::translate(-base)
                .then(&Affine::scale_xy(V2::ZERO, i.scale[0], i.scale[1]))
                .then(&Affine::rotate(V2::ZERO, i.rotation))
                .then(&Affine::translate(V2::from(i.at)));
            out = place_block(d, hist, i.block.handle, space, &m, &o)?;
        }
        _ => match dim_block(d, h) {
            Some(b) => out = place_block(d, hist, b, space, &Affine::IDENTITY, &o)?,
            None => return Ok(vec![]),
        },
    }
    if !out.is_empty() {
        d.erase(hist, h)?;
    }
    Ok(out)
}

/// A block's entities mapped by `m` into `space`; layer "0" and by-block properties take the
/// reference's (`src`).
fn place_block(
    d: &mut Drawing,
    hist: &mut History,
    block: u32,
    space: u32,
    m: &Affine,
    src: &crate::object::Object,
) -> R<Vec<u32>> {
    let layer0 = d.layer("0").unwrap_or(0);
    let src_ent = src.ent.clone().unwrap_or_default();
    let mut out = vec![];
    for e in block_entities(d, block) {
        let Some(o) = d.objects.get(&e) else { continue };
        if matches!(
            o.data,
            Data::Block(_) | Data::EndBlk(_) | Data::AttDef(_) | Data::SeqEnd(_) | Data::Attrib(_)
        ) {
            continue;
        }
        let Some(mut ent) = o.ent.clone() else { continue };
        let Some(data) = mapped(d, e, m, false) else { continue };
        if ent.layer.handle == layer0 {
            ent.layer = src_ent.layer;
        }
        if ent.color.index == 0 {
            ent.color = src_ent.color.clone();
        }
        if ent.lineweight == 30 {
            ent.lineweight = src_ent.lineweight;
        }
        if ent.ltype_flags == 1 {
            ent.ltype_flags = src_ent.ltype_flags;
            ent.ltype = src_ent.ltype;
        }
        let h = d.add_entity(hist, space, ent.layer.handle, data)?;
        set_ent(d, hist, h, ent)?;
        out.push(h);
    }
    Ok(out)
}
