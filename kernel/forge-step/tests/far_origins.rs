//! A line's origin, and a cylinder's axis origin, may lie anywhere along it: CATIA writes some a
//! few kilometres out (NIST's stc_07), Onshape some hundreds of metres (ABC). Such a file is the
//! same part as one whose origins sit by it, and reads as one, not refused as outside the model
//! box. Both files were made from nothing by research/formats/repro/make_brep.py (OCCT's export
//! of a box and a cylinder, one origin then moved 2 km along its line).
use forge_step::{Limits, import};

fn read(name: &str) -> (usize, f64, f64) {
    let path = format!("{}/tests/data/{name}", env!("CARGO_MANIFEST_DIR"));
    let got = import(&std::fs::read(path).unwrap(), &Limits::default()).expect("reads");
    assert!(got.refusals.is_empty(), "{:?}", got.refusals);
    let (mut v, mut a) = (0.0, 0.0);
    for s in &got.solids {
        assert!(
            forge_topo::check(&s.body).is_empty(),
            "{:?}",
            forge_topo::check(&s.body)
        );
        let m = forge_topo::mass::mass_rel(&s.body, 1e-9).expect("measures");
        v += m.volume.abs();
        a += m.area;
    }
    (got.solids.len(), v, a)
}

#[test]
fn a_line_whose_origin_lies_2_km_along_it_reads() {
    let (n, v, a) = read("far-line-origin.step");
    assert_eq!(n, 1);
    assert!((v - 1000.0).abs() < 1e-9 && (a - 600.0).abs() < 1e-9, "{v} {a}");
}

#[test]
fn a_cylinder_whose_axis_origin_lies_2_km_along_it_reads() {
    let (n, v, a) = read("far-cylinder-axis.step");
    assert_eq!(n, 1);
    let pi = std::f64::consts::PI;
    assert!((v - pi * 25.0 * 20.0).abs() < 1e-9 * v, "{v}");
    assert!(
        (a - (2.0 * pi * 25.0 + 2.0 * pi * 5.0 * 20.0)).abs() < 1e-9 * a,
        "{a}"
    );
}

#[test]
fn a_point_outside_the_box_is_still_refused_where_it_is_a_point() {
    // a vertex 2 km out: refused, as before
    let path = format!("{}/tests/data/far-line-origin.step", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(path).unwrap();
    let v = text.find("VERTEX_POINT('',#").unwrap();
    let at: String = text[v + 17..]
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    let line = text
        .lines()
        .find(|l| l.starts_with(&format!("#{at} = CARTESIAN_POINT")))
        .unwrap();
    let far = format!("#{at} = CARTESIAN_POINT('',(2.E+06,0.,0.));");
    let got = import(text.replace(line, &far).as_bytes(), &Limits::default());
    let refused = match got {
        Err(_) => true,
        Ok(g) => g.solids.is_empty() || !g.refusals.is_empty(),
    };
    assert!(refused);
}
