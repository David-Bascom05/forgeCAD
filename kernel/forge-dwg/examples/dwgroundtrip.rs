//! Reads each file, writes it in its own version to OUT_DIR, reads that back and compares.
//! `dwgroundtrip OUT_DIR FILE..`
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let out = &a[1];
    for path in &a[2..] {
        let buf = std::fs::read(path).unwrap();
        let name = std::path::Path::new(path).file_name().unwrap().to_string_lossy().to_string();
        let d = match forge_dwg::file::read(&buf) {
            Ok(d) => d,
            Err(e) => {
                println!("{name}: read: {}", e.0);
                continue;
            }
        };
        let bytes = match forge_dwg::file::write(&d) {
            Ok(b) => b,
            Err(e) => {
                println!("{name}: write: {}", e.0);
                continue;
            }
        };
        let to = format!("{out}/{name}");
        std::fs::write(&to, &bytes).unwrap();
        match forge_dwg::file::read(&bytes) {
            Ok(e) => {
                let same_objs = d.objects.len() == e.objects.len()
                    && d.objects.iter().zip(&e.objects).all(|(x, y)| x.0 == y.0 && x.1.data == y.1.data && x.1.ent == y.1.ent);
                println!(
                    "{name}: {} -> {} bytes; {} objects {}; header {}; classes {}",
                    buf.len(),
                    bytes.len(),
                    e.objects.len(),
                    if same_objs { "same" } else { "DIFFERENT" },
                    if d.header == e.header { "same" } else { "DIFFERENT" },
                    if d.classes == e.classes { "same" } else { "DIFFERENT" },
                );
            }
            Err(x) => println!("{name}: re-read: {}", x.0),
        }
    }
}
