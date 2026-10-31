//! Fillets on real parts that once failed: each builds valid, sequentially and n-ary.

use crate::model::Model;
use forge_model::Part;

fn valid(src: &str) {
    let m = Model::new(src);
    if let Some(f) = &m.error {
        panic!("line {}: {}\n{src}", m.lines[f.at].no, f.msg);
    }
    let part = Part {
        features: m.features.clone(),
    };
    for (how, built) in [("seq", part.regenerate()), ("n-ary", part.regenerate_nary(2))] {
        let b = built.unwrap_or_else(|e| panic!("{how}: {e:?}\n{src}"));
        let problems = forge_topo::check(&b);
        assert!(problems.is_empty(), "{how}: {problems:?}\n{src}");
    }
}

#[test]
fn shell_after_a_variable_fillet() {
    // the blend's run-outs past its ends are tighter than the shell is thick, the face is not;
    // its offset springs meet the offset top and sides tangentially
    valid("extrude 14 rect 0 -15 170 30\nfillet 4..3 (edges top line along +x)\nshell 1.6 (faces bottom)\n");
}

#[test]
fn round_a_tapered_arm_with_its_tip() {
    // the sides and the tip arc are one tangent chain round the tip, so it comes back beside the
    // root corner it leaves: its middle is halfway along it, not across
    valid(
        "extrude 14 loop line 0 -15 169.905858 -15.999723 arc 170 0 16 -90.337123 180.674246 line 169.905858 15.999723 0 15 line 0 15 0 -15\nfillet 2 (edges top)\n",
    );
}
