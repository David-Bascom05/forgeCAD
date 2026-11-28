//! Test fixture implementing the installed Rust electronics library protocol.
use forge_lang::{
    electronics_library as sdk,
    electronics_library::worker::{Check, Diagnostic, Fact, Input},
};
use std::sync::atomic::{AtomicUsize, Ordering};

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
fn main() {
    sdk::worker::serve(&ROOT, "1").unwrap();
}
