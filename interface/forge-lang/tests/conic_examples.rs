//! examples/conic: each part builds valid, its volume that of the same part built in OpenCascade
//! from the exact conic profile (research/fillets/conic/occt_conic.py, adaptive integration to
//! 1e-12), or of the closed form where the part has one.

use forge_lang::Options;

fn volume(name: &str) -> f64 {
    let path = format!("{}/../../examples/conic/{name}.fg", env!("CARGO_MANIFEST_DIR"));
    let src = std::fs::read_to_string(&path).expect("example");
    let r = forge_lang::run(&src, Options::default(), Default::default()).0;
    assert!(r.errors.is_empty(), "{name}: {:?}", r.errors);
    let m = r.measure.as_ref().expect("measured");
    assert_eq!(m.problems, 0, "{name}: invalid");
    m.volume
}

#[test]
fn conic_examples_match_their_references() {
    // (part, reference, what it is)
    let cases = [
        // H (L W) less H r^2/2 (1 - 2/3): a parabola's corner at a right angle
        ("edge", 40.0 * 30.0 * 20.0 - 20.0 * 12.5 / 3.0, "closed form"),
        // the four prisms less what two take at each corner (fillet_conic.rs, mitred_rim)
        ("rim", 23_649.498_758_537_1, "closed form"),
        ("boss", 32_649.222_203_9, "OCCT, turned profile"),
        ("slot", 8_220.875_941_39, "OCCT, swept and turned"),
        ("step", 15_573.608_017_9, "OCCT, swept profile"),
        ("wedge", 28_189.629_935_7, "OCCT, swept profile"),
    ];
    for (name, want, what) in cases {
        let got = volume(name);
        assert!(
            (got - want).abs() <= 1e-9 * want,
            "{name}: {got} against {want} ({what})"
        );
    }
}
