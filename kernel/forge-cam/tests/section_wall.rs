//! A side wall that is one L-shaped planar face (a step flush with the outline: the block on
//! part of a base shares the base's side planes) has a section that changes with height. It was
//! refused as NotPrism (TOOLPATH-QUALITY's open_step benchmark); its section at each height is
//! the trim cut there, and a height at the step itself stays refused.
use forge_cam::fixture::area;
use forge_cam::section::{SectionError, section};
use forge_cam_geom::check::{Face, Surf, TSeg, Trim};
use forge_cam_geom::ival::Ival;

const MU: f64 = 1e-9;

fn iv(lo: f64, hi: f64) -> Ival {
    Ival { lo, hi }
}

/// A vertical wall o + p u + q z over p in [0, w], q in [0, h], with an optional (p, q) trim.
fn wall(o: [f64; 3], u: [f64; 3], w: f64, h: f64, trim: Option<Vec<[f64; 2]>>, id: u32) -> Face {
    let trim = trim.map(|pts| Trim {
        segs: (0..pts.len())
            .map(|i| TSeg::Line {
                a: pts[i],
                b: pts[(i + 1) % pts.len()],
            })
            .collect(),
    });
    Face::new(
        Surf::Plane {
            o,
            u,
            v: [0.0, 0.0, 1.0],
        },
        iv(0.0, w),
        iv(0.0, h),
        trim,
        MU,
        id,
    )
}

/// The base 80 × 50 × 10 and the block x ∈ [30, 80] to z = 20: its four outer walls (the two
/// long ones L-shaped) and the step's riser at x = 30.
fn walls() -> Vec<Face> {
    // in (p, q): the base's full length to q = 10, the block's part of it to q = 20
    let l_front = vec![
        [0.0, 0.0],
        [80.0, 0.0],
        [80.0, 20.0],
        [30.0, 20.0],
        [30.0, 10.0],
        [0.0, 10.0],
    ];
    // the back wall runs the other way (p from x = 80 to x = 0)
    let l_back = vec![
        [0.0, 0.0],
        [80.0, 0.0],
        [80.0, 10.0],
        [50.0, 10.0],
        [50.0, 20.0],
        [0.0, 20.0],
    ];
    vec![
        wall([0.0, 0.0, 0.0], [1.0, 0.0, 0.0], 80.0, 20.0, Some(l_front), 1),
        wall([80.0, 0.0, 0.0], [0.0, 1.0, 0.0], 50.0, 20.0, None, 2),
        wall([80.0, 50.0, 0.0], [-1.0, 0.0, 0.0], 80.0, 20.0, Some(l_back), 3),
        wall([0.0, 50.0, 0.0], [0.0, -1.0, 0.0], 50.0, 10.0, None, 4),
        wall([30.0, 50.0, 10.0], [0.0, -1.0, 0.0], 50.0, 10.0, None, 5),
    ]
}

#[test]
fn an_l_shaped_side_wall_has_its_section_at_each_height() {
    let f = walls();
    // above the step: the block alone
    let r = section(&f, 15.0, MU).unwrap();
    assert_eq!(r.loops.len(), 1);
    assert!((area(&r).abs() - 50.0 * 50.0).abs() < 1e-6, "{r:?}");
    // below it: the whole base
    let r = section(&f, 5.0, MU).unwrap();
    assert_eq!(r.loops.len(), 1);
    assert!((area(&r).abs() - 80.0 * 50.0).abs() < 1e-6, "{r:?}");
    // at the step's height the section changes: refused, not guessed
    assert_eq!(section(&f, 10.0, MU), Err(SectionError::NotPrism { face: 1 }));
}
