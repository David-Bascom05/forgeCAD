//! git itself as the judge: repos written here pass `git fsck --strict` and read back through
//! `git`, and repos git wrote (loose, packed, deltified, SHA-1 and SHA-256) read back here.

use forge_git::object::{self, Kind};
use forge_git::{HashKind, Oid, Repo, Sig};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static N: AtomicU32 = AtomicU32::new(0);

fn tmp(name: &str) -> PathBuf {
    let d = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "forge-git-{name}-{}-{}",
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
        .env("GIT_AUTHOR_NAME", "T")
        .env("GIT_AUTHOR_EMAIL", "t@x")
        .env("GIT_COMMITTER_NAME", "T")
        .env("GIT_COMMITTER_EMAIL", "t@x")
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
        tz: 60,
    }
}

fn ours(kind: HashKind) {
    let w = tmp("ours");
    let r = Repo::init(&w, kind).unwrap();
    std::fs::create_dir_all(w.join("parts/sub")).unwrap();
    std::fs::write(w.join("parts/bracket.fg"), "w = 40\n").unwrap();
    std::fs::write(w.join("parts/sub/a.fg"), "x = 1\n").unwrap();
    std::fs::write(w.join("top.fg"), "assembly\n").unwrap();
    let paths: Vec<String> = ["parts/bracket.fg", "parts/sub/a.fg", "top.fg"]
        .map(String::from)
        .to_vec();
    let c1 = r
        .commit_paths(&paths, "first", &who(1_700_000_000))
        .unwrap()
        .unwrap();
    assert!(c1.parent.is_none());
    assert!(
        r.commit_paths(&paths, "again", &who(1_700_000_001))
            .unwrap()
            .is_none()
    );
    std::fs::write(w.join("parts/bracket.fg"), "w = 42\n").unwrap();
    let c2 = r
        .commit_paths(&paths[..1], "bracket: w 40 -> 42", &who(1_700_000_002))
        .unwrap()
        .unwrap();
    assert_eq!(c2.changed, ["parts/bracket.fg"]);
    assert!(c2.index_updated);
    std::fs::remove_file(w.join("parts/sub/a.fg")).unwrap();
    let c3 = r
        .commit_paths(&paths[1..2], "remove a", &who(1_700_000_003))
        .unwrap()
        .unwrap();
    r.tag(
        "forgecad/v/parts%2Fbracket.fg/1",
        &c2.id,
        "Bracket rev A\n\nnotes\n",
        &who(1_700_000_004),
    )
    .unwrap();
    assert!(
        r.tag("forgecad/v/parts%2Fbracket.fg/1", &c3.id, "again", &who(5))
            .is_err()
    );

    git(&w, &["fsck", "--strict", "--no-dangling"]);
    let log = git(&w, &["log", "--format=%H %s"]);
    assert_eq!(log.lines().count(), 3);
    assert!(log.starts_with(&format!("{} remove a", c3.id)));
    assert_eq!(git(&w, &["show", "HEAD~1:parts/bracket.fg"]), "w = 42\n");
    assert_eq!(git(&w, &["status", "--porcelain"]), "");
    assert_eq!(
        git(&w, &["log", "--format=%H", "--", "parts/bracket.fg"])
            .lines()
            .count(),
        2
    );
    assert!(git(&w, &["cat-file", "-p", "forgecad/v/parts%2Fbracket.fg/1"]).contains("Bracket rev A"));
    assert!(git(&w, &["reflog"]).contains("bracket: w 40 -> 42"));

    // and back: what git says of our history, we say too
    let h = forge_git::walk::path_history(&r.odb, &c3.id, "parts/bracket.fg", 100).unwrap();
    let ids: Vec<Oid> = h.iter().map(|x| x.id).collect();
    assert_eq!(ids, [c2.id, c1.id]);
    assert_eq!(
        r.file_at(&c1.id, "parts/bracket.fg").unwrap().unwrap(),
        b"w = 40\n"
    );
}

#[test]
fn our_repos_pass_git_sha1() {
    ours(HashKind::Sha1);
}

#[test]
fn our_repos_pass_git_sha256() {
    ours(HashKind::Sha256);
}

fn theirs(format: &str) {
    let w = tmp("theirs");
    git(
        &w,
        &["init", "-q", "-b", "main", &format!("--object-format={format}")],
    );
    // one fast-import stream: 60 commits of a growing file and small ones, and a tag
    let mut st = String::new();
    let mut text = String::new();
    let data = |s: &mut String, d: &str| s.push_str(&format!("data {}\n{d}\n", d.len()));
    for i in 0..60 {
        for k in 0..40 {
            text.push_str(&format!(
                "line {k} of a growing file, edit {}\n",
                if k == i % 40 { i } else { 0 }
            ));
        }
        st.push_str(&format!(
            "commit refs/heads/main\ncommitter T <t@x> {} +0000\n",
            1_700_000_000 + i
        ));
        data(&mut st, &format!("c{i}"));
        st.push_str("M 100644 inline f.txt\n");
        data(&mut st, &text);
        st.push_str(&format!("M 100644 inline n{}.txt\n", i % 7));
        data(&mut st, &format!("{i}"));
    }
    st.push_str("tag t1\nfrom refs/heads/main\ntagger T <t@x> 1700000100 +0000\n");
    data(&mut st, "a tag");
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
    git(&w, &["repack", "-adq", "--depth=50", "--window=10"]);
    git(&w, &["checkout", "-q", "main"]);
    // and some loose ones on top
    for i in 0..5 {
        std::fs::write(w.join("loose.txt"), format!("{i}\n")).unwrap();
        git(&w, &["add", "-A"]);
        git(&w, &["commit", "-q", "-m", &format!("l{i}")]);
    }
    let all = git(
        &w,
        &[
            "cat-file",
            "--batch-all-objects",
            "--batch-check=%(objectname) %(objecttype)",
        ],
    );
    let r = Repo::discover(&w.join("f.txt")).unwrap();
    let kind = if format == "sha256" {
        HashKind::Sha256
    } else {
        HashKind::Sha1
    };
    assert_eq!(r.hash, kind);
    assert!(!r.odb.packs().is_empty());
    let mut n = 0;
    for line in all.lines() {
        let (h, t) = line.split_once(' ').unwrap();
        let id = Oid::from_hex(kind, h).unwrap();
        let o = r.odb.read(&id).unwrap();
        assert_eq!(o.kind.name(), t);
        assert_eq!(object::id_of(kind, o.kind, &o.data), id);
        // parse and write back byte for byte
        let back = match o.kind {
            Kind::Tree => object::write_tree(&object::parse_tree(kind, &o.data).unwrap()),
            Kind::Commit | Kind::Tag => object::Headed::parse(&o.data).unwrap().write(),
            Kind::Blob => o.data.clone(),
        };
        assert_eq!(back, o.data, "{id}");
        n += 1;
    }
    assert!(n > 100);
    let head = r.head().unwrap().unwrap();
    assert_eq!(head.hex(), git(&w, &["rev-parse", "HEAD"]).trim());
    let tags = r.refs.list("refs/tags/").unwrap();
    assert_eq!(tags.len(), 1);
    let ours: Vec<String> = forge_git::walk::path_history(&r.odb, &head, "n3.txt", 1000)
        .unwrap()
        .iter()
        .map(|c| c.id.hex())
        .collect();
    let theirs: Vec<String> = git(&w, &["log", "--format=%H", "--", "n3.txt"])
        .lines()
        .map(String::from)
        .collect();
    assert_eq!(ours, theirs);
    let walked = forge_git::walk::Walk::new(&r.odb, &[head]).unwrap().count();
    assert_eq!(walked, 65);
}

#[test]
fn git_repos_read_sha1() {
    theirs("sha1");
}

#[test]
fn git_repos_read_sha256() {
    theirs("sha256");
}

#[test]
fn our_commits_keep_a_git_index_in_step() {
    let w = tmp("index");
    git(&w, &["init", "-q", "-b", "main"]);
    for f in ["a.fg", "b.fg", "notes.txt"] {
        std::fs::write(w.join(f), f).unwrap();
    }
    git(&w, &["add", "-A"]);
    git(&w, &["commit", "-q", "-m", "base"]);
    // someone's staged work, and someone's unstaged work, both left as they are
    std::fs::write(w.join("notes.txt"), "staged").unwrap();
    git(&w, &["add", "notes.txt"]);
    std::fs::write(w.join("b.fg"), "unstaged").unwrap();
    git(&w, &["update-index", "--index-version", "4"]);
    let r = Repo::discover(&w).unwrap();
    std::fs::write(w.join("a.fg"), "saved").unwrap();
    let c = r
        .commit_paths(&["a.fg".into()], "save a", &who(1_700_000_100))
        .unwrap()
        .unwrap();
    assert!(c.index_updated);
    git(&w, &["fsck", "--strict", "--no-dangling"]);
    let st = git(&w, &["status", "--porcelain"]);
    assert!(st.contains("M  notes.txt"));
    assert!(st.contains(" M b.fg"));
    assert!(!st.contains("a.fg"), "{st}");
    assert_eq!(git(&w, &["show", "HEAD:a.fg"]), "saved");
}

#[test]
fn gitignore_rules() {
    let dir = tmp("ign");
    std::fs::create_dir_all(dir.join("sub/deep")).unwrap();
    std::fs::write(dir.join(".gitignore"), "*.stl\n/exports/\nbuild\n!keep.stl\n").unwrap();
    std::fs::write(dir.join("sub/.gitignore"), "!b.stl\nlocal.txt\n").unwrap();
    let mut g = forge_git::ignore::Ignore::new(&dir, &dir.join("no-git"), Some(dir.join("none")));
    for (p, want) in [
        ("a.stl", true),
        ("keep.stl", false),
        ("exports/x.step", true),
        ("sub/exports/x.step", false),
        ("sub/deep/build/x", true),
        ("sub/b.stl", false),
        ("sub/local.txt", true),
        ("part.fg", false),
    ] {
        assert_eq!(g.ignored(p, false), want, "{p}");
    }
    // git agrees
    git(&dir, &["init", "-q"]);
    for (p, want) in [
        ("a.stl", true),
        ("keep.stl", false),
        ("sub/b.stl", false),
        ("sub/deep/build/x", true),
    ] {
        let o = Command::new("git")
            .arg("-C")
            .arg(&dir)
            .args(["check-ignore", "-q", "--no-index", p])
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .status()
            .unwrap();
        assert_eq!(o.success(), want, "git on {p}");
    }
}
