//! An object as stored: the frame round it (size, type, handle, extended data, reactors, its
//! extension dictionary), an entity's common data, and its own fields (`Data`). Chapter 20.1-2.

use crate::bits::{BitReader, BitWriter, R, bad, crc16, ms, read_mc, read_ms};
use crate::io::{Color, Href, Io, Reader, Writer};
use crate::str::Str;
use crate::types::Data;
use crate::version::Version;

/// Extended data: an application's items, kept as stored (chapter 28).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Eed {
    pub app: Href,
    pub data: Vec<u8>,
}

/// What every entity has besides its own fields.
#[derive(Clone, Debug, PartialEq)]
pub struct EntCommon {
    /// Proxy graphics, when stored.
    pub graphic: Option<Vec<u8>>,
    /// 0: owner handle stored; 1 paper space; 2 model space.
    pub entmode: u8,
    /// R2000 and before: prev/next are the handles either side (not stored).
    pub nolinks: bool,
    pub prev: Href,
    pub next: Href,
    pub color: Color,
    pub ltype_scale: f64,
    /// 0 bylayer, 1 byblock, 2 continuous, 3 `ltype`.
    pub ltype_flags: u8,
    pub plotstyle_flags: u8,
    pub material_flags: u8,
    pub shadow_flags: u8,
    /// R2010+: full, face and edge visual styles present.
    pub visual_styles: [bool; 3],
    pub visual_style: [Href; 3],
    pub invisible: i16,
    /// Lineweight index (29 bylayer, 30 byblock, 31 default).
    pub lineweight: u8,
    pub layer: Href,
    pub ltype: Href,
    pub plotstyle: Href,
    pub material: Href,
}

impl Default for EntCommon {
    fn default() -> EntCommon {
        EntCommon {
            graphic: None,
            entmode: 2,
            nolinks: true,
            prev: Href::NULL_SOFT,
            next: Href::NULL_SOFT,
            color: Color::index(256),
            ltype_scale: 1.0,
            ltype_flags: 0,
            plotstyle_flags: 0,
            material_flags: 0,
            shadow_flags: 0,
            visual_styles: [false; 3],
            visual_style: [Href::NULL_HARD; 3],
            invisible: 0,
            lineweight: 29,
            layer: Href::NULL_HARD,
            ltype: Href::NULL_HARD,
            plotstyle: Href::NULL_HARD,
            material: Href::NULL_HARD,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Object {
    pub handle: u32,
    /// The type number as stored (classes' are 500 + their index).
    pub type_code: u16,
    pub eed: Vec<Eed>,
    pub owner: Href,
    pub reactors: Vec<Href>,
    /// None when R2004+ marks it missing.
    pub xdic: Option<Href>,
    /// R2013+: binary data in the data store section.
    pub ds_binary: bool,
    pub ent: Option<EntCommon>,
    pub data: Data,
    /// The object as read (MS to CRC). Written back as is while `data` and the rest are
    /// unchanged: clear it (or use `Object::touch`) after an edit.
    pub raw: Option<Vec<u8>>,
}

impl Object {
    /// Marks the object edited: it is encoded afresh on write.
    pub fn touch(&mut self) {
        self.raw = None;
    }
    pub fn is_entity(&self) -> bool {
        self.ent.is_some()
    }
}

/// What a type is, from its number and the classes.
pub struct Classes<'a> {
    /// (DXF name, makes entities) by index from 500.
    pub list: &'a [(String, bool)],
}

impl Classes<'_> {
    pub fn name(&self, type_code: u16) -> Option<&str> {
        let k = (type_code as usize).checked_sub(500)?;
        self.list.get(k).map(|c| c.0.as_str())
    }
    pub fn is_entity(&self, type_code: u16) -> bool {
        match type_code {
            1..=0x29 | 0x2B..=0x2F | 0x4A | 0x4D | 0x4E => true,
            0x1F2 => true,
            0x1F3 => false,
            t if t >= 500 => self.list.get(t as usize - 500).is_some_and(|c| c.1),
            _ => false,
        }
    }
}

fn eed_io<S: Io>(s: &mut S, eed: &mut Vec<Eed>) -> R<()> {
    if S::READ {
        eed.clear();
        loop {
            let mut n = 0i16;
            s.bs(&mut n)?;
            if n == 0 {
                return Ok(());
            }
            let mut e = Eed::default();
            s.h_data(&mut e.app)?;
            s.bytes(&mut e.data, n as u16 as usize)?;
            eed.push(e);
        }
    }
    for e in eed.iter_mut() {
        let mut n = e.data.len() as i16;
        s.bs(&mut n)?;
        s.h_data(&mut e.app)?;
        let len = e.data.len();
        s.bytes(&mut e.data, len)?;
    }
    s.bs(&mut 0)
}

/// The object's fields after its handle: extended data, reactors, extension dictionary, an
/// entity's common data and handles, then its own.
pub fn body_io<S: Io>(s: &mut S, o: &mut Object) -> R<()> {
    let v = s.ver();
    eed_io(s, &mut o.eed)?;
    let mut nreact = o.reactors.len() as i32;
    let mut xdic_missing = o.xdic.is_none();
    if let Some(e) = &mut o.ent {
        let mut graphic = e.graphic.is_some();
        s.b(&mut graphic)?;
        if graphic {
            let mut bytes = e.graphic.take().unwrap_or_default();
            let mut n = bytes.len() as i64;
            if v >= Version::R2010 {
                let mut u = n as u64;
                s.bll(&mut u)?;
                n = u as i64;
            } else {
                let mut l = n as i32;
                s.rl(&mut l)?;
                n = l as i64;
            }
            if !(0..1 << 28).contains(&n) {
                return bad("graphic size");
            }
            s.bytes(&mut bytes, n as usize)?;
            e.graphic = Some(bytes);
        }
        s.bb(&mut e.entmode)?;
        s.bl(&mut nreact)?;
        if v >= Version::R2004 {
            s.b(&mut xdic_missing)?;
        }
        if v >= Version::R2013 {
            s.b(&mut o.ds_binary)?;
        }
        if v <= Version::R2000 {
            s.b(&mut e.nolinks)?;
        }
        s.enc(&mut e.color)?;
        s.bd(&mut e.ltype_scale)?;
        s.bb(&mut e.ltype_flags)?;
        s.bb(&mut e.plotstyle_flags)?;
        if v >= Version::R2007 {
            s.bb(&mut e.material_flags)?;
            s.rc(&mut e.shadow_flags)?;
        }
        if v >= Version::R2010 {
            for b in &mut e.visual_styles {
                s.b(b)?;
            }
        }
        s.bs(&mut e.invisible)?;
        s.rc(&mut e.lineweight)?;
        // the handles
        if e.entmode == 0 {
            s.h(&mut o.owner)?;
        }
    } else {
        s.bl(&mut nreact)?;
        if v >= Version::R2004 {
            s.b(&mut xdic_missing)?;
        }
        if v >= Version::R2013 {
            s.b(&mut o.ds_binary)?;
        }
        s.h(&mut o.owner)?;
    }
    if S::READ {
        if !(0..100_000).contains(&nreact) {
            return bad("reactor count");
        }
        o.reactors = vec![Href::default(); nreact as usize];
    }
    for r in &mut o.reactors {
        s.h(r)?;
    }
    if S::READ {
        o.xdic = if xdic_missing { None } else { Some(Href::default()) };
    }
    if let Some(x) = &mut o.xdic {
        s.h(x)?;
    }
    if let Some(e) = &mut o.ent {
        if v <= Version::R2000 && !e.nolinks {
            s.h(&mut e.prev)?;
            s.h(&mut e.next)?;
        }
        if let Some(b) = &mut e.color.book {
            s.h(b)?;
        }
        s.h(&mut e.layer)?;
        if e.ltype_flags == 3 {
            s.h(&mut e.ltype)?;
        }
        if v >= Version::R2007 && e.material_flags == 3 {
            s.h(&mut e.material)?;
        }
        if e.plotstyle_flags == 3 {
            s.h(&mut e.plotstyle)?;
        }
        if v >= Version::R2010 {
            for k in 0..3 {
                if e.visual_styles[k] {
                    s.h(&mut e.visual_style[k])?;
                }
            }
        }
    }
    o.data.io(s)
}

/// Reads the object at `at` in `buf` (its MS size first).
pub fn read_object(buf: &[u8], at: usize, ver: Version, classes: &Classes, cp: Option<()>) -> R<Object> {
    let _ = cp;
    let (size, mut start) = read_ms(buf, at)?;
    let mut hsize = None;
    if ver >= Version::R2010 {
        let (h, s) = read_mc(buf, start, false)?;
        hsize = Some(h as usize);
        start = s;
    }
    let end = start + size as usize;
    if end + 2 > buf.len() {
        return bad("object past the end");
    }
    let crc = u16::from_le_bytes([buf[end], buf[end + 1]]);
    if crc16(0xC0C1, &buf[at..end]) != crc {
        return bad(format!("object at {at:#x}: CRC"));
    }
    let obj = &buf[start..end];
    let mut d = BitReader::new(obj, 0);
    let type_code = if ver >= Version::R2010 {
        match d.bb()? {
            0 => d.rc()? as u16,
            1 => d.rc()? as u16 + 0x1F0,
            _ => d.rs()?,
        }
    } else {
        d.bs()? as u16
    };
    let endbit = match hsize {
        Some(h) => (size as usize * 8).checked_sub(h).ok_or(crate::bits::Bad("handle stream size".into()))?,
        None => d.rl()? as usize,
    };
    let (code, handle) = d.h()?;
    if code != 0 {
        return bad(format!("object handle code {code}"));
    }
    // the string stream: R2007+, its presence the bit before the handles
    let mut strs = None;
    let mut data_end = endbit;
    if ver >= Version::R2007 {
        let mut b = BitReader { d: obj, p: endbit - 1 };
        data_end = endbit - 1;
        if b.b()? {
            let mut at = endbit - 1 - 16;
            let mut r = BitReader { d: obj, p: at };
            let mut n = r.rs()? as usize;
            if n & 0x8000 != 0 {
                at -= 16;
                let mut r = BitReader { d: obj, p: at };
                let hi = r.rs()? as usize;
                n = (n & 0x7FFF) | (hi << 15);
            }
            let s = at.checked_sub(n).ok_or(crate::bits::Bad("string stream size".into()))?;
            strs = Some(BitReader { d: obj, p: s });
            data_end = s;
        }
    }
    let mut r = Reader {
        ver,
        data: d,
        strs,
        hdl: BitReader { d: obj, p: endbit },
        base: handle,
    };
    let entity = classes.is_entity(type_code);
    let mut o = Object {
        handle,
        type_code,
        eed: vec![],
        owner: Href::default(),
        reactors: vec![],
        xdic: Some(Href::default()),
        ds_binary: false,
        ent: entity.then(EntCommon::default),
        data: Data::of(type_code, classes),
        raw: Some(buf[at..end + 2].to_vec()),
    };
    body_io(&mut r, &mut o)?;
    if r.data.p > data_end {
        return bad(format!("object {handle:#x} type {type_code}: data overran by {} bits", r.data.p - data_end));
    }
    if r.data.p != data_end && !matches!(o.data, Data::Raw(_)) {
        return bad(format!("object {handle:#x} type {type_code}: {} data bits left", data_end - r.data.p));
    }
    if r.hdl.p > obj.len() * 8 {
        return bad(format!("object {handle:#x}: handles overran"));
    }
    Ok(o)
}

/// The object's bytes: MS size, the streams, CRC.
pub fn write_object(o: &Object, ver: Version) -> R<Vec<u8>> {
    if let Some(raw) = &o.raw {
        return Ok(raw.clone());
    }
    Ok(encode_object(o, ver)?.0)
}

/// The object encoded afresh, and how many of its bits mean something (the rest pad to a byte,
/// then the CRC).
pub fn encode_object(o: &Object, ver: Version) -> R<(Vec<u8>, usize)> {
    let mut o = o.clone();
    let mut w = Writer::new(ver);
    w.base = o.handle;
    if ver >= Version::R2010 {
        match o.type_code {
            t @ 0..=0xFF => {
                w.data.bb(0);
                w.data.rc(t as u8);
            }
            t @ 0x1F0..=0x2EF => {
                w.data.bb(1);
                w.data.rc((t - 0x1F0) as u8);
            }
            t => {
                w.data.bb(2);
                w.data.rs(t);
            }
        }
    } else {
        w.data.bs(o.type_code as i16);
        w.data.rl(0); // the size in bits, below
    }
    w.data.h(0, o.handle);
    body_io(&mut w, &mut o)?;
    let mut data = w.data;
    if ver >= Version::R2007 {
        let has = w.strs.len > 0;
        if has {
            data.append(&w.strs);
            let n = w.strs.len;
            if n >= 0x8000 {
                data.rs((n >> 15) as u16);
                data.rs((n & 0x7FFF) as u16 | 0x8000);
            } else {
                data.rs(n as u16);
            }
        }
        data.b(has);
    }
    let endbit = data.len;
    if ver < Version::R2010 {
        let at = match data.bytes[0] >> 6 {
            0 => 18,
            1 => 10,
            _ => 2,
        };
        data.patch_rl(at, endbit as u32);
    }
    let mut all = BitWriter::new();
    all.append(&data);
    all.append(&w.hdl);
    let mut out = vec![];
    ms(&mut out, all.bytes.len() as u32);
    if ver >= Version::R2010 {
        // the handle stream's bits, padding included
        let hbits = all.bytes.len() * 8 - endbit;
        crate::bits::umc(&mut out, hbits as u32);
    }
    let meaningful = out.len() * 8 + all.len;
    out.extend_from_slice(&all.bytes);
    let crc = crc16(0xC0C1, &out);
    out.extend_from_slice(&crc.to_le_bytes());
    Ok((out, meaningful))
}

/// A string for a version: code-page bytes before R2007, UTF-16 since.
pub fn s(text: &str, ver: Version) -> Str {
    Str::new(text, ver >= Version::R2007)
}
