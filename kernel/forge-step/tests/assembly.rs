//! An assembly's placements are read: each part stands at every place the file puts it, and one
//! whose placement can't be followed is refused, never left where it stands. two-instances.step was
//! made from nothing by research/formats/repro/make_brep.py: OCCT's XCAF export of one 10 mm box
//! placed twice, at the origin and 30 mm along x (two NEXT_ASSEMBLY_USAGE_OCCURRENCEs, each placed
//! by a CONTEXT_DEPENDENT_SHAPE_REPRESENTATION's ITEM_DEFINED_TRANSFORMATION).
use forge_step::{Limits, import};

fn text() -> String {
    std::fs::read_to_string(format!(
        "{}/tests/data/two-instances.step",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}

fn boxes(t: &str) -> forge_step::Imported {
    import(t.as_bytes(), &Limits::default()).expect("reads")
}

#[test]
fn a_part_placed_twice_is_two_bodies_each_where_it_is_placed() {
    let got = boxes(&text());
    assert!(got.refusals.is_empty(), "{:?}", got.refusals);
    assert_eq!(got.stats.assembly, 0);
    assert_eq!(got.solids.len(), 2);
    let mut lows: Vec<[f64; 3]> = Vec::new();
    for s in &got.solids {
        assert!(!s.body.unplaced);
        assert!(
            forge_topo::check(&s.body).is_empty(),
            "{:?}",
            forge_topo::check(&s.body)
        );
        let m = forge_topo::mass::mass_rel(&s.body, 1e-9).unwrap();
        assert!((m.volume - 1000.0).abs() < 1e-9 && (m.area - 600.0).abs() < 1e-9);
        // its integrals as read move with it
        let c = s.mass.as_ref().unwrap().centroid;
        assert!(c.distance(m.centroid) < 1e-9, "{c:?} {:?}", m.centroid);
        let b = s.body.faces_box();
        lows.push([b.x.lo, b.y.lo, b.z.lo]);
    }
    lows.sort_by(|a, b| a[0].total_cmp(&b[0]));
    // (a face box is padded by 1e-5)
    let want = [[0.0, 0.0, 0.0], [30.0, 0.0, 0.0]];
    for (got, want) in lows.iter().zip(want) {
        assert!(
            got.iter().zip(want).all(|(g, w)| (g - w).abs() < 1e-4),
            "{lows:?}"
        );
    }
    // no two bodies share a name
    let names = |b: &forge_topo::Body| b.faces.iter().map(|f| f.name).collect::<Vec<_>>();
    let (a, b) = (names(&got.solids[0].body), names(&got.solids[1].body));
    assert!(a.iter().all(|n| !b.contains(n)));
}

#[test]
fn a_placement_that_cannot_be_followed_refuses_its_part() {
    // the second placement's transformation pointed at no placement
    let t = text();
    let line = t
        .lines()
        .filter(|l| l.contains("ITEM_DEFINED_TRANSFORMATION"))
        .nth(1)
        .unwrap();
    let broken = t.replace(
        line,
        &line.replace(
            "ITEM_DEFINED_TRANSFORMATION('',''",
            "ITEM_DEFINED_TRANSFORMATION('','',#1,#2);\n#99999=X(''",
        ),
    );
    let got = boxes(&broken);
    assert!(
        got.solids.is_empty(),
        "a part half placed is not read as one placement"
    );
    assert!(
        got.refusals.iter().any(|r| r.kind == "assembly"),
        "{:?}",
        got.refusals
    );
}

#[test]
fn an_occurrence_with_no_placement_refuses_its_part() {
    // the placements taken away: the occurrences say the part is used twice, but not where
    let t: String = text()
        .lines()
        .filter(|l| !l.contains("CONTEXT_DEPENDENT_SHAPE_REPRESENTATION"))
        .map(|l| format!("{l}\n"))
        .collect();
    let got = boxes(&t);
    assert!(got.solids.is_empty());
    assert!(
        got.refusals.iter().any(|r| r.kind == "assembly"),
        "{:?}",
        got.refusals
    );
}
