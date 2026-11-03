//! The picture hides what is behind thin sheet at assembly scale: a 1.5 mm plate in a scene a
//! metre across hides edges 0.5 to 5 mm behind it, at any picture size and from any side, and a
//! face that touches a sheet from behind hides behind it.
use forge_agent::history::model;
use forge_agent::render::{Picture, picture};
use forge_math::{Frame, Point3, Vec3};
use forge_topo::Body;

fn part(src: &str) -> Body {
    let m = model(src);
    assert!(m.error.is_none(), "{src}: {:?}", m.error.map(|e| e.msg));
    m.body().expect("a body").clone()
}

/// `src` built, moved by (x, y, z).
fn at(src: &str, x: f64, y: f64, z: f64) -> Body {
    let f = Frame::from_axes(Point3::new(x, y, z), Vec3::X, Vec3::Y, Vec3::Z).unwrap();
    forge_asm::check::rigid_copy(&part(src), &f)
}

fn scene(parts: &[Body]) -> Body {
    let mut all = Body::new();
    for p in parts {
        all.append(p);
    }
    all
}

fn draw(body: &Body, view: &str, px: usize) -> Picture {
    let mut cache = forge_agent::facemesh::Cache::default();
    let mesh = forge_agent::facemesh::join(&cache.faces(body));
    picture(body, &mesh, view, px, &[]).unwrap()
}

/// Ink on or next to the pixel a point lands on.
fn inked(p: &Picture, q: Point3) -> bool {
    let (x, y, _) = (p.to_px)(q);
    (-1..=1).any(|dy| {
        (-1..=1).any(|dx| {
            let (xx, yy) = (x as isize + dx, y as isize + dy);
            xx >= 0
                && yy >= 0
                && (xx as usize) < p.w
                && (yy as usize) < p.h
                && p.ink(xx as usize, yy as usize)
        })
    })
}

/// Points along the top edges of a block 100 x 100 at (450, 450), inset from its corners.
fn block_edges(top: f64) -> Vec<Point3> {
    let mut out = Vec::new();
    for i in 1..10 {
        let t = 450.0 + i as f64 * 10.0;
        out.extend([
            Point3::new(t, 450.0, top),
            Point3::new(t, 550.0, top),
            Point3::new(450.0, t, top),
            Point3::new(550.0, t, top),
        ]);
    }
    out
}

/// A plate 1.5 thick over a block whose top is `gap` below the plate, and a speck at the far
/// corner so the scene is a metre across.
fn plate_over_block(gap: f64, plate: bool) -> Body {
    let mut parts = vec![
        at("extrude 40 rect 450 450 100 100", 0.0, 0.0, -gap - 40.0),
        at("extrude 5 rect 0 0 5 5", 1000.0, 1000.0, -100.0),
        at("extrude 5 rect 0 0 5 5", -5.0, -5.0, -100.0),
    ];
    if plate {
        parts.push(part("extrude 1.5 rect 300 300 400 400"));
    }
    scene(&parts)
}

#[test]
fn edges_behind_a_thin_plate_stay_hidden() {
    for gap in [0.5, 1.0, 2.0, 5.0] {
        let open = plate_over_block(gap, false);
        let shut = plate_over_block(gap, true);
        for view in ["top", "iso"] {
            for px in [300, 600, 2400] {
                let seen = draw(&open, view, px);
                let edges = block_edges(-gap);
                // the probes are on the edges: with no plate they show
                let shown = edges.iter().filter(|&&q| inked(&seen, q)).count();
                assert!(
                    shown * 10 >= edges.len() * 9,
                    "{view} {px}: {shown} of {} shown",
                    edges.len()
                );
                let pic = draw(&shut, view, px);
                let bled: Vec<_> = edges.iter().filter(|&&q| inked(&pic, q)).collect();
                assert!(
                    bled.is_empty(),
                    "gap {gap}, {view} at {px} px: {} edge points through the plate",
                    bled.len()
                );
                // the plate's own edges still show
                for q in [Point3::new(500.0, 300.0, 1.5), Point3::new(700.0, 500.0, 1.5)] {
                    assert!(
                        inked(&pic, q),
                        "gap {gap}, {view} at {px} px: plate edge at {q:?} missing"
                    );
                }
            }
        }
    }
}

/// A side panel's inner face touches a post: the post's holes there are behind the panel.
#[test]
fn a_face_touching_a_sheet_from_behind_stays_hidden() {
    let post = at(
        "extrude 3 rect 450 450 100 100\nhole 10 -xy at 500 500 3 thru",
        0.0,
        0.0,
        -3.0,
    );
    let panel = part("extrude 1.5 rect 300 300 400 400");
    let far = at("extrude 5 rect 0 0 5 5", 1000.0, 1000.0, -100.0);
    let body = scene(&[post, panel, far]);
    let rim: Vec<Point3> = (0..16)
        .map(|i| {
            let a = i as f64 / 16.0 * std::f64::consts::TAU;
            Point3::new(
                500.0 + 5.0 * forge_math::dmath::cos(a),
                500.0 + 5.0 * forge_math::dmath::sin(a),
                0.0,
            )
        })
        .collect();
    for view in ["top", "iso"] {
        for px in [300, 600, 2400] {
            let pic = draw(&body, view, px);
            let bled = rim.iter().filter(|&&q| inked(&pic, q)).count();
            assert_eq!(bled, 0, "{view} at {px} px: hole rim shows through the panel");
            for q in [Point3::new(450.0, 500.0, 0.0), Point3::new(500.0, 550.0, 0.0)] {
                assert!(!inked(&pic, q), "{view} at {px} px: post edge at {q:?} shows");
            }
        }
    }
}

/// Two plates side by side, flush: their faces shade as one, the seam between them is drawn,
/// and no pixel of either face speckles with the other's depth.
#[test]
fn flush_faces_side_by_side_draw_cleanly() {
    let a = part("extrude 1.5 rect 0 0 500 1000");
    let b = part("extrude 1.5 rect 500 0 500 1000");
    let body = scene(&[a, b]);
    for view in ["top", "iso"] {
        let pic = draw(&body, view, 1200);
        assert!(
            inked(&pic, Point3::new(500.0, 500.0, 1.5)),
            "{view}: seam missing"
        );
        let shade = |q: Point3| {
            let (x, y, _) = (pic.to_px)(q);
            pic.px[y as usize * pic.w + x as usize]
        };
        let first = shade(Point3::new(250.0, 500.0, 1.5));
        for i in 1..20 {
            let q = Point3::new(i as f64 * 50.0 + 7.0, 333.0, 1.5);
            if (q.x - 500.0).abs() > 10.0 {
                assert_eq!(shade(q), first, "{view}: shade at {q:?}");
            }
        }
    }
}
