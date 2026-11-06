//! Re-lays a file's sections, as read, in a new R2004 container (no decoding). `dwgrecontain IN OUT`
use forge_dwg::file::{r2004, sections};

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let buf = std::fs::read(&a[1]).unwrap();
    let s = sections(&buf).unwrap();
    let mut c: Vec<r2004::Content> = vec![];
    for (n, d) in [("AcDb:Header", &s.header), ("AcDb:Classes", &s.classes), ("AcDb:Handles", &s.handles), ("AcDb:AcDbObjects", &s.objects)] {
        let comp = if std::env::var("NOCOMP").is_ok() { 1 } else { 2 };
        c.push((n.to_string(), d.clone(), 0x7400, comp, 0, 1));
    }
    for e in &s.other {
        c.push((e.name.clone(), e.data.clone(), e.max_page, e.compressed, e.encrypted, e.unknown));
    }
    let out = r2004::container(s.ver, s.maint, s.codepage, &s.file_header, &c).unwrap();
    std::fs::write(&a[2], out).unwrap();
}
