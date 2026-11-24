//! As git: a repository's format (`extensions.objectFormat`, `core.repositoryformatversion`) and
//! `core.bare` are its own config's, never a global one's. A global config that sets them (here
//! through GIT_CONFIG_GLOBAL) leaves a sha1 repository with a work tree read as one; other global
//! settings are still read. The test runs itself again as a child with that environment.
#![cfg(not(target_os = "wasi"))]

use forge_git::{HashKind, Repo};

#[test]
fn a_global_config_cannot_change_a_repositorys_format_or_bareness() {
    let dir = std::env::temp_dir().join(format!("forge-git-global-format-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let work = dir.join("work");
    std::fs::create_dir_all(&work).unwrap();
    Repo::init(&work, HashKind::Sha1).unwrap();
    let global = dir.join("gitconfig");
    std::fs::write(
        &global,
        "[core]\n\tbare = true\n\trepositoryformatversion = 1\n[extensions]\n\tobjectFormat = sha256\n\
         \tnoSuchExtension = true\n[user]\n\tname = Global Name\n",
    )
    .unwrap();
    let out = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "opened_under_a_global_config", "--nocapture"])
        .env("GIT_CONFIG_GLOBAL", &global)
        .env("FORGE_GIT_TEST_WORK", &work)
        .output()
        .unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success() && text.contains("1 passed"),
        "{text}{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// The child: run only by the test above, with GIT_CONFIG_GLOBAL set (a no-op otherwise).
#[test]
fn opened_under_a_global_config() {
    let Some(work) = std::env::var_os("FORGE_GIT_TEST_WORK").map(std::path::PathBuf::from) else {
        return;
    };
    let r = Repo::open(&work.join(".git"), Some(&work)).unwrap();
    assert_eq!(r.hash, HashKind::Sha1);
    assert!(r.work.is_some(), "a global core.bare made it bare");
    // the global config is still a config: its other settings are read
    let cfg = forge_git::config::Config::load(&work.join(".git"), &work.join(".git"));
    assert_eq!(cfg.get("user.name"), Some("Global Name"));
}
