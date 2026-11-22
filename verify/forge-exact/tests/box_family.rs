//! The box-box family's coverage (research/kernel-audit/coverage.md): every consistent sign pattern
//! of two axis-aligned boxes and an op, through `forge_bool::boolean`, against the exact answer.
//! The Lean model (research/kernel-audit/coverage/Coverage/Theorems.lean, `gaps`) lists the same
//! eight gaps these tests mark.
use forge_bool::Op;
use forge_exact::{Outcome, Pattern, Rel, exact, kernel};

const UNIT: f64 = 10.0;

/// The gap patterns (`Coverage.gapLabels`): two boxes meeting at one corner, united.
const GAPS: [[&str; 3]; 8] = [
    ["meets", "meets", "meets"],
    ["meets", "meets", "metBy"],
    ["meets", "metBy", "meets"],
    ["meets", "metBy", "metBy"],
    ["metBy", "meets", "meets"],
    ["metBy", "meets", "metBy"],
    ["metBy", "metBy", "meets"],
    ["metBy", "metBy", "metBy"],
];

fn pattern(names: [&str; 3], op: Op) -> Pattern {
    Pattern {
        rel: names.map(|n| Rel::by_name(n).expect("a relation")),
        op,
    }
}

fn is_gap(p: &Pattern) -> bool {
    p.op == Op::Union && GAPS.iter().any(|g| (0..3).all(|k| p.rel[k].name == g[k]))
}

/// Every pattern off the gaps: the kernel's outcome is the exact one (a valid body of the exact
/// volume, Empty, or ZeroThickness exactly where the result is not a manifold).
#[test]
fn box_family_off_the_gaps_is_exact() {
    let mut wrong = Vec::new();
    for p in Pattern::all().iter().filter(|p| !is_gap(p)) {
        let (k, e) = (kernel(p, UNIT), exact(p, UNIT));
        if !k.agrees(&e) {
            wrong.push(format!("{}: kernel {}, exact {}", p.label(), k.word(), e.word()));
        }
    }
    assert!(wrong.is_empty(), "{} patterns: {:?}", wrong.len(), &wrong[..wrong.len().min(10)]);
}

fn corner_touch(names: [&str; 3]) {
    let p = pattern(names, Op::Union);
    assert_eq!(exact(&p, UNIT), Outcome::ZeroThickness);
    assert_eq!(kernel(&p, UNIT), Outcome::ZeroThickness, "{}", p.label());
}

// Two boxes meeting only at a corner, united: the union is not a manifold at the corner, so the
// boolean must refuse with ZeroThickness there. No flat-face site decides a corner touch: the
// four-use test (boolean.rs:1724-1737) sees edges only, and the touch test (1061-1094) is fed only by
// `touches` (2048-2101), which finds nothing between two planes. The corner's two vertices merge in
// `place_cuts` (4561-4644, an alias), `split_old_pinches` (631-653, from `general` 571) gives each fan
// its own vertex again, and a pinched body that `check` accepts is returned.
// The part: verify/forge-exact/parts/corner_touch.fg (the scoreboard's plane_plane/vertex_on_vertex/Add).
macro_rules! gap {
    ($name:ident, $x:literal, $y:literal, $z:literal) => {
        #[test]
        #[ignore = concat!(
            "coverage gap: ", $x, "/", $y, "/", $z,
            " union (corner touch) at kernel/forge-bool/src/boolean.rs:1724 (no corner test beside the four-use edge test; touches 2048 empty for planes)"
        )]
        fn $name() {
            corner_touch([$x, $y, $z]);
        }
    };
}

gap!(corner_touch_meets_meets_meets, "meets", "meets", "meets");
gap!(corner_touch_meets_meets_metby, "meets", "meets", "metBy");
gap!(corner_touch_meets_metby_meets, "meets", "metBy", "meets");
gap!(corner_touch_meets_metby_metby, "meets", "metBy", "metBy");
gap!(corner_touch_metby_meets_meets, "metBy", "meets", "meets");
gap!(corner_touch_metby_meets_metby, "metBy", "meets", "metBy");
gap!(corner_touch_metby_metby_meets, "metBy", "metBy", "meets");
gap!(corner_touch_metby_metby_metby, "metBy", "metBy", "metBy");

/// The gaps are what they are today: a body, not a refusal (so a fix shows up here first).
#[test]
fn corner_touch_today_makes_a_pinched_body() {
    for g in GAPS {
        let p = pattern(g, Op::Union);
        assert!(matches!(kernel(&p, UNIT), Outcome::Ok(_)), "{}", p.label());
    }
}
