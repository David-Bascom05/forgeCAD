//! Compares each section this re-makes with the file's. `dwgsecdiff FILE..`
use forge_dwg::file::{r2000, sections};

fn main() {
    for path in std::env::args().skip(1) {
        let buf = std::fs::read(&path).unwrap();
        let s = sections(&buf).unwrap();
        let d = forge_dwg::file::read(&buf).unwrap();
        let h = forge_dwg::header::write_header(&d.header, s.ver, s.maint).unwrap();
        let c = r2000::write_classes(&d.classes, s.ver, s.maint).unwrap();
        let map = r2000::read_map(&s.handles, 0, s.handles.len()).unwrap();
        let locs: Vec<(u32, u64)> = map.iter().map(|m| (m.0, m.1 as u64)).collect();
        let mut sorted = locs.clone();
        sorted.sort();
        let m = r2000::write_map(&sorted);
        let first = |a: &[u8], b: &[u8]| a.iter().zip(b).position(|(x, y)| x != y);
        println!("== {path}");
        println!("  header  {} vs {} first diff {:?}", h.len(), s.header.len(), first(&h, &s.header));
        println!("  classes {} vs {} first diff {:?}", c.len(), s.classes.len(), first(&c, &s.classes));
        println!("  map     {} vs {} first diff {:?} (map sorted: {})", m.len(), s.handles.len(), first(&m, &s.handles), locs == sorted);
    }
}
