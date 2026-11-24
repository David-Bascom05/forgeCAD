//! `forge-cam-quality [--json OUT] [--machine NAME] [--cell MM] [FILE..]`: the quality table of
//! each machining document (default: examples/machined/*_cam.fg and this crate's parts/*_cam.fg).
use forge_cam_quality::{Config, json, measure, table};
use std::path::PathBuf;

fn main() {
    let mut cfg = Config::default();
    let mut out: Option<PathBuf> = None;
    let mut files: Vec<PathBuf> = Vec::new();
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--json" => out = args.next().map(PathBuf::from),
            "--machine" => cfg.machine = args.next(),
            "--dump" => {
                // the posted program of one document
                let f = PathBuf::from(args.next().expect("--dump FILE"));
                let src = std::fs::read_to_string(&f).expect("read");
                let cam = forge_cam_lang::run(&src, f.parent().unwrap());
                for p in &cam.programs {
                    println!("{}", p.text.as_deref().unwrap_or("(no text)"));
                }
                return;
            }
            "--why" => {
                // each op's status and refusal
                let f = PathBuf::from(args.next().expect("--why FILE"));
                let src = std::fs::read_to_string(&f).expect("read");
                let cam = forge_cam_lang::run(&src, f.parent().unwrap());
                for o in &cam.ops {
                    println!("{} {} {:?}", o.name, o.status, o.refusal);
                    for c in o.checks.iter().filter(|c| c.verdict != "pass") {
                        println!(
                            "  {} {} value {} limit {} {}",
                            c.rule, c.verdict, c.value, c.limit, c.other
                        );
                    }
                }
                return;
            }
            "--moves" => {
                // one op's planned moves i0..i1: --moves FILE OP I0 I1
                let f = PathBuf::from(args.next().expect("--moves FILE OP I0 I1"));
                let op = args.next().expect("OP");
                let i0: usize = args.next().and_then(|x| x.parse().ok()).unwrap_or(0);
                let i1: usize = args.next().and_then(|x| x.parse().ok()).unwrap_or(i0 + 20);
                let src = std::fs::read_to_string(&f).expect("read");
                let cam = forge_cam_lang::run(&src, f.parent().unwrap());
                for o in cam.ops.iter().filter(|o| o.name == op) {
                    for (pi, p) in o.paths.iter().enumerate() {
                        for (i, m) in p.moves.iter().enumerate().skip(i0).take(i1.saturating_sub(i0)) {
                            println!("path {pi} move {i}: {m:?}");
                        }
                    }
                }
                return;
            }
            "--cell" => cfg.cell = args.next().and_then(|s| s.parse().ok()),
            _ => files.push(PathBuf::from(a)),
        }
    }
    if files.is_empty() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        for d in [root.join("../../examples/machined"), root.join("parts")] {
            let mut v: Vec<PathBuf> = std::fs::read_dir(&d)
                .into_iter()
                .flatten()
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.to_string_lossy().ends_with("_cam.fg"))
                .collect();
            v.sort();
            files.extend(v);
        }
    }
    let jobs: Vec<_> = files.iter().map(|f| measure(f, &cfg)).collect();
    print!("{}", table(&jobs));
    if let Some(o) = out {
        std::fs::write(&o, json(&jobs)).expect("write the JSON");
    }
}
