//! Saves, versions, restore and the saves list on real repositories, with git as the judge.

use forge_git::Sig;
use forge_history::{History, Paused, Saved};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static N: AtomicU32 = AtomicU32::new(0);

fn tmp(name: &str) -> PathBuf {
    let d = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "forge-history-{name}-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn git(dir: &Path, args: &[&str]) -> String {
    let o = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .unwrap();
    assert!(
        o.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&o.stderr)
    );
    String::from_utf8(o.stdout).unwrap()
}

fn who(t: i64) -> Sig {
    Sig {
        name: "David Bascom".into(),
        email: "davidsprojects7@gmail.com".into(),
        time: t,
        tz: 0,
    }
}

#[test]
fn saves_versions_and_restore() {
    let w = tmp("flow");
    let h = History::turn_on(&w).unwrap();
    assert!(w.join(".gitignore").exists());
    let part = w.join("bracket.fg");
    let asm = w.join("top.fg");
    let shop = w.join("shop.fg");
    std::fs::write(&shop, "shared t = 3\n").unwrap();
    std::fs::write(&part, "w = 40\n").unwrap();
    std::fs::write(w.join("bracket.stl"), "solid\n").unwrap();
    // a save: the part and the new file it reads; never the derived mesh
    let s1 = h
        .save_as(
            &part,
            &[shop.clone(), w.join("bracket.stl")],
            "bracket.fg: saved",
            None,
            &who(100),
        )
        .unwrap();
    let Saved::New(c1) = s1 else { panic!("{s1:?}") };
    assert_eq!(
        h.save_as(&part, &[], "again", None, &who(101)).unwrap(),
        Saved::Unchanged
    );
    let v1 = h
        .create_version_as("bracket.fg", "Bracket rev A", "first cut", None, &who(102))
        .unwrap();
    assert_eq!((v1.n, v1.save), (1, c1));
    std::fs::write(&part, "w = 42\n").unwrap();
    let Saved::New(c2) = h
        .save_as(&part, &[], "bracket.fg: w 40 -> 42", Some("claude"), &who(103))
        .unwrap()
    else {
        panic!()
    };
    std::fs::write(&asm, "assembly\n").unwrap();
    h.save_as(&asm, &[], "top.fg: saved", None, &who(104)).unwrap();
    let v2 = h
        .create_version_as("bracket.fg", "", "", Some("claude"), &who(105))
        .unwrap();
    assert_eq!((v2.n, v2.name.as_str(), v2.save != c2), (2, "v2", true));

    let vs = h.versions(Some("bracket.fg")).unwrap();
    assert_eq!(
        vs.iter().map(|v| (v.n, v.name.as_str())).collect::<Vec<_>>(),
        [(1, "Bracket rev A"), (2, "v2")]
    );
    assert_eq!(vs[0].notes, "first cut");
    assert_eq!(vs[1].agent.as_deref(), Some("claude"));
    let part_saves = h.saves(Some("bracket.fg")).unwrap();
    assert_eq!(part_saves.iter().map(|r| r.id).collect::<Vec<_>>(), [c2, c1]);
    assert_eq!(part_saves[0].agent.as_deref(), Some("claude"));
    assert_eq!(h.saves(None).unwrap().len(), 3);
    assert_eq!(h.resolve("bracket.fg", "v1").unwrap(), Some(c1));
    assert_eq!(h.resolve("bracket.fg", &c2.hex()[..8]).unwrap(), Some(c2));
    assert_eq!(h.resolve("bracket.fg", "current").unwrap(), None);

    // restore v1: a new save with v1's content
    let r = h
        .restore_as(
            &["bracket.fg".into()],
            &c1,
            "bracket.fg: restored v1 (Bracket rev A)",
            None,
            &who(106),
        )
        .unwrap();
    let Saved::New(c4) = r else { panic!("{r:?}") };
    assert_eq!(std::fs::read_to_string(&part).unwrap(), "w = 40\n");
    assert_eq!(h.saves(Some("bracket.fg")).unwrap()[0].id, c4);

    git(&w, &["fsck", "--strict", "--no-dangling"]);
    assert_eq!(git(&w, &["status", "--porcelain"]), "");
    assert_eq!(
        git(&w, &["log", "--format=%s", "--", "bracket.fg"])
            .lines()
            .count(),
        3
    );
    assert!(git(&w, &["ls-files"]).contains("shop.fg"));
    assert!(!git(&w, &["ls-files"]).contains("bracket.stl"));
    assert_eq!(git(&w, &["tag", "-l"]).lines().count(), 2);
}

#[test]
fn saves_pause_rather_than_land_mid_merge() {
    let w = tmp("busy");
    let h = History::turn_on(&w).unwrap();
    let f = w.join("a.fg");
    std::fs::write(&f, "x = 1\n").unwrap();
    h.save_as(&f, &[], "a", None, &who(1)).unwrap();
    std::fs::write(h.repo.git_dir.join("MERGE_HEAD"), "x").unwrap();
    std::fs::write(&f, "x = 2\n").unwrap();
    assert_eq!(
        h.save_as(&f, &[], "a", None, &who(2)).unwrap(),
        Saved::Paused(Paused::Busy("merge".into()))
    );
    std::fs::write(w.join(".gitignore"), "*.fg\n").unwrap();
    let g = w.join("b.fg");
    std::fs::write(&g, "y = 1\n").unwrap();
    std::fs::rename(h.repo.git_dir.join("MERGE_HEAD"), w.join("merge-head-moved")).unwrap();
    assert_eq!(
        h.save_as(&g, &[], "b", None, &who(3)).unwrap(),
        Saved::Paused(Paused::Ignored)
    );
}

#[test]
fn the_saves_list_follows_head_moved_by_git() {
    let w = tmp("moved");
    let h = History::turn_on(&w).unwrap();
    let f = w.join("a.fg");
    let mut ids = vec![];
    for i in 0..5 {
        std::fs::write(&f, format!("x = {i}\n")).unwrap();
        let Saved::New(c) = h.save_as(&f, &[], &format!("s{i}"), None, &who(i)).unwrap() else {
            panic!()
        };
        ids.push(c);
    }
    assert_eq!(h.log().unwrap().len(), 5);
    git(&w, &["reset", "-q", "--hard", &ids[2].hex()]);
    let h = History::open(&f).unwrap();
    assert_eq!(
        h.log().unwrap().iter().map(|r| r.id).collect::<Vec<_>>(),
        [ids[2], ids[1], ids[0]]
    );
    std::fs::write(&f, "x = 9\n").unwrap();
    git(
        &w,
        &[
            "-c",
            "user.name=G",
            "-c",
            "user.email=g@x",
            "commit",
            "-qam",
            "by git",
        ],
    );
    let log = h.log().unwrap();
    assert_eq!(log.len(), 4);
    assert_eq!(log[0].summary, "by git");
    assert_eq!(log[0].paths, ["a.fg"]);
}

/// A long history (git fast-import writes it), listed from cold and then from the cache.
fn long(n: usize) -> (std::time::Duration, std::time::Duration, usize) {
    let w = tmp("long");
    git(&w, &["init", "-q", "-b", "main"]);
    let mut st = String::new();
    for i in 0..n {
        st.push_str(&format!(
            "commit refs/heads/main\ncommitter D <d@x> {} +0000\ndata 3\nc{}\nM 100644 inline p{}.fg\ndata {}\n{}\n",
            1_700_000_000 + i,
            i % 10,
            i % 25,
            format!("{i}").len() + 1,
            i
        ));
    }
    let mut fi = Command::new("git")
        .arg("-C")
        .arg(&w)
        .args(["fast-import", "--quiet"])
        .stdin(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    use std::io::Write;
    fi.stdin.take().unwrap().write_all(st.as_bytes()).unwrap();
    assert!(fi.wait().unwrap().success());
    let h = History::open(&w).unwrap();
    let t = std::time::Instant::now();
    let cold = h.log().unwrap().len();
    let cold_t = t.elapsed();
    let t = std::time::Instant::now();
    let warm = h.saves(Some("p3.fg")).unwrap().len();
    let warm_t = t.elapsed();
    assert_eq!(cold, n);
    assert_eq!(warm, n / 25);
    (cold_t, warm_t, n)
}

#[test]
fn a_long_history_lists_from_its_cache() {
    let (cold, warm, n) = long(1500);
    eprintln!("{n} saves: cold {cold:.2?}, cached {warm:.2?}");
}

#[test]
#[ignore]
fn ten_thousand_saves_open_at_once() {
    let (cold, warm, n) = long(10_000);
    eprintln!("{n} saves: cold {cold:.2?}, cached {warm:.2?}");
    assert!(warm.as_millis() < 50, "{warm:?}");
}

/// One changed file saved in a project of a thousand: the commit, its objects synced, the
/// branch moved and the index kept in step.
#[test]
#[ignore]
fn a_save_takes_milliseconds() {
    let w = tmp("speed");
    let h = History::turn_on(&w).unwrap();
    std::fs::create_dir_all(w.join("parts")).unwrap();
    let mut all = vec![];
    for i in 0..1000 {
        let p = format!("parts/p{i}.fg");
        std::fs::write(w.join(&p), format!("w = {i}\n")).unwrap();
        all.push(p);
    }
    h.repo.commit_paths(&all, "all", &who(1)).unwrap();
    let f = w.join("parts/p500.fg");
    let mut times = vec![];
    for i in 0..30 {
        std::fs::write(&f, format!("w = {}\n", 1000 + i)).unwrap();
        let t = std::time::Instant::now();
        let s = h.save_as(&f, &[], "p500: saved", None, &who(2 + i)).unwrap();
        times.push(t.elapsed().as_secs_f64() * 1e3);
        assert!(matches!(s, Saved::New(_)));
    }
    times.sort_by(|a, b| a.total_cmp(b));
    eprintln!("save: median {:.2} ms, worst {:.2} ms", times[15], times[29]);
    git(&w, &["fsck", "--strict", "--no-dangling"]);
    assert_eq!(git(&w, &["status", "--porcelain"]), "");
}

#[test]
fn a_group_save_commits_only_its_files_and_preserves_other_staged_work() {
    let root = tmp("agent-group");
    let mut h = History::turn_on(&root).unwrap();
    h.set_identity("Group Test", "group@example.com").unwrap();
    std::fs::create_dir(root.join("parts")).unwrap();
    for name in ["parts/a.fg", "parts/b.fg", "other.txt"] {
        std::fs::write(root.join(name), "before").unwrap();
    }
    h.save_group(
        &[
            root.join("parts/a.fg"),
            root.join("parts/b.fg"),
            root.join("other.txt"),
        ],
        "Baseline",
        None,
    )
    .unwrap();
    std::fs::write(root.join("other.txt"), "staged unrelated edit").unwrap();
    git(&root, &["add", "other.txt"]);
    for name in ["parts/a.fg", "parts/b.fg"] {
        std::fs::write(root.join(name), "after").unwrap();
    }
    let Saved::New(id) = h
        .save_group(
            &[root.join("parts/a.fg"), root.join("parts/b.fg")],
            "Agent changed two parts",
            Some("codex"),
        )
        .unwrap()
    else {
        panic!("one commit")
    };
    assert_eq!(h.repo.file_at(&id, "parts/a.fg").unwrap().unwrap(), b"after");
    assert_eq!(h.repo.file_at(&id, "parts/b.fg").unwrap().unwrap(), b"after");
    assert_eq!(h.repo.file_at(&id, "other.txt").unwrap().unwrap(), b"before");
    assert_eq!(
        git(&root, &["diff", "--cached", "--name-only"]).trim(),
        "other.txt"
    );
    assert_eq!(h.log().unwrap()[0].agent.as_deref(), Some("codex"));
    assert_eq!(
        h.save_group(
            &[root.join("parts/a.fg"), root.join("parts/b.fg")],
            "Again",
            Some("codex")
        )
        .unwrap(),
        Saved::Unchanged
    );
    std::fs::remove_dir_all(root).unwrap();
}
