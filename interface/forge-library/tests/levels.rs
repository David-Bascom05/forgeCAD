//! The library's levels, shadowing and saving, on a toy kind: a file is lines `id = value`.
use forge_library::{Catalog, Kind, Level, save};

#[derive(Clone, Debug, PartialEq)]
struct Rec {
    id: String,
    v: String,
}

struct Toy;

impl Kind for Toy {
    type Record = Rec;
    const WORD: &'static str = "toy";
    const FOLDER: &'static str = "toys";
    fn builtin() -> Vec<(String, String)> {
        vec![("base.toml".into(), "a = 1\nb = 2\n".into())]
    }
    fn parse(text: &str) -> Result<Vec<Rec>, String> {
        text.lines()
            .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
            .map(|l| {
                let (i, v) = l.split_once(" = ").ok_or(format!("bad line `{l}`"))?;
                Ok(Rec {
                    id: i.into(),
                    v: v.into(),
                })
            })
            .collect()
    }
    fn write(header: &str, rs: &[Rec]) -> Result<String, String> {
        let mut s = format!("# {header}\n");
        for r in rs {
            s.push_str(&format!("{} = {}\n", r.id, r.v));
        }
        Ok(s)
    }
    fn id(r: &Rec) -> &str {
        &r.id
    }
}

fn tmp(t: &str) -> std::path::PathBuf {
    let d = forge_dirs::temp_dir().join(format!("forge-library-{t}-{}", forge_dirs::process_id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn levels_hide_in_order_and_a_broken_file_hides_nothing() {
    let cat = Catalog::<Toy>::from_sources(
        &[("p.toml".into(), "b = project\n".into())],
        &[
            ("u.toml".into(), "a = user\nb = user\n".into()),
            ("bad.toml".into(), "nonsense\n".into()),
        ],
    );
    let ids: Vec<_> = cat.entries.iter().map(|e| (e.def.id.as_str(), e.level)).collect();
    assert_eq!(ids, [("b", Level::Project), ("a", Level::User)]);
    assert_eq!(cat.get("b").unwrap().hides.len(), 2);
    assert_eq!(
        cat.get("a").unwrap().hides,
        [(Level::Builtin, "builtin/base.toml".to_string())]
    );
    assert_eq!(cat.errors.len(), 1);
    assert_eq!(cat.errors[0].0, Level::User);
}

#[test]
fn save_reads_back_refuses_what_it_should_and_moves_the_generation() {
    let d = tmp("save");
    let g0 = forge_library::generation();
    let r = Rec {
        id: "c".into(),
        v: "3".into(),
    };
    let f = save::<Toy>(&r, &[("parts/c/x.fg".into(), "x".into())], &d, Level::User, false).unwrap();
    assert!(f.ends_with("c.toml") && d.join("parts/c/x.fg").is_file());
    assert!(forge_library::generation() > g0);
    // again: the file exists
    assert!(save::<Toy>(&r, &[], &d, Level::User, false).is_err());
    assert!(save::<Toy>(&r, &[], &d, Level::User, true).is_ok());
    // an id in another file of the level
    std::fs::write(d.join("other.toml"), "e = 9\n").unwrap();
    let e = Rec {
        id: "e".into(),
        v: "1".into(),
    };
    assert!(
        save::<Toy>(&e, &[], &d, Level::User, false)
            .unwrap_err()
            .contains("already in")
    );
    // the built-in level and a bad id
    assert!(save::<Toy>(&r, &[], &d, Level::Builtin, true).is_err());
    let bad = Rec {
        id: "../x".into(),
        v: "1".into(),
    };
    assert!(save::<Toy>(&bad, &[], &d, Level::User, true).is_err());
    // a value the writer cannot take back is refused before anything is written
    let nl = Rec {
        id: "n".into(),
        v: "1\nz = 2".into(),
    };
    assert!(
        save::<Toy>(&nl, &[], &d, Level::User, true)
            .unwrap_err()
            .contains("different record")
    );
    assert!(!d.join("n.toml").exists());
    let _ = std::fs::remove_dir_all(&d);
}
