//! More non-graphical objects: views, UCSs, groups, xrecords, the index and filter objects, image
//! definitions, fields, geographic data (chapter 20.4).

use super::Spec;
use super::objects::{PlotSettings, plot_settings_io};
use super::tables::{Entry, entry_io};
use crate::bits::{R, bad};
use crate::io::{Color, Href, Io};
use crate::str::Str;
use crate::version::Version;

/// A view mode as stored: four bits, the 71 group's 1, 2 and 4 bits and the opposite of its 16.
fn view_mode_io<S: Io>(s: &mut S, mode: &mut u8) -> R<()> {
    let mut bits = [*mode & 8 != 0, *mode & 4 != 0, *mode & 2 != 0, *mode & 1 != 0];
    for b in &mut bits {
        s.b(b)?;
    }
    *mode = bits.iter().fold(0, |m, &b| (m << 1) | b as u8);
    Ok(())
}

/// A UCS as a view or a UCS record keeps it.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct UcsFrame {
    pub origin: [f64; 3],
    pub x: [f64; 3],
    pub y: [f64; 3],
    pub elevation: f64,
    pub ortho_view: i16,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct View {
    pub entry: Entry,
    pub height: f64,
    pub width: f64,
    pub center: [f64; 2],
    pub target: [f64; 3],
    pub dir: [f64; 3],
    pub twist: f64,
    pub lens: f64,
    pub front: f64,
    pub back: f64,
    /// The stored bits, first read highest (see `view_mode_io`).
    pub view_mode: u8,
    pub render_mode: u8,
    pub default_lights: bool,
    pub lighting_type: u8,
    pub brightness: f64,
    pub contrast: f64,
    pub ambient: Color,
    pub pspace: bool,
    /// R2000+: the view has its own UCS.
    pub ucs: Option<UcsFrame>,
    pub camera_plottable: bool,
    pub background: Href,
    pub visual_style: Href,
    pub sun: Href,
    pub base_ucs: Href,
    pub named_ucs: Href,
    pub live_section: Href,
}

impl Spec for View {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        let v = s.ver();
        entry_io(&mut self.entry, s)?;
        s.bd(&mut self.height)?;
        s.bd(&mut self.width)?;
        s.rd2(&mut self.center)?;
        s.bd3(&mut self.target)?;
        s.bd3(&mut self.dir)?;
        s.bd(&mut self.twist)?;
        s.bd(&mut self.lens)?;
        s.bd(&mut self.front)?;
        s.bd(&mut self.back)?;
        view_mode_io(s, &mut self.view_mode)?;
        s.rc(&mut self.render_mode)?;
        if v >= Version::R2007 {
            s.b(&mut self.default_lights)?;
            s.rc(&mut self.lighting_type)?;
            s.bd(&mut self.brightness)?;
            s.bd(&mut self.contrast)?;
            s.cmc(&mut self.ambient)?;
        }
        s.b(&mut self.pspace)?;
        let mut has_ucs = self.ucs.is_some();
        s.b(&mut has_ucs)?;
        if S::READ {
            self.ucs = has_ucs.then(UcsFrame::default);
        }
        if let Some(u) = &mut self.ucs {
            s.bd3(&mut u.origin)?;
            s.bd3(&mut u.x)?;
            s.bd3(&mut u.y)?;
            s.bd(&mut u.elevation)?;
            s.bs(&mut u.ortho_view)?;
        }
        if v >= Version::R2007 {
            s.b(&mut self.camera_plottable)?;
        }
        s.h(&mut self.entry.xref_block)?;
        if v >= Version::R2007 {
            s.h(&mut self.background)?;
            s.h(&mut self.visual_style)?;
            s.h(&mut self.sun)?;
        }
        // the spec has these unconditionally (the one sample has a UCS, so either way fits it)
        s.h(&mut self.base_ucs)?;
        s.h(&mut self.named_ucs)?;
        if v >= Version::R2007 {
            s.h(&mut self.live_section)?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Ucs {
    pub entry: Entry,
    pub frame: UcsFrame,
    pub ortho_type: i16,
    pub base_ucs: Href,
    pub named_ucs: Href,
}

impl Spec for Ucs {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        entry_io(&mut self.entry, s)?;
        let f = &mut self.frame;
        s.bd3(&mut f.origin)?;
        s.bd3(&mut f.x)?;
        s.bd3(&mut f.y)?;
        s.bd(&mut f.elevation)?;
        s.bs(&mut f.ortho_view)?;
        s.bs(&mut self.ortho_type)?;
        s.h(&mut self.entry.xref_block)?;
        s.h(&mut self.base_ucs)?;
        s.h(&mut self.named_ucs)
    }
}

/// A viewport entity header (VX table record).
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Vx {
    pub entry: Entry,
    /// The 70 group's 1 bit: the viewport is on.
    pub on: bool,
    pub viewport: Href,
    /// The previous header in the chain (null at its end).
    pub prev: Href,
}

impl Spec for Vx {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        entry_io(&mut self.entry, s)?;
        s.b(&mut self.on)?;
        s.h(&mut self.entry.xref_block)?;
        s.h(&mut self.viewport)?;
        s.h(&mut self.prev)
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Group {
    pub name: Str,
    pub unnamed: i16,
    pub selectable: i16,
    pub entities: Vec<Href>,
}

impl Spec for Group {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        s.tv(&mut self.name)?;
        s.bs(&mut self.unnamed)?;
        s.bs(&mut self.selectable)?;
        s.list(&mut self.entities, |s, h| s.h(h))
    }
}

/// An xrecord item's value; its group code decides which.
#[derive(Clone, Debug, PartialEq)]
pub enum XValue {
    Str(Str),
    /// 3 doubles.
    Point([f64; 3]),
    Real(f64),
    I8(u8),
    I16(i16),
    I32(i32),
    I64(i64),
    /// 8 bytes, little-endian.
    Handle(u64),
    /// A binary chunk (up to 255 bytes).
    Bytes(Vec<u8>),
}

impl Default for XValue {
    fn default() -> XValue {
        XValue::I16(0)
    }
}

/// How a group code's value is stored in an xrecord.
#[derive(Clone, Copy, Debug, PartialEq)]
enum XKind {
    Str,
    Point,
    Real,
    I8,
    I16,
    I32,
    I64,
    Handle,
    Bytes,
}

fn xkind(code: i16) -> Option<XKind> {
    use XKind::*;
    Some(match code {
        0..=9 | 100..=102 | 105 | 300..=309 | 410..=419 | 430..=439 | 470..=479 | 999 | 1000..=1003 => Str,
        10..=19 | 110..=112 | 1010..=1013 => Point,
        20..=59 | 113..=149 | 210..=239 | 460..=469 | 1014..=1059 => Real,
        60..=79 | 170..=179 | 270..=279 | 370..=389 | 400..=409 | 1060..=1070 => I16,
        90..=99 | 420..=429 | 440..=459 | 1071 => I32,
        160..=169 => I64,
        280..=299 => I8,
        310..=319 | 1004 => Bytes,
        320..=369 | 390..=399 | 480..=481 | 1005 => Handle,
        _ => return None,
    })
}

/// A group code and its value.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct XItem {
    pub code: i16,
    pub value: XValue,
}

fn take<'a>(b: &'a [u8], at: &mut usize, n: usize) -> R<&'a [u8]> {
    let s = b
        .get(*at..*at + n)
        .ok_or(crate::bits::Bad("xrecord item past its data".into()))?;
    *at += n;
    Ok(s)
}

fn le<const N: usize>(b: &[u8], at: &mut usize) -> R<[u8; N]> {
    Ok(take(b, at, N)?.try_into().unwrap())
}

/// The items in an xrecord's data bytes.
pub fn xitems_read(b: &[u8], ver: Version) -> R<Vec<XItem>> {
    let mut at = 0;
    let mut out = vec![];
    while at < b.len() {
        let code = i16::from_le_bytes(le(b, &mut at)?);
        let Some(kind) = xkind(code) else {
            return bad(format!("xrecord group code {code}"));
        };
        let f = |at: &mut usize| -> R<f64> { Ok(f64::from_le_bytes(le(b, at)?)) };
        let value = match kind {
            XKind::Str if ver >= Version::R2007 => {
                let n = u16::from_le_bytes(le(b, &mut at)?) as usize;
                let w = take(b, &mut at, 2 * n)?;
                XValue::Str(Str::Wide(
                    w.chunks(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect(),
                ))
            }
            XKind::Str => {
                // RS length, RC code page, the bytes (the spec's "RC length" is wrong)
                let n = u16::from_le_bytes(le(b, &mut at)?) as usize;
                let _cp = take(b, &mut at, 1)?;
                XValue::Str(Str::Cp(take(b, &mut at, n)?.to_vec()))
            }
            XKind::Point => XValue::Point([f(&mut at)?, f(&mut at)?, f(&mut at)?]),
            XKind::Real => XValue::Real(f(&mut at)?),
            XKind::I8 => XValue::I8(take(b, &mut at, 1)?[0]),
            XKind::I16 => XValue::I16(i16::from_le_bytes(le(b, &mut at)?)),
            XKind::I32 => XValue::I32(i32::from_le_bytes(le(b, &mut at)?)),
            XKind::I64 => XValue::I64(i64::from_le_bytes(le(b, &mut at)?)),
            XKind::Handle => XValue::Handle(u64::from_le_bytes(le(b, &mut at)?)),
            XKind::Bytes => {
                let n = take(b, &mut at, 1)?[0] as usize;
                XValue::Bytes(take(b, &mut at, n)?.to_vec())
            }
        };
        out.push(XItem { code, value });
    }
    Ok(out)
}

/// An xrecord's data bytes for its items. `cp` is the drawing's code page number.
pub fn xitems_write(items: &[XItem], ver: Version, cp: u8) -> R<Vec<u8>> {
    let mut o = vec![];
    for it in items {
        let kind = xkind(it.code).ok_or(crate::bits::Bad(format!("xrecord group code {}", it.code)))?;
        o.extend_from_slice(&it.code.to_le_bytes());
        let ok = match (&it.value, kind) {
            (XValue::Str(s), XKind::Str) if ver >= Version::R2007 => {
                let w = s.wide();
                o.extend_from_slice(&(w.len() as u16).to_le_bytes());
                w.iter().for_each(|u| o.extend_from_slice(&u.to_le_bytes()));
                true
            }
            (XValue::Str(s), XKind::Str) => {
                let b = s.cp();
                o.extend_from_slice(&(b.len() as u16).to_le_bytes());
                o.push(cp);
                o.extend_from_slice(&b);
                true
            }
            (XValue::Point(p), XKind::Point) => {
                p.iter().for_each(|x| o.extend_from_slice(&x.to_le_bytes()));
                true
            }
            (XValue::Real(x), XKind::Real) => {
                o.extend_from_slice(&x.to_le_bytes());
                true
            }
            (XValue::I8(x), XKind::I8) => {
                o.push(*x);
                true
            }
            (XValue::I16(x), XKind::I16) => {
                o.extend_from_slice(&x.to_le_bytes());
                true
            }
            (XValue::I32(x), XKind::I32) => {
                o.extend_from_slice(&x.to_le_bytes());
                true
            }
            (XValue::I64(x), XKind::I64) => {
                o.extend_from_slice(&x.to_le_bytes());
                true
            }
            (XValue::Handle(x), XKind::Handle) => {
                o.extend_from_slice(&x.to_le_bytes());
                true
            }
            (XValue::Bytes(b), XKind::Bytes) if b.len() < 256 => {
                o.push(b.len() as u8);
                o.extend_from_slice(b);
                true
            }
            _ => false,
        };
        if !ok {
            return bad(format!("xrecord group code {} with {:?}", it.code, it.value));
        }
    }
    Ok(o)
}

#[derive(Clone, Debug, PartialEq)]
pub struct Xrecord {
    pub items: Vec<XItem>,
    /// The code page byte strings carry before R2007 (written for every string).
    pub code_page: u8,
    pub cloning: i16,
    /// The objects the items' handles refer to.
    pub objects: Vec<Href>,
}

impl Default for Xrecord {
    fn default() -> Xrecord {
        Xrecord {
            items: vec![],
            code_page: 30, // ANSI_1252
            cloning: 1,
            objects: vec![],
        }
    }
}

impl Spec for Xrecord {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        let v = s.ver();
        let mut bytes = if S::READ {
            vec![]
        } else {
            xitems_write(&self.items, v, self.code_page)?
        };
        let mut n = bytes.len() as i32;
        s.bl(&mut n)?;
        if n < 0 {
            return bad("xrecord size");
        }
        s.bytes(&mut bytes, n as usize)?;
        if S::READ {
            self.items = xitems_read(&bytes, v)?;
            if v < Version::R2007 {
                // the first string's code page stands for all
                if let Some(at) = first_string(&self.items) {
                    self.code_page = bytes[at];
                }
            }
        }
        s.bs(&mut self.cloning)?;
        if S::READ {
            self.objects.clear();
            // as many as are left (the handle stream's padding is under a byte)
            while s.hdl_left().is_some_and(|n| n >= 8) {
                let mut h = Href::default();
                s.h(&mut h)?;
                self.objects.push(h);
            }
        } else {
            for h in &mut self.objects {
                s.h(h)?;
            }
        }
        Ok(())
    }
}

/// Where the first string's code page byte is in an xrecord's data before R2007.
fn first_string(items: &[XItem]) -> Option<usize> {
    let mut at = 0;
    for it in items {
        if let XValue::Str(_) = it.value {
            return Some(at + 4);
        }
        at += xitems_write(std::slice::from_ref(it), Version::R2000, 0)
            .ok()?
            .len();
    }
    None
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct DictionaryVar {
    pub int: u8,
    pub value: Str,
}

impl Spec for DictionaryVar {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        s.rc(&mut self.int)?;
        // the spec says BS; it is a TV
        s.tv(&mut self.value)
    }
}

/// An annotation scale.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Scale {
    pub unknown: i16,
    pub name: Str,
    pub paper_units: f64,
    /// Divided by 10 for like units.
    pub drawing_units: f64,
    pub unit_scale: bool,
}

impl Spec for Scale {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        s.bs(&mut self.unknown)?;
        s.tv(&mut self.name)?;
        s.bd(&mut self.paper_units)?;
        s.bd(&mut self.drawing_units)?;
        s.b(&mut self.unit_scale)
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct IdBuffer {
    pub unknown: u8,
    pub objects: Vec<Href>,
}

impl Spec for IdBuffer {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        s.rc(&mut self.unknown)?;
        s.list(&mut self.objects, |s, h| s.h(h))
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct LayerIndexEntry {
    pub index: i32,
    pub layer: Str,
    pub handle: Href,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct LayerIndex {
    pub timestamp: [i32; 2],
    pub entries: Vec<LayerIndexEntry>,
}

impl Spec for LayerIndex {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        s.bl(&mut self.timestamp[0])?;
        s.bl(&mut self.timestamp[1])?;
        s.list(&mut self.entries, |s, e| {
            s.bl(&mut e.index)?;
            s.tv(&mut e.layer)
        })?;
        for e in &mut self.entries {
            s.h(&mut e.handle)?;
        }
        Ok(())
    }
}

/// An xref clip.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct SpatialFilter {
    pub points: Vec<[f64; 2]>,
    pub extrusion: [f64; 3],
    pub origin: [f64; 3],
    pub display_boundary: i16,
    pub front: Option<f64>,
    pub back: Option<f64>,
    /// 4x3, column major.
    pub inverse_block: [f64; 12],
    pub clip_bound: [f64; 12],
}

fn clip_io<S: Io>(s: &mut S, d: &mut Option<f64>) -> R<()> {
    let mut on = d.is_some() as i16;
    s.bs(&mut on)?;
    if S::READ {
        *d = (on == 1).then_some(0.0);
    }
    if let Some(x) = d {
        s.bd(x)?;
    }
    Ok(())
}

impl Spec for SpatialFilter {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        let mut n = self.points.len() as i16;
        s.bs(&mut n)?;
        s.items(&mut self.points, n as i32, &mut |s, p| s.rd2(p))?;
        s.bd3(&mut self.extrusion)?;
        s.bd3(&mut self.origin)?;
        s.bs(&mut self.display_boundary)?;
        clip_io(s, &mut self.front)?;
        clip_io(s, &mut self.back)?;
        for x in self.inverse_block.iter_mut().chain(self.clip_bound.iter_mut()) {
            s.bd(x)?;
        }
        Ok(())
    }
}

/// The spatial index: its body is undocumented, kept as the bits.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct SpatialIndex {
    pub timestamp: [i32; 2],
    /// The rest of the data stream: whole bytes, then the bits after them.
    pub rest: Vec<u8>,
    pub rest_bits: Vec<bool>,
}

impl Spec for SpatialIndex {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        s.bl(&mut self.timestamp[0])?;
        s.bl(&mut self.timestamp[1])?;
        if S::READ {
            let left = s.data_left().unwrap_or(0);
            s.bytes(&mut self.rest, left / 8)?;
            self.rest_bits = vec![false; left % 8];
        } else {
            let n = self.rest.len();
            s.bytes(&mut self.rest, n)?;
        }
        for b in &mut self.rest_bits {
            s.b(b)?;
        }
        Ok(())
    }
}

/// A block's draw order: each entity's sort handle.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct SortEntsTable {
    pub block: Href,
    /// (sort handle, entity).
    pub entries: Vec<(Href, Href)>,
}

impl Spec for SortEntsTable {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        // the sort handles are in the data stream
        s.list(&mut self.entries, |s, e| s.h_data(&mut e.0))?;
        s.h(&mut self.block)?;
        for e in &mut self.entries {
            s.h(&mut e.1)?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct ImageDef {
    pub class_version: i32,
    pub size: [f64; 2],
    pub path: Str,
    pub loaded: bool,
    /// 0 none, 2 centimetres, 5 inches.
    pub resolution_units: u8,
    pub pixel_size: [f64; 2],
}

impl Spec for ImageDef {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        s.bl(&mut self.class_version)?;
        s.rd2(&mut self.size)?;
        s.tv(&mut self.path)?;
        s.b(&mut self.loaded)?;
        s.rc(&mut self.resolution_units)?;
        s.rd2(&mut self.pixel_size)
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct ImageDefReactor {
    pub class_version: i32,
}

impl Spec for ImageDefReactor {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        s.bl(&mut self.class_version)
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct RasterVariables {
    pub class_version: i32,
    pub display_frame: i16,
    pub display_quality: i16,
    pub units: i16,
}

impl Spec for RasterVariables {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        s.bl(&mut self.class_version)?;
        s.bs(&mut self.display_frame)?;
        s.bs(&mut self.display_quality)?;
        s.bs(&mut self.units)
    }
}

/// Not in the spec; as stored: only the frame display flag (no class version, unlike
/// RASTERVARIABLES).
#[derive(Clone, Debug, PartialEq, Default)]
pub struct WipeoutVariables {
    pub display_frame: i16,
}

impl Spec for WipeoutVariables {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        s.bs(&mut self.display_frame)
    }
}

/// A field's or table cell's value (20.4.99).
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Value {
    /// R2007+: 1 the general type (no data stored).
    pub flags: i32,
    /// 0 unknown, 1 long, 2 double, 4 string, 8 date, 16 point, 32 3D point, 64 object, 512 general.
    pub data_type: i32,
    pub long: i32,
    pub real: f64,
    pub text: Str,
    /// Dates and general values; points' stored size.
    pub bytes: Vec<u8>,
    pub size: i32,
    pub point: [f64; 3],
    pub object: Href,
    pub unit_type: i32,
    pub format: Str,
    pub value_string: Str,
}

pub fn value_io<S: Io>(s: &mut S, x: &mut Value) -> R<()> {
    let v = s.ver();
    if v >= Version::R2007 {
        s.bl(&mut x.flags)?;
    }
    s.bl(&mut x.data_type)?;
    if !(v >= Version::R2007 && x.flags & 1 != 0) {
        match x.data_type {
            0 | 1 => s.bl(&mut x.long)?,
            2 => s.bd(&mut x.real)?,
            4 => s.tv(&mut x.text)?,
            8 | 512 => {
                let mut n = x.bytes.len() as i32;
                s.bl(&mut n)?;
                if n < 0 {
                    return bad("value size");
                }
                s.bytes(&mut x.bytes, n as usize)?;
            }
            16 | 32 => {
                if !S::READ && x.size == 0 {
                    x.size = if x.data_type == 16 { 16 } else { 24 };
                }
                s.bl(&mut x.size)?;
                s.rd(&mut x.point[0])?;
                s.rd(&mut x.point[1])?;
                if x.data_type == 32 {
                    s.rd(&mut x.point[2])?;
                }
            }
            64 => s.h(&mut x.object)?,
            t => return bad(format!("value type {t}")),
        }
    }
    if v >= Version::R2007 {
        s.bl(&mut x.unit_type)?;
        s.tv(&mut x.format)?;
        s.tv(&mut x.value_string)?;
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Field {
    pub evaluator: Str,
    pub code: Str,
    pub children: Vec<Href>,
    pub objects: Vec<Href>,
    /// Before R2007 (it is in the value since).
    pub format: Str,
    pub evaluation: i32,
    pub filing: i32,
    pub state: i32,
    pub status: i32,
    pub error_code: i32,
    pub error: Str,
    pub value: Value,
    pub value_string: Str,
    pub value_string_len: i32,
    pub child_values: Vec<(Str, Value)>,
}

impl Spec for Field {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        s.tv(&mut self.evaluator)?;
        s.tv(&mut self.code)?;
        s.list(&mut self.children, |s, h| s.h(h))?;
        s.list(&mut self.objects, |s, h| s.h(h))?;
        if s.ver() < Version::R2007 {
            s.tv(&mut self.format)?;
        }
        for x in [
            &mut self.evaluation,
            &mut self.filing,
            &mut self.state,
            &mut self.status,
            &mut self.error_code,
        ] {
            s.bl(x)?;
        }
        s.tv(&mut self.error)?;
        value_io(s, &mut self.value)?;
        s.tv(&mut self.value_string)?;
        // the spec says TV; DXF 98 is an integer (unvalidated)
        s.bl(&mut self.value_string_len)?;
        s.list(&mut self.child_values, |s, c| {
            s.tv(&mut c.0)?;
            value_io(s, &mut c.1)
        })
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct FieldList {
    pub unknown: bool,
    pub fields: Vec<Href>,
}

impl Spec for FieldList {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        let mut n = self.fields.len() as i32;
        s.bl(&mut n)?;
        s.b(&mut self.unknown)?;
        s.items(&mut self.fields, n, &mut |s, h| s.h(h))
    }
}

/// GEODATA's civil data (DWG R2007 and before).
#[derive(Clone, Debug, PartialEq, Default)]
pub struct GeoCivil {
    pub flag: bool,
    /// The reference point, stored twice (y, x).
    pub reference: [[f64; 2]; 2],
    pub zero: [i32; 2],
    pub points: [[f64; 2]; 2],
    pub flag2: bool,
    pub north_degrees: f64,
    pub north_radians: f64,
    pub scale_method: i32,
    pub scale: f64,
    pub sea_level_correction: bool,
    pub sea_level: f64,
    pub projection_radius: f64,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct GeoData {
    /// 1 AutoCAD 2009, 2 AutoCAD 2010, 3 AutoCAD 2013 (as 2).
    pub version: i32,
    pub host_block: Href,
    /// 0 unknown, 1 local grid, 2 projected grid, 3 geographic.
    pub coord_type: i16,
    pub design_point: [f64; 3],
    pub reference_point: [f64; 3],
    pub scale_h: f64,
    pub units_h: i32,
    pub scale_v: f64,
    pub units_v: i32,
    pub up: [f64; 3],
    pub north: [f64; 3],
    pub scale_method: i32,
    pub scale: f64,
    pub sea_level_correction: bool,
    pub sea_level: f64,
    pub projection_radius: f64,
    pub coord_system: Str,
    pub geo_rss: Str,
    /// Version 1 only.
    pub obsolete_a: [f64; 3],
    pub north_angle: f64,
    pub obsolete_b: [f64; 3],
    pub datum: Str,
    pub wkt: Str,
    pub obs_from: Str,
    pub obs_to: Str,
    pub obs_coverage: Str,
    /// (source, destination).
    pub mesh_points: Vec<([f64; 2], [f64; 2])>,
    pub mesh_faces: Vec<[i32; 3]>,
    pub civil: Option<GeoCivil>,
}

impl Spec for GeoData {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        s.bl(&mut self.version)?;
        s.h(&mut self.host_block)?;
        s.bs(&mut self.coord_type)?;
        if self.version >= 2 {
            s.bd3(&mut self.design_point)?;
            s.bd3(&mut self.reference_point)?;
            s.bd(&mut self.scale_h)?;
            s.bl(&mut self.units_h)?;
            s.bd(&mut self.scale_v)?;
            s.bl(&mut self.units_v)?;
            s.bd3(&mut self.up)?;
            s.rd3(&mut self.north)?;
            s.bl(&mut self.scale_method)?;
            s.bd(&mut self.scale)?;
            s.b(&mut self.sea_level_correction)?;
            s.bd(&mut self.sea_level)?;
            s.bd(&mut self.projection_radius)?;
            s.tv(&mut self.coord_system)?;
            s.tv(&mut self.geo_rss)?;
        } else {
            s.bd3(&mut self.reference_point)?;
            s.bl(&mut self.units_h)?;
            s.bd3(&mut self.design_point)?;
            s.bd3(&mut self.obsolete_a)?;
            s.bd3(&mut self.up)?;
            s.bd(&mut self.north_angle)?;
            s.bd3(&mut self.obsolete_b)?;
            s.tv(&mut self.coord_system)?;
            s.tv(&mut self.geo_rss)?;
            s.bd(&mut self.scale_h)?;
            s.tv(&mut self.datum)?;
            s.tv(&mut self.wkt)?;
        }
        s.tv(&mut self.obs_from)?;
        s.tv(&mut self.obs_to)?;
        s.tv(&mut self.obs_coverage)?;
        s.list(&mut self.mesh_points, |s, p| {
            s.rd2(&mut p.0)?;
            s.rd2(&mut p.1)
        })?;
        s.list(&mut self.mesh_faces, |s, f| {
            for x in f {
                s.bl(x)?;
            }
            Ok(())
        })?;
        if s.ver() <= Version::R2007 {
            let mut has = self.civil.is_some();
            s.b(&mut has)?;
            if S::READ {
                self.civil = has.then(GeoCivil::default);
            }
            if let Some(c) = &mut self.civil {
                s.b(&mut c.flag)?;
                for p in &mut c.reference {
                    s.rd(&mut p[0])?;
                    s.rd(&mut p[1])?;
                }
                s.bl(&mut c.zero[0])?;
                s.bl(&mut c.zero[1])?;
                s.rd2(&mut c.points[0])?;
                s.rd2(&mut c.points[1])?;
                s.b(&mut c.flag2)?;
                s.bd(&mut c.north_degrees)?;
                s.bd(&mut c.north_radians)?;
                s.bl(&mut c.scale_method)?;
                s.bd(&mut c.scale)?;
                s.b(&mut c.sea_level_correction)?;
                s.bd(&mut c.sea_level)?;
                s.bd(&mut c.projection_radius)?;
            }
        }
        Ok(())
    }
}

/// A named page setup: a layout's plot settings on their own.
impl Spec for PlotSettings {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        plot_settings_io(self, s)?;
        if s.ver() >= Version::R2004 {
            s.h(&mut self.view)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::object::{Classes, Object, encode_object, read_object, s as text};
    use crate::types::Data;

    const VERSIONS: [Version; 6] = [
        Version::R2000,
        Version::R2004,
        Version::R2007,
        Version::R2010,
        Version::R2013,
        Version::R2018,
    ];

    const CLASSES: [&str; 13] = [
        "IDBUFFER",
        "LAYER_INDEX",
        "SPATIAL_FILTER",
        "SPATIAL_INDEX",
        "SORTENTSTABLE",
        "IMAGEDEF",
        "IMAGEDEF_REACTOR",
        "RASTERVARIABLES",
        "FIELD",
        "FIELDLIST",
        "GEODATA",
        "PLOTSETTINGS",
        "XRECORD",
    ];

    fn h(code: u8, n: u32) -> Href {
        Href::new(code, n)
    }

    /// One value of each type the sample files lack (and an xrecord with trailing handles), as a
    /// version stores it.
    fn samples(v: Version) -> Vec<(u16, Data)> {
        let t = |x: &str| text(x, v);
        let r2007 = v >= Version::R2007;
        let value = |data_type: i32| {
            let mut x = Value {
                data_type,
                unit_type: r2007 as i32,
                format: if r2007 { t("%lu2") } else { Str::default() },
                value_string: if r2007 { t("1.00") } else { Str::default() },
                ..Value::default()
            };
            match data_type {
                1 => x.long = 7,
                2 => x.real = 2.5,
                4 => x.text = t("val"),
                8 | 512 => x.bytes = vec![1, 2, 3, 4, 5, 6, 7, 8],
                16 => (x.size, x.point) = (16, [1.0, 2.0, 0.0]),
                32 => (x.size, x.point) = (24, [1.0, 2.0, 3.0]),
                _ => x.object = h(4, 0x33),
            }
            x
        };
        let mut view = View {
            entry: Entry {
                name: t("V1"),
                ..Entry::default()
            },
            height: 10.0,
            width: 20.0,
            center: [1.0, 2.0],
            dir: [0.0, 0.0, 1.0],
            lens: 50.0,
            view_mode: 0b1001,
            ucs: Some(UcsFrame {
                x: [1.0, 0.0, 0.0],
                y: [0.0, 1.0, 0.0],
                ortho_view: 1,
                ..UcsFrame::default()
            }),
            base_ucs: h(5, 0x40),
            ..View::default()
        };
        if r2007 {
            view.default_lights = true;
            view.lighting_type = 1;
            view.ambient = Color {
                index: 250,
                rgb: Some(0xC300_0000),
                ..Color::default()
            };
            view.camera_plottable = true;
            view.sun = h(3, 0x41);
            view.live_section = h(4, 0x42);
        }
        let civil = (v <= Version::R2007).then(|| GeoCivil {
            reference: [[1.0, 2.0], [1.0, 2.0]],
            north_degrees: 90.0,
            scale_method: 1,
            scale: 1.0,
            ..GeoCivil::default()
        });
        let field = Field {
            evaluator: t("AcVar"),
            code: t("%<\\AcVar Date>%"),
            children: vec![h(3, 0x70)],
            objects: vec![h(4, 0x71), h(4, 0x72)],
            format: if r2007 { Str::default() } else { t("%tc1") },
            evaluation: 1,
            state: 8,
            status: 2,
            value: value(2),
            value_string: t("2.5"),
            value_string_len: 3,
            child_values: [1, 4, 8, 16, 32, 64]
                .into_iter()
                .chain(r2007.then_some(512))
                .map(|k| (t(&format!("k{k}")), value(k)))
                .collect(),
            ..Field::default()
        };
        let geo2 = GeoData {
            version: 2,
            host_block: h(4, 0x1F),
            coord_type: 2,
            design_point: [1.0, 2.0, 0.0],
            reference_point: [-71.0, 42.0, 0.0],
            scale_h: 1.0,
            units_h: 6,
            scale_v: 1.0,
            units_v: 6,
            up: [0.0, 0.0, 1.0],
            north: [0.0, 1.0, 0.0],
            scale_method: 1,
            scale: 1.0,
            coord_system: t("<xml/>"),
            geo_rss: t("rss"),
            mesh_points: vec![([0.0, 0.0], [1.0, 1.0])],
            mesh_faces: vec![[0, 1, 2]],
            civil: civil.clone(),
            ..GeoData::default()
        };
        let geo1 = GeoData {
            version: 1,
            host_block: h(4, 0x1F),
            coord_type: 3,
            reference_point: [-71.0, 42.0, 0.0],
            units_h: 6,
            design_point: [1.0, 2.0, 0.0],
            obsolete_b: [1.0, 1.0, 1.0],
            up: [0.0, 0.0, 1.0],
            north_angle: 0.5,
            coord_system: t("PROJCS[]"),
            scale_h: 1.0,
            datum: t("WGS84"),
            obs_from: t("from"),
            civil,
            ..GeoData::default()
        };
        let xrecord = Xrecord {
            items: vec![
                XItem {
                    code: 1,
                    value: XValue::Str(t("text")),
                },
                XItem {
                    code: 10,
                    value: XValue::Point([1.0, 2.0, 3.0]),
                },
                XItem {
                    code: 40,
                    value: XValue::Real(0.5),
                },
                XItem {
                    code: 70,
                    value: XValue::I16(-3),
                },
                XItem {
                    code: 90,
                    value: XValue::I32(1 << 20),
                },
                XItem {
                    code: 160,
                    value: XValue::I64(-1),
                },
                XItem {
                    code: 280,
                    value: XValue::I8(4),
                },
                XItem {
                    code: 310,
                    value: XValue::Bytes(vec![9, 8, 7]),
                },
                XItem {
                    code: 340,
                    value: XValue::Handle(0x75),
                },
            ],
            objects: vec![h(4, 0x75), h(5, 0x76)],
            ..Xrecord::default()
        };
        vec![
            (61, Data::View(view)),
            (
                63,
                Data::Ucs(Ucs {
                    entry: Entry {
                        name: t("MYUCS"),
                        ..Entry::default()
                    },
                    frame: UcsFrame {
                        origin: [5.0, 6.0, 7.0],
                        x: [0.0, 1.0, 0.0],
                        y: [-1.0, 0.0, 0.0],
                        ..UcsFrame::default()
                    },
                    ortho_type: 2,
                    base_ucs: h(5, 0x10),
                    named_ucs: h(5, 0x11),
                }),
            ),
            (
                500,
                Data::IdBuffer(IdBuffer {
                    unknown: 0,
                    objects: vec![h(4, 0x20), h(4, 0x21)],
                }),
            ),
            (
                501,
                Data::LayerIndex(LayerIndex {
                    timestamp: [2458000, 1234],
                    entries: vec![
                        LayerIndexEntry {
                            index: 3,
                            layer: t("0"),
                            handle: h(5, 0x50),
                        },
                        LayerIndexEntry {
                            index: 9,
                            layer: t("WALLS"),
                            handle: h(5, 0x51),
                        },
                    ],
                }),
            ),
            (
                502,
                Data::SpatialFilter(SpatialFilter {
                    points: vec![[0.0, 0.0], [4.0, 3.0]],
                    extrusion: [0.0, 0.0, 1.0],
                    display_boundary: 1,
                    front: Some(2.0),
                    back: None,
                    inverse_block: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0],
                    clip_bound: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 5.0, 5.0, 0.0],
                    ..SpatialFilter::default()
                }),
            ),
            (
                503,
                Data::SpatialIndex(SpatialIndex {
                    timestamp: [1, 2],
                    rest: vec![0xDE, 0xAD, 0xBE],
                    rest_bits: vec![true, false, true],
                }),
            ),
            (
                504,
                Data::SortEntsTable(SortEntsTable {
                    block: h(4, 0x1F),
                    entries: vec![(h(0, 0x90), h(4, 0x60)), (h(0, 0x80), h(4, 0x61))],
                }),
            ),
            (
                505,
                Data::ImageDef(ImageDef {
                    class_version: 0,
                    size: [640.0, 480.0],
                    path: t("C:\\img\\a.png"),
                    loaded: true,
                    resolution_units: 5,
                    pixel_size: [0.01, 0.01],
                }),
            ),
            (506, Data::ImageDefReactor(ImageDefReactor { class_version: 2 })),
            (
                507,
                Data::RasterVariables(RasterVariables {
                    class_version: 0,
                    display_frame: 1,
                    display_quality: 1,
                    units: 3,
                }),
            ),
            (508, Data::Field(field)),
            (
                509,
                Data::FieldList(FieldList {
                    unknown: false,
                    fields: vec![h(4, 0x73), h(4, 0x74)],
                }),
            ),
            (510, Data::GeoData(geo2)),
            (510, Data::GeoData(geo1)),
            (
                511,
                Data::PlotSettings(PlotSettings {
                    page_setup: t("Setup1"),
                    printer: t("None"),
                    flags: 688,
                    paper: [210.0, 297.0],
                    paper_size: t("ISO_A4"),
                    real_units: 1.0,
                    drawing_units: 1.0,
                    scale: 1.0,
                    view: if v >= Version::R2004 {
                        h(5, 0x77)
                    } else {
                        Href::default()
                    },
                    ..PlotSettings::default()
                }),
            ),
            (512, Data::Xrecord(xrecord)),
        ]
    }

    #[test]
    fn unvalidated_types_write_and_read_back() {
        let list: Vec<(String, bool)> = CLASSES.iter().map(|n| (n.to_string(), false)).collect();
        let classes = Classes { list: &list };
        for v in VERSIONS {
            for (type_code, data) in samples(v) {
                let o = Object {
                    handle: 0x2A,
                    type_code,
                    eed: vec![],
                    owner: h(4, 0x0C),
                    reactors: vec![h(4, 0x0C)],
                    xdic: if v >= Version::R2004 { None } else { Some(h(3, 0)) },
                    ds_binary: false,
                    ent: None,
                    data,
                    raw: None,
                };
                let name = o.data.name();
                let (bytes, _) = encode_object(&o, v).unwrap_or_else(|e| panic!("{v:?} {name}: {}", e.0));
                let back = read_object(&bytes, 0, v, &classes, None)
                    .unwrap_or_else(|e| panic!("{v:?} {name}: {}", e.0));
                // R2007+ reads empty strings left at their default as UTF-16
                let norm = |d: &Data| format!("{d:?}").replace("Cp([])", "Wide([])");
                assert_eq!(norm(&back.data), norm(&o.data), "{v:?} {name}");
                let mut again = back.clone();
                again.raw = None;
                assert_eq!(encode_object(&again, v).unwrap().0, bytes, "{v:?} {name}");
            }
        }
    }

    #[test]
    fn xrecord_items_keep_their_types() {
        let items = vec![
            XItem {
                code: 102,
                value: XValue::Str(Str::Cp(b"{ACAD".to_vec())),
            },
            XItem {
                code: 290,
                value: XValue::I8(1),
            },
            XItem {
                code: 1071,
                value: XValue::I32(5),
            },
        ];
        let b = xitems_write(&items, Version::R2000, 30).unwrap();
        assert_eq!(&b[..7], &[102, 0, 5, 0, 30, b'{', b'A']);
        assert_eq!(xitems_read(&b, Version::R2000).unwrap(), items);
        // a value its code does not take
        let wrong = [XItem {
            code: 40,
            value: XValue::I16(1),
        }];
        assert!(xitems_write(&wrong, Version::R2000, 30).is_err());
    }
}
