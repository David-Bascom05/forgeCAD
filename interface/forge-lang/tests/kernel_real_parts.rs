//! Real parts that once broke the kernel, as Forge parts: drillings that broke a boolean, drafted
//! faces and lofts, screw faces (coils, threads) meeting other surfaces, and shells whose walls
//! meet as they thicken. Each builds valid feature by feature (as the app does) and in one pass.
mod common;
mod kernel_util;

use kernel_util::{built, joined};

fn quality() -> forge_mesh::Quality {
    forge_mesh::Quality {
        chord: 0.002,
        max_angle: 0.05,
    }
}

/// Builds valid feature by feature and in one pass, on one thread and on two, the same solid by
/// its mesh; that mesh's volume.
fn builds_valid(src: &str) -> f64 {
    let b = built(src);
    let p = forge_topo::check(&b.body);
    assert!(p.is_empty(), "{p:?}");
    let v = forge_mesh::tessellate(&b.body, &quality())
        .expect("meshes")
        .volume();
    for threads in [1, 2] {
        let n = joined(&b.nary(threads).expect("regenerates in one pass"));
        let p = forge_topo::check(&n);
        assert!(p.is_empty(), "{p:?}");
        let w = forge_mesh::tessellate(&n, &quality()).expect("meshes").volume();
        assert!((v - w).abs() <= 1e-6 * v, "{v} vs {w}");
    }
    v
}

/// `builds_valid`'s part's exact volume.
fn exact_volume(src: &str) -> f64 {
    builds_valid(src);
    built(src).volume()
}

// ---- drillings (were forge-agent/tests/drilled_parts.rs)

const BLOCK40: &str = "sketch base(on: xy) {
  r = rect(corner: (-20, -20), size: (40, 40))
}
block = extrude(base, 40)
sketch top(on: block.end) {
  p = pt(0, 0)
}
down = hole(at: top.p, d: 8, depth: thru)
sketch side(on: offset(yz, -20)) {
  p = pt(0, 20)
}
";

/// Two equal through holes whose axes cross (a + drilling): the bores meet in two ellipses
/// crossing where they touch. Removed: two bores less the Steinmetz solid 16 r^3 / 3 they share.
#[test]
fn equal_cross_holes() {
    let v = builds_valid(&format!(
        "{BLOCK40}across = hole(at: side.p, d: 8, depth: thru, flip: true)\n"
    ));
    let want = 64000.0 - (2.0 * forge_math::dmath::PI * 16.0 * 40.0 - 16.0 * 64.0 / 3.0);
    assert!((v - want).abs() < 1e-3 * (64000.0 - want), "{v} vs {want}");
    // a T, ending at the other's axis
    builds_valid(&format!(
        "{BLOCK40}across = hole(at: side.p, d: 8, depth: 20, flip: true)\n"
    ));
}

const BLOCK55: &str = "sketch base(on: xy) {
  r = rect(corner: (-45, -35), size: (90, 70))
}
block = extrude(base, 55)
";

/// A cross hole whose lowest line is tangent to another hole's flat floor: the bore curve
/// touches the floor circle from inside the bore's wall, a vertex on both.
#[test]
fn cross_drill_tangent_to_floor() {
    builds_valid(&format!(
        "{BLOCK55}sketch top(on: block.end) {{
  p = pt(0, 0)
}}
down = hole(at: top.p, d: 7.5, depth: 30)
sketch side(on: offset(xz, -35)) {{
  p = pt(0, 30)
}}
across = hole(at: side.p, d: 10, depth: 35, flip: true)
"
    ));
}

/// The same with drill points, the cross hole drilled to a plane through the first's axis.
#[test]
fn cross_drill_tangent_to_floor_pointed() {
    builds_valid(&format!(
        "{BLOCK55}sketch top(on: block.end) {{
  p = pt(0, -12)
}}
down = hole(at: top.p, d: 8, depth: 35, point: 118deg)
yp = workplane(on: offset(xz, 12))
sketch side(on: offset(xz, 35)) {{
  p = pt(0, 25)
}}
across = hole(at: side.p, d: 10, depth: upto(yp, 0), point: 118deg)
"
    ));
}

/// A blind hole whose rim circle just touches a chamfer's top boundary line (the curve touching
/// an edge from inside, which gets a vertex on both since the touch vertices went in).
#[test]
fn hole_tangent_to_chamfer_edge() {
    builds_valid(&format!(
        "{BLOCK55}bevel = chamfer(edges(of: block), 1.5)
sketch top(on: block.end) {{
  p = pt(-36, -27)
}}
pit = hole(at: top.p, d: 15, depth: 9)
"
    ));
}

/// Two blocks apart in one body: two lumps, a shell each (the union once kept one shell for
/// both, and check counted its Euler characteristic as 4).
#[test]
fn disjoint_lumps_are_shells_apart() {
    let src = "sketch a(on: xy) {
  r = rect(corner: (0, 0), size: (10, 10))
}
one = extrude(a, 10)
sketch b(on: a.plane) {
  r = rect(corner: (20, 0), size: (10, 10))
}
other = extrude(b, 10)
";
    let v = builds_valid(src);
    assert!((v - 2000.0).abs() < 1e-6 * 2000.0, "{v}");
    assert_eq!(built(src).body.shells.len(), 2);
}

/// A pocket whose mouth stands 2e-5 (two tolerances) above the top face, and a pad laid 2e-5
/// under the top across it: the pocket's wall keeps a sliver two tolerances tall under the pad.
/// The n-ary regen without shapes probed that sliver from a point a step outside it and kept it,
/// leaving open edges; every point of it is within tolerance of a face, so it is refused, never
/// built invalid.
#[test]
fn sliver_two_tolerances_tall() {
    let b = built(
        "sketch base(on: xy) {
  r = rect(corner: (0, 0), size: (60, 40))
  pit = pt(15.77764088281603, 16.262082660203596)
  pad = pt(15.14935454448211, 11.64952639287997)
}
block = extrude(base.r, 10)
sketch pk(on: offset(base.plane, 7.00001), at: base.pit) {
  c = circle((0, 0), 3)
}
pocket = cut(pk, -3.00001)
sketch pd(on: offset(base.plane, 9.99998), at: base.pad) {
  r = rect(corner: (0, 0), size: (10, 8))
}
padded = extrude(pd, 3)
",
    );
    for shapes in [true, false] {
        if let Ok(n) = b.part().regenerate_nary_with(1, shapes) {
            assert!(forge_topo::check(&n).is_empty(), "{:?}", forge_topo::check(&n));
        }
    }
}

// ---- drafted faces and lofts (were forge-agent/tests/real_parts.rs)

/// Builds valid in sequence and in n-ary booleans on two threads.
fn valid_both_ways(src: &str) {
    let b = built(src);
    let p = forge_topo::check(&b.body);
    assert!(p.is_empty(), "seq: {p:?}");
    let n = joined(&b.nary(2).expect("n-ary regenerates"));
    let p = forge_topo::check(&n);
    assert!(p.is_empty(), "n-ary: {p:?}");
}

const BOX: &str = "sketch base(on: xy) {
  r = rect(corner: (-120, -65), size: (240, 130))
}
box = extrude(base, 95)
";

/// A modeled thread in a boss on a 2 degree wall: the tilted wall meets the thread's faces
/// where their tracks turn, finer than the region's tracing step.
#[test]
fn thread_in_boss_on_drafted_wall() {
    valid_both_ways(&format!(
        "{BOX}tilt = draft(box.side(base.r[1]), angle: -2deg, neutral: box.end)
sketch bs(on: offset(yz, 114)) {{
  c = circle((0, 22), 14)
}}
boss = extrude(bs, 12)
sketch hs(on: offset(yz, 126)) {{
  p = pt(0, 22)
}}
tap = hole(at: hs.p, d: 14.4, depth: 16, thread: (16, 1.5))
tapped = thread(hole: tap, length: 12)
"
    ));
}

/// A drafted rib whose side runs through the vertex blend at a foot's corner: the cut crosses
/// the blend's rims where it meets the foot's top and the wall tangentially. (The half-round
/// boss is two quarter arcs and the foot is drawn where the part's patterned copy stood: a single
/// half arc names its joints alike, and a fillet after a pattern misses its edges n-ary, two
/// separate bugs.)
#[test]
fn drafted_rib_into_blend() {
    valid_both_ways(&format!(
        "{BOX}sketch hb(on: offset(xz, -87)) {{
  top = line((20, 95), (104, 95))
  a = arc(center: (62, 95), from: (104, 95), to: (62, 53), cw: true)
  b = arc(center: (62, 95), from: (62, 53), to: (20, 95), cw: true)
}}
boss = extrude(hb, 44)
sketch fs(on: base.plane) {{
  path o(start: (52, -55)) {{
    line(to: (52, -85))
    tarc(to: (60, -93))
    line(to: (88, -93))
    tarc(to: (96, -85))
    line(to: (96, -55))
    close()
  }}
}}
feet = extrude(fs, 16)
foot = fillet(edges(between: (feet, box), crease: \"concave\"), 5)
sketch rs(on: offset(yz, 62)) {{
  l = line((63, 20), (83, 49))
}}
web = rib(rs.l, 8, draft: 1deg)
"
    ));
}

/// A tapered loft arm into a boss, filleted round the junction: the ball rolls on the loft's
/// face, offset along its normal like any other surface.
#[test]
fn fillet_loft_into_boss() {
    valid_both_ways(
        "sketch p0(on: offset(yz, 12)) {
  o = hull(disc((-9, -6), 6), disc((9, -6), 6), disc((9, 6), 6), disc((-9, 6), 6))
}
sketch p1(on: offset(yz, 166)) {
  o = hull(disc((-5, -3.5), 4), disc((5, -3.5), 4), disc((5, 3.5), 4), disc((-5, 3.5), 4))
}
arm = loft([p0.o, p1.o])
sketch ps(on: xy) {
  c = circle((170, 0), 12)
}
pedal = extrude(ps, sym(18))
foot = fillet(edges(between: (arm, pedal), crease: \"concave\"), 1.2)
",
    );
}

const TUBE: &str = "sketch s0(on: xy) {
  c = circle((0, 0), 10)
}
sketch s1(on: offset(xy, 30)) {
  c = circle((0, 5), 14)
}
sketch s2(on: offset(xy, 60)) {
  c = circle((0, 15), 18)
}
tube = loft([s0.c, s1.c, s2.c], smooth: true)
";

/// A boss standing on a smooth three-profile loft, filleted round its foot.
#[test]
fn fillet_boss_into_smooth_loft() {
    valid_both_ways(&format!(
        "{TUBE}sketch bs(on: yz) {{
  c = circle((5, 30), 4)
}}
boss = extrude(bs, 25)
foot = fillet(edges(between: (boss, tube), crease: \"concave\"), 2)
"
    ));
}

/// An internal thread in a spindle boss with a smooth loft arm just outside it: the thread's
/// faces meet the loft's region but not the loft (R13).
#[test]
fn thread_near_smooth_loft() {
    valid_both_ways(
        "sketch p0(on: offset(yz, 12)) {
  o = hull(disc((-9, -6), 6), disc((9, -6), 6), disc((9, 6), 6), disc((-9, 6), 6))
}
sketch p1(on: offset(yz, 85)) {
  o = hull(disc((-6.5, -4.5), 5), disc((6.5, -4.5), 5), disc((6.5, 4.5), 5), disc((-6.5, 4.5), 5))
}
sketch p2(on: offset(yz, 162)) {
  o = hull(disc((-5, -3.5), 4), disc((5, -3.5), 4), disc((5, 3.5), 4), disc((-5, 3.5), 4))
}
arm = loft([p0.o, p1.o, p2.o], smooth: true)
sketch ss(on: xy) {
  c = circle((0, 0), 21)
}
spindle = extrude(ss, sym(30))
sketch top(on: offset(xy, 15)) {
  p = pt(0, 0)
}
bore = hole(at: top.p, d: 20.9, depth: 7)
puller = thread(on: top.plane, at: top.p, spec: \"M22x1\", internal: true, length: 6)
",
    );
}

/// A cut whose floor and wall both cross a smooth spiral loft (a volute's chamber, R17).
#[test]
fn cut_across_smooth_spiral_loft() {
    valid_both_ways(
        "# the spiral's sections stand square to its path round the axis: at angle a, the plane
# through the radius there (x out along it, normal along the turn), centred r out
sketch dirs(on: xy) {
  for k in 0..7 {
    out = line((0, 0), (10mm * cos(60deg + k * 50deg), 10mm * sin(60deg + k * 50deg)), construction: true)
    turn = line((0, 0), (-10mm * sin(60deg + k * 50deg), 10mm * cos(60deg + k * 50deg)), construction: true)
  }
  e7 = pt(103, 55)
  e8 = pt(103, 110)
}
shared rs = [87.17, 89.81, 92.44, 95.08, 97.72, 100.4, 103]
shared cr = [16.67, 18.89, 21.11, 23.33, 25.56, 27.78, 30]
for k in 0..7 {
  sketch sec(on: plane(at: dirs.out[k].a, normal: dirs.turn[k].dir, x: dirs.out[k].dir)) {
    c = circle((rs[k], 0), cr[k])
  }
}
sketch exit7(on: plane(at: dirs.e7, normal: Y, x: X)) {
  c = circle((0, 0), 33.5)
}
sketch exit8(on: plane(at: dirs.e8, normal: Y, x: X)) {
  c = circle((0, 0), 37)
}
volute = loft([sec[0].c, sec[1].c, sec[2].c, sec[3].c, sec[4].c, sec[5].c, sec[6].c, exit7.c, exit8.c], smooth: true)
sketch ch(on: offset(xy, -15)) {
  c = circle((0, 0), 78)
}
chamber = cut(ch, -37)
",
    );
}

/// A cylinder extruded on from a smooth loft's end circle: the loft meets the cylinder along
/// that circle, touching it where the loft stands straight up (R15).
#[test]
fn extrude_from_smooth_loft_end() {
    valid_both_ways(&format!("{TUBE}on = extrude(s2.c, 40)\n"));
}

/// A fillet where a half-round boss meets a 2 degree drafted wall: the springs are ellipses'
/// feet, and their speed bound held a whole turn's box, which holds the boss's axis (R5).
#[test]
fn fillet_boss_on_drafted_wall() {
    valid_both_ways(&format!(
        "{BOX}tilt = draft(box.side(base.r[2]), angle: -2deg, neutral: box.end)
sketch hb(on: offset(xz, -87)) {{
  top = line((-105.5, 95), (-38.5, 95))
  a = arc(center: (-72, 95), from: (-38.5, 95), to: (-72, 61.5), cw: true)
  b = arc(center: (-72, 95), from: (-72, 61.5), to: (-105.5, 95), cw: true)
}}
boss = extrude(hb, 36)
foot = fillet(edges(between: (boss, box), crease: \"concave\"), 2)
"
    ));
}

/// A half-round boss across a flange, over a drafted and filleted corner: the corner's
/// ellipse rims on the top face stood across it as their own plane, so the boss's edge never
/// split there (R1).
#[test]
fn boss_through_drafted_fillet() {
    valid_both_ways(&format!(
        "{BOX}dr = draft(box.side(base.r[0]) + box.side(base.r[1]) + box.side(base.r[2]) + box.side(base.r[3]), angle: -2deg, neutral: box.end)
corners = fillet(edges(of: box, shorter: 100), 12)
sketch fl(on: box.end) {{
  r = rect(corner: (-142, -87), size: (284, 174))
}}
rim = extrude(fl, -14)
sketch hb(on: xz) {{
  top = line((-111.5, 95), (-44.5, 95))
  a = arc(center: (-78, 95), from: (-44.5, 95), to: (-78, 61.5), cw: true)
  b = arc(center: (-78, 95), from: (-78, 61.5), to: (-111.5, 95), cw: true)
}}
boss = extrude(hb, sym(174))
"
    ));
}

/// A shell of a plain loft between offset circles (an oblique cone, in two tangent halves):
/// the halves' seam vertices were pinned by the halves only once (R16).
#[test]
fn shell_oblique_loft() {
    valid_both_ways(
        "sketch s0(on: xy) {
  c = circle((0, 0), 10)
}
sketch s1(on: offset(xy, 30)) {
  c = circle((0, 5), 14)
}
cone = loft([s0.c, s1.c])
cup = shell(2, open: cone.end)
",
    );
}

// ---- screw faces (were forge-agent/tests/screw_parts.rs)

/// A spring round its guide rod, coaxial, not touching: the coil's round section never reaches
/// the rod's wall or end planes, which its box over the coil's turns shows. The two volumes add.
#[test]
fn coil_round_rod() {
    let v = builds_valid(
        "sketch base(on: xy) {
  c = circle((0, 0), 6)
}
rod = extrude(base, 30)
sketch spr(on: xz) {
  c = circle((9, 5), 1.2)
  ax = line((0, 5), (0, 10), construction: true)
}
spring = coil(spr.c, axis: spr.ax, pitch: 6, turns: 2)
",
    );
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
        "# points: the origin, ends of the tube's, the boss's and the hole's axes, the boss's foot
# and the hole's mouth
sketch o(on: xy) {
  p = pt(0, 0)
}
sketch tz(on: offset(xy, 0.8)) {
  p = pt(0.6, 0)
}
sketch bz(on: offset(xy, 0.6)) {
  p = pt(-0.8, 0)
}
sketch hz(on: offset(xy, -0.6)) {
  p = pt(0.8, 0)
}
sketch bo(on: offset(xy, 120)) {
  p = pt(90, 0)
}
sketch ho(on: offset(xy, 132.6)) {
  p = pt(73.2, 0)
}
sketch tube(on: plane(at: o.p, normal: axis(o.p, tz.p))) {
  c = circle((0, 0), 17.5)
}
outer = extrude(tube, 300)
sketch boss(on: plane(at: bo.p, normal: axis(o.p, bz.p))) {
  c = circle((0, 0), 4.5)
}
bossed = extrude(boss, 21)
sketch bores(on: tube.plane) {
  c = circle((0, 0), 16.5)
}
bored = cut(bores, -300)
sketch mouth(on: plane(at: ho.p, normal: axis(o.p, hz.p))) {
  p = pt(0, 0)
}
bh = hole(at: mouth.p, d: 4.2, depth: 6, thread: (5, 0.8), flip: true)
tapped = thread(hole: bh, length: \"full\")
",
    );
}

/// An internal BSA thread in a bottom bracket shell a chainstay runs into on an oblique axis.
#[test]
fn bsa_thread_with_chainstay() {
    builds_valid(
        "sketch bbs(on: xz) {
  c = circle((0, 0), 21)
}
bb = extrude(bbs, sym(68))
sketch o(on: xy) {
  p = pt(0, 0)
  foot = pt(-12, -22)
}
sketch dz(on: offset(xy, 0.17)) {
  p = pt(-0.98, -0.09)
}
sketch cs(on: plane(at: o.foot, normal: axis(o.p, dz.p))) {
  c = circle((0, 0), 11)
}
stay = extrude(cs, 300)
sketch face(on: offset(xz, 34)) {
  p = pt(0, 0)
}
bore = hole(at: face.p, d: 33.65, depth: 68)
# BSA 1.370-24: 1.370 in across, 24 threads an inch
bsa = thread(on: face.plane, at: face.p, spec: (34.798, 1.0583333333333333), internal: true, length: 16, left: true, chamfer: true)
",
    );
}

/// A tapped block shelled open at the top: hollowed as if the thread were not there, the wall
/// measured from just past its major radius, the thread whole in the skin (the flanks have no
/// offset where the hole cuts them: this refused). What it took is the closed form: the box less
/// the bore's wall, the bore's ring r = 4 + 1.25 / 8 through.
#[test]
fn shell_of_a_tapped_block() {
    let block = "sketch base(on: xy) {
  r = rect(corner: (-20, -15), size: (40, 30))
}
block = extrude(base, 20)
sketch top(on: block.end) {
  p = pt(0, 0)
}
tap = hole(at: top.p, d: 6.8, depth: thru, thread: (8, 1.25))
tapped = thread(hole: tap, length: \"full\")
";
    let whole = exact_volume(block);
    let pi = forge_math::dmath::PI;
    for t in [1.0, 0.5] {
        let v = exact_volume(&format!("{block}cup = shell({t}, open: block.end)\n"));
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
    let rod = "sketch base(on: xy) {
  c = circle((0, 0), 6)
}
rod = extrude(base, 30)
m12 = thread(on: rod.end, spec: \"M12\", length: 20)
";
    let whole = exact_volume(rod);
    let pi = forge_math::dmath::PI;
    let p = 1.75;
    let inner = 6.0 - 5.0 / 16.0 * 3f64.sqrt() * p - p / 8.0;
    let step = 10.0 - p / 4.0;
    for t in [1.0, 0.5] {
        let v = exact_volume(&format!("{rod}cup = shell({t}, open: rod.start)\n"));
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
    let part = "sketch base(on: xy) {
  c = circle((0, 0), 6)
}
rod = extrude(base, 30)
sketch top(on: rod.end) {
  c = circle((0, 0), 11)
}
collar = extrude(top, 10)
m12 = thread(on: top.plane, spec: \"M12\", length: 20)
";
    let whole = exact_volume(part);
    let p = 1.75;
    let minor = 6.0 - 5.0 / 16.0 * 3f64.sqrt() * p;
    let (lo, hi) = (minor - p / 8.0, 6.0 + p / 16.0 + p + p / 8.0);
    let (s, top) = (10.0 - p / 4.0, 30.0 + 1.5 * p);
    for t in [1.0, 0.5] {
        let v = exact_volume(&format!("{part}cup = shell({t}, open: rod.start)\n"));
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

// ---- shells whose walls meet (were forge-agent/tests/shell_events.rs)

/// The part's exact volume; it must build valid, and the same in one pass.
fn shelled_volume(src: &str) -> f64 {
    let b = built(src);
    let p = forge_topo::check(&b.body);
    assert!(p.is_empty(), "{p:?}");
    let v = b.volume();
    let n = joined(&b.nary(1).expect("regenerates in one pass"));
    let w = forge_topo::mass::mass(&n).expect("a volume").volume;
    assert!((v - w).abs() <= 1e-9 * v, "{v} vs {w} in one pass");
    v
}

const PLATE: &str = "sketch base(on: xy) {
  r = rect(corner: (0, 0), size: (40, 30))
}
plate = extrude(base, 10)
";

/// A boss 1.6 by 3 on a plate, shelled open below: thicker than half its width, its walls meet and
/// the wall runs on flat across under it (the plate's cavity alone); thinner, the cavity reaches up
/// into it, its footprint less the wall all round (this refused: the boss's walls crossed).
#[test]
fn a_boss_the_walls_close_over() {
    let part = format!(
        "{PLATE}sketch top(on: plate.end) {{
  r = rect(corner: (15, 10), size: (1.6, 3))
}}
boss = extrude(top, 2)
"
    );
    let whole = 40.0 * 30.0 * 10.0 + 1.6 * 3.0 * 2.0;
    for (t, lobe) in [
        (1.0, 0.0),
        (0.9, 0.0),
        (0.5, 0.6 * 2.0 * 2.0),
        (0.3, 1.0 * 2.4 * 2.0),
    ] {
        let v = shelled_volume(&format!("{part}cup = shell({t}, open: plate.start)\n"));
        let cavity = (40.0 - 2.0 * t) * (30.0 - 2.0 * t) * (10.0 - t) + lobe;
        assert!(
            (whole - v - cavity).abs() < 1e-9 * whole,
            "t {t}: {} vs {cavity}",
            whole - v
        );
    }
}

/// A triangular boss, the walls meeting at its incentre: right-angled 3, 4, 5, its inradius 1.
#[test]
fn a_triangle_closed_over_past_its_inradius() {
    let part = format!(
        "{PLATE}sketch top(on: plate.end) {{
  t = poly([(10, 10), (13, 10), (10, 14)])
}}
boss = extrude(top, 2)
"
    );
    let whole = 40.0 * 30.0 * 10.0 + 6.0 * 2.0;
    for (t, lobe) in [(1.2, 0.0), (0.5, 6.0 * 0.25 * 2.0)] {
        // the footprint less t all round: the same triangle scaled about the incentre by (1 - t / r)
        let v = shelled_volume(&format!("{part}cup = shell({t}, open: plate.start)\n"));
        let cavity = (40.0 - 2.0 * t) * (30.0 - 2.0 * t) * (10.0 - t) + lobe;
        assert!(
            (whole - v - cavity).abs() < 1e-9 * whole,
            "t {t}: {} vs {cavity}",
            whole - v
        );
    }
}
