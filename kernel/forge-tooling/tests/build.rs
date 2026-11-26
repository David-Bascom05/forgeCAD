//! `build::derive` and the checks (MACHINE-BUILDER's library builder).
use forge_tooling::build::{DeriveError, Record, Severity, check, check_fit, check_stickout, derive};
use forge_tooling::toml::Val;
use forge_tooling::{CiteStatus, Library};

#[test]
fn derive_keeps_the_sources_cites_and_marks_the_users() {
    let lib = Library::builtin();
    let r = derive(
        &lib,
        "tool",
        "harvey-792208",
        "my-792208-long",
        &[("oal".into(), Val::Num(50.8))],
    )
    .unwrap();
    let Record::Tool(t) = r else { panic!() };
    assert!(!t.real);
    assert_eq!((t.dc, t.oal), (3.175, 50.8));
    let st = |p: &str| t.cites.iter().find(|c| c.path == p).map(|c| c.status);
    assert_eq!(st("oal"), Some(CiteStatus::User));
    assert_eq!(st("dc"), Some(CiteStatus::Published)); // the maker's cite kept
    assert!(
        t.cites
            .iter()
            .any(|c| c.path == "dc" && c.source.contains("harveytool.com"))
    );
    // every kind derives from a real record
    for (kind, from) in [
        ("holder", "schunk-1388977"),
        ("collet", lib.collets[0].id.as_str()),
        ("collet_nut", lib.collet_nuts[0].id.as_str()),
        ("pull_stud", lib.pull_studs[0].id.as_str()),
        ("insert", lib.inserts[0].id.as_str()),
        ("turn_holder", lib.turn_holders[0].id.as_str()),
    ] {
        let r = derive(&lib, kind, from, &format!("mine-{kind}"), &[])
            .unwrap_or_else(|e| panic!("{kind} {from}: {e:?}"));
        assert_eq!(r.id(), format!("mine-{kind}"));
    }
    // a generic record (no cites): its values are cited unknown, "copied from"
    let Record::Tool(g) = derive(&lib, "tool", "flat-6", "my-flat", &[]).unwrap() else {
        panic!()
    };
    assert!(
        g.cites
            .iter()
            .all(|c| c.status == CiteStatus::Unknown && c.basis.contains("copied from tool `flat-6`"))
    );
    // refusals
    assert!(
        matches!(derive(&lib, "tool", "harvey-79220", "x", &[]), Err(DeriveError::Unknown { near, .. }) if near.contains(&"harvey-792208".to_string()))
    );
    assert!(matches!(
        derive(&lib, "tool", "flat-6", "flat-10", &[]),
        Err(DeriveError::Taken(_))
    ));
    assert!(matches!(
        derive(&lib, "wrench", "flat-6", "x", &[]),
        Err(DeriveError::Kind(_))
    ));
    assert!(matches!(
        derive(&lib, "tool", "flat-6", "x", &[("oal".into(), Val::Num(5.0))]),
        Err(DeriveError::Bad(_))
    )); // oal <= lcf
}

#[test]
fn checks_name_the_field_the_measure_and_the_limit() {
    let lib = Library::builtin();
    let Record::Tool(t) = derive(
        &lib,
        "tool",
        "flat-6",
        "bad-bull",
        &[("re".into(), Val::Num(4.0))],
    )
    .unwrap() else {
        panic!()
    };
    let f = check(&lib, &Record::Tool(t.clone()));
    assert_eq!((f[0].path.as_str(), f[0].measured, f[0].limit), ("re", 4.0, 3.0));
    assert_eq!(check_stickout(&t, 10.0)[0].limit, t.lcf);
    assert!(check_stickout(&t, t.lcf).is_empty());
    // an ER16 collet does not hold a 20 mm shank; a 6 mm one fits a 6 mm collet's range
    let er = lib
        .collets
        .iter()
        .find(|c| c.system == "ER16" && c.clamp.is_some())
        .expect("an ER16 collet with a range");
    let Record::Tool(fat) =
        derive(&lib, "tool", "flat-6", "fat", &[("dcon".into(), Val::Num(20.0))]).unwrap()
    else {
        panic!()
    };
    assert_eq!(check_fit(&fat, er)[0].path, "dcon");
    // a holder whose nose is narrower than its system's nut
    let Record::Holder(h) = derive(
        &lib,
        "holder",
        "schunk-1388977",
        "thin",
        &[(
            "profile".into(),
            Val::Arr(vec![
                Val::Arr(vec![Val::Num(0.0), Val::Num(10.0)]),
                Val::Arr(vec![Val::Num(63.5), Val::Num(10.0)]),
            ]),
        )],
    )
    .unwrap() else {
        panic!()
    };
    let f = check(&lib, &Record::Holder(h.clone()));
    // no `nut`: compared with the system's largest nut, as a warning
    assert!(
        f.iter().any(|f| f.severity == Severity::Warning
            && f.path == "profile"
            && f.measured == 10.0
            && f.limit >= 22.0),
        "{f:?}"
    );
    // with its own nut named: an error against that nut
    let nut = lib
        .collet_nuts
        .iter()
        .find(|n| n.system == "ER16")
        .expect("an ER16 nut");
    let mut own = h.clone();
    own.nut = Some(nut.id.clone());
    let f = check(&lib, &Record::Holder(own.clone()));
    assert!(
        f.iter()
            .any(|f| f.severity == Severity::Error && f.limit == nut.od),
        "{f:?}"
    );
    own.nut = Some("no-such-nut".into());
    assert!(check(&lib, &Record::Holder(own)).iter().any(|f| f.path == "nut"));
    // the library's real holders: at most warnings (none names its nut yet)
    for h in &lib.holders {
        assert!(
            check(&lib, &Record::Holder(h.clone()))
                .iter()
                .all(|f| f.severity == Severity::Warning),
            "{}",
            h.id
        );
    }
    // every real record of the library passes its own checks
    for t in &lib.tools {
        assert!(check(&lib, &Record::Tool(t.clone())).is_empty(), "{}", t.id);
    }
    for c in &lib.collets {
        assert!(check(&lib, &Record::Collet(c.clone())).is_empty(), "{}", c.id);
    }
}
