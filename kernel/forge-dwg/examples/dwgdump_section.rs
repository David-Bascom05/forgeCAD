//! Writes one decompressed section of an R2004-style file to stdout. `dwgdump_section FILE NAME`
use forge_dwg::file::r2004;
use std::io::Write;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let buf = std::fs::read(&a[1]).unwrap();
    let l = r2004::layout(&buf).unwrap();
    let d = r2004::section_data(&buf, &l, &a[2]).unwrap().unwrap();
    std::io::stdout().write_all(&d).unwrap();
}
