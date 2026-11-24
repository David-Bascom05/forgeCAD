//! The reply and posted NC of each document, written whole to OUT/<name>.txt (to compare builds).
fn main() {
    let mut a = std::env::args().skip(1);
    let out = std::path::PathBuf::from(a.next().expect("OUT"));
    std::fs::create_dir_all(&out).expect("out");
    for d in a {
        let p = std::path::Path::new(&d);
        let cam = forge_cam_lang::run(&std::fs::read_to_string(p).expect("read"), p.parent().unwrap());
        let mut text = cam.json().to_string();
        for pr in &cam.programs {
            text.push_str(&format!("\n{:?}", pr.text));
        }
        std::fs::write(
            out.join(format!("{}.txt", p.file_stem().unwrap().to_string_lossy())),
            text,
        )
        .expect("write");
    }
}
