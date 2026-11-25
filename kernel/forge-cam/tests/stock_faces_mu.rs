//! A stock or fixture box's faces carry mu 0 only when they are the box exactly: each face runs
//! from a corner out to corner + (max − min), which is max only when that difference is exact.
//! Then a tool exactly at a fixture's clearance can be proven (forge-cam-geom's axisplane.rs);
//! otherwise the faces keep the ulp's tolerance, and a tie stays unproven.
use forge_cam::ops::Stock;

#[test]
fn a_box_is_exact_only_when_its_extents_are() {
    let exact = Stock {
        min: [-2.0, -2.0, 0.0],
        max: [42.0, 32.0, 14.0],
    };
    assert!(exact.faces().iter().all(|f| f.mu == 0.0));
    // 1 − 1e-20 rounds to 1: the face would end at 1 + 1e-20, not at 1
    let inexact = Stock {
        min: [1e-20, 0.0, 0.0],
        max: [1.0, 1.0, 1.0],
    };
    assert_eq!(1.0 - 1e-20, 1.0);
    assert!(inexact.faces().iter().all(|f| f.mu == 1e-9));
}
