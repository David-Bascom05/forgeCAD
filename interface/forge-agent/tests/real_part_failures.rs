//! Kernel failures the real-part benchmarks (tools/bench-parts) ran into, each shrunk to the
//! lines that fail. Ignored until fixed: each builds valid when its failure is gone. The part
//! files carry a TODO naming the test where they work round it.
use forge_agent::bench;
use forge_model::Part;

/// The lines build without error to a valid part, seq; n-ary too when asked.
fn holds(src: &str, nary: bool) {
    let m = bench::model(src);
    if let Some(e) = &m.error {
        panic!("line {}: {}", m.lines[e.at].no, e.msg);
    }
    let body = m.body().expect("a part");
    let problems = forge_topo::check::check(body);
    assert!(problems.is_empty(), "{:?}", &problems[..problems.len().min(3)]);
    if nary {
        let part = Part {
            features: m.features.clone(),
        };
        let b = part.regenerate_nary(1).unwrap_or_else(|e| panic!("nary: {e:?}"));
        assert!(forge_topo::check::check(&b).is_empty(), "nary: invalid");
    }
}

/// `holds`, n-ary too, and seq and n-ary give the same faces by name.
fn holds_alike(src: &str) {
    holds(src, true);
    let part = Part {
        features: bench::model(src).features.clone(),
    };
    let faces = |b: forge_topo::Body| {
        let mut n: Vec<_> = b.face_ids().map(|f| b.face(f).name).collect();
        n.sort();
        n
    };
    assert_eq!(part.regenerate().map(faces), part.regenerate_nary(1).map(faces));
}

/// `holds`, n-ary too, and seq and n-ary give the same number of faces.
fn holds_same_faces(src: &str) {
    holds(src, true);
    same_faces(src);
}

/// Seq and n-ary give the same number of faces.
fn same_faces(src: &str) {
    let part = Part {
        features: bench::model(src).features.clone(),
    };
    let faces = |b: forge_topo::Body| b.face_ids().count();
    assert_eq!(part.regenerate().map(faces), part.regenerate_nary(1).map(faces));
}

/// seq step: boolean Internal(face split crossed itself); half-disc boss crossing drafted+filleted
/// corner under a flange; ok without flange, without fillet, or boss r=25
#[test]
fn boss_through_drafted_fillet() {
    holds(
        "\
box: extrude 95 rect -120 -65 240 130
dr: draft -2 xy at 0 0 95 (faces of box normal +x) (faces of box normal -x) (faces of box normal +y) (faces of box normal -y)
fillet 12 (edges of box shorter 100)
flange: extrude 14 -xy at 0 0 95 rect -142 -87 284 174
extrude sym 174 xz at -78 0 95 loop line -33.5 0 33.5 0 arc 0 0 33.5 0 -180\n",
        false,
    );
}

/// fillet Overflow; concave fillet r3 on a foot pad whose side is 0.7mm from the tangent line of a
/// r12 fillet on drafted corner; ok when the pad is 14mm further
#[test]
fn fillet_runs_onto_drafted_corner_fillet() {
    holds(
        "\
box: extrude 95 rect -120 -65 240 130
draft -2 xy at 0 0 95 (faces of box normal -x) (faces of box normal -y)
fillet 12 (edges of box shorter 100 near -120 -65 50)
extrude 16 rect -104 -93 44 40
fillet 3 (edges concave)\n",
        false,
    );
}

/// rib draft 1 whose end runs into a foot's corner fillet blend: Rib(Boolean(Unsupported(surface
/// pair without a closed form))); ok without draft
#[test]
#[ignore = "R3: kernel failure, see the comment"]
fn drafted_rib_into_blend() {
    holds(
        "\
extrude 95 rect -120 -65 240 130
extrude 44 xz at 62 87 95 loop line -42 0 42 0 arc 0 0 42 0 -180
feet: extrude 16 loop line -96 -55 -96 -85 arc -88 -85 8 180 90 line -88 -93 -60 -93 arc -60 -85 8 270 90 line -52 -85 -52 -55 line -52 -55 -96 -55
pattern feet lin 1 0 0 2 148 0 1 0 2 146
fillet 5 (edges between feet f1 concave)
rib 8 draft 1 yz at 62 0 0 line 63 20 83 49\n",
        false,
    );
}

/// modeled thread (thread hole, 12 deep) in a blind hole in a boss standing on a 2deg drafted face:
/// boolean Unsupported(surface pair without a closed form); same without the draft builds
#[test]
fn thread_in_boss_on_drafted_wall() {
    holds(
        "\
box: extrude 95 rect -120 -65 240 130
draft -2 xy at 0 0 95 (faces of box normal +x)
extrude 12 yz at 114 0 22 circle 0 0 14
hole 14.4 -yz at 126 0 22 16 thread 16 1.5
thread hole hole1 12\n",
        false,
    );
}

/// fillet 2 on the curve where a half-round boss meets a 2deg drafted wall: Overflow; same without
/// draft builds
#[test]
fn fillet_boss_on_drafted_wall() {
    holds(
        "\
box: extrude 95 rect -120 -65 240 130
draft -2 xy at 0 0 95 (faces of box normal +y)
boss: extrude 36 xz at -72 87 95 loop line -33.5 0 33.5 0 arc 0 0 33.5 0 -180
fillet 2 (edges between boss box concave)\n",
        false,
    );
}

/// half-disc boss, concentric bore cut through, chamfer 1 the bore's semicircle edge: chamfer
/// Invalid(DuplicateName x3); a rect block with the same half-bore chamfers fine
#[test]
fn chamfer_half_ring_bore() {
    holds(
        "\
boss: extrude 36 xz at -72 87 95 loop line -33.5 0 33.5 0 arc 0 0 33.5 0 -180
bore: extrude cut thru both xz at -72 0 95 circle 0 0 23.5
chamfer 1 (edges of bore circle back)\n",
        false,
    );
}

/// extrude of a loop of a line and one 180deg arc is invalid: DuplicateName x3 (check); a 90deg arc
/// or two 90deg arcs are valid. Root of R6 and of gearbox invalid stages
#[test]
fn half_disc_extrude() {
    holds(
        "\
extrude 36 loop line -33 0 33 0 arc 0 0 33 0 -180\n",
        false,
    );
}

/// two overlapping half-round bosses, a revolve-cut ring groove in one that reaches the overlap:
/// DuplicateName (check); either boss alone is valid
#[test]
fn groove_across_overlapping_bosses() {
    holds(
        "\
extrude 36 xz at -8 87 95 loop line -41 0 41 0 arc 0 0 41 0 -90 arc 0 0 41 -90 -90
extrude 36 xz at 62 87 95 loop line -42 0 42 0 arc 0 0 42 0 -90 arc 0 0 42 -90 -90
revolve cut yz at 62 0 95 axis x rect 79 33 2.2 3.4\n",
        false,
    );
}

/// an M22x1 internal thread in a bore of a spindle boss, a smooth loft arm starting 1mm outside the
/// thread's major radius (no contact): boolean Unsupported(surface pair without a closed form); the
/// same with a plain loft builds
#[test]
fn thread_near_smooth_loft() {
    holds(
        "\
arm: loft smooth profile yz at 12 0 0 rrect -15 -12 30 24 6 profile yz at 85 0 0 rrect -11.5 -9.5 23 19 5 profile yz at 162 0 0 rrect -9 -7.5 18 15 4
extrude sym 30 circle 0 0 21
hole 20.9 -xy at 0 0 15 7
thread M22x1 -xy at 0 0 15 int 6\n",
        false,
    );
}

/// a two-profile tapered loft arm into a cylindrical boss, fillet 1.2 (also 0.5) round the
/// junction: fillet NoBlend; an extruded arm of the same section fillets fine
#[test]
fn fillet_loft_into_boss() {
    holds(
        "\
arm: loft profile yz at 12 0 0 rrect -15 -12 30 24 6 profile yz at 166 0 0 rrect -9 -7.5 18 15 4
pedal: extrude sym 18 at 170 0 0 circle 0 0 12
fillet 1.2 (edges between arm pedal concave)\n",
        false,
    );
}

/// smooth loft through three circles, then a cylinder extruded on from its end circle: boolean
/// Unsupported(surface pair without a closed form); the same off a plain extrude is fine
#[test]
fn extrude_from_smooth_loft_end() {
    holds(
        "\
loft smooth profile at 0 0 0 circle 0 0 10 profile at 0 5 30 circle 0 0 14 profile at 0 15 60 circle 0 0 18
extrude 40 at 0 15 60 circle 0 0 18\n",
        false,
    );
}

/// shell 2 of a plain loft between two circles whose centres are offset (an oblique cone), top
/// open: shell Rebuild(Edge); a coaxial loft shells fine. So no shelled volute
#[test]
fn shell_oblique_loft() {
    holds(
        "\
loft profile at 0 0 0 circle 0 0 10 profile at 0 5 30 circle 0 0 14
shell 2 (faces plane top)\n",
        false,
    );
}

/// smooth spiral loft, then extrude cut 37 at z=-15 of a circle r78 whose bottom plane and wall
/// both cross the loft: boolean Unsupported(surface pair without a closed form); the same cut sym
/// 80 (no plane crossing) or r84 builds
#[test]
fn cut_cylinder_plane_corner_on_smooth_loft() {
    holds(
        "\
volute: loft smooth profile z -sin(60) cos(60) 0 x cos(60) sin(60) 0 at 87.17*cos(60) 87.17*sin(60) 0 circle 0 0 16.67 profile z -sin(110) cos(110) 0 x cos(110) sin(110) 0 at 89.81*cos(110) 89.81*sin(110) 0 circle 0 0 18.89 profile z -sin(160) cos(160) 0 x cos(160) sin(160) 0 at 92.44*cos(160) 92.44*sin(160) 0 circle 0 0 21.11 profile z -sin(210) cos(210) 0 x cos(210) sin(210) 0 at 95.08*cos(210) 95.08*sin(210) 0 circle 0 0 23.33 profile z -sin(260) cos(260) 0 x cos(260) sin(260) 0 at 97.72*cos(260) 97.72*sin(260) 0 circle 0 0 25.56 profile z -sin(310) cos(310) 0 x cos(310) sin(310) 0 at 100.4*cos(310) 100.4*sin(310) 0 circle 0 0 27.78 profile z -sin(360) cos(360) 0 x cos(360) sin(360) 0 at 103*cos(360) 103*sin(360) 0 circle 0 0 30 profile z 0 1 0 x 1 0 0 at 103 55 0 circle 0 0 33.5 profile z 0 1 0 x 1 0 0 at 103 110 0 circle 0 0 37
chamber: extrude cut 37 at 0 0 -15 circle 0 0 78\n",
        false,
    );
}

/// a flat pad standing into a smooth three-circle loft, fillet 2 round the junction: fillet
/// NoBlend; a plain loft too. Same family as R14: no fillet holds on any loft face
#[test]
#[ignore = "R18: kernel failure, see the comment"]
fn fillet_pad_into_smooth_loft() {
    holds(
        "\
tube: loft smooth profile at 0 0 0 circle 0 0 10 profile at 0 5 30 circle 0 0 14 profile at 0 15 60 circle 0 0 18
pad: extrude 5 yz at 8 -30 -10 rect 0 0 60 80
fillet 2 (edges between pad tube concave)\n",
        false,
    );
}

/// a 28.6 top tube into a revolved tapered head tube (cone), fillet 4..2.5 round the joint: builds
/// but check fails EdgeOffFace 0.0014; constant fillet 3 is valid; the same with axes at a round
/// angle did not reproduce
#[test]
fn variable_fillet_tube_into_cone() {
    holds(
        "\
reach = 385
stack = 575
hta = 73 deg
sta = 73.5 deg
htl = 160
stl = 540
hdx = cos(hta)
hdz = -sin(hta)
s1x = -(stl-35)*cos(sta)
s1z = (stl-35)*sin(sta)
h1x = reach + 25*hdx
h1z = stack + 25*hdz
ttl = sqrt((h1x-s1x)^2 + (h1z-s1z)^2)
ht: revolve xz at reach 0 stack axis 0 0 hdx hdz poly 0 0 25*sin(hta) 25*cos(hta) htl*hdx+30*sin(hta) -htl*sin(hta)+30*cos(hta) htl*hdx -htl*sin(hta)
tt: extrude ttl z (h1x-s1x)/ttl 0 (h1z-s1z)/ttl at s1x 0 s1z circle 0 0 14.3
fillet 4..2.5 (edges between tt ht concave)\n",
        false,
    );
}

/// BB shell, a chainstay cylinder (oblique axis) running into it, the shell bored to the BSA minor,
/// thread 1.370-24 int 16 left: boolean Unsupported(surface pair without a closed form); without
/// the stay it builds. The stay's surface no longer reaches the bore once it is cut, so this is the
/// helix paired with a face it never meets (as R13, R4)
#[test]
fn bsa_thread_with_chainstay() {
    holds(
        "\
extrude sym 68 -xz circle 0 0 21
extrude 300 z -0.98 -0.09 0.17 at -12 -22 0 circle 0 0 11
hole 33.65 -xz at 0 -34 0 68
thread 1.370-24 -xz at 0 -34 0 int 16 left chamfer\n",
        false,
    );
}

/// a bottle boss on a tube whose axis is not a world axis, M5 tapped through into the bore, thread
/// hole full: boolean Unsupported(surface pair without a closed form); the same tube along x builds
/// (helix vs a cylinder at a general angle has no closed form)
#[test]
fn thread_boss_into_oblique_tube() {
    holds(
        "\
extrude 300 z 0.6 0 0.8 circle 0 0 17.5
extrude 21 z -0.8 0 0.6 at 90 0 120 circle 0 0 4.5
extrude cut 300 z 0.6 0 0.8 circle 0 0 16.5
bh: hole 4.2 z 0.8 0 -0.6 at 73.2 0 132.6 6 thread 5 0.8
thread hole bh full\n",
        false,
    );
}

/// a tapered arm outline (two lines tangent to a 180.7deg tip arc, square root end) extruded,
/// fillet 2 all four top edges: fillet Corner{vertex 1}; any subset (root edge alone, arc alone,
/// sides alone, arc+sides) builds, and the same with a rrect or a plain taper poly builds
#[test]
fn fillet_top_tapered_arm_tip() {
    holds(
        "\
extrude 14 loop line 0 -15 169.905858 -15.999723 arc 170 0 16 -90.337123 180.674246 line 169.905858 15.999723 0 15 line 0 15 0 -15
fillet 2 (edges top)\n",
        false,
    );
}

/// a bar, fillet 4..3 on two top edges, shell 1.6 open below: shell Collapses on the variable blend
/// (its radius never under 3); fillet 3 constant shells fine
#[test]
fn shell_after_variable_fillet() {
    holds(
        "\
extrude 14 rect 0 -15 170 30
fillet 4..3 (edges top line along +x)
shell 1.6 (faces bottom)\n",
        false,
    );
}

/// a disc (axis y) and a block flush with both its faces (a conrod bolt boss), fillet 3 on the two
/// concave lines where they meet: fillet Overflow; the flush end faces stay as three unmerged
/// coplanar faces (see union-coplanar-faces-unmerged), likely why the blend cannot end on them
#[test]
#[ignore = "R24: kernel failure, see the comment"]
fn fillet_block_on_cylinder_flush() {
    holds(
        "\
big: extrude sym 22 xz circle 0 0 31
b: extrude sym 22 xz rect 26 -24 16 46
fillet 3 (edges between b big concave)\n",
        false,
    );
}

/// a rod r6 and a coil (wire r1.2 on radius 9) round it, coaxial, not touching: boolean
/// Unsupported(surface pair without a closed form); a coil whose end sinks into a disc builds. A
/// spring over its guide rod cannot be modelled
#[test]
fn coil_round_rod() {
    holds(
        "\
extrude 30 at 0 0 0 circle 0 0 6
coil xz at 0 0 5 circle 9 0 1.2 pitch 6 turns 2\n",
        false,
    );
}

/// a disc with a block flush to both its faces, the disc bored through, chamfer 1 the bore's rim
/// circles: chamfer Rebuild(Edge); the disc alone chamfers fine (the unmerged coplanar end faces
/// again, as R24)
#[test]
fn chamfer_bore_past_flush_block() {
    holds(
        "\
big: extrude sym 22 xz circle 0 0 31
bx: extrude sym 22 xz rect 20 -10 30 20
bigbore: hole 48 xz at 0 11 0 thru
chamfer 1 (edges of bigbore circle)\n",
        false,
    );
}

/// seq builds valid; n-ary fails Chamfer NoSuchEdge on the bore-rim chamfer after a fillet of the
/// shank into the big end (without the bolt-boss block it passes)
#[test]
fn nary_chamfer_after_fillet_conrod() {
    holds(
        "\
c = 150
rb = 31
rs = 15
tb = 22
ts = 20
side: sketch xz
  line -14 rb-6 -8 c-rs+5
  line l1.b 8 c-rs+5
  line l2.b 14 rb-6
  line l3.b l1.a
end
shank: extrude sym ts side
big: extrude sym tb xz circle 0 0 rb
bolts: extrude sym tb xz rect 26 -24 16 46
fillet 6 (edges between shank big concave line)
bigbore: hole 48 xz at 0 tb/2 0 thru
chamfer 1 (edges of bigbore circle)\n",
        true,
    );
}

/// a crank spider (arm on a rotated frame, patterned 5 round the hub), fillet 3 where the arms meet
/// the hub: seq valid; n-ary fails MissingEdge on the fillet
#[test]
fn nary_fillet_after_circular_pattern() {
    holds_alike(
        "\
spindle: extrude sym 30 circle 0 0 21
spider: extrude 6 z 0 0 1 x 0.809017 0.587785 0 at 0 0 -15 rrect 0 -7.5 62 15 6
spiders: pattern spider circ 0 0 0 0 0 1 5
fillet 3 (edges between spider spindle concave)\n",
    );
}

/// a block, two separate foot pads, one fillet 5 where both pads meet the block: seq valid; n-ary
/// fails MissingEdge on the fillet (one pad per fillet line passes; a lin pattern of one pad fails
/// the same). Root of R29 and of the gearbox and frame n-ary failures
#[test]
fn nary_fillet_after_linear_pattern() {
    holds_alike(
        "\
box: extrude 95 rect -120 -65 240 130
a: extrude 16 rect -96 -93 44 34
b: extrude 16 rect 52 -93 44 34
fillet 5 (edges between a box concave) (edges between b box concave)\n",
    );
}

/// a three-profile plain loft arm into a pedal boss, the boss bored and threaded 9/16-20 left: seq
/// valid; n-ary fails Boolean Unsupported(surface pair without a closed form) (n-ary twin of R13;
/// two profiles pass both ways)
#[test]
fn nary_thread_near_three_profile_loft() {
    holds_same_faces(
        "\
arm: loft profile yz at 12 0 0 rrect -15 -12 30 24 6 profile yz at 85 0 0 rrect -11.5 -9.5 23 19 5 profile yz at 166 0 0 rrect -9 -7.5 18 15 4
pedal: extrude sym 18 at 170 0 0 circle 0 0 12
hole 12.913 -xy at 170 0 9 thru
thread 9/16-20 -xy at 170 0 9 int 18 left chamfer\n",
    );
}

/// smooth spiral loft volute, a cover-flange disc through it, the smooth core loft cut, then one
/// discharge-flange bolt hole far from the rest: seq valid; n-ary fails Boolean Unsupported(surface
/// pair without a closed form) at the hole (without the disc or without the core it passes)
#[test]
fn nary_volute_core_and_flange_hole() {
    holds(
        "\
volute: loft smooth profile z -sin(60) cos(60) 0 x cos(60) sin(60) 0 at 87.17*cos(60) 87.17*sin(60) 0 circle 0 0 16.67 profile z -sin(110) cos(110) 0 x cos(110) sin(110) 0 at 89.81*cos(110) 89.81*sin(110) 0 circle 0 0 18.89 profile z -sin(160) cos(160) 0 x cos(160) sin(160) 0 at 92.44*cos(160) 92.44*sin(160) 0 circle 0 0 21.11 profile z -sin(210) cos(210) 0 x cos(210) sin(210) 0 at 95.08*cos(210) 95.08*sin(210) 0 circle 0 0 23.33 profile z -sin(260) cos(260) 0 x cos(260) sin(260) 0 at 97.72*cos(260) 97.72*sin(260) 0 circle 0 0 25.56 profile z -sin(310) cos(310) 0 x cos(310) sin(310) 0 at 100.4*cos(310) 100.4*sin(310) 0 circle 0 0 27.78 profile z -sin(360) cos(360) 0 x cos(360) sin(360) 0 at 103*cos(360) 103*sin(360) 0 circle 0 0 30 profile z 0 1 0 x 1 0 0 at 103 55 0 circle 0 0 33.5 profile z 0 1 0 x 1 0 0 at 103 110 0 circle 0 0 37
cflange: extrude 12 at 0 0 10 circle 0 0 104
core: loft cut smooth profile z -sin(60) cos(60) 0 x cos(60) sin(60) 0 at 87.17*cos(60) 87.17*sin(60) 0 circle 0 0 11.67 profile z -sin(110) cos(110) 0 x cos(110) sin(110) 0 at 89.81*cos(110) 89.81*sin(110) 0 circle 0 0 13.89 profile z -sin(160) cos(160) 0 x cos(160) sin(160) 0 at 92.44*cos(160) 92.44*sin(160) 0 circle 0 0 16.11 profile z -sin(210) cos(210) 0 x cos(210) sin(210) 0 at 95.08*cos(210) 95.08*sin(210) 0 circle 0 0 18.33 profile z -sin(260) cos(260) 0 x cos(260) sin(260) 0 at 97.72*cos(260) 97.72*sin(260) 0 circle 0 0 20.56 profile z -sin(310) cos(310) 0 x cos(310) sin(310) 0 at 100.4*cos(310) 100.4*sin(310) 0 circle 0 0 22.78 profile z -sin(360) cos(360) 0 x cos(360) sin(360) 0 at 103*cos(360) 103*sin(360) 0 circle 0 0 25 profile z 0 1 0 x 1 0 0 at 103 55 0 circle 0 0 28.5 profile z 0 1 0 x 1 0 0 at 103 110 0 circle 0 0 32 profile z 0 1 0 x 1 0 0 at 103 125 0 circle 0 0 33
dbolt: hole 14 -xz at 103+45 96 0 thru\n",
        true,
    );
}

/// a block, two separate foot pads, fillet 5 the first pad, then fillet 5 the second: the second
/// fails Overflow (seq); each alone, or pad-fillet-pad-fillet interleaved, builds
#[test]
fn second_pad_fillet_overflow() {
    holds(
        "\
box: extrude 95 rect -120 -65 240 130
a: extrude 16 rect -96 -93 44 34
b: extrude 16 rect 52 -93 44 34
fillet 5 (edges between a box concave)
fillet 5 (edges between b box concave)\n",
        false,
    );
}

/// bottom bracket shell, seat tube and down tube from its axis, one fillet 3 over all three joints:
/// seq valid; n-ary fails MissingEdge (any of the joints filleted alone fails Overflow seq, they
/// cross)
#[test]
fn nary_bb_cluster_fillet() {
    holds_alike(
        "\
reach = 385
stack = 575
hta = 73 deg
sta = 73.5 deg
htl = 160
stl = 540
hdx = cos(hta)
hdz = -sin(hta)
hbx = reach + htl*hdx
hbz = stack + htl*hdz
dtx = hbx - 45*hdx
dtz = hbz - 45*hdz
dtl = sqrt(dtx^2 + dtz^2)
bb: extrude sym 68 -xz circle 0 0 21
st: extrude stl z -cos(sta) 0 sin(sta) circle 0 0 15.9
dt: extrude dtl z dtx/dtl 0 dtz/dtl circle 0 0 17.5
fillet 3 (edges between st bb concave) (edges between dt bb concave) (edges between st dt concave)\n",
    );
}

/// down tube, tapered head tube (revolved cone), top tube, fillet 3 where the top tube meets the
/// head tube: seq valid; n-ary fails MissingEdge (without the down tube it passes)
#[test]
fn nary_fillet_top_tube_head_tube() {
    holds_alike(
        "\
reach = 385
stack = 575
hta = 73 deg
sta = 73.5 deg
htl = 160
stl = 540
hdx = cos(hta)
hdz = -sin(hta)
hbx = reach + htl*hdx
hbz = stack + htl*hdz
dtx = hbx - 45*hdx
dtz = hbz - 45*hdz
dtl = sqrt(dtx^2 + dtz^2)
s1x = -(stl-45)*cos(sta)
s1z = (stl-45)*sin(sta)
h1x = reach + 25*hdx
h1z = stack + 25*hdz
ttl = sqrt((h1x-s1x)^2 + (h1z-s1z)^2)
dt: extrude dtl z dtx/dtl 0 dtz/dtl circle 0 0 17.5
ht: revolve xz at reach 0 stack axis 0 0 hdx hdz poly 0 0 25*sin(hta) 25*cos(hta) htl*hdx+30*sin(hta) -htl*sin(hta)+30*cos(hta) htl*hdx -htl*sin(hta)
tt: extrude ttl z (h1x-s1x)/ttl 0 (h1z-s1z)/ttl at s1x 0 s1z circle 0 0 14.3
fillet 3 (edges between tt ht concave)\n",
    );
}

/// a disc and a block flush with both its faces: seq leaves 12 faces (each end face split in two),
/// n-ary 10 (merged); the two paths give different topology for the same part. In the benchmarks:
/// gearbox 359 vs 338, crank 145 vs 144, conrod 158 vs 154 faces. Likely the same root as R24, R26
/// and union-coplanar-faces-unmerged
#[test]
fn seq_nary_face_count_differ() {
    same_faces(
        "\
big: extrude sym 22 xz circle 0 0 31
b: extrude sym 22 xz rect 26 -24 16 46\n",
    );
}

/// seat tube, top tube, fillet 0:3,0.5:4.5,1:3 at their joint, two loft cuts (stay bores, one
/// missing the part), the pair mirrored: the mirror leaves an invalid body, EdgeUses (seq; build
/// says ok, check fails); with a constant fillet, or mirroring only the cut that hits, it is valid
#[test]
fn mirror_cuts_after_variable_fillet() {
    holds(
        "\
reach = 385
stack = 575
hta = 73 deg
sta = 73.5 deg
stl = 540
csl = 410
drop = 70
sp = 130
hdx = cos(hta)
hdz = -sin(hta)
ax = -sqrt(csl^2 - drop^2)
az = drop
s1x = -(stl-45)*cos(sta)
s1z = (stl-45)*sin(sta)
h1x = reach + 25*hdx
h1z = stack + 25*hdz
ttl = sqrt((h1x-s1x)^2 + (h1z-s1z)^2)
c0x = -17.5
c0y = 22
c1x = ax + 6
c1y = sp/2 - 4
csx = c1x - c0x
csy = c1y - c0y
csz = az
cl = sqrt(csx^2 + csy^2 + csz^2)
q0x = -(stl-80)*cos(sta)
q0z = (stl-80)*sin(sta)
q0y = 10
q1x = ax + 10
q1z = az + 14
q1y = sp/2 - 4
ssx = q1x - q0x
ssy = q1y - q0y
ssz = q1z - q0z
sl = sqrt(ssx^2 + ssy^2 + ssz^2)
st: extrude stl z -cos(sta) 0 sin(sta) circle 0 0 15.9
tt: extrude ttl z (h1x-s1x)/ttl 0 (h1z-s1z)/ttl at s1x 0 s1z circle 0 0 14.3
fillet 0:3,0.5:4.5,1:3 (edges between tt st concave)
csb: loft cut profile z csx/cl csy/cl csz/cl at c0x+12*csx/cl c0y+12*csy/cl 12*csz/cl circle 0 0 9.9 profile z csx/cl csy/cl csz/cl at c1x c1y az circle 0 0 5.5
ssb: loft cut profile z ssx/sl ssy/sl ssz/sl at q0x q0y q0z circle 0 0 7 profile z ssx/sl ssy/sl ssz/sl at q1x q1y q1z circle 0 0 5
pattern csb ssb mirror xz\n",
        false,
    );
}

/// boolean Unsupported(surface pair without a closed form): a tube swept along an arc and its mirror, 12
/// apart, never touching; a torus face's box is its whole torus, and torus/torus with parallel
/// axes has no closed form; 20 apart builds
#[test]
fn disjoint_swept_arcs() {
    holds(
        "\
s: sweep z 0 1 0 at 0 10 0 circle 0 0 4 path arc yz at 0 10 -100 100 90 -40
pattern s mirror xz\n",
        false,
    );
}

/// HANGS: a tube swept along a line and a 2.4 degree arc in a general plane, bored by a sweep cut of a
/// smaller circle along the same path; the same in a world plane builds in 20 ms
#[test]
fn bent_tube_bore_hangs() {
    holds(
        "\
sweep z -0.59048313866411 0.10148065235589 -0.800644328120049 at -130.647058563804 10 441.057078039369 circle 0 0 7 path line -130.647058563804 10 441.057078039369 -313.740381258896 41.4664866995883 192.798286663673 arc z 0.804799093005477 -6.93889390390723e-18 -0.593547319004611 x -0.58966414290279 0.114200861333259 -0.799533840337893 at -306.512352962495 160.846988255089 202.598871100508 120 -90.7330947118864 2.44323436805379
sweep cut z -0.59048313866411 0.10148065235589 -0.800644328120049 at -130.647058563804 10 441.057078039369 circle 0 0 6 path line -130.647058563804 10 441.057078039369 -313.740381258896 41.4664866995883 192.798286663673 arc z 0.804799093005477 -6.93889390390723e-18 -0.593547319004611 x -0.58966414290279 0.114200861333259 -0.799533840337893 at -306.512352962495 160.846988255089 202.598871100508 120 -90.7330947118864 2.44323436805379\n",
        false,
    );
}

/// fillet NoBlend: facefillet between an upright unioned onto a plate and the plate's top; a plain
/// fillet of the same edge builds. face_fillet compacts the body, then uses the FaceIds it was given
#[test]
fn facefillet_after_union() {
    holds(
        "\
base: extrude 10 rect -40 -30 80 60
up: extrude 50 xy at 0 0 10 rect -40 -30 12 60
facefillet 6 (faces of up normal +x) to (faces of base normal +z)\n",
        false,
    );
}

/// fillet Overflow: fullround of a rib unioned onto a plate; the rib alone builds (stale FaceIds, as B3)
#[test]
fn fullround_after_union() {
    holds(
        "\
base: extrude 10 rect -40 -30 80 60
rib: extrude 40 xy at 0 0 10 rect -30 -4 60 8
fullround (faces of rib normal -y) (faces of rib normal +z) (faces of rib normal +y)\n",
        false,
    );
}

/// HANGS: thicken a flange face after a line-arc-line runner swept out of it and bored by a sweep
/// cut; thicken first, or a straight runner, builds
#[test]
fn thicken_after_bent_bore() {
    holds(
        "\
flange: extrude 12 yz at -12 0 0 rrect -60 -30 120 60 8
runner: sweep yz at -12 0 0 circle 0 0 19 path line -12 0 0 30 0 0 arc xz at 30 0 -60 60 90 -90 line 90 0 -60 90 0 -115
sweep cut yz at -12 0 0 circle 0 0 15.5 path line -12 0 0 30 0 0 arc xz at 30 0 -60 60 90 -90 line 90 0 -60 90 0 -130
thicken 1.5 (faces of flange normal -x)\n",
        false,
    );
}

/// edit Rebuild(Vertex): replace the top face of a block whose pocket rim is rounded; without the
/// round it builds
#[test]
fn replace_face_by_round() {
    holds(
        "\
blk: extrude 40 rect -60 -40 120 80
cav: extrude cut 14 -xy at 0 0 40 rect -40 -25 80 50
fillet 2 (edges of cav top)
part: plane at 0 0 36
replace (faces of blk normal +z) with part\n",
        true,
    );
}

/// edit Rebuild(Vertex): move a pocket floor up 1 after its edges are filleted; before, it builds
#[test]
fn move_face_by_blend() {
    holds(
        "\
blk: extrude 40 rect -60 -40 120 80
cav: extrude cut 14 -xy at 0 0 40 rrect -40 -25 80 50 8
fillet 2 (edges of cav bottom)
move (faces of cav normal +z) by 0 0 1\n",
        true,
    );
}

/// edit Rebuild(Edge): offset a rect pocket's walls, floor and floor fillet; a rounded-rect
/// pocket builds
#[test]
fn offset_pocket_with_floor_round() {
    holds(
        "\
blk: extrude 40 rect -60 -40 120 80
cav: extrude cut 14 -xy at 0 0 40 rect -40 -25 80 50
fl: fillet 2 (edges of cav bottom)
offset -0.3 (faces of cav) (faces of fl)\n",
        true,
    );
}

/// A face edit after a fillet builds, seq and n-ary, the shape the same edit gives before it:
/// the blends roll again on the changed faces with the laws the history gave them.
fn edits_through_blends(after: &str, before: &str) {
    holds(after, true);
    let (a, b) = (bench::model(after), bench::model(before));
    let (a, b) = (
        a.body().expect("a part").compact(),
        b.body().expect("a part").compact(),
    );
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
    let block = "b: extrude 30 rect 0 0 60 40\n";
    let round = "fillet 0:2,0.3:5,1:3 (edges of b top longer 50)\n";
    let up = "move (faces of b normal +z) by 0 0 4\n";
    edits_through_blends(&format!("{block}{round}{up}"), &format!("{block}{up}{round}"));
    let lin = "fillet 1.5..4 (edges of b top shorter 50)\n";
    let out = "offset 2 (faces of b normal +z)\n";
    edits_through_blends(&format!("{block}{lin}{out}"), &format!("{block}{out}{lin}"));
}

#[test]
fn offset_boss_under_rolling_blend() {
    // a boss on a shaft: its root blend has no closed form
    let shaft = "s: extrude sym 60 yz circle 0 0 20\nboss: extrude 30 xy circle 0 0 8\n";
    let round = "fillet 2 (edges between boss s concave)\n";
    let out = "offset 1 (faces of boss cyl)\n";
    edits_through_blends(&format!("{shaft}{round}{out}"), &format!("{shaft}{out}{round}"));
}

/// a revolve cut of a cap screw whose head shoulder lies on the top face. It was found as
/// invalid (EdgeUses 1) from a profile that ran the shank up through the head's base line,
/// crossing itself exactly at a sample corner: that profile is now refused
#[test]
fn revolve_cut_shoulder_on_face() {
    let block = "u: extrude sym 30 xz rect -50 1 100 28\n";
    holds(
        &format!("{block}revolve cut xz at -37 0 29 axis 0 0 0 1 poly 0 -50 5 -50 5 0 8 0 8 10 0 10\n"),
        true,
    );
    let m = bench::model(&format!(
        "{block}revolve cut xz at -37 0 29 axis 0 0 0 1 poly 0 0 8 0 8 10 5 10 5 -50 0 -50\n"
    ));
    assert!(m.error.is_some_and(|e| e.msg.contains("SelfIntersecting")));
}

/// chamfer Unsupported(a vertex where other than three faces meet): a revolved hub's bore rim, its
/// round faces in halves meeting at the seam
#[test]
fn chamfer_revolved_bore_rim() {
    holds(
        "\
hub: revolve xz axis y poly 20 0 70 0 70 12 36 12 36 42 20 42
chamfer 1 (edges of hub circle r=20 top)\n",
        true,
    );
}

/// a revolve's flange halves merge round a fillet's spring into one face with a hole, not one
/// slit along the far seam (its mesh crossed itself), and a cut through the near seam keeps it
#[test]
fn fillet_revolved_step_meshes() {
    let src = "\
hub: revolve xz axis y poly 20 0 70 0 70 12 36 12 36 42 20 42
fillet 3 (edges of hub circle r=36 concave)
sc: extrude cut 12 xy at 70 0 0 circle 0 0 6\n";
    holds(src, true);
    let m = bench::model(src);
    forge_mesh::tessellate(m.body().expect("a part"), &forge_mesh::Quality::default()).expect("meshes");
}

/// fillet Overflow: fillet 1 at the root of a 1.5 tab on a 1.5 ring wall; 0.5 builds, a 7 wall
/// builds
#[test]
fn tab_root_fillet_on_thin_wall() {
    holds(
        "\
ring: extrude 50 circle 0 0 69.5 circle 0 0 68
tab: extrude 1.5 rect -88 -10 19.25 20
fillet 1 (edges between tab ring concave circle)\n",
        false,
    );
}

/// seq valid; n-ary fails Fillet Corner: two mirrored gussets in a face-filleted corner, their tops
/// filleted, then their roots
#[test]
fn nary_fillet_two_gussets() {
    holds(
        "\
l: extrude sym 80 xz poly -60 0 60 0 60 12 -46 12 -46 90 -60 90
corner: facefillet 8 (faces normal +x left) to (faces normal +z bottom)
gus: extrude sym 10 xz at 0 28 0 poly -47 11 20 11 -47 60
pattern gus mirror xz
fillet 2 (edges of gus convex)
fillet 3 (edges between gus l concave) (edges between gus corner concave)\n",
        true,
    );
}

/// The hole rim's fillet, made on the finished part, named an edge only the finished part has:
/// `regenerate_solids` worked on an unfinished one, missed it and left the fillet out, while
/// `regenerate` and the n-ary regens rounded it.
#[test]
fn fillet_on_hole_rim_across_dimple_on_every_path() {
    let src = "\
extrude 21.74 rect 0 0 120.4 91.68
ball cut 21.73 at 34 24.53 11.77
hole 6.9 -xy at 41.73 44.09 21.74 thru pt 118
fillet 1.22 (edges near 39.04 46.25 21.74)
";
    holds(src, true);
    let part = Part {
        features: bench::model(src).features.clone(),
    };
    let unrounded = Part {
        features: part.features[..3].to_vec(),
    }
    .regenerate()
    .expect("three lines");
    let seq = part.regenerate().expect("seq");
    assert!(
        seq.faces.len() > unrounded.compact().faces.len(),
        "seq left the fillet out"
    );
    let solids = part.regenerate_solids().expect("seq solids");
    assert_eq!(solids[0].body.compact().faces.len(), seq.faces.len());
    let nary = part.regenerate_nary_solids(1).expect("n-ary solids");
    assert_eq!(nary[0].body.compact().faces.len(), seq.faces.len());
}
