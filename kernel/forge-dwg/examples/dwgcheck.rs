//! Reads DWG files and reports, per object type, how many decode exactly and how many encode
//! back to the same bytes. `cargo run -p forge-dwg --example dwgcheck -- FILE..`

use forge_dwg::file::r2000;
use forge_dwg::object::{Classes, encode_object, read_object};
use std::collections::BTreeMap;

fn main() {
    for path in std::env::args().skip(1) {
        let buf = std::fs::read(&path).expect("read");
        let s = match forge_dwg::file::sections(&buf) {
            Ok(s) => s,
            Err(e) => {
                println!("== {path}: container: {}", e.0);
                continue;
            }
        };
        let ver = s.ver;
        println!("== {path}: {ver:?}");
        let classes = match r2000::classes(&s.classes, ver, s.maint) {
            Ok(c) => c,
            Err(e) => {
                println!("  classes: {}", e.0);
                continue;
            }
        };
        let list: Vec<(String, bool)> = classes.iter().map(|c| (c.dxf.text(), c.item == 0x1F2)).collect();
        let cl = Classes { list: &list };
        let map = match r2000::read_map(&s.handles, 0, s.handles.len()) {
            Ok(m) => m,
            Err(e) => {
                println!("  object map: {}", e.0);
                continue;
            }
        };
        let buf = &s.objects;
        // type -> (ok, same bytes, errors)
        let mut stats: BTreeMap<String, (usize, usize, Vec<String>)> = BTreeMap::new();
        for (h, at) in map {
            let name_of = |t: u16| cl.name(t).map(|s| s.to_string()).unwrap_or(format!("{t}"));
            match read_object(buf, at, ver, &cl, None) {
                Ok(o) => {
                    let key = format!("{} ({})", name_of(o.type_code), o.data.name());
                    let e = stats.entry(key).or_default();
                    e.0 += 1;
                    let mut fresh = o.clone();
                    fresh.raw = None;
                    match encode_object(&fresh, ver) {
                        Ok((b, n)) if same_bits(&b, o.raw.as_ref().unwrap(), n) => e.1 += 1,
                        Ok((b, _)) => {
                            if e.2.len() < 2 && !matches!(o.data, forge_dwg::types::Data::Raw(_)) {
                                e.2.push(format!("{h:#x}: re-encoded {} vs {} bytes", b.len(), o.raw.as_ref().unwrap().len()));
                            }
                        }
                        Err(x) => e.2.push(format!("{h:#x}: encode: {}", x.0)),
                    }
                }
                Err(x) => {
                    let t = r2000::peek_type(buf, at, ver).unwrap_or(0);
                    let e = stats.entry(format!("{} (ERR)", name_of(t))).or_default();
                    e.0 += 1;
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
