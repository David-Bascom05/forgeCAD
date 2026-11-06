//! Reads each file's header variables, re-encodes them and compares. `dwgheader FILE..`
use forge_dwg::header::{encode_header, has_hi_size, read_header};

fn main() {
    for path in std::env::args().skip(1) {
        let buf = std::fs::read(&path).unwrap();
        let s = match forge_dwg::file::sections(&buf) {
            Ok(s) => s,
            Err(e) => {
                println!("== {path}: {}", e.0);
                continue;
            }
        };
        print!("== {path} {:?}: ", s.ver);
        match read_header(&s.header, s.ver, s.maint) {
            Ok(h) => {
                let (b, n) = encode_header(&h, s.ver).unwrap();
                let at = if has_hi_size(s.ver, s.maint) { 24 } else { 20 };
                let size = u32::from_le_bytes(s.header[16..20].try_into().unwrap()) as usize;
                let orig = &s.header[at..at + size];
                let same = orig.len() >= b.len()
                    && (0..n).all(|k| (orig[k / 8] ^ b[k / 8]) & (0x80 >> (k % 8)) == 0);
                println!(
                    "{} vars, re-encoded {} ({} vs {} bytes)",
                    h.vars.len(),
                    if same { "SAME" } else { "DIFFERENT" },
                    b.len(),
                    orig.len()
                );
                for name in [
                    "HANDSEED",
                    "EXTMIN",
                    "EXTMAX",
                    "INSUNITS",
                    "CLAYER",
                    "FINGERPRINTGUID",
                    "PROJECTNAME",
                    "unknown_bs4",
                ] {
                    if let Some(v) = h.get(name) {
                        println!("    {name} = {v:?}");
                    }
                }
            }
            Err(e) => println!("ERROR {}", e.0),
        }
    }
}
