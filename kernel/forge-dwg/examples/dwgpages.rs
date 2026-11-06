//! Decompresses each page of an R2004-style file whole and reports its length. `dwgpages FILE`
use forge_dwg::file::r2004;

fn main() {
    let path = std::env::args().nth(1).unwrap();
    let buf = std::fs::read(&path).unwrap();
    let l = r2004::layout(&buf).unwrap();
    for s in &l.sections {
        for &(page, size, start) in &s.pages {
            let addr = l.pages.iter().find(|p| p.0 == page as i32).unwrap().1 as usize;
            let data = &buf[addr + 32..addr + 32 + size as usize];
            if s.compressed == 2 {
                let d = r2004::decompress(data, 0x10000).unwrap();
                println!("{:20} page {page:2} start {start:#7x} comp {size:5} -> {:5} bytes; tail {:02x?}", s.name, d.len(), &d[d.len().saturating_sub(8)..]);
            }
        }
    }
}
