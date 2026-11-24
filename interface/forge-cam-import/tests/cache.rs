//! The makers' cached Fusion libraries (tools/fetch-tool-libraries.sh), opt-in: runs only when
//! FORGECAD_TOOL_LIBRARIES names the cache. Each imports with cites and presets and reads back
//! into forge-tooling; only aggregate counts are printed (nothing from the libraries is kept).
use forge_cam_import::load_dir;

#[test]
fn cached_libraries_import() {
    let Some(dir) = std::env::var_os("FORGECAD_TOOL_LIBRARIES") else {
        eprintln!("FORGECAD_TOOL_LIBRARIES not set: skipped");
        return;
    };
    let libs = load_dir(std::path::Path::new(&dir), "2026-11-24").unwrap();
    assert!(!libs.is_empty());
    let (mut tools, mut presets) = (0, 0);
    for (name, imp) in &libs {
        let mut lib = forge_tooling::Library::default();
        lib.add(name, &imp.toml).unwrap_or_else(|e| panic!("{name}: {e}"));
        let p: usize = lib.tools.iter().map(|t| t.presets.len()).sum();
        eprintln!(
            "{name}: {} tools, {p} presets, {} warnings",
            lib.tools.len(),
            imp.warnings.len()
        );
        tools += lib.tools.len();
        presets += p;
    }
    eprintln!(
        "total: {} libraries, {tools} tools, {presets} presets",
        libs.len()
    );
    assert!(tools > 0);
}
