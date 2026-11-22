//! Kernel failures the real-part benchmarks (tools/bench-parts) ran into, each shrunk to the
//! features that fail and written as a Forge part. Ignored until fixed: each builds valid when
//! its failure is gone. The bench parts carry a TODO naming the test where they work round it.
mod common;

use forge_model::Part;

/// The kernel's refusal of the part, as the kernel names it.
fn kernel_refusal(src: &str) -> String {
    let (r, _) = common::run(src);
    r.errors
        .iter()
        .find_map(|e| match e {
            forge_lang::error::Error::Build { error, .. } => Some(format!("{error:?}")),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no kernel refusal: {}", forge_lang::report::text(&r, src)))
}

/// Seq and n-ary give the same number of faces.
fn same_faces(src: &str) {
    let faces = |bs: Vec<forge_topo::Body>| bs.iter().map(|b| b.face_ids().count()).sum::<usize>();
    let (seq, nary) = common::regens(src);
    assert_eq!(seq.map(faces), nary.map(faces));
}

/// `holds`, n-ary too, and seq and n-ary give the same number of faces.
fn holds_same_faces(src: &str) {
    common::holds_nary(src);
    same_faces(src);
}

/// The part's history as the kernel takes it: its features and their ids.
fn history(src: &str) -> (Part, Vec<u64>) {
    let (r, cache) = common::run(src);
    assert!(r.errors.is_empty(), "{}", forge_lang::report::text(&r, src));
    let (f, ids) = cache.history();
    (Part { features: f.to_vec() }, ids.to_vec())
}

/// The finished part.
fn body(src: &str) -> forge_topo::Body {
    let (r, _) = common::run(src);
    assert!(r.errors.is_empty(), "{}", forge_lang::report::text(&r, src));
    r.body.expect("a part")
}

/// seq step: boolean Internal(face split crossed itself); half-disc boss crossing drafted+filleted
/// corner under a flange; ok without flange, without fillet, or boss r=25
#[test]
fn boss_through_drafted_fillet() {
    common::holds(
        r#"
XY = plane(at: xy.origin, normal: xy.normal, x: yz.normal)
XZ = plane(at: xy.origin, normal: xz.normal, x: yz.normal)
sketch foot(on: xy) {
  r = rect(corner: (-120, -65), size: (240, 130))
}
box = extrude(foot, 95)
dr = draft(box.side(foot.r), angle: -2deg, neutral: offset(foot.plane, 95))
round = fillet(edges(of: box, shorter: 100), 12)
sketch fl(on: offset(XY, 95)) {
  r = rect(corner: (-142, -87), size: (284, 174))
}
flg = extrude(fl, -14)
sketch hb(on: XZ) {
  a = pt(-111.5, 95)
  b = pt(-44.5, 95)
  c = pt(-78, 95)
  l = line(a, b)
  q = arc(center: c, from: b, to: a, cw: true)
}
boss = extrude(hb, sym(174))
"#,
    );
}

/// fillet Overflow; concave fillet r3 on a foot pad whose side is 0.7mm from the tangent line of a
/// r12 fillet on drafted corner; ok when the pad is 14mm further
#[test]
fn fillet_runs_onto_drafted_corner_fillet() {
    common::holds(
        r#"
sketch foot(on: xy) {
  r = rect(corner: (-120, -65), size: (240, 130))
}
box = extrude(foot, 95)
dr = draft(box.side(foot.r[3]) + box.side(foot.r[0]), angle: -2deg, neutral: offset(foot.plane, 95))
round = fillet(edges(between: (box.side(foot.r[0]), box.side(foot.r[3]))), 12)
sketch pd(on: foot.plane) {
  r = rect(corner: (-104, -93), size: (44, 40))
}
pad = extrude(pd, 16)
root = fillet(edges(of: pad, crease: "concave"), 3)
"#,
    );
}

/// rib draft 1 whose end runs into a foot's corner fillet blend: Rib(Boolean(Unsupported(surface
/// pair without a closed form))); ok without draft
#[test]
fn drafted_rib_into_blend() {
    common::holds(
        r#"
XZ = plane(at: xy.origin, normal: xz.normal, x: yz.normal)
YZ = plane(at: xy.origin, normal: yz.normal, x: -xz.normal)
sketch foot(on: xy) {
  r = rect(corner: (-120, -65), size: (240, 130))
}
box = extrude(foot, 95)
sketch hb(on: offset(XZ, -87)) {
  a = pt(20, 95)
  b = pt(104, 95)
  c = pt(62, 95)
  l = line(a, b)
  q = arc(center: c, from: b, to: a, cw: true)
}
boss = extrude(hb, 44)
sketch fs(on: foot.plane) {
  path o(start: (-96, -55)) {
    line(to: (-96, -85))
    tarc(to: (-88, -93))
    line(to: (-60, -93))
    tarc(to: (-52, -85))
    line(to: (-52, -55))
    close()
  }
}
feet = extrude(fs.o, 16)
feetp = pattern(of: feet, layout: lin(along: foot.r[0].dir, n: count(2), step: 148, along2: foot.r[1].dir, n2: count(2), step2: 146))
root = fillet(edges(between: ([feet, feetp], box), crease: "concave"), 5)
sketch rs(on: offset(YZ, 62)) {
  l = line((63, 20), (83, 49))
}
web = rib(rs.l, 8, draft: 1deg)
"#,
    );
}

/// modeled thread (thread hole, 12 deep) in a blind hole in a boss standing on a 2deg drafted face:
/// boolean Unsupported(surface pair without a closed form); same without the draft builds
#[test]
fn thread_in_boss_on_drafted_wall() {
    common::holds(
        r#"
YZ = plane(at: xy.origin, normal: yz.normal, x: -xz.normal)
sketch foot(on: xy) {
  r = rect(corner: (-120, -65), size: (240, 130))
}
box = extrude(foot, 95)
dr = draft(box.side(foot.r[1]), angle: -2deg, neutral: offset(foot.plane, 95))
sketch bs(on: offset(YZ, 114)) {
  c = circle((0, 22), 14)
}
boss = extrude(bs, 12)
sketch hs(on: offset(YZ, 126)) {
  p = pt(0, 22)
}
hole1 = hole(at: hs.p, d: 14.4, depth: 16, thread: (16, 1.5))
tap = thread(hole: hole1, length: 12)
"#,
    );
}

/// fillet 2 on the curve where a half-round boss meets a 2deg drafted wall: Overflow; same without
/// draft builds
#[test]
fn fillet_boss_on_drafted_wall() {
    common::holds(
        r#"
XZ = plane(at: xy.origin, normal: xz.normal, x: yz.normal)
sketch foot(on: xy) {
  r = rect(corner: (-120, -65), size: (240, 130))
}
box = extrude(foot, 95)
dr = draft(box.side(foot.r[2]), angle: -2deg, neutral: offset(foot.plane, 95))
sketch hb(on: offset(XZ, -87)) {
  a = pt(-105.5, 95)
  b = pt(-38.5, 95)
  c = pt(-72, 95)
  l = line(a, b)
  q = arc(center: c, from: b, to: a, cw: true)
}
boss = extrude(hb, 36)
root = fillet(edges(between: (boss, box), crease: "concave"), 2)
"#,
    );
}

/// half-disc boss, concentric bore cut through, chamfer 1 the bore's semicircle edge: chamfer
/// Invalid(DuplicateName x3); a rect block with the same half-bore chamfers fine
#[test]
fn chamfer_half_ring_bore() {
    common::holds(
        r#"
sketch hb(on: offset(xz, -87)) {
  a = pt(-105.5, 95)
  b = pt(-38.5, 95)
  c = pt(-72, 95)
  l = line(a, b)
  q = arc(center: c, from: b, to: a, cw: true)
}
boss = extrude(hb, 36)
sketch bs(on: offset(hb.plane, 87)) {
  c = circle((-72, 95), 23.5)
}
bore = cut(bs.c, thru_all)
edge = chamfer(edges(between: (bore, boss.start)), 1)
"#,
    );
}

/// extrude of a loop of a line and one 180deg arc is invalid: DuplicateName x3 (check); a 90deg arc
/// or two 90deg arcs are valid. Root of R6 and of gearbox invalid stages
#[test]
fn half_disc_extrude() {
    common::holds(
        r#"
sketch s(on: xy) {
  a = pt(-33, 0)
  b = pt(33, 0)
  c = pt(0, 0)
  l = line(a, b)
  q = arc(center: c, from: b, to: a, cw: true)
}
e = extrude(s, 36)
"#,
    );
}

/// two overlapping half-round bosses, a revolve-cut ring groove in one that reaches the overlap:
/// DuplicateName (check); either boss alone is valid
#[test]
fn groove_across_overlapping_bosses() {
    common::holds(
        r#"
YZ = plane(at: xy.origin, normal: yz.normal, x: -xz.normal)
sketch b1(on: offset(xz, -87)) {
  a = pt(-49, 95)
  b = pt(33, 95)
  c = pt(-8, 95)
  m = pt(-8, 54)
  l = line(a, b)
  q1 = arc(center: c, from: b, to: m, cw: true)
  q2 = arc(center: c, from: m, to: a, cw: true)
}
boss1 = extrude(b1, 36)
sketch b2(on: b1.plane) {
  a = pt(20, 95)
  b = pt(104, 95)
  c = pt(62, 95)
  m = pt(62, 53)
  l = line(a, b)
  q1 = arc(center: c, from: b, to: m, cw: true)
  q2 = arc(center: c, from: m, to: a, cw: true)
}
boss2 = extrude(b2, 36)
sketch g(on: offset(YZ, 62)) {
  ax = line((0, 95), (10, 95), construction: true)
  r = rect(corner: (79, 128), size: (2.2, 3.4))
}
groove = revolve(g.r, axis: g.ax, cut: true)
"#,
    );
}

/// an M22x1 internal thread in a bore of a spindle boss, a smooth loft arm starting 1mm outside the
/// thread's major radius (no contact): boolean Unsupported(surface pair without a closed form); the
/// same with a plain loft builds
#[test]
fn thread_near_smooth_loft() {
    common::holds(
        r#"
YZ = plane(at: xy.origin, normal: yz.normal, x: -xz.normal)
XY = plane(at: xy.origin, normal: xy.normal, x: yz.normal)
fn rrect(a, b, r) {
  return hull(disc((a - r, r - b), r), disc((a - r, b - r), r), disc((r - a, b - r), r), disc((r - a, r - b), r))
}
sketch s1(on: offset(YZ, 12)) {
  o = rrect(15, 12, 6)
}
sketch s2(on: offset(YZ, 85)) {
  o = rrect(11.5, 9.5, 5)
}
sketch s3(on: offset(YZ, 162)) {
  o = rrect(9, 7.5, 4)
}
arm = loft([s1, s2, s3], smooth: true)
sketch sp(on: XY) {
  c = circle((0, 0), 21)
}
spindle = extrude(sp, sym(30))
sketch top(on: spindle.end) {
  c = pt(0, 0)
}
puller = hole(at: top.c, d: 20.9, depth: 7)
tap = thread(spec: "M22x1", on: spindle.end, at: top.c, internal: true, length: 6)
"#,
    );
}

/// a two-profile tapered loft arm into a cylindrical boss, fillet 1.2 (also 0.5) round the
/// junction: fillet NoBlend; an extruded arm of the same section fillets fine
#[test]
fn fillet_loft_into_boss() {
    common::holds(
        r#"
YZ = plane(at: xy.origin, normal: yz.normal, x: -xz.normal)
XY = plane(at: xy.origin, normal: xy.normal, x: yz.normal)
fn rrect(a, b, r) {
  return hull(disc((a - r, r - b), r), disc((a - r, b - r), r), disc((r - a, b - r), r), disc((r - a, r - b), r))
}
sketch s1(on: offset(YZ, 12)) {
  o = rrect(15, 12, 6)
}
sketch s3(on: offset(YZ, 166)) {
  o = rrect(9, 7.5, 4)
}
arm = loft([s1, s3])
sketch pd(on: XY) {
  c = circle((170, 0), 12)
}
pedal = extrude(pd, sym(18))
root = fillet(edges(between: (arm, pedal), crease: "concave"), 1.2)
"#,
    );
}

/// smooth loft through three circles, then a cylinder extruded on from its end circle: boolean
/// Unsupported(surface pair without a closed form); the same off a plain extrude is fine
#[test]
fn extrude_from_smooth_loft_end() {
    common::holds(
        r#"
XY = plane(at: xy.origin, normal: xy.normal, x: yz.normal)
sketch sa(on: XY) {
  c = circle((0, 0), 10)
}
sketch sb(on: offset(XY, 30)) {
  c = circle((0, 5), 14)
}
sketch sc(on: offset(XY, 60)) {
  c = circle((0, 15), 18)
}
tube = loft([sa, sb, sc], smooth: true)
sketch d(on: offset(XY, 60)) {
  c = circle((0, 15), 18)
}
ext = extrude(d, 40)
"#,
    );
}

/// shell 2 of a plain loft between two circles whose centres are offset (an oblique cone), top
/// open: shell Rebuild(Edge); a coaxial loft shells fine. So no shelled volute
#[test]
fn shell_oblique_loft() {
    common::holds(
        r#"
XY = plane(at: xy.origin, normal: xy.normal, x: yz.normal)
sketch sa(on: XY) {
  c = circle((0, 0), 10)
}
sketch sb(on: offset(XY, 30)) {
  c = circle((0, 5), 14)
}
tube = loft([sa, sb])
cup = shell(2, open: tube.end)
"#,
    );
}

/// smooth spiral loft, then extrude cut 37 at z=-15 of a circle r78 whose bottom plane and wall
/// both cross the loft: boolean Unsupported(surface pair without a closed form); the same cut sym
/// 80 (no plane crossing) or r84 builds
#[test]
fn cut_cylinder_plane_corner_on_smooth_loft() {
    common::holds(
        r#"
shared ang = [60deg, 110deg, 160deg, 210deg, 260deg, 310deg, 360deg]
shared rc = [87.17, 89.81, 92.44, 95.08, 97.72, 100.4, 103]
shared ro = [16.67, 18.89, 21.11, 23.33, 25.56, 27.78, 30]
shared n = count(7)
shared dx = 103
shared dy = [55, 110]
shared dro = [33.5, 37]
shared tick = 10
sketch datum(on: xy) {
  for i in 0..n {
    c = pt(rc[i] * cos(ang[i]), rc[i] * sin(ang[i]))
    radial = line(c, ((rc[i] + tick) * cos(ang[i]), (rc[i] + tick) * sin(ang[i])), construction: true)
  }
  for k in 0..2 {
    diff = line((dx, dy[k]), (dx + tick, dy[k]), construction: true)
  }
}
for i in 0..n {
  sketch vo(on: rotate(datum.plane, about: datum.radial[i], by: -90deg)) {
    c = circle((0, 0), ro[i])
  }
}
for k in 0..2 {
  sketch dob(on: rotate(datum.plane, about: datum.diff[k], by: -90deg)) {
    c = circle((0, 0), dro[k])
  }
}
volute = loft([vo[i].c for i in 0..n] + [dob[k].c for k in 0..2], smooth: true)
sketch chs(on: offset(datum.plane, -15)) {
  c = circle((0, 0), 78)
}
chamber = cut(chs.c, -37)
"#,
    );
}

/// a flat pad standing into a smooth three-circle loft, fillet 2 round the junction: fillet
/// NoBlend; a plain loft too. Same family as R14: no fillet holds on any loft face
#[test]
#[ignore = "R18: at r = 2 the blend reaches the loft's end rim and would roll over it: refused as Corner, not supported"]
fn fillet_pad_into_smooth_loft() {
    common::holds_nary(&pad_into_smooth_loft(2.0));
}

fn pad_into_smooth_loft(r: f64) -> String {
    r#"
XY = plane(at: xy.origin, normal: xy.normal, x: yz.normal)
YZ = plane(at: xy.origin, normal: yz.normal, x: -xz.normal)
sketch sa(on: XY) {
  c = circle((0, 0), 10)
}
sketch sb(on: offset(XY, 30)) {
  c = circle((0, 5), 14)
}
sketch sc(on: offset(XY, 60)) {
  c = circle((0, 15), 18)
}
tube = loft([sa, sb, sc], smooth: true)
sketch ps(on: offset(YZ, 8)) {
  r = rect(corner: (-30, -10), size: (60, 80))
}
pad = extrude(ps, 5)
root = fillet(edges(between: (pad, tube), crease: "concave"), R)
"#
    .replace('R', &r.to_string())
}

/// R18 at a radius whose blend stays on its faces.
#[test]
fn fillet_pad_into_smooth_loft_small() {
    // its corners are two blends beside a sharp edge: a mitre, whose spring here runs past the
    // pad's back face. Refused until that is followed; a patch would be a shape nobody asked for
    let msg = kernel_refusal(&pad_into_smooth_loft(0.5));
    assert!(msg.contains("Overflow"), "{msg}");
}

/// a 28.6 top tube into a revolved tapered head tube (cone), fillet 4..2.5 round the joint: builds
/// but check fails EdgeOffFace 0.0014; constant fillet 3 is valid; the same with axes at a round
/// angle did not reproduce
#[test]
fn variable_fillet_tube_into_cone() {
    common::holds(
        r#"
XZ = plane(at: xy.origin, normal: xz.normal, x: yz.normal)
shared reach = 385
shared stack = 575
shared hta = 73deg
shared sta = 73.5deg
shared htl = 160
shared stl = 540
shared hdx = cos(hta)
shared hdz = 0 - sin(hta)
shared s1x = 0 - (stl - 35) * cos(sta)
shared s1z = (stl - 35) * sin(sta)
shared h1x = reach + 25mm * hdx
shared h1z = stack + 25mm * hdz
shared ttl = sqrt((h1x - s1x) ^ 2 + (h1z - s1z) ^ 2)
sketch hs(on: XZ) {
  ax = line((reach, stack), (reach + 10mm * hdx, stack + 10mm * hdz), construction: true)
  o = poly([(reach, stack), (reach + 25mm * sin(hta), stack + 25mm * cos(hta)), (reach + htl * hdx + 30mm * sin(hta), stack - htl * sin(hta) + 30mm * cos(hta)), (reach + htl * hdx, stack - htl * sin(hta))])
  tl = line((s1x, s1z), (h1x, h1z), construction: true)
}
ht = revolve(hs.o, axis: hs.ax)
sketch ts(on: plane(at: hs.tl.a, normal: hs.tl.dir)) {
  c = circle((0, 0), 14.3)
}
tt = extrude(ts, ttl)
joint = fillet(edges(between: (tt, ht), crease: "concave"), vary(4, 2.5))
"#,
    );
}

/// BB shell, a chainstay cylinder (oblique axis) running into it, the shell bored to the BSA minor,
/// thread 1.370-24 int 16 left: boolean Unsupported(surface pair without a closed form); without
/// the stay it builds. The stay's surface no longer reaches the bore once it is cut, so this is the
/// helix paired with a face it never meets (as R13, R4)
#[test]
fn bsa_thread_with_chainstay() {
    common::holds(
        r#"
XZ = plane(at: xy.origin, normal: xz.normal, x: yz.normal)
sketch bbs(on: XZ) {
  c = circle((0, 0), 21)
}
bb = extrude(bbs, sym(68))
sketch p0(on: offset(XZ, 22)) {
  p = pt(-12, 0)
}
sketch p1(on: offset(XZ, 22.9)) {
  p = pt(-21.8, 1.7)
}
sketch cs(on: plane(at: p0.p, normal: axis(p0.p, p1.p))) {
  c = circle((0, 0), 11)
}
stay = extrude(cs, 300)
sketch hs(on: offset(XZ, 34)) {
  p = pt(0, 0)
}
bore = hole(at: hs.p, d: 33.65, depth: 68)
tap = thread(spec: (1.37 * 25.4, 25.4 / 24), on: hs.plane, at: hs.p, internal: true, length: 16, left: true, chamfer: true)
"#,
    );
}

/// a bottle boss on a tube whose axis is not a world axis, M5 tapped through into the bore, thread
/// hole full: boolean Unsupported(surface pair without a closed form); the same tube along x builds
/// (helix vs a cylinder at a general angle has no closed form)
#[test]
fn thread_boss_into_oblique_tube() {
    common::holds(
        r#"
XZ = plane(at: xy.origin, normal: xz.normal, x: yz.normal)
sketch d(on: XZ) {
  t = line((0, 0), (6, 8), construction: true)
  b = line((90, 120), (82, 126), construction: true)
}
sketch ts(on: plane(at: d.t.a, normal: d.t.dir)) {
  c = circle((0, 0), 17.5)
}
tube = extrude(ts, 300)
sketch bs(on: plane(at: d.b.a, normal: d.b.dir)) {
  c = circle((0, 0), 4.5)
}
boss = extrude(bs, 21)
sketch cs(on: ts.plane) {
  c = circle((0, 0), 16.5)
}
bore = cut(cs, -300)
sketch hs(on: boss.end) {
  p = pt(0, 0)
}
bh = hole(at: hs.p, d: 4.2, depth: 6, thread: (5, 0.8))
tap = thread(hole: bh, length: "full")
"#,
    );
}

/// a tapered arm outline (two lines tangent to a 180.7deg tip arc, square root end) extruded,
/// fillet 2 all four top edges: fillet Corner{vertex 1}; any subset (root edge alone, arc alone,
/// sides alone, arc+sides) builds, and the same with a rrect or a plain taper poly builds
#[test]
fn fillet_top_tapered_arm_tip() {
    common::holds(
        r#"
sketch s(on: xy) {
  a = pt(0, -15)
  b = pt(169.905858, -15.999723)
  c = pt(170, 0)
  d = pt(169.905858, 15.999723)
  e = pt(0, 15)
  l1 = line(a, b)
  q = arc(center: c, from: b, to: d)
  l2 = line(d, e)
  l3 = line(e, a)
}
arm = extrude(s, 14)
round = fillet(edges(of: arm.end), 2)
"#,
    );
}

/// a bar, fillet 4..3 on two top edges, shell 1.6 open below: shell Collapses on the variable blend
/// (its radius never under 3); fillet 3 constant shells fine
#[test]
fn shell_after_variable_fillet() {
    common::holds(
        r#"
sketch s(on: xy) {
  r = rect(corner: (0, -15), size: (170, 30))
}
bar = extrude(s, 14)
round = fillet(edges(of: bar.end, along: X), vary(4, 3))
cup = shell(1.6, open: bar.start)
"#,
    );
}

/// A disc (axis y) and a block flush with both its faces (a conrod bolt boss), the concave lines
/// where the block's side leaves the disc. A ball of radius r between them touches the side at
/// height sqrt(285 + 114 r); the side stops at 22, so up to r = 1.745 the fillet fits and past it
/// the refusal is the right answer.
#[test]
fn fillet_block_on_cylinder_flush() {
    let part = |r: f64| {
        r#"
XZ = plane(at: xy.origin, normal: xz.normal, x: yz.normal)
sketch bg(on: XZ) {
  c = circle((0, 0), 31)
}
big = extrude(bg, sym(22))
sketch bk(on: XZ) {
  r = rect(corner: (26, -24), size: (16, 46))
}
b = extrude(bk, sym(22))
root = fillet(edges(between: (b, big), crease: "concave"), R)
"#
        .replace('R', &r.to_string())
    };
    common::holds_nary(&part(1.7));
    assert!(kernel_refusal(&part(1.8)).contains("Overflow"));
}

/// a rod r6 and a coil (wire r1.2 on radius 9) round it, coaxial, not touching: boolean
/// Unsupported(surface pair without a closed form); a coil whose end sinks into a disc builds. A
/// spring over its guide rod cannot be modelled
#[test]
fn coil_round_rod() {
    common::holds(
        r#"
XZ = plane(at: xy.origin, normal: xz.normal, x: yz.normal)
sketch r(on: xy) {
  c = circle((0, 0), 6)
}
rod = extrude(r, 30)
sketch cs(on: XZ) {
  c = circle((9, 5), 1.2)
  ax = line((0, 5), (0, 15), construction: true)
}
spring = coil(cs.c, axis: cs.ax, pitch: 6, turns: 2)
"#,
    );
}

/// a disc with a block flush to both its faces, the disc bored through, chamfer 1 the bore's rim
/// circles: chamfer Rebuild(Edge); the disc alone chamfers fine (the unmerged coplanar end faces
/// again, as R24)
#[test]
fn chamfer_bore_past_flush_block() {
    common::holds(
        r#"
XZ = plane(at: xy.origin, normal: xz.normal, x: yz.normal)
XZn = plane(at: xy.origin, normal: -xz.normal, x: yz.normal)
sketch bg(on: XZ) {
  c = circle((0, 0), 31)
}
big = extrude(bg, sym(22))
sketch bk(on: XZ) {
  r = rect(corner: (20, -10), size: (30, 20))
}
bx = extrude(bk, sym(22))
sketch hs(on: offset(XZn, 11)) {
  p = pt(0, 0)
}
bigbore = hole(at: hs.p, d: 48, depth: thru)
edge = chamfer(edges(of: bigbore, shape: "circle"), 1)
"#,
    );
}

/// seq builds valid; n-ary fails Chamfer NoSuchEdge on the bore-rim chamfer after a fillet of the
/// shank into the big end (without the bolt-boss block it passes)
#[test]
fn nary_chamfer_after_fillet_conrod() {
    common::holds_nary(
        r#"
XZ = plane(at: xy.origin, normal: xz.normal, x: yz.normal)
XZn = plane(at: xy.origin, normal: -xz.normal, x: yz.normal)
sketch side(on: XZ) {
  o = poly([(-14, 25), (-8, 140), (8, 140), (14, 25)])
}
shank = extrude(side, sym(20))
sketch bg(on: XZ) {
  c = circle((0, 0), 31)
}
big = extrude(bg, sym(22))
sketch bk(on: XZ) {
  r = rect(corner: (26, -24), size: (16, 46))
}
bolts = extrude(bk, sym(22))
root = fillet(edges(between: (shank, big), crease: "concave", shape: "line"), 6)
sketch hs(on: offset(XZn, 11)) {
  p = pt(0, 0)
}
bigbore = hole(at: hs.p, d: 48, depth: thru)
edge = chamfer(edges(of: bigbore, shape: "circle"), 1)
"#,
    );
}

/// a crank spider (arm on a rotated frame, patterned 5 round the hub), fillet 3 where the arms meet
/// the hub: seq valid; n-ary fails MissingEdge on the fillet
#[test]
fn nary_fillet_after_circular_pattern() {
    common::holds_nary(SPIDER);
    // named by position, as the regression was found
    let (part, _) = history(SPIDER);
    let names = |b: forge_topo::Body| {
        let mut n: Vec<_> = b.face_ids().map(|f| b.face(f).name).collect();
        n.sort();
        n
    };
    assert_eq!(part.regenerate().map(names), part.regenerate_nary(1).map(names));
}

/// `nary_fillet_after_circular_pattern` named by the Forge part's ids. Seq and n-ary named the
/// spindle's bottom (joined across the arms' flush feet) apart: the pattern's pass split it
/// again and named a region as the first pass had named a piece, recorded as its own whole, and
/// the join took that, the lesser, for the original. A piece is no whole of its own now.
#[test]
fn nary_fillet_after_circular_pattern_by_ids() {
    common::holds_alike(SPIDER);
}

const SPIDER: &str = r#"
sketch sp(on: xy) {
  c = circle((0, 0), 21)
  arm1 = line((0, 0), (10mm * cos(36deg), 10mm * sin(36deg)), construction: true)
}
spindle = extrude(sp.c, sym(30))
sketch spk(on: plane(at: spindle.start.plane.origin, normal: sp.plane.normal, x: sp.arm1.dir)) {
  outline = hull(disc((6, -1.5), 6), disc((56, -1.5), 6), disc((56, 1.5), 6), disc((6, 1.5), 6))
}
spider = extrude(spk.outline, 6)
spiders = pattern(of: spider, layout: circ(about: spindle.axis, n: count(5)))
root = fillet(edges(between: ([spider, spiders], spindle), crease: "concave"), 3)
"#;

/// a block, two separate foot pads, one fillet 5 where both pads meet the block: seq valid; n-ary
/// fails MissingEdge on the fillet (one pad per fillet line passes; a lin pattern of one pad fails
/// the same). Root of R29 and of the gearbox and frame n-ary failures
#[test]
fn nary_fillet_after_linear_pattern() {
    common::holds_alike(
        r#"
sketch foot(on: xy) {
  r = rect(corner: (-120, -65), size: (240, 130))
}
box = extrude(foot, 95)
sketch pa(on: foot.plane) {
  r = rect(corner: (-96, -93), size: (44, 34))
}
a = extrude(pa, 16)
sketch pb(on: foot.plane) {
  r = rect(corner: (52, -93), size: (44, 34))
}
b = extrude(pb, 16)
root = fillet(edges(between: (a, box), crease: "concave") + edges(between: (b, box), crease: "concave"), 5)
"#,
    );
}

/// a three-profile plain loft arm into a pedal boss, the boss bored and threaded 9/16-20 left: seq
/// valid; n-ary fails Boolean Unsupported(surface pair without a closed form) (n-ary twin of R13;
/// two profiles pass both ways)
#[test]
fn nary_thread_near_three_profile_loft() {
    holds_same_faces(
        r#"
YZ = plane(at: xy.origin, normal: yz.normal, x: -xz.normal)
XY = plane(at: xy.origin, normal: xy.normal, x: yz.normal)
fn rrect(a, b, r) {
  return hull(disc((a - r, r - b), r), disc((a - r, b - r), r), disc((r - a, b - r), r), disc((r - a, r - b), r))
}
sketch s1(on: offset(YZ, 12)) {
  o = rrect(15, 12, 6)
}
sketch s2(on: offset(YZ, 85)) {
  o = rrect(11.5, 9.5, 5)
}
sketch s3(on: offset(YZ, 166)) {
  o = rrect(9, 7.5, 4)
}
arm = loft([s1, s2, s3])
sketch pd(on: XY) {
  c = circle((170, 0), 12)
}
pedal = extrude(pd, sym(18))
sketch pe(on: pedal.end) {
  c = pt(170, 0)
}
bore = hole(at: pe.c, d: 12.913, depth: thru)
tap = thread(spec: "9/16-20", on: pedal.end, at: pe.c, internal: true, length: 18, left: true, chamfer: true)
"#,
    );
}

/// smooth spiral loft volute, a cover-flange disc through it, the smooth core loft cut, then one
/// discharge-flange bolt hole far from the rest: seq valid; n-ary fails Boolean Unsupported(surface
/// pair without a closed form) at the hole (without the disc or without the core it passes)
#[test]
fn nary_volute_core_and_flange_hole() {
    common::holds_nary(
        r#"
shared ang = [60deg, 110deg, 160deg, 210deg, 260deg, 310deg, 360deg]
shared rc = [87.17, 89.81, 92.44, 95.08, 97.72, 100.4, 103]
shared ro = [16.67, 18.89, 21.11, 23.33, 25.56, 27.78, 30]
shared ri = [11.67, 13.89, 16.11, 18.33, 20.56, 22.78, 25]
shared n = count(7)
shared dx = 103
shared dy = [55, 110]
shared dro = [33.5, 37]
shared dri = [28.5, 32]
shared dyc = 125
shared drc = 33
shared tick = 10
shared fl = 14
sketch datum(on: xy) {
  for i in 0..n {
    c = pt(rc[i] * cos(ang[i]), rc[i] * sin(ang[i]))
    radial = line(c, ((rc[i] + tick) * cos(ang[i]), (rc[i] + tick) * sin(ang[i])), construction: true)
  }
  for k in 0..2 {
    diff = line((dx, dy[k]), (dx + tick, dy[k]), construction: true)
  }
  core_end = line((dx, dyc), (dx + tick, dyc), construction: true)
  flange_face = line((dx, dy[1] - fl), (dx + tick, dy[1] - fl), construction: true)
}
for i in 0..n {
  sketch vo(on: rotate(datum.plane, about: datum.radial[i], by: -90deg)) {
    c = circle((0, 0), ro[i])
  }
  sketch vi(on: rotate(datum.plane, about: datum.radial[i], by: -90deg)) {
    c = circle((0, 0), ri[i])
  }
}
for k in 0..2 {
  sketch dob(on: rotate(datum.plane, about: datum.diff[k], by: -90deg)) {
    c = circle((0, 0), dro[k])
  }
  sketch dib(on: rotate(datum.plane, about: datum.diff[k], by: -90deg)) {
    c = circle((0, 0), dri[k])
  }
}
sketch dic(on: rotate(datum.plane, about: datum.core_end, by: -90deg)) {
  c = circle((0, 0), drc)
}
volute = loft([vo[i].c for i in 0..n] + [dob[k].c for k in 0..2], smooth: true)
sketch cfs(on: offset(datum.plane, 10)) {
  c = circle((0, 0), 104)
}
cflange = extrude(cfs.c, 12)
core = loft([vi[i].c for i in 0..n] + [dib[k].c for k in 0..2] + [dic.c], smooth: true, cut: true)
sketch bolt(on: rotate(datum.plane, about: datum.flange_face, by: 90deg)) {
  p = pt(45, 0)
}
dbolt = hole(at: bolt.p, d: 14, depth: thru)
"#,
    );
}

/// a block, two separate foot pads, fillet 5 the first pad, then fillet 5 the second: the second
/// fails Overflow (seq); each alone, or pad-fillet-pad-fillet interleaved, builds
#[test]
fn second_pad_fillet_overflow() {
    common::holds(
        r#"
sketch foot(on: xy) {
  r = rect(corner: (-120, -65), size: (240, 130))
}
box = extrude(foot, 95)
sketch pa(on: foot.plane) {
  r = rect(corner: (-96, -93), size: (44, 34))
}
a = extrude(pa, 16)
sketch pb(on: foot.plane) {
  r = rect(corner: (52, -93), size: (44, 34))
}
b = extrude(pb, 16)
ra = fillet(edges(between: (a, box), crease: "concave"), 5)
rb = fillet(edges(between: (b, box), crease: "concave"), 5)
"#,
    );
}

/// bottom bracket shell, seat tube and down tube from its axis, one fillet 3 over all three joints:
/// seq valid; n-ary fails MissingEdge (any of the joints filleted alone fails Overflow seq, they
/// cross)
#[test]
fn nary_bb_cluster_fillet() {
    common::holds_alike(
        r#"
XZ = plane(at: xy.origin, normal: xz.normal, x: yz.normal)
shared reach = 385
shared stack = 575
shared hta = 73deg
shared sta = 73.5deg
shared htl = 160
shared stl = 540
shared hdx = cos(hta)
shared hdz = 0 - sin(hta)
shared hbx = reach + htl * hdx
shared hbz = stack + htl * hdz
shared dtx = hbx - 45mm * hdx
shared dtz = hbz - 45mm * hdz
shared dtl = sqrt(dtx ^ 2 + dtz ^ 2)
sketch g(on: XZ) {
  c = circle((0, 0), 21)
  sl = line((0, 0), (0 - 10mm * cos(sta), 10mm * sin(sta)), construction: true)
  dl = line((0, 0), (dtx, dtz), construction: true)
}
bb = extrude(g.c, sym(68))
sketch sts(on: plane(at: g.sl.a, normal: g.sl.dir)) {
  c = circle((0, 0), 15.9)
}
st = extrude(sts, stl)
sketch dts(on: plane(at: g.dl.a, normal: g.dl.dir)) {
  c = circle((0, 0), 17.5)
}
dt = extrude(dts, dtl)
joints = fillet(edges(between: (st, bb), crease: "concave") + edges(between: (dt, bb), crease: "concave") + edges(between: (st, dt), crease: "concave"), 3)
"#,
    );
}

/// down tube, tapered head tube (revolved cone), top tube, fillet 3 where the top tube meets the
/// head tube: seq valid; n-ary fails MissingEdge (without the down tube it passes)
#[test]
fn nary_fillet_top_tube_head_tube() {
    common::holds_alike(
        r#"
XZ = plane(at: xy.origin, normal: xz.normal, x: yz.normal)
shared reach = 385
shared stack = 575
shared hta = 73deg
shared sta = 73.5deg
shared htl = 160
shared stl = 540
shared hdx = cos(hta)
shared hdz = 0 - sin(hta)
shared hbx = reach + htl * hdx
shared hbz = stack + htl * hdz
shared dtx = hbx - 45mm * hdx
shared dtz = hbz - 45mm * hdz
shared dtl = sqrt(dtx ^ 2 + dtz ^ 2)
shared s1x = 0 - (stl - 45) * cos(sta)
shared s1z = (stl - 45) * sin(sta)
shared h1x = reach + 25mm * hdx
shared h1z = stack + 25mm * hdz
shared ttl = sqrt((h1x - s1x) ^ 2 + (h1z - s1z) ^ 2)
sketch hs(on: XZ) {
  dl = line((0, 0), (dtx, dtz), construction: true)
  ax = line((reach, stack), (reach + 10mm * hdx, stack + 10mm * hdz), construction: true)
  o = poly([(reach, stack), (reach + 25mm * sin(hta), stack + 25mm * cos(hta)), (reach + htl * hdx + 30mm * sin(hta), stack - htl * sin(hta) + 30mm * cos(hta)), (reach + htl * hdx, stack - htl * sin(hta))])
  tl = line((s1x, s1z), (h1x, h1z), construction: true)
}
sketch dts(on: plane(at: hs.dl.a, normal: hs.dl.dir)) {
  c = circle((0, 0), 17.5)
}
dt = extrude(dts, dtl)
ht = revolve(hs.o, axis: hs.ax)
sketch ts(on: plane(at: hs.tl.a, normal: hs.tl.dir)) {
  c = circle((0, 0), 14.3)
}
tt = extrude(ts, ttl)
joint = fillet(edges(between: (tt, ht), crease: "concave"), 3)
"#,
    );
}

/// a disc and a block flush with both its faces: seq leaves 12 faces (each end face split in two),
/// n-ary 10 (merged); the two paths give different topology for the same part. In the benchmarks:
/// gearbox 359 vs 338, crank 145 vs 144, conrod 158 vs 154 faces. Likely the same root as R24, R26
/// and union-coplanar-faces-unmerged
#[test]
fn seq_nary_face_count_differ() {
    same_faces(
        r#"
XZ = plane(at: xy.origin, normal: xz.normal, x: yz.normal)
sketch bg(on: XZ) {
  c = circle((0, 0), 31)
}
big = extrude(bg, sym(22))
sketch bk(on: XZ) {
  r = rect(corner: (26, -24), size: (16, 46))
}
b = extrude(bk, sym(22))
"#,
    );
}

/// seat tube, top tube, fillet 0:3,0.5:4.5,1:3 at their joint, two loft cuts (stay bores, one
/// missing the part), the pair mirrored: the mirror leaves an invalid body, EdgeUses (seq; build
/// says ok, check fails); with a constant fillet, or mirroring only the cut that hits, it is valid
#[test]
fn mirror_cuts_after_variable_fillet() {
    common::holds(
        r#"
XZ = plane(at: xy.origin, normal: xz.normal, x: yz.normal)
shared reach = 385
shared stack = 575
shared hta = 73deg
shared sta = 73.5deg
shared stl = 540
shared csl = 410
shared drop = 70
shared sp = 130
shared hdx = cos(hta)
shared hdz = 0 - sin(hta)
shared ax = 0 - sqrt(csl ^ 2 - drop ^ 2)
shared az = drop
shared s1x = 0 - (stl - 45) * cos(sta)
shared s1z = (stl - 45) * sin(sta)
shared h1x = reach + 25mm * hdx
shared h1z = stack + 25mm * hdz
shared ttl = sqrt((h1x - s1x) ^ 2 + (h1z - s1z) ^ 2)
shared c0x = -17.5
shared c0y = 22
shared c1x = ax + 6
shared c1y = sp / 2 - 4
shared csx = c1x - c0x
shared csy = c1y - c0y
shared csz = az
shared cl = sqrt(csx ^ 2 + csy ^ 2 + csz ^ 2)
shared q0x = 0 - (stl - 80) * cos(sta)
shared q0z = (stl - 80) * sin(sta)
shared q0y = 10
shared q1x = ax + 10
shared q1z = az + 14
shared q1y = sp / 2 - 4
sketch g(on: XZ) {
  sl = line((0, 0), (0 - 10mm * cos(sta), 10mm * sin(sta)), construction: true)
  tl = line((s1x, s1z), (h1x, h1z), construction: true)
}
sketch sts(on: plane(at: g.sl.a, normal: g.sl.dir)) {
  c = circle((0, 0), 15.9)
}
st = extrude(sts, stl)
sketch tts(on: plane(at: g.tl.a, normal: g.tl.dir)) {
  c = circle((0, 0), 14.3)
}
tt = extrude(tts, ttl)
joint = fillet(edges(between: (tt, st), crease: "concave"), vary([(0, 3), (ratio(0.5), 4.5), (1, 3)]))
sketch c0(on: offset(XZ, 0 - c0y)) {
  p = pt(c0x, 0)
}
sketch c1(on: offset(XZ, 0 - c1y)) {
  p = pt(c1x, az)
}
sketch ca(on: offset(XZ, 0 - (c0y + 12mm * csy / cl))) {
  p = pt(c0x + 12mm * csx / cl, 12mm * csz / cl)
}
sketch cs1(on: plane(at: ca.p, normal: axis(c0.p, c1.p))) {
  c = circle((0, 0), 9.9)
}
sketch cs2(on: plane(at: c1.p, normal: axis(c0.p, c1.p))) {
  c = circle((0, 0), 5.5)
}
csb = loft([cs1, cs2], cut: true)
sketch q0(on: offset(XZ, 0 - q0y)) {
  p = pt(q0x, q0z)
}
sketch q1(on: offset(XZ, 0 - q1y)) {
  p = pt(q1x, q1z)
}
sketch ss1(on: plane(at: q0.p, normal: axis(q0.p, q1.p))) {
  c = circle((0, 0), 7)
}
sketch ss2(on: plane(at: q1.p, normal: axis(q0.p, q1.p))) {
  c = circle((0, 0), 5)
}
ssb = loft([ss1, ss2], cut: true)
stays = pattern(of: [csb, ssb], layout: mirror(on: XZ))
"#,
    );
}

/// boolean Unsupported(surface pair without a closed form): a tube swept along an arc and its mirror, 12
/// apart, never touching; a torus face's box is its whole torus, and torus/torus with parallel
/// axes has no closed form; 20 apart builds
#[test]
fn disjoint_swept_arcs() {
    common::holds(
        r#"
XZ = plane(at: xy.origin, normal: xz.normal, x: yz.normal)
YZ = plane(at: xy.origin, normal: yz.normal, x: -xz.normal)
sketch rail(on: YZ) {
  c = pt(10, -100)
  p = pt(10, 0)
  q = pt(10 + 100mm * cos(50deg), -100 + 100mm * sin(50deg))
  a = arc(center: c, from: p, to: q, cw: true)
}
sketch prof(on: plane(at: rail.p, normal: -xz.normal)) {
  c = circle((0, 0), 4)
}
s = sweep(prof.c, along: rail.a)
sm = pattern(of: s, layout: mirror(on: XZ))
"#,
    );
}

/// HANGS: a tube swept along a line and a 2.4 degree arc in a general plane, bored by a sweep cut of a
/// smaller circle along the same path; the same in a world plane builds in 20 ms
#[test]
fn bent_tube_bore_hangs() {
    common::holds(
        r#"
XZ = plane(at: xy.origin, normal: xz.normal, x: yz.normal)
sketch pc(on: offset(XZ, -160.846988255089)) {
  p = pt(-306.512352962495, 202.598871100508)
}
sketch pn(on: offset(XZ, -160.846988255089)) {
  p = pt(-298.46436203244025, 196.6633979104619)
}
sketch px(on: offset(XZ, -161.9889968684216)) {
  p = pt(-312.4089943915229, 194.60353269712908)
}
sketch rail(on: plane(at: pc.p, normal: axis(pc.p, pn.p), x: axis(pc.p, px.p))) {
  s = pt(-311.5837200301258, -116.02291803024991)
  m = pt(-1.53534808142159, -119.99017754078403)
  e = pt(3.5811763335663773, -119.94655133044844)
  l = line(s, m)
  a = arc(center: (0, 0), from: m, to: e)
}
sketch outer(on: plane(at: rail.s, normal: rail.l.dir)) {
  c = circle((0, 0), 7)
}
tube = sweep(outer.c, along: [rail.l, rail.a])
sketch inner(on: plane(at: rail.s, normal: rail.l.dir)) {
  c = circle((0, 0), 6)
}
bore = sweep(inner.c, along: [rail.l, rail.a], cut: true)
"#,
    );
}

/// fillet NoBlend: facefillet between an upright unioned onto a plate and the plate's top; a plain
/// fillet of the same edge builds. face_fillet compacts the body, then uses the FaceIds it was given
#[test]
fn facefillet_after_union() {
    common::holds(
        r#"
sketch bs(on: xy) {
  r = rect(corner: (-40, -30), size: (80, 60))
}
base = extrude(bs, 10)
sketch us(on: base.end) {
  r = rect(corner: (-40, -30), size: (12, 60))
}
up = extrude(us, 50)
ff = facefillet(up.side(us.r[1]), base.end, 6)
"#,
    );
}

/// fillet Overflow: fullround of a rib unioned onto a plate; the rib alone builds (stale FaceIds, as B3)
#[test]
fn fullround_after_union() {
    common::holds(
        r#"
sketch bs(on: xy) {
  r = rect(corner: (-40, -30), size: (80, 60))
}
base = extrude(bs, 10)
sketch rs(on: base.end) {
  r = rect(corner: (-30, -4), size: (60, 8))
}
rb = extrude(rs, 40)
fr = fullround(side: rb.side(rs.r[0]), middle: rb.end, other: rb.side(rs.r[2]))
"#,
    );
}

/// HANGS: thicken a flange face after a line-arc-line runner swept out of it and bored by a sweep
/// cut; thicken first, or a straight runner, builds
#[test]
fn thicken_after_bent_bore() {
    common::holds(
        r#"
XZ = plane(at: xy.origin, normal: xz.normal, x: yz.normal)
YZ = plane(at: xy.origin, normal: yz.normal, x: -xz.normal)
fn rrect(a, b, r) {
  return hull(disc((a - r, r - b), r), disc((a - r, b - r), r), disc((r - a, b - r), r), disc((r - a, r - b), r))
}
sketch fs(on: offset(YZ, -12)) {
  o = rrect(60, 30, 8)
}
flg = extrude(fs, 12)
sketch rp(on: offset(YZ, -12)) {
  c = circle((0, 0), 19)
}
sketch rail(on: XZ) {
  path r(start: (-12, 0)) {
    line(to: (30, 0))
    tarc(to: (90, -60))
    line(to: (90, -115))
  }
}
runner = sweep(rp.c, along: rail.r)
sketch bp(on: offset(YZ, -12)) {
  c = circle((0, 0), 15.5)
}
sketch rail2(on: XZ) {
  path r(start: (-12, 0)) {
    line(to: (30, 0))
    tarc(to: (90, -60))
    line(to: (90, -130))
  }
}
bore = sweep(bp.c, along: rail2.r, cut: true)
slab = thicken(flg.start, 1.5)
"#,
    );
}

/// edit Rebuild(Vertex): replace the top face of a block whose pocket rim is rounded; without the
/// round it builds
#[test]
fn replace_face_by_round() {
    common::holds_nary(
        r#"
sketch bs(on: xy) {
  r = rect(corner: (-60, -40), size: (120, 80))
}
blk = extrude(bs, 40)
sketch cs(on: blk.end) {
  r = rect(corner: (-40, -25), size: (80, 50))
}
cav = cut(cs, 14)
rim = fillet(edges(between: (cav, blk.end)), 2)
part = workplane(on: offset(bs.plane, 36))
flat = replace_face(blk.end, with: part)
"#,
    );
}

/// edit Rebuild(Vertex): move a pocket floor up 1 after its edges are filleted; before, it builds
#[test]
fn move_face_by_blend() {
    common::holds_nary(
        r#"
fn rrect(a, b, r) {
  return hull(disc((a - r, r - b), r), disc((a - r, b - r), r), disc((r - a, b - r), r), disc((r - a, r - b), r))
}
sketch bs(on: xy) {
  r = rect(corner: (-60, -40), size: (120, 80))
}
blk = extrude(bs, 40)
sketch cs(on: blk.end) {
  o = rrect(40, 25, 8)
}
cav = cut(cs, 14)
floor = fillet(edges(of: cav.end), 2)
up = move_faces(cav.end, along: blk.end.normal, d: 1)
"#,
    );
}

/// edit Rebuild(Edge): offset a rect pocket's walls, floor and floor fillet; a rounded-rect
/// pocket builds
#[test]
fn offset_pocket_with_floor_round() {
    common::holds_nary(
        r#"
sketch bs(on: xy) {
  r = rect(corner: (-60, -40), size: (120, 80))
}
blk = extrude(bs, 40)
sketch cs(on: blk.end) {
  r = rect(corner: (-40, -25), size: (80, 50))
}
cav = cut(cs, 14)
fl = fillet(edges(of: cav.end), 2)
grow = offset_faces(cav.faces + fl.faces, -0.3)
"#,
    );
}

/// A face edit after a fillet builds, seq and n-ary, the shape the same edit gives before it:
/// the blends roll again on the changed faces with the laws the history gave them.
fn edits_through_blends(after: &str, before: &str) {
    common::holds_nary(after);
    let (a, b) = (body(after).compact(), body(before).compact());
    assert_eq!(a.faces.len(), b.faces.len());
    for v in &a.vertices {
        let d = b
            .vertices
            .iter()
            .map(|w| w.point.distance(v.point))
            .fold(f64::INFINITY, f64::min);
        assert!(d < 1e-6, "{:?} off by {d}", v.point);
    }
}

#[test]
fn move_face_by_variable_blend() {
    let block = "sketch s(on: xy) {\n  r = rect(corner: (0, 0), size: (60, 40))\n}\nb = extrude(s, 30)\n";
    let round = "round = fillet(edges(of: b.end, longer: 50), vary([(0, 2), (ratio(0.3), 5), (1, 3)]))\n";
    let up = "up = move_faces(b.end, along: b.end.normal, d: 4)\n";
    edits_through_blends(&format!("{block}{round}{up}"), &format!("{block}{up}{round}"));
    let lin = "round = fillet(edges(of: b.end, shorter: 50), vary(1.5, 4))\n";
    let out = "out = offset_faces(b.end, 2)\n";
    edits_through_blends(&format!("{block}{lin}{out}"), &format!("{block}{out}{lin}"));
}

#[test]
fn offset_boss_under_rolling_blend() {
    // a boss on a shaft: its root blend has no closed form
    let shaft = "sketch ss(on: yz) {
  c = circle((0, 0), 20)
}
s = extrude(ss, sym(60))
sketch bs(on: plane(at: ss.origin, normal: xy.normal, x: yz.normal)) {
  c = circle((0, 0), 8)
}
boss = extrude(bs, 30)
";
    let round = "round = fillet(edges(between: (boss, s), crease: \"concave\"), 2)\n";
    let out = "out = offset_faces(faces(of: boss, shape: \"cyl\"), 1)\n";
    edits_through_blends(&format!("{shaft}{round}{out}"), &format!("{shaft}{out}{round}"));
}

/// a revolve cut of a cap screw whose head shoulder lies on the top face. It was found as
/// invalid (EdgeUses 1) from a profile that ran the shank up through the head's base line,
/// crossing itself exactly at a sample corner: that profile is now refused
#[test]
fn revolve_cut_shoulder_on_face() {
    common::holds_nary(
        r#"
sketch us(on: xz) {
  r = rect(corner: (-50, 1), size: (100, 28))
}
u = extrude(us, sym(30))
sketch sc(on: us.plane) {
  ax = line((-37, 29), (-37, 30), construction: true)
  o = poly([(-37, -21), (-32, -21), (-32, 29), (-29, 29), (-29, 39), (-37, 39)])
}
screw = revolve(sc.o, axis: sc.ax, cut: true)
"#,
    );
    // the crossing profile is refused at its sketch, before the kernel
    let msg = common::refused(
        r#"
sketch us(on: xz) {
  r = rect(corner: (-50, 1), size: (100, 28))
}
u = extrude(us, sym(30))
sketch sc(on: us.plane) {
  ax = line((-37, 29), (-37, 30), construction: true)
  o = poly([(-37, 29), (-29, 29), (-29, 39), (-32, 39), (-32, -21), (-37, -21)])
}
screw = revolve(sc.o, axis: sc.ax, cut: true)
"#,
    );
    assert!(
        msg.contains("SketchProfile") && msg.contains("self intersection"),
        "{msg}"
    );
}

/// chamfer Unsupported(a vertex where other than three faces meet): a revolved hub's bore rim, its
/// round faces in halves meeting at the seam
#[test]
fn chamfer_revolved_bore_rim() {
    common::holds_nary(
        r#"
sketch sec(on: xz) {
  outline = poly([(20, 0), (70, 0), (70, 12), (36, 12), (36, 42), (20, 42)])
  ax = line((0, 0), (0, 20), construction: true)
}
hub = revolve(sec.outline, axis: sec.ax)
edge = chamfer(edges(between: (hub.side(sec.outline[5]), hub.side(sec.outline[4]))), 1)
"#,
    );
}

/// a revolve's flange halves merge round a fillet's spring into one face with a hole, not one
/// slit along the far seam (its mesh crossed itself), and a cut through the near seam keeps it
#[test]
fn fillet_revolved_step_meshes() {
    let src = r#"
XY = plane(at: xy.origin, normal: xy.normal, x: yz.normal)
sketch sec(on: xz) {
  outline = poly([(20, 0), (70, 0), (70, 12), (36, 12), (36, 42), (20, 42)])
  ax = line((0, 0), (0, 20), construction: true)
}
hub = revolve(sec.outline, axis: sec.ax)
round = fillet(edges(of: hub, shape: "circle", r: 36, crease: "concave"), 3)
sketch scs(on: XY) {
  c = circle((70, 0), 6)
}
sc = cut(scs, -12)
"#;
    common::holds_nary(src);
    forge_mesh::tessellate(&body(src), &forge_mesh::Quality::default()).expect("meshes");
}

/// fillet Overflow: fillet 1 at the root of a 1.5 tab on a 1.5 ring wall; 0.5 builds, a 7 wall
/// builds
#[test]
fn tab_root_fillet_on_thin_wall() {
    common::holds(
        r#"
sketch rs(on: xy) {
  o = circle((0, 0), 69.5)
  i = circle((0, 0), 68)
}
ring = extrude(rs, 50)
sketch ts(on: rs.plane) {
  r = rect(corner: (-88, -10), size: (19.25, 20))
}
tab = extrude(ts, 1.5)
root = fillet(edges(between: (tab, ring), crease: "concave", shape: "circle"), 1)
"#,
    );
}

/// seq valid; n-ary fails Fillet Corner: two mirrored gussets in a face-filleted corner, their tops
/// filleted, then their roots
#[test]
fn nary_fillet_two_gussets() {
    common::holds_nary(
        r#"
sketch ls(on: xz) {
  o = poly([(-60, 0), (60, 0), (60, 12), (-46, 12), (-46, 90), (-60, 90)])
}
l = extrude(ls, sym(80))
corner = facefillet(l.side(ls.o[3]), l.side(ls.o[2]), 8)
sketch gs(on: offset(ls.plane, -28)) {
  o = poly([(-47, 11), (20, 11), (-47, 60)])
}
gus = extrude(gs, sym(10))
guss = pattern(of: gus, layout: mirror(on: ls.plane))
top = fillet(edges(of: [gus, guss], crease: "convex"), 2)
root = fillet(edges(between: ([gus, guss], l), crease: "concave") + edges(between: ([gus, guss], corner), crease: "concave"), 3)
"#,
    );
}

/// The hole rim's fillet, made on the finished part, named an edge only the finished part has:
/// `regenerate_solids` worked on an unfinished one, missed it and left the fillet out, while
/// `regenerate` and the n-ary regens rounded it.
#[test]
fn fillet_on_hole_rim_across_dimple_on_every_path() {
    let src = r#"
sketch bs(on: xy) {
  r = rect(corner: (0, 0), size: (120.4, 91.68))
}
blk = extrude(bs, 21.74)
sketch bc(on: offset(bs.plane, 11.77)) {
  c = pt(34, 24.53)
}
dimple = ball(bc.c, 21.73, cut: true)
sketch hs(on: blk.end) {
  p = pt(41.73, 44.09)
}
h = hole(at: hs.p, d: 6.9, depth: thru)
rim = fillet(edges(between: (h.wall, blk.end)), 1.22)
"#;
    common::holds_nary(src);
    let (part, ids) = history(src);
    let n = part.features.len();
    let unrounded = Part {
        features: part.features[..n - 1].to_vec(),
    }
    .regenerate_ids(Some(&ids[..n - 1]))
    .expect("all but the fillet");
    let seq = part.regenerate_ids(Some(&ids)).expect("seq");
    assert!(
        seq.faces.len() > unrounded.compact().faces.len(),
        "seq left the fillet out"
    );
    let solids = part.regenerate_solids_ids(Some(&ids)).expect("seq solids");
    assert_eq!(solids[0].body.compact().faces.len(), seq.faces.len());
    let nary = part
        .regenerate_nary_solids_ids(1, Some(&ids))
        .expect("n-ary solids");
    assert_eq!(nary[0].body.compact().faces.len(), seq.faces.len());
}

/// A chamfer naming an edge as only the sequential regen names it (a long n-ary run with text):
/// the n-ary regen rebuilds feature by feature, the same solid, and the fallback is counted
/// (the naming gap of docs/naming.md, history seed 27). The `.fc` lines it was found as, and the
/// Forge part built to the same solid.
#[test]
fn nary_takes_the_sequential_regen_for_a_name_and_counts_it() {
    let fc = "\
L0: extrude 32.31 rrect 0 0 99.32 87.16 13.8
L1: revolve cut sym 82.88 xz at 49.66 20.05 35 axis 0 0 0 -1 circle 26.29 -8.46 9.57
L7: text add \"DO\" font \"block.ttf\" 25.81 3.44 z -1 0 0 at 0 63.47 16.16
L8: movebody rot 47.94 49.8 9.69 0 0 1 -23.29
L18: chamfer 3.22 ang 49.43 face (faces near 47.06 43.41 32.31) (edges near 16.16 9.26 32.31)
";
    let fg = r#"
XZ = plane(at: xy.origin, normal: xz.normal, x: yz.normal)
sketch plan(on: xy) {
  outline = hull(disc((13.8, 13.8), 13.8), disc((85.52, 13.8), 13.8), disc((85.52, 73.36), 13.8), disc((13.8, 73.36), 13.8))
}
base = extrude(plan, 32.31)
sketch groove(on: offset(XZ, -20.05)) {
  c = circle((75.95, 26.54), 9.57)
  ax = line((49.66, 35), (49.66, 34), construction: true)
}
ring = revolve(groove.c, axis: groove.ax, angle: sym(82.88deg), cut: true)
sketch at_text(on: offset(plan.plane, 16.16)) {
  p = pt(0, 63.47)
}
sketch pivot(on: plan) {
  p = pt(47.94, 49.8)
}
sketch pivot_up(on: offset(plan.plane, 9.69)) {
  p = pt(47.94, 49.8)
}
mark = text("DO", font: "block.ttf", size: 25.81, depth: 3.44, on: plane(at: at_text.p, normal: -yz.normal), at: at_text.p)
turned = movebody(of: base, moves: [spin(about: axis(pivot.p, pivot_up.p), by: -23.29deg)])
edge = chamfer(edges(between: (base.end, base.side(plan.outline[1])), longer: 20), 3.22, angle: 49.43deg, face: base.end)
"#;
    // the same solid as the .fc lines
    let m = common::holds(fg);
    let fcm = forge_agent::bench::model(fc);
    assert!(fcm.error.is_none(), "fc: {:?}", fcm.error.map(|e| e.msg));
    let fcb = fcm.body().expect("fc part");
    let mp = forge_topo::mass::mass(fcb).expect("fc mass");
    assert!(
        (m.volume - mp.volume).abs() <= 1e-7 * mp.volume,
        "{} {}",
        m.volume,
        mp.volume
    );
    assert_eq!(m.faces, fcb.face_ids().count());
    let (part, ids) = history(fg);
    let before = forge_model::nary_name_fallbacks();
    let nary = part
        .regenerate_nary_solids_ids(1, Some(&ids))
        .expect("n-ary")
        .remove(0)
        .body;
    // counted, once (the count is this thread's)
    assert_eq!(forge_model::nary_name_fallbacks(), before + 1);
    let seq = part.regenerate_ids(Some(&ids)).expect("seq");
    assert_eq!(nary.face_ids().count(), seq.face_ids().count());
    assert!(forge_topo::check::check(&nary).is_empty());
}
