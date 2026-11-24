//! Machine tools as parts (COORDINATOR §4.7: "a machine library modelled as real parts").
//!
//! A machine definition's bodies (kernel/forge-machine) are collision envelopes: boxes and
//! cylinders the check reasons about exactly. Each body names the real part it stands for, a
//! `.fg` file. This builds those parts with forge-lang and checks every part lies inside its
//! envelope: every vertex of its tessellation (points on the exact surface) inside one of the
//! body's primitives, and the tessellation within `chord` of the surface, so the part lies in
//! the envelope grown by `chord` (0.01 mm by default; the collision check's clearance is 1 mm).
use forge_machine::def::{MachineDef, Prim};
use forge_mesh::{Quality, tessellate};
use std::path::Path;

#[derive(Clone, Debug, PartialEq)]
pub struct PartCheck {
    pub body: String,
    pub part: String,
    pub volume: f64,
    /// The envelope's volume (the sum of its primitives': an upper bound when they overlap).
    pub envelope: f64,
    pub vertices: usize,
    /// Vertices outside every primitive by more than the tolerance, with how far.
    pub outside: Vec<([f64; 3], f64)>,
}

/// How far a point lies outside a primitive (≤ 0 inside).
pub fn outside(p: &Prim, x: [f64; 3]) -> f64 {
    match p {
        Prim::Box { min, max } => {
            let mut d: f64 = f64::NEG_INFINITY;
            for k in 0..3 {
                d = d.max(min[k] - x[k]).max(x[k] - max[k]);
            }
            d
        }
        Prim::OBox { o, axes, size } => {
            let w = [x[0] - o[0], x[1] - o[1], x[2] - o[2]];
            let mut d: f64 = f64::NEG_INFINITY;
            for k in 0..3 {
                let s = w[0] * axes[k][0] + w[1] * axes[k][1] + w[2] * axes[k][2];
                d = d.max(-s).max(s - size[k]);
            }
            d
        }
        Prim::Cyl { base, axis, r, h } => {
            let w = [x[0] - base[0], x[1] - base[1], x[2] - base[2]];
            let z = w[0] * axis[0] + w[1] * axis[1] + w[2] * axis[2];
            let rad = [w[0] - z * axis[0], w[1] - z * axis[1], w[2] - z * axis[2]];
            let rho = (rad[0] * rad[0] + rad[1] * rad[1] + rad[2] * rad[2]).sqrt();
            (rho - r).max(-z).max(z - h)
        }
    }
}

fn prim_volume(p: &Prim) -> f64 {
    match p {
        Prim::Box { min, max } => (max[0] - min[0]) * (max[1] - min[1]) * (max[2] - min[2]),
        Prim::Cyl { r, h, .. } => forge_math::dmath::PI * r * r * h,
        Prim::OBox { size, .. } => size[0] * size[1] * size[2],
    }
}

/// Builds every body's part of a machine (paths relative to `dir`) and checks it against its
/// envelope with tolerance `tol` (≥ the tessellation's chord).
pub fn check_parts(def: &MachineDef, dir: &Path, tol: f64) -> Result<Vec<PartCheck>, String> {
    let q = Quality {
        chord: tol.min(Quality::default().chord),
        ..Quality::default()
    };
    let mut out = Vec::new();
    for b in &def.bodies {
        let Some(part) = &b.part else { continue };
        let path = dir.join(part);
        let src = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let opts = forge_lang::Options {
            dir: path.parent().map(Path::to_path_buf),
            ..forge_lang::Options::default()
        };
        let (run, _) = forge_lang::run(&src, opts, forge_lang::build::Cache::default());
        if !run.ok() {
            return Err(format!("{}: {:?}", path.display(), run.errors));
        }
        let body = run.body.ok_or_else(|| format!("{}: no body", path.display()))?;
        let mesh = tessellate(&body, &q).map_err(|e| format!("{}: {e:?}", path.display()))?;
        let mut bad = Vec::new();
        for p in &mesh.positions {
            let x = [p.x, p.y, p.z];
            let d = b
                .prims
                .iter()
                .map(|pr| outside(pr, x))
                .fold(f64::INFINITY, f64::min);
            if d > tol {
                bad.push((x, d));
            }
        }
        out.push(PartCheck {
            body: b.name.clone(),
            part: part.clone(),
            volume: run.measure.as_ref().map(|m| m.volume).unwrap_or(f64::NAN),
            envelope: b.prims.iter().map(prim_volume).sum(),
            vertices: mesh.positions.len(),
            outside: bad,
        });
    }
    Ok(out)
}

pub type MachineDefRef = MachineDef;

/// `check_parts` as the machine check's envelope findings (forge_machine_builder::check), at
/// the 0.01 mm tolerance of tests/parts.rs.
pub fn envelope_checks(
    def: &MachineDef,
    dir: &Path,
) -> Vec<(String, forge_machine_builder::check::EnvelopeCheck)> {
    use forge_machine_builder::check::EnvelopeCheck;
    let mut out = Vec::new();
    for b in &def.bodies {
        let Some(_) = &b.part else { continue };
        let mut one = def.clone();
        one.bodies = vec![b.clone()];
        let e = match check_parts(&one, dir, 0.01) {
            Ok(v) => match v.first() {
                Some(c) if c.outside.is_empty() => EnvelopeCheck::Inside { vertices: c.vertices },
                Some(c) => EnvelopeCheck::Outside {
                    vertices: c.vertices,
                    outside: c.outside.len(),
                    worst: c.outside.iter().map(|(_, d)| *d).fold(0.0, f64::max),
                },
                None => EnvelopeCheck::NotChecked("no part".into()),
            },
            Err(e) => EnvelopeCheck::NotChecked(format!("the part does not build: {e}")),
        };
        out.push((b.name.clone(), e));
    }
    out
}

/// What `forgecad machine SETUP.toml` answers.
pub struct Outcome {
    /// No violation, no contact, no rapid into material.
    pub ok: bool,
    pub text: String,
    /// `forgecad.machine/1`.
    pub json: String,
}

fn jstr(s: &str) -> String {
    let mut o = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

fn jnum(x: f64) -> String {
    if x.is_finite() {
        format!("{x:?}")
    } else {
        "null".into()
    }
}

/// Machines in the search order (docs/cam-machine.md): the project's `cam/machines/*.toml`
/// beside the setup, then the user's (`<config>/forgecad/cam/machines/`), then the built-in set.
/// A file at either level that does not load is an error (it may hold the machine asked for).
pub fn library(dir: &Path) -> Result<forge_machine::library::Library, String> {
    let user = forge_machine_builder::catalog::user_dir();
    let cat = forge_machine_builder::catalog::Catalog::load(Some(dir), user.as_deref());
    if let Some((_, f, e)) = cat.errors.first() {
        return Err(format!("{f}: {e}"));
    }
    Ok(forge_machine_builder::catalog::library(&cat))
}

/// Runs a setup file: the program on the machine, its timeline, collisions and removal.
/// A dialect's text: a file `NAME.toml` beside the setup wins over a built-in of the same name.
fn dialect_text(dir: &Path, name: &str) -> Result<String, String> {
    let dfile = dir.join(format!("{name}.toml"));
    match std::fs::read_to_string(&dfile) {
        Ok(s) => Ok(s),
        Err(_) => forge_machine::gcode::Control::dialect_src(name)
            .map(str::to_string)
            .ok_or_else(|| {
                format!(
                    "no dialect `{name}` (built in: fanuc haas linuxcnc grbl; or {})",
                    dfile.display()
                )
            }),
    }
}

/// A setup of several channels (`[[setup.channel]]`): each channel's program on its own letters,
/// synchronised at waits, every body checked over every timing the bounds allow
/// (forge_machine::multi). Reply: `forgecad.machine-multi/1`.
fn run_channels(dir: &Path, specs: &[forge_machine::setup::SetupSpec]) -> Result<Outcome, String> {
    use forge_machine::collide::Verdict;
    use forge_machine::multi::{self, Channel, MultiError};
    let lib = library(dir)?;
    let def = lib.get(&specs[0].machine).map_err(|e| e.to_string())?;
    let mut setups = Vec::new();
    let mut progs = Vec::new();
    for sp in specs {
        let dsrc = dialect_text(dir, &sp.dialect)?;
        setups.push(
            sp.build(def, &dsrc)
                .map_err(|e| format!("channel {}: {e}", sp.channel))?,
        );
        let pp = dir.join(&sp.program);
        progs.push(std::fs::read_to_string(&pp).map_err(|e| format!("{}: {e}", pp.display()))?);
    }
    let mut work = Vec::new();
    for (sp, s) in specs.iter().zip(&setups) {
        work.extend(sp.work_bodies(s));
    }
    let chans: Vec<Channel> = specs
        .iter()
        .zip(&setups)
        .zip(&progs)
        .map(|((sp, s), p)| Channel {
            ch: sp.channel,
            setup: s,
            program: p,
        })
        .collect();
    let run = match multi::run(def, &chans, &work) {
        Ok(r) => r,
        Err(MultiError::Deadlock(stuck)) => {
            let what: Vec<String> = stuck
                .iter()
                .map(|(c, l, w)| format!("channel {c} line {l} waits at M{w}"))
                .collect();
            let text = format!("REFUSED: deadlock: {}\n", what.join("; "));
            let json = format!(
                "{{\"format\":\"forgecad.machine-multi/1\",\"ok\":false,\"deadlock\":[{}]}}",
                stuck
                    .iter()
                    .map(|(c, l, w)| format!("{{\"channel\":{c},\"line\":{l},\"wait\":{w}}}"))
                    .collect::<Vec<_>>()
                    .join(",")
            );
            return Ok(Outcome {
                ok: false,
                text,
                json,
            });
        }
        Err(MultiError::Sim { ch, err }) => return Err(format!("channel {ch}: {err}")),
        Err(MultiError::TwoChannels(b)) => {
            return Err(format!(
                "body `{b}` is moved by joints of two channels: not modelled"
            ));
        }
    };
    let verdict_json = |v: &Verdict| match v {
        Verdict::Hit { s, witness } => format!(
            "\"hit\",\"s\":{},\"at\":[{},{},{}]",
            jnum(*s),
            jnum(witness[0]),
            jnum(witness[1]),
            jnum(witness[2])
        ),
        Verdict::Near { s0, s1, gap_lo } => format!(
            "\"near\",\"s0\":{},\"s1\":{},\"gap_lo\":{}",
            jnum(*s0),
            jnum(*s1),
            jnum(*gap_lo)
        ),
        Verdict::Clear => "\"clear\"".into(),
    };
    let ok = run.timelines.iter().all(|tl| tl.violations.is_empty())
        && run.own.iter().all(|r| r.contacts.is_empty())
        && run.cross.is_empty();
    let mut text = format!(
        "{} channels on {}: {:.1} s at least, {} at most\n",
        specs.len(),
        def.name,
        run.t[0],
        if run.t[1].is_finite() {
            format!("{:.1} s", run.t[1])
        } else {
            "unbounded".into()
        }
    );
    let mut chans_json = Vec::new();
    for (k, sp) in specs.iter().enumerate() {
        let tl = &run.timelines[k];
        text.push_str(&format!(
            "channel {} ({}): {} segments, {:.1} s at least alone\n",
            sp.channel,
            sp.program,
            tl.segments.len(),
            tl.t_min
        ));
        for v in &tl.violations {
            text.push_str(&format!("  violation: {v:?}\n"));
        }
        for c in &run.own[k].contacts {
            text.push_str(&format!(
                "  line {}: {} / {}: {:?}\n",
                c.line, c.a, c.b, c.verdict
            ));
        }
        let mut unchecked = forge_machine::sim::unchecked(def, tl);
        unchecked.extend(forge_machine::sim::unchecked_cycles(def, &progs[k]));
        for u in &unchecked {
            text.push_str(&format!("  not checked: {} {} ({})\n", u.path, u.why, u.check));
        }
        let own: Vec<String> = run.own[k]
            .contacts
            .iter()
            .map(|c| {
                format!(
                    "{{\"line\":{},\"a\":{},\"b\":{},\"verdict\":{}}}",
                    c.line,
                    jstr(&c.a),
                    jstr(&c.b),
                    verdict_json(&c.verdict)
                )
            })
            .collect();
        let windows: Vec<String> = run.windows[k]
            .iter()
            .map(|w| {
                format!(
                    "[{},{}]",
                    jnum(w[0]),
                    if w[1].is_finite() {
                        jnum(w[1])
                    } else {
                        "null".into()
                    }
                )
            })
            .collect();
        let ucs: Vec<String> = unchecked
            .iter()
            .map(|u| {
                format!(
                    "{{\"path\":{},\"why\":{},\"check\":{},\"line\":{}}}",
                    jstr(&u.path),
                    jstr(u.why),
                    jstr(u.check),
                    u.line
                )
            })
            .collect();
        chans_json.push(format!(
            "{{\"channel\":{},\"program\":{},\"timeline\":{},\"windows\":[{}],\"collisions\":[{}],\"not_checked\":[{}]}}",
            sp.channel,
            jstr(&sp.program),
            forge_machine::sim::timeline_json(tl),
            windows.join(","),
            own.join(","),
            ucs.join(",")
        ));
    }
    for c in &run.cross {
        text.push_str(&format!(
            "channel {} line {} / channel {} line {}: {} / {}: {:?} (some timing within the bounds)\n",
            c.a_channel, c.a_line, c.b_channel, c.b_line, c.a, c.b, c.verdict
        ));
    }
    for c in &run.conservative {
        text.push_str(&format!("conservative: {c}\n"));
    }
    text.push_str("removal: not modelled for several channels\n");
    text.push_str(if ok { "ok\n" } else { "REFUSED\n" });
    let cross: Vec<String> = run
        .cross
        .iter()
        .map(|c| {
            format!(
                "{{\"a_channel\":{},\"a_line\":{},\"b_channel\":{},\"b_line\":{},\"a\":{},\"b\":{},\"verdict\":{}}}",
                c.a_channel,
                c.a_line,
                c.b_channel,
                c.b_line,
                jstr(&c.a),
                jstr(&c.b),
                verdict_json(&c.verdict)
            )
        })
        .collect();
    let json = format!(
        "{{\"format\":\"forgecad.machine-multi/1\",\"ok\":{ok},\"machine\":{},\"t\":[{},{}],\"channels\":[{}],\"cross\":[{}],\"conservative\":[{}],\"claims\":{}}}",
        jstr(&def.name),
        jnum(run.t[0]),
        if run.t[1].is_finite() {
            jnum(run.t[1])
        } else {
            "null".into()
        },
        chans_json.join(","),
        cross.join(","),
        run.conservative
            .iter()
            .map(|c| jstr(c))
            .collect::<Vec<_>>()
            .join(","),
        jstr(
            "Channels synchronised at waits; every move takes between its t_min and its exact-stop time (no upper bound without declared accelerations and rates: such windows are open and listed under conservative). Bodies of two channels are checked over their whole motions with independent timing wherever their windows meet and no shared wait orders them: a Clear holds for every timing within the bounds; a cross contact may need a timing the control would not take. Each channel's own bodies are checked as for one channel. Not modelled: removal, superimposition, synchronised spindles."
        )
    );
    Ok(Outcome { ok, text, json })
}

pub fn run_setup(path: &Path) -> Result<Outcome, String> {
    use forge_machine::collide::{self, Verdict};
    use forge_machine::removal;
    use forge_machine::setup::{SetupSpec, Stock};
    let dir = path.parent().map(Path::to_path_buf).unwrap_or_default();
    let src = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if let Some(specs) = SetupSpec::parse_channels(&src).map_err(|e| format!("{}: {e}", path.display()))? {
        return run_channels(&dir, &specs);
    }
    let spec = SetupSpec::parse(&src).map_err(|e| format!("{}: {e}", path.display()))?;
    let lib = library(&dir)?;
    let def = lib.get(&spec.machine).map_err(|e| e.to_string())?;
    let dsrc = dialect_text(&dir, &spec.dialect)?;
    let setup = spec.build(def, &dsrc).map_err(|e| e.to_string())?;
    let ppath = dir.join(&spec.program);
    let prog = std::fs::read_to_string(&ppath).map_err(|e| format!("{}: {e}", ppath.display()))?;
    let tl = forge_machine::sim::simulate(&prog, &setup).map_err(|e| format!("{}: {e}", ppath.display()))?;
    // the stock: by the removal model (as cut so far) where it applies; otherwise the interval
    // check against the stock as first given
    let all_work = spec.work_bodies(&setup);
    let fixtures: Vec<_> = all_work
        .iter()
        .filter(|b| b.role != collide::Role::Stock)
        .cloned()
        .collect();
    let hh = |hs: &[removal::HolderHit]| -> Vec<removal::HolderHit> { hs.to_vec() };
    let (rem_json, rapid_cuts, holder_hits, rem_text, modelled) = match &spec.stock {
        Some(Stock::Box { min, max }) => match removal::mill(&setup, &tl, *min, *max, spec.step) {
            Ok(r) => (
                format!(
                    "{{\"kind\":\"height_map\",\"step\":{},\"volume\":{},\"removed\":{},\"air_feed_s\":{}}}",
                    jnum(spec.step),
                    jnum(r.map.volume()),
                    jnum(r.removed.iter().sum()),
                    jnum(r.air_feed_s)
                ),
                r.rapid_cuts.clone(),
                hh(&r.holder_hits),
                format!(
                    "removal: {:.0} mm³ removed, {:.1} s of feed in air",
                    r.removed.iter().sum::<f64>(),
                    r.air_feed_s
                ),
                true,
            ),
            Err(e) => (
                format!(
                    "{{\"kind\":\"not_modelled\",\"why\":{}}}",
                    jstr(&format!("{e:?}"))
                ),
                Vec::new(),
                Vec::new(),
                format!("removal: not modelled ({e:?}); the stock as first given is checked instead"),
                false,
            ),
        },
        Some(Stock::Bar { r0, z0, z1 }) => match removal::turn(&setup, &tl, *r0, *z0, *z1, spec.step) {
            Ok(r) => (
                format!(
                    "{{\"kind\":\"radius_map\",\"step\":{},\"volume\":{},\"removed\":{}}}",
                    jnum(spec.step),
                    jnum(r.map.volume()),
                    jnum(r.removed.iter().sum())
                ),
                r.rapid_cuts.clone(),
                hh(&r.holder_hits),
                format!("removal: {:.0} mm³ removed", r.removed.iter().sum::<f64>()),
                true,
            ),
            Err(e) => (
                format!(
                    "{{\"kind\":\"not_modelled\",\"why\":{}}}",
                    jstr(&format!("{e:?}"))
                ),
                Vec::new(),
                Vec::new(),
                format!("removal: not modelled ({e:?}); the stock as first given is checked instead"),
                false,
            ),
        },
        None => (
            "null".into(),
            Vec::new(),
            Vec::new(),
            "removal: no stock given".into(),
            false,
        ),
    };
    let col = collide::check(&setup, &tl, if modelled { &fixtures } else { &all_work });
    let (load, load_unchecked) = spec.check_load(&setup);
    let ok = tl.violations.is_empty()
        && load.is_ok()
        && col.contacts.is_empty()
        && rapid_cuts.is_empty()
        && holder_hits.is_empty();
    let mut text = format!(
        "{} on {} ({}): {} segments, {:.1} s at least, {:.1} s with exact stops{}\n",
        spec.program,
        def.name,
        spec.dialect,
        tl.segments.len(),
        tl.t_min,
        tl.t_stop,
        if tl.accel_missing {
            " (some joint has no accel: estimate = bound there)"
        } else {
            ""
        }
    );
    for v in &tl.violations {
        text.push_str(&format!("violation: {v:?}\n"));
    }
    // checks made against an unknown or estimated value: never a silent pass
    let mut unchecked = forge_machine::sim::unchecked(def, &tl);
    collide::narrow_skip_unchecked(&mut unchecked, &tl, &col);
    unchecked.extend(forge_machine::sim::unchecked_cycles(def, &prog));
    unchecked.extend(col.unchecked.iter().cloned());
    unchecked.extend(load_unchecked);
    if let Err((w, max)) = load {
        text.push_str(&format!(
            "violation: the work's {w} kg is over the {} mount's {max} kg\n",
            setup.part_mount
        ));
    }
    for u in &unchecked {
        text.push_str(&format!(
            "not checked: {} {} ({}{})\n",
            u.path,
            u.why,
            u.check,
            if u.line > 0 {
                format!(", from line {}", u.line)
            } else {
                String::new()
            }
        ));
    }
    for c in &col.contacts {
        let what = match &c.verdict {
            Verdict::Hit { witness, .. } => {
                format!("HIT at ({:.3}, {:.3}, {:.3})", witness[0], witness[1], witness[2])
            }
            Verdict::Near { gap_lo, .. } => format!(
                "not proven clear (gap ≥ {gap_lo:.3} mm shown, clearance {} mm)",
                def.clearance
            ),
            Verdict::Clear => "clear".into(),
        };
        text.push_str(&format!("line {}: {} / {}: {what}\n", c.line, c.a, c.b));
    }
    for r in &rapid_cuts {
        text.push_str(&format!(
            "line {}: rapid into material, {:.3} mm deep at ({:.3}, {:.3}, {:.3})\n",
            r.line, r.depth, r.at[0], r.at[1], r.at[2]
        ));
    }
    for h in &holder_hits {
        text.push_str(&format!(
            "line {}: tool piece {} into material, {:.3} mm at ({:.3}, {:.3}, {:.3})\n",
            h.line, h.piece, h.depth, h.at[0], h.at[1], h.at[2]
        ));
    }
    text.push_str(&format!(
        "collision: {} pairs × segments checked, {} spans\n{rem_text}\n",
        col.pairs_checked, col.spans
    ));
    text.push_str(if ok { "ok\n" } else { "REFUSED\n" });
    let contacts: Vec<String> = col
        .contacts
        .iter()
        .map(|c| {
            let v = match &c.verdict {
                Verdict::Hit { s, witness } => format!(
                    "\"hit\",\"s\":{},\"at\":[{},{},{}]",
                    jnum(*s),
                    jnum(witness[0]),
                    jnum(witness[1]),
                    jnum(witness[2])
                ),
                Verdict::Near { s0, s1, gap_lo } => format!(
                    "\"near\",\"s0\":{},\"s1\":{},\"gap_lo\":{}",
                    jnum(*s0),
                    jnum(*s1),
                    jnum(*gap_lo)
                ),
                Verdict::Clear => "\"clear\"".into(),
            };
            format!(
                "{{\"line\":{},\"segment\":{},\"a\":{},\"b\":{},\"verdict\":{v}}}",
                c.line,
                c.segment,
                jstr(&c.a),
                jstr(&c.b)
            )
        })
        .collect();
    let rcs: Vec<String> = rapid_cuts
        .iter()
        .map(|r| {
            format!(
                "{{\"line\":{},\"depth\":{},\"at\":[{},{},{}]}}",
                r.line,
                jnum(r.depth),
                jnum(r.at[0]),
                jnum(r.at[1]),
                jnum(r.at[2])
            )
        })
        .collect();
    let hhs: Vec<String> = holder_hits
        .iter()
        .map(|h| {
            format!(
                "{{\"line\":{},\"piece\":{},\"depth\":{},\"at\":[{},{},{}]}}",
                h.line,
                h.piece,
                jnum(h.depth),
                jnum(h.at[0]),
                jnum(h.at[1]),
                jnum(h.at[2])
            )
        })
        .collect();
    let ucs: Vec<String> = unchecked
        .iter()
        .map(|u| {
            format!(
                "{{\"path\":{},\"why\":{},\"check\":{},\"line\":{}}}",
                jstr(&u.path),
                jstr(u.why),
                jstr(u.check),
                u.line
            )
        })
        .collect();
    let json = format!(
        "{{\"format\":\"forgecad.machine/1\",\"ok\":{ok},\"machine\":{},\"program\":{},\"timeline\":{},\"collisions\":[{}],\"rapid_cuts\":[{}],\"holder_hits\":[{}],\"not_checked\":[{}],\"load\":{},\"removal\":{rem_json},\"claims\":{}}}",
        jstr(&def.name),
        jstr(&spec.program),
        forge_machine::sim::timeline_json(&tl),
        contacts.join(","),
        rcs.join(","),
        hhs.join(","),
        ucs.join(","),
        match load {
            Ok(()) => "null".to_string(),
            Err((w, max)) => format!("{{\"work_kg\":{},\"max_kg\":{}}}", jnum(w), jnum(max)),
        },
        jstr(
            "Proven: every pair of bodies on different links (machine envelopes, the tool's cylinders, the fixture; the stock as first given only where removal is not modelled) keeps the machine's clearance on every reported-clear segment, from interval poses (forge-math's interval contracts). Hits carry a witness point found in floating point. Times: t_min is a lower bound under the feed and axis limits; t_stop an exact-stop estimate. Removal heights, rapids into material and tool pieces into material are floating-point simulation at the grid nodes with the stock as cut so far, not a proof. Not claimed: the control's own interpolation, look-ahead or tolerances; machine fidelity beyond the envelopes."
        )
    );
    Ok(Outcome { ok, text, json })
}
