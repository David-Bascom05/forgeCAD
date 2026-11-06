//! Compresses a file's bytes to stdout. `dwgcomp IN`
use std::io::Write;
fn main() {
    let p = std::fs::read(std::env::args().nth(1).unwrap()).unwrap();
    std::io::stdout().write_all(&forge_dwg::file::r2004::compress(&p)).unwrap();
}
