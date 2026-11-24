//! Contour leads and entry position (Fusion parity, 2D contour's Linking tab): tangent arc or
//! line leads in and out on the air side, the loop started nearest `entry:`, an overlap past
//! the start; every move certified like the rest.
mod common;
use common::{dir, doc};
use forge_cam::ops::{PMove, Phase};

const PRE: &str = "skim = facing(op1, tool: t1, to: 18, step: 1)\ntop = clear(op1, tool: t1, from: 18, to: 12, step: 3, stepover: 4)\n";

fn prof(extra: &str) -> forge_cam_lang::Cam {
    let body = format!(
        "{PRE}prof = contour(op1, tool: t1, side: \"outside\", from: 12, to: -0.5, step: 4{extra})\nprog = program(op1, post: \"fanuc\", number: 1)\n"
    );
    forge_cam_lang::run(&doc(&body), &dir())
}

fn built(cam: &forge_cam_lang::Cam) -> &forge_cam_lang::OpOut {
    assert!(cam.errors.is_empty(), "{:?}", cam.errors);
    let o = cam.ops.iter().find(|o| o.name == "prof").unwrap();
    assert_eq!(o.status, "built", "{:?} {:?}", o.refusal, o.fix);
    o
}

/// Every plunge starts the level off the loop: the first cut after it is the lead.
#[test]
fn arc_leads_come_down_in_the_air_and_are_certified() {
    let cam = prof("");
    let plain = built(&cam).paths[0].moves.clone();
    let cam = prof(", lead: \"arc\", lead_radius: 4, entry: (0, -100), overlap: 1");
    let o = built(&cam);
    let mv = &o.paths[0].moves;
    assert_ne!(mv, &plain);
    let mut plunges = 0;
    for (i, m) in mv.iter().enumerate() {
        if let PMove::Line {
            to,
            phase: Phase::Plunge,
            ..
        } = m
        {
            plunges += 1;
            // the lead-in arc follows the plunge
            assert!(matches!(mv[i + 1], PMove::Arc { .. }), "{:?}", mv[i + 1]);
            // the plunge is at the entry side (y < 0): the loop starts nearest (0, -100)
            assert!(to[1] < 0.0, "{to:?}");
        }
    }
    assert!(plunges >= 4, "one per level");
    let (txt, _) = cam.program_text().unwrap();
    assert!(txt.contains("(prof contour)"));
}

#[test]
fn line_leads_and_refusals() {
    let o_ok = prof(", lead: \"line\", lead_length: 3");
    built(&o_ok);
    for (extra, msg) in [
        (", lead_radius: 3", "needs a lead"),
        (
            ", lead: \"line\", lead_radius: 3",
            "a line lead takes `lead_length:` only",
        ),
        (", lead: \"arc\", lead_sweep: 200", "lead_sweep:"),
        (", lead: \"zigzag\"", "lead: \"none\", \"arc\" or \"line\""),
        (", entry: 3", "entry: a point"),
    ] {
        let cam = prof(extra);
        assert!(
            cam.errors.iter().any(|e| e.contains(msg)),
            "{extra}: {:?}",
            cam.errors
        );
    }
    let cam = forge_cam_lang::run(
        &doc("top = clear(op1, tool: t1, from: 18, to: 12, lead: \"arc\")\n"),
        &dir(),
    );
    assert!(
        cam.errors
            .iter()
            .any(|e| e.contains("`lead` is not an argument of `clear`")),
        "{:?}",
        cam.errors
    );
}

/// A lead that does not fit (an arc bigger than the hole it is in) cuts the part: the checks
/// refuse the op, as for any other move.
#[test]
fn a_lead_into_the_part_is_refused() {
    let body = |extra: &str| {
        format!(
            "{PRE}prof = contour(op1, tool: t2, side: \"inside\", from: 12, to: 6, step: 3{extra})\nprog = program(op1, post: \"fanuc\", number: 1)\n"
        )
    };
    let cam = forge_cam_lang::run(&doc(&body(", lead: \"arc\", lead_radius: 0.2")), &dir());
    let o = cam.ops.iter().find(|o| o.name == "prof").unwrap();
    assert_eq!(o.status, "built", "{:?}", o.refusal);
    let cam = forge_cam_lang::run(
        &doc(&body(", lead: \"arc\", lead_radius: 60, lead_sweep: 180")),
        &dir(),
    );
    let o = cam.ops.iter().find(|o| o.name == "prof").unwrap();
    assert_eq!(o.status, "refused", "{:?}", o.paths.len());
    eprintln!("{:?} {:?}", o.refusal, o.fix);
}

/// Holding tabs (Fusion's 2D contour Tabs): on the levels below the tabs' top the tool lifts over
/// each tab; certified; the stock keeps the bridges (the band counts as cleared only to their top).
#[test]
fn tabs_lift_on_the_low_levels() {
    let lifted = |cam: &forge_cam_lang::Cam, ztab: f64| {
        let o = built(cam);
        o.paths[0]
            .moves
            .iter()
            .filter(
                |m| matches!(m, PMove::Line { to, phase: Phase::Link, .. } if (to[2] - ztab).abs() < 1e-9),
            )
            .count()
    };
    // bottom -0.5, tabs 2 high: their top at 1.5; levels 12 → -0.5 in 4 steps: two below it? no,
    // only the last (-0.5) is under 1.5
    let cam = prof(", tabs: 4, tab_width: 6, tab_height: 2");
    assert_eq!(lifted(&cam, 1.5), 4);
    let cam = prof(", tabs: 3, tab_height: 2, tab_shape: \"triangle\", lead: \"arc\"");
    built(&cam);
    let cam = prof(", tab_at: [(0, 50), (0, -50)], tab_height: 2");
    assert_eq!(lifted(&cam, 1.5), 2);
    for (extra, msg) in [
        (", tab_width: 3", "needs tabs"),
        (", tabs: 2, tab_spacing: 9", "one of `tabs:`"),
        (", tabs: 2, tab_height: 40", "tab_height:"),
        (", tabs: 200", "tabs: a whole number"),
        (", tabs: 60, tab_width: 20", "tabs overlap"),
    ] {
        let cam = prof(extra);
        let msgs: Vec<String> = cam
            .errors
            .iter()
            .cloned()
            .chain(cam.ops.iter().filter_map(|o| o.refusal.clone()))
            .collect();
        assert!(msgs.iter().any(|e| e.contains(msg)), "{extra}: {msgs:?}");
    }
}
