//! Lists an object's handle stream as stored. `dwghandles FILE HANDLE`
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
    let endbit;
    if s.ver >= Version::R2010 {
        let (h, q) = read_mc(b, start, false).unwrap();
        endbit = size as usize * 8 - h as usize;
        start = q;
    } else {
        let mut r = BitReader::new(&b[start..], 0);
        r.bs().unwrap();
        endbit = r.rl().unwrap() as usize;
    }
    let obj = &b[start..start + size as usize];
    let mut r = BitReader::new(obj, 0);
    r.p = endbit;
    while obj.len() * 8 - r.p >= 8 {
        let p = r.p;
        match r.h() {
            Ok(h) => println!("bit {p}: {:x}.{:x}", h.0, h.1),
            Err(_) => break,
        }
    }
    println!("{} bits left", obj.len() * 8 - r.p);
}
