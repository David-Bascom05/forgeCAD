//! The symbol tables: their control objects and records (chapter 20.4.51-70).

use super::Spec;
use crate::bits::R;
use crate::io::{Color, Href, Io};
use crate::str::Str;
use crate::version::Version;

/// A control object: its records, and the few kept outside the count (*MODEL_SPACE and
/// *PAPER_SPACE, ByBlock and ByLayer, a dimstyle control's extra ones).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Control {
    pub entries: Vec<Href>,
    pub extra: Vec<Href>,
}

fn control_io<S: Io>(c: &mut Control, s: &mut S, extra: Option<usize>) -> R<()> {
    let mut n = c.entries.len() as i32;
    s.bl(&mut n)?;
    let mut nx = c.extra.len() as i32;
    match extra {
        Some(k) => nx = k as i32,
        None => {
            // a dimstyle control: RC count of further handles
            let mut b = nx as u8;
            s.rc(&mut b)?;
            nx = b as i32;
        }
    }
    s.items(&mut c.entries, n, &mut |s, h| s.h(h))?;
    s.items(&mut c.extra, nx, &mut |s, h| s.h(h))
}

macro_rules! controls {
    ($($name:ident: $extra:expr),* $(,)?) => {
        $(
            #[derive(Clone, Debug, Default, PartialEq)]
            pub struct $name(pub Control);

            impl Spec for $name {
                fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
                    control_io(&mut self.0, s, $extra)
                }
            }
        )*
    };
}

controls! {
    BlockControl: Some(2),
    LayerControl: Some(0),
    StyleControl: Some(0),
    LtypeControl: Some(2),
    ViewControl: Some(0),
    UcsControl: Some(0),
    VportControl: Some(0),
    AppIdControl: Some(0),
    DimStyleControl: None,
    VxControl: Some(0),
}

/// What every table record starts with.
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub name: Str,
    /// The 64 bit of DXF 70: referenced.
    pub referenced: bool,
    /// The xref it came from, plus one (0: none).
    pub xref_index: i16,
    pub xdep: bool,
    pub xref_block: Href,
}

impl Default for Entry {
    fn default() -> Entry {
        Entry {
            name: Str::default(),
            referenced: true,
            xref_index: 0,
            xdep: false,
            xref_block: Href::NULL_HARD,
        }
    }
}

pub(crate) fn entry_io<S: Io>(e: &mut Entry, s: &mut S) -> R<()> {
    s.tv(&mut e.name)?;
    s.b(&mut e.referenced)?;
    // R2007+ files have no xref index here, though the spec lists it
    if s.ver() < Version::R2007 {
        s.bs(&mut e.xref_index)?;
    }
    s.b(&mut e.xdep)
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct BlockHeader {
    pub entry: Entry,
    pub anonymous: bool,
    pub has_attribs: bool,
    pub is_xref: bool,
    pub overlaid: bool,
    pub loaded: bool,
    pub base: [f64; 3],
    pub xref_path: Str,
    /// R2000+: one non-zero byte per insert handle.
    pub insert_count: Vec<u8>,
    pub description: Str,
    pub preview: Vec<u8>,
    pub units: i16,
    pub explodable: bool,
    pub scaling: u8,
    pub block: Href,
    /// R2000 and before: the first and last entity (when not an xref).
    pub first: Href,
    pub last: Href,
    /// R2004+: the entities owned.
    pub owned: Vec<Href>,
    pub endblk: Href,
    pub inserts: Vec<Href>,
    pub layout: Href,
}

impl Spec for BlockHeader {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        let v = s.ver();
        entry_io(&mut self.entry, s)?;
        s.b(&mut self.anonymous)?;
        s.b(&mut self.has_attribs)?;
        s.b(&mut self.is_xref)?;
        s.b(&mut self.overlaid)?;
        s.b(&mut self.loaded)?;
        let mut owned = self.owned.len() as i32;
        if v >= Version::R2004 && !self.is_xref && !self.overlaid {
            s.bl(&mut owned)?;
        } else {
            owned = 0;
        }
        s.bd3(&mut self.base)?;
        s.tv(&mut self.xref_path)?;
        if S::READ {
            self.insert_count.clear();
            loop {
                let mut c = 0u8;
                s.rc(&mut c)?;
                if c == 0 {
                    break;
                }
                self.insert_count.push(c);
            }
        } else {
            self.insert_count = vec![1; self.inserts.len()];
            for c in &mut self.insert_count.clone() {
                s.rc(c)?;
            }
            s.rc(&mut 0)?;
        }
        s.tv(&mut self.description)?;
        let mut n = self.preview.len() as i32;
        s.bl(&mut n)?;
        s.bytes(&mut self.preview, n.max(0) as usize)?;
        if v >= Version::R2007 {
            s.bs(&mut self.units)?;
            s.b(&mut self.explodable)?;
            s.rc(&mut self.scaling)?;
        }
        s.h(&mut self.entry.xref_block)?;
        s.h(&mut self.block)?;
        if v <= Version::R2000 && !self.is_xref && !self.overlaid {
            s.h(&mut self.first)?;
            s.h(&mut self.last)?;
        }
        s.items(&mut self.owned, owned, &mut |s, h| s.h(h))?;
        s.h(&mut self.endblk)?;
        let ni = self.insert_count.len() as i32;
        s.items(&mut self.inserts, ni, &mut |s, h| s.h(h))?;
        s.h(&mut self.layout)
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Layer {
    pub entry: Entry,
    /// 1 frozen, 2 off, 4 frozen in new viewports, 8 locked, 16 plotted, lineweight << 5.
    pub flags: i16,
    /// Negative when off.
    pub color: Color,
    pub plotstyle: Href,
    pub material: Href,
    pub ltype: Href,
    /// R2013+: a further hard pointer, null in every file seen.
    pub unknown: Href,
}

impl Spec for Layer {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        entry_io(&mut self.entry, s)?;
        s.bs(&mut self.flags)?;
        s.cmc(&mut self.color)?;
        s.h(&mut self.entry.xref_block)?;
        s.h(&mut self.plotstyle)?;
        if s.ver() >= Version::R2007 {
            s.h(&mut self.material)?;
        }
        s.h(&mut self.ltype)?;
        if s.ver() >= Version::R2013 {
            s.h(&mut self.unknown)?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Style {
    pub entry: Entry,
    pub vertical: bool,
    pub shape_file: bool,
    pub fixed_height: f64,
    pub width: f64,
    pub oblique: f64,
    pub generation: u8,
    pub last_height: f64,
    pub font: Str,
    pub bigfont: Str,
}

impl Spec for Style {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        entry_io(&mut self.entry, s)?;
        s.b(&mut self.vertical)?;
        s.b(&mut self.shape_file)?;
        s.bd(&mut self.fixed_height)?;
        s.bd(&mut self.width)?;
        s.bd(&mut self.oblique)?;
        s.rc(&mut self.generation)?;
        s.bd(&mut self.last_height)?;
        s.tv(&mut self.font)?;
        s.tv(&mut self.bigfont)?;
        s.h(&mut self.entry.xref_block)
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Dash {
    pub length: f64,
    pub shape: i16,
    pub offset: [f64; 2],
    pub scale: f64,
    pub rotation: f64,
    pub flags: i16,
    pub shapefile: Href,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Ltype {
    pub entry: Entry,
    pub description: Str,
    pub length: f64,
    pub alignment: u8,
    pub dashes: Vec<Dash>,
    /// The text area complex dashes index into (256 bytes; R2007+ 512 when present).
    pub strings: Vec<u8>,
}

impl Spec for Ltype {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        entry_io(&mut self.entry, s)?;
        s.tv(&mut self.description)?;
        s.bd(&mut self.length)?;
        s.rc(&mut self.alignment)?;
        let mut n = self.dashes.len() as u8;
        s.rc(&mut n)?;
        s.items(&mut self.dashes, n as i32, &mut |s, d| {
            s.bd(&mut d.length)?;
            s.bs(&mut d.shape)?;
            s.rd(&mut d.offset[0])?;
            s.rd(&mut d.offset[1])?;
            s.bd(&mut d.scale)?;
            s.bd(&mut d.rotation)?;
            s.bs(&mut d.flags)
        })?;
        let len = if s.ver() < Version::R2007 {
            256
        } else if self.dashes.iter().any(|d| d.flags & 2 != 0) {
            512
        } else {
            0
        };
        if !S::READ && self.strings.len() != len {
            self.strings.resize(len, 0);
        }
        s.bytes(&mut self.strings, len)?;
        s.h(&mut self.entry.xref_block)?;
        for d in &mut self.dashes {
            s.h(&mut d.shapefile)?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct AppId {
    pub entry: Entry,
    pub unknown: u8,
}

impl Spec for AppId {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        entry_io(&mut self.entry, s)?;
        s.rc(&mut self.unknown)?;
        s.h(&mut self.entry.xref_block)
    }
}

/// The dimension variables a DIMSTYLE holds (and the header holds the current ones of).
#[derive(Clone, Debug, PartialEq, Default)]
pub struct DimVars {
    pub post: Str,
    pub apost: Str,
    pub scale: f64,
    pub asz: f64,
    pub exo: f64,
    pub dli: f64,
    pub exe: f64,
    pub rnd: f64,
    pub dle: f64,
    pub tp: f64,
    pub tm: f64,
    pub fxl: f64,
    pub jogang: f64,
    pub tfill: i16,
    pub tfillclr: Color,
    pub tol: bool,
    pub lim: bool,
    pub tih: bool,
    pub toh: bool,
    pub se1: bool,
    pub se2: bool,
    pub tad: i16,
    pub zin: i16,
    pub azin: i16,
    pub arcsym: i16,
    pub txt: f64,
    pub cen: f64,
    pub tsz: f64,
    pub altf: f64,
    pub lfac: f64,
    pub tvp: f64,
    pub tfac: f64,
    pub gap: f64,
    pub altrnd: f64,
    pub alt: bool,
    pub altd: i16,
    pub tofl: bool,
    pub sah: bool,
    pub tix: bool,
    pub soxd: bool,
    pub clrd: Color,
    pub clre: Color,
    pub clrt: Color,
    pub adec: i16,
    pub dec: i16,
    pub tdec: i16,
    pub altu: i16,
    pub alttd: i16,
    pub aunit: i16,
    pub frac: i16,
    pub lunit: i16,
    pub dsep: i16,
    pub tmove: i16,
    pub just: i16,
    pub sd1: bool,
    pub sd2: bool,
    pub tolj: i16,
    pub tzin: i16,
    pub altz: i16,
    pub alttz: i16,
    pub upt: bool,
    pub atfit: i16,
    pub fxlon: bool,
    pub txtdirection: bool,
    pub altmzf: f64,
    pub altmzs: Str,
    pub mzf: f64,
    pub mzs: Str,
    pub lwd: i16,
    pub lwe: i16,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct DimStyle {
    pub entry: Entry,
    pub vars: DimVars,
    pub flag0: bool,
    pub txsty: Href,
    pub ldrblk: Href,
    pub blk: Href,
    pub blk1: Href,
    pub blk2: Href,
    pub ltype: Href,
    pub ltex1: Href,
    pub ltex2: Href,
}

impl Spec for DimStyle {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        let v = s.ver();
        entry_io(&mut self.entry, s)?;
        let d = &mut self.vars;
        s.tv(&mut d.post)?;
        s.tv(&mut d.apost)?;
        for x in [
            &mut d.scale,
            &mut d.asz,
            &mut d.exo,
            &mut d.dli,
            &mut d.exe,
            &mut d.rnd,
            &mut d.dle,
            &mut d.tp,
            &mut d.tm,
        ] {
            s.bd(x)?;
        }
        if v >= Version::R2007 {
            s.bd(&mut d.fxl)?;
            s.bd(&mut d.jogang)?;
            s.bs(&mut d.tfill)?;
            s.cmc(&mut d.tfillclr)?;
        }
        for x in [
            &mut d.tol, &mut d.lim, &mut d.tih, &mut d.toh, &mut d.se1, &mut d.se2,
        ] {
            s.b(x)?;
        }
        for x in [&mut d.tad, &mut d.zin, &mut d.azin] {
            s.bs(x)?;
        }
        if v >= Version::R2007 {
            s.bs(&mut d.arcsym)?;
        }
        for x in [
            &mut d.txt,
            &mut d.cen,
            &mut d.tsz,
            &mut d.altf,
            &mut d.lfac,
            &mut d.tvp,
            &mut d.tfac,
            &mut d.gap,
            &mut d.altrnd,
        ] {
            s.bd(x)?;
        }
        s.b(&mut d.alt)?;
        s.bs(&mut d.altd)?;
        for x in [&mut d.tofl, &mut d.sah, &mut d.tix, &mut d.soxd] {
            s.b(x)?;
        }
        for c in [&mut d.clrd, &mut d.clre, &mut d.clrt] {
            s.cmc(c)?;
        }
        for x in [
            &mut d.adec,
            &mut d.dec,
            &mut d.tdec,
            &mut d.altu,
            &mut d.alttd,
            &mut d.aunit,
            &mut d.frac,
            &mut d.lunit,
            &mut d.dsep,
            &mut d.tmove,
            &mut d.just,
        ] {
            s.bs(x)?;
        }
        s.b(&mut d.sd1)?;
        s.b(&mut d.sd2)?;
        for x in [&mut d.tolj, &mut d.tzin, &mut d.altz, &mut d.alttz] {
            s.bs(x)?;
        }
        s.b(&mut d.upt)?;
        s.bs(&mut d.atfit)?;
        if v >= Version::R2007 {
            s.b(&mut d.fxlon)?;
        }
        if v >= Version::R2010 {
            s.b(&mut d.txtdirection)?;
            s.bd(&mut d.altmzf)?;
            s.tv(&mut d.altmzs)?;
            s.bd(&mut d.mzf)?;
            s.tv(&mut d.mzs)?;
        }
        s.bs(&mut d.lwd)?;
        s.bs(&mut d.lwe)?;
        s.b(&mut self.flag0)?;
        s.h(&mut self.entry.xref_block)?;
        for h in [
            &mut self.txsty,
            &mut self.ldrblk,
            &mut self.blk,
            &mut self.blk1,
            &mut self.blk2,
        ] {
            s.h(h)?;
        }
        if v >= Version::R2007 {
            for h in [&mut self.ltype, &mut self.ltex1, &mut self.ltex2] {
                s.h(h)?;
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Vport {
    pub entry: Entry,
    pub height: f64,
    /// The aspect ratio times the height.
    pub aspect: f64,
    pub center: [f64; 2],
    pub target: [f64; 3],
    pub dir: [f64; 3],
    pub twist: f64,
    pub lens: f64,
    pub front: f64,
    pub back: f64,
    pub view_mode: u8,
    pub render_mode: u8,
    pub default_lights: bool,
    pub lighting_type: u8,
    pub brightness: f64,
    pub contrast: f64,
    pub ambient: Color,
    pub lower_left: [f64; 2],
    pub upper_right: [f64; 2],
    pub ucsfollow: bool,
    pub circle_zoom: i16,
    pub fast_zoom: bool,
    pub ucsicon: u8,
    pub grid: bool,
    pub grid_spacing: [f64; 2],
    pub snap: bool,
    pub snap_style: bool,
    pub snap_isopair: i16,
    pub snap_rotation: f64,
    pub snap_base: [f64; 2],
    pub snap_spacing: [f64; 2],
    pub unknown: bool,
    pub ucs_per_viewport: bool,
    pub ucs_origin: [f64; 3],
    pub ucs_x: [f64; 3],
    pub ucs_y: [f64; 3],
    pub ucs_elevation: f64,
    pub ucs_ortho: i16,
    pub grid_flags: i16,
    pub grid_major: i16,
    pub background: Href,
    pub visual_style: Href,
    pub sun: Href,
    pub named_ucs: Href,
    pub base_ucs: Href,
}

impl Spec for Vport {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        let v = s.ver();
        entry_io(&mut self.entry, s)?;
        s.bd(&mut self.height)?;
        s.bd(&mut self.aspect)?;
        s.rd2(&mut self.center)?;
        s.bd3(&mut self.target)?;
        s.bd3(&mut self.dir)?;
        s.bd(&mut self.twist)?;
        s.bd(&mut self.lens)?;
        s.bd(&mut self.front)?;
        s.bd(&mut self.back)?;
        let mut vm = self.view_mode as u64;
        if S::READ {
            vm = 0;
            for _ in 0..4 {
                let mut b = false;
                s.b(&mut b)?;
                vm = (vm << 1) | b as u64;
            }
        } else {
            for k in (0..4).rev() {
                s.b(&mut ((vm >> k) & 1 == 1))?;
            }
        }
        self.view_mode = vm as u8;
        s.rc(&mut self.render_mode)?;
        if v >= Version::R2007 {
            s.b(&mut self.default_lights)?;
            s.rc(&mut self.lighting_type)?;
            s.bd(&mut self.brightness)?;
            s.bd(&mut self.contrast)?;
            s.cmc(&mut self.ambient)?;
        }
        s.rd2(&mut self.lower_left)?;
        s.rd2(&mut self.upper_right)?;
        s.b(&mut self.ucsfollow)?;
        s.bs(&mut self.circle_zoom)?;
        s.b(&mut self.fast_zoom)?;
        let mut a = self.ucsicon & 2 != 0;
        let mut b = self.ucsicon & 1 != 0;
        s.b(&mut a)?;
        s.b(&mut b)?;
        self.ucsicon = (a as u8) << 1 | b as u8;
        s.b(&mut self.grid)?;
        s.rd2(&mut self.grid_spacing)?;
        s.b(&mut self.snap)?;
        s.b(&mut self.snap_style)?;
        s.bs(&mut self.snap_isopair)?;
        s.bd(&mut self.snap_rotation)?;
        s.rd2(&mut self.snap_base)?;
        s.rd2(&mut self.snap_spacing)?;
        s.b(&mut self.unknown)?;
        s.b(&mut self.ucs_per_viewport)?;
        s.bd3(&mut self.ucs_origin)?;
        s.bd3(&mut self.ucs_x)?;
        s.bd3(&mut self.ucs_y)?;
        s.bd(&mut self.ucs_elevation)?;
        s.bs(&mut self.ucs_ortho)?;
        if v >= Version::R2007 {
            s.bs(&mut self.grid_flags)?;
            s.bs(&mut self.grid_major)?;
        }
        s.h(&mut self.entry.xref_block)?;
        if v >= Version::R2007 {
            s.h(&mut self.background)?;
            s.h(&mut self.visual_style)?;
            s.h(&mut self.sun)?;
        }
        s.h(&mut self.named_ucs)?;
        s.h(&mut self.base_ucs)
    }
}
