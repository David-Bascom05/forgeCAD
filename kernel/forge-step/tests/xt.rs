//! Parasolid transmit files: the fixtures written from the XT Format Reference by
//! research/parasolid/fixtures.py, read in each form, and what a broken file is refused with.
use forge_math::Box3;
use forge_step::{ImportError, Limits, import, xt};
use forge_topo::mass;

fn data(name: &str) -> Vec<u8> {
    std::fs::read(format!("{}/tests/data/xt/{name}", env!("CARGO_MANIFEST_DIR"))).expect("fixture")
}

fn volume(b: &forge_topo::Body) -> f64 {
    mass::mass_rel(b, 1e-9).expect("mass").volume
}

fn near(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol * b.abs().max(1.0)
}

/// The box of the vertices: a face box is padded by the kernel's tolerance.
fn boxed(b: &forge_topo::Body) -> Box3 {
    Box3::from_points(b.vertices.iter().map(|v| v.point))
}

#[test]
fn a_block_reads_the_same_from_every_form() {
    for f in ["block.x_t", "block_embedded.x_t", "block.x_b", "block_little.x_b"] {
        let d = data(f);
        assert!(xt::is_xt(&d), "{f}");
        let imp = import(&d, &Limits::default()).unwrap_or_else(|e| panic!("{f}: {e}"));
        assert!(imp.refusals.is_empty(), "{f}: {:?}", imp.refusals);
        assert_eq!(imp.solids.len(), 1, "{f}");
        let b = &imp.solids[0].body;
        assert!(forge_topo::check(b).is_empty(), "{f}: {:?}", forge_topo::check(b));
        assert_eq!(b.face_ids().count(), 6, "{f}");
        // 30 x 20 x 10 mm from metres
        assert!(near(volume(b), 6000.0, 1e-9), "{f}: {}", volume(b));
        let bx = boxed(b);
        assert!(
            near(bx.x.hi, 30.0, 1e-9) && near(bx.y.hi, 20.0, 1e-9) && near(bx.z.hi, 10.0, 1e-9),
            "{f}: {bx:?}"
        );
        assert_eq!(imp.stats.units_mm, 1000.0);
    }
}

#[test]
fn a_cylinders_rims_with_no_vertex_close_up() {
    for f in ["cylinder.x_t", "cylinder.x_b"] {
        let imp = import(&data(f), &Limits::default()).unwrap();
        assert_eq!(imp.solids.len(), 1, "{f}: {:?}", imp.refusals);
        let b = &imp.solids[0].body;
        assert!(forge_topo::check(b).is_empty(), "{f}: {:?}", forge_topo::check(b));
        let want = std::f64::consts::PI * 10.0 * 10.0 * 25.0;
        assert!(near(volume(b), want, 1e-6), "{f}: {} against {want}", volume(b));
    }
}

#[test]
fn an_assembly_places_each_instance_by_its_transform() {
    let d = data("assembly.x_t");
    let imp = import(&d, &Limits::default()).unwrap();
    assert_eq!(imp.solids.len(), 2, "{:?}", imp.refusals);
    let (a, b) = (boxed(&imp.solids[0].body), boxed(&imp.solids[1].body));
    // the first where it stands, 20 x 10 x 5
    assert!(
        near(a.x.lo, 0.0, 1e-9) && near(a.x.hi, 20.0, 1e-9) && near(a.y.hi, 10.0, 1e-9),
        "{a:?}"
    );
    // the second turned a quarter about z (x to y) and moved 100 along x: x' = 100 - y, y' = x
    assert!(near(b.x.lo, 90.0, 1e-9) && near(b.x.hi, 100.0, 1e-9), "{b:?}");
    assert!(near(b.y.lo, 0.0, 1e-9) && near(b.y.hi, 20.0, 1e-9), "{b:?}");
    for s in &imp.solids {
        assert!(near(volume(&s.body), 1000.0, 1e-9));
        assert!(!s.body.unplaced);
    }
    // unplaced, both where the part stands
    let un = xt::import_unplaced(&d, &Limits::default()).unwrap();
    assert_eq!(un.solids.len(), 2);
    for s in &un.solids {
        let bx = boxed(&s.body);
        assert!(near(bx.x.lo, 0.0, 1e-9) && near(bx.x.hi, 20.0, 1e-9), "{bx:?}");
    }
}

#[test]
fn the_header_and_census_say_what_wrote_the_file() {
    let (h, census) = xt::census(&data("block_embedded.x_t"), &Limits::default()).unwrap();
    assert_eq!(h.key("APPL"), Some("forgecad-fixture"));
    assert_eq!(h.schema, "SCH_1300000_13006_13006");
    assert!(!h.binary);
    assert!(census.contains(&(14, 6)), "{census:?}");
    assert!(census.contains(&(16, 12)), "{census:?}");
    let (h, _) = xt::census(&data("block.x_b"), &Limits::default()).unwrap();
    assert!(h.binary);
}

#[test]
fn a_file_cut_short_says_where() {
    let d = data("block.x_t");
    let cut = &d[..d.len() * 2 / 3];
    match import(cut, &Limits::default()) {
        Err(ImportError::Xt(xt::XtError::Bytes { want, node, .. })) => {
            assert!(!want.is_empty());
            assert!(node.is_some(), "the node being read is named");
        }
        Err(e) => panic!("{e}"),
        Ok(_) => panic!("read a file cut short"),
    }
    let b = data("block.x_b");
    assert!(matches!(
        import(&b[..b.len() / 2], &Limits::default()),
        Err(ImportError::Xt(xt::XtError::Bytes { .. }))
    ));
}

#[test]
fn a_node_type_nothing_describes_is_refused() {
    // a type the base schema lacks, in a file that carries no schema of its own
    let d = String::from_utf8(data("block.x_t")).unwrap();
    // the root, first after the schema name and the user field size (`SCH_1300000_13006` `0 `
    // `12 1 `); records joined
    let (head, rest) = d.split_at(d.find("**END_OF_HEADER").unwrap());
    let (marker, body) = rest.split_at(rest.find('\n').unwrap() + 1);
    let body: String = body.lines().collect();
    let bad = format!(
        "{head}{marker}{}",
        body.replacen("_130060 12 1 ", "_130060 77 1 ", 1)
    );
    assert_ne!(bad.len(), 0);
    match import(bad.as_bytes(), &Limits::default()) {
        Err(ImportError::Xt(xt::XtError::UnknownType { ty: 77, .. })) => {}
        other => panic!("{:?}", other.map(|i| i.solids.len())),
    }
}

#[test]
fn a_sheet_body_is_refused_with_its_kind() {
    // the block's body type made sheet (3): field 15 of the BODY node
    let d = String::from_utf8(data("block.x_t")).unwrap();
    let flat: String = d
        .split("**END_OF_HEADER")
        .nth(1)
        .unwrap()
        .lines()
        .skip(1)
        .collect();
    let k = flat.find(" 1000 1e-08 ").expect("the body's resolutions");
    // ... res_size res_linear ref_instance next previous state owner body_type
    let tail = &flat[k + " 1000 1e-08 ".len()..];
    let fields: Vec<&str> = tail.splitn(7, ' ').collect();
    assert_eq!(fields[5], "1", "body type solid");
    let mut changed = fields.clone();
    changed[5] = "3";
    let flat2 = format!("{}{}{}", &flat[..k + " 1000 1e-08 ".len()], changed.join(" "), "");
    let head = d.split("**END_OF_HEADER").next().unwrap();
    let file = format!("{head}**END_OF_HEADER****\n{flat2}\n");
    let imp = import(file.as_bytes(), &Limits::default()).unwrap();
    assert!(imp.solids.is_empty());
    assert_eq!(imp.refusals.len(), 1);
    assert!(imp.refusals[0].why.contains("sheet"), "{:?}", imp.refusals[0]);
}

#[test]
fn nothing_else_is_taken_for_a_transmit_file() {
    assert!(!xt::is_xt(b"ISO-10303-21;\nHEADER;"));
    assert!(!xt::is_xt(b""));
    assert!(!xt::is_xt(b"T"));
    assert!(!xt::is_xt(b"PS\0\0"));
}
