//! The local-imprint boolean against the general one: every boolean it takes on these parts is
//! done both ways and must come out the same, entity for entity.
use forge_bool::imprint::{Mode, set_mode, taken};
use forge_model::{suite, torture};

#[test]
fn imprint_matches_the_general_boolean() {
    set_mode(Mode::Check);
    torture::part(221).regenerate().expect("torture regenerates");
    let on_torture = taken();
    for name in suite::NAMES {
        let _ = suite::part(name).map(|p| p.regenerate());
    }
    set_mode(Mode::Off);
    assert!(
        on_torture >= 100,
        "only {on_torture} booleans took the imprint path"
    );
}
