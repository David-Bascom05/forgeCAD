//! `forge-cam-quality [--json OUT] [--machine NAME] [--cell MM] [FILE..]`: the quality table of
//! each machining document (default: examples/machined/*_cam.fg, *_turn.fg and this crate's
//! parts/*_cam.fg), then coverage: each document's ops certified and its refusals.
use forge_cam_quality::{Config, json, measure, table};
use std::path::PathBuf;

fn main() {
    let mut cfg = Config::default();
    let mut out: Option<PathBuf> = None;
    let mut files: Vec<PathBuf> = Vec::new();
    let mut ngc: Vec<(PathBuf, ([f64; 3], [f64; 3]))> = Vec::new();
    let mut flutes: Option<u32> = None;
    let mut part: Option<(PathBuf, [f64; 3])> = None;
    // each job's stock against the part, after the table
    let ends = |jobs: &[forge_cam_quality::JobQ], part: &Option<(PathBuf, [f64; 3])>| {
        let Some((pf, shift)) = part else { return };
        let tris = forge_cam_quality::endstate::part_triangles(pf, *shift).expect("the part");
        for j in jobs {
            for (si, m) in j.ends.iter().enumerate() {
                let Some(m) = m else { continue };
                let tops = forge_cam_quality::endstate::part_tops(&tris, m);
                let e = forge_cam_quality::endstate::end_state(
                    m,
                    &tops,
                    forge_cam_quality::endstate::footprint(&tris),
                    0.02,
                );
                println!(
                    "{} setup {si}: end state vs {}: left {:.0} mm3 (max {:.2} mm, {:.0} mm2 over 0.05), gouge {:.0} mm3 (max {:.2} mm); cell {} mm",
                    j.file,
                    pf.display(),
                    e.left_vol,
                    e.left_max,
                    e.left_area,
                    e.gouge_vol,
                    e.gouge_max,
                    m.h
                );
            }
        }
    };
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
            "--ngc" => {
                // a foreign program: --ngc FILE X0 Y0 Z0 X1 Y1 Z1 [--flutes N] (the stock box)
                let f = PathBuf::from(args.next().expect("--ngc FILE X0 Y0 Z0 X1 Y1 Z1"));
                let v: Vec<f64> = (0..6)
                    .map(|_| {
                        args.next()
                            .and_then(|x| x.parse().ok())
                            .expect("the stock box: six numbers")
                    })
                    .collect();
                ngc.push((f, ([v[0], v[1], v[2]], [v[3], v[4], v[5]])));
            }
            "--part" => {
                // the end state against a part: --part FILE.fg DX DY DZ (part frame -> map frame)
                let f = PathBuf::from(args.next().expect("--part FILE DX DY DZ"));
                let v: Vec<f64> = (0..3)
                    .map(|_| {
                        args.next()
                            .and_then(|x| x.parse().ok())
                            .expect("--part: three numbers")
                    })
                    .collect();
                part = Some((f, [v[0], v[1], v[2]]));
            }
            "--feeds" => {
                // NAME=CUT,PLUNGE: run op NAME at these feeds (mm/min)
                let a = args.next().expect("--feeds NAME=CUT,PLUNGE");
                let (n, v) = a.split_once('=').expect("NAME=CUT,PLUNGE");
                let (c, p) = v.split_once(',').expect("NAME=CUT,PLUNGE");
                cfg.feeds
                    .push((n.to_string(), c.parse().expect("CUT"), p.parse().expect("PLUNGE")));
            }
            "--la" => {
                // the lookahead planner: --la ACCEL_SCALE BLOCKS (grbl's defaults otherwise)
                let a: f64 = args
                    .next()
                    .and_then(|x| x.parse().ok())
                    .expect("--la ACCEL_SCALE BLOCKS");
                let b: usize = args
                    .next()
                    .and_then(|x| x.parse().ok())
                    .expect("--la ACCEL_SCALE BLOCKS");
                cfg.lookahead = Some(forge_machine::lookahead::LookAhead {
                    accel_scale: a,
                    blocks: b,
                    ..Default::default()
                });
            }
            "--scurve" => {
                // the jerk-limited planner: --scurve ACCEL JERK TOL (linear; rotary 3600/36000)
                let mut num = || -> f64 {
                    args.next()
                        .and_then(|x| x.parse().ok())
                        .expect("--scurve ACCEL JERK TOL")
                };
                let (a, j, d) = (num(), num(), num());
                cfg.scurve = Some(forge_machine::lookahead::SCurve {
                    accel: [a, 3600.0],
                    jerk: [j, 36000.0],
                    tolerance: d,
                });
            }
            "--flutes" => flutes = args.next().and_then(|s| s.parse().ok()),
            "--cell" => cfg.cell = args.next().and_then(|s| s.parse().ok()),
            _ => files.push(PathBuf::from(a)),
        }
    }
    if !ngc.is_empty() {
        let jobs: Vec<_> = ngc
            .iter()
            .map(|(f, stock)| {
                let text = std::fs::read_to_string(f).expect("read the program");
                let fo = forge_cam_quality::foreign::Foreign {
                    stock: *stock,
                    flutes,
                    dialect: "linuxcnc".into(),
                    machine: cfg.machine.clone().unwrap_or_else(|| "generic-vmc3".into()),
                    cell: cfg.cell.unwrap_or(0.1),
                };
                forge_cam_quality::foreign::measure_text(&text, f.display().to_string(), &fo)
            })
            .collect();
        print!("{}", table(&jobs));
        ends(&jobs, &part);
        if let Some(o) = out {
            std::fs::write(&o, json(&jobs)).expect("write the JSON");
        }
        return;
    }
    if files.is_empty() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        for d in [root.join("../../examples/machined"), root.join("parts")] {
            let mut v: Vec<PathBuf> = std::fs::read_dir(&d)
                .into_iter()
                .flatten()
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| {
                    let n = p.to_string_lossy();
                    n.ends_with("_cam.fg") || n.ends_with("_turn.fg")
                })
                .collect();
            v.sort();
            files.extend(v);
        }
    }
    let jobs: Vec<_> = files.iter().map(|f| measure(f, &cfg)).collect();
    print!("{}", table(&jobs));
    print!("{}", forge_cam_quality::coverage(&jobs));
    ends(&jobs, &part);
    if let Some(o) = out {
        std::fs::write(&o, json(&jobs)).expect("write the JSON");
    }
}
