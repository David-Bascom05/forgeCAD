//! The press-brake kinds: canonical round trip over the whole built-in library, the forms, the
//! templates, the checks and the previews.
use forge_brake_builder::{Dies, Presses, Punches, check, fields, preview, templates};
use forge_library::Kind;

fn round_trip<K: Kind<Record = forge_brake_builder::Rec>>() -> usize {
    let mut n = 0;
    for (file, text) in K::builtin() {
        let rs = K::parse(&text).unwrap_or_else(|e| panic!("{file}: {e}"));
        let w = K::write("", &rs).unwrap();
        assert_eq!(K::parse(&w).unwrap(), rs, "{file}: parse(write(r)) == r");
        assert_eq!(
            K::write("", &K::parse(&w).unwrap()).unwrap(),
            w,
            "{file}: canonical"
        );
        n += rs.len();
    }
    n
}

#[test]
fn every_built_in_record_writes_canonically() {
    let (p, d, s) = (
        round_trip::<Punches>(),
        round_trip::<Dies>(),
        round_trip::<Presses>(),
    );
    assert!(p > 50 && d > 50 && s >= 30, "{p} {d} {s}");
}

#[test]
fn forms_templates_checks_previews() {
    for k in ["punch", "die", "press"] {
        let f = fields(k);
        assert!(f.iter().any(|x| x.path == "id"));
        assert!(f.len() > 5);
    }
    for t in templates() {
        // a template is complete in its working geometry: it previews
        preview(&t).unwrap_or_else(|e| panic!("{}: {:?}", t.id, e));
        let fs = check(&t);
        assert!(fs.iter().all(|f| f.not_checked), "{}: {fs:?}", t.id);
    }
    // a real record with unknown geometry: findings name the fields
    let amada = Dies::parse(&Dies::builtin()[0].1)
        .unwrap()
        .into_iter()
        .find(|r| r.id.starts_with("amada/"))
        .unwrap();
    let fs = check(&amada);
    assert!(fs.iter().any(|f| f.path == "height" && !f.not_checked), "{fs:?}");
    assert!(preview(&amada).is_err());
    // a press: unknown values are findings that do not fail
    let press = Presses::parse(&Presses::builtin()[0].1).unwrap().remove(0);
    let fs = check(&press);
    assert!(fs.iter().all(|f| f.not_checked), "{fs:?}");
    assert!(preview(&press).unwrap().iter().any(|(n, _, _)| n == "ram"));
}

#[test]
fn one_folder_for_three_kinds() {
    // a file of dies reads as no punches (not as a broken file), and the other way about
    let dies = &Dies::builtin()[0].1;
    assert!(Punches::parse(dies).unwrap().is_empty());
    assert!(!Dies::parse(dies).unwrap().is_empty());
    assert_eq!(Punches::ROOT, "bend");
    assert_eq!(Punches::FOLDER, "tooling");
}
