//! A session keeps the stages it built by the key of the history up to them: an edit taken back
//! builds nothing, and what it gives back is what a fresh build makes, bit for bit.

use forge_lang::build::Cache;
use forge_lang::{Options, Run, report};
use std::path::Path;

fn run(src: &str, dir: &Path, cache: Cache) -> (Run, Cache) {
    let opts = Options {
        dir: Some(dir.to_path_buf()),
        ..Options::default()
    };
    forge_lang::run(src, opts, cache)
}

/// The body to the bit: every vertex, every face's and edge's name, in order.
fn bits(r: &Run) -> Vec<u64> {
    let Some(b) = &r.body else { return vec![] };
    let mut out = vec![b.face_ids().count() as u64, b.edge_ids().count() as u64];
    for v in &b.vertices {
        out.extend([v.point.x, v.point.y, v.point.z].map(f64::to_bits));
    }
    out.extend(b.face_ids().map(|f| b.face(f).name.0));
    out.extend(b.edge_ids().map(|e| b.edge(e).name.0));
    out
}

/// The report less what says how it was built (times, rebuilt and reused counts).
fn shown(r: &Run, src: &str) -> String {
    let mut s = String::new();
    report::json(r, src).write(&mut s);
    let cut = |s: &str, key: &str| -> String {
        let Some(at) = s.find(key) else { return s.into() };
        let end = at + s[at..].find([',', '}']).expect("ends") + 1;
        format!("{}{}", &s[..at], &s[end..])
    };
    let s = match s.find("\"timings_ms\":{") {
        Some(at) => {
            let end = at + s[at..].find('}').expect("closed") + 2;
            format!("{}{}", &s[..at], &s[end..])
        }
        None => s,
    };
    let s = cut(&s, "\"rebuilt\":");
    let s = cut(&s, "\"reused\":");
    cut(&s, "\"remade\":")
}

/// The number `name` is set to, nudged.
fn nudged(src: &str, name: &str) -> Option<String> {
    let line = src.lines().find(|l| {
        (l.starts_with("param ") || l.starts_with("shared ")) && l.split_whitespace().nth(1) == Some(name)
    })?;
    let (head, rhs) = line.split_once(" = ")?;
    let num: String = rhs
        .chars()
        .enumerate()
        .take_while(|(k, c)| c.is_ascii_digit() || *c == '.' || (*k == 0 && *c == '-'))
        .map(|x| x.1)
        .collect();
    let v: f64 = num.parse().ok()?;
    Some(src.replacen(
        line,
        &format!("{head} = {}{}", v * 1.01 + 0.1, &rhs[num.len()..]),
        1,
    ))
}

#[test]
fn an_edit_taken_back_is_served_its_stages_and_they_are_a_fresh_builds() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    let files = [
        // a text's font is a file, so its step is built again
        ("demo/part_tools.fg", "extrude2.d", 1),
        ("demo/part_tools.fg", "sk1.x1", 1),
        ("machined/flange.fg", "t", 0),
        ("rack18u/frame/post.fg", "t3", 0),
        ("rack18u/hw/iso7380_m6x12.fg", "l", 0),
        ("rack18u/panels/hinge_bracket.fg", "t2", 0),
    ];
    for (f, name, unkeyed) in files {
        let path = root.join(f);
        let dir = path.parent().unwrap();
        let src = std::fs::read_to_string(&path).unwrap();
        let (fresh, _) = run(&src, dir, Cache::default());
        assert!(
            fresh.errors.is_empty(),
            "{f}: {:?}",
            fresh.errors.iter().map(|e| e.text()).collect::<Vec<_>>()
        );
        let other = nudged(&src, name).unwrap_or_else(|| panic!("{f}: no {name} to nudge"));
        let (_, cache) = run(&src, dir, Cache::default());
        let (moved, cache) = run(&other, dir, cache);
        assert!(
            moved.rebuilt > 0,
            "{f}: a changed input is not served the old stages"
        );
        let (back, _) = run(&src, dir, cache);
        assert_eq!(
            back.rebuilt, unkeyed,
            "{f}: taken back, nothing is built but what reads a file"
        );
        assert_eq!(bits(&back), bits(&fresh), "{f}: the body to the bit");
        assert_eq!(shown(&back, &src), shown(&fresh, &src), "{f}: the report");
    }
}

/// Built after `before` in the same session, `after` gives what a fresh build of it gives: the key
/// held what changed, so nothing of `before`'s was served wrongly.
fn after_is_fresh(before: &str, after: &str, dir: &Path, opts: impl Fn() -> Options) {
    let go = |src: &str, cache: Cache| forge_lang::run(src, opts(), cache);
    let (_, cache) = go(before, Cache::default());
    let (kept, _) = go(after, cache);
    let (fresh, _) = go(after, Cache::default());
    assert!(
        fresh.errors.is_empty(),
        "{:?}",
        fresh.errors.iter().map(|e| e.text()).collect::<Vec<_>>()
    );
    assert_eq!(bits(&kept), bits(&fresh), "{after}");
    assert_eq!(shown(&kept, after), shown(&fresh, after));
    let _ = dir;
}

const PLATE: &str = "shared w = 40
shared t = 6
sketch s(on: xy) {
  r = rect(corner: (0, 0), size: (w, 30))
}
base = extrude(s, t)
sketch spot(on: base.end) {
  p = near(10, 10)
}
hole1 = hole(at: spot.p, d: 4, depth: thru)
holes = pattern(of: hole1, layout: lin(along: s.r[0].dir, n: count(3), step: 8))
";

fn opts_in(dir: &Path) -> impl Fn() -> Options + '_ {
    move || Options {
        dir: Some(dir.to_path_buf()),
        ..Options::default()
    }
}

// 1: a parameter's last bit is part of the key
#[test]
fn the_key_holds_every_bit_of_a_number() {
    let d = std::env::temp_dir();
    after_is_fresh(
        PLATE,
        &PLATE.replace("t = 6", "t = 6.000000000000001"),
        &d,
        opts_in(&d),
    );
}

// 2, 3: the stamp a step names its faces with (its id) is part of the key
#[test]
fn the_key_holds_the_names_a_step_stamps() {
    let d = std::env::temp_dir();
    let renamed = PLATE.replace("base", "slab");
    after_is_fresh(PLATE, &renamed, &d, opts_in(&d));
}

// 2, 6: what a step reads of the steps before it (its input shape, a pattern's source) is
#[test]
fn the_key_holds_what_came_before() {
    let d = std::env::temp_dir();
    after_is_fresh(PLATE, &PLATE.replace("w = 40", "w = 41"), &d, opts_in(&d));
    after_is_fresh(PLATE, &PLATE.replace("d: 4,", "d: 5,"), &d, opts_in(&d));
}

// 4: an option that changes what is built (a drawing's cosmetic threads) is
#[test]
fn the_key_holds_the_options_that_change_a_step() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/rack18u/hw/iso7380_m6x12.fg");
    let dir = path.parent().unwrap().to_path_buf();
    let src = std::fs::read_to_string(&path).unwrap();
    let plain = || Options {
        dir: Some(dir.clone()),
        ..Options::default()
    };
    let cosmetic = || Options {
        dir: Some(dir.clone()),
        cosmetic_threads: true,
        checks: false,
        ..Options::default()
    };
    let (_, cache) = forge_lang::run(&src, plain(), Cache::default());
    let (kept, _) = forge_lang::run(&src, cosmetic(), cache);
    let (fresh, _) = forge_lang::run(&src, cosmetic(), Cache::default());
    assert_eq!(bits(&kept), bits(&fresh));
}

// 5: the same history built on one thread or on all gives the same output, so the thread count
// is no input
#[test]
fn a_step_is_the_same_on_any_number_of_threads() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    for f in [
        "demo/part_tools.fg",
        "machined/flange.fg",
        "rack18u/frame/post.fg",
    ] {
        let path = root.join(f);
        let out = |threads: Option<&str>| {
            let mut c = std::process::Command::new(env!("CARGO_BIN_EXE_fg"));
            c.args(["run", path.to_str().unwrap(), "--json", "--step"])
                .arg(std::env::temp_dir().join(format!("kept-threads-{}.step", threads.unwrap_or("all"))))
                .env("FORGECAD_CACHE", "off");
            if let Some(t) = threads {
                c.env("FORGECAD_THREADS", t);
            }
            let o = c.output().unwrap();
            let json = String::from_utf8(o.stdout).unwrap();
            let src = std::fs::read_to_string(&path).unwrap();
            let _ = src;
            let at = json.find("\"timings_ms\":{").unwrap();
            let end = at + json[at..].find('}').unwrap() + 2;
            let step = std::fs::read_to_string(
                std::env::temp_dir().join(format!("kept-threads-{}.step", threads.unwrap_or("all"))),
            )
            .unwrap_or_default();
            // the STEP file names its time of writing
            let step: String = step.lines().filter(|l| !l.starts_with("FILE_NAME")).collect();
            (format!("{}{}", &json[..at], &json[end..]), step)
        };
        assert_eq!(out(Some("1")), out(None), "{f}");
    }
}
