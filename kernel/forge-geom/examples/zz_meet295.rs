//! (scratch, not committed) fuzz 295's torus against its two cylinders, by meet::intersect
use forge_geom::intersect::SurfaceSurface;
use forge_geom::{CurveGeom, Cylinder, Surface, SurfaceGeom, Torus};
use forge_math::{Box3, Frame, Point3, Vec3};

fn frame(o: [f64; 3], x: [f64; 3], y: [f64; 3], z: [f64; 3]) -> Frame {
    let v = |a: [f64; 3]| Vec3::new(a[0], a[1], a[2]);
    Frame::from_axes(Point3::new(o[0], o[1], o[2]), v(x), v(y), v(z)).expect("frame")
}

fn main() {
    let torus = Surface::Torus(Torus {
        frame: frame(
            [57.513833943223545, 82.64151449414277, 31.529651297690172],
            [1.0, 0.0, 0.0],
            [0.0, -1.0, 0.0],
            [0.0, 0.0, -1.0],
        ),
        major: 1.018123715855934,
        minor: 0.22881392007971665,
    });
    let target = Surface::Cylinder(Cylinder {
        frame: frame(
            [49.46533740519021, 77.35050823191905, 22.132670240696257],
            [0.1618463761554596, 0.9775539170300888, 0.13488546928589437],
            [-0.9865098439700136, 0.16368197546096477, -0.0025570802598708677],
            [-0.02457800389789156, -0.13265198908545095, 0.9908578967319525],
        ),
        radius: 10.575244744739232,
    });
    let zt = Vec3::new(-0.04760542051711398, 0.17499343179871532, 0.9834180305265392);
    let xt = Vec3::new(-0.34481495645685883, 0.921132764696941, -0.18060198117801576);
    let tool = Surface::Cylinder(Cylinder {
        frame: Frame::from_axes(
            Point3::new(64.28286796892466, 86.4342803264453, 17.574469684406452),
            xt,
            zt.cross(xt),
            zt,
        )
        .expect("frame"),
        radius: 8.70706734376011,
    });
    let c = Point3::new(57.513833943223545, 82.64151449414277, 31.529651297690172);
    let region = Box3::from_point(c).grow(1.018123715855934 + 0.22881392007971665 + 1e-6);
    for (name, cyl) in [("target", &target), ("tool", &tool)] {
        match forge_geom::meet::intersect(cyl, &torus, region) {
            Some(SurfaceSurface::Curves(cs)) => {
                println!("{name}: {} curves", cs.len());
                for (k, (cv, ct)) in cs.iter().enumerate() {
                    let d = cv.domain();
                    let p = cv.eval(d.lo + 0.3 * (d.hi - d.lo));
                    let q = p - c;
                    let u = (-q.y).atan2(q.x).to_degrees().rem_euclid(360.0);
                    println!(
                        "  {k} {ct:?} closed {:?} u {u:.1} at {p:?} off {:.1e} {:.1e}",
                        d.period,
                        cyl.implicit(p),
                        torus.implicit(p)
                    );
                }
            }
            o => println!("{name}: {o:?}"),
        }
    }
}
