//! Indexed-colour PNG, deflated with LZ77 and the fixed Huffman code.

pub fn encode(w: usize, h: usize, palette: &[[u8; 3]], px: &[u8]) -> Vec<u8> {
    let mut raw = Vec::with_capacity((w + 1) * h);
    for row in px.chunks(w) {
        raw.push(0);
        raw.extend_from_slice(row);
    }
    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut ihdr = Vec::new();
    ihdr.extend((w as u32).to_be_bytes());
    ihdr.extend((h as u32).to_be_bytes());
    ihdr.extend([8, 3, 0, 0, 0]);
    chunk(&mut out, b"IHDR", &ihdr);
    chunk(&mut out, b"PLTE", &palette.concat());
    chunk(&mut out, b"IDAT", &zlib(&raw));
    chunk(&mut out, b"IEND", &[]);
    out
}

fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend((data.len() as u32).to_be_bytes());
    let start = out.len();
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    let crc = crc32(&out[start..]);
    out.extend(crc.to_be_bytes());
}

fn crc32(b: &[u8]) -> u32 {
    let mut c = !0u32;
    for &x in b {
        c ^= x as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 {
                0xedb8_8320 ^ (c >> 1)
            } else {
                c >> 1
            };
        }
    }
    !c
}

struct Bits {
    out: Vec<u8>,
    acc: u32,
    n: u32,
}

impl Bits {
    /// `n` bits of `v`, least significant first.
    fn put(&mut self, v: u32, n: u32) {
        self.acc |= v << self.n;
        self.n += n;
        while self.n >= 8 {
            self.out.push(self.acc as u8);
            self.acc >>= 8;
            self.n -= 8;
        }
    }
    /// A Huffman code, most significant bit first.
    fn code(&mut self, c: u32, n: u32) {
        self.put(c.reverse_bits() >> (32 - n), n);
    }
    fn lit(&mut self, v: u32) {
        match v {
            0..=143 => self.code(0x30 + v, 8),
            144..=255 => self.code(0x190 + v - 144, 9),
            256..=279 => self.code(v - 256, 7),
            _ => self.code(0xc0 + v - 280, 8),
        }
    }
}

const LEN_BASE: [u16; 29] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131, 163, 195,
    227, 258,
];
const LEN_EXTRA: [u8; 29] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
];
const DIST_BASE: [u16; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537, 2049, 3073,
    4097, 6145, 8193, 12289, 16385, 24577,
];
const DIST_EXTRA: [u8; 30] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13, 13,
];

fn zlib(data: &[u8]) -> Vec<u8> {
    let mut b = Bits {
        out: vec![0x78, 0x01],
        acc: 0,
        n: 0,
    };
    b.put(1, 1); // final block
    b.put(1, 2); // fixed Huffman
    const WINDOW: usize = 32768;
    let mut head = vec![usize::MAX; 1 << 15];
    let mut prev = vec![usize::MAX; data.len()];
    let hash =
        |k: usize| ((data[k] as usize) << 10 ^ (data[k + 1] as usize) << 5 ^ data[k + 2] as usize) & 0x7fff;
    let mut k = 0;
    let insert = |k: usize, head: &mut Vec<usize>, prev: &mut Vec<usize>| {
        if k + 2 < data.len() {
            let h = hash(k);
            prev[k] = head[h];
            head[h] = k;
        }
    };
    while k < data.len() {
        let (mut best, mut at) = (0, 0);
        if k + 2 < data.len() {
            let mut c = head[hash(k)];
            let mut tries = 64;
            while c != usize::MAX && k - c <= WINDOW && tries > 0 {
                let n = data[c..]
                    .iter()
                    .zip(&data[k..])
                    .take(258)
                    .take_while(|(a, b)| a == b)
                    .count();
                if n > best {
                    (best, at) = (n, k - c);
                    if n == 258 {
                        break;
                    }
                }
                c = prev[c];
                tries -= 1;
            }
        }
        if best >= 3 {
            let li = LEN_BASE
                .iter()
                .rposition(|&l| l as usize <= best)
                .expect("length >= 3");
            b.lit(257 + li as u32);
            b.put((best - LEN_BASE[li] as usize) as u32, LEN_EXTRA[li] as u32);
            let di = DIST_BASE
                .iter()
                .rposition(|&d| d as usize <= at)
                .expect("distance >= 1");
            b.code(di as u32, 5);
            b.put((at - DIST_BASE[di] as usize) as u32, DIST_EXTRA[di] as u32);
            for j in k..k + best {
                insert(j, &mut head, &mut prev);
            }
            k += best;
        } else {
            b.lit(data[k] as u32);
            insert(k, &mut head, &mut prev);
            k += 1;
        }
    }
    b.lit(256);
    b.put(0, 7);
    let (mut s1, mut s2) = (1u32, 0u32);
    for &x in data {
        s1 = (s1 + x as u32) % 65521;
        s2 = (s2 + s1) % 65521;
    }
    let mut out = b.out;
    out.extend(((s2 << 16) | s1).to_be_bytes());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Inflates a fixed-Huffman stream (what `zlib` writes), to check it.
    fn inflate(z: &[u8]) -> Vec<u8> {
        let mut pos = 16usize; // past the zlib header, in bits
        let mut bit = |n: u32| -> u32 {
            let mut v = 0;
            for i in 0..n {
                v |= ((z[pos / 8] >> (pos % 8)) as u32 & 1) << i;
                pos += 1;
            }
            v
        };
        assert_eq!(bit(3), 0b011);
        let mut out: Vec<u8> = Vec::new();
        loop {
            let mut code = 0u32;
            let mut len = 0;
            let sym = loop {
                code = code << 1 | bit(1);
                len += 1;
                match (len, code) {
                    (7, 0..=23) => break 256 + code,
                    (8, 0x30..=0xbf) => break code - 0x30,
                    (8, 0xc0..=0xc7) => break 280 + code - 0xc0,
                    (9, 0x190..=0x1ff) => break 144 + code - 0x190,
                    _ => {}
                }
            };
            match sym {
                0..=255 => out.push(sym as u8),
                256 => return out,
                _ => {
                    let li = (sym - 257) as usize;
                    let n = LEN_BASE[li] as usize + bit(LEN_EXTRA[li] as u32) as usize;
                    let mut d = 0;
                    for _ in 0..5 {
                        d = d << 1 | bit(1);
                    }
                    let dist = DIST_BASE[d as usize] as usize + bit(DIST_EXTRA[d as usize] as u32) as usize;
                    for _ in 0..n {
                        out.push(out[out.len() - dist]);
                    }
                }
            }
        }
    }

    #[test]
    fn deflates_and_inflates() {
        let mut r = forge_math::rng::Rng::new(5);
        let mut data: Vec<u8> = (0..20000).map(|i| ((i / 37) % 7) as u8).collect();
        data.extend((0..3000).map(|_| r.below(256) as u8));
        data.extend(std::iter::repeat_n(9, 1000));
        let z = zlib(&data);
        assert!(z.len() < data.len() / 3, "{} from {}", z.len(), data.len());
        assert_eq!(inflate(&z), data);
        assert_eq!(crc32(b"IEND"), 0xae42_6082);
    }
}
