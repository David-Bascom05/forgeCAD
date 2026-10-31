//! Part files whose screw faces (coils, threads) once met another surface with no closed form:
//! each builds valid feature by feature and in one pass, on one thread and on two, the same solid.
use forge_agent::model::Model;
use forge_model::Part;

fn builds_valid(src: &str) -> f64 {
    let m = Model::new(src);
    assert!(m.error.is_none(), "{:?}", m.error.as_ref().map(|e| &e.msg));
    let seq = m.body().expect("a body");
    assert!(forge_topo::check(seq).is_empty(), "{:?}", forge_topo::check(seq));
    let part = Part {
        features: m.features.clone(),
    };
    let q = forge_mesh::Quality {
        chord: 0.002,
        max_angle: 0.05,
    };
    let v = forge_mesh::tessellate(seq, &q).expect("meshes").volume();
    for threads in [1, 2] {
        let b = part.regenerate_nary(threads).expect("regenerates in one pass");
        assert!(forge_topo::check(&b).is_empty(), "{:?}", forge_topo::check(&b));
        let w = forge_mesh::tessellate(&b, &q).expect("meshes").volume();
        assert!((v - w).abs() <= 1e-6 * v, "{v} vs {w}");
    }
    v
}

/// A spring round its guide rod, coaxial, not touching: the coil's round section never reaches
/// the rod's wall or end planes, which its box over the coil's turns shows. The two volumes add.
#[test]
fn coil_round_rod() {
    let v =
        builds_valid("extrude 30 at 0 0 0 circle 0 0 6\ncoil xz at 0 0 5 circle 9 0 1.2 pitch 6 turns 2\n");
    let pi = forge_math::dmath::PI;
    // the wire: its section's area along its centre's helix, 2 turns of radius 9 rising 6 each
    let wire = pi * 1.44 * 2.0 * ((2.0 * pi * 9.0) * (2.0 * pi * 9.0) + 36.0).sqrt();
    let want = pi * 36.0 * 30.0 + wire;
    assert!((v - want).abs() < 2e-3 * want, "{v} vs {want}");
}

/// An M5 tapped boss into a tube whose axis is no world axis: the thread's faces meet the tube's
/// walls at a general angle, swept section by section.
#[test]
fn thread_boss_into_oblique_tube() {
    builds_valid(
        "extrude 300 z 0.6 0 0.8 circle 0 0 17.5\nextrude 21 z -0.8 0 0.6 at 90 0 120 circle 0 0 4.5\nextrude cut 300 z 0.6 0 0.8 circle 0 0 16.5\nbh: hole 4.2 z 0.8 0 -0.6 at 73.2 0 132.6 6 thread 5 0.8\nthread hole bh full\n",
    );
}

/// An internal BSA thread in a bottom bracket shell a chainstay runs into on an oblique axis.
#[test]
fn bsa_thread_with_chainstay() {
    builds_valid(
        "extrude sym 68 -xz circle 0 0 21\nextrude 300 z -0.98 -0.09 0.17 at -12 -22 0 circle 0 0 11\nhole 33.65 -xz at 0 -34 0 68\nthread 1.370-24 -xz at 0 -34 0 int 16 left chamfer\n",
    );
}
