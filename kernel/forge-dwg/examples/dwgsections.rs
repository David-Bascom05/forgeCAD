//! Lists an R2004-style file's sections and checks each decompresses. `dwgsections FILE..`
use forge_dwg::file::r2004;

fn main() {
    for path in std::env::args().skip(1) {
        let buf = std::fs::read(&path).unwrap();
        println!("== {path} {}", String::from_utf8_lossy(&buf[0..6]));
        let l = match r2004::layout(&buf) {
            Ok(l) => l,
            Err(e) => {
                println!("  layout: {}", e.0);
                continue;
            }
        };
        println!("  {} pages, page map {} at {:#x}, section map {}", l.pages.len(), l.header.page_map_id, l.header.page_map_addr, l.header.section_map_id);
        for s in &l.sections {
            let d = r2004::section_data(&buf, &l, &s.name);
            println!(
                "  {:22} size {:7} pages {:2} max {:#x} comp {} enc {} id {} -> {}",
                s.name, s.size, s.pages.len(), s.max_page, s.compressed, s.encrypted, s.id,
                match d {
                    Ok(Some(d)) => format!("{} bytes, starts {:02x?}", d.len(), &d[..d.len().min(16)]),
                    Ok(None) => "none".into(),
                    Err(e) => e.0,
                }
            );
        }
    }
}
