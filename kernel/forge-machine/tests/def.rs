use forge_machine::def::{DefError, Kind, MachineDef};
use forge_machine::library;

#[test]
fn builtin_machines_parse_and_have_their_kinds() {
    let ms: Vec<MachineDef> = library::builtin_files()
        .into_iter()
        .filter(|(_, f)| f == "builtin/generic.toml")
        .map(|(m, _)| m)
        .collect();
    let kinds: Vec<(&str, Kind)> = ms.iter().map(|m| (m.name.as_str(), m.kind)).collect();
    assert_eq!(
        kinds,
        vec![
            ("generic-vmc3", Kind::Vmc3),
            ("generic-trunnion-bc", Kind::TableTable),
            ("generic-lathe", Kind::Lathe),
            ("generic-mill-turn", Kind::MillTurn),
        ]
    );
    let t = ms[0].to_tooling().unwrap();
    assert_eq!(t.rpm, [100.0, 12000.0]);
    assert_eq!(t.joints.len(), 3);
}

const MINI: &str = r#"
[[machine]]
name = "m"
kind = "vmc3"
control = "fanuc"
length_joint = "Z"
clearance = 1
source = "test"
[[machine.joint]]
name = "X"
parent = "base"
side = "table"
type = "linear"
axis = [1, 0, 0]
range = [-1, 1]
rapid = 1000
max_feed = 500
[[machine.joint]]
name = "Y"
parent = "X"
side = "table"
type = "linear"
axis = [0, 1, 0]
range = [-1, 1]
rapid = 1000
max_feed = 500
[[machine.joint]]
name = "Z"
parent = "base"
side = "head"
type = "linear"
axis = [0, 0, 1]
range = [-1, 1]
rapid = 1000
max_feed = 500
[[machine.spindle]]
name = "S"
link = "Z"
kind = "milling"
axis = [0, 0, 1]
nose = [0, 0, 10]
rpm = [1, 100]
max = [10, 10]
[[machine.mount]]
name = "t"
kind = "tool"
link = "Z"
at = [0, 0, 10]
dir = [0, 0, -1]
[[machine.mount]]
name = "p"
kind = "part"
link = "Y"
at = [0, 0, 0]
dir = [0, 0, 1]
xdir = [1, 0, 0]
"#;

#[test]
fn minimal_reads() {
    let m = MachineDef::parse_all(MINI).unwrap();
    assert_eq!(m[0].chain("Y").unwrap(), vec![0, 1]);
}

fn refused(src: &str) -> DefError {
    MachineDef::parse_all(src).unwrap_err()
}

#[test]
fn structural_errors_are_typed() {
    // a cycle
    let e = refused(&MINI.replace(
        "parent = \"base\"\nside = \"table\"",
        "parent = \"Y\"\nside = \"table\"",
    ));
    assert!(matches!(e, DefError::Shape { .. }), "{e:?}");
    // a head joint under the part mount
    let e = refused(&MINI.replacen("side = \"table\"", "side = \"head\"", 1));
    assert!(
        matches!(e, DefError::Shape { ref why, .. } if why.contains("lies on mount")),
        "{e:?}"
    );
    // missing max_feed
    let e = refused(&MINI.replacen("max_feed = 500\n", "", 1));
    assert!(
        matches!(
            e,
            DefError::Missing {
                field: "max_feed",
                ..
            }
        ),
        "{e:?}"
    );
    // a rotary letter on a linear joint
    let e = refused(&MINI.replacen("name = \"X\"", "name = \"X\"\nletter = \"A\"", 1));
    assert!(matches!(e, DefError::Bad { field: "letter", .. }), "{e:?}");
    // the kind
    let e = refused(&MINI.replace("kind = \"vmc3\"", "kind = \"table-table\""));
    assert!(
        matches!(e, DefError::Shape { ref why, .. } if why.contains("does not fit")),
        "{e:?}"
    );
    // unknown kind names the choices
    let e = refused(&MINI.replace("kind = \"vmc3\"", "kind = \"robot\""));
    assert!(
        matches!(e, DefError::Unknown { field: "kind", ref known, .. } if known.len() == 9),
        "{e:?}"
    );
    // a rotary without a pivot
    let e = refused(&MINI.replace(
        "type = \"linear\"\naxis = [0, 1, 0]",
        "type = \"rotary\"\nletter = \"B\"\naxis = [0, 1, 0]",
    ));
    assert!(matches!(e, DefError::Missing { field: "pivot", .. }), "{e:?}");
}

#[test]
fn project_level_hides_builtin() {
    let mut lib = library::Library::with_builtin();
    let src = MINI.replace("name = \"m\"", "name = \"generic-vmc3\"");
    lib.add_level(&[("cam/machines/mine.toml".into(), src)]).unwrap();
    let m = lib.get("generic-vmc3").unwrap();
    assert_eq!(m.source, "test");
    assert!(lib.get("generic-lathe").is_ok());
    assert!(matches!(
        lib.get("nope"),
        Err(library::LookupError::Unknown { .. })
    ));
}

/// One file per vendor (data/machines/<vendor>.toml), each body's part under
/// parts/<vendor>/<machine>/, names unique across the files.
#[test]
fn builtin_layout_is_per_vendor() {
    let ms = library::builtin_files();
    assert_eq!(ms[0].1, "builtin/generic.toml");
    for (m, file) in &ms {
        let vendor = file
            .strip_prefix("builtin/")
            .and_then(|f| f.strip_suffix(".toml"))
            .unwrap();
        for b in &m.bodies {
            let p = b.part.as_deref().unwrap_or("");
            assert!(
                p.starts_with(&format!("parts/{vendor}/{}/", m.name)),
                "{file} {} body {}: part {p:?} not under parts/{vendor}/{}/",
                m.name,
                b.name,
                m.name
            );
        }
    }
}

/// Provenance: a real record cites every value; cites name real values; each status carries what
/// it needs.
#[test]
fn cites_are_checked() {
    // not real: no cites needed
    assert!(MachineDef::parse_all(MINI).is_ok());
    let real = MINI.replace("source = \"test\"", "source = \"test\"\nreal = true");
    match MachineDef::parse_all(&real) {
        Err(DefError::Bad {
            field: "real", why, ..
        }) => {
            assert!(
                why.contains("joint.X.range") && why.contains("mount.p.at"),
                "{why}"
            )
        }
        r => panic!("{r:?}"),
    }
    let m = MachineDef::parse_all(MINI).unwrap().remove(0);
    let mut all = real.clone();
    for p in m.cite_paths() {
        all.push_str(&format!(
            "[[machine.cite]]\npath = \"{p}\"\nstatus = \"published\"\nsource = \"https://example.com/spec\"\ntaken = \"2026-11-23\"\n"
        ));
    }
    let ok = MachineDef::parse_all(&all).unwrap().remove(0);
    assert!(ok.real && ok.cites.len() == m.cite_paths().len());
    let bad = |extra: &str| MachineDef::parse_all(&format!("{all}[[machine.cite]]\n{extra}")).unwrap_err();
    assert!(matches!(
        bad("path = \"joint.Q.range\"\nstatus = \"published\"\nsource = \"s\"\ntaken = \"2026-11-23\"\n"),
        DefError::Bad { field: "path", .. }
    ));
    assert!(matches!(
        bad("path = \"joint.X.range\"\nstatus = \"published\"\n"),
        DefError::Missing { field: "source", .. }
    ));
    assert!(matches!(
        bad("path = \"joint.X.range\"\nstatus = \"published\"\nsource = \"s\"\n"),
        DefError::Missing { field: "taken", .. }
    ));
    assert!(matches!(
        bad("path = \"joint.X.accel\"\nstatus = \"estimate\"\n"),
        DefError::Missing { field: "basis", .. }
    ));
    assert!(matches!(
        bad("path = \"joint.X.range\"\nstatus = \"conflict\"\nsource = \"s\"\ntaken = \"2026-11-23\"\n"),
        DefError::Missing { field: "note", .. }
    ));
    assert!(matches!(
        bad("path = \"joint.X.range\"\nstatus = \"maybe\"\n"),
        DefError::Unknown { field: "status", .. }
    ));
    // an estimate with its basis is fine
    assert!(MachineDef::parse_all(&format!(
        "{all}[[machine.cite]]\npath = \"joint.X.home\"\nstatus = \"estimate\"\nbasis = \"centre of travel\"\n"
    ))
    .is_ok());
}
