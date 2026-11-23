//! The kernel's certificates (forge-bool's `certify` feature) handed to the proven checker
//! (`forge-certify`, the Lean executable `lake build` makes here), and its verdicts.

use forge_bool::Op;
use forge_bool::certify::Record;
use forge_exact_fuzz::exact;
use forge_geom::{Curve, Surface};
use forge_math::Point3;
use forge_topo::{Body, FaceId};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

fn p3(p: Point3) -> String {
    format!("[{},{},{}]", exact(p.x), exact(p.y), exact(p.z))
}

/// The key of a point: its bits (a vertex is the same point wherever it appears).
fn bits(p: Point3) -> [u64; 3] {
    let b = |x: f64| if x == 0.0 { 0 } else { x.to_bits() };
    [b(p.x), b(p.y), b(p.z)]
}

#[derive(Default)]
struct Keys {
    index: HashMap<[u64; 3], usize>,
    points: Vec<Point3>,
}

impl Keys {
    fn of(&mut self, p: Point3) -> usize {
        let n = self.points.len();
        *self.index.entry(bits(p)).or_insert_with(|| {
            self.points.push(p);
            n
        })
    }
    fn lp(&mut self, l: &[Point3]) -> String {
        let ks: Vec<String> = l.iter().map(|&p| self.of(p).to_string()).collect();
        format!("[{}]", ks.join(","))
    }
}

/// A face's plane, as the kernel holds it, or why the face is not flat.
fn plane(b: &Body, f: FaceId) -> Result<(Point3, Point3), String> {
    let Surface::Plane(pl) = &b.face(f).surface else {
        return Err("a face is not flat".into());
    };
    let n = pl.frame.z().get();
    Ok((pl.frame.origin, Point3::new(n.x, n.y, n.z)))
}

fn loops(b: &Body, f: FaceId) -> Vec<Vec<Point3>> {
    b.face(f)
        .loops
        .iter()
        .map(|&l| {
            b.lp(l)
                .coedges()
                .map(|c| {
                    let ce = b.coedge(c);
                    let e = b.edge(ce.edge);
                    b.vertex(if ce.forward { e.start } else { e.end }).point
                })
                .collect()
        })
        .collect()
}

fn flat(b: &Body) -> Result<(), String> {
    for e in b.edge_ids() {
        if !matches!(b.edge(e).curve, Curve::Line(_)) {
            return Err("an edge is not straight".into());
        }
    }
    Ok(())
}

pub fn op_name(op: Op) -> &'static str {
    match op {
        Op::Union => "union",
        Op::Subtract => "diff",
        Op::Intersect => "inter",
    }
}

/// The certificate as the checker reads it (ForgeCertify/Input.lean), or why the boolean is out of
/// its scope.
pub fn dump(rec: &Record, frames: &str) -> Result<String, String> {
    let Some(res) = &rec.result else {
        return Err("the kernel refused".into());
    };
    for b in [&rec.a, &rec.b, res] {
        flat(b)?;
    }
    let mut keys = Keys::default();
    let mut planes: Vec<(usize, usize, (Point3, Point3))> = Vec::new();
    let mut operands = Vec::new();
    for (side, b) in [&rec.a, &rec.b].into_iter().enumerate() {
        let ids: Vec<FaceId> = b.face_ids().collect();
        let mut faces = Vec::new();
        for (j, &f) in ids.iter().enumerate() {
            let (o, n) = plane(b, f)?;
            planes.push((side, j, (o, n)));
            let ls: Vec<String> = loops(b, f).iter().map(|l| keys.lp(l)).collect();
            faces.push(format!(
                "{{\"o\":{},\"n\":{},\"loops\":[{}]}}",
                p3(o),
                p3(n),
                ls.join(",")
            ));
        }
        let mut pieces = Vec::new();
        for pc in rec.pieces.iter().filter(|p| p.side as usize == side) {
            let Some(j) = ids.iter().position(|&f| f == pc.face) else {
                return Err("a piece of a dead face".into());
            };
            let ls: Vec<String> = pc.loops.iter().map(|l| keys.lp(l)).collect();
            pieces.push(format!("{{\"face\":{j},\"loops\":[{}]}}", ls.join(",")));
        }
        operands.push(format!(
            "{{\"faces\":[{}],\"pieces\":[{}]}}",
            faces.join(","),
            pieces.join(",")
        ));
    }
    let mut rfaces = Vec::new();
    for f in res.face_ids() {
        let (o, n) = plane(res, f)?;
        let src = planes
            .iter()
            .find(|(_, _, p)| bits(p.0) == bits(o) && bits(p.1) == bits(n))
            .map_or((9999, 0), |&(i, j, _)| (i, j));
        let ls: Vec<String> = loops(res, f).iter().map(|l| keys.lp(l)).collect();
        rfaces.push(format!(
            "{{\"src\":[{},{}],\"o\":{},\"n\":{},\"loops\":[{}]}}",
            src.0,
            src.1,
            p3(o),
            p3(n),
            ls.join(",")
        ));
    }
    let ks: Vec<String> = keys.points.iter().map(|&p| p3(p)).collect();
    Ok(format!(
        "{{\"linear\":{},\"op\":\"{}\",\"frames\":\"{frames}\",\"keys\":[{}],\"operands\":[{}],\"result\":{{\"faces\":[{}]}}}}",
        exact(forge_math::tol::LINEAR),
        op_name(rec.op),
        ks.join(","),
        operands.join(","),
        rfaces.join(",")
    ))
}

/// The checker's answer.
#[derive(Clone, Debug, PartialEq)]
pub enum Verdict {
    Accepted,
    Rejected(String),
    Scope(String),
}

/// The checker executable, kept running.
pub struct Checker {
    child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
}

impl Checker {
    /// FORGE_CERTIFY, else the lake build beside this crate.
    pub fn start() -> Checker {
        let exe = std::env::var("FORGE_CERTIFY").unwrap_or_else(|_| {
            concat!(env!("CARGO_MANIFEST_DIR"), "/.lake/build/bin/forge-certify").to_string()
        });
        let mut child = Command::new(&exe)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap_or_else(|e| panic!("{exe}: {e} (run `lake build` in verify/forge-certify)"));
        let input = child.stdin.take().expect("stdin");
        let output = BufReader::new(child.stdout.take().expect("stdout"));
        Checker { child, input, output }
    }

    pub fn ask(&mut self, line: &str) -> Verdict {
        writeln!(self.input, "{line}").expect("the checker's stdin");
        self.input.flush().expect("flush");
        let mut out = String::new();
        self.output.read_line(&mut out).expect("the checker's answer");
        let field = |k: &str| {
            let key = format!("\"{k}\":\"");
            out.find(&key).map(|i| {
                let rest = &out[i + key.len()..];
                rest[..rest.find('"').unwrap_or(rest.len())].to_string()
            })
        };
        match field("verdict").as_deref() {
            Some("accepted") => Verdict::Accepted,
            Some("rejected") => Verdict::Rejected(field("reason").unwrap_or_default()),
            _ => Verdict::Scope(field("reason").unwrap_or_default()),
        }
    }
}

impl Drop for Checker {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

/// The kernel's own time for the boolean (best of `reps`), recording off.
pub fn kernel_time(rec: &Record, reps: usize) -> std::time::Duration {
    forge_bool::certify::record(false);
    let mut best = std::time::Duration::MAX;
    for _ in 0..reps {
        let mut t = rec.a.clone();
        let now = std::time::Instant::now();
        let _ = forge_bool::boolean_into(&mut t, &rec.b, rec.op, 0);
        best = best.min(now.elapsed());
    }
    best
}
