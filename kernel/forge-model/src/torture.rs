//! A deliberately nasty test part: a plate with 200 features that overlap
//! one another, placed by a fixed-seed generator so it is the same part
//! every time.

use crate::{Combine, Extent, Feature, Part};
use forge_math::rng::Rng;
use forge_math::{Frame, Point2, Point3, Vec3};
use forge_ops::Profile;

pub const PLATE: (f64, f64, f64) = (300.0, 200.0, 25.0);

fn circle_at(x: f64, y: f64, z: f64, r: f64) -> Profile {
    Profile {
        plane: Frame::WORLD.with_origin(Point3::new(x, y, z)),
        loops: vec![Profile::circle(Point2::ORIGIN, r, 0)],
    }
}

/// The first `count` features of the torture part (221 in all: a plate, 60
/// through holes, 40 pockets, 30 bosses, 30 slots, 20 dimples, 20 counterbores).
pub fn part(count: usize) -> Part {
    let (w, h, t) = PLATE;
    let mut r = Rng::new(0x7047);
    let mut features = vec![Feature::Extrude {
        profile: Profile {
            plane: Frame::WORLD,
            loops: vec![Profile::rectangle(Point2::ORIGIN, w, h, 0)],
        },
        extent: Extent::Distance(t),
        op: Combine::New,
    }];
    let spot = |r: &mut Rng, m: f64| (r.range(m, w - m), r.range(m, h - m));
    for _ in 0..60 {
        let (x, y) = spot(&mut r, 8.0);
        features.push(Feature::Extrude {
            profile: circle_at(x, y, -1.0, r.range(1.5, 5.0)),
            extent: Extent::Distance(t + 2.0),
            op: Combine::Cut,
        });
    }
    for _ in 0..40 {
        let (x, y) = spot(&mut r, 10.0);
        let depth = r.range(4.0, 15.0);
        features.push(Feature::Extrude {
            profile: circle_at(x, y, t - depth, r.range(3.0, 8.0)),
            extent: Extent::Distance(depth + 1.0),
            op: Combine::Cut,
        });
    }
    for _ in 0..30 {
        let (x, y) = spot(&mut r, 10.0);
        features.push(Feature::Extrude {
            profile: circle_at(x, y, t - 1.3, r.range(2.0, 6.0)),
            extent: Extent::Distance(r.range(5.0, 20.0)),
            op: Combine::Add,
        });
    }
    for _ in 0..30 {
        let (x, y) = spot(&mut r, 15.0);
        let a = r.range(0.0, 3.1);
        let (sa, ca) = forge_math::dmath::sin_cos(a);
        let plane = Frame::from_zx(
            Point3::new(x, y, t - r.range(3.0, 8.0)),
            Vec3::Z.normalize().unwrap(),
            Vec3::new(ca, sa, 0.0),
        )
        .unwrap();
        let (sw, sh) = (r.range(10.0, 30.0), r.range(3.0, 8.0));
        features.push(Feature::Extrude {
            profile: Profile {
                plane,
                loops: vec![Profile::rectangle(Point2::new(-sw / 2.0, -sh / 2.0), sw, sh, 0)],
            },
            extent: Extent::Distance(20.0),
            op: Combine::Cut,
        });
    }
    for _ in 0..20 {
        let (x, y) = spot(&mut r, 10.0);
        let rad = r.range(3.0, 7.0);
        let frame = Frame::from_zx(
            Point3::new(x, y, t + rad * r.range(0.2, 0.8)),
            Vec3::Y.normalize().unwrap(),
            Vec3::X,
        )
        .unwrap();
        features.push(Feature::Ball {
            frame,
            radius: rad,
            op: Combine::Cut,
        });
    }
    for _ in 0..20 {
        let (x, y) = spot(&mut r, 12.0);
        let hole = r.range(2.0, 4.0);
        features.push(Feature::Extrude {
            profile: circle_at(x, y, -1.0, hole),
            extent: Extent::Distance(t + 2.0),
            op: Combine::Cut,
        });
        let depth = r.range(3.0, 6.0);
        features.push(Feature::Extrude {
            profile: circle_at(x, y, t - depth, hole * 1.8),
            extent: Extent::Distance(depth + 1.0),
            op: Combine::Cut,
        });
    }
    features.truncate(count.max(1));
    Part { features }
}
