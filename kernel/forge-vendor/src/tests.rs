use super::*;

const IN: f64 = 25.4;

#[test]
fn every_builtin_reads_and_writes_back_the_same() {
    for (file, text) in BUILTIN {
        let v = Vendor::parse(text).unwrap_or_else(|e| panic!("{file}: {e}"));
        let again = Vendor::parse(&v.to_text()).unwrap_or_else(|e| panic!("{file} rewritten: {e}"));
        assert_eq!(again, v, "{file}");
        // each says where it is from, when, and to verify before ordering
        assert!(
            v.source.as_deref().is_some_and(|s| s.starts_with("https://")),
            "{file}"
        );
        assert_eq!(v.taken.as_deref(), Some("2026-11-22"), "{file}");
        assert!(v.note.as_deref().is_some_and(|n| n.contains(VERIFY)), "{file}");
        // every material named for density is one forge-material has
        for m in &v.materials {
            if let Some(l) = &m.library {
                assert!(forge_material::find(l).is_some(), "{file}: {} -> {l}", m.name);
            }
        }
    }
}

#[test]
fn sendcutsend_bends_by_its_calculator() {
    let v = Profiles::builtin().find("sendcutsend").unwrap().vendor.clone();
    let m = v.material("5052").expect("found by alias");
    assert_eq!(m.name, "5052-H32 aluminum");
    let shop = v.shop(m);
    let s = shop.stock(0.063 * IN).expect("0.063 in stocked");
    assert_eq!(s.rule, fs::BendRule::K(0.42));
    assert_eq!(s.radii, vec![0.035 * IN]);
    assert!((s.min_flange.at(s.thickness, 0.0) - 0.303 * IN).abs() < 1e-12);
    // half the die width, from the bend's centre
    assert_eq!(s.bend_ref, fs::BendRef::Centre);
    assert!((s.hole_to_bend.at(s.thickness, 0.0) - 0.236 * IN).abs() < 1e-12);
    // the vendor's 0.5 t
    assert!((s.min_hole.at(s.thickness, 0.0) - 0.5 * 0.063 * IN).abs() < 1e-12);
    assert!((s.relief.bend.depth.at(0.0, 0.0) - 0.118 * IN).abs() < 1e-12);
    assert!(shop.stock(0.07 * IN).is_none());
    // hems are not offered; 6061 is cut, not bent
    assert!(!shop.offers(fs::Operation::Hem));
    let alu6 = v.shop(v.material("6061-T6").unwrap());
    assert!(!alu6.offers_in(fs::Operation::Bend, 0.125 * IN));
    // nuts for this material only (the stainless ones are SP-)
    assert!(shop.hardware.iter().all(|h| !h.code.starts_with("SP-")));
    let ss = v.shop(v.material("304").unwrap());
    assert!(ss.hardware.iter().all(|h| h.code.starts_with("SP-")));
    assert_eq!(shop.process.bent_max, Some((30.0 * IN, 44.0 * IN)));
}

#[test]
fn a_deduction_becomes_the_k_that_gives_it() {
    let text = r#"
format = 1
id = "d"
units = "mm"
[[material]]
name = "m"
[[material.stock]]
t = 2
radius = [2, 4]
bend_deduction = [3.5, 4.0]
"#;
    let v = Vendor::parse(text).unwrap();
    let shop = v.shop(&v.materials[0]);
    let s = &shop.stock[0];
    // back through the kernel's allowance: BD = 2 (R + t) - BA at each radius
    for (r, bd) in [(2.0, 3.5), (4.0, 4.0)] {
        let ba = s.rule.allowance(2.0, r, std::f64::consts::FRAC_PI_2).unwrap();
        assert!((2.0 * (r + 2.0) - ba - bd).abs() < 1e-12, "r {r}");
    }
}

#[test]
fn lengths_read_as_written() {
    let l = Len::parse("2.5t + r + 0.02").unwrap();
    assert_eq!(
        l,
        Len::Lin {
            per_t: 2.5,
            per_r: 1.0,
            plus: 0.02
        }
    );
    assert_eq!(Len::parse(&l.text()).unwrap(), l);
    assert_eq!(Len::parse("2t - 0.5").unwrap().at(1.0, 0.0, 1.0), 1.5);
    assert_eq!(Len::parse("1e-3").unwrap().at(0.0, 0.0, 1.0), 1e-3);
    let m = Len::parse("min(2t, 0.125)").unwrap();
    assert_eq!(m.at(0.05 * IN, 0.0, IN), 0.1 * IN);
    assert_eq!(m.at(0.1 * IN, 0.0, IN), 0.125 * IN);
    assert_eq!(Len::parse(&m.text()).unwrap(), m);
    assert!(Len::parse("2 inches").is_err());
}

#[test]
fn refuses_what_it_does_not_know() {
    let base = "format = 1\nid = \"x\"\n";
    assert!(Vendor::parse(&format!("{base}min_hoel = 1\n")).is_err());
    assert!(Vendor::parse(&format!("{base}[rules]\nmin_hoel = \"1t\"\n")).is_err());
    assert!(Vendor::parse(&format!("{base}operations = [\"teleport\"]\n")).is_err());
    assert!(Vendor::parse("format = 2\nid = \"x\"\n").is_err());
    assert!(Vendor::parse("id = \"x\"\n").is_err());
    assert!(
        Vendor::parse(&format!(
            "{base}[[material]]\nname = \"m\"\n[[material.stock]]\nt = 1\nk = 1.5\n"
        ))
        .is_err()
    );
    let e = Vendor::parse("format = 1\nid = \"x\"\nkerf = \n").unwrap_err();
    assert_eq!(e.line, Some(3));
    // a partial vendor reads
    let v = Vendor::parse(base).unwrap();
    assert!(v.materials.is_empty() && v.operations.is_none());
}

#[test]
fn partial_vendors_check_only_what_they_state() {
    let p = Profiles::builtin();
    let osh = &p.find("oshcut").unwrap().vendor;
    let shop = osh.shop(osh.material("5052").unwrap());
    let s = shop.stock(0.063 * IN).unwrap();
    // no radii stated: none refused
    assert!(s.radii.is_empty());
    assert!((s.min_hole.at(s.thickness, 0.0) - 0.01575 * IN).abs() < 1e-12);
    assert_eq!(s.hole_to_bend, fs::Scaled::NONE);
    let xo = &p.find("Xometry").unwrap().vendor;
    let shop = xo.shop(xo.material("304").unwrap());
    let s = shop.stock(0.03 * IN).unwrap();
    // min(2t, 0.125) at t = 0.03 in: 2t
    assert!((s.hole_to_edge.at(s.thickness, 0.0) - 0.06 * IN).abs() < 1e-12);
}

#[test]
fn saves_and_imports() {
    let dir = std::env::temp_dir().join(format!("forge-vendor-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let mut v = Profiles::builtin().find("fabworks").unwrap().vendor.clone();
    v.id = "my-shop".into();
    v.name = "My shop".into();
    let f = dir.join("vendors").join("my-shop.toml");
    save(&f, &v).unwrap();
    let p = Profiles::load(Some(&dir));
    let e = p.find("my-shop").unwrap();
    assert_eq!(e.lib, Lib::Project);
    assert_eq!(e.vendor, v);
    assert_eq!(p.ids()[0], "my-shop");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn tooling_is_kept_as_written() {
    let text = r#"format = 1
id = "t"
[tooling]
base = "std-eu"
clearance = 2
[[tooling.punches]]
id = "P1"
kind = "gooseneck"
tip_radius = 1
relief = { depth = 30, z0 = 20, z1 = 60 }
segments = [{ length = 100, count = 2 }]
[tooling.press]
id = "brake"
backgauge = { x = [0, 600], fingers = [{ y = 0 }] }
"#;
    let v = Vendor::parse(text).unwrap();
    let Some(Tooling::Table(t)) = &v.tooling else {
        panic!("{:?}", v.tooling)
    };
    assert_eq!(t.get("base"), Some(&toml::Item::Str("std-eu".into())));
    assert_eq!(Vendor::parse(&v.to_text()).unwrap(), v);
    let set = Vendor::parse("format = 1\nid = \"t\"\ntooling = \"std-us\"\n").unwrap();
    assert_eq!(set.tooling, Some(Tooling::Set("std-us".into())));
    assert_eq!(Vendor::parse(&set.to_text()).unwrap(), set);
    assert!(Vendor::parse("format = 1\nid = \"t\"\ntooling = 3\n").is_err());
}
