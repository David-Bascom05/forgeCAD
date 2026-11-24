//! Cutter radius compensation on the SINUMERIK 840D control (src/comp.rs; PGsl 03/2013 ch. 10):
//! the tool centre's path against hand-worked equidistants, approach and retract (NORM), outside
//! corners (G450 circle, G451 intersection), inside corners, an arc; and what is refused.
use forge_machine::comp;
use forge_machine::gcode::{self, Block, Centre, Control, Ev};
use forge_machine::library::builtin_named;
use forge_machine::sim::{self, Cut, Setup, SimTool};

const U: f64 = 10000.0; // the control's 4 decimals

fn control(m: &forge_machine::MachineDef) -> Control {
    let mut c = Control::for_dialect(m, "siemens-840d", 1).unwrap();
    c.tool_names = vec![("E10".into(), 1)];
    c
}

fn program(modes: &str, side: &str, contour: &str) -> String {
    format!(
        "N10 G17 G710 G90 G94 {modes}\nN20 T=\"E10\"\nN30 M6\nN40 G0 X-20. Y-20. Z5. D1\nN50 G1 Z-2. F500.\n\
         N60 {side} G1 X0 Y0\n{contour}N200 G40 G1 X-20. Y-20.\nN210 G0 Z5.\n"
    )
}

/// The tool centre's moves in the plane from the G41/G42 block on: (kind, x, y, centre).
/// (kind: R rapid, L line, W cw arc, C ccw arc; x; y; an arc's centre)
type Move = (char, f64, f64, Option<(f64, f64)>);

fn centre_path(text: &str, r: f64) -> Result<Vec<Move>, (usize, String)> {
    let m = builtin_named("generic-vmc3").unwrap();
    let c = control(&m);
    let (bs, _) = gcode::run(text, &c).map_err(|e| (e.line, e.what))?;
    let ix = [c.idx('X').unwrap(), c.idx('Y').unwrap()];
    let out: Vec<Block> = comp::apply(&bs, ix, 4, None, c.comp.unwrap(), &|t| (t == 1).then_some(r))?;
    // the text's lines from the G41 block (6) to the G40 block (the last but one)
    let last = text.lines().count() - 1;
    Ok(out
        .iter()
        .filter(|b| b.line >= 6 && b.line <= last)
        .filter_map(|b| {
            let p = |to: &Vec<Option<i64>>| (to[ix[0]].unwrap() as f64 / U, to[ix[1]].unwrap() as f64 / U);
            match &b.ev {
                Ev::Rapid(to) => Some(('R', p(to).0, p(to).1, None)),
                Ev::Feed { to, .. } => Some(('L', p(to).0, p(to).1, None)),
                Ev::Arc {
                    to,
                    centre: Centre::Abs([a, cc]),
                    cw,
                    ..
                } => Some((
                    if *cw { 'W' } else { 'C' },
                    p(to).0,
                    p(to).1,
                    Some((*a as f64 / U, *cc as f64 / U)),
                )),
                _ => None,
            }
        })
        .collect())
}

// a 40 x 30 rectangle, clockwise from the origin: G41 puts the tool outside
const CW_SQUARE: &str = "N70 Y30.\nN80 X40.\nN90 Y0\nN100 X0\n";

#[test]
fn outside_corners_on_a_circle_with_g450() {
    let p = centre_path(&program("NORM G450", "G41", CW_SQUARE), 5.0).unwrap();
    assert_eq!(
        p,
        vec![
            // NORM: straight to the first block's start, perpendicular to it
            ('L', -5.0, 0.0, None),
            ('L', -5.0, 30.0, None),
            ('W', 0.0, 35.0, Some((0.0, 30.0))),
            ('L', 40.0, 35.0, None),
            ('W', 45.0, 30.0, Some((40.0, 30.0))),
            ('L', 45.0, 0.0, None),
            ('W', 40.0, -5.0, Some((40.0, 0.0))),
            ('L', 0.0, -5.0, None),
            // NORM: from the last block's end, perpendicular, to the uncompensated end
            ('L', -20.0, -20.0, None),
        ]
    );
}

#[test]
fn outside_corners_to_the_intersection_with_g451() {
    let p = centre_path(&program("NORM G451", "G41", CW_SQUARE), 5.0).unwrap();
    assert_eq!(
        p,
        vec![
            ('L', -5.0, 0.0, None),
            ('L', -5.0, 35.0, None),
            ('L', 45.0, 35.0, None),
            ('L', 45.0, -5.0, None),
            ('L', 0.0, -5.0, None),
            ('L', -20.0, -20.0, None),
        ]
    );
}

#[test]
fn inside_corners_meet_and_g42_is_the_other_side() {
    // the same clockwise rectangle with G42: the tool inside (a pocket's wall)
    let p = centre_path(&program("NORM G450", "G42", CW_SQUARE), 5.0).unwrap();
    assert_eq!(
        p,
        vec![
            ('L', 5.0, 0.0, None),
            ('L', 5.0, 25.0, None),
            ('L', 35.0, 25.0, None),
            ('L', 35.0, 5.0, None),
            ('L', 0.0, 5.0, None),
            ('L', -20.0, -20.0, None),
        ]
    );
    // a negative register is the other side (PGsl 10.1 p. 263)
    assert_eq!(
        p,
        centre_path(&program("NORM G450", "G41", CW_SQUARE), -5.0).unwrap()
    );
}

#[test]
fn an_arc_keeps_its_centre_and_a_wear_register_moves_the_path_by_the_wear() {
    // up the Y axis, a clockwise quarter about (20, 20), along +X: tangent throughout
    let contour = "N70 Y20.\nN80 G2 X20. Y40. I20. J0\nN90 G1 X40.\n";
    let p = centre_path(&program("NORM G450", "G41", contour), 5.0).unwrap();
    assert_eq!(
        p,
        vec![
            ('L', -5.0, 0.0, None),
            ('L', -5.0, 20.0, None),
            ('W', 20.0, 45.0, Some((20.0, 20.0))),
            ('L', 40.0, 45.0, None),
            ('L', -20.0, -20.0, None),
        ]
    );
    // computer mode: CAM posted the centre path, the register holds the wear (-0.02 here)
    let w = centre_path(&program("NORM G450", "G41", contour), -0.02).unwrap();
    assert_eq!(w[1], ('L', 0.02, 20.0, None));
    assert_eq!(w[2], ('W', 20.0, 39.98, Some((20.0, 20.0))));
}

#[test]
fn compensation_simulates() {
    let m = builtin_named("generic-vmc3").unwrap();
    let mut s = Setup::new(&m, control(&m)).unwrap();
    s.wcs = vec![(54, vec![0.0, 0.0, -500.0])];
    s.tools = vec![SimTool {
        t: 1,
        name: "E10".into(),
        tip: [0.0, 0.0, -100.0],
        body: vec![(5.0, 0.0, 25.0), (12.0, 25.0, 80.0)],
        cut: Cut::Mill { r: 5.0, corner: 0.0 },
    }];
    s.h = vec![(1, 100.0)];
    let text = program("NORM G450", "G41", CW_SQUARE);
    // no register: refused
    let e = sim::simulate(&text, &s).unwrap_err();
    assert!(e.to_string().contains("radius"), "{e}");
    s.radius = vec![(1, 5.0)];
    let tl = sim::simulate(&text, &s).unwrap();
    // the three corner circles are arcs of the timeline
    let arcs = tl
        .segments
        .iter()
        .filter(|g| matches!(g.path, sim::Path::Arc { .. }))
        .count();
    assert_eq!(arcs, 3);
}

#[test]
fn what_is_not_modelled_is_refused() {
    let refused = |text: String, r: f64| centre_path(&text, r).unwrap_err().1;
    for (modes, side, contour, r, word) in [
        ("G450", "G41", CW_SQUARE, 5.0, "NORM"),
        ("NORM", "G41", CW_SQUARE, 5.0, "G450 or G451"),
        ("KONT G450", "G41", CW_SQUARE, 5.0, "KONT"),
        // a slot 8 wide with a 5 mm radius inside: the walls' equidistants cross
        (
            "NORM G450",
            "G42",
            "N70 Y30.\nN80 X8.\nN90 Y0\nN100 X0\n",
            5.0,
            "look-ahead",
        ),
        ("NORM G450", "G41", "N70 Y30.\nN80 G0 X40.\n", 5.0, "rapid"),
        ("NORM G450", "G41", "N70 Y30.\nN80 Z-3.\n", 5.0, "no X/Y motion"),
        (
            "NORM G450",
            "G41",
            "N70 Y30.\nN80 D0\n",
            5.0,
            "D or a tool change",
        ),
        // a clockwise arc of radius 3 inside a 5 mm tool's path: G42 puts the tool inside it
        (
            "NORM G450",
            "G42",
            "N70 Y30.\nN80 G2 X6. Y30. I3. J0\n",
            5.0,
            "radius",
        ),
    ] {
        let what = refused(program(modes, side, contour), r);
        assert!(what.contains(word), "{modes} {side}: {what}");
    }
    // G41 on an arc: the approach must be G0/G1 (PGsl 10.1 p. 258)
    let text = program("NORM G450", "G41", CW_SQUARE)
        .replace("N60 G41 G1 X0 Y0", "N60 G41 G2 X0 Y0 I10. J10.")
        .replace("N70 Y30.", "N70 G1 Y30.");
    let what = refused(text, 5.0);
    assert!(what.contains("G0 or G1"), "{what}");
}

#[test]
fn a_line_meeting_an_arc_at_a_corner() {
    // up to (0, 20), then clockwise about (10, 10) (radius √200) to (20, 20): a right turn of 45°
    let contour = "N70 Y20.\nN80 G2 X20. Y20. I10. J-10.\n";
    let close = |a: (char, f64, f64), b: (char, f64, f64)| {
        a.0 == b.0 && (a.1 - b.1).abs() < 1e-4 && (a.2 - b.2).abs() < 1e-4
    };
    let r2 = 200f64.sqrt();
    // G42: the tool inside, the arc's radius √200 - 5; the corner where x = 5 meets it:
    // y = 10 + √((√200 - 5)² - 25)
    let p = centre_path(&program("NORM G450", "G42", contour), 5.0).unwrap();
    let ri = r2 - 5.0;
    let want = [
        ('L', 5.0, 0.0),
        ('L', 5.0, 10.0 + (ri * ri - 25.0).sqrt()),
        ('W', 10.0 + ri / 2f64.sqrt(), 10.0 + ri / 2f64.sqrt()),
        ('L', -20.0, -20.0),
    ];
    assert_eq!(p.len(), want.len(), "{p:?}");
    for (g, w) in p.iter().zip(want) {
        assert!(close((g.0, g.1, g.2), w), "{g:?} vs {w:?}");
    }
    // G41 with G451: the tool outside, radius √200 + 5; x = -5 meets it at
    // y = 10 + √((√200 + 5)² - 225)
    let p = centre_path(&program("NORM G451", "G41", contour), 5.0).unwrap();
    let ro = r2 + 5.0;
    let want = [
        ('L', -5.0, 0.0),
        ('L', -5.0, 10.0 + (ro * ro - 225.0).sqrt()),
        ('W', 10.0 + ro / 2f64.sqrt(), 10.0 + ro / 2f64.sqrt()),
        ('L', -20.0, -20.0),
    ];
    assert_eq!(p.len(), want.len(), "{p:?}");
    for (g, w) in p.iter().zip(want) {
        assert!(close((g.0, g.1, g.2), w), "{g:?} vs {w:?}");
    }
    // G41 with G450: a circle of radius 5 about (0, 20) from (-5, 20) to the arc's start offset
    let p = centre_path(&program("NORM G450", "G41", contour), 5.0).unwrap();
    let s = 5.0 / 2f64.sqrt();
    assert!(close((p[2].0, p[2].1, p[2].2), ('W', -s, 20.0 + s)), "{p:?}");
    assert_eq!(p[2].3, Some((0.0, 20.0)));
}

// ---------------------------------------------------------------------------------------------
// LinuxCNC (its interpreter's documented source, v2.9.9 interp_convert.cc; the dialect's cites)

fn lcnc_path(contour: &str, exit: &str, r: f64) -> Result<Vec<Move>, (usize, String)> {
    let m = builtin_named("generic-vmc3").unwrap();
    let c = Control::for_dialect(&m, "linuxcnc", 1).unwrap();
    let cm = c.comp.expect("linuxcnc cites compensation");
    assert!(!cm.entry_norm && cm.corner_arc && cm.tol == 0.05);
    let text = format!(
        "G17 G21 G90 G94\nG0 X-20. Y-20. Z5.\nG1 Z-2. F500.\nG41 D1\nG1 X0 Y0\n{contour}G40\n{exit}\n"
    );
    let (bs, _) = gcode::run(&text, &c).map_err(|e| (e.line, e.what))?;
    let ix = [c.idx('X').unwrap(), c.idx('Y').unwrap()];
    let u = 10f64.powi(c.decimals as i32);
    let out = comp::apply(&bs, ix, c.decimals, None, cm, &|t| (t == 1).then_some(r))?;
    Ok(out
        .iter()
        .filter(|b| b.line >= 5)
        .filter_map(|b| {
            let p = |to: &Vec<Option<i64>>| (to[ix[0]].unwrap() as f64 / u, to[ix[1]].unwrap() as f64 / u);
            match &b.ev {
                Ev::Rapid(to) => Some(('R', p(to).0, p(to).1, None)),
                Ev::Feed { to, .. } => Some(('L', p(to).0, p(to).1, None)),
                Ev::Arc {
                    to,
                    centre: Centre::Abs([a, cc]),
                    cw,
                    ..
                } => Some((
                    if *cw { 'W' } else { 'C' },
                    p(to).0,
                    p(to).1,
                    Some((*a as f64 / u, *cc as f64 / u)),
                )),
                _ => None,
            }
        })
        .collect())
}

fn near(got: &[Move], want: &[Move], tol: f64) {
    assert_eq!(got.len(), want.len(), "{got:?}\nvs {want:?}");
    for (g, w) in got.iter().zip(want) {
        let ok = g.0 == w.0
            && (g.1 - w.1).abs() <= tol
            && (g.2 - w.2).abs() <= tol
            && match (g.3, w.3) {
                (Some(a), Some(b)) => (a.0 - b.0).abs() <= tol && (a.1 - b.1).abs() <= tol,
                (None, None) => true,
                _ => false,
            };
        assert!(ok, "{g:?} vs {w:?}\nin {got:?}");
    }
}

#[test]
fn linuxcnc_entry_is_perpendicular_to_itself_and_meets_the_contour_at_a_corner() {
    let p = lcnc_path("Y30.\nX40.\nY0\nX0\n", "G0 X-20. Y-20.", 5.0).unwrap();
    // the entry from (-20, -20) to (0, 0) ends 5 to its left, at (-5, 5)/√2; with the first block
    // (up the Y axis) it makes an inside corner: both meet where x = -5 crosses y = x + 5√2
    let h = 5.0 * 2f64.sqrt();
    near(
        &p,
        &[
            ('L', -5.0, h - 5.0, None),
            ('L', -5.0, 30.0, None),
            ('W', 0.0, 35.0, Some((0.0, 30.0))),
            ('L', 40.0, 35.0, None),
            ('W', 45.0, 30.0, Some((40.0, 30.0))),
            ('L', 45.0, 0.0, None),
            ('W', 40.0, -5.0, Some((40.0, 0.0))),
            ('L', 0.0, -5.0, None),
            // after G40: straight from the compensated end to the programmed point
            ('R', -20.0, -20.0, None),
        ],
        1e-4,
    );
}

#[test]
fn linuxcnc_turns_within_its_tolerance_join_straight() {
    // up to (0, 30), then to (0.5, 40): a right turn of atan(0.05) = 0.04996 rad < 0.05
    let p = lcnc_path("Y30.\nX0.5 Y40.\n", "G0 X-20. Y-20.", 5.0).unwrap();
    let l = (0.25f64 + 100.0).sqrt();
    let end = (0.5 - 5.0 * 10.0 / l, 40.0 + 5.0 * 0.5 / l);
    assert_eq!(p.len(), 4, "{p:?}");
    near(
        &p[1..3],
        &[('L', -5.0, 30.0, None), ('L', end.0, end.1, None)],
        1e-4,
    );
    // to (0.6, 40): atan(0.06) = 0.0599 rad, past it: a circle about (0, 30)
    let p = lcnc_path("Y30.\nX0.6 Y40.\n", "G0 X-20. Y-20.", 5.0).unwrap();
    assert_eq!(p[2].0, 'W', "{p:?}");
    assert_eq!(p[2].3, Some((0.0, 30.0)));
}

#[test]
fn linuxcnc_exit_and_refusals() {
    // a Z-only move after G40 still goes back to the programmed X Y (it runs from the compensated
    // end to the programmed point)
    let p = lcnc_path("Y30.\n", "G0 Z5.", 5.0).unwrap();
    assert_eq!(p.last().unwrap().0, 'R');
    assert_eq!((p.last().unwrap().1, p.last().unwrap().2), (0.0, 30.0));
    // an entry not longer than the radius
    let e = lcnc_path("Y30.\n", "G0 X-20. Y-20.", 30.0).unwrap_err();
    assert!(e.1.contains("longer than the radius"), "{e:?}");
    // an arc after G40
    let e = lcnc_path("Y30.\n", "G2 X10. Y30. I5. J0", 5.0).unwrap_err();
    assert!(e.1.contains("straight"), "{e:?}");
    // Fanuc cites no corner behaviour: G41 refused by the shell
    let m = builtin_named("generic-vmc3").unwrap();
    let c = Control::for_dialect(&m, "fanuc", 1).unwrap();
    assert!(c.comp.is_none());
    assert!(
        gcode::run("G41 D1\n", &c)
            .unwrap_err()
            .what
            .contains("compensation")
    );
}

// ---------------------------------------------------------------------------------------------
// CAM-2D-FINISH's samples (2026-11-24): an 80 x 80 square about 0, flat-6 (r 3), climb, z -2,
// in its three modes. 1 is forge-cam's tool-centre path in 840D words; 2 and 3 are targets.

/// The 840D sample with a tool-change preamble (T, M6, D1 on the first Z move): the tool centre's
/// moves.
fn sample(body: &str, r: f64) -> Result<Vec<Move>, (usize, String)> {
    let m = builtin_named("generic-vmc3").unwrap();
    let c = control(&m);
    let pre = "T=\"E10\"\nM6\n";
    // the first move names Z after M6: it carries the D
    let body = body.replacen("Z5\n", "Z5 D1\n", 1);
    let text = format!("{pre}{body}");
    let (bs, _) = gcode::run(&text, &c).map_err(|e| (e.line, e.what))?;
    let ix = [c.idx('X').unwrap(), c.idx('Y').unwrap()];
    let out = comp::apply(&bs, ix, 4, None, c.comp.unwrap(), &|t| (t == 1).then_some(r))?;
    Ok(out
        .iter()
        .filter_map(|b| {
            let p = |to: &Vec<Option<i64>>| (to[ix[0]].unwrap() as f64 / U, to[ix[1]].unwrap() as f64 / U);
            match &b.ev {
                Ev::Feed { to, .. } => Some(('L', p(to).0, p(to).1, None)),
                Ev::Rapid(to) => Some(('R', p(to).0, p(to).1, None)),
                Ev::Arc {
                    to,
                    centre: Centre::Abs([a, cc]),
                    cw,
                    ..
                } => Some((
                    if *cw { 'W' } else { 'C' },
                    p(to).0,
                    p(to).1,
                    Some((*a as f64 / U, *cc as f64 / U)),
                )),
                _ => None,
            }
        })
        .collect())
}

const CONTOUR: &str = "G3 X-43 Y-40 I0 J3\nG1 Y40\nG2 X-40 Y43 I3 J0\nG1 X40\nG2 X43 Y40 I0 J-3\nG1 Y-40\nG2 X40 Y-43 I-3 J0\nG1 X-40\nG2 X-43 Y-40 I0 J3\nG3 X-46 Y-37 I-3 J0\n";

#[test]
fn sample_wear_mode_is_the_centre_path_offset_by_the_wear() {
    let computer = format!("G17 G90 G94 G710\nG0 X-46 Y-43 Z5\nG1 Z-2 F300\nG1 F800\n{CONTOUR}G0 Z5\n");
    let wear = format!(
        "G17 G90 G94 G710\nG0 X-52 Y-43 Z5\nG1 Z-2 F300\nNORM G450\nG41 G1 X-46 Y-43 F800\n{CONTOUR}G40 G1 X-52 Y-37\nG0 Z5\n"
    );
    let c = sample(&computer, 3.0).unwrap();
    // wear 0: the contour's moves are program 1's (between the approach and the exit)
    let w0 = sample(&wear, 0.0).unwrap();
    let core = |v: &[Move]| -> Vec<Move> {
        // from the lead-in arc's end through the lead-out arc
        let i = v.iter().position(|m| (m.1, m.2) == (-43.0, -40.0)).unwrap();
        let j = v.iter().position(|m| (m.1, m.2) == (-46.0, -37.0)).unwrap();
        v[i..=j].to_vec()
    };
    assert_eq!(core(&c), core(&w0));
    // wear +0.02: the wall lines 0.02 further out (the tool is on the left: outside)
    let w = sample(&wear, 0.02).unwrap();
    assert!(w.iter().any(|m| *m == ('L', -43.02, 40.0, None)), "{w:?}");
    assert!(w.iter().any(|m| *m == ('L', 40.0, 43.02, None)), "{w:?}");
    // the corner arcs keep their centres (G2 about the part corners), radius 3.02
    assert!(
        w.iter().any(|m| *m == ('W', -40.0, 43.02, Some((-40.0, 40.0)))),
        "{w:?}"
    );
}

#[test]
fn sample_control_mode_needs_lead_arcs_larger_than_the_tool() {
    // as sent: lead arcs R3 about (-43, 0), concave to the tool's side: with D = 3 the tool
    // centre's radius is 3 - 3 = 0 (the arc curves toward the tool, not away): refused
    let sent = "G17 G90 G94 G710\nG0 X-49 Y-3 Z5\nG1 Z-2 F300\nNORM G450\nG41 G1 X-43 Y-3 F800\nG3 X-40 Y0 I0 J3\nG1 Y40\nX40\nY-40\nX-40\nY0\nG3 X-43 Y3 I-3 J0\nG40 G1 X-49 Y3\nG0 Z5\n";
    let e = sample(sent, 3.0).unwrap_err();
    assert!(e.1.contains("radius"), "{e:?}");
    // with R6 lead arcs about (-46, 0): the tool centre runs R3 about (-46, 0), the ±43 square
    // with R3 corner circles (G450), and the lead-out R3 about (-46, 0)
    let fixed = "G17 G90 G94 G710\nG0 X-52 Y-6 Z5\nG1 Z-2 F300\nNORM G450\nG41 G1 X-46 Y-6 F800\nG3 X-40 Y0 I0 J6\nG1 Y40\nX40\nY-40\nX-40\nY0\nG3 X-46 Y6 I-6 J0\nG40 G1 X-52 Y6\nG0 Z5\n";
    let p = sample(fixed, 3.0).unwrap();
    let i = p
        .iter()
        .position(|m| *m == ('L', -46.0, -3.0, None))
        .expect("the approach");
    assert_eq!(
        p[i..].to_vec(),
        vec![
            ('L', -46.0, -3.0, None),
            ('C', -43.0, 0.0, Some((-46.0, 0.0))),
            ('L', -43.0, 40.0, None),
            ('W', -40.0, 43.0, Some((-40.0, 40.0))),
            ('L', 40.0, 43.0, None),
            ('W', 43.0, 40.0, Some((40.0, 40.0))),
            ('L', 43.0, -40.0, None),
            ('W', 40.0, -43.0, Some((40.0, -40.0))),
            ('L', -40.0, -43.0, None),
            ('W', -43.0, -40.0, Some((-40.0, -40.0))),
            ('L', -43.0, 0.0, None),
            ('C', -46.0, 3.0, Some((-46.0, 0.0))),
            ('L', -52.0, 6.0, None),
            ('R', -52.0, 6.0, None),
        ]
    );
}
