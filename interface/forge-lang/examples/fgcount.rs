//! One build of a bench part (parts/bench/*.fg), to count under callgrind against
//! tools/speed/fc-reference.txt:
//!   fgcount PART fga|fg full|edit
//! fga: the app's options (no checks, no expects); fg: the CLI's. full: one cold build; edit: a
//! cold build, then the part again with one dimension (`EDITED`) times 1.01 through the first
//! build's cache (the reference's edit is that sum too).

/// The dimension each part's edit changes: the .fc part's first param where the .fg part has
/// it, else the .fg part's first (as the reference's counts were taken).
const EDITED: &[(&str, &str)] = &[
    ("gearbox", "L"),
    ("volute", "dx"),
    ("crank", "al"),
    ("valve", "bx"),
    ("heatsink", "L"),
    ("quadarm", "L"),
    ("conrod", "c"),
    ("jack", "rf"),
    ("frame", "reach"),
    ("manifold", "pitch"),
    ("bracket", "w"),
    ("insert", "pl"),
    ("clamp", "bore"),
    ("hub", "pcd"),
    ("guard", "r1"),
];

/// The text with `shared NAME = V` (or `param NAME = V`) as V times 1.01.
fn edited(src: &str, name: &str) -> String {
    let mut out = String::with_capacity(src.len() + 8);
    let mut done = false;
    for line in src.split_inclusive('\n') {
        if !done {
            let body = line.trim_end();
            let head = body
                .strip_prefix("shared ")
                .or_else(|| body.strip_prefix("param "));
            if let Some(h) = head
                && let Some((n, v)) = h.split_once(" = ")
                && n.trim() == name
            {
                let v = v.split('#').next().unwrap_or("").trim();
                if let Ok(x) = v.trim_end_matches("mm").trim().parse::<f64>() {
                    let prefix = &line[..line.find(" = ").expect("found") + 3];
                    out.push_str(&format!("{prefix}{}\n", x * 1.01));
                    done = true;
                    continue;
                }
            }
        }
        out.push_str(line);
    }
    assert!(done, "no {name} to edit");
    out
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let (part, eng, what) = (a[1].as_str(), a[2].as_str(), a[3].as_str());
    let src = std::fs::read_to_string(format!("{}/parts/bench/{part}.fg", env!("CARGO_MANIFEST_DIR")))
        .expect("fg part");
    let name = EDITED.iter().find(|e| e.0 == part).expect("a bench part").1;
    let edit = edited(&src, name);
    let opts = || {
        let dir = std::env::temp_dir().join(format!("forge-fgcount-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let font = dir.join(forge_lang::bench::FONT);
        if !font.exists() {
            std::fs::write(&font, forge_lang::bench::font_bytes()).expect("font");
        }
        let o = forge_lang::Options {
            dir: Some(dir),
            ..forge_lang::Options::default()
        };
        match eng {
            "fga" => forge_lang::Options {
                checks: false,
                expects: false,
                ..o
            },
            "fg" => o,
            _ => panic!("usage: fgcount PART fga|fg full|edit"),
        }
    };
    let (r, cache) = forge_lang::run(&src, opts(), Default::default());
    assert!(r.errors.is_empty(), "{part}.fg");
    match what {
        "full" => {}
        "edit" => {
            let (r, _) = forge_lang::run(&edit, opts(), cache);
            assert!(r.errors.is_empty(), "{part}.fg edited");
            std::hint::black_box(r.body.map(|b| b.faces.len()));
        }
        _ => panic!("usage: fgcount PART fga|fg full|edit"),
    }
}
