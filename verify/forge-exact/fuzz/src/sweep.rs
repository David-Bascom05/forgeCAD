//! The swept solids against their definitions (`sweep-ref`, the Lean executable built by
//! `lake build` in verify/forge-exact; research/sweep-spec/DESIGN.md §4.1).
//!
//! A feature is handed over as its *definition*: the profile's loops in sketch coordinates, the
//! sketch frame, the travel and the range, every number exact. Its tool as the kernel builds it
//! comes with it. The reference decides the definition's point set itself (`prism_iff`) and
//! compares the two exactly: agree, differ, or, where the kernel refused, whether the definition
//! was in the domain.

use crate::exact;
use forge_math::{Frame, Point2, Vec3};
use forge_model::{Extent, Feature};
use forge_ops::{Profile, Segment};
use forge_topo::Body;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

fn v3(v: Vec3) -> String {
    format!("[{},{},{}]", exact(v.x), exact(v.y), exact(v.z))
}

/// The polygon loops of a profile of lines only, in sketch coordinates (each segment's start).
pub fn polygon(profile: &Profile) -> Option<Vec<Vec<Point2>>> {
    profile
        .loops
        .iter()
        .map(|l| {
            l.segments
                .iter()
                .map(|(s, _)| match s {
                    Segment::Line { a, .. } => Some(*a),
                    _ => None,
                })
                .collect()
        })
        .collect()
}

/// What the definition of an extrude is: its range along the travel `v` (in units of `v`).
pub fn prism_range(profile: &Profile, extent: &Extent) -> Option<(Vec3, f64, f64)> {
    let z = profile.plane.z().get();
    Some(match *extent {
        Extent::Distance(d) => (z, d.min(0.0), d.max(0.0)),
        Extent::Symmetric(d) => (z, -d / 2.0, d / 2.0),
        Extent::TwoSided { forward, back } => (z, -back, forward),
        Extent::Along { forward, back, dir } => {
            // the travel along `dir`, scaled to rise 1 along the normal: dir / (dir . z), exact
            // in the reference; here the dyadic dir and the range are handed over as they are
            // the reference scales `dir` to rise 1 along the normal, exactly (its `n`)
            return Some((dir, -back, forward));
        }
        _ => return None,
    })
}

/// The reference's input line for an extrude: its definition and the kernel's answer.
/// `along`: the travel `v` is a direction to be scaled to rise 1 along the frame's normal.
pub fn prism_line(
    frame: &Frame,
    v: Vec3,
    along: bool,
    lo: f64,
    hi: f64,
    loops: &[Vec<Point2>],
    kernel: Result<&Body, &str>,
) -> Result<String, String> {
    let o = frame.origin;
    let n = if along {
        format!(",\"n\":{}", v3(frame.z().get()))
    } else {
        String::new()
    };
    // the kernel's least rise for a direction (forge_model's `tool`): a stated precondition
    let along_min = if along {
        format!("\"along_min\":{},", exact(0.0175))
    } else {
        String::new()
    };
    let fr = format!(
        "{{\"o\":{},\"x\":{},\"y\":{},\"v\":{}{n}}}",
        v3(Vec3::new(o.x, o.y, o.z)),
        v3(frame.x().get()),
        v3(frame.y().get()),
        v3(v)
    );
    let ls: Vec<String> = loops
        .iter()
        .map(|l| {
            let ps: Vec<String> = l
                .iter()
                .map(|p| format!("[{},{}]", exact(p.x), exact(p.y)))
                .collect();
            format!("[{}]", ps.join(","))
        })
        .collect();
    let k = match kernel {
        Ok(b) => format!("\"kernel\":{}", crate::dump_solid(b, forge_model::Combine::New)?),
        Err(why) => format!("\"kernel_refused\":{}", serde_free_string(why)),
    };
    Ok(format!(
        "{{\"linear\":{},\"kind\":\"prism\",{along_min}\"frame\":{fr},\"lo\":{},\"hi\":{},\"loops\":[{}],{k}}}",
        exact(forge_math::tol::LINEAR),
        exact(lo),
        exact(hi),
        ls.join(",")
    ))
}

/// A JSON string literal.
pub fn serde_free_string(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 => out.push(' '),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// The input line for feature `i` of `features`, if it is an extrude the reference can define (a
/// profile of lines, an extent fixed by the feature alone).
pub fn feature_line(features: &[Feature], i: usize) -> Option<Result<String, String>> {
    let Feature::Extrude { profile, extent, .. } = &features[i] else {
        return None;
    };
    let loops = polygon(profile)?;
    if let Extent::Tapered { distance, angle } = *extent {
        return Some(taper_line(features, i, profile, &loops, distance, angle));
    }
    let (v, lo, hi) = prism_range(profile, extent)?;
    let along = matches!(extent, Extent::Along { .. });
    let h = forge_model::History::new(features, None);
    let tools = crate::guarded(|| forge_model::feature_tools(h, i).map_err(|e| format!("{e:?}")));
    Some(match tools {
        Ok(ts) if ts.len() == 1 => prism_line(&profile.plane, v, along, lo, hi, &loops, Ok(&ts[0].0)),
        Ok(ts) => Err(format!("{} tools", ts.len())),
        Err(e) => prism_line(&profile.plane, v, along, lo, hi, &loops, Err(&e)),
    })
}

/// `sweep-ref`, kept running.
pub struct SweepRef {
    child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
}

impl SweepRef {
    /// SWEEP_REF, else the lake build beside this crate.
    pub fn start() -> SweepRef {
        let exe = std::env::var("SWEEP_REF").unwrap_or_else(|_| {
            concat!(env!("CARGO_MANIFEST_DIR"), "/../.lake/build/bin/sweep-ref").to_string()
        });
        let mut child = Command::new(&exe)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap_or_else(|e| panic!("{exe}: {e} (run `lake build` in verify/forge-exact)"));
        let input = child.stdin.take().expect("stdin");
        let output = BufReader::new(child.stdout.take().expect("stdout"));
        SweepRef { child, input, output }
    }

    pub fn ask(&mut self, line: &str) -> String {
        writeln!(self.input, "{line}").expect("the reference's stdin");
        self.input.flush().expect("flush");
        let mut out = String::new();
        self.output.read_line(&mut out).expect("the reference's answer");
        out.trim_end().to_string()
    }
}

impl Drop for SweepRef {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

/// A string field of a JSON answer: the last one of the name (the answer's keys come sorted, and
/// a nested object, such as `against_roof`, sorts before the top-level `verdict`).
pub fn field<'a>(answer: &'a str, key: &str) -> Option<&'a str> {
    let k = format!("\"{key}\":\"");
    let at = answer.rfind(&k)? + k.len();
    let rest = &answer[at..];
    Some(&rest[..rest.find('"')?])
}

/// The input line for a tapered extrude: the slope as the kernel takes it, `sin / cos` of
/// `|angle|` (forge_ops::tapered), the side it leans, the range.
fn taper_line(
    features: &[Feature],
    i: usize,
    profile: &Profile,
    loops: &[Vec<Point2>],
    distance: f64,
    angle: f64,
) -> Result<String, String> {
    let h = forge_model::History::new(features, None);
    let tools = crate::guarded(|| forge_model::feature_tools(h, i).map_err(|e| format!("{e:?}")));
    let kernel = match &tools {
        Ok(ts) if ts.len() == 1 => Ok(&ts[0].0),
        Ok(ts) => return Err(format!("{} tools", ts.len())),
        Err(e) => Err(e.as_str()),
    };
    let base = prism_line(
        &profile.plane,
        profile.plane.z().get(),
        false,
        distance.min(0.0),
        distance.max(0.0),
        loops,
        kernel,
    )?;
    let (sn, cs) = forge_math::dmath::sin_cos(angle.abs());
    let sign = if angle < 0.0 { -1.0 } else { 1.0 };
    Ok(base.replacen(
        "\"kind\":\"prism\"",
        &format!(
            "\"kind\":\"taper\",\"sin\":{},\"cos\":{},\"sign\":{}",
            exact(sn),
            exact(cs),
            exact(sign)
        ),
        1,
    ))
}
