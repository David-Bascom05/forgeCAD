//! End to end: a real drawing's entities trimmed, offset, filleted, mirrored and broken through
//! the editing core, written, read back and checked.

use super::commit::{add_like, replace};
use super::entity::{from_entity, transform_data, write_curve};
use super::fillet::fillet;
use super::geom::{PI, V2, v2};
use super::offset::offset;
use super::ops::trim;
use super::{Affine, Curve};
use crate::edit::History;
use crate::file::{self, Drawing};
use crate::types::Data;

fn load() -> Drawing {
    let b = std::fs::read(format!("{}/tests/data/mix_2018.dwg", env!("CARGO_MANIFEST_DIR"))).unwrap();
    file::read(&b).unwrap()
}

fn wc(d: &Drawing, c: &Curve, like: u32) -> Data {
    write_curve(c, &d.objects[&like].data).unwrap()
}

fn curve(d: &Drawing, h: u32) -> Curve {
    from_entity(&d.objects[&h].data).unwrap()
}

#[test]
fn trim_offset_fillet_a_real_drawing() {
    let orig = load();
    let mut d = orig.clone();
    let mut hist = History::default();
    hist.begin();

    // the drawing's circle (centre (100, 150), radius 36) and a line across it
    let circle_h = 0x69;
    let circle = curve(&d, circle_h);
    let Curve::Circle { c, r } = circle else {
        panic!("{circle:?}")
    };
    assert!((r - 36.0).abs() < 1e-9);
    let across = Curve::line(c - v2(50.0, 0.0), c + v2(50.0, 0.0));
    let line_h = {
        let x = wc(&d, &across, circle_h);
        add_like(&mut d, &mut hist, circle_h, x)
    }
    .unwrap();

    // TRIM the circle's top half by the line: an ARC now
    let pieces = trim(&circle, std::slice::from_ref(&across), c + v2(0.0, r), false).unwrap();
    let data: Vec<Data> = pieces
        .iter()
        .map(|p| write_curve(p, &d.objects[&circle_h].data).unwrap())
        .collect();
    let arc_hs = replace(&mut d, &mut hist, circle_h, data).unwrap();
    assert_eq!(arc_hs.len(), 1);
    // TRIM the line's left end by the circle
    let cut = trim(&across, std::slice::from_ref(&circle), c - v2(45.0, 0.0), false).unwrap();
    let data: Vec<Data> = cut
        .iter()
        .map(|p| write_curve(p, &d.objects[&line_h].data).unwrap())
        .collect();
    assert_eq!(replace(&mut d, &mut hist, line_h, data).unwrap(), vec![line_h]);

    // OFFSET the arc (centre (20, 30), radius 4) outward, the ellipse and the polyline
    let arc = curve(&d, 0x9e);
    let o = offset(&arc, 1.0, v2(20.0, 40.0)).unwrap();
    let off_arc = {
        let x = wc(&d, &o, 0x9e);
        add_like(&mut d, &mut hist, 0x9e, x)
    }
    .unwrap();
    let el = curve(&d, 0x9d);
    let oe = offset(&el, 2.0, v2(0.0, 0.0)).unwrap();
    let off_el = {
        let x = wc(&d, &oe, 0x9d);
        add_like(&mut d, &mut hist, 0x9d, x)
    }
    .unwrap();
    let pl = curve(&d, 0x9c);
    let op = offset(&pl, 0.5, v2(3.0, 0.0)).unwrap();
    let off_pl = {
        let x = wc(&d, &op, 0x9c);
        add_like(&mut d, &mut hist, 0x9c, x)
    }
    .unwrap();

    // FILLET the rectangle's top right corner: lines 0x6a (x = 224 up to 186) and 0x6e (y = 186)
    let (a, b) = (curve(&d, 0x6a), curve(&d, 0x6e));
    let f = fillet(&a, v2(224.0, 160.0), &b, v2(200.0, 186.0), 5.0).unwrap();
    let fa = write_curve(&f.a, &d.objects[&0x6a].data).unwrap();
    let fb = write_curve(&f.b, &d.objects[&0x6e].data).unwrap();
    replace(&mut d, &mut hist, 0x6a, vec![fa]).unwrap();
    replace(&mut d, &mut hist, 0x6e, vec![fb]).unwrap();
    let fillet_h = {
        let x = wc(&d, f.added.as_ref().unwrap(), 0x6a);
        add_like(&mut d, &mut hist, 0x6a, x)
    }
    .unwrap();

    // MIRROR a text (MIRRTEXT 0) in place
    let m = Affine::mirror(v2(0.0, 0.0), v2(0.0, 1.0));
    let t = transform_data(&d.objects[&0x70].data, &m, false).unwrap();
    replace(&mut d, &mut hist, 0x70, vec![t]).unwrap();
    hist.end();

    let out = file::write(&d).unwrap();
    let e = file::read(&out).unwrap();
    let ms = e.model_space();
    // each in model space on its source's layer, with its colour
    for (h, like) in [
        (arc_hs[0], circle_h),
        (line_h, circle_h),
        (off_arc, 0x9e),
        (off_el, 0x9d),
        (off_pl, 0x9c),
        (fillet_h, 0x6a),
        (0x6a, 0x6a),
        (0x6e, 0x6e),
        (0x70, 0x70),
    ] {
        assert_eq!(e.block_of(h), Some(ms), "{h:#x}");
        let (x, y) = (
            e.objects[&h].ent.as_ref().unwrap(),
            orig.objects[&like].ent.as_ref().unwrap(),
        );
        assert_eq!(
            (x.layer, &x.color, x.lineweight),
            (y.layer, &y.color, y.lineweight),
            "{h:#x}"
        );
    }
    assert!(!e.objects.contains_key(&circle_h));
    // the circle's lower half
    let Data::Arc(x) = &e.objects[&arc_hs[0]].data else {
        panic!()
    };
    assert!((x.start - PI).abs() < 1e-9 && x.end.abs() < 1e-9 && (x.circle.radius - 36.0).abs() < 1e-12);
    // the line from the circle's left crossing
    let lc = from_entity(&e.objects[&line_h].data).unwrap();
    assert!(lc.start().dist(c - v2(36.0, 0.0)) < 1e-9 && lc.end().dist(c + v2(50.0, 0.0)) < 1e-9);
    let Data::Arc(oa) = &e.objects[&off_arc].data else {
        panic!()
    };
    assert!((oa.circle.radius - 5.0).abs() < 1e-12);
    // the ellipse's offset: a fitted SPLINE 2 from the ellipse
    let Data::Spline(_) = &e.objects[&off_el].data else {
        panic!()
    };
    let sc = from_entity(&e.objects[&off_el].data).unwrap();
    for k in 0..=20 {
        let p = sc.point(sc.domain().0 + (sc.domain().1 - sc.domain().0) * k as f64 / 20.0);
        assert!((el.dist(p) - 2.0).abs() < 1e-4, "{k}");
    }
    let Data::LwPolyline(lp) = &e.objects[&off_pl].data else {
        panic!()
    };
    assert_eq!(lp.points.len(), 3);
    // the fillet's lines and arc
    let la = from_entity(&e.objects[&0x6a].data).unwrap();
    let lb = from_entity(&e.objects[&0x6e].data).unwrap();
    assert!(
        la.end().dist(v2(224.0, 181.0)) < 1e-4 && lb.start().dist(v2(219.0, 186.0)) < 1e-4,
        "{la:?} {lb:?}"
    );
    let Data::Arc(fa) = &e.objects[&fillet_h].data else {
        panic!()
    };
    assert!(
        (fa.circle.radius - 5.0).abs() < 1e-12 && V2::from(fa.circle.center).dist(v2(219.0, 181.0)) < 1e-4
    );
    let Data::Text(tx) = &e.objects[&0x70].data else {
        panic!()
    };
    let Data::Text(t0) = &orig.objects[&0x70].data else {
        panic!()
    };
    assert!((tx.insert[0] + t0.insert[0]).abs() < 1e-9 && tx.rotation == 0.0 && tx.generation == 0);

    // undo puts it all back
    assert!(hist.undo(&mut d));
    assert!(d.objects.keys().eq(orig.objects.keys()));
    assert_eq!(d.objects[&0x6a].data, orig.objects[&0x6a].data);
    assert!(matches!(d.objects[&circle_h].data, Data::Circle(_)));
}
