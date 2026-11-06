//! Lists an R2004-style or R2007 file's sections and checks each decompresses. `dwgsections FILE..`
use forge_dwg::file::{r2004, r2007};

fn main() {
    for path in std::env::args().skip(1) {
        let buf = std::fs::read(&path).unwrap();
        println!("== {path} {}", String::from_utf8_lossy(&buf[0..6]));
        if &buf[0..6] == b"AC1021" {
            r2007_sections(&buf);
            continue;
        }
        let l = match r2004::layout(&buf) {
            Ok(l) => l,
            Err(e) => {
                println!("  layout: {}", e.0);
                continue;
            }
        };
        println!(
            "  {} pages, page map {} at {:#x}, section map {}",
            l.pages.len(),
            l.header.page_map_id,
            l.header.page_map_addr,
            l.header.section_map_id
        );
        for s in &l.sections {
            let d = r2004::section_data(&buf, &l, &s.name);
            println!(
                "  {:22} size {:7} pages {:2} max {:#x} comp {} enc {} id {} -> {}",
                s.name,
                s.size,
                s.pages.len(),
                s.max_page,
                s.compressed,
                s.encrypted,
                s.id,
                match d {
                    Ok(Some(d)) => format!("{} bytes, starts {:02x?}", d.len(), &d[..d.len().min(16)]),
                    Ok(None) => "none".into(),
                    Err(e) => e.0,
                }
            );
        }
    }
}

fn r2007_sections(buf: &[u8]) {
    let l = match r2007::layout(buf) {
        Ok(l) => l,
        Err(e) => {
            println!("  layout: {}", e.0);
            return;
        }
    };
    let h = &l.header;
    println!(
        "  {} pages, page map {} at {:#x}, section map {}, crc seed {:#x}, random seed {:#x}",
        l.pages.len(),
        h.pages_map_id,
        r2007::PAGES_START + h.pages_map_offset,
        h.sections_map_id,
        h.crc_seed,
        h.random_seed
    );
    for s in &l.sections {
        let d = r2007::section_data(buf, &l, &s.name);
        println!(
            "  {:22} size {:7} pages {:2} max {:#x} enc {} encoding {} hash {:#x} -> {}",
            s.name,
            s.size,
            s.pages.len(),
            s.max_size,
            s.encryption,
            s.encoding,
            s.hash,
            match d {
                Ok(Some(d)) => format!("{} bytes, starts {:02x?}", d.len(), &d[..d.len().min(16)]),
                Ok(None) => "none".into(),
                Err(e) => e.0,
            }
        );
    }
}
