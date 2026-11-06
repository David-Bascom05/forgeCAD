//! Reads DWG files and reports, per object type, how many decode exactly and how many encode
//! back to the same bytes. `cargo run -p forge-dwg --example dwgcheck -- FILE..`

use forge_dwg::file::r2000;
use forge_dwg::object::{Classes, encode_object, read_object};
use forge_dwg::version::Version;
use std::collections::BTreeMap;

fn main() {
    for path in std::env::args().skip(1) {
        let buf = std::fs::read(&path).expect("read");
        let ver = Version::from_magic(&buf[0..6]);
        println!("== {path}: {ver:?}");
        if ver != Some(Version::R2000) {
            continue;
        }
        let d = match forge_dwg::file::r2000::read_shell(&buf) {
            Ok(d) => d,
            Err(e) => {
                println!("  container: {}", e.0);
                continue;
            }
        };
        let (map, list) = d;
        let cl = Classes { list: &list };
        // type -> (ok, same bytes, errors)
        let mut stats: BTreeMap<String, (usize, usize, Vec<String>)> = BTreeMap::new();
        for (h, at) in map {
            let name_of = |t: u16| cl.name(t).map(|s| s.to_string()).unwrap_or(format!("{t}"));
            match read_object(&buf, at, Version::R2000, &cl, None) {
                Ok(o) => {
                    let key = format!("{} ({})", name_of(o.type_code), o.data.name());
                    let e = stats.entry(key).or_default();
                    e.0 += 1;
                    let mut fresh = o.clone();
                    fresh.raw = None;
                    match encode_object(&fresh, Version::R2000) {
                        Ok((b, n)) if same_bits(&b, o.raw.as_ref().unwrap(), n) => e.1 += 1,
                        Ok((b, _)) => {
                            if e.2.len() < 2 {
                                e.2.push(format!("{h:#x}: re-encoded {} vs {} bytes", b.len(), o.raw.as_ref().unwrap().len()));
                            }
                        }
                        Err(x) => e.2.push(format!("{h:#x}: encode: {}", x.0)),
                    }
                }
                Err(x) => {
                    let t = r2000::peek_type(&buf, at).unwrap_or(0);
                    let e = stats.entry(format!("{} (ERR)", name_of(t))).or_default();
                    if e.2.len() < 3 {
                        e.2.push(format!("{h:#x}: {}", x.0));
                    }
                }
            }
        }
        for (k, (ok, same, errs)) in stats {
            println!("  {k:40} ok {ok:5} same {same:5}");
            for e in errs {
                println!("      {e}");
            }
        }
    }
}

/// The same length and the same first `n` bits (AutoCAD pads with whatever bits it had).
fn same_bits(a: &[u8], b: &[u8], n: usize) -> bool {
    a.len() == b.len() && (0..n).all(|k| (a[k / 8] ^ b[k / 8]) & (0x80 >> (k % 8)) == 0)
}
