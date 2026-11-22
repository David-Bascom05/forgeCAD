
/// The near-coincident part (research/kernel-audit/near-coincident.md; 94fc9fb6's case): a box
/// added on a notched plate, its bottom 4e-16 inside a tolerance under the top (adv
/// near_tolerance 2, its first three features), the plate's stage moved by ulps as the `ulp`
/// property moves it. Under the tolerance rule (§6.1, rule `snap`) the box's bottom and the top
/// are one carrier and every variant has one answer: a solid, V26 E40 F16 S1, 23439.9988 mm³.
/// The kernel agrees unmoved, but of the 400 ulp seeds 301 refuse Internal SplitCrossed (imprint's
/// on-edge test re-decided across the gap) and 32 build an invalid body, EdgeDegenerate
/// (place_cuts' zero-length edge): seeds 0 and 13 here.
#[test]
#[ignore = "exact reference: a stage moved by ulps refuses SplitCrossed or builds EdgeDegenerate where the tolerance rule gives one solid"]
fn notched_top_under_ulps_has_one_answer() {
    let g = forge_model::adversarial::families()
        .into_iter()
        .find(|(f, _)| *f == "near_tolerance")
        .expect("the family")
        .1;
    let mut part = forge_model::adversarial::canonical(&g(2));
    part.features.truncate(3);
    let mut reference = Reference::start();
    let ans = reference.ask(&reference_input_rule(&part, "snap").expect("flat"));
    assert_eq!(ans.verdict, "solid", "{:?}", ans.notes);
    let mut bad = Vec::new();
    for seed in [0, 13] {
        let k = nudged_last(&part, seed);
        let c = judge_kernel(&mut reference, &part, &ans, &k);
        if !matches!(c, Class::Agree) {
            bad.push(format!("seed {seed}: {} {}", c.key(), c.detail()));
        }
    }
    assert!(bad.is_empty(), "reference: solid volume {}\n{}", ans.volume, bad.join("\n"));
}
