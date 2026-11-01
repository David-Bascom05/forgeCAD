//! A file's small face must not cost a viewer the machine: what a procedural face asks of the mesher
//! is bounded, and past that it says why.
use forge_math::dmath;
use forge_mesh::{Quality, tessellate};
use forge_step::{Limits, import};

/// A body of revolution of two faces joined along two meridians that end on the axis.
fn lens() -> String {
    let generator = [
        (0.0, -0.36),
        (0.18, -0.3),
        (0.32, -0.13),
        (0.32, 0.12),
        (0.18, 0.29),
        (0.0, 0.35),
    ];
    let mut out = String::from(
        "ISO-10303-21;HEADER;ENDSEC;DATA;
#1=(LENGTH_UNIT()NAMED_UNIT(*)SI_UNIT(.MILLI.,.METRE.));
#2=(NAMED_UNIT(*)PLANE_ANGLE_UNIT()SI_UNIT($,.RADIAN.));
#3=(NAMED_UNIT(*)SI_UNIT($,.STERADIAN.)SOLID_ANGLE_UNIT());
#4=UNCERTAINTY_MEASURE_WITH_UNIT(LENGTH_MEASURE(0.005),#1,'distance_accuracy_value','');
#5=(GEOMETRIC_REPRESENTATION_CONTEXT(3)GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT((#4))GLOBAL_UNIT_ASSIGNED_CONTEXT((#1,#2,#3))REPRESENTATION_CONTEXT('',''));
#6=ADVANCED_BREP_SHAPE_REPRESENTATION('',(#90),#5);
#7=CARTESIAN_POINT('',(0.,0.,0.));
#8=DIRECTION('',(0.,0.,1.));
#9=AXIS1_PLACEMENT('',#7,#8);
#10=CARTESIAN_POINT('',(0.,0.,-0.36));
#11=CARTESIAN_POINT('',(0.,0.,0.35));
#12=VERTEX_POINT('',#10);
#13=VERTEX_POINT('',#11);
",
    );
    // the surface's own generator, then a meridian a third of a turn round either way
    let curve = |id: u32, angle: f64| {
        let (s, c) = dmath::sin_cos(angle);
        let mut t = String::new();
        for (k, (r, z)) in generator.iter().enumerate() {
            t += &format!(
                "#{}=CARTESIAN_POINT('',({},{},{}));\n",
                id + 1 + k as u32,
                r * c,
                r * s,
                z
            );
        }
        t + &format!(
            "#{id}=B_SPLINE_CURVE_WITH_KNOTS('',5,(#{},#{},#{},#{},#{},#{}),.UNSPECIFIED.,.F.,.U.,(6,6),(1.9,2.8),.UNSPECIFIED.);\n",
            id + 1,
            id + 2,
            id + 3,
            id + 4,
            id + 5,
            id + 6
        )
    };
    let third = dmath::TAU / 3.0;
    out += &curve(20, 0.0);
    out += &curve(30, third);
    out += &curve(40, 2.0 * third);
    out += "#50=SURFACE_OF_REVOLUTION('',#20,#9);
#60=EDGE_CURVE('',#12,#13,#30,.T.);
#61=EDGE_CURVE('',#12,#13,#40,.T.);
#70=ORIENTED_EDGE('',*,*,#60,.T.);
#71=ORIENTED_EDGE('',*,*,#61,.F.);
#72=EDGE_LOOP('',(#70,#71));
#73=FACE_OUTER_BOUND('',#72,.T.);
#74=ADVANCED_FACE('',(#73),#50,.T.);
#75=ORIENTED_EDGE('',*,*,#61,.T.);
#76=ORIENTED_EDGE('',*,*,#60,.F.);
#77=EDGE_LOOP('',(#75,#76));
#78=FACE_OUTER_BOUND('',#77,.T.);
#79=ADVANCED_FACE('',(#78),#50,.T.);
#80=CLOSED_SHELL('',(#74,#79));
#90=MANIFOLD_SOLID_BREP('',#80);
ENDSEC;END-ISO-10303-21;
";
    out
}

#[test]
fn a_face_reaching_its_axis_meshes_or_says_why() {
    let got = import(lens().as_bytes(), &Limits::default()).expect("reads");
    assert!(got.refusals.is_empty(), "{:?}", got.refusals);
    let body = &got.solids[0].body;
    // a chord that asks for millions of grid points: before the bound this took gigabytes
    let chord = 1e-8 * body.faces_box().diagonal();
    if let Ok(m) = tessellate(
        body,
        &Quality {
            chord,
            ..Quality::default()
        },
    ) {
        assert!(m.positions.len() < 1 << 20, "{}", m.positions.len());
    }
}
