//! One description of each object, read and write alike: an `Io` walks a value's fields in file
//! order, a `Reader` filling them from the bits, a `Writer` putting them into bits. An object's
//! fields go to three streams: the data, the strings (R2007 on: Unicode text kept at the end of
//! the data) and the handles.

use crate::bits::{BitReader, BitWriter, Bad, R, bad};
use crate::str::Str;
use crate::version::Version;

/// A handle reference: how it refers (2 soft owner, 3 hard owner, 4 soft pointer, 5 hard
/// pointer, or 6/8/A/C relative to the referring object), and the handle it resolves to.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Href {
    pub code: u8,
    pub handle: u32,
}

impl Href {
    pub const NULL_SOFT: Href = Href { code: 4, handle: 0 };
    pub const NULL_HARD: Href = Href { code: 5, handle: 0 };
    pub fn new(code: u8, handle: u32) -> Href {
        Href { code, handle }
    }
    pub fn is_null(self) -> bool {
        self.handle == 0
    }
}

/// A colour as R2004 on stores it on entities (and R2000 as a plain index).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Color {
    /// AutoCAD colour index; 256 bylayer, 0 byblock.
    pub index: i16,
    /// R2004+: 0xRRGGBB with the method byte above it, when a true colour.
    pub rgb: Option<u32>,
    /// R2004+: a DBCOLOR (colour book) object.
    pub book: Option<Href>,
    /// R2004+: transparency, its kind in the top byte.
    pub alpha: Option<u32>,
    /// R2004+ CMC: colour and book names (the flag byte's 1 and 2 bits).
    pub name: Option<Str>,
    pub book_name: Option<Str>,
}

impl Color {
    pub fn index(i: i16) -> Color {
        Color {
            index: i,
            ..Color::default()
        }
    }
}

pub trait Io {
    const READ: bool;
    fn ver(&self) -> Version;
    fn b(&mut self, v: &mut bool) -> R<()>;
    fn bb(&mut self, v: &mut u8) -> R<()>;
    /// 3B (R2010+ 1..3 bits).
    fn b3(&mut self, v: &mut u8) -> R<()>;
    fn bs(&mut self, v: &mut i16) -> R<()>;
    fn bl(&mut self, v: &mut i32) -> R<()>;
    fn bll(&mut self, v: &mut u64) -> R<()>;
    fn bd(&mut self, v: &mut f64) -> R<()>;
    fn rc(&mut self, v: &mut u8) -> R<()>;
    fn rs(&mut self, v: &mut i16) -> R<()>;
    fn rl(&mut self, v: &mut i32) -> R<()>;
    fn rd(&mut self, v: &mut f64) -> R<()>;
    fn dd(&mut self, v: &mut f64, default: f64) -> R<()>;
    fn bt(&mut self, v: &mut f64) -> R<()>;
    fn be(&mut self, v: &mut [f64; 3]) -> R<()>;
    /// MC in the data stream (rare: some objects' counts).
    fn mc(&mut self, v: &mut i64) -> R<()>;
    /// `n` raw bytes.
    fn bytes(&mut self, v: &mut Vec<u8>, n: usize) -> R<()>;
    /// TV: the code page's bytes before R2007, UTF-16 in the string stream since.
    fn tv(&mut self, v: &mut Str) -> R<()>;
    /// T: always 8-bit, always in the data (the odd R2007+ field that stays so).
    fn t8(&mut self, v: &mut Str) -> R<()>;
    /// TU: always UTF-16 in the data stream (R2007+ fields read outside the string stream).
    fn tu_data(&mut self, v: &mut Str) -> R<()>;
    /// H in the handle stream.
    fn h(&mut self, v: &mut Href) -> R<()>;
    /// H in the data stream (an object's own handle, the handle seed).
    fn h_data(&mut self, v: &mut Href) -> R<()>;
    /// CMC: R2000 a BS; R2004+ index, RGB and names.
    fn cmc(&mut self, v: &mut Color) -> R<()>;
    /// ENC: an entity's colour, R2004+ with flags (its book handle in the handle stream).
    fn enc(&mut self, v: &mut Color) -> R<()>;
    /// The rest of the data stream as `n` bits, high first (a proxy's own data).
    fn rest(&mut self, v: &mut Vec<u8>, n: &mut usize) -> R<()>;
    /// Whether another handle can follow in the handle stream (a proxy's run to the end).
    fn more_handles(&self) -> bool;

    fn rd2(&mut self, v: &mut [f64; 2]) -> R<()> {
        self.rd(&mut v[0])?;
        self.rd(&mut v[1])
    }
    fn rd3(&mut self, v: &mut [f64; 3]) -> R<()> {
        self.rd(&mut v[0])?;
        self.rd(&mut v[1])?;
        self.rd(&mut v[2])
    }
    fn bd2(&mut self, v: &mut [f64; 2]) -> R<()> {
        self.bd(&mut v[0])?;
        self.bd(&mut v[1])
    }
    fn bd3(&mut self, v: &mut [f64; 3]) -> R<()> {
        self.bd(&mut v[0])?;
        self.bd(&mut v[1])?;
        self.bd(&mut v[2])
    }
    /// 2DD with the previous point as the defaults.
    fn dd2(&mut self, v: &mut [f64; 2], d: [f64; 2]) -> R<()> {
        self.dd(&mut v[0], d[0])?;
        self.dd(&mut v[1], d[1])
    }
    fn dd3(&mut self, v: &mut [f64; 3], d: [f64; 3]) -> R<()> {
        self.dd(&mut v[0], d[0])?;
        self.dd(&mut v[1], d[1])?;
        self.dd(&mut v[2], d[2])
    }
    /// A count as BL, then that many items, each walked by `f`.
    fn list<T: Default>(&mut self, v: &mut Vec<T>, mut f: impl FnMut(&mut Self, &mut T) -> R<()>) -> R<()> {
        let mut n = v.len() as i32;
        self.bl(&mut n)?;
        self.items(v, n, &mut f)
    }
    /// `n` items (a count read earlier), each walked by `f`.
    fn items<T: Default>(&mut self, v: &mut Vec<T>, n: i32, f: &mut impl FnMut(&mut Self, &mut T) -> R<()>) -> R<()> {
        if Self::READ {
            if !(0..=10_000_000).contains(&n) {
                return bad(format!("count {n}"));
            }
            v.clear();
            v.resize_with(n as usize, T::default);
        } else if v.len() != n as usize {
            return bad(format!("count {n} for {} items", v.len()));
        }
        for x in v.iter_mut() {
            f(self, x)?;
        }
        Ok(())
    }
}

/// Reads an object's three streams.
pub struct Reader<'a> {
    pub ver: Version,
    pub data: BitReader<'a>,
    /// R2007+: where the strings are.
    pub strs: Option<BitReader<'a>>,
    pub hdl: BitReader<'a>,
    /// The handle relative references resolve against.
    pub base: u32,
    /// Where the data stream ends (in bits).
    pub data_end: usize,
}

/// Writes them.
pub struct Writer {
    pub ver: Version,
    pub data: BitWriter,
    pub strs: BitWriter,
    pub hdl: BitWriter,
    pub base: u32,
}

impl Writer {
    pub fn new(ver: Version) -> Writer {
        Writer {
            ver,
            data: BitWriter::new(),
            strs: BitWriter::new(),
            hdl: BitWriter::new(),
            base: 0,
        }
    }
}

fn read_cmc(r: &mut BitReader, ver: Version) -> R<Color> {
    let mut c = Color::index(r.bs()?);
    if ver >= Version::R2004 {
        let rgb = r.bl()? as u32;
        let flags = r.rc()?;
        c.rgb = Some(rgb);
        // the names are text: in the string stream for R2007+; handled by the caller there
        if flags & 1 != 0 {
            c.name = Some(Str::Cp(r.t()?));
        }
        if flags & 2 != 0 {
            c.book_name = Some(Str::Cp(r.t()?));
        }
    }
    Ok(c)
}

impl Io for Reader<'_> {
    const READ: bool = true;
    fn ver(&self) -> Version {
        self.ver
    }
    fn b(&mut self, v: &mut bool) -> R<()> {
        *v = self.data.b()?;
        Ok(())
    }
    fn bb(&mut self, v: &mut u8) -> R<()> {
        *v = self.data.bb()?;
        Ok(())
    }
    fn b3(&mut self, v: &mut u8) -> R<()> {
        let mut x = 0u8;
        for _ in 0..3 {
            let b = self.data.b()?;
            x = (x << 1) | b as u8;
            if !b {
                break;
            }
        }
        *v = x;
        Ok(())
    }
    fn bs(&mut self, v: &mut i16) -> R<()> {
        *v = self.data.bs()?;
        Ok(())
    }
    fn bl(&mut self, v: &mut i32) -> R<()> {
        *v = self.data.bl()?;
        Ok(())
    }
    fn bll(&mut self, v: &mut u64) -> R<()> {
        // three bits of byte count, then the bytes, low first (the spec's 3B reading is wrong)
        let n = self.data.bits(3)? as u32;
        let mut x = 0u64;
        for k in 0..n {
            x |= (self.data.rc()? as u64) << (8 * k);
        }
        *v = x;
        Ok(())
    }
    fn bd(&mut self, v: &mut f64) -> R<()> {
        *v = self.data.bd()?;
        Ok(())
    }
    fn rc(&mut self, v: &mut u8) -> R<()> {
        *v = self.data.rc()?;
        Ok(())
    }
    fn rs(&mut self, v: &mut i16) -> R<()> {
        *v = self.data.rs()? as i16;
        Ok(())
    }
    fn rl(&mut self, v: &mut i32) -> R<()> {
        *v = self.data.rl()? as i32;
        Ok(())
    }
    fn rd(&mut self, v: &mut f64) -> R<()> {
        *v = self.data.rd()?;
        Ok(())
    }
    fn dd(&mut self, v: &mut f64, default: f64) -> R<()> {
        *v = self.data.dd(default)?;
        Ok(())
    }
    fn bt(&mut self, v: &mut f64) -> R<()> {
        *v = self.data.bt()?;
        Ok(())
    }
    fn be(&mut self, v: &mut [f64; 3]) -> R<()> {
        *v = self.data.be()?;
        Ok(())
    }
    fn mc(&mut self, v: &mut i64) -> R<()> {
        let mut x = 0i64;
        let mut shift = 0;
        loop {
            let b = self.data.rc()?;
            if b & 0x80 != 0 {
                x |= ((b & 0x7F) as i64) << shift;
                shift += 7;
                if shift > 56 {
                    return bad("MC too long");
                }
            } else {
                x |= ((b & 0x3F) as i64) << shift;
                *v = if b & 0x40 != 0 { -x } else { x };
                return Ok(());
            }
        }
    }
    fn bytes(&mut self, v: &mut Vec<u8>, n: usize) -> R<()> {
        *v = self.data.bytes(n)?;
        Ok(())
    }
    fn tv(&mut self, v: &mut Str) -> R<()> {
        *v = match &mut self.strs {
            Some(s) => Str::Wide(read_tu(s)?),
            None if self.ver >= Version::R2007 => Str::Wide(vec![]),
            None => Str::Cp(self.data.t()?),
        };
        Ok(())
    }
    fn t8(&mut self, v: &mut Str) -> R<()> {
        *v = Str::Cp(self.data.t()?);
        Ok(())
    }
    fn tu_data(&mut self, v: &mut Str) -> R<()> {
        *v = Str::Wide(read_tu(&mut self.data)?);
        Ok(())
    }
    fn h(&mut self, v: &mut Href) -> R<()> {
        let (code, n) = self.hdl.h()?;
        *v = resolve(code, n, self.base)?;
        Ok(())
    }
    fn h_data(&mut self, v: &mut Href) -> R<()> {
        let (code, n) = self.data.h()?;
        *v = Href::new(code, n);
        Ok(())
    }
    fn cmc(&mut self, v: &mut Color) -> R<()> {
        if self.ver >= Version::R2007 {
            let mut c = Color::index(self.data.bs()?);
            c.rgb = Some(self.data.bl()? as u32);
            let flags = self.data.rc()?;
            if flags & 1 != 0 {
                let mut s = Str::default();
                self.tv(&mut s)?;
                c.name = Some(s);
            }
            if flags & 2 != 0 {
                let mut s = Str::default();
                self.tv(&mut s)?;
                c.book_name = Some(s);
            }
            *v = c;
        } else {
            *v = read_cmc(&mut self.data, self.ver)?;
        }
        Ok(())
    }
    fn enc(&mut self, v: &mut Color) -> R<()> {
        if self.ver < Version::R2004 {
            *v = Color::index(self.data.bs()?);
            return Ok(());
        }
        let raw = self.data.bs()? as u16;
        let flags = raw >> 8;
        let mut c = Color::index((raw & 0x1FF) as i16);
        if flags & 0x80 != 0 {
            c.rgb = Some(self.data.bl()? as u32);
        }
        if flags & 0x40 != 0 {
            c.book = Some(Href::default()); // filled from the handle stream by the entity
        }
        if flags & 0x20 != 0 {
            c.alpha = Some(self.data.bl()? as u32);
        }
        *v = c;
        Ok(())
    }
    fn rest(&mut self, v: &mut Vec<u8>, n: &mut usize) -> R<()> {
        *n = self.data_end.saturating_sub(self.data.p);
        let mut w = BitWriter::new();
        for _ in 0..*n {
            w.bit(self.data.b()?);
        }
        *v = w.bytes;
        Ok(())
    }
    fn more_handles(&self) -> bool {
        self.hdl.p + 8 <= self.hdl.d.len() * 8
    }
}

/// A TU: BS length in UTF-16 units, the units little-endian.
fn read_tu(r: &mut BitReader) -> R<Vec<u16>> {
    let n = r.bs()?;
    if n < 0 {
        return bad("negative text length");
    }
    (0..n).map(|_| r.rs()).collect()
}

fn write_tu(w: &mut BitWriter, s: &[u16]) {
    w.bs(s.len() as i16);
    for &u in s {
        w.rs(u);
    }
}

/// A reference's absolute handle, from its code and the referring object's.
pub fn resolve(code: u8, n: u32, base: u32) -> R<Href> {
    let handle = match code {
        0..=5 => n,
        6 => base.wrapping_add(1),
        8 => base.wrapping_sub(1),
        0xA => base.wrapping_add(n),
        0xC => base.wrapping_sub(n),
        _ => return Err(Bad(format!("handle code {code:#x}"))),
    };
    Ok(Href { code, handle })
}

/// How a reference is written: its code with the offset it needs.
pub fn encode_ref(h: Href, base: u32) -> (u8, u32) {
    match h.code {
        6 | 8 | 0xA | 0xC => {
            if h.handle == base.wrapping_add(1) {
                (6, 0)
            } else if h.handle == base.wrapping_sub(1) {
                (8, 0)
            } else if h.handle > base {
                (0xA, h.handle - base)
            } else {
                (0xC, base - h.handle)
            }
        }
        c => (c, h.handle),
    }
}

impl Io for Writer {
    const READ: bool = false;
    fn ver(&self) -> Version {
        self.ver
    }
    fn b(&mut self, v: &mut bool) -> R<()> {
        self.data.b(*v);
        Ok(())
    }
    fn bb(&mut self, v: &mut u8) -> R<()> {
        self.data.bb(*v);
        Ok(())
    }
    fn b3(&mut self, v: &mut u8) -> R<()> {
        // read until a zero or the third bit: 0, 10, 110, 111
        match *v {
            0 => self.data.bits(1, 0),
            2 => self.data.bits(2, 0b10),
            6 => self.data.bits(3, 0b110),
            7 => self.data.bits(3, 0b111),
            x => return bad(format!("3B value {x}")),
        }
        Ok(())
    }
    fn bs(&mut self, v: &mut i16) -> R<()> {
        self.data.bs(*v);
        Ok(())
    }
    fn bl(&mut self, v: &mut i32) -> R<()> {
        self.data.bl(*v);
        Ok(())
    }
    fn bll(&mut self, v: &mut u64) -> R<()> {
        let n = (8 - v.leading_zeros() / 8).min(7);
        if *v >> (8 * n) != 0 {
            return bad("BLL wider than 7 bytes");
        }
        self.data.bits(3, n as u64);
        for k in 0..n {
            self.data.rc((*v >> (8 * k)) as u8);
        }
        Ok(())
    }
    fn bd(&mut self, v: &mut f64) -> R<()> {
        self.data.bd(*v);
        Ok(())
    }
    fn rc(&mut self, v: &mut u8) -> R<()> {
        self.data.rc(*v);
        Ok(())
    }
    fn rs(&mut self, v: &mut i16) -> R<()> {
        self.data.rs(*v as u16);
        Ok(())
    }
    fn rl(&mut self, v: &mut i32) -> R<()> {
        self.data.rl(*v as u32);
        Ok(())
    }
    fn rd(&mut self, v: &mut f64) -> R<()> {
        self.data.rd(*v);
        Ok(())
    }
    fn dd(&mut self, v: &mut f64, default: f64) -> R<()> {
        self.data.dd(*v, default);
        Ok(())
    }
    fn bt(&mut self, v: &mut f64) -> R<()> {
        self.data.bt(*v);
        Ok(())
    }
    fn be(&mut self, v: &mut [f64; 3]) -> R<()> {
        self.data.be(*v);
        Ok(())
    }
    fn mc(&mut self, v: &mut i64) -> R<()> {
        let mut out = vec![];
        crate::bits::mc(&mut out, *v);
        self.data.raw(&out);
        Ok(())
    }
    fn bytes(&mut self, v: &mut Vec<u8>, n: usize) -> R<()> {
        if v.len() != n {
            return bad(format!("{} bytes for {n}", v.len()));
        }
        self.data.raw(v);
        Ok(())
    }
    fn tv(&mut self, v: &mut Str) -> R<()> {
        if self.ver >= Version::R2007 {
            write_tu(&mut self.strs, &v.wide());
        } else {
            self.data.t(&v.cp());
        }
        Ok(())
    }
    fn t8(&mut self, v: &mut Str) -> R<()> {
        self.data.t(&v.cp());
        Ok(())
    }
    fn tu_data(&mut self, v: &mut Str) -> R<()> {
        write_tu(&mut self.data, &v.wide());
        Ok(())
    }
    fn h(&mut self, v: &mut Href) -> R<()> {
        let (code, n) = encode_ref(*v, self.base);
        self.hdl.h(code, n);
        Ok(())
    }
    fn h_data(&mut self, v: &mut Href) -> R<()> {
        self.data.h(v.code, v.handle);
        Ok(())
    }
    fn cmc(&mut self, v: &mut Color) -> R<()> {
        self.data.bs(v.index);
        if self.ver >= Version::R2004 {
            self.data.bl(v.rgb.unwrap_or(0) as i32);
            let flags = v.name.is_some() as u8 | (v.book_name.is_some() as u8) << 1;
            self.data.rc(flags);
            for s in [&mut v.name, &mut v.book_name].into_iter().flatten() {
                if self.ver >= Version::R2007 {
                    write_tu(&mut self.strs, &s.wide());
                } else {
                    self.data.t(&s.cp());
                }
            }
        }
        Ok(())
    }
    fn enc(&mut self, v: &mut Color) -> R<()> {
        if self.ver < Version::R2004 {
            self.data.bs(v.index);
            return Ok(());
        }
        let flags = (v.rgb.is_some() as u16) << 7 | (v.book.is_some() as u16) << 6 | (v.alpha.is_some() as u16) << 5;
        self.data.bs(((flags << 8) | (v.index as u16 & 0x1FF)) as i16);
        if let Some(rgb) = v.rgb {
            self.data.bl(rgb as i32);
        }
        if let Some(a) = v.alpha {
            self.data.bl(a as i32);
        }
        Ok(())
    }
    fn rest(&mut self, v: &mut Vec<u8>, n: &mut usize) -> R<()> {
        if v.len() * 8 < *n {
            return bad(format!("{n} bits in {} bytes", v.len()));
        }
        for k in 0..*n {
            self.data.bit(v[k / 8] & (0x80 >> (k % 8)) != 0);
        }
        Ok(())
    }
    fn more_handles(&self) -> bool {
        false
    }
}
