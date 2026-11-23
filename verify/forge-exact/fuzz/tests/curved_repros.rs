//! Disagreements between the kernel and the exact curved reference (ForgeExact/QRef), each a
//! minimal repro: a degenerate pair of the reliability scoreboard (two features), kept as a
//! failing test until the kernel is fixed (`cargo test -p forge-exact-fuzz --test curved_repros
//! -- --ignored`, after `lake build` in verify/forge-exact). Judged under §6.1's snap rule
//! (research/kernel-audit/exact-comparison-curved.md). The first two are checks that pass: the
//! reference and the kernel agree there.

use forge_exact_fuzz::curved::judge_curved;
use forge_exact_fuzz::{Class, Reference};
use forge_model::Part;

fn degenerate(name: &str) -> Part {
    let part = forge_model::adversarial::degenerate_parts()
        .into_iter()
        .find(|(n, _)| n == name)
        .unwrap_or_else(|| panic!("no degenerate part {name}"))
        .1;
    forge_model::adversarial::canonical(&part)
}

/// The part's sequential and n-ary regens judged under the snap rule: every one must agree.
fn agrees_curved(part: &Part) {
    let mut reference = Reference::start();
    let mut bad = Vec::new();
    for (path, c, ans) in judge_curved(&mut reference, part, "snap") {
        if !matches!(c, Class::Agree) {
            bad.push(format!("{path}: {} {}\nreference: {ans:?}", c.key(), c.detail()));
        }
    }
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

/// Two parallel cylinders touching along a line (plane_cylinder/tangent_wall_outside/Add): the
/// union is non-manifold along it (the reference proves the touch with a sum-of-squares
/// certificate), and the kernel refuses with ZeroThickness.
#[test]
#[cfg_attr(
    not(feature = "reference"),
    ignore = "asks the Lean reference: lake build in verify/forge-exact, then cargo test -p forge-exact-fuzz --features reference"
)]
fn cylinder_touching_a_wall_union_is_refused() {
    agrees_curved(&degenerate("plane_cylinder/tangent_wall_outside/Add"));
}

/// A cylinder cut by an identical one: empty, and the kernel says so.
#[test]
#[cfg_attr(
    not(feature = "reference"),
    ignore = "asks the Lean reference: lake build in verify/forge-exact, then cargo test -p forge-exact-fuzz --features reference"
)]
fn identical_cylinder_cut_is_empty() {
    agrees_curved(&degenerate("cylinder_cylinder/identical/Cut"));
}

/// A ball resting on the base's top face, touching it at one point (plane_sphere/tangent_top_above
/// /Add). The union is non-manifold at (0, 0, 0): the two solids meet at a point. The kernel
/// builds a body of 32 523.6 mm³ that passes `forge_topo::check`, as it did for a cube on a
/// corner vertex (exact-comparison.md). Expected: ZeroThickness there.
#[test]
#[ignore = "exact curved reference: a ball touching the top at a point is built, not refused ZeroThickness"]
fn ball_resting_on_the_top_is_non_manifold() {
    agrees_curved(&degenerate("plane_sphere/tangent_top_above/Add"));
}

/// A ball inside the base touching its top face from below, cut away
/// (plane_sphere/tangent_top_inside/Cut). What is left pinches to nothing at (0, 0, 0): the
/// material between the cavity and the top is a cusp ring. The kernel builds 31 476.4 mm³.
#[test]
#[ignore = "exact curved reference: a cavity touching the top from inside at a point is built"]
fn cavity_touching_the_top_from_inside_is_non_manifold() {
    agrees_curved(&degenerate("plane_sphere/tangent_top_inside/Cut"));
}

/// Two balls touching from outside (sphere_sphere/tangent_outside/Add): the union is two
/// solids meeting at a point. The kernel builds 4 712.4 mm³.
#[test]
#[ignore = "exact curved reference: two balls touching at a point are united"]
fn balls_touching_at_a_point_are_non_manifold() {
    agrees_curved(&degenerate("sphere_sphere/tangent_outside/Add"));
}

/// A cone standing with its apex on the base's corner vertex, outside it
/// (plane_cone/apex_on_vertex/Add). The union meets at the apex alone: the reference's
/// tangent-cone check at the apex finds the kept directions in two pieces. The kernel builds
/// 32 261.8 mm³.
#[test]
#[ignore = "exact curved reference: a cone touching the base at its apex is united"]
fn cone_apex_on_a_corner_is_non_manifold() {
    agrees_curved(&degenerate("plane_cone/apex_on_vertex/Add"));
}

/// A ball centred on the cylinder's top rim (cylinder_sphere/centre_on_rim/Add). The union is
/// an ordinary solid (the reference certifies interior boxes, finds no contact, and its volume
/// estimate is 6 806.8 mm³). The kernel refuses Unsupported(SplitThroughSingular) on the sphere,
/// whose pole lies on the rim. Cut and Keep refuse alike.
#[test]
#[ignore = "exact curved reference: a ball centred on a cylinder's rim is refused Unsupported(SplitThroughSingular)"]
fn ball_centred_on_a_rim_builds() {
    agrees_curved(&degenerate("cylinder_sphere/centre_on_rim/Add"));
}
