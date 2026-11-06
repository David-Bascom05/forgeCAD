//! zlib streams (RFC 1950) around DEFLATE (RFC 1951): inflate of stored, fixed and dynamic
//! blocks, and a deflate of hash-chained LZ77 with dynamic Huffman blocks (fixed or stored when
//! those are smaller). Input to inflate is untrusted: its output is capped and every code and
//! distance is checked.

use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InflateError {
    /// The stream ends before its last block does.
    Truncated,
    /// Not a zlib header (method, window, check bits), or a preset dictionary.
    Header,
    /// Block type 3, or a stored block whose length and its complement disagree.
    Block,
    /// A Huffman code that is over-subscribed, or a code no symbol has.
    Code,
    /// A distance back past the start of the output.
    Distance,
    /// The output would pass the cap.
    TooBig,
    /// The Adler-32 trailer disagrees with the output.
    Checksum,
}

impl fmt::Display for InflateError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

pub fn adler32(data: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    for c in data.chunks(5552) {
        for &x in c {
            a += x as u32;
            b += a;
        }
        a %= 65521;
        b %= 65521;
    }
    (b << 16) | a
}

/// A zlib stream's content and how many bytes of `data` it took.
pub fn inflate_zlib(data: &[u8], cap: usize) -> Result<(Vec<u8>, usize), InflateError> {
    let mut out = Vec::new();
    let used = inflate_zlib_into(data, cap, &mut out)?;
    Ok((out, used))
}

/// Appends a zlib stream's content to `out` (back-references see only this stream's output).
pub fn inflate_zlib_into(data: &[u8], cap: usize, out: &mut Vec<u8>) -> Result<usize, InflateError> {
    if data.len() < 2 {
        return Err(InflateError::Truncated);
    }
    let (cmf, flg) = (data[0], data[1]);
    if cmf & 15 != 8 || cmf >> 4 > 7 || !(cmf as u32 * 256 + flg as u32).is_multiple_of(31) || flg & 0x20 != 0
    {
        return Err(InflateError::Header);
    }
    let start = out.len();
    let used = 2 + inflate_into(&data[2..], cap, out)?;
    let t = data.get(used..used + 4).ok_or(InflateError::Truncated)?;
    if u32::from_be_bytes([t[0], t[1], t[2], t[3]]) != adler32(&out[start..]) {
        return Err(InflateError::Checksum);
    }
    Ok(used + 4)
}

struct Bits<'a> {
    d: &'a [u8],
    pos: usize,
    buf: u64,
    n: u32,
}

impl Bits<'_> {
    #[inline]
    fn refill(&mut self) {
        while self.n <= 56 {
            let Some(&b) = self.d.get(self.pos) else {
                return;
            };
            self.buf |= (b as u64) << self.n;
            self.pos += 1;
            self.n += 8;
        }
    }

    #[inline]
    fn get(&mut self, k: u32) -> Result<u32, InflateError> {
        if self.n < k {
            self.refill();
            if self.n < k {
                return Err(InflateError::Truncated);
            }
        }
        let v = (self.buf & ((1u64 << k) - 1)) as u32;
        self.buf >>= k;
        self.n -= k;
        Ok(v)
    }

    fn align(&mut self) {
        let r = self.n % 8;
        self.buf >>= r;
        self.n -= r;
    }

    /// Bytes taken so far: those read less the whole bytes still buffered.
    fn used(&self) -> usize {
        self.pos - (self.n / 8) as usize
    }
}

const FAST: u32 = 9;

struct Huff {
    /// By the next FAST bits: symbol << 4 | length, or 0 where the code is longer.
    fast: Vec<u16>,
    count: [u16; 16],
    sym: Vec<u16>,
}

impl Huff {
    fn new(lens: &[u8]) -> Result<Huff, InflateError> {
        let mut count = [0u16; 16];
        for &l in lens {
            count[l as usize] += 1;
        }
        count[0] = 0;
        let mut left = 1i32;
        for c in &count[1..] {
            left = (left << 1) - *c as i32;
            if left < 0 {
                return Err(InflateError::Code);
            }
        }
        let mut offs = [0u16; 16];
        for l in 1..15 {
            offs[l + 1] = offs[l] + count[l];
        }
        let mut sym = vec![0u16; lens.len()];
        for (s, &l) in lens.iter().enumerate() {
            if l != 0 {
                sym[offs[l as usize] as usize] = s as u16;
                offs[l as usize] += 1;
            }
        }
        let mut fast = vec![0u16; 1 << FAST];
        let mut next = [0u32; 16];
        let mut code = 0u32;
        for l in 1..16 {
            code = (code + count[l - 1] as u32) << 1;
            next[l] = code;
        }
        for (s, &l) in lens.iter().enumerate() {
            if l == 0 {
                continue;
            }
            let c = next[l as usize];
            next[l as usize] += 1;
            if l as u32 > FAST {
                continue;
            }
            let r = reverse(c, l as u32);
            let mut i = r;
            while i < (1 << FAST) {
                fast[i as usize] = ((s as u16) << 4) | l as u16;
                i += 1 << l;
            }
        }
        Ok(Huff { fast, count, sym })
    }

    #[inline]
    fn decode(&self, b: &mut Bits) -> Result<u16, InflateError> {
        if b.n < 15 {
            b.refill();
        }
        let e = self.fast[(b.buf & ((1 << FAST) - 1)) as usize];
        if e != 0 {
            let l = (e & 15) as u32;
            if l > b.n {
                return Err(InflateError::Truncated);
            }
            b.buf >>= l;
            b.n -= l;
            return Ok(e >> 4);
        }
        let (mut code, mut first, mut index) = (0i32, 0i32, 0i32);
        for l in 1..16u32 {
            if l > b.n {
                return Err(InflateError::Truncated);
            }
            code |= ((b.buf >> (l - 1)) & 1) as i32;
            let count = self.count[l as usize] as i32;
            if code - count < first {
                b.buf >>= l;
                b.n -= l;
                return Ok(self.sym[(index + code - first) as usize]);
            }
            index += count;
            first = (first + count) << 1;
            code <<= 1;
        }
        Err(InflateError::Code)
    }
}

fn reverse(mut c: u32, l: u32) -> u32 {
    let mut r = 0;
    for _ in 0..l {
        r = (r << 1) | (c & 1);
        c >>= 1;
    }
    r
}

const LBASE: [u16; 29] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131, 163, 195,
    227, 258,
];
const LEXT: [u8; 29] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
];
const DBASE: [u16; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537, 2049, 3073,
    4097, 6145, 8193, 12289, 16385, 24577,
];
const DEXT: [u8; 30] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13, 13,
];
const CL_ORDER: [usize; 19] = [16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15];

fn fixed_lens() -> ([u8; 288], [u8; 30]) {
    let mut l = [0u8; 288];
    for (i, x) in l.iter_mut().enumerate() {
        *x = match i {
            0..=143 => 8,
            144..=255 => 9,
            256..=279 => 7,
            _ => 8,
        };
    }
    (l, [5; 30])
}

/// Appends a raw DEFLATE stream's output to `out`; the bytes of `data` it took.
pub fn inflate_into(data: &[u8], cap: usize, out: &mut Vec<u8>) -> Result<usize, InflateError> {
    let start = out.len();
    let mut b = Bits {
        d: data,
        pos: 0,
        buf: 0,
        n: 0,
    };
    loop {
        let last = b.get(1)?;
        match b.get(2)? {
            0 => {
                b.align();
                let len = b.get(16)?;
                let nlen = b.get(16)?;
                if len != !nlen & 0xffff {
                    return Err(InflateError::Block);
                }
                if out.len() - start + len as usize > cap {
                    return Err(InflateError::TooBig);
                }
                // whole bytes still buffered go first, then straight from the input
                let mut len = len as usize;
                while len > 0 && b.n >= 8 {
                    out.push(b.get(8)? as u8);
                    len -= 1;
                }
                let src = data.get(b.pos..b.pos + len).ok_or(InflateError::Truncated)?;
                out.extend_from_slice(src);
                b.pos += len;
            }
            1 => {
                let (l, d) = fixed_lens();
                codes(&mut b, &Huff::new(&l)?, &Huff::new(&d)?, out, start, cap)?;
            }
            2 => {
                let hlit = b.get(5)? as usize + 257;
                let hdist = b.get(5)? as usize + 1;
                let hclen = b.get(4)? as usize + 4;
                if hlit > 286 || hdist > 30 {
                    return Err(InflateError::Code);
                }
                let mut cl = [0u8; 19];
                for &i in &CL_ORDER[..hclen] {
                    cl[i] = b.get(3)? as u8;
                }
                let clh = Huff::new(&cl)?;
                let mut lens = vec![0u8; hlit + hdist];
                let mut i = 0;
                while i < lens.len() {
                    let s = clh.decode(&mut b)?;
                    let (v, rep) = match s {
                        0..=15 => (s as u8, 1),
                        16 => {
                            if i == 0 {
                                return Err(InflateError::Code);
                            }
                            (lens[i - 1], 3 + b.get(2)? as usize)
                        }
                        17 => (0, 3 + b.get(3)? as usize),
                        _ => (0, 11 + b.get(7)? as usize),
                    };
                    if i + rep > lens.len() {
                        return Err(InflateError::Code);
                    }
                    lens[i..i + rep].fill(v);
                    i += rep;
                }
                if lens[256] == 0 {
                    return Err(InflateError::Code);
                }
                let lh = Huff::new(&lens[..hlit])?;
                let dh = Huff::new(&lens[hlit..])?;
                codes(&mut b, &lh, &dh, out, start, cap)?;
            }
            _ => return Err(InflateError::Block),
        }
        if last == 1 {
            b.align();
            return Ok(b.used());
        }
    }
}

fn codes(
    b: &mut Bits,
    lit: &Huff,
    dist: &Huff,
    out: &mut Vec<u8>,
    start: usize,
    cap: usize,
) -> Result<(), InflateError> {
    loop {
        let s = lit.decode(b)?;
        if s < 256 {
            if out.len() - start >= cap {
                return Err(InflateError::TooBig);
            }
            out.push(s as u8);
            continue;
        }
        if s == 256 {
            return Ok(());
        }
        let i = (s - 257) as usize;
        if i >= 29 {
            return Err(InflateError::Code);
        }
        let len = LBASE[i] as usize + b.get(LEXT[i] as u32)? as usize;
        let ds = dist.decode(b)? as usize;
        if ds >= 30 {
            return Err(InflateError::Code);
        }
        let d = DBASE[ds] as usize + b.get(DEXT[ds] as u32)? as usize;
        if d > out.len() - start {
            return Err(InflateError::Distance);
        }
        if out.len() - start + len > cap {
            return Err(InflateError::TooBig);
        }
        let from = out.len() - d;
        if d >= len {
            out.extend_from_within(from..from + len);
        } else {
            for k in 0..len {
                let c = out[from + k];
                out.push(c);
            }
        }
    }
}

// ---- deflate

struct BitOut {
    out: Vec<u8>,
    acc: u64,
    n: u32,
}

impl BitOut {
    #[inline]
    fn put(&mut self, v: u32, k: u32) {
        self.acc |= (v as u64) << self.n;
        self.n += k;
        while self.n >= 8 {
            self.out.push(self.acc as u8);
            self.acc >>= 8;
            self.n -= 8;
        }
    }

    fn flush_byte(&mut self) {
        if self.n > 0 {
            self.out.push(self.acc as u8);
            self.acc = 0;
            self.n = 0;
        }
    }
}

#[derive(Clone, Copy)]
struct Sym {
    /// A literal byte, or a match's length (3..=258) when `dist` is not 0.
    v: u16,
    dist: u16,
}

const WIN: usize = 32768;
const HBITS: u32 = 15;
const CHAIN: usize = 48;
const NICE: usize = 128;
const BLOCK_SYMS: usize = 32768;

/// `data` as a zlib stream.
pub fn deflate_zlib(data: &[u8]) -> Vec<u8> {
    let mut o = BitOut {
        out: vec![0x78, 0x9c],
        acc: 0,
        n: 0,
    };
    deflate_raw(data, &mut o);
    o.out.extend_from_slice(&adler32(data).to_be_bytes());
    o.out
}

/// `data` as a raw DEFLATE stream (no zlib wrapper: zip members, gzip bodies).
pub fn deflate(data: &[u8]) -> Vec<u8> {
    let mut o = BitOut {
        out: vec![],
        acc: 0,
        n: 0,
    };
    deflate_raw(data, &mut o);
    o.out
}

/// A raw DEFLATE stream's output, at most `cap` bytes.
pub fn inflate(data: &[u8], cap: usize) -> Result<Vec<u8>, InflateError> {
    let mut out = Vec::new();
    inflate_into(data, cap, &mut out)?;
    Ok(out)
}

fn hash3(d: &[u8], i: usize) -> usize {
    let v = (d[i] as u32) | ((d[i + 1] as u32) << 8) | ((d[i + 2] as u32) << 16);
    (v.wrapping_mul(0x9E37_79B1) >> (32 - HBITS)) as usize
}

fn deflate_raw(data: &[u8], o: &mut BitOut) {
    let mut head = vec![u32::MAX; 1 << HBITS];
    let mut prev = vec![u32::MAX; WIN];
    let mut syms: Vec<Sym> = Vec::with_capacity(BLOCK_SYMS);
    let mut block_start = 0;
    let mut i = 0;
    let insert = |head: &mut Vec<u32>, prev: &mut Vec<u32>, i: usize| {
        if i + 2 < data.len() {
            let h = hash3(data, i);
            prev[i % WIN] = head[h];
            head[h] = i as u32;
        }
    };
    while i < data.len() {
        let mut best = 0;
        let mut best_d = 0;
        if i + 2 < data.len() {
            let h = hash3(data, i);
            let mut c = head[h];
            let max = (data.len() - i).min(258);
            let mut steps = 0;
            while c != u32::MAX && steps < CHAIN {
                let j = c as usize;
                if i - j > WIN - 1 || j >= i {
                    break;
                }
                if data[j + best] == data[i + best] || best == 0 {
                    let mut l = 0;
                    while l < max && data[j + l] == data[i + l] {
                        l += 1;
                    }
                    if l > best {
                        best = l;
                        best_d = i - j;
                        if l >= NICE.min(max) {
                            break;
                        }
                    }
                }
                let nx = prev[j % WIN];
                if nx != u32::MAX && nx as usize >= j {
                    break;
                }
                c = nx;
                steps += 1;
            }
        }
        if best >= 3 {
            syms.push(Sym {
                v: best as u16,
                dist: best_d as u16,
            });
            for k in 0..best {
                insert(&mut head, &mut prev, i + k);
            }
            i += best;
        } else {
            syms.push(Sym {
                v: data[i] as u16,
                dist: 0,
            });
            insert(&mut head, &mut prev, i);
            i += 1;
        }
        if syms.len() >= BLOCK_SYMS {
            write_block(o, &syms, &data[block_start..i], false);
            syms.clear();
            block_start = i;
        }
    }
    write_block(o, &syms, &data[block_start..], true);
    o.flush_byte();
}

fn len_code(l: usize) -> usize {
    let mut i = 28;
    while LBASE[i] as usize > l {
        i -= 1;
    }
    i
}

fn dist_code(d: usize) -> usize {
    let mut i = 29;
    while DBASE[i] as usize > d {
        i -= 1;
    }
    i
}

/// Code lengths of a Huffman code over `freq`, none past `limit` bits; a used symbol gets at
/// least one bit and an unused one none.
fn huff_lengths(freq: &[u32], limit: u32) -> Vec<u8> {
    let mut f: Vec<u64> = freq.iter().map(|&x| x as u64).collect();
    loop {
        let lens = huff_once(&f);
        if lens.iter().all(|&l| l as u32 <= limit) {
            return lens;
        }
        for x in f.iter_mut() {
            if *x > 0 {
                *x = (*x >> 1) | 1;
            }
        }
    }
}

fn huff_once(f: &[u64]) -> Vec<u8> {
    use std::cmp::Reverse;
    use std::collections::BinaryHeap;
    let mut lens = vec![0u8; f.len()];
    let used: Vec<usize> = (0..f.len()).filter(|&i| f[i] > 0).collect();
    if used.len() == 1 {
        lens[used[0]] = 1;
        return lens;
    }
    let mut parent: Vec<usize> = vec![usize::MAX; used.len()];
    let mut heap = BinaryHeap::new();
    for (k, &i) in used.iter().enumerate() {
        heap.push(Reverse((f[i], k)));
    }
    while heap.len() > 1 {
        let Reverse((wa, a)) = heap.pop().expect("two");
        let Reverse((wb, b)) = heap.pop().expect("two");
        let n = parent.len();
        parent.push(usize::MAX);
        parent[a] = n;
        parent[b] = n;
        heap.push(Reverse((wa + wb, n)));
    }
    // depth of node n is depth of its parent + 1; parents come after children
    let mut depth = vec![0u8; parent.len()];
    for n in (0..parent.len()).rev() {
        if parent[n] != usize::MAX {
            depth[n] = depth[parent[n]].saturating_add(1);
        }
    }
    for (k, &i) in used.iter().enumerate() {
        lens[i] = depth[k];
    }
    lens
}

/// Canonical codes for `lens`, bit-reversed for writing low bit first.
fn codes_of(lens: &[u8]) -> Vec<u16> {
    let mut count = [0u32; 16];
    for &l in lens {
        count[l as usize] += 1;
    }
    count[0] = 0;
    let mut next = [0u32; 16];
    let mut code = 0;
    for l in 1..16 {
        code = (code + count[l - 1]) << 1;
        next[l] = code;
    }
    lens.iter()
        .map(|&l| {
            if l == 0 {
                return 0;
            }
            let c = next[l as usize];
            next[l as usize] += 1;
            reverse(c, l as u32) as u16
        })
        .collect()
}

/// The code-length alphabet's run-length coding of `lens`: (symbol, extra bits value).
fn rle(lens: &[u8]) -> Vec<(u8, u8)> {
    let mut out = vec![];
    let mut i = 0;
    while i < lens.len() {
        let v = lens[i];
        let mut run = 1;
        while i + run < lens.len() && lens[i + run] == v {
            run += 1;
        }
        let mut left = run;
        if v == 0 {
            while left >= 11 {
                let k = left.min(138);
                out.push((18, (k - 11) as u8));
                left -= k;
            }
            if left >= 3 {
                out.push((17, (left - 3) as u8));
                left = 0;
            }
        } else {
            out.push((v, 0));
            left -= 1;
            while left >= 3 {
                let k = left.min(6);
                out.push((16, (k - 3) as u8));
                left -= k;
            }
        }
        for _ in 0..left {
            out.push((v, 0));
        }
        i += run;
    }
    out
}

fn write_block(o: &mut BitOut, syms: &[Sym], raw: &[u8], last: bool) {
    let mut lf = vec![0u32; 286];
    let mut df = vec![0u32; 30];
    let mut extra_bits = 0u64;
    for s in syms {
        if s.dist == 0 {
            lf[s.v as usize] += 1;
        } else {
            let lc = len_code(s.v as usize);
            let dc = dist_code(s.dist as usize);
            lf[257 + lc] += 1;
            df[dc] += 1;
            extra_bits += LEXT[lc] as u64 + DEXT[dc] as u64;
        }
    }
    lf[256] = 1;
    let mut ll = huff_lengths(&lf, 15);
    let mut dl = huff_lengths(&df, 15);
    // complete codes, as zlib makes them: two symbols at least in each
    for l in [&mut ll, &mut dl] {
        let used = l.iter().filter(|&&x| x > 0).count();
        if used < 2 {
            let free: Vec<usize> = (0..l.len()).filter(|&i| l[i] == 0).take(2 - used).collect();
            for i in free {
                l[i] = 1;
            }
            for x in l.iter_mut() {
                if *x > 0 {
                    *x = 1;
                }
            }
        }
    }
    let hlit = (257..=286)
        .rev()
        .find(|&n| ll[n - 1] != 0)
        .unwrap_or(257)
        .max(257);
    let hdist = (1..=30).rev().find(|&n| dl[n - 1] != 0).unwrap_or(1).max(1);
    let mut all = ll[..hlit].to_vec();
    all.extend_from_slice(&dl[..hdist]);
    let runs = rle(&all);
    let mut cf = vec![0u32; 19];
    for r in &runs {
        cf[r.0 as usize] += 1;
    }
    let cl = huff_lengths(&cf, 7);
    let hclen = (4..=19).rev().find(|&n| cl[CL_ORDER[n - 1]] != 0).unwrap_or(4);
    let data_bits = |l: &[u8], d: &[u8]| -> u64 {
        let mut b = extra_bits;
        for (i, &f) in lf.iter().enumerate() {
            b += f as u64 * l[i] as u64;
        }
        for (i, &f) in df.iter().enumerate() {
            b += f as u64 * d[i] as u64;
        }
        b
    };
    let mut dyn_bits = 14 + 3 * hclen as u64 + data_bits(&ll, &dl);
    for r in &runs {
        dyn_bits += cl[r.0 as usize] as u64
            + match r.0 {
                16 => 2,
                17 => 3,
                18 => 7,
                _ => 0,
            };
    }
    let (fl, fd) = fixed_lens();
    let fixed_bits = data_bits(&fl[..286], &fd);
    let stored_bits = (raw.len() as u64 + 5 * raw.len().div_ceil(65535).max(1) as u64) * 8 + 7;
    let fin = last as u32;
    if stored_bits <= dyn_bits.min(fixed_bits) {
        let chunks: Vec<&[u8]> = if raw.is_empty() {
            vec![&[]]
        } else {
            raw.chunks(65535).collect()
        };
        let n = chunks.len();
        for (k, c) in chunks.into_iter().enumerate() {
            o.put(if k + 1 == n { fin } else { 0 }, 1);
            o.put(0, 2);
            o.flush_byte();
            o.put(c.len() as u32, 16);
            o.put(!(c.len() as u32) & 0xffff, 16);
            o.out.extend_from_slice(c);
        }
        return;
    }
    o.put(fin, 1);
    let (lcodes, dcodes, ll, dl) = if fixed_bits <= dyn_bits {
        o.put(1, 2);
        (codes_of(&fl), codes_of(&fd), fl[..].to_vec(), fd.to_vec())
    } else {
        o.put(2, 2);
        o.put((hlit - 257) as u32, 5);
        o.put((hdist - 1) as u32, 5);
        o.put((hclen - 4) as u32, 4);
        for &i in &CL_ORDER[..hclen] {
            o.put(cl[i] as u32, 3);
        }
        let ccodes = codes_of(&cl);
        for r in &runs {
            o.put(ccodes[r.0 as usize] as u32, cl[r.0 as usize] as u32);
            match r.0 {
                16 => o.put(r.1 as u32, 2),
                17 => o.put(r.1 as u32, 3),
                18 => o.put(r.1 as u32, 7),
                _ => {}
            }
        }
        (codes_of(&ll), codes_of(&dl), ll, dl)
    };
    for s in syms {
        if s.dist == 0 {
            o.put(lcodes[s.v as usize] as u32, ll[s.v as usize] as u32);
        } else {
            let lc = len_code(s.v as usize);
            o.put(lcodes[257 + lc] as u32, ll[257 + lc] as u32);
            o.put((s.v - LBASE[lc]) as u32, LEXT[lc] as u32);
            let dc = dist_code(s.dist as usize);
            o.put(dcodes[dc] as u32, dl[dc] as u32);
            o.put((s.dist - DBASE[dc]) as u32, DEXT[dc] as u32);
        }
    }
    o.put(lcodes[256] as u32, ll[256] as u32);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rng(seed: &mut u64) -> u64 {
        *seed ^= *seed << 13;
        *seed ^= *seed >> 7;
        *seed ^= *seed << 17;
        *seed
    }

    fn round(d: &[u8]) {
        let z = deflate_zlib(d);
        let (back, used) = inflate_zlib(&z, d.len()).expect("inflate");
        assert_eq!(used, z.len());
        assert_eq!(back, d);
    }

    #[test]
    fn round_trips() {
        round(b"");
        round(b"a");
        round(b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        round(b"tree 1234\0hello hello hello world");
        let mut s = 7u64;
        for n in [10, 300, 5000, 70000, 200_000] {
            let rand: Vec<u8> = (0..n).map(|_| rng(&mut s) as u8).collect();
            round(&rand);
            let text: Vec<u8> = (0..n)
                .map(|_| b"abcd efg\n"[(rng(&mut s) % 9) as usize])
                .collect();
            round(&text);
        }
    }

    #[test]
    fn compresses_text() {
        let src = include_bytes!("zlib.rs");
        let z = deflate_zlib(src);
        assert!(z.len() * 3 < src.len(), "{} of {}", z.len(), src.len());
    }

    #[test]
    fn refuses_bad_streams() {
        let z = deflate_zlib(b"hello hello hello");
        assert_eq!(inflate_zlib(&z, 5).unwrap_err(), InflateError::TooBig);
        assert_eq!(
            inflate_zlib(&z[..z.len() - 6], 100).unwrap_err(),
            InflateError::Truncated
        );
        let mut bad = z.clone();
        let n = bad.len();
        bad[n - 1] ^= 1;
        assert_eq!(inflate_zlib(&bad, 100).unwrap_err(), InflateError::Checksum);
        assert_eq!(
            inflate_zlib(&[0x78, 0x9d], 100).unwrap_err(),
            InflateError::Header
        );
        // a fixed block reaching back past its start
        let mut s = 1u64;
        for _ in 0..2000 {
            let n = (rng(&mut s) % 40) as usize;
            let junk: Vec<u8> = (0..n).map(|_| rng(&mut s) as u8).collect();
            let _ = inflate_into(&junk, 1 << 16, &mut vec![]);
        }
    }
}
