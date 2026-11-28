//! Run the real worker in an isolated user configuration. WASI cannot spawn a worker process.
#![cfg(not(target_family = "wasm"))]

use forge_lang::{electronics_library as sdk, pcb, schematic as sch};
use std::{path::Path, process::Command};

fn exercise() {
    let exe = Path::new(env!("CARGO_BIN_EXE_electronics-worker"));
    assert_eq!(sdk::install(exe).unwrap(), "rust:fixture@1");
    assert!(sdk::install(exe).is_err());
    let source = r#"schematic
lib=library(file:"rust:fixture@1")
same=library(file:"rust:fixture@1/checks")
chip=symbol(pins:{IO:pin(number:"1",kind:"bidirectional",at:(0,0),specs:[{kind:"pull_up",state:"configurable"}],functions:{sense:function(kind:"input",specs:[{kind:"leakage_current",max:5,unit:"nA"}])})})
main=sheet()
u=place(sheet:main,symbol:chip,at:(0,0),fields:{Manufacturer:"Acme"},functions:{IO:sense})
v=place(sheet:main,symbol:chip,at:(10,0))
"#;
    let (doc, c) = sch::compile(source, None, None).unwrap();
    assert!(
        c.issues
            .iter()
            .any(|i| i.kind == "fixture.Ratings" && i.objects == ["main/u.IO"])
    );
    let withheld: Vec<_> = c
        .not_run
        .iter()
        .filter(|r| r.rule.starts_with("fixture."))
        .collect();
    assert_eq!(withheld.len(), 2);
    assert!(
        withheld
            .iter()
            .any(|r| r.rule == "fixture.Part" && r.object == "main/v" && r.fields == ["Manufacturer"])
    );
    assert!(withheld.iter().any(|r| r.rule == "fixture.Ratings"
        && r.object == "main/v.IO"
        && r.fields == ["selected function"]));
    let findings = c
        .issues
        .iter()
        .filter(|i| i.kind == "fixture.Part")
        .collect::<Vec<_>>();
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].message, "batch 0");
    assert_eq!(findings[0].priority(&doc), "warning");
    let board = pcb::compile(
        r#"pcb
policy=drc_rule(rule:"fixture.Track",severity:"info")
lib=library(file:"rust:fixture@1/checks")
outline=edge(points:[(0,0),(20,0),(20,20),(0,20)])
top=copper(weight_oz_ft2:1,thickness:0.035)
core=dielectric(material:"FR4",thickness:1.5)
bottom=copper(weight_oz_ft2:1,thickness:0.035)
stack=stackup(layers:[top,core,bottom])
vin=net(name:"48V")
feed=track(layer:top,net:vin,width:0.25,points:[(2,2),(6,2)])
"#,
        None,
        None,
    )
    .unwrap();
    let report = board.check();
    let row = report
        .findings
        .iter()
        .find(|f| f.rule.name() == "fixture.Track")
        .unwrap();
    assert_eq!(board.severity(row.rule), "info");
    assert!(
        matches!(&row.witness,pcb::Witness::Library{message,objects} if message=="batch 1" && objects==&["feed"])
    );
    assert_eq!(
        sch::compile(&source.replace("fixture@1", "fixture@2"), None, None)
            .unwrap_err()
            .kind,
        "RustLibraryUnavailable"
    );
}
#[test]
fn installed_library_rules_and_evidence() {
    if std::env::var_os("FORGE_TEST_INSTALLED_LIBRARY_CHILD").is_some() {
        exercise();
        return;
    }
    let dir = std::env::temp_dir().join(format!(
        "forge-worker-test-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let status = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "installed_library_rules_and_evidence", "--nocapture"])
        .env("FORGE_TEST_INSTALLED_LIBRARY_CHILD", "1")
        .env("XDG_CONFIG_HOME", &dir)
        .status()
        .unwrap();
    std::fs::remove_dir_all(dir).unwrap();
    assert!(status.success());
}
