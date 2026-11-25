//! Sections of solids taken out of the Haas machine models (tools/kernel-stress/extract.py;
//! research/kernel-stress/corpus.tsv).
use forge_agent::features::section;
use forge_math::{Frame, Point3, UnitVec3};

/// VF-5/40 #532896 (K112), 9 faces, cut across x through its middle: one point where the plane
/// meets an edge was refined two ways, onto the edge and onto the face, 7.6e-6 apart; kept as two
/// nodes, the piece between them ran backwards and the one loop fell into five open ones. A
/// piece shorter than the model's tolerance is now a point.
#[test]
fn a_section_through_a_point_refined_two_ways_closes() {
    let p = format!(
        "{}/tests/data/vf5_40_section_9faces.step",
        env!("CARGO_MANIFEST_DIR")
    );
    let imp = forge_step::import(&std::fs::read(&p).unwrap(), &forge_step::Limits::default()).unwrap();
    let b = &imp.solids[0].body;
    let bx = b.faces_box();
    let c = bx.center();
    let area = |x: f64| {
        let l = section(b, &Frame::from_z(Point3::new(x, c.y, c.z), UnitVec3::X)).unwrap();
        assert_eq!(l.len(), 1, "x {x}: {} loops", l.len());
        assert!(l[0].closed, "x {x}");
        l[0].area
    };
    let (mid, off) = (area(c.x), area(c.x + 1.37e-4 * bx.diagonal()));
    assert!((mid - off).abs() < 1e-5 * off, "{mid} vs {off}");
    assert!((mid - 115.1235).abs() < 1e-3, "{mid}");
}
