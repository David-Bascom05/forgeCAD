//! This test binary doubles as a separately installed worker. The consumer is a fresh
//! process with an isolated user configuration, with no environment mutation in the host.
use forge_lang::{
    electronics_library as sdk,
    electronics_library::worker::{Check, Diagnostic, Fact, Input},
    pcb, schematic as sch,
};
use std::{
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
};

fn batch(rows: &[Fact]) -> Vec<Diagnostic> {
    static BATCH: AtomicUsize = AtomicUsize::new(0);
    let n = BATCH.fetch_add(1, Ordering::SeqCst);
    rows.iter()
        .filter_map(|r| {
            let id = match r {
                Fact::Component { id, .. } | Fact::Track { id, .. } => id,
                _ => return None,
            };
            Some(Diagnostic {
                objects: vec![id.clone()],
                message: format!("batch {n}"),
            })
        })
        .collect()
}
fn ratings(rows: &[Fact]) -> Vec<Diagnostic> {
    rows.iter()
        .filter_map(|row| {
            let Fact::PinSpecifications {
                pin,
                active,
                base,
                function,
                ..
            } = row
            else {
                return None;
            };
            sdk::evidence::require("fixture.Ratings", pin, "selected function", function.as_ref())?;
            (function.as_deref() == Some("sense")
                && base.to_string().contains("pull_up")
                && active.to_string().contains("leakage_current"))
            .then(|| Diagnostic {
                objects: vec![pin.clone()],
                message: "conditional pin data received".into(),
            })
        })
        .collect()
}
static LEAF: sdk::Library = sdk::Library::new("checks")
    .with_checks(&[
        Check {
            rule: sdk::RuleSpec {
                id: "fixture.Part",
                severity: sdk::Severity::Warning,
            },
            input: Input::Components,
            evaluate: batch,
        },
        Check {
            rule: sdk::RuleSpec {
                id: "fixture.Ratings",
                severity: sdk::Severity::Info,
            },
            input: Input::PinSpecifications,
            evaluate: ratings,
        },
        Check {
            rule: sdk::RuleSpec {
                id: "fixture.Track",
                severity: sdk::Severity::Error,
            },
            input: Input::Tracks,
            evaluate: batch,
        },
    ])
    .with_requirements(&[sdk::worker::RequiredField {
        rule: "fixture.Part",
        field: "Manufacturer",
        symbol: Some("chip"),
    }]);
static ROOT: sdk::Library = sdk::Library::new("fixture").with_children(&[&LEAF]);
fn exercise() {
    let exe = std::env::current_exe().unwrap();
    assert_eq!(sdk::install(&exe).unwrap(), "rust:fixture@1");
    assert!(sdk::install(&exe).is_err());
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
fn main() {
    match std::env::args().nth(1).as_deref() {
        Some("--manifest" | "--worker") => sdk::worker::serve(&ROOT, "1").unwrap(),
        Some("--exercise") => exercise(),
        _ => {
            let dir = std::env::temp_dir().join(format!(
                "forge-worker-test-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            let status = Command::new(std::env::current_exe().unwrap())
                .arg("--exercise")
                .env("XDG_CONFIG_HOME", &dir)
                .status()
                .unwrap();
            std::fs::remove_dir_all(dir).unwrap();
            assert!(status.success());
        }
    }
}
