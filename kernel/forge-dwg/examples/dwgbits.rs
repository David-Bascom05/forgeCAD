//! Prints an object's data-stream bits after its handle, and its string stream. `dwgbits FILE HANDLE`
use forge_dwg::bits::{BitReader, read_mc, read_ms};
use forge_dwg::file::{r2000, sections};
use forge_dwg::version::Version;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let buf = std::fs::read(&a[1]).unwrap();
    let want = u32::from_str_radix(a[2].trim_start_matches("0x"), 16).unwrap();
    let s = sections(&buf).unwrap();
    let map = r2000::read_map(&s.handles, 0, s.handles.len()).unwrap();
    let at = map.iter().find(|m| m.0 == want).unwrap().1;
    let b = &s.objects;
    let (size, mut start) = read_ms(b, at).unwrap();
    let mut hsize = 0;
    if s.ver >= Version::R2010 {
        let (h, q) = read_mc(b, start, false).unwrap();
        hsize = h as usize;
        start = q;
    }
    let obj = &b[start..start + size as usize];
    let mut r = BitReader::new(obj, 0);
    let t = if s.ver >= Version::R2010 {
        match r.bb().unwrap() {
            0 => r.rc().unwrap() as u16,
            1 => r.rc().unwrap() as u16 + 0x1F0,
            _ => r.rs().unwrap(),
        }
    } else {
        r.bs().unwrap() as u16
    };
    let endbit = if s.ver >= Version::R2010 {
        size as usize * 8 - hsize
    } else {
        r.rl().unwrap() as usize
    };
    let h = r.h().unwrap();
    println!(
        "type {t} handle {h:?} size {size} endbit {endbit} hsize {hsize} data from bit {}",
        r.p
    );
    let bits: String = (r.p..endbit)
        .map(|k| {
            if obj[k / 8] & (0x80 >> (k % 8)) != 0 {
                '1'
            } else {
                '0'
            }
        })
        .collect();
    for (k, c) in bits.as_bytes().chunks(64).enumerate() {
        println!("{:5} {}", r.p + 64 * k, std::str::from_utf8(c).unwrap());
    }
}
