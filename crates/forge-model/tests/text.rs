//! Text embossed and engraved, flat and wrapped: valid solids that add or remove the glyphs'
//! area times their depth (flat), or that area carried between two radii (wrapped).
use forge_font::Seg;
use forge_font::testing::{TestGlyph, build, square_ring};
use forge_math::{Frame, Point2, Point3};
use forge_mesh::{Quality, tessellate};
use forge_model::{Combine, Extent, Feature, FontRef, Part, Text, TextOn};
use forge_ops::Profile;
use std::sync::Arc;

fn font() -> FontRef {
    let bytes = build(
        1000,
        &[
            ('I', TestGlyph::bar(100.0, 0.0, 200.0, 700.0, 300.0)),
            ('O', square_ring()),
        ],
        &[('I', 'O', -50)],
    );
    FontRef {
        name: "test.ttf".into(),
        font: Some(Arc::new(forge_font::Font::parse(bytes).expect("a font"))),
    }
}

/// The glyphs' area in font units: chords, and each arc's 2/3 of its control triangle.
fn units_area(g: &TestGlyph) -> f64 {
    let a2: f64 = g
        .contours
        .iter()
        .flatten()
        .map(|s| match *s {
            Seg::Line(a, b) => a.x * b.y - b.x * a.y,
            Seg::Quad(a, m, b) => a.x * b.y - b.x * a.y + 2.0 / 3.0 * (m - a).cross(b - a),
        })
        .sum();
    a2.abs() / 2.0
}

fn volume(part: &Part) -> f64 {
    let b = part.regenerate().expect("regenerates");
    assert!(forge_topo::check(&b).is_empty(), "{:?}", forge_topo::check(&b));
    let m = tessellate(
        &b,
        &Quality {
            chord: 0.001,
            ..Quality::default()
        },
    )
    .expect("meshes");
    assert!(m.open_edges().is_empty(), "closed mesh");
    m.volume()
}

fn text(on: TextOn, depth: f64, op: Combine) -> Feature {
    Feature::Text(Box::new(Text {
        text: "IO I".into(),
        font: font(),
        size: 10.0,
        on,
        depth,
        op,
    }))
}

/// Area of "IO I" in mm^2 at 10 mm to the em.
fn area() -> f64 {
    let k = 0.01 * 0.01;
    (2.0 * units_area(&TestGlyph::bar(100.0, 0.0, 200.0, 700.0, 300.0)) + units_area(&square_ring())) * k
}

#[test]
fn flat_text_adds_and_removes_its_area_times_depth() {
    let block = Feature::Extrude {
        profile: Profile {
            plane: Frame::WORLD,
            loops: vec![Profile::rectangle(Point2::new(-5.0, -5.0), 40.0, 20.0, 0)],
        },
        extent: Extent::Distance(5.0),
        op: Combine::New,
    };
    let on = TextOn::Plane(Frame::WORLD.with_origin(Point3::new(0.0, 0.0, 5.0)));
    let v0 = 40.0 * 20.0 * 5.0;
    for (op, sign) in [(Combine::Add, 1.0), (Combine::Cut, -1.0)] {
        let v = volume(&Part {
            features: vec![block.clone(), text(on, 1.5, op)],
        });
        let want = v0 + sign * area() * 1.5;
        assert!((v - want).abs() < 1e-3 * v0, "{op:?}: {v} vs {want}");
    }
}

#[test]
fn wrapped_text_adds_and_removes_its_area_between_radii() {
    let r = 12.0;
    let rod = Feature::Extrude {
        profile: Profile {
            plane: Frame::WORLD,
            loops: vec![Profile::circle(Point2::ORIGIN, r, 0)],
        },
        extent: Extent::Distance(20.0),
        op: Combine::New,
    };
    let on = TextOn::Cylinder {
        axis: Frame::WORLD.with_origin(Point3::new(0.0, 0.0, 5.0)),
        radius: r,
    };
    let v0 = volume(&Part {
        features: vec![rod.clone()],
    });
    let d = 0.8;
    // area / R of angle, times the integral of rho between the radii
    let between = |r0: f64, r1: f64| area() / r * (r1 * r1 - r0 * r0) / 2.0;
    for (op, want) in [
        (Combine::Cut, v0 - between(r - d, r)),
        (Combine::Add, v0 + between(r, r + d)),
    ] {
        let v = volume(&Part {
            features: vec![rod.clone(), text(on, d, op)],
        });
        assert!(
            (v - want).abs() < 2e-3 * (v0 - want).abs().max(1.0),
            "{op:?}: {v} vs {want}"
        );
    }
}

/// Scrambled fonts lay out and build into solids or errors, never a panic.
#[test]
fn scrambled_fonts_never_panic() {
    let base = build(
        1000,
        &[
            ('I', TestGlyph::bar(100.0, 0.0, 200.0, 700.0, 300.0)),
            ('O', square_ring()),
        ],
        &[('I', 'O', -50)],
    );
    let mut r = forge_math::rng::Rng::new(77);
    let places = [
        forge_ops::Place::Flat(Frame::WORLD),
        forge_ops::Place::Wrapped {
            axis: Frame::WORLD,
            radius: 20.0,
        },
    ];
    for _ in 0..200 {
        let mut d = base.clone();
        for _ in 0..1 + r.below(6) {
            let k = r.below(d.len());
            d[k] = r.below(256) as u8;
        }
        let Ok(f) = forge_font::Font::parse(d) else {
            continue;
        };
        let Ok(regions) = forge_model::text::layout(&f, "IO OI", 10.0) else {
            continue;
        };
        for reg in regions.iter().take(8) {
            for place in places {
                let _ = forge_ops::glyph(reg, place, 20.0, 21.0, forge_topo::Name::feature(1));
            }
        }
    }
}
