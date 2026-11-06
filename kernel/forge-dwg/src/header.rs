//! The header variables (chapter 9): the drawing's system variables, read and written by name
//! from one table that says which versions store each and how.

use crate::bits::{BitReader, R, bad, crc16};
use crate::io::{Color, Href, Io, Reader, Writer};
use crate::str::Str;
use crate::version::Version::{self, *};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ty {
    B,
    Bs,
    Bl,
    Bll,
    Bd,
    Rc,
    Tv,
    /// A handle in the handle stream (R2007+), else with the data.
    H,
    /// A handle always in the data stream (HANDSEED).
    Hd,
    Cmc,
    Rd2,
    Bd3,
}

/// A variable's value.
#[derive(Clone, Debug, PartialEq)]
pub enum Val {
    B(bool),
    I(i64),
    D(f64),
    S(Str),
    H(Href),
    C(Color),
    P2([f64; 2]),
    P3([f64; 3]),
}

impl Val {
    fn empty(t: Ty) -> Val {
        match t {
            Ty::B => Val::B(false),
            Ty::Bs | Ty::Bl | Ty::Bll | Ty::Rc => Val::I(0),
            Ty::Bd => Val::D(0.0),
            Ty::Tv => Val::S(Str::default()),
            Ty::H | Ty::Hd => Val::H(Href::NULL_HARD),
            Ty::Cmc => Val::C(Color::index(256)),
            Ty::Rd2 => Val::P2([0.0; 2]),
            Ty::Bd3 => Val::P3([0.0; 3]),
        }
    }
}

/// (name, type, first version, last version)
type Var = (&'static str, Ty, Version, Version);

const fn v(name: &'static str, t: Ty) -> Var {
    (name, t, R2000, R2018)
}
const fn since(name: &'static str, t: Ty, from: Version) -> Var {
    (name, t, from, R2018)
}
const fn until(name: &'static str, t: Ty, to: Version) -> Var {
    (name, t, R2000, to)
}

use Ty::*;

/// Every variable, in file order. Names in lower case are undocumented.
pub const VARS: &[Var] = &[
    since("REQUIREDVERSIONS", Bll, R2013),
    v("unknown_bd1", Bd),
    v("unknown_bd2", Bd),
    v("unknown_bd3", Bd),
    v("unknown_bd4", Bd),
    v("unknown_tv1", Tv),
    v("unknown_tv2", Tv),
    v("unknown_tv3", Tv),
    v("unknown_tv4", Tv),
    v("unknown_bl1", Bl),
    v("unknown_bl2", Bl),
    until("CVPORT_ENTITY_HEADER", H, R2000),
    v("DIMASO", B),
    v("DIMSHO", B),
    v("PLINEGEN", B),
    v("ORTHOMODE", B),
    v("REGENMODE", B),
    v("FILLMODE", B),
    v("QTEXTMODE", B),
    v("PSLTSCALE", B),
    v("LIMCHECK", B),
    since("unknown_b1", B, R2004),
    v("USRTIMER", B),
    v("SKPOLY", B),
    v("ANGDIR", B),
    v("SPLFRAME", B),
    v("MIRRTEXT", B),
    v("WORLDVIEW", B),
    v("TILEMODE", B),
    v("PLIMCHECK", B),
    v("VISRETAIN", B),
    v("DISPSILH", B),
    v("PELLIPSE", B),
    v("PROXYGRAPHICS", Bs),
    v("TREEDEPTH", Bs),
    v("LUNITS", Bs),
    v("LUPREC", Bs),
    v("AUNITS", Bs),
    v("AUPREC", Bs),
    v("ATTMODE", Bs),
    v("PDMODE", Bs),
    since("unknown_bl3", Bl, R2004),
    since("unknown_bl4", Bl, R2004),
    since("unknown_bl5", Bl, R2004),
    v("USERI1", Bs),
    v("USERI2", Bs),
    v("USERI3", Bs),
    v("USERI4", Bs),
    v("USERI5", Bs),
    v("SPLINESEGS", Bs),
    v("SURFU", Bs),
    v("SURFV", Bs),
    v("SURFTYPE", Bs),
    v("SURFTAB1", Bs),
    v("SURFTAB2", Bs),
    v("SPLINETYPE", Bs),
    v("SHADEDGE", Bs),
    v("SHADEDIF", Bs),
    v("UNITMODE", Bs),
    v("MAXACTVP", Bs),
    v("ISOLINES", Bs),
    v("CMLJUST", Bs),
    v("TEXTQLTY", Bs),
    v("LTSCALE", Bd),
    v("TEXTSIZE", Bd),
    v("TRACEWID", Bd),
    v("SKETCHINC", Bd),
    v("FILLETRAD", Bd),
    v("THICKNESS", Bd),
    v("ANGBASE", Bd),
    v("PDSIZE", Bd),
    v("PLINEWID", Bd),
    v("USERR1", Bd),
    v("USERR2", Bd),
    v("USERR3", Bd),
    v("USERR4", Bd),
    v("USERR5", Bd),
    v("CHAMFERA", Bd),
    v("CHAMFERB", Bd),
    v("CHAMFERC", Bd),
    v("CHAMFERD", Bd),
    v("FACETRES", Bd),
    v("CMLSCALE", Bd),
    v("CELTSCALE", Bd),
    v("MENUNAME", Tv),
    v("TDCREATE_DAYS", Bl),
    v("TDCREATE_MS", Bl),
    v("TDUPDATE_DAYS", Bl),
    v("TDUPDATE_MS", Bl),
    since("unknown_bl6", Bl, R2004),
    since("unknown_bl7", Bl, R2004),
    since("unknown_bl8", Bl, R2004),
    v("TDINDWG_DAYS", Bl),
    v("TDINDWG_MS", Bl),
    v("TDUSRTIMER_DAYS", Bl),
    v("TDUSRTIMER_MS", Bl),
    v("CECOLOR", Cmc),
    v("HANDSEED", Hd),
    v("CLAYER", H),
    v("TEXTSTYLE", H),
    v("CELTYPE", H),
    since("CMATERIAL", H, R2007),
    v("DIMSTYLE", H),
    v("CMLSTYLE", H),
    v("PSVPSCALE", Bd),
    v("PINSBASE", Bd3),
    v("PEXTMIN", Bd3),
    v("PEXTMAX", Bd3),
    v("PLIMMIN", Rd2),
    v("PLIMMAX", Rd2),
    v("PELEVATION", Bd),
    v("PUCSORG", Bd3),
    v("PUCSXDIR", Bd3),
    v("PUCSYDIR", Bd3),
    v("PUCSNAME", H),
    v("PUCSORTHOREF", H),
    v("PUCSORTHOVIEW", Bs),
    v("PUCSBASE", H),
    v("PUCSORGTOP", Bd3),
    v("PUCSORGBOTTOM", Bd3),
    v("PUCSORGLEFT", Bd3),
    v("PUCSORGRIGHT", Bd3),
    v("PUCSORGFRONT", Bd3),
    v("PUCSORGBACK", Bd3),
    v("INSBASE", Bd3),
    v("EXTMIN", Bd3),
    v("EXTMAX", Bd3),
    v("LIMMIN", Rd2),
    v("LIMMAX", Rd2),
    v("ELEVATION", Bd),
    v("UCSORG", Bd3),
    v("UCSXDIR", Bd3),
    v("UCSYDIR", Bd3),
    v("UCSNAME", H),
    v("UCSORTHOREF", H),
    v("UCSORTHOVIEW", Bs),
    v("UCSBASE", H),
    v("UCSORGTOP", Bd3),
    v("UCSORGBOTTOM", Bd3),
    v("UCSORGLEFT", Bd3),
    v("UCSORGRIGHT", Bd3),
    v("UCSORGFRONT", Bd3),
    v("UCSORGBACK", Bd3),
    v("DIMPOST", Tv),
    v("DIMAPOST", Tv),
    v("DIMSCALE", Bd),
    v("DIMASZ", Bd),
    v("DIMEXO", Bd),
    v("DIMDLI", Bd),
    v("DIMEXE", Bd),
    v("DIMRND", Bd),
    v("DIMDLE", Bd),
    v("DIMTP", Bd),
    v("DIMTM", Bd),
    since("DIMFXL", Bd, R2007),
    since("DIMJOGANG", Bd, R2007),
    since("DIMTFILL", Bs, R2007),
    since("DIMTFILLCLR", Cmc, R2007),
    v("DIMTOL", B),
    v("DIMLIM", B),
    v("DIMTIH", B),
    v("DIMTOH", B),
    v("DIMSE1", B),
    v("DIMSE2", B),
    v("DIMTAD", Bs),
    v("DIMZIN", Bs),
    v("DIMAZIN", Bs),
    since("DIMARCSYM", Bs, R2007),
    v("DIMTXT", Bd),
    v("DIMCEN", Bd),
    v("DIMTSZ", Bd),
    v("DIMALTF", Bd),
    v("DIMLFAC", Bd),
    v("DIMTVP", Bd),
    v("DIMTFAC", Bd),
    v("DIMGAP", Bd),
    v("DIMALTRND", Bd),
    v("DIMALT", B),
    v("DIMALTD", Bs),
    v("DIMTOFL", B),
    v("DIMSAH", B),
    v("DIMTIX", B),
    v("DIMSOXD", B),
    v("DIMCLRD", Cmc),
    v("DIMCLRE", Cmc),
    v("DIMCLRT", Cmc),
    v("DIMADEC", Bs),
    v("DIMDEC", Bs),
    v("DIMTDEC", Bs),
    v("DIMALTU", Bs),
    v("DIMALTTD", Bs),
    v("DIMAUNIT", Bs),
    v("DIMFRAC", Bs),
    v("DIMLUNIT", Bs),
    v("DIMDSEP", Bs),
    v("DIMTMOVE", Bs),
    v("DIMJUST", Bs),
    v("DIMSD1", B),
    v("DIMSD2", B),
    v("DIMTOLJ", Bs),
    v("DIMTZIN", Bs),
    v("DIMALTZ", Bs),
    v("DIMALTTZ", Bs),
    v("DIMUPT", B),
    v("DIMATFIT", Bs),
    since("DIMFXLON", B, R2007),
    since("DIMTXTDIRECTION", B, R2010),
    since("DIMALTMZF", Bd, R2010),
    since("DIMALTMZS", Tv, R2010),
    since("DIMMZF", Bd, R2010),
    since("DIMMZS", Tv, R2010),
    v("DIMTXSTY", H),
    v("DIMLDRBLK", H),
    v("DIMBLK", H),
    v("DIMBLK1", H),
    v("DIMBLK2", H),
    since("DIMLTYPE", H, R2007),
    since("DIMLTEX1", H, R2007),
    since("DIMLTEX2", H, R2007),
    v("DIMLWD", Bs),
    v("DIMLWE", Bs),
    v("BLOCK_CONTROL", H),
    v("LAYER_CONTROL", H),
    v("STYLE_CONTROL", H),
    v("LTYPE_CONTROL", H),
    v("VIEW_CONTROL", H),
    v("UCS_CONTROL", H),
    v("VPORT_CONTROL", H),
    v("APPID_CONTROL", H),
    v("DIMSTYLE_CONTROL", H),
    until("VX_CONTROL", H, R2000),
    v("DICT_ACAD_GROUP", H),
    v("DICT_ACAD_MLINESTYLE", H),
    v("DICT_NAMED_OBJECTS", H),
    v("TSTACKALIGN", Bs),
    v("TSTACKSIZE", Bs),
    v("HYPERLINKBASE", Tv),
    v("STYLESHEET", Tv),
    v("DICT_LAYOUTS", H),
    v("DICT_PLOTSETTINGS", H),
    v("DICT_PLOTSTYLES", H),
    since("DICT_MATERIALS", H, R2004),
    since("DICT_COLORS", H, R2004),
    since("DICT_VISUALSTYLE", H, R2007),
    since("unknown_h1", H, R2013),
    v("FLAGS", Bl),
    v("INSUNITS", Bs),
    v("CEPSNTYPE", Bs),
    // CPSNID follows here when CEPSNTYPE is 3
    v("FINGERPRINTGUID", Tv),
    v("VERSIONGUID", Tv),
    since("SORTENTS", Rc, R2004),
    since("INDEXCTL", Rc, R2004),
    since("HIDETEXT", Rc, R2004),
    since("XCLIPFRAME", Rc, R2004),
    since("DIMASSOC", Rc, R2004),
    since("HALOGAP", Rc, R2004),
    since("OBSCUREDCOLOR", Bs, R2004),
    since("INTERSECTIONCOLOR", Bs, R2004),
    since("OBSCUREDLTYPE", Rc, R2004),
    since("INTERSECTIONDISPLAY", Rc, R2004),
    since("PROJECTNAME", Tv, R2004),
    v("BLOCK_RECORD_PAPER_SPACE", H),
    v("BLOCK_RECORD_MODEL_SPACE", H),
    v("LTYPE_BYLAYER", H),
    v("LTYPE_BYBLOCK", H),
    v("LTYPE_CONTINUOUS", H),
    since("CAMERADISPLAY", B, R2007),
    since("unknown_bl9", Bl, R2007),
    since("unknown_bl10", Bl, R2007),
    since("unknown_bd5", Bd, R2007),
    since("STEPSPERSEC", Bd, R2007),
    since("STEPSIZE", Bd, R2007),
    since("3DDWFPREC", Bd, R2007),
    since("LENSLENGTH", Bd, R2007),
    since("CAMERAHEIGHT", Bd, R2007),
    since("SOLIDHIST", Rc, R2007),
    since("SHOWHIST", Rc, R2007),
    since("PSOLWIDTH", Bd, R2007),
    since("PSOLHEIGHT", Bd, R2007),
    since("LOFTANG1", Bd, R2007),
    since("LOFTANG2", Bd, R2007),
    since("LOFTMAG1", Bd, R2007),
    since("LOFTMAG2", Bd, R2007),
    since("LOFTPARAM", Bs, R2007),
    since("LOFTNORMALS", Rc, R2007),
    since("LATITUDE", Bd, R2007),
    since("LONGITUDE", Bd, R2007),
    since("NORTHDIRECTION", Bd, R2007),
    since("TIMEZONE", Bl, R2007),
    since("LIGHTGLYPHDISPLAY", Rc, R2007),
    since("TILEMODELIGHTSYNCH", Rc, R2007),
    since("DWFFRAME", Rc, R2007),
    since("DGNFRAME", Rc, R2007),
    since("unknown_b2", B, R2007),
    since("INTERFERECOLOR", Cmc, R2007),
    since("INTERFEREOBJVS", H, R2007),
    since("INTERFEREVPVS", H, R2007),
    since("DRAGVS", H, R2007),
    since("CSHADOW", Rc, R2007),
    since("unknown_bd6", Bd, R2007),
    until("unknown_bs1", Bs, R2004),
    until("unknown_bs2", Bs, R2004),
    until("unknown_bs3", Bs, R2004),
    until("unknown_bs4", Bs, R2004),
];

/// The header variables, in file order.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Header {
    /// (name, value) for each variable the version stores.
    pub vars: Vec<(&'static str, Val)>,
    /// CPSNID, when CEPSNTYPE is 3.
    pub cpsnid: Option<Href>,
    /// R2007+: four undocumented shorts after the handles, at the handle stream's end (before,
    /// they are the last variables).
    pub tail: [i16; 4],
}

impl Header {
    pub fn get(&self, name: &str) -> Option<&Val> {
        self.vars.iter().find(|v| v.0 == name).map(|v| &v.1)
    }
    pub fn get_mut(&mut self, name: &str) -> Option<&mut Val> {
        self.vars.iter_mut().find(|v| v.0 == name).map(|v| &mut v.1)
    }
    pub fn handle(&self, name: &str) -> Option<u32> {
        match self.get(name) {
            Some(Val::H(h)) => Some(h.handle),
            _ => None,
        }
    }
    pub fn set_handle(&mut self, name: &str, h: u32) {
        if let Some(Val::H(x)) = self.get_mut(name) {
            x.handle = h;
        }
    }
}

/// Walks the variables (filling `h.vars` when reading).
pub fn header_io<S: Io>(h: &mut Header, s: &mut S) -> R<()> {
    let ver = s.ver();
    if S::READ {
        h.vars = VARS
            .iter()
            .filter(|x| (x.2..=x.3).contains(&ver))
            .map(|x| (x.0, Val::empty(x.1)))
            .collect();
    }
    let tys: Vec<Ty> = VARS.iter().filter(|x| (x.2..=x.3).contains(&ver)).map(|x| x.1).collect();
    if tys.len() != h.vars.len() {
        return bad("header variables for another version");
    }
    for (k, t) in tys.into_iter().enumerate() {
        let val = &mut h.vars[k].1;
        match (t, val) {
            (B, Val::B(b)) => s.b(b)?,
            (Bs, Val::I(i)) => {
                let mut x = *i as i16;
                s.bs(&mut x)?;
                *i = x as i64;
            }
            (Bl, Val::I(i)) => {
                let mut x = *i as i32;
                s.bl(&mut x)?;
                *i = x as i64;
            }
            (Bll, Val::I(i)) => {
                let mut x = *i as u64;
                s.bll(&mut x)?;
                *i = x as i64;
            }
            (Rc, Val::I(i)) => {
                let mut x = *i as u8;
                s.rc(&mut x)?;
                *i = x as i64;
            }
            (Bd, Val::D(d)) => s.bd(d)?,
            (Tv, Val::S(x)) => s.tv(x)?,
            (H, Val::H(x)) => s.h(x)?,
            (Hd, Val::H(x)) => s.h_data(x)?,
            (Cmc, Val::C(c)) => s.cmc(c)?,
            (Rd2, Val::P2(p)) => s.rd2(p)?,
            (Bd3, Val::P3(p)) => s.bd3(p)?,
            (t, _) => return bad(format!("header variable {} is not a {t:?}", h.vars[k].0)),
        }
        if h.vars[k].0 == "CEPSNTYPE" {
            let three = matches!(h.vars[k].1, Val::I(3));
            if three {
                let mut c = h.cpsnid.unwrap_or(Href::NULL_HARD);
                s.h(&mut c)?;
                h.cpsnid = Some(c);
            }
        }
    }
    Ok(())
}

const SN_HEADER: [u8; 16] = [
    0xCF, 0x7B, 0x1F, 0x23, 0xFD, 0xDE, 0x38, 0xA9, 0x5F, 0x7C, 0x68, 0xB8, 0x4E, 0x6D, 0x33, 0x5F,
];
const SN_HEADER_END: [u8; 16] = [
    0x30, 0x84, 0xE0, 0xDC, 0x02, 0x21, 0xC7, 0x56, 0xA0, 0x83, 0x97, 0x47, 0xB1, 0x92, 0xCC, 0xA0,
];

/// Whether the size's high half follows it (R2010/R2013 past maintenance 3, R2018 always).
pub fn has_hi_size(ver: Version, maint: u8) -> bool {
    (ver >= R2010 && maint > 3) || ver >= R2018
}

/// Reads the header section (its sentinels, size and CRC checked).
pub fn read_header(sec: &[u8], ver: Version, maint: u8) -> R<Header> {
    if sec.get(0..16) != Some(&SN_HEADER[..]) {
        return bad("header: no sentinel");
    }
    let rl = |at: usize| -> R<usize> {
        match sec.get(at..at + 4) {
            Some(b) => Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize),
            None => bad("header: past the end"),
        }
    };
    let size = rl(16)?;
    let at = if has_hi_size(ver, maint) { 24 } else { 20 };
    let Some(data) = sec.get(at..at + size) else {
        return bad("header: past the end");
    };
    let Some(c) = sec.get(at + size..at + size + 2) else {
        return bad("header: no CRC");
    };
    if crc16(0xC0C1, &sec[16..at + size]) != u16::from_le_bytes([c[0], c[1]]) {
        return bad("header: CRC");
    }
    let mut r = if ver >= R2007 {
        // the size in bits of data and strings, counted from itself; the handles after it
        let bits = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
        let (strs, end) = crate::file::r2000::string_stream(data, bits)?;
        Reader {
            ver,
            data: BitReader::new(data, 4),
            strs,
            hdl: BitReader { d: data, p: bits },
            base: 0,
            data_end: end,
        }
    } else {
        Reader {
            ver,
            data: BitReader::new(data, 0),
            strs: None,
            hdl: BitReader::new(data, 0),
            base: 0,
            data_end: data.len() * 8,
        }
    };
    let mut h = Header::default();
    if ver < R2007 {
        // handles share the one stream
        let mut w = HeaderOneStream { r: &mut r };
        header_io(&mut h, &mut w)?;
    } else {
        header_io(&mut h, &mut r)?;
    }
    if ver >= R2007 {
        for t in &mut h.tail {
            *t = r.hdl.bs()?;
        }
    }
    Ok(h)
}

/// The header's bits as stored, and how many of them mean something.
pub fn encode_header(h: &Header, ver: Version) -> R<(Vec<u8>, usize)> {
    let mut h = h.clone();
    let mut w = Writer::new(ver);
    if ver < R2007 {
        let mut one = HeaderOneStreamW { w: &mut w };
        header_io(&mut h, &mut one)?;
        return Ok((w.data.bytes, w.data.len));
    }
    w.data.rl(0); // the size in bits, below
    header_io(&mut h, &mut w)?;
    for t in h.tail {
        w.hdl.bs(t);
    }
    let mut d = w.data;
    let has = w.strs.len > 0;
    if has {
        d.append(&w.strs);
        let n = w.strs.len;
        if n >= 0x8000 {
            d.rs((n >> 15) as u16);
            d.rs((n & 0x7FFF) as u16 | 0x8000);
        } else {
            d.rs(n as u16);
        }
    }
    d.b(has);
    let bits = d.len;
    d.patch_rl(0, bits as u32);
    d.append(&w.hdl);
    let n = d.len;
    Ok((d.bytes, n))
}

/// The header section: sentinel, size, the bits, CRC, sentinel.
pub fn write_header(h: &Header, ver: Version, maint: u8) -> R<Vec<u8>> {
    let (data, _) = encode_header(h, ver)?;
    let mut body = (data.len() as u32).to_le_bytes().to_vec();
    if has_hi_size(ver, maint) {
        body.extend_from_slice(&[0; 4]);
    }
    body.extend_from_slice(&data);
    let crc = crc16(0xC0C1, &body);
    let mut out = SN_HEADER.to_vec();
    out.extend_from_slice(&body);
    out.extend_from_slice(&crc.to_le_bytes());
    out.extend_from_slice(&SN_HEADER_END);
    Ok(out)
}

/// Before R2007 the header's handles are inline: an `Io` sending `h` to the data stream.
struct HeaderOneStream<'a, 'b> {
    r: &'a mut Reader<'b>,
}

struct HeaderOneStreamW<'a> {
    w: &'a mut Writer,
}

macro_rules! forward {
    ($t:ty, $f:ident, $read:expr) => {
        impl Io for $t {
            const READ: bool = $read;
            fn ver(&self) -> Version {
                self.$f.ver()
            }
            fn b(&mut self, v: &mut bool) -> R<()> {
                self.$f.b(v)
            }
            fn bb(&mut self, v: &mut u8) -> R<()> {
                self.$f.bb(v)
            }
            fn b3(&mut self, v: &mut u8) -> R<()> {
                self.$f.b3(v)
            }
            fn bs(&mut self, v: &mut i16) -> R<()> {
                self.$f.bs(v)
            }
            fn bl(&mut self, v: &mut i32) -> R<()> {
                self.$f.bl(v)
            }
            fn bll(&mut self, v: &mut u64) -> R<()> {
                self.$f.bll(v)
            }
            fn bd(&mut self, v: &mut f64) -> R<()> {
                self.$f.bd(v)
            }
            fn rc(&mut self, v: &mut u8) -> R<()> {
                self.$f.rc(v)
            }
            fn rs(&mut self, v: &mut i16) -> R<()> {
                self.$f.rs(v)
            }
            fn rl(&mut self, v: &mut i32) -> R<()> {
                self.$f.rl(v)
            }
            fn rd(&mut self, v: &mut f64) -> R<()> {
                self.$f.rd(v)
            }
            fn dd(&mut self, v: &mut f64, d: f64) -> R<()> {
                self.$f.dd(v, d)
            }
            fn bt(&mut self, v: &mut f64) -> R<()> {
                self.$f.bt(v)
            }
            fn be(&mut self, v: &mut [f64; 3]) -> R<()> {
                self.$f.be(v)
            }
            fn mc(&mut self, v: &mut i64) -> R<()> {
                self.$f.mc(v)
            }
            fn bytes(&mut self, v: &mut Vec<u8>, n: usize) -> R<()> {
                self.$f.bytes(v, n)
            }
            fn tv(&mut self, v: &mut Str) -> R<()> {
                self.$f.tv(v)
            }
            fn t8(&mut self, v: &mut Str) -> R<()> {
                self.$f.t8(v)
            }
            fn tu_data(&mut self, v: &mut Str) -> R<()> {
                self.$f.tu_data(v)
            }
            fn h(&mut self, v: &mut Href) -> R<()> {
                self.$f.h_data(v)
            }
            fn h_data(&mut self, v: &mut Href) -> R<()> {
                self.$f.h_data(v)
            }
            fn cmc(&mut self, v: &mut Color) -> R<()> {
                self.$f.cmc(v)
            }
            fn enc(&mut self, v: &mut Color) -> R<()> {
                self.$f.enc(v)
            }
            fn rest(&mut self, v: &mut Vec<u8>, n: &mut usize) -> R<()> {
                self.$f.rest(v, n)
            }
            fn more_handles(&self) -> bool {
                self.$f.more_handles()
            }
        }
    };
}

forward!(HeaderOneStream<'_, '_>, r, true);
forward!(HeaderOneStreamW<'_>, w, false);
