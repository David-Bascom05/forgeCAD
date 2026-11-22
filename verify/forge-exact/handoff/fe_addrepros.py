import sys
sp = sys.argv[1]
crate = sys.argv[2]
cw = open(sp + 'fe_cw1.rs').read().strip()
fg = open(sp + 'fe_lang28.fg').read().rstrip('\n') + '\n'
p = crate + '/tests/repros.rs'
s = open(p).read()
s = s.replace('''use forge_exact_fuzz::{Class, Reference, judge_kernel, nary, reference_input, sequential};
use forge_model::Part;
use forge_topo::Body;''', '''use forge_exact_fuzz::{Class, Reference, judge_kernel, nary, reference_input, sequential};
use forge_math::Point2;
use forge_model::adversarial::fr;
use forge_model::{Combine, Extent, Feature, Part};
use forge_ops::{Profile, ProfileLoop, Segment};
use forge_topo::Body;''')
i = s.index("/// coincident_walls 1, its blocks")
s = s[:i] + '''/// coincident_walls 1, its blocks, shrunk to five features: a pocket floor at z = 7, a deeper
/// pocket shifted by half its width with walls exactly on the first's, and a block added back
/// exactly onto the floor. The reference, with no decision within tolerance, gives a valid solid;
/// the kernel refuses ZeroThickness at (29.78, 23.23, 7), on the coincident floor: the refill's
/// bottom and the pocket floor are one plane and should merge.
#[test]
#[ignore = "exact reference: ZeroThickness refused where the exact result is a valid solid"]
fn refill_onto_a_coincident_floor_builds() {
    let part = ''' + cw.replace('\n', '\n    ') + ''';
    agrees(&part);
}

/// forge_lang::fuzz seed 28, its flat statements, shrunk: a block, four-sided prisms extruded to
/// the next face and patterned, then a pocket cut across a wall. The sequential regen agrees with
/// the reference (a solid); the n-ary regen refuses NotClosed: "the edge from (-24.199, 84.300,
/// 17.010) to (-29.499, 84.300, 21.220) is on 1 face(s), not 2" (a face lost in the n-ary pass).
#[test]
#[ignore = "exact reference: the n-ary regen loses a face where the sequential one builds the exact solid"]
fn nary_keeps_every_face_of_a_patterned_prism_cut() {
    let src = r#"''' + fg + '''"#;
    agrees(&forge_lang::fuzz::build(src).part());
}
'''
open(p, 'w').write(s)
p = crate + '/Cargo.toml'
s = open(p).read()
if 'dev-dependencies' not in s:
    s = s.replace('[lints]', '''[dev-dependencies]
# the repros' part literals
forge-ops = { path = "../../../kernel/forge-ops" }

[lints]''')
open(p, 'w').write(s)
