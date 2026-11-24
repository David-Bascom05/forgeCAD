//! L6: on forge-post's own output, the interpreter does what forge-post's reader says the control
//! does, motion for motion, on every dialect.
use forge_machine::gcode::{self, Centre, Control, Ev};
use forge_post::{Coolant, CycleKind, Dialect, Motion, Program, Rec};

struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 33
    }
    fn f(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * (self.next() % 1_000_000) as f64 / 1_000_000.0
    }
    fn pick(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

fn random_program(r: &mut Lcg, dialect: &str) -> Program {
    let mut recs = vec![Rec::Comment("op".into()), Rec::Wcs(0)];
    let mut tool = 1;
    recs.push(Rec::ToolChange {
        t: tool,
        h: tool,
        name: "flat-6".into(),
    });
    recs.push(Rec::Spindle {
        rpm: r.f(1000.0, 9000.0).round(),
        cw: true,
    });
    let mut p = [0.0, 0.0, 25.0];
    recs.push(Rec::Rapid(p));
    for _ in 0..40 {
        match r.pick(10) {
            0 => {
                p = [r.f(-50.0, 50.0), r.f(-50.0, 50.0), r.f(-5.0, 30.0)];
                recs.push(Rec::Rapid(p));
            }
            1..=3 => {
                p = [r.f(-50.0, 50.0), r.f(-50.0, 50.0), r.f(-5.0, 5.0)];
                recs.push(Rec::Line(p, r.f(50.0, 3000.0)));
            }
            4 | 5 => {
                // an arc about a centre at radius from the current point
                let c = [p[0] + r.f(-10.0, 10.0), p[1] + r.f(-10.0, 10.0)];
                let rad = ((p[0] - c[0]).powi(2) + (p[1] - c[1]).powi(2)).sqrt();
                let a = r.f(0.0, forge_math::dmath::TAU);
                let full = r.pick(5) == 0;
                let to = if full {
                    [p[0], p[1], p[2] - r.f(0.0, 1.0)]
                } else {
                    [
                        c[0] + rad * forge_math::dmath::cos(a),
                        c[1] + rad * forge_math::dmath::sin(a),
                        p[2],
                    ]
                };
                recs.push(Rec::Arc {
                    to,
                    c,
                    cw: r.pick(2) == 0,
                    f: r.f(50.0, 2000.0),
                });
                p = to;
            }
            6 => {
                // a cycle from a clearance height
                let clear = r.f(10.0, 30.0);
                let h0 = [r.f(-40.0, 40.0), r.f(-40.0, 40.0)];
                p = [h0[0], h0[1], clear];
                recs.push(Rec::Rapid(p));
                let kinds = [
                    CycleKind::Drill,
                    CycleKind::Dwell,
                    CycleKind::Peck,
                    CycleKind::ChipBreak,
                    CycleKind::Ream,
                    CycleKind::Tap,
                ];
                let mut kind = kinds[r.pick(6) as usize];
                if (dialect == "grbl" || dialect == "linuxcnc") && kind == CycleKind::Tap {
                    kind = CycleKind::Drill;
                }
                let n = 1 + r.pick(3) as usize;
                let mut holes = vec![h0];
                for _ in 1..n {
                    holes.push([r.f(-40.0, 40.0), r.f(-40.0, 40.0)]);
                }
                let last = *holes.last().unwrap();
                recs.push(Rec::Cycle {
                    kind,
                    holes,
                    z: r.f(-20.0, -1.0),
                    r: r.f(1.0, 5.0),
                    clear,
                    q: if matches!(kind, CycleKind::Peck | CycleKind::ChipBreak) {
                        r.f(0.5, 5.0)
                    } else {
                        0.0
                    },
                    dwell: if kind == CycleKind::Dwell { 0.25 } else { 0.0 },
                    f: r.f(50.0, 500.0),
                });
                p = [last[0], last[1], clear];
            }
            7 => recs.push(Rec::Dwell(0.5)),
            8 => recs.push(Rec::Coolant(
                [Coolant::Flood, Coolant::Mist, Coolant::Off][r.pick(3) as usize],
            )),
            _ => {
                tool += 1;
                if dialect != "grbl" {
                    recs.push(Rec::Home);
                }
                recs.push(Rec::ToolChange {
                    t: tool,
                    h: tool,
                    name: format!("tool-{tool}"),
                });
                recs.push(Rec::Spindle {
                    rpm: 3000.0,
                    cw: r.pick(2) == 0,
                });
                p = [r.f(-50.0, 50.0), r.f(-50.0, 50.0), 25.0];
                recs.push(Rec::Rapid(p));
            }
        }
    }
    recs.extend([Rec::Coolant(Coolant::Off), Rec::SpindleStop]);
    Program {
        number: 1001,
        title: "diff".into(),
        recs,
    }
}

/// The program without what the dialect refuses by design (forge-post's typed refusals are tested
/// in forge-post): a coolant it has no code for, a dwell whose units no open source gives, a
/// refused tap, a machine-home retract it cannot write, a work offset past its count.
fn fit(mut p: Program, d: &Dialect) -> Program {
    let key = |c: &Coolant| match c {
        Coolant::Flood => "coolant_flood",
        Coolant::Mist => "coolant_mist",
        Coolant::Through => "coolant_through",
        Coolant::Off => "coolant_off",
    };
    p.recs.retain(|r| match r {
        Rec::Coolant(c) => !d.template(key(c)).is_empty(),
        Rec::Dwell(_) => !d.template("dwell").is_empty(),
        Rec::Cycle {
            kind: CycleKind::Dwell,
            ..
        } => !d.template("dwell").is_empty() || d.template("cycle_dwell") != "expand",
        Rec::Cycle {
            kind: CycleKind::Tap, ..
        } => !matches!(d.template("cycle_tap"), "refuse" | ""),
        Rec::Home => d.has_lines("retract_home"),
        _ => true,
    });
    p
}

/// Our events in forge-post's Motion vocabulary.
fn as_motions(evs: &[Ev], c: &Control) -> Vec<Motion> {
    let ix = |l: char| c.letters.iter().position(|x| *x == l).unwrap();
    let (xi, yi, zi) = (ix('X'), ix('Y'), ix('Z'));
    let mut out = Vec::new();
    let mut last_comment = String::new();
    let full3 = |v: &Vec<Option<i64>>| [v[xi].unwrap(), v[yi].unwrap(), v[zi].unwrap()];
    for e in evs {
        match e {
            Ev::Rapid(v) => {
                let p = full3(v);
                out.push(Motion::Rapid(p));
            }
            Ev::Feed { to, f, .. } => {
                let p = full3(to);
                out.push(Motion::Feed(p, *f));
            }
            Ev::Arc {
                to,
                plane: 17,
                centre: Centre::Abs([i, j]),
                cw,
                full,
                f,
                ..
            } => {
                let p = full3(to);
                out.push(Motion::Arc {
                    to: p,
                    c: [*i, *j],
                    cw: *cw,
                    f: *f,
                    full: *full,
                });
            }
            Ev::Arc { .. } => panic!("an arc forge-post does not write"),
            Ev::Dwell { ms } => out.push(Motion::Dwell(*ms)),
            Ev::Tool { t, .. } => out.push(Motion::Tool(*t)),
            Ev::Spindle { s, cw, .. } => out.push(Motion::Spindle(*s, *cw)),
            Ev::SpindleStop => out.push(Motion::SpindleStop),
            Ev::Coolant(c) => out.push(Motion::Coolant(match c {
                gcode::Coolant::Flood => Coolant::Flood,
                gcode::Coolant::Mist => Coolant::Mist,
                gcode::Coolant::Through => Coolant::Through,
                gcode::Coolant::Off => Coolant::Off,
            })),
            Ev::Home { .. } | Ev::Machine(_) => {
                if out.last() != Some(&Motion::Home) {
                    out.push(Motion::Home);
                }
            }
            Ev::Comment(c) => last_comment = c.clone(),
            // grbl: the tool is named in the comment before M0
            Ev::Stop { optional: false } => {
                let t = last_comment
                    .strip_prefix("TOOL ")
                    .and_then(|r| r.split(':').next())
                    .and_then(|n| n.parse().ok())
                    .unwrap_or(0);
                out.push(Motion::Tool(t));
            }
            Ev::Select(_) | Ev::Length(_) | Ev::Wcs(_) | Ev::Clamp(_) | Ev::Stop { .. } | Ev::End => {}
        }
    }
    out
}

#[test]
fn interpreter_agrees_with_forge_post_reader() {
    let mut r = Lcg(0x5eed);
    let mut n = 0;
    for (name, src) in forge_post::DIALECTS {
        let d = Dialect::get(name).unwrap();
        let def = &forge_machine::library::builtin()[0];
        let c = Control::for_machine(def, src, 1).unwrap();
        for _ in 0..60 {
            let prog = fit(random_program(&mut r, name), &d);
            let q = forge_post::quantize(&prog, &d).unwrap();
            let text = forge_post::print(&q, &d).unwrap();
            let theirs = forge_post::read(&text, &d).unwrap();
            let (evs, _) = gcode::run(&text, &c)
                .unwrap_or_else(|e| panic!("{name} line {}: {}\n{text}", e.line, e.what));
            let evs: Vec<Ev> = evs.into_iter().map(|b| b.ev).collect();
            let ours = as_motions(&evs, &c);
            let (a, b) = (forge_post::normalize(&ours), forge_post::normalize(&theirs));
            if a != b {
                let i = a
                    .iter()
                    .zip(&b)
                    .position(|(x, y)| x != y)
                    .unwrap_or(a.len().min(b.len()));
                panic!(
                    "{name}: first difference at {i}: ours {:?} theirs {:?}\n{text}",
                    a.get(i.saturating_sub(1)..i + 2),
                    b.get(i.saturating_sub(1)..i + 2)
                );
            }
            n += 1;
        }
    }
    assert_eq!(n, 60 * forge_post::DIALECTS.len());
}

/// CAM-MACHINE review R1/R2: a `%` is skipped only as a tape mark or a `%` header/last block;
/// an absolute-centre dialect's R arc keeps its R; an absolute-centre arc outside G17 is refused.
#[test]
fn dialect_shell_edges() {
    let def = &forge_machine::library::builtin()[0];
    let fanuc = Control::for_dialect(def, "fanuc", 1).unwrap();
    let f = Dialect::get("fanuc").unwrap();
    // R1: a block with % inside is refused, by both readers; % alone is a tape mark
    let bad = "%\nG0 X0. Y0. Z5.\nG1 X10. F100. %\n";
    assert!(gcode::run(bad, &fanuc).is_err());
    assert!(forge_post::read(bad, &f).is_err());
    assert!(gcode::run("%\nG0 X0. Y0. Z5.\n%\n", &fanuc).is_ok());
    // Heidenhain: the header and the numbered last block are skipped
    let hc = Control::for_dialect(def, "heidenhain", 1).unwrap();
    let h = Dialect::get("heidenhain").unwrap();
    let head = "%P1 G71 *\nN10 G00 X+0 Y+0 Z+5*\n";
    let tail = "N99999999 %P1 G71 *\n";
    // R2a: G02 with R after a centre block: no I/J added (the core takes the R arc)
    let r_arc = format!("{head}N20 I+5 J+0*\nN30 G01 Z-1 F100*\nN40 G02 X+10 Y+0 R+5*\n{tail}");
    let (blocks, _) = gcode::run(&r_arc, &hc).unwrap_or_else(|e| panic!("line {}: {}", e.line, e.what));
    let r_ev = blocks.iter().find_map(|b| match &b.ev {
        Ev::Arc { centre, .. } => Some(centre.clone()),
        _ => None,
    });
    assert!(matches!(r_ev, Some(Centre::R(_))), "{r_ev:?}");
    // the centre form gives the same arc as the R form here (a half circle about (5, 0))
    let c_arc = format!("{head}N20 I+5 J+0*\nN30 G01 Z-1 F100*\nN40 G02 X+10 Y+0*\n{tail}");
    assert!(gcode::run(&c_arc, &hc).is_ok());
    assert!(forge_post::read(&c_arc, &h).is_ok());
    // R2b: an absolute-centre arc in G18 is refused (I K / J K are not modelled)
    let g18 = format!("{head}N20 G18*\nN30 I+5 J+0*\nN40 G02 X+10 Z+5*\n{tail}");
    let e = gcode::run(&g18, &hc).unwrap_err();
    assert!(e.what.contains("outside G17"), "{}", e.what);
    // a stray % in a Heidenhain block is refused too
    assert!(gcode::run(&format!("{head}N20 G01 X+1 % F100*\n"), &hc).is_err());
}
