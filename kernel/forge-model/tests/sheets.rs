//! Sheets in a part's history: a body of their own, never the body a solid feature acts on, and
//! named by a feature that changes material, refused (`OnSheet`). Sequential and n-ary agree.

use forge_math::{Frame, Point2, Point3, Vec2};
use forge_model::sheets::{Sheet, SheetHow};
use forge_model::{BodyError, BodyFeature, Combine, Extent, Feature, Part, RegenError};
use forge_ops::{Axis2, Profile, Segment};
use forge_topo::check;
use forge_topo::mass::{mass, sheet_area};

fn block(corner: Point3, w: f64, d: f64, h: f64, op: Combine) -> Feature {
    Feature::Extrude {
        profile: Profile {
            plane: Frame::WORLD.with_origin(corner),
            loops: vec![Profile::rectangle(Point2::ORIGIN, w, d, 0)],
        },
        extent: Extent::Distance(h),
        op,
    }
}

fn wall() -> Feature {
    let l = |a: (f64, f64), b: (f64, f64)| Segment::Line {
        a: Point2::new(a.0, a.1),
        b: Point2::new(b.0, b.1),
    };
    Feature::Sheet(Box::new(Sheet {
        plane: Frame::WORLD,
        chains: vec![vec![
            (l((0.0, 30.0), (20.0, 30.0)), 1),
            (l((20.0, 30.0), (20.0, 50.0)), 2),
        ]],
        how: SheetHow::Extrude { lo: -2.0, hi: 8.0 },
        thin: None,
    }))
}

fn both(features: Vec<Feature>) -> Vec<forge_model::Solid> {
    let part = Part { features };
    let seq = part.regenerate_solids().expect("sequential");
    let nary = part.regenerate_nary_solids(2).expect("n-ary");
    assert_eq!(seq.len(), nary.len());
    for (a, b) in seq.iter().zip(&nary) {
        assert_eq!(a.body.is_sheet(), b.body.is_sheet());
        assert!(check(&a.body).is_empty(), "{:?}", check(&a.body));
        assert!(check(&b.body).is_empty(), "{:?}", check(&b.body));
        // a sheet's mesh is open only along its boundary, once; a solid's closed
        let c = a.body.compact();
        let m = forge_mesh::tessellate(&c, &forge_mesh::Quality::default()).unwrap();
        assert!(m.open_edges_of(&c).is_empty());
        assert_eq!(m.open_edges().is_empty(), !c.is_sheet());
    }
    seq
}

#[test]
fn a_sheet_is_a_body_of_its_own_and_an_add_goes_to_the_solid() {
    let s = both(vec![
        block(Point3::ORIGIN, 10.0, 10.0, 10.0, Combine::New),
        wall(),
        block(Point3::new(10.0, 0.0, 0.0), 5.0, 10.0, 10.0, Combine::Add),
    ]);
    assert_eq!(s.len(), 2);
    let solid = s.iter().find(|x| !x.body.is_sheet()).unwrap();
    assert!((mass(&solid.body).unwrap().volume - 1500.0).abs() < 1e-6);
    let sheet = s.iter().find(|x| x.body.is_sheet()).unwrap();
    assert_eq!(sheet.id, 1);
    assert!((sheet_area(&sheet.body).unwrap().0 - 400.0).abs() < 1e-9);
}

#[test]
fn a_sheet_first_then_a_solid() {
    let s = both(vec![
        wall(),
        block(Point3::ORIGIN, 10.0, 10.0, 10.0, Combine::New),
    ]);
    assert_eq!(s.len(), 2);
    // a sheet alone gives nothing for an add to join
    let r = Part {
        features: vec![wall(), block(Point3::ORIGIN, 10.0, 10.0, 10.0, Combine::Add)],
    }
    .regenerate_solids();
    assert!(
        matches!(r, Err(RegenError::NothingToCombine { feature: 1 })),
        "{r:?}"
    );
}

#[test]
fn combining_with_a_sheet_is_refused() {
    let r = Part {
        features: vec![
            block(Point3::ORIGIN, 10.0, 10.0, 10.0, Combine::New),
            wall(),
            Feature::Body(Box::new(BodyFeature::Combine {
                op: Combine::Cut,
                target: 0,
                tools: vec![1],
                keep: false,
            })),
        ],
    }
    .regenerate_solids();
    assert_eq!(
        r,
        Err(RegenError::Body {
            feature: 2,
            error: BodyError::OnSheet { id: 1 }
        })
    );
}

#[test]
fn a_revolved_sheet_from_an_angle() {
    let s = both(vec![Feature::Sheet(Box::new(Sheet {
        plane: Frame::WORLD,
        chains: vec![vec![(
            Segment::Line {
                a: Point2::new(5.0, 0.0),
                b: Point2::new(5.0, 4.0),
            },
            1,
        )]],
        how: SheetHow::Revolve {
            axis: Axis2 {
                origin: Point2::ORIGIN,
                dir: Vec2 { x: 0.0, y: 1.0 },
            },
            from: 1.0,
            angle: 2.0,
        },
        thin: None,
    }))]);
    let a = sheet_area(&s[0].body).unwrap().0;
    assert!((a - 5.0 * 2.0 * 4.0).abs() < 1e-9, "{a}");
}

#[test]
fn a_thin_wall_joins_the_part() {
    let l = |a: (f64, f64), b: (f64, f64)| Segment::Line {
        a: Point2::new(a.0, a.1),
        b: Point2::new(b.0, b.1),
    };
    // an L of wall 2 thick, out (its corner's outside squared), 10 high, on a 10 x 10 x 10 block
    let wall = Feature::Sheet(Box::new(Sheet {
        plane: Frame::WORLD,
        chains: vec![vec![
            (l((0.0, 10.0), (10.0, 10.0)), 1),
            (l((10.0, 10.0), (10.0, 20.0)), 2),
        ]],
        how: SheetHow::Extrude { lo: 0.0, hi: 10.0 },
        thin: Some(forge_model::sheets::Thin {
            lo: -2.0,
            hi: 0.0,
            op: Combine::Add,
        }),
    }));
    let part = Part {
        features: vec![block(Point3::ORIGIN, 10.0, 10.0, 10.0, Combine::New), wall],
    };
    let b = part.regenerate().expect("builds");
    assert!(check(&b).is_empty(), "{:?}", check(&b));
    // in (the left of the chain): 20 long less the inner corner, 2 thick, 10 high
    let v = mass(&b).unwrap().volume;
    assert!((v - (1000.0 + 10.0 * (2.0 * 20.0 - 4.0))).abs() < 1e-9, "{v}");
}

#[test]
fn a_sheet_thickened_joins_the_solid_or_is_a_body() {
    use forge_model::{Edit, Side};
    let s = Part {
        features: vec![block(Point3::ORIGIN, 10.0, 10.0, 10.0, Combine::New), wall()],
    }
    .regenerate_solids()
    .unwrap();
    let faces: Vec<forge_topo::Name> = {
        let b = &s.iter().find(|x| x.body.is_sheet()).unwrap().body;
        b.face_ids().map(|f| b.face(f).name).collect()
    };
    for (op, bodies, vol) in [
        (Combine::Add, 2, 1000.0 + 10.0 * (40.0 + 1.0)),
        (Combine::New, 3, 1000.0 + 10.0 * (40.0 + 1.0)),
    ] {
        let s = both(vec![
            block(Point3::ORIGIN, 10.0, 10.0, 10.0, Combine::New),
            wall(),
            Feature::Edit(Edit::Thicken {
                faces: faces.clone(),
                thickness: 1.0,
                side: Side::Pos,
                op,
            }),
        ]);
        assert_eq!(s.len(), bodies);
        let v: f64 = s.iter().map(|x| mass(&x.body).unwrap().volume).sum();
        assert!((v - vol).abs() < 1e-9, "{op:?} {v}");
        assert!(s.iter().any(|x| x.body.is_sheet()));
    }
}
