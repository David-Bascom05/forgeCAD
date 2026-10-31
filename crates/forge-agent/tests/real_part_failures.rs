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

/// Seq and n-ary give the same number of faces.
fn same_faces(src: &str) {
    let part = Part {
        features: bench::model(src).features.clone(),
    };
    let faces = |b: forge_topo::Body| b.faces.len();
    assert_eq!(part.regenerate().map(faces), part.regenerate_nary(1).map(faces));
}

/// seq step: boolean Internal(face split crossed itself); half-disc boss crossing drafted+filleted
/// corner under a flange; ok without flange, without fillet, or boss r=25
#[test]
#[ignore = "R1: kernel failure, see the comment"]
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
#[ignore = "R2: kernel failure, see the comment"]
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
#[ignore = "R4: kernel failure, see the comment"]
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
#[ignore = "R5: kernel failure, see the comment"]
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
#[ignore = "R6: kernel failure, see the comment"]
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
#[ignore = "R7: kernel failure, see the comment"]
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
#[ignore = "R8: kernel failure, see the comment"]
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
#[ignore = "R13: kernel failure, see the comment"]
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
#[ignore = "R14: kernel failure, see the comment"]
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
#[ignore = "R15: kernel failure, see the comment"]
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
#[ignore = "R16: kernel failure, see the comment"]
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
#[ignore = "R17: kernel failure, see the comment"]
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
#[ignore = "R19: kernel failure, see the comment"]
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
#[ignore = "R20: kernel failure, see the comment"]
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
#[ignore = "R21: kernel failure, see the comment"]
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
#[ignore = "R22: kernel failure, see the comment"]
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
#[ignore = "R23: kernel failure, see the comment"]
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
#[ignore = "R25: kernel failure, see the comment"]
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
#[ignore = "R26: kernel failure, see the comment"]
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
#[ignore = "R27: kernel failure, see the comment"]
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
#[ignore = "R29: kernel failure, see the comment"]
fn nary_fillet_after_circular_pattern() {
    holds(
        "\
spindle: extrude sym 30 circle 0 0 21
spider: extrude 6 z 0 0 1 x 0.809017 0.587785 0 at 0 0 -15 rrect 0 -7.5 62 15 6
spiders: pattern spider circ 0 0 0 0 0 1 5
fillet 3 (edges between spider spindle concave)\n",
        true,
    );
}

/// a block, two separate foot pads, one fillet 5 where both pads meet the block: seq valid; n-ary
/// fails MissingEdge on the fillet (one pad per fillet line passes; a lin pattern of one pad fails
/// the same). Root of R29 and of the gearbox and frame n-ary failures
#[test]
#[ignore = "R30: kernel failure, see the comment"]
fn nary_fillet_after_linear_pattern() {
    holds(
        "\
box: extrude 95 rect -120 -65 240 130
a: extrude 16 rect -96 -93 44 34
b: extrude 16 rect 52 -93 44 34
fillet 5 (edges between a box concave) (edges between b box concave)\n",
        true,
    );
}

/// a three-profile plain loft arm into a pedal boss, the boss bored and threaded 9/16-20 left: seq
/// valid; n-ary fails Boolean Unsupported(surface pair without a closed form) (n-ary twin of R13;
/// two profiles pass both ways)
#[test]
#[ignore = "R31: kernel failure, see the comment"]
fn nary_thread_near_three_profile_loft() {
    holds(
        "\
arm: loft profile yz at 12 0 0 rrect -15 -12 30 24 6 profile yz at 85 0 0 rrect -11.5 -9.5 23 19 5 profile yz at 166 0 0 rrect -9 -7.5 18 15 4
pedal: extrude sym 18 at 170 0 0 circle 0 0 12
hole 12.913 -xy at 170 0 9 thru
thread 9/16-20 -xy at 170 0 9 int 18 left chamfer\n",
        true,
    );
}

/// smooth spiral loft volute, a cover-flange disc through it, the smooth core loft cut, then one
/// discharge-flange bolt hole far from the rest: seq valid; n-ary fails Boolean Unsupported(surface
/// pair without a closed form) at the hole (without the disc or without the core it passes)
#[test]
#[ignore = "R32: kernel failure, see the comment"]
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
#[ignore = "R33: kernel failure, see the comment"]
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
#[ignore = "R34: kernel failure, see the comment"]
fn nary_bb_cluster_fillet() {
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
hbx = reach + htl*hdx
hbz = stack + htl*hdz
dtx = hbx - 45*hdx
dtz = hbz - 45*hdz
dtl = sqrt(dtx^2 + dtz^2)
bb: extrude sym 68 -xz circle 0 0 21
st: extrude stl z -cos(sta) 0 sin(sta) circle 0 0 15.9
dt: extrude dtl z dtx/dtl 0 dtz/dtl circle 0 0 17.5
fillet 3 (edges between st bb concave) (edges between dt bb concave) (edges between st dt concave)\n",
        true,
    );
}

/// down tube, tapered head tube (revolved cone), top tube, fillet 3 where the top tube meets the
/// head tube: seq valid; n-ary fails MissingEdge (without the down tube it passes)
#[test]
#[ignore = "R35: kernel failure, see the comment"]
fn nary_fillet_top_tube_head_tube() {
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
        true,
    );
}

/// a disc and a block flush with both its faces: seq leaves 12 faces (each end face split in two),
/// n-ary 10 (merged); the two paths give different topology for the same part. In the benchmarks:
/// gearbox 359 vs 338, crank 145 vs 144, conrod 158 vs 154 faces. Likely the same root as R24, R26
/// and union-coplanar-faces-unmerged
#[test]
#[ignore = "R36: kernel failure, see the comment"]
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
#[ignore = "R37: kernel failure, see the comment"]
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
