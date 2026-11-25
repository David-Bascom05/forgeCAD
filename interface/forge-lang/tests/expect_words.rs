//! A failed `expect` reads in words (`expected no interferences, found 1`), its line as written in
//! the detail (`expect interferences = 0`): every comparison, counts none, one and many, amounts
//! with their units, an `=`'s tolerance, an angle in degrees, `valid`; and the same words from a
//! real part's and a real assembly's run.

use forge_lang::expect_words::{raw, raw_in, words, words_in};
use forge_lang::syntax::Cmp::{Eq, Ge, Gt, Le, Lt};

#[test]
fn counts_say_none_one_and_many() {
    let w = |c, l, m| words("interferences", c, l, 0.0, m);
    assert_eq!(w(Eq, 0.0, 1.0), "expected no interferences, found 1");
    assert_eq!(w(Eq, 1.0, 3.0), "expected 1 interference, found 3");
    assert_eq!(w(Eq, 2.0, 0.0), "expected 2 interferences, found 0");
    assert_eq!(w(Lt, 1.0, 2.0), "expected no interferences, found 2");
    assert_eq!(w(Lt, 2.0, 2.0), "expected fewer than 2 interferences, found 2");
    assert_eq!(w(Le, 0.0, 1.0), "expected no interferences, found 1");
    assert_eq!(w(Le, 1.0, 2.0), "expected at most 1 interference, found 2");
    assert_eq!(w(Le, 3.0, 4.0), "expected at most 3 interferences, found 4");
    assert_eq!(w(Gt, 0.0, 0.0), "expected at least 1 interference, found 0");
    assert_eq!(w(Gt, 1.0, 1.0), "expected more than 1 interference, found 1");
    assert_eq!(w(Gt, 2.0, 1.0), "expected more than 2 interferences, found 1");
    assert_eq!(w(Ge, 1.0, 0.0), "expected at least 1 interference, found 0");
    assert_eq!(w(Ge, 5.0, 4.0), "expected at least 5 interferences, found 4");
    assert_eq!(
        words("mobility", Eq, 1.0, 0.0, 0.0),
        "expected 1 degree of freedom, found 0"
    );
    assert_eq!(
        words("mobility", Eq, 0.0, 0.0, 2.0),
        "expected no degrees of freedom, found 2"
    );
    assert_eq!(words("faces", Eq, 12.0, 0.0, 11.0), "expected 12 faces, found 11");
    assert_eq!(words("bodies", Eq, 1.0, 0.0, 2.0), "expected 1 body, found 2");
    assert_eq!(words("holes", Ge, 4.0, 0.0, 3.0), "expected at least 4 holes, found 3");
}

#[test]
fn amounts_say_their_unit_and_comparison() {
    assert_eq!(
        words("clearance", Ge, 2.0, 0.0, 1.4),
        "expected clearance at least 2 mm, found 1.4 mm"
    );
    assert_eq!(words("clearance", Gt, 2.0, 0.0, 2.0), "expected clearance over 2 mm, found 2 mm");
    assert_eq!(words("gap", Le, 0.5, 0.0, 0.7), "expected gap at most 0.5 mm, found 0.7 mm");
    assert_eq!(words("distance", Lt, 10.0, 0.0, 12.0), "expected distance under 10 mm, found 12 mm");
    assert_eq!(words("x", Eq, 10.0, 0.0, 9.8), "expected x of 10 mm, found 9.8 mm");
    assert_eq!(
        words("volume", Eq, 11966.0, 12.0, 11800.0),
        "expected volume of 11966 mm³ ± 12 mm³, found 11800 mm³"
    );
    assert_eq!(words("area", Ge, 100.0, 0.0, 90.0), "expected area at least 100 mm², found 90 mm²");
    assert_eq!(
        words("interference", Eq, 0.0, 0.0, 3.5),
        "expected interference volume of 0 mm³, found 3.5 mm³"
    );
    assert_eq!(words("axis_angle", Le, 1.0, 0.0, 2.5), "expected axis angle at most 1°, found 2.5°");
    assert_eq!(words("valid", Eq, 1.0, 0.0, 0.0), "expected a valid solid, found an invalid one");
}

#[test]
fn an_angle_limit_is_said_in_degrees() {
    let r = |d: f64| d.to_radians();
    assert_eq!(
        words_in("value", Eq, r(-20.0), r(0.5), r(-18.0), true),
        "expected value of -20° ± 0.5°, found -18°"
    );
    assert_eq!(raw_in("value", Eq, r(-20.0), r(0.5), true), "value = -20deg +- 0.5deg");
}

#[test]
fn the_line_as_written() {
    assert_eq!(raw("interferences", Eq, 0.0, 0.0), "interferences = 0");
    assert_eq!(raw("clearance", Ge, 2.0, 0.0), "clearance >= 2");
    assert_eq!(raw("gap", Le, 0.5, 0.0), "gap <= 0.5");
    assert_eq!(raw("gap", Lt, 0.5, 0.0), "gap < 0.5");
    assert_eq!(raw("gap", Gt, 0.5, 0.0), "gap > 0.5");
    assert_eq!(raw("volume", Eq, 100.0, 1.5), "volume = 100 +- 1.5");
    assert_eq!(raw("valid", Eq, 1.0, 0.0), "valid");
}

#[test]
fn a_part_says_its_failed_expect_in_words() {
    let src = "sketch s(on: xy) {\n  o = rect(corner: (0, 0), size: (10, 10))\n}\nb = extrude(s, 5)\nexpect faces = 7\nexpect volume = 400 +- 1\n";
    let (r, _) = forge_lang::run(src, forge_lang::Options::default(), Default::default());
    let failed: Vec<_> = r.errors.iter().filter(|e| e.kind() == "ExpectFailed").collect();
    let texts: Vec<String> = failed.iter().map(|e| e.text()).collect();
    assert_eq!(
        texts,
        [
            "expected 7 faces, found 6",
            "expected volume of 400 mm³ ± 1 mm³, found 500 mm³"
        ],
        "{texts:?}"
    );
    let details: Vec<Option<String>> = failed.iter().map(|e| e.detail()).collect();
    assert_eq!(
        details,
        [Some("expect faces = 7".to_string()), Some("expect volume = 400 +- 1".to_string())]
    );
}

#[test]
fn an_assembly_says_its_failed_expect_in_words() {
    let d = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("expect-words-asm");
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(
        d.join("cube.fg"),
        "sketch s(on: xy) {\n  o = rect(corner: (0, 0), size: (10, 10))\n}\nb = extrude(s, 10)\n",
    )
    .unwrap();
    // two cubes overlapping by half: one interference
    let src = "a = part(\"cube.fg\", ground: true)\nb = part(\"cube.fg\", at: (5, 0, 0), ground: true)\nexpect interferences = 0\n";
    let opts = forge_lang::Options {
        dir: Some(d),
        ..forge_lang::Options::default()
    };
    let (r, _) = forge_lang::run(src, opts, Default::default());
    let failed: Vec<_> = r.errors.iter().filter(|e| e.kind() == "ExpectFailed").collect();
    assert_eq!(failed.len(), 1, "{:?}", r.errors.iter().map(|e| e.text()).collect::<Vec<_>>());
    assert_eq!(failed[0].text(), "expected no interferences, found 1");
    assert_eq!(failed[0].detail().as_deref(), Some("expect interferences = 0"));
}
