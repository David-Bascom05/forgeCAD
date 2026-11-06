//! Prints an R2004-style file's header data, page map and section map. `dwglayout FILE`
use forge_dwg::file::r2004;

fn main() {
    let path = std::env::args().nth(1).unwrap();
    let buf = std::fs::read(&path).unwrap();
    println!("len {:#x}; first 0x80: {:02x?}", buf.len(), &buf[0..0x30]);
    let l = r2004::layout(&buf).unwrap();
    let h = &l.header;
    println!("header data: {:02x?}", &h.raw[0x0C..0x6C]);
    println!(
        "{h:#x?}",
        h = (
            h.last_page_id,
            h.last_page_end,
            h.second_header_addr,
            h.gap_amount,
            h.page_amount,
            h.page_map_id,
            h.page_map_addr,
            h.section_map_id,
            h.page_array_size,
            h.gap_array_size
        )
    );
    for p in &l.pages {
        println!("  page {:3} at {:#8x} size {:#x}", p.0, p.1, p.2);
    }
    for s in &l.sections {
        println!(
            "  section {:24} id {:2} size {:#x} max {:#x} unk {:#x} comp {} enc {} pages {:?}",
            s.name, s.id, s.size, s.max_page, s.unknown, s.compressed, s.encrypted, s.pages
        );
    }
    let sh = h.second_header_addr as usize;
    println!(
        "at second header addr {sh:#x}: {:02x?}",
        &buf[sh..(sh + 0x80).min(buf.len())]
    );
}
