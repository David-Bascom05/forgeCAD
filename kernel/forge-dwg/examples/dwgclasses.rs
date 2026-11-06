//! Walks an R2007+ classes section class by class. `dwgclasses SECTION.bin VERSION_MAGIC`
use forge_dwg::bits::BitReader;
use forge_dwg::file::{Class, r2000};
use forge_dwg::io::{Io, Reader};
use forge_dwg::version::Version;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let sec = std::fs::read(&a[1]).unwrap();
    let ver = Version::from_magic(a[2].as_bytes()).unwrap();
    let size = u32::from_le_bytes(sec[16..20].try_into().unwrap()) as usize;
    let data = &sec[24..24 + size];
    let bits = u32::from_le_bytes(data[0..4].try_into().unwrap()) as usize;
    let (strs, end) = r2000::string_stream(data, bits).unwrap();
    let mut r = Reader { ver, data: BitReader::new(data, 4), strs, hdl: BitReader::new(data, 0), base: 0, data_end: end };
    let (mut max, mut z, mut t) = (0i16, 0u8, false);
    r.bs(&mut max).unwrap();
    r.rc(&mut z).unwrap();
    r.rc(&mut z).unwrap();
    r.b(&mut t).unwrap();
    println!("max {max} end {end}");
    let mut n = 0;
    while r.data.p + 18 < end {
        let p = r.data.p;
        let mut c = Class::default();
        match r2000::class_io(&mut c, &mut r) {
            Ok(()) => {
                n += 1;
                if n < 3 || n % 50 == 0 {
                    println!("{n} at {p}: {} {} {}", c.number, c.dxf.text(), c.count);
                }
            }
            Err(e) => {
                println!("{n} at {p}: {} (last ok above); strs at {:?}", e.0, r.strs.as_ref().map(|s| s.p));
                break;
            }
        }
    }
}
