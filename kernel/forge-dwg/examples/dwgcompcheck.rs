//! Compresses every page of every section of a file and checks it decompresses back; a page that
//! doesn't is written to BAD_OUT. `dwgcompcheck FILE [BAD_OUT]`
use forge_dwg::file::r2004;

fn main() {
    let path = std::env::args().nth(1).unwrap();
    let bad_out = std::env::args().nth(2).unwrap_or_else(|| "badpage.bin".into());
    let buf = std::fs::read(&path).unwrap();
    let l = r2004::layout(&buf).unwrap();
    for s in &l.sections {
        let Some(d) = r2004::section_data(&buf, &l, &s.name).unwrap() else { continue };
        for (k, chunk) in d.chunks(0x7400).enumerate() {
            let mut p = chunk.to_vec();
            p.resize(0x7400, 0);
            let c = r2004::compress(&p);
            match r2004::decompress(&c, 0x7400) {
                Ok(x) if x[..0x7400] == p[..] => {}
                Ok(x) => {
                    let at = x.iter().zip(&p).position(|(a, b)| a != b);
                    println!("{} page {k}: differs at {at:?} (len {})", s.name, x.len());
                    std::fs::write(&bad_out, &p).unwrap();
                    return;
                }
                Err(e) => {
                    println!("{} page {k}: {}", s.name, e.0);
                    std::fs::write(&bad_out, &p).unwrap();
                    return;
                }
            }
        }
    }
    println!("all pages round-trip");
}
