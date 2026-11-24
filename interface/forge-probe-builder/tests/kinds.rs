use forge_library::{Catalog, Kind};
use forge_probe_builder::{KINDS, ToolSetters, WorkProbes, check, fields, preview, templates};

#[test]
fn builtin_kinds_load_and_write_canonically() {
    let w = Catalog::<WorkProbes>::from_sources(&[], &[]);
    let t = Catalog::<ToolSetters>::from_sources(&[], &[]);
    assert!(w.errors.is_empty() && t.errors.is_empty());
    assert!(w.get("renishaw-omp40-2").is_some() && w.get("renishaw-ts27r").is_none());
    assert!(t.get("renishaw-ts27r").is_some());
    for (_, text) in WorkProbes::builtin() {
        let rs = WorkProbes::parse(&text).unwrap();
        assert_eq!(WorkProbes::write("", &rs).unwrap(), text);
    }
    assert_eq!(KINDS.len(), 2);
}

#[test]
fn a_user_record_hides_the_builtin_and_a_wrong_kind_is_refused() {
    let tpl = templates();
    let mut mine = tpl[0].clone();
    mine.id = "renishaw-omp40-2".into();
    let text = WorkProbes::write("mine", std::slice::from_ref(&mine)).unwrap();
    let w = Catalog::<WorkProbes>::from_sources(&[], &[("mine.toml".into(), text)]);
    let e = w.get("renishaw-omp40-2").unwrap();
    assert_eq!(e.def, mine);
    assert!(!e.hides.is_empty());
    assert!(ToolSetters::write("", std::slice::from_ref(&mine)).is_err());
}

#[test]
fn forms_checks_and_previews() {
    for (k, _) in KINDS {
        let f = fields(k);
        assert!(f.iter().any(|x| x.path == "body"));
        assert!(f.iter().any(|x| x.path == "repeat" && x.cited));
    }
    assert!(fields("work_probe").iter().any(|x| x.path == "overtravel"));
    assert!(fields("tool_setter").iter().any(|x| x.path == "trigger"));
    let b = forge_probe::builtin();
    let omp = forge_probe::find(&b, "renishaw-omp40-2").unwrap();
    let c = check(omp);
    assert!(c.iter().all(|f| f.not_checked));
    assert!(c.iter().any(|f| f.path == "max_feed"));
    assert!(preview(omp).unwrap().len() >= 4);
    let ts = forge_probe::find(&b, "renishaw-ts27r").unwrap();
    assert!(preview(ts).unwrap_err().not_checked);
    for t in templates() {
        assert!(check(&t).is_empty(), "{}", t.id);
        assert!(preview(&t).is_ok());
    }
}
