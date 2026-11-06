//! Compresses every page of every section of a file and checks it decompresses back. `dwgcompcheck FILE`
use forge_dwg::file::r2004;

fn main() {
    let path = std::env::args().nth(1).unwrap();
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
                    std::fs::write(format!("/tmp/claude-1000/-home-david-bascom/58bdb4d8-e033-4a46-9734-ef5f0f817aa9/scratchpad/badpage.bin"), &p).unwrap();
                    return;
                }
                Err(e) => {
                    println!("{} page {k}: {}", s.name, e.0);
                    std::fs::write(format!("/tmp/claude-1000/-home-david-bascom/58bdb4d8-e033-4a46-9734-ef5f0f817aa9/scratchpad/badpage.bin"), &p).unwrap();
                    return;
                }
            }
        }
    }
    println!("all pages round-trip");
}
