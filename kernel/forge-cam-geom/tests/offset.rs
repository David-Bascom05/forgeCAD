use forge_cam_geom::offset::{OLoop, offset};
use forge_cam_geom::region::{Loop, P2, Region, Seg, p2};

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Loop {
    let (a, b, c, d) = (p2(x0, y0), p2(x1, y0), p2(x1, y1), p2(x0, y1));
    Loop { segs: vec![Seg::Line { a, b }, Seg::Line { a: b, b: c }, Seg::Line { a: c, b: d }, Seg::Line { a: d, b: a }] }
}
fn circle(cx: f64, cy: f64, r: f64, ccw: bool) -> Loop {
    let a = p2(cx + r, cy);
    Loop { segs: vec![Seg::Arc { a, b: a, c: p2(cx, cy), ccw }] }
}
fn verts(l: &OLoop) -> Vec<P2> {
    l.segs.iter().map(|s| s.start()).collect()
}
fn has(vs: &[P2], x: f64, y: f64) -> bool {
    vs.iter().any(|p| (p.x - x).abs() < 1e-12 && (p.y - y).abs() < 1e-12)
}

#[test]
fn square_shrinks_to_a_square() {
    let rg = Region { loops: vec![rect(0.0, 0.0, 10.0, 10.0)] };
    let (out, st) = offset(&rg, 2.0).unwrap();
    assert_eq!(out.len(), 1, "{out:?}");
    let v = verts(&out[0]);
    assert_eq!(v.len(), 4, "{out:?} {st:?}");
    for (x, y) in [(2.0, 2.0), (8.0, 2.0), (8.0, 8.0), (2.0, 8.0)] {
        assert!(has(&v, x, y), "{v:?}");
    }
    // exactly half the width: C is the centre point only (not a loop); past it, empty
    assert!(offset(&rg, 5.0).unwrap().0.is_empty());
    assert!(offset(&rg, 6.0).unwrap().0.is_empty());
}

#[test]
fn square_grows_with_round_corners() {
    let rg = Region { loops: vec![rect(0.0, 0.0, 10.0, 10.0)] };
    let (out, _) = offset(&rg, -1.0).unwrap();
    assert_eq!(out.len(), 1);
    let s = &out[0].segs;
    assert_eq!(s.len(), 8, "{s:?}");
    assert_eq!(s.iter().filter(|x| matches!(x, Seg::Arc { .. })).count(), 4);
    // the outward offset's loop has the outside region on its left: it runs clockwise
    let v = verts(&out[0]);
    assert!(has(&v, 0.0, -1.0) && has(&v, 10.0, -1.0) && has(&v, 11.0, 0.0));
}

#[test]
fn slot_exactly_twice_r_is_there_and_back() {
    let slot = Loop {
        segs: vec![
            Seg::Line { a: p2(0.0, 0.0), b: p2(10.0, 0.0) },
            Seg::Arc { a: p2(10.0, 0.0), b: p2(10.0, 4.0), c: p2(10.0, 2.0), ccw: true },
            Seg::Line { a: p2(10.0, 4.0), b: p2(0.0, 4.0) },
            Seg::Arc { a: p2(0.0, 4.0), b: p2(0.0, 0.0), c: p2(0.0, 2.0), ccw: true },
        ],
    };
    let rg = Region { loops: vec![slot] };
    let (out, _) = offset(&rg, 2.0).unwrap();
    assert_eq!(out.len(), 1, "{out:?}");
    let s = &out[0].segs;
    assert_eq!(s.len(), 2, "{s:?}");
    let v = verts(&out[0]);
    assert!(has(&v, 0.0, 2.0) && has(&v, 10.0, 2.0), "{v:?}");
    // a little less: a thin loop
    let (out, _) = offset(&rg, 1.5).unwrap();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].segs.len(), 4);
}

#[test]
fn island_and_topology_change() {
    // a 40 x 10 pocket with a round island of radius 2 at its middle: at r = 1.5 the loop goes
    // round the island; at r = 2 the band over the island closes up (width 3 above, 3 below);
    // past 1.5 the island splits the pocket's offset into two loops
    let rg = Region { loops: vec![rect(0.0, 0.0, 40.0, 10.0), circle(20.0, 5.0, 2.0, false)] };
    let (out, _) = offset(&rg, 1.0).unwrap();
    assert_eq!(out.len(), 2, "{out:?}");
    let (out, _) = offset(&rg, 1.6).unwrap();
    assert_eq!(out.len(), 2, "{out:?}");
    let (out, _) = offset(&rg, 3.0).unwrap();
    assert!(out.len() == 2, "{out:?}");
}

#[test]
fn circular_pocket_the_size_of_the_tool_is_a_point() {
    let rg = Region { loops: vec![circle(0.0, 0.0, 3.0, true)] };
    let (out, _) = offset(&rg, 3.0).unwrap();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].segs, vec![Seg::Line { a: p2(0.0, 0.0), b: p2(0.0, 0.0) }]);
    let (out, _) = offset(&rg, 1.0).unwrap();
    assert_eq!(out.len(), 1);
    match out[0].segs[0] {
        Seg::Arc { a, b, ccw, .. } => assert!(a == b && ccw && (a.x - 2.0).abs() < 1e-12),
        s => panic!("{s:?}"),
    }
}

#[test]
fn l_shape_reflex_corner_gets_its_arc() {
    // an L, outward: the reflex corner of the L's inside becomes... inward: the reflex corner
    // gives an arc of radius r about it
    let l = Loop {
        segs: vec![
            Seg::Line { a: p2(0.0, 0.0), b: p2(20.0, 0.0) },
            Seg::Line { a: p2(20.0, 0.0), b: p2(20.0, 10.0) },
            Seg::Line { a: p2(20.0, 10.0), b: p2(10.0, 10.0) },
            Seg::Line { a: p2(10.0, 10.0), b: p2(10.0, 20.0) },
            Seg::Line { a: p2(10.0, 20.0), b: p2(0.0, 20.0) },
            Seg::Line { a: p2(0.0, 20.0), b: p2(0.0, 0.0) },
        ],
    };
    let rg = Region { loops: vec![l] };
    let (out, _) = offset(&rg, 2.0).unwrap();
    assert_eq!(out.len(), 1);
    let arcs: Vec<_> = out[0].segs.iter().filter(|s| matches!(s, Seg::Arc { .. })).collect();
    assert_eq!(arcs.len(), 1, "{out:?}");
    match arcs[0] {
        Seg::Arc { a, b, c, ccw } => {
            assert!(!ccw && (c.x - 10.0).abs() < 1e-12 && (c.y - 10.0).abs() < 1e-12);
            assert!(has(&[*a, *b], 10.0, 8.0) && has(&[*a, *b], 8.0, 10.0), "{a:?} {b:?}");
        }
        _ => unreachable!(),
    }
}
