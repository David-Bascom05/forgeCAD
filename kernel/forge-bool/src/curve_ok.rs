//! H.curveOK's conic parameters, asserted on every operand in debug and test builds (release:
//! nothing, the zero-speed rule). `across` builds its surface from an ellipse edge's radii and a
//! parabola edge's focal length; with them finite, the major radius positive and the focal length
//! not zero (a negative one opens the other way, as a STEP file may say) its normalize cannot fail
//! there (verify/forge-bool-fallback Across.lean,
//! `axis_sq_ge_one`, `across_ok`; LEDGER H.curveOK, U.across). A violation is a bug in the
//! feature that made the operand.

use forge_geom::Curve;
use forge_topo::Body;

/// Each live ellipse and parabola edge of each operand has finite parameters, a positive major
/// radius and a focal length that is not zero.
#[inline]
pub(crate) fn debug_check(_bodies: &[&Body]) {
    #[cfg(debug_assertions)]
    for b in _bodies {
        for (i, e) in b.edges.iter().enumerate() {
            if !e.alive {
                continue;
            }
            match &e.curve {
                Curve::Ellipse(el) => assert!(
                    el.major.is_finite() && el.major > 0.0 && el.minor.is_finite() && el.minor >= 0.0,
                    "boolean operand outside H.curveOK: ellipse edge {i} with radii {} {}",
                    el.major,
                    el.minor
                ),
                Curve::Parabola(pa) => assert!(
                    pa.focal.is_finite() && pa.focal != 0.0,
                    "boolean operand outside H.curveOK: parabola edge {i} with focal length {}",
                    pa.focal
                ),
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_geom::{Ellipse, Line};
    use forge_math::{Frame, Interval, Point3, UnitVec3};
    use forge_topo::Name;

    fn with_curve(curve: Curve) -> Body {
        let mut b = Body::new();
        let v = b.add_vertex(Point3::ORIGIN, Name::NONE);
        b.add_edge(curve, Interval::new(0.0, 1.0), v, v, Name::NONE);
        b
    }

    #[test]
    #[should_panic(expected = "outside H.curveOK")]
    fn an_ellipse_with_no_major_radius_is_caught() {
        let b = with_curve(Curve::Ellipse(Ellipse {
            frame: Frame::WORLD,
            major: 0.0,
            minor: 1.0,
        }));
        debug_check(&[&b]);
    }

    #[test]
    #[should_panic(expected = "outside H.curveOK")]
    fn a_parabola_with_no_focal_length_is_caught() {
        let b = with_curve(Curve::Parabola(forge_geom::Parabola {
            frame: Frame::WORLD,
            focal: 0.0,
        }));
        debug_check(&[&b]);
    }

    #[test]
    fn a_line_a_good_ellipse_and_a_parabola_opening_the_other_way_pass() {
        let b = with_curve(Curve::Line(Line::new(Point3::ORIGIN, UnitVec3::X)));
        debug_check(&[&b]);
        let b = with_curve(Curve::Ellipse(Ellipse {
            frame: Frame::WORLD,
            major: 2.0,
            minor: 1.0,
        }));
        debug_check(&[&b]);
        let b = with_curve(Curve::Parabola(forge_geom::Parabola {
            frame: Frame::WORLD,
            focal: -0.5,
        }));
        debug_check(&[&b]);
    }
}
