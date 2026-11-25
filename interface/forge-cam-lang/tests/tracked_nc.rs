//! Every posted program kept in the repository (examples/machined/*_cam.nc) is what its document
//! posts now, byte for byte: a change that moves a program must re-bless it in the same commit
//! (judged), never leave it stale. (flange_cam.nc and plate_break_cam.nc went stale twice, after
//! merges that changed feeds, with nothing to say so.)
mod common;
use common::dir;

// git, files: native only (the wasm runner has neither)
#[cfg(not(target_os = "wasi"))]
#[test]
fn the_kept_programs_are_what_their_documents_post() {
    let mut checked = 0;
    let mut stale = vec![];
    for e in std::fs::read_dir(dir()).unwrap().flatten() {
        let p = e.path();
        let name = p.file_name().unwrap().to_string_lossy().to_string();
        let Some(stem) = name.strip_suffix("_cam.nc") else {
            continue;
        };
        // only the programs kept in the repository (others are written by runs and ignored)
        let tracked = std::process::Command::new("git")
            .args(["ls-files", "--error-unmatch", &name])
            .current_dir(dir())
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if !tracked {
            continue;
        }
        let src = std::fs::read_to_string(dir().join(format!("{stem}_cam.fg"))).unwrap();
        let cam = forge_cam_lang::run(&src, &dir());
        let now = cam.program_text().map(|(t, _)| t.to_string()).unwrap_or_default();
        let kept = std::fs::read_to_string(&p).unwrap();
        if now != kept {
            stale.push(name);
        }
        checked += 1;
    }
    assert!(checked >= 2, "no kept program found (git missing?)");
    assert!(
        stale.is_empty(),
        "stale posted programs (re-bless them in the commit that moved them: `forgecad cam F_cam.fg` writes F_cam.nc): {stale:?}"
    );
}
