//! TO BE DELETED with the .fc engine (interface/forge-agent's model and tools/bench-parts): a
//! measuring tool, not a user of .fc. Built as forge-lang's example `fcvsfg` (its Cargo.toml
//! points here); run it under callgrind (x86-64-v2), one build per process.
//!
//! The real-part benchmarks built by the retired `.fc` engine and by Forge, for comparing their
//! cost:
//!   fcvsfg PART fc|fg full      a full build of tools/bench-parts/PART.fc or parts/bench/PART.fg
//!                               (fg: the CLI's options; fga: the app's, no checks or expects)
//!   fcvsfg PART fc|fg edit      a full build, then a rebuild after one dimension changed (the
//!                               part's first param, the same name in both where there is one):
//!                               the rebuild's cost is `edit` less `full`
//!   fcvsfg PART fc|fg time N    both builds N times each, the least wall time of each (ms)

/// The text with the first param's (or param `name`'s) value scaled by 1.01, and that name.
fn edited(src: &str, fg: bool, name: Option<&str>) -> (String, String) {
    let mut out = String::with_capacity(src.len() + 8);
    let mut done: Option<String> = None;
    for line in src.split_inclusive('\n') {
        if done.is_none() {
            let body = line.trim_end();
            let head = if fg {
                body.strip_prefix("shared ")
                    .or_else(|| body.strip_prefix("param "))
            } else {
                Some(body)
            };
            if let Some(h) = head
                && let Some((n, v)) = h.split_once(" = ")
                && !n.contains(':')
                && !n.trim().contains(' ')
                && name.is_none_or(|w| n.trim() == w)
            {
                let v = v.split('#').next().unwrap_or("").trim();
                if let Ok(x) = v.trim_end_matches("mm").trim().parse::<f64>() {
                    let prefix = &line[..line.find(" = ").expect("found") + 3];
                    out.push_str(&format!("{prefix}{}\n", x * 1.01));
                    done = Some(n.trim().to_string());
                    continue;
                }
            }
        }
        out.push_str(line);
    }
    (out, done.unwrap_or_default())
}

fn opts() -> forge_lang::Options {
    let dir = std::env::temp_dir().join(format!("forge-fcvsfg-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let font = dir.join(forge_agent::bench::FONT);
    if !font.exists() {
        std::fs::write(&font, forge_agent::bench::font_bytes()).expect("font");
    }
    forge_lang::Options {
        dir: Some(dir),
        ..forge_lang::Options::default()
    }
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let (part, eng, what) = (&a[1], a[2].as_str(), a[3].as_str());
    let root = env!("CARGO_MANIFEST_DIR"); // forge-lang's, whose example this is
    let fc_src =
        std::fs::read_to_string(format!("{root}/../../tools/bench-parts/{part}.fc")).expect("fc part");
    let fg_src = std::fs::read_to_string(format!("{root}/parts/bench/{part}.fg")).expect("fg part");
    // the dimension changed: the .fc part's first param, and the same name in the .fg part if
    // it has one, else its first
    let (fc_edit, name) = edited(&fc_src, false, None);
    let (mut fg_edit, fg_name) = edited(&fg_src, true, Some(&name));
    if fg_name.is_empty() {
        fg_edit = edited(&fg_src, true, None).0;
    }
    let fc = |edit: bool| {
        let mut m = forge_agent::bench::model(&fc_src);
        assert!(m.error.is_none(), "{part}.fc");
        if edit {
            m.update(&fc_edit);
            assert!(m.error.is_none(), "{part}.fc edited");
        }
        std::hint::black_box(m.body().map(|b| b.faces.len()));
    };
    let app = eng == "fga";
    let fg_opts = || {
        let o = opts();
        if app {
            forge_lang::Options {
                checks: false,
                expects: false,
                ..o
            }
        } else {
            o
        }
    };
    let fg = |edit: bool| {
        let (r, cache) = forge_lang::run(&fg_src, fg_opts(), Default::default());
        assert!(r.errors.is_empty(), "{part}.fg");
        if edit {
            let (r, _) = forge_lang::run(&fg_edit, fg_opts(), cache);
            assert!(r.errors.is_empty(), "{part}.fg edited");
            std::hint::black_box(r.body.map(|b| b.faces.len()));
        }
    };
    match (eng, what) {
        ("fc", "full") => fc(false),
        ("fc", "edit") => fc(true),
        ("fg" | "fga", "full") => fg(false),
        ("fg" | "fga", "edit") => fg(true),
        (_, "time") => {
            let n: usize = a.get(4).and_then(|x| x.parse().ok()).unwrap_or(5);
            let best = |f: &dyn Fn(bool), e: bool| {
                (0..n)
                    .map(|_| {
                        let t = std::time::Instant::now();
                        f(e);
                        t.elapsed().as_secs_f64() * 1e3
                    })
                    .fold(f64::INFINITY, f64::min)
            };
            let (cf, ce) = (best(&fc, false), best(&fc, true));
            let (gf, ge) = (best(&fg, false), best(&fg, true));
            println!(
                "{part} param {name}/{} fc full {cf:.1} edit {:.1}  fg full {gf:.1} edit {:.1}",
                if fg_name.is_empty() { "-" } else { &fg_name },
                ce - cf,
                ge - gf
            );
        }
        _ => panic!("usage: fcvsfg PART fc|fg|fga full|edit|time"),
    }
}
