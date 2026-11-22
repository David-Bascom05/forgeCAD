//! The kernel's flat-face booleans against the exact reference (`forge-exact`, the Lean
//! executable built by `lake build` in verify/forge-exact).
//!
//! A part is handed to the reference as its tools: every feature's tool solids as the regen
//! combines them (`forge_model::feature_tools`), each a planar B-rep whose numbers are written
//! exactly, and how each combines. The reference decides the whole history as one regularised
//! boolean of those solids, exactly, and answers solid / empty / non-manifold / ambiguous, with the
//! volume and the face, edge, vertex and shell counts. Each kernel answer (the sequential regen,
//! the n-ary one, a stage moved by ulps) is then judged against it: agree, kernel wrong, kernel
//! refused, or reference ambiguous.

use forge_geom::{Curve, Surface};
use forge_model::{Combine, Feature, History, Part};
use forge_topo::Body;
use std::io::{BufRead, BufReader, Write};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

/// An f64 exactly, as `[m, e]` meaning m·2^e.
pub fn exact(x: f64) -> String {
    if x == 0.0 {
        return "[0,0]".into();
    }
    let b = x.to_bits();
    let sign = if b >> 63 == 1 { -1i64 } else { 1 };
    let e = ((b >> 52) & 0x7ff) as i64;
    let m = (b & ((1u64 << 52) - 1)) as i64;
    let (mut m, mut e) = if e == 0 {
        (m, -1074)
    } else {
        (m | (1i64 << 52), e - 1075)
    };
    while m % 2 == 0 {
        m /= 2;
        e += 1;
    }
    format!("[{},{}]", sign * m, e)
}

fn p3(x: f64, y: f64, z: f64) -> String {
    format!("[{},{},{}]", exact(x), exact(y), exact(z))
}

fn op_name(c: Combine) -> &'static str {
    match c {
        Combine::New => "new",
        Combine::Add => "add",
        Combine::Cut => "cut",
        Combine::Keep => "keep",
    }
}

/// A planar solid in the reference's format, or why it is not one.
pub fn dump_solid(body: &Body, op: Combine) -> Result<String, String> {
    let b = body.compact();
    for e in b.edge_ids() {
        let c = &b.edge(e).curve;
        if !matches!(c, Curve::Line(_)) {
            let kind = format!("{c:?}");
            return Err(format!(
                "an edge is a {}",
                kind.split(['(', ' ', '{']).next().unwrap_or("")
            ));
        }
    }
    let verts: Vec<String> = b
        .vertices
        .iter()
        .map(|v| p3(v.point.x, v.point.y, v.point.z))
        .collect();
    let mut faces = Vec::new();
    for f in b.face_ids() {
        let face = b.face(f);
        let Surface::Plane(pl) = &face.surface else {
            return Err("a face is not flat".into());
        };
        let o = pl.frame.origin;
        let n = pl.frame.z().get();
        let mut loops = Vec::new();
        for &lp in face.loops.iter() {
            let ids: Vec<String> = b
                .lp(lp)
                .coedges()
                .map(|c| {
                    let ce = b.coedge(c);
                    let ed = b.edge(ce.edge);
                    (if ce.forward { ed.start } else { ed.end }).index().to_string()
                })
                .collect();
            loops.push(format!("[{}]", ids.join(",")));
        }
        faces.push(format!(
            "{{\"o\":{},\"n\":{},\"loops\":[{}]}}",
            p3(o.x, o.y, o.z),
            p3(n.x, n.y, n.z),
            loops.join(",")
        ));
    }
    Ok(format!(
        "{{\"op\":\"{}\",\"verts\":[{}],\"faces\":[{}]}}",
        op_name(op),
        verts.join(","),
        faces.join(",")
    ))
}

/// Features that make nothing.
fn inert(f: &Feature) -> bool {
    matches!(
        f,
        Feature::Param(_) | Feature::Sketch(_) | Feature::Plane(_) | Feature::Work(_)
    )
}

/// The part as the reference reads it: every tool solid of its history, or why it is out of
/// the flat-face reference's scope.
pub fn reference_input(part: &Part) -> Result<String, String> {
    if forge_model::bodies::multi(&part.features) {
        return Err("several bodies".into());
    }
    let h = History::new(&part.features, None);
    let mut solids = Vec::new();
    for (i, f) in part.features.iter().enumerate() {
        if inert(f) {
            continue;
        }
        if !matches!(f, Feature::Extrude { .. } | Feature::Pattern { .. }) {
            return Err(format!("feature {i} is a {}", forge_lang::fuzz::feature_kind(f)));
        }
        let tools = guarded(|| forge_model::feature_tools(h, i).map_err(|e| format!("{e:?}")))
            .map_err(|e| format!("feature {i}'s tool: {}", short(&e)))?;
        for (body, op) in tools {
            if !solids.is_empty() && op == Combine::New {
                return Err("several bodies".into());
            }
            solids.push(dump_solid(&body, op).map_err(|e| format!("feature {i}: {e}"))?);
        }
    }
    if solids.is_empty() {
        return Err("no solids".into());
    }
    Ok(format!(
        "{{\"linear\":{},\"rule\":\"{}\",\"solids\":[{}]}}",
        exact(forge_math::tol::LINEAR),
        rule(),
        solids.join(",")
    ))
}

/// The part with every feature whose tool is not flat dropped (a family's rods, a history's
/// balls): its flat-face sub-history, which the stage-1 reference decides. Features that make
/// nothing are kept.
pub fn flat_part(part: &Part) -> Part {
    let mut p = part.clone();
    let mut i = 1;
    while i < p.features.len() {
        let f = &p.features[i];
        let flat = inert(f)
            || (matches!(f, Feature::Extrude { .. } | Feature::Pattern { .. })
                && guarded(|| {
                    forge_model::feature_tools(History::new(&p.features, None), i)
                        .map_err(|e| format!("{e:?}"))
                })
                .is_ok_and(|ts| ts.iter().all(|(b, op)| dump_solid(b, *op).is_ok())));
        if flat {
            i += 1;
        } else {
            p.features.remove(i);
        }
    }
    p
}

/// Which rule the reference applies: `strict` (any decision within LINEAR is ambiguous: the
/// default until the kernel implements §6.1) or `snap` (§6.1: tolerance-equal surfaces merged,
/// ambiguous only past the spread cap). FORGE_EXACT_RULE picks.
pub fn rule() -> String {
    std::env::var("FORGE_EXACT_RULE").unwrap_or_else(|_| "strict".into())
}

/// Volumes agree to this relative slack where the result has sub-LINEAR features the kernel
/// may collapse (each at most LINEAR thick).
pub const SLIVER_SLACK: f64 = 1e-4;

/// The reference's answer.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Answer {
    pub verdict: String,
    pub volume: f64,
    pub volume_exact: String,
    pub faces: usize,
    pub edges: usize,
    pub vertices: usize,
    pub shells: usize,
    pub notes: Vec<String>,
    pub error: Option<String>,
}

/// The reference executable, kept running.
pub struct Reference {
    child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
}

impl Reference {
    /// FORGE_EXACT, else the lake build beside this crate.
    pub fn start() -> Reference {
        let exe = std::env::var("FORGE_EXACT").unwrap_or_else(|_| {
            concat!(env!("CARGO_MANIFEST_DIR"), "/../.lake/build/bin/forge-exact").to_string()
        });
        let mut child = Command::new(&exe)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap_or_else(|e| panic!("{exe}: {e} (run `lake build` in verify/forge-exact)"));
        let input = child.stdin.take().expect("stdin");
        let output = BufReader::new(child.stdout.take().expect("stdout"));
        Reference { child, input, output }
    }

    pub fn ask(&mut self, line: &str) -> Answer {
        writeln!(self.input, "{line}").expect("the reference's stdin");
        self.input.flush().expect("flush");
        let mut out = String::new();
        self.output.read_line(&mut out).expect("the reference's answer");
        if std::env::var_os("FORGE_EXACT_TRACE").is_some() {
            eprintln!("> {line}\n< {out}");
        }
        parse_answer(&out)
    }
}

impl Drop for Reference {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

/// The reference's flat JSON answer (strings, numbers, an array of strings).
pub fn parse_answer(s: &str) -> Answer {
    let mut a = Answer::default();
    let mut rest = s.trim().trim_start_matches('{');
    while let Some(q) = rest.find('"') {
        rest = &rest[q + 1..];
        let Some(e) = rest.find('"') else { break };
        let key = rest[..e].to_string();
        rest = rest[e + 1..].trim_start().trim_start_matches(':').trim_start();
        if rest.starts_with('"') {
            let (v, r) = json_string(rest);
            rest = r;
            match key.as_str() {
                "verdict" => a.verdict = v,
                "volume" => a.volume_exact = v,
                "error" => a.error = Some(v),
                _ => {}
            }
        } else if rest.starts_with('[') {
            rest = &rest[1..];
            loop {
                rest = rest.trim_start().trim_start_matches(',').trim_start();
                if rest.starts_with(']') || rest.is_empty() {
                    rest = rest.get(1..).unwrap_or("");
                    break;
                }
                let (v, r) = json_string(rest);
                a.notes.push(v);
                rest = r;
            }
        } else {
            let end = rest.find([',', '}']).unwrap_or(rest.len());
            let v = rest[..end].trim();
            match key.as_str() {
                "volume_f" => a.volume = v.parse().unwrap_or(f64::NAN),
                "faces" => a.faces = v.parse().unwrap_or(0),
                "edges" => a.edges = v.parse().unwrap_or(0),
                "vertices" => a.vertices = v.parse().unwrap_or(0),
                "shells" => a.shells = v.parse().unwrap_or(0),
                _ => {}
            }
            rest = &rest[end..];
        }
    }
    a
}

fn json_string(s: &str) -> (String, &str) {
    let mut out = String::new();
    let mut chars = s[1..].char_indices();
    while let Some((i, c)) = chars.next() {
        match c {
            '"' => return (out, &s[i + 2..]),
            '\\' => {
                if let Some((_, n)) = chars.next() {
                    out.push(match n {
                        'n' => '\n',
                        't' => '\t',
                        o => o,
                    });
                }
            }
            c => out.push(c),
        }
    }
    (out, "")
}

/// What the kernel made of a part.
#[derive(Clone, Debug)]
pub enum Kernel {
    Built {
        volume: f64,
        sig: forge_proof::props::Sig,
        invalid: Option<String>,
    },
    Refused(String),
}

pub fn short(s: &str) -> String {
    let s: String = s.chars().take(160).collect();
    s.replace('\n', " ")
}

pub fn guarded<T>(f: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(r) => r,
        Err(p) => Err(format!(
            "panic: {}",
            p.downcast_ref::<String>()
                .cloned()
                .or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_default()
        )),
    }
}

pub fn judge_body(r: Result<Body, String>) -> Kernel {
    match r {
        Err(e) => Kernel::Refused(short(&e)),
        Ok(b) => {
            let invalid = forge_topo::check(&b).first().map(|p| short(&format!("{p:?}")));
            let volume = forge_topo::mass::mass(&b).map(|m| m.volume).unwrap_or(f64::NAN);
            Kernel::Built {
                volume,
                sig: forge_proof::props::sig(&b),
                invalid,
            }
        }
    }
}

/// The sequential regen.
pub fn sequential(part: &Part) -> Kernel {
    judge_body(guarded(|| part.regenerate().map_err(|e| format!("{e:?}"))))
}

/// The n-ary regen, on one thread.
pub fn nary(part: &Part) -> Kernel {
    judge_body(guarded(|| part.regenerate_nary(1).map_err(|e| format!("{e:?}"))))
}

/// The last feature applied to the stage before it moved by ulps (`forge_proof::props::nudged`).
pub fn nudged_last(part: &Part, seed: u64) -> Kernel {
    let h = History::new(&part.features, None);
    let k = part.features.len() - 1;
    judge_body(guarded(|| {
        let mut body = None;
        for i in 0..k {
            body = forge_model::step(h, i, body).map_err(|e| format!("{e:?}"))?;
        }
        let stage = body.ok_or("no stage")?;
        let moved = forge_proof::props::nudged(&stage, seed);
        forge_model::step(h, k, Some(moved))
            .map_err(|e| format!("{e:?}"))?
            .ok_or_else(|| "no body".to_string())
    }))
}

/// How a kernel answer compares with the reference.
#[derive(Clone, Debug, PartialEq)]
pub enum Class {
    Agree,
    /// The kernel built something the reference contradicts (or an invalid body).
    Wrong(String),
    /// The kernel refused where the reference gives a solid (or refused for another reason).
    Refused(String),
    /// The reference found a decision within LINEAR: no judgement.
    Ambiguous,
    /// Not a flat-face boolean.
    Scope(String),
}

impl Class {
    pub fn key(&self) -> &'static str {
        match self {
            Class::Agree => "agree",
            Class::Wrong(_) => "kernel wrong",
            Class::Refused(_) => "kernel refused",
            Class::Ambiguous => "reference ambiguous",
            Class::Scope(_) => "out of scope",
        }
    }
    pub fn detail(&self) -> String {
        match self {
            Class::Wrong(s) | Class::Refused(s) | Class::Scope(s) => s.clone(),
            _ => String::new(),
        }
    }
    /// The kind of disagreement, without numbers: what groups repros.
    pub fn kind(&self) -> String {
        let d = self.detail();
        let words: Vec<&str> = d
            .split([' ', '{', '(', ':'])
            .filter(|w| !w.is_empty())
            .take(3)
            .collect();
        format!("{}: {}", self.key(), words.join(" "))
    }
}

/// Volumes agree to this relative slack (both are exact up to the kernel's ulps).
pub const VOLUME_SLACK: f64 = 1e-7;

pub fn classify(r: &Answer, k: &Kernel) -> Class {
    if let Some(e) = &r.error {
        return Class::Scope(format!("reference: {e}"));
    }
    // a history that combines before any part exists (a flat sub-program whose base was
    // curved): the kernel's refusal states its reason and the history means nothing else
    if let Kernel::Refused(e) = k
        && e.starts_with("NothingToCombine")
    {
        return Class::Scope("the history combines before any part exists".into());
    }
    match r.verdict.as_str() {
        "ambiguous" => Class::Ambiguous,
        "solid" => match k {
            Kernel::Refused(e) => Class::Refused(format!("refused {e} where the reference gives a solid")),
            Kernel::Built { invalid: Some(p), .. } => Class::Wrong(format!("invalid body {p}")),
            Kernel::Built { volume, sig, .. } => {
                let (v, e, f) = (sig.verts - sig.redundant, sig.edges - sig.redundant, sig.faces);
                if (volume - r.volume).abs() > VOLUME_SLACK * r.volume.abs().max(1.0) {
                    Class::Wrong(format!("volume {volume} where the reference gives {}", r.volume))
                } else if (v, e, f, sig.shells) != (r.vertices, r.edges, r.faces, r.shells) {
                    Class::Wrong(format!(
                        "topology V{v} E{e} F{f} S{} where the reference gives V{} E{} F{} S{}",
                        sig.shells, r.vertices, r.edges, r.faces, r.shells
                    ))
                } else {
                    Class::Agree
                }
            }
        },
        "sliver" => match k {
            Kernel::Refused(e) if e.contains("ZeroThickness") => Class::Agree,
            Kernel::Refused(e) => Class::Refused(format!(
                "refused {e} where the result is a solid with sub-LINEAR features (ZeroThickness or a collapse expected)"
            )),
            Kernel::Built { invalid: Some(p), .. } => Class::Wrong(format!("invalid body {p}")),
            Kernel::Built { volume, .. } => {
                if (volume - r.volume).abs() > SLIVER_SLACK * r.volume.abs().max(1.0) {
                    Class::Wrong(format!("volume {volume} where the reference gives {}", r.volume))
                } else {
                    Class::Agree
                }
            }
        },
        "empty" => match k {
            Kernel::Refused(e) if e.starts_with("Empty") => Class::Agree,
            Kernel::Refused(e) => Class::Refused(format!("refused {e} where the result is empty")),
            Kernel::Built { volume, .. } => {
                Class::Wrong(format!("built volume {volume} where the result is empty"))
            }
        },
        "nonmanifold" => match k {
            Kernel::Refused(e) if e.contains("ZeroThickness") => Class::Agree,
            Kernel::Refused(e) => Class::Refused(format!(
                "refused {e} where the result is non-manifold (ZeroThickness expected)"
            )),
            Kernel::Built { volume, .. } => Class::Wrong(format!(
                "built volume {volume} where the result is non-manifold ({})",
                r.notes.first().cloned().unwrap_or_default()
            )),
        },
        v => Class::Scope(format!("reference verdict {v}")),
    }
}

/// One case: a part from some source, judged on each kernel path.
pub struct Case {
    pub name: String,
    /// .fg text when the case came from the language, else the part's Rust literal.
    pub source: String,
    pub part: Part,
}

pub struct Judged {
    pub name: String,
    pub path: &'static str,
    pub class: Class,
    pub answer: Option<Answer>,
}

/// `classify`, and where the kernel says a feature empties the part (a refusal with a stated
/// reason), the reference asked about the history up to that feature.
pub fn judge_kernel(reference: &mut Reference, part: &Part, ans: &Answer, k: &Kernel) -> Class {
    if let Kernel::Refused(e) = k
        && let Some(rest) = e.strip_prefix("Empty { feature: ")
        && let Some(n) = rest.split(' ').next().and_then(|n| n.parse::<usize>().ok())
        && n < part.features.len()
    {
        let mut prefix = part.clone();
        prefix.features.truncate(n + 1);
        return match reference_input(&prefix) {
            Err(e) => Class::Scope(e),
            Ok(input) => {
                let a = reference.ask(&input);
                match a.verdict.as_str() {
                    "empty" => Class::Agree,
                    "ambiguous" => Class::Ambiguous,
                    v => Class::Refused(format!(
                        "refused Empty at feature {n} where the reference gives {v} (volume {}) there",
                        a.volume
                    )),
                }
            }
        };
    }
    classify(ans, k)
}

/// Every kernel path of a case against the reference.
pub fn judge(reference: &mut Reference, case: &Case, ulp_seeds: &[u64]) -> Vec<Judged> {
    let input = match reference_input(&case.part) {
        Ok(i) => i,
        Err(e) => {
            return vec![Judged {
                name: case.name.clone(),
                path: "-",
                class: Class::Scope(e),
                answer: None,
            }];
        }
    };
    let ans = reference.ask(&input);
    let mut out = Vec::new();
    let mut paths: Vec<(&'static str, Kernel)> =
        vec![("sequential", sequential(&case.part)), ("nary", nary(&case.part))];
    for &s in ulp_seeds {
        paths.push((
            Box::leak(format!("ulp {s}").into_boxed_str()),
            nudged_last(&case.part, s),
        ));
    }
    for (path, k) in paths {
        out.push(Judged {
            name: case.name.clone(),
            path,
            class: judge_kernel(reference, &case.part, &ans, &k),
            answer: Some(ans.clone()),
        });
    }
    out
}

pub mod shrink;
