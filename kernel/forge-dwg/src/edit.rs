//! Changing a drawing: new handles, entities added to a space or block and erased from it (in
//! each version's way of listing a block's entities), layers, and undo.

use crate::bits::{R, bad};
use crate::file::Drawing;
use crate::header::{Header, Val};
use crate::io::{Color, Href};
use crate::object::{EntCommon, Object};
use crate::str::Str;
use crate::types::{Data, Layer};
use crate::version::Version;
use std::collections::BTreeMap;

/// What a change replaced: objects as they were (None: the object didn't exist), and the header.
#[derive(Clone, Debug, Default)]
pub struct Change {
    pub objects: BTreeMap<u32, Option<Object>>,
    pub header: Option<Header>,
}

/// Undo and redo, a change at a time.
#[derive(Clone, Debug, Default)]
pub struct History {
    pub undo: Vec<Change>,
    pub redo: Vec<Change>,
    open: Option<Change>,
}

impl History {
    /// Starts a change: everything saved until `end` undoes together.
    pub fn begin(&mut self) {
        self.open.get_or_insert_with(Change::default);
    }
    pub fn end(&mut self) {
        if let Some(c) = self.open.take()
            && (!c.objects.is_empty() || c.header.is_some())
        {
            self.undo.push(c);
            self.redo.clear();
        }
    }
    /// Keeps an object's state before it changes (once a change).
    pub fn save(&mut self, d: &Drawing, h: u32) {
        let c = self.open.get_or_insert_with(Change::default);
        c.objects.entry(h).or_insert_with(|| d.objects.get(&h).cloned());
    }
    pub fn save_header(&mut self, d: &Drawing) {
        let c = self.open.get_or_insert_with(Change::default);
        if c.header.is_none() {
            c.header = Some(d.header.clone());
        }
    }
    fn apply(d: &mut Drawing, c: Change) -> Change {
        let mut back = Change::default();
        for (h, o) in c.objects {
            back.objects.insert(h, d.objects.get(&h).cloned());
            match o {
                Some(o) => {
                    d.objects.insert(h, o);
                }
                None => {
                    d.objects.remove(&h);
                }
            }
        }
        if let Some(hd) = c.header {
            back.header = Some(std::mem::replace(&mut d.header, hd));
        }
        back
    }
    pub fn undo(&mut self, d: &mut Drawing) -> bool {
        self.end();
        match self.undo.pop() {
            Some(c) => {
                let back = History::apply(d, c);
                self.redo.push(back);
                true
            }
            None => false,
        }
    }
    pub fn redo(&mut self, d: &mut Drawing) -> bool {
        self.end();
        match self.redo.pop() {
            Some(c) => {
                let back = History::apply(d, c);
                self.undo.push(back);
                true
            }
            None => false,
        }
    }
}

impl Drawing {
    /// A new handle: the seed, which moves on.
    pub fn new_handle(&mut self, hist: &mut History) -> u32 {
        hist.save_header(self);
        let h = self.handseed().max(self.objects.keys().max().map_or(1, |m| m + 1));
        if let Some(Val::H(x)) = self.header.get_mut("HANDSEED") {
            x.handle = h + 1;
        }
        h
    }
    pub fn model_space(&self) -> u32 {
        self.header.handle("BLOCK_RECORD_MODEL_SPACE").unwrap_or(0)
    }
    pub fn paper_space(&self) -> u32 {
        self.header.handle("BLOCK_RECORD_PAPER_SPACE").unwrap_or(0)
    }
    /// A layer by name (case-insensitive, as AutoCAD compares them).
    pub fn layer(&self, name: &str) -> Option<u32> {
        self.objects.iter().find_map(|(h, o)| match &o.data {
            Data::Layer(l) if l.entry.name.text().eq_ignore_ascii_case(name) => Some(*h),
            _ => None,
        })
    }
    /// The type number for an object's data: its fixed number, or its class's (`dxf`).
    pub fn type_code(&self, data: &Data, dxf: &str) -> R<u16> {
        if let Some(c) = data.fixed_code() {
            return Ok(c);
        }
        match self.classes.iter().position(|c| c.dxf.text() == dxf) {
            Some(k) => Ok(500 + k as u16),
            None => bad(format!("no {dxf} class")),
        }
    }
    fn wide(&self) -> bool {
        self.ver() >= Version::R2007
    }
    /// A string as this drawing stores them.
    pub fn s(&self, text: &str) -> Str {
        Str::new(text, self.wide())
    }

    /// Adds an entity at the end of `block` (a block record: model space, a layout's paper space or
    /// a block definition) on `layer`, colour bylayer. Its handle.
    pub fn add_entity(&mut self, hist: &mut History, block: u32, layer: u32, data: Data) -> R<u32> {
        let ver = self.ver();
        let type_code = self.type_code(&data, "")?;
        let h = self.new_handle(hist);
        let entmode = if block == self.model_space() {
            2
        } else if block == self.paper_space() {
            1
        } else {
            0
        };
        let mut ent = EntCommon {
            entmode,
            layer: Href::new(5, layer),
            color: Color::index(256),
            ..EntCommon::default()
        };
        hist.save(self, block);
        let Some(Data::BlockHeader(bh)) = self.objects.get_mut(&block).map(|o| &mut o.data) else {
            return bad(format!("{block:#x} is not a block record"));
        };
        if ver >= Version::R2004 {
            bh.owned.push(Href::new(3, h));
        } else {
            // R2000: a chain from the block's first entity to its last
            ent.nolinks = false;
            let last = bh.last.handle;
            ent.prev = Href::new(4, last);
            ent.next = Href::new(4, 0);
            if last == 0 {
                bh.first = Href::new(4, h);
            }
            bh.last = Href::new(4, h);
        }
        if ver < Version::R2004 && ent.prev.handle != 0 {
            let last = ent.prev.handle;
            self.set_link(hist, last, None, Some(h));
        }
        if let Some(o) = self.objects.get_mut(&block) {
            o.raw = None;
        }
        let o = Object {
            handle: h,
            type_code,
            eed: vec![],
            owner: Href::new(4, block),
            reactors: vec![],
            xdic: if ver < Version::R2004 { Some(Href::new(3, 0)) } else { None },
            ds_binary: false,
            ent: Some(ent),
            data,
            trailing: vec![],
            raw: None,
        };
        hist.save(self, h);
        self.objects.insert(h, o);
        Ok(h)
    }

    /// R2000: an entity's neighbours in its block, the implied ones (handle -1, +1) when it stores
    /// none.
    pub fn links(&self, h: u32) -> (u32, u32) {
        match self.objects.get(&h).and_then(|o| o.ent.as_ref()) {
            Some(e) if e.nolinks => (h - 1, h + 1),
            Some(e) => (e.prev.handle, e.next.handle),
            None => (0, 0),
        }
    }

    /// R2000: sets one of an entity's links, storing both explicitly from now on.
    fn set_link(&mut self, hist: &mut History, h: u32, prev: Option<u32>, next: Option<u32>) {
        let (p0, n0) = self.links(h);
        hist.save(self, h);
        if let Some(o) = self.objects.get_mut(&h) {
            o.raw = None;
            if let Some(e) = o.ent.as_mut() {
                e.nolinks = false;
                e.prev = Href::new(4, prev.unwrap_or(p0));
                e.next = Href::new(4, next.unwrap_or(n0));
            }
        }
    }

    /// The block record an entity is in.
    pub fn block_of(&self, h: u32) -> Option<u32> {
        let o = self.objects.get(&h)?;
        match o.ent.as_ref()?.entmode {
            2 => Some(self.model_space()),
            1 => Some(self.paper_space()),
            _ => Some(o.owner.handle),
        }
    }

    /// Erases an entity, with what it owns (attributes, vertices, its SEQEND).
    pub fn erase(&mut self, hist: &mut History, h: u32) -> R<()> {
        let Some(block) = self.block_of(h) else {
            return bad(format!("{h:#x} is not an entity"));
        };
        let ver = self.ver();
        hist.save(self, block);
        let (prev, next) = if ver < Version::R2004 { self.links(h) } else { (0, 0) };
        // the block's ends have no neighbour on that side
        let (prev, next) = match self.objects.get(&block).map(|o| &o.data) {
            Some(Data::BlockHeader(bh)) => (
                if bh.first.handle == h { 0 } else { prev },
                if bh.last.handle == h { 0 } else { next },
            ),
            _ => (prev, next),
        };
        if let Some(Data::BlockHeader(bh)) = self.objects.get_mut(&block).map(|o| &mut o.data) {
            if ver >= Version::R2004 {
                bh.owned.retain(|x| x.handle != h);
            } else {
                if bh.first.handle == h {
                    bh.first = Href::new(4, next);
                }
                if bh.last.handle == h {
                    bh.last = Href::new(4, prev);
                }
            }
        }
        if let Some(o) = self.objects.get_mut(&block) {
            o.raw = None;
        }
        if ver < Version::R2004 {
            if prev != 0 {
                self.set_link(hist, prev, None, Some(next));
            }
            if next != 0 {
                self.set_link(hist, next, Some(prev), None);
            }
        }
        for sub in self.owned_by(h) {
            hist.save(self, sub);
            self.objects.remove(&sub);
        }
        hist.save(self, h);
        self.objects.remove(&h);
        Ok(())
    }

    /// The sub-entities an entity owns: an insert's attributes, a polyline's vertices, and their
    /// SEQEND.
    pub fn owned_by(&self, h: u32) -> Vec<u32> {
        let Some(o) = self.objects.get(&h) else { return vec![] };
        let owned = match &o.data {
            Data::Insert(i) if i.has_attribs => Some(&i.attribs),
            Data::MInsert(i) if i.insert.has_attribs => Some(&i.insert.attribs),
            Data::Polyline2d(p) => Some(&p.vertices),
            Data::Polyline3d(p) => Some(&p.vertices),
            Data::PolylinePface(p) => Some(&p.vertices),
            Data::PolylineMesh(p) => Some(&p.vertices),
            _ => None,
        };
        let Some(ow) = owned else { return vec![] };
        let mut out: Vec<u32> = ow.items.iter().map(|x| x.handle).collect();
        if out.is_empty() && ow.first.handle != 0 {
            // R2000: the chain from first to last
            let mut x = ow.first.handle;
            while x != 0 && !out.contains(&x) && out.len() < 1_000_000 {
                out.push(x);
                if x == ow.last.handle {
                    break;
                }
                x = self.objects.get(&x).and_then(|o| o.ent.as_ref()).map_or(0, |e| e.next.handle);
            }
        }
        if ow.seqend.handle != 0 {
            out.push(ow.seqend.handle);
        }
        out
    }

    /// Adds a layer (continuous, colour `aci`), or returns the one with that name.
    pub fn add_layer(&mut self, hist: &mut History, name: &str, aci: i16) -> R<u32> {
        if let Some(h) = self.layer(name) {
            return Ok(h);
        }
        let Some(ctrl) = self.header.handle("LAYER_CONTROL") else {
            return bad("no layer table");
        };
        let template = self
            .objects
            .values()
            .find_map(|o| match &o.data {
                Data::Layer(l) => Some((l.clone(), o.type_code)),
                _ => None,
            })
            .ok_or(crate::bits::Bad("no layer to copy".into()))?;
        let h = self.new_handle(hist);
        let mut l: Layer = template.0;
        l.entry.name = self.s(name);
        l.entry.xref_index = 0;
        l.entry.xdep = false;
        l.color = Color { index: aci, ..l.color.clone() };
        if let Some(rgb) = &mut l.color.rgb {
            *rgb = 0xC300_0000 | aci as u32;
        }
        l.flags = 0x10 | (31 << 5); // plotted, default lineweight
        l.ltype = Href::new(5, self.header.handle("LTYPE_CONTINUOUS").unwrap_or(0));
        hist.save(self, ctrl);
        if let Some(o) = self.objects.get_mut(&ctrl) {
            o.raw = None;
            if let Data::LayerControl(c) = &mut o.data {
                c.0.entries.push(Href::new(2, h));
            }
        }
        let ver = self.ver();
        let o = Object {
            handle: h,
            type_code: template.1,
            eed: vec![],
            owner: Href::new(4, ctrl),
            reactors: vec![],
            xdic: if ver < Version::R2004 { Some(Href::new(3, 0)) } else { None },
            ds_binary: false,
            ent: None,
            data: Data::Layer(l),
            trailing: vec![],
            raw: None,
        };
        hist.save(self, h);
        self.objects.insert(h, o);
        Ok(h)
    }

    /// Changes an object through `f`, keeping it for undo and marking it edited.
    pub fn modify(&mut self, hist: &mut History, h: u32, f: impl FnOnce(&mut Object)) -> R<()> {
        hist.save(self, h);
        match self.objects.get_mut(&h) {
            Some(o) => {
                f(o);
                o.raw = None;
                Ok(())
            }
            None => bad(format!("no object {h:#x}")),
        }
    }
}
