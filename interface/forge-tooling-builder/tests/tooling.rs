//! The tooling kinds on the library builder: every built-in record written and read back the
//! same, and the CLI deriving, listing and checking custom records.
use forge_library::Kind;
use forge_tooling_builder::cli::{self, Env};
use forge_tooling_builder::{KINDS, builtin_of, parse_kind, write};

#[test]
fn every_builtin_record_round_trips_byte_stable() {
    let mut n = 0;
    for (kind, _, _) in KINDS {
        for (_, text) in builtin_of(kind) {
            let recs = parse_kind(kind, &text).unwrap_or_else(|e| panic!("{kind}: {e}"));
            let again = write::file(kind, "", &recs).unwrap();
            assert_eq!(again, text, "{kind}: not byte-stable");
            n += recs.len();
        }
    }
    let lib = forge_tooling::Library::builtin();
    assert_eq!(
        n,
        lib.sources
            .iter()
            .filter(|(k, _, _)| KINDS.iter().any(|(w, _, _)| w == k))
            .count()
    );
    assert!(n > 100, "{n} records");
}

fn tmp(t: &str) -> std::path::PathBuf {
    let d = forge_dirs::temp_dir().join(format!("fmtb-{t}-{}", forge_dirs::process_id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn run(e: &Env, word: &str, args: &[&str]) -> cli::Out {
    let a: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    cli::run(e, word, &a).unwrap()
}

#[test]
fn derive_list_check_a_custom_tool_and_holder() {
    let root = tmp("cli");
    let e = Env {
        project: root.join("proj"),
        user_cam: Some(root.join("user")),
    };
    let lib = forge_tooling::Library::builtin();
    let src = lib.tools.iter().find(|t| t.re == 0.0 && t.dc > 6.0).unwrap();
    let o = run(
        &e,
        "tool",
        &["new", "my-endmill", "--from", &src.id, "oal=88", "--project"],
    );
    assert_eq!(o.code, 0, "{}", o.err);
    assert!(o.out.contains("values yours (user)"), "{}", o.out);
    let f = root.join("proj/cam/tools/my-endmill.toml");
    let text = std::fs::read_to_string(&f).unwrap();
    // byte-stable on disk
    let recs = forge_tooling_builder::Tools::parse(&text).unwrap();
    assert_eq!(recs[0].id, "my-endmill");
    let header = forge_tooling_builder::Tools::header("my-endmill");
    assert_eq!(forge_tooling_builder::Tools::write(&header, &recs).unwrap(), text);
    let forge_tooling::build::Record::Tool(t) = &recs[0].typed else {
        panic!()
    };
    assert_eq!(t.oal, 88.0);
    assert!(
        t.cites
            .iter()
            .any(|c| c.path == "oal" && format!("{:?}", c.status) == "User")
    );
    // listed at the project level, checked clean
    let o = run(&e, "tool", &["list"]);
    assert!(
        o.out
            .lines()
            .any(|l| l.starts_with("my-endmill") && l.contains("project")),
        "{}",
        o.out
    );
    let o = run(&e, "tool", &["check", "my-endmill"]);
    assert_eq!(o.code, 0, "{}", o.out);
    // a corner radius past half the diameter: refused by the check with the fix
    let o = run(
        &e,
        "tool",
        &[
            "new",
            "bad-bull",
            "--from",
            &src.id,
            &format!("re={}", src.dc),
            "--project",
        ],
    );
    if o.code == 0 {
        let o = run(&e, "tool", &["check", "bad-bull", "--json"]);
        assert_eq!(o.code, 1, "{}", o.out);
        assert!(o.out.contains("\"path\":\"re\""), "{}", o.out);
    }
    // a holder too, at the user level
    let h = &lib.holders[0].id;
    let o = run(&e, "holder", &["new", "my-holder", "--from", h]);
    assert_eq!(o.code, 0, "{}", o.err);
    assert!(root.join("user/holders/my-holder.toml").is_file());
    // the levels feed the library CAM reads
    let (all, errs) = forge_tooling_builder::library(Some(&e.project), e.user_cam.as_deref());
    assert!(errs.is_empty(), "{errs:?}");
    assert!(all.tools.iter().any(|t| t.id == "my-endmill"));
    assert!(all.holders.iter().any(|t| t.id == "my-holder"));
    // refusals
    assert_eq!(
        run(&e, "tool", &["new", "my-endmill", "--from", &src.id, "--project"]).code,
        1
    );
    assert_eq!(run(&e, "tool", &["new", "x", "--from", "no-such-tool"]).code, 2);
    let _ = std::fs::remove_dir_all(&root);
}
