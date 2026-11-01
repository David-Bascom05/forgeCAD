//! A closed face with rims and no seam takes its seam on the side the rims say, long or short.
use forge_math::dmath;
use forge_step::{Limits, import};
use forge_topo::{check, mass};

/// Three quarters of a ring: a torus face between two rims, with no edge between them, and two discs.
const RING: &str = "ISO-10303-21;HEADER;ENDSEC;DATA;
#1=(LENGTH_UNIT()NAMED_UNIT(*)SI_UNIT(.MILLI.,.METRE.));
#2=(NAMED_UNIT(*)PLANE_ANGLE_UNIT()SI_UNIT($,.RADIAN.));
#3=(NAMED_UNIT(*)SI_UNIT($,.STERADIAN.)SOLID_ANGLE_UNIT());
#4=UNCERTAINTY_MEASURE_WITH_UNIT(LENGTH_MEASURE(1.E-5),#1,'distance_accuracy_value','');
#5=(GEOMETRIC_REPRESENTATION_CONTEXT(3)GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT((#4))GLOBAL_UNIT_ASSIGNED_CONTEXT((#1,#2,#3))REPRESENTATION_CONTEXT('',''));
#6=ADVANCED_BREP_SHAPE_REPRESENTATION('',(#90),#5);
#10=CARTESIAN_POINT('',(0.,0.,0.));
#11=DIRECTION('',(0.,0.,1.));
#12=DIRECTION('',(1.,0.,0.));
#13=AXIS2_PLACEMENT_3D('',#10,#11,#12);
#14=TOROIDAL_SURFACE('',#13,10.,2.);
#20=CARTESIAN_POINT('',(10.,0.,0.));
#21=DIRECTION('',(0.,-1.,0.));
#22=AXIS2_PLACEMENT_3D('',#20,#21,#12);
#23=CIRCLE('',#22,2.);
#24=PLANE('',#22);
#25=CARTESIAN_POINT('',(12.,0.,0.));
#26=VERTEX_POINT('',#25);
#27=EDGE_CURVE('',#26,#26,#23,.T.);
#30=CARTESIAN_POINT('',(0.,-10.,0.));
#31=DIRECTION('',(1.,0.,0.));
#32=DIRECTION('',(0.,-1.,0.));
#33=AXIS2_PLACEMENT_3D('',#30,#31,#32);
#34=CIRCLE('',#33,2.);
#35=PLANE('',#33);
#36=CARTESIAN_POINT('',(0.,-12.,0.));
#37=VERTEX_POINT('',#36);
#38=EDGE_CURVE('',#37,#37,#34,.T.);
#40=ORIENTED_EDGE('',*,*,#27,.T.);
#41=EDGE_LOOP('',(#40));
#42=FACE_BOUND('',#41,.T.);
#43=ADVANCED_FACE('',(#42),#24,.T.);
#50=ORIENTED_EDGE('',*,*,#38,.T.);
#51=EDGE_LOOP('',(#50));
#52=FACE_BOUND('',#51,.T.);
#53=ADVANCED_FACE('',(#52),#35,.T.);
#60=ORIENTED_EDGE('',*,*,#27,.F.);
#61=EDGE_LOOP('',(#60));
#62=FACE_BOUND('',#61,.T.);
#63=ORIENTED_EDGE('',*,*,#38,.F.);
#64=EDGE_LOOP('',(#63));
#65=FACE_BOUND('',#64,.T.);
#66=ADVANCED_FACE('',(#62,#65),#14,.T.);
#80=CLOSED_SHELL('',(#43,#53,#66));
#90=MANIFOLD_SOLID_BREP('',#80);
ENDSEC;END-ISO-10303-21;
";

#[test]
fn a_seam_goes_the_long_way_when_the_face_does() {
    let got = import(RING.as_bytes(), &Limits::default()).expect("reads");
    assert!(got.refusals.is_empty(), "{:?}", got.refusals);
    let b = &got.solids[0].body;
    assert_eq!(check(b), vec![]);
    let m = mass::mass_rel(b, 1e-8).unwrap();
    let want = 0.75 * 2.0 * dmath::PI * dmath::PI * 10.0 * 4.0;
    assert!((m.volume - want).abs() < 1e-5 * want, "{} {want}", m.volume);
    // a quarter's worth is what the short way round would hold: the face's box says which
    let bx = b.faces_box();
    assert!(bx.x.lo < -11.0 && bx.y.hi > 11.0, "{bx:?}");
}
