//! The native side of the browser's parity test: each file given built as the page builds it
//! (`parity::build_report`), one JSON line each, `{"file":..,..}`. `--bench DIR` writes the
//! bench parts (and their font) into DIR first and builds those.

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().is_some_and(|a| a == "--bench") {
        let Some(dir) = args.get(1).cloned() else {
            eprintln!("forge-parity --bench DIR | FILE..");
            std::process::exit(2);
        };
        std::fs::create_dir_all(&dir).expect("the folder");
        let mut files = vec![];
        for (name, bytes) in forge_render::parity::bench_files() {
            let p = format!("{dir}/{name}");
            std::fs::write(&p, bytes).expect("written");
            if name.ends_with(".fg") {
                files.push(p);
            }
        }
        args = files;
    }
    for f in args {
        let r = forge_render::parity::build_report(&f);
        println!("{{\"file\":{:?},{}", f, &r[1..]);
    }
}
