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

/// `builds_valid`'s part's exact volume.
fn volume(src: &str) -> f64 {
    builds_valid(src);
    let m = Model::new(src);
    forge_topo::mass::mass(m.body().expect("a body"))
        .expect("a volume")
        .volume
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

/// A tapped block shelled open at the top: hollowed as if the thread were not there, the wall
/// measured from just past its major radius, the thread whole in the skin (the flanks have no
/// offset where the hole cuts them: this refused). What it took is the closed form: the box less
/// the bore's wall, the bore's ring r = 4 + 1.25 / 8 through.
#[test]
fn shell_of_a_tapped_block() {
    let block = "extrude 20 rect -20 -15 40 30\ntap: hole 6.8 -xy at 0 0 20 thru thread 8 1.25\nthread hole tap full\n";
    let whole = volume(block);
    let pi = forge_math::dmath::PI;
    for t in [1.0, 0.5] {
        let v = volume(&format!("{block}shell {t} (faces normal +z top)\n"));
        let r = 4.0 + 1.25 / 8.0 + t;
        let cavity = (40.0 - 2.0 * t) * (30.0 - 2.0 * t) * (20.0 - t) - pi * r * r * (20.0 - t);
        assert!(
            (whole - v - cavity).abs() < 1e-6 * cavity,
            "t {t}: {} vs {cavity}",
            whole - v
        );
    }
}

/// A rod threaded M12 its top 20, shelled open at the bottom: the cavity steps in where the thread
/// starts, its wall measured from the thread's minor radius less P / 8; the ring ends P / 4 past
/// the thread.
#[test]
fn shell_of_a_threaded_rod() {
    let rod = "extrude 30 circle 0 0 6\nthread M12 -xy at 0 0 30 ext 20\n";
    let whole = volume(rod);
    let pi = forge_math::dmath::PI;
    let p = 1.75;
    let inner = 6.0 - 5.0 / 16.0 * 3f64.sqrt() * p - p / 8.0;
    let step = 10.0 - p / 4.0;
    for t in [1.0, 0.5] {
        let v = volume(&format!("{rod}shell {t} (faces normal -z bottom)\n"));
        let cavity = pi * (6.0 - t) * (6.0 - t) * (step - t) + pi * (inner - t) * (inner - t) * (30.0 - step);
        assert!(
            (whole - v - cavity).abs() < 1e-6 * cavity,
            "t {t}: {} vs {cavity}",
            whole - v
        );
    }
}

/// A rod threaded from a collar it stands under: the groove's first turn, from a pitch back, cuts
/// into the collar's underside out to where its section runs on square, so the ring takes that too
/// (from the minor less P / 8 to the major plus P / 16, a pitch and P / 8, up 1.5 P into the collar).
/// The cavity is the ring-notched section's own inward offset, revolved.
#[test]
fn shell_of_a_rod_threaded_from_a_collar() {
    let part =
        "extrude 30 circle 0 0 6\nextrude 10 at 0 0 30 circle 0 0 11\nthread M12 -xy at 0 0 30 ext 20\n";
    let whole = volume(part);
    let p = 1.75;
    let minor = 6.0 - 5.0 / 16.0 * 3f64.sqrt() * p;
    let (lo, hi) = (minor - p / 8.0, 6.0 + p / 16.0 + p + p / 8.0);
    let (s, top) = (10.0 - p / 4.0, 30.0 + 1.5 * p);
    for t in [1.0, 0.5] {
        let v = volume(&format!("{part}shell {t} (faces normal -z bottom)\n"));
        let rz = [
            (0.0, 0.0),
            (6.0 - t, 0.0),
            (6.0 - t, s - t),
            (lo - t, s - t),
            (lo - t, top + t),
            (hi + t, top + t),
            (hi + t, 30.0 + t),
            (11.0 - t, 30.0 + t),
            (11.0 - t, 40.0 - t),
            (0.0, 40.0 - t),
        ];
        // a section's volume of revolution, Pappus edge by edge
        let mut cavity = 0.0;
        for k in 0..rz.len() {
            let ((r0, z0), (r1, z1)) = (rz[k], rz[(k + 1) % rz.len()]);
            cavity += (z1 - z0) * (r0 * r0 + r0 * r1 + r1 * r1);
        }
        let cavity = (forge_math::dmath::PI / 3.0 * cavity).abs();
        assert!(
            (whole - v - cavity).abs() < 1e-6 * cavity,
            "t {t}: {} vs {cavity}",
            whole - v
        );
    }
}
