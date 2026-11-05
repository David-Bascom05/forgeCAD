//! Each loader reads only its own kind of file, decided by its content, and no byte of a file it
//! read reaches any output: key text, /etc/passwd text, a binary blob, a device, a fifo, a huge
//! file are refused typed, and a canary planted in each is in no error, report or JSON.

use forge_lang::{Options, Run};
use std::path::{Path, PathBuf};

const CANARY: &str = "CANARY-7f3a9e";

fn dir(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("fg-read-kinds-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn run_in(d: &Path, src: &str) -> Run {
    let opts = Options {
        dir: Some(d.to_path_buf()),
        ..Options::default()
    };
    forge_lang::run(src, opts, Default::default()).0
}

/// Everything a run says: each error's text and JSON, each warning, and the whole report.
fn said(r: &Run, src: &str) -> String {
    let mut out = String::new();
    for e in &r.errors {
        out.push_str(&e.text());
        out.push_str(&format!("{}", e.json(src)));
        out.push_str(&format!("{e:?}"));
    }
    out.push_str(&format!("{:?}", r.warnings));
    out.push_str(&format!("{}", forge_lang::report::json(r, src)));
    out
}

fn plant(d: &Path) -> Vec<&'static str> {
    let key = format!(
        "-----BEGIN OPENSSH PRIVATE KEY-----\nb3BlbnNzaC1rZXktdjEAAAAA{CANARY}\n-----END OPENSSH PRIVATE KEY-----\n"
    );
    let passwd = format!("root:x:0:0:{CANARY}:/root:/bin/bash\ndaemon:x:1:1::/usr/sbin:/usr/sbin/nologin\n");
    let mut blob: Vec<u8> = (0..4096u32).map(|k| (k * 7 % 251) as u8).collect();
    blob.extend_from_slice(CANARY.as_bytes());
    let mut names = vec![];
    for ext in ["fg", "ttf", "step"] {
        std::fs::write(d.join(format!("key.{ext}")), &key).unwrap();
        std::fs::write(d.join(format!("passwd.{ext}")), &passwd).unwrap();
        std::fs::write(d.join(format!("blob.{ext}")), &blob).unwrap();
        // huge: past every cap, the canary at its start
        let f = std::fs::File::create(d.join(format!("huge.{ext}"))).unwrap();
        std::io::Write::write_all(&mut &f, key.as_bytes()).unwrap();
        f.set_len((65 << 20) + 1).unwrap();
        #[cfg(unix)]
        {
            let _ = std::fs::remove_file(d.join(format!("zero.{ext}")));
            std::os::unix::fs::symlink("/dev/zero", d.join(format!("zero.{ext}"))).unwrap();
            let fifo = d.join(format!("pipe.{ext}"));
            if !fifo.exists() {
                assert!(
                    std::process::Command::new("mkfifo")
                        .arg(&fifo)
                        .status()
                        .unwrap()
                        .success()
                );
            }
        }
    }
    std::fs::write(d.join("id_rsa"), &key).unwrap();
    names.extend(["key", "passwd", "blob", "huge"]);
    #[cfg(unix)]
    names.extend(["zero", "pipe"]);
    names
}

const BODY: &str = "sketch p(on: xy) {\n  c = circle((0, 0), 5)\n}\nb = extrude(p, 4)\n";

#[test]
fn no_loader_reads_another_kind_and_nothing_read_is_said() {
    let d = dir("canary");
    let names = plant(&d);
    let mut cases: Vec<(String, &str)> = vec![];
    for n in &names {
        cases.push((format!("x = part(\"{n}.fg\", ground: true)\n"), "PartFile"));
        cases.push((format!("x = subassembly(\"{n}.fg\", ground: true)\n"), "PartFile"));
        cases.push((format!("t = use(\"{n}.fg\")\n{BODY}"), "UseNotForge"));
        cases.push((
            format!("{BODY}m = text(\"A\", font: \"{n}.ttf\", size: 3, depth: 1, on: b.end, at: (0, 0))\n"),
            "File",
        ));
        cases.push((format!("other = import(\"{n}.step\")\n"), "File"));
    }
    cases.push(("x = part(\"id_rsa\", ground: true)\n".into(), "PartFile"));
    cases.push(("x = part(\"/dev/zero\", ground: true)\n".into(), "PartFile"));
    cases.push(("x = part(\"/etc/passwd\", ground: true)\n".into(), "PartFile"));
    for (src, kind) in &cases {
        let t = std::time::Instant::now();
        let r = run_in(&d, src);
        assert!(t.elapsed().as_secs() < 30, "{src}: {:?}", t.elapsed());
        let all = said(&r, src);
        assert!(!all.contains(CANARY), "{src}: {all}");
        assert!(
            !all.contains("OPENSSH") && !all.contains("root:x"),
            "{src}: {all}"
        );
        assert!(
            r.errors.iter().any(|e| e.kind() == *kind),
            "{src}: {:?}",
            r.errors.iter().map(|e| (e.kind(), e.text())).collect::<Vec<_>>()
        );
    }
}

#[test]
fn a_part_that_does_not_parse_is_refused_with_nothing_of_it() {
    let d = dir("unparsed");
    std::fs::write(
        d.join("bad.fg"),
        format!("shared w = 4\nthis is {CANARY} not forge\n"),
    )
    .unwrap();
    for src in ["x = part(\"bad.fg\", ground: true)\n", "t = use(\"bad.fg\")\n"] {
        let r = run_in(&d, src);
        let all = said(&r, src);
        assert!(!all.contains(CANARY), "{all}");
        // no detail worked out from it either: not where it stopped parsing
        assert!(
            !r.errors.is_empty() && !all.contains("line 2") && !all.contains("\"line\":2"),
            "{all}"
        );
    }
}

/// The rack copied outside any git checkout: its assemblies read ../hw and ../frame as before.
#[test]
fn the_rack_outside_git_loads() {
    let src_dir = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/rack18u"));
    let d = dir("rack");
    let to = d.join("rack18u");
    let status = std::process::Command::new("cp")
        .arg("-r")
        .arg(src_dir)
        .arg(&d)
        .status()
        .unwrap();
    assert!(status.success());
    assert!(
        !to.ancestors().any(|a| a.join(".git").exists()),
        "{to:?} is in a checkout"
    );
    let asm = to.join("asm");
    let src = std::fs::read_to_string(asm.join("carrier_kit.fg")).unwrap();
    let r = run_in(&asm, &src);
    assert!(
        r.ok(),
        "{:?}",
        r.errors.iter().map(|e| e.text()).collect::<Vec<_>>()
    );
    assert!(
        r.asm
            .as_ref()
            .is_some_and(|a| a.bom.iter().any(|b| b.0.contains("hw/")))
    );
}

/// Under a session's root (fg serve), a file outside it is still never read.
#[test]
fn a_session_root_still_fences() {
    let d = dir("root");
    let inside = d.join("in");
    std::fs::create_dir_all(&inside).unwrap();
    std::fs::write(d.join("out.fg"), BODY).unwrap();
    let opts = Options {
        dir: Some(inside.clone()),
        root: Some(inside),
        ..Options::default()
    };
    let r = forge_lang::run(
        "x = part(\"../out.fg\", ground: true)\n",
        opts,
        Default::default(),
    )
    .0;
    assert!(
        r.errors
            .iter()
            .any(|e| e.text().contains("outside the session's root"))
    );
}

/// A drawing under a session's root reads no model outside it, nor any part its BOM names there.
#[test]
fn a_drawing_under_a_session_root_reads_nothing_outside_it() {
    let d = dir("drawing-root");
    let inside = d.join("in");
    std::fs::create_dir_all(&inside).unwrap();
    std::fs::write(d.join("out.fg"), format!("# {CANARY}-9 outside part\n{BODY}")).unwrap();
    #[cfg(unix)]
    {
        let _ = std::fs::remove_file(inside.join("link.fg"));
        std::os::unix::fs::symlink(d.join("out.fg"), inside.join("link.fg")).unwrap();
    }
    for model in ["../out.fg", "link.fg"] {
        let src = format!(
            "drawing\nm = part(\"{model}\")\ns1 = sheet(\"A4\")\ntop = base_view(m, sheet: s1, dir: \"top\", at: (100, 100))\n"
        );
        let dw = forge_lang::drawing::regen::make_in(&src, &inside, Some(&inside));
        let all = format!("{}", forge_lang::drawing::regen::json(&dw, &src));
        assert!(all.contains("outside the session's root"), "{model}: {all}");
        assert!(!all.contains(CANARY), "{model}: {all}");
        // with no root, the same drawing reads it (a Forge file)
        let open = forge_lang::drawing::regen::make(&src, &inside);
        assert!(
            !format!("{}", forge_lang::drawing::regen::json(&open, &src)).contains("outside the session")
        );
    }
}
