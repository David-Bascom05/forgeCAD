//! The examples in examples/machine: the slot and the turned bar pass, the crash is refused at
//! its line.
use std::path::Path;

fn run(f: &str) -> forge_machine_lang::Outcome {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/machine")
        .join(f);
    forge_machine_lang::run_setup(&p).unwrap()
}

#[test]
fn examples_answer_as_documented() {
    let o = run("vmc_slot.toml");
    assert!(o.ok, "{}", o.text);
    assert!(
        o.json
            .starts_with("{\"format\":\"forgecad.machine/1\",\"ok\":true")
    );
    let o = run("vmc_crash.toml");
    assert!(!o.ok);
    assert!(
        o.text.contains("line 15: rapid into material, 5.000 mm deep"),
        "{}",
        o.text
    );
    let o = run("lathe_turn.toml");
    assert!(o.ok, "{}", o.text);
}
