//! One STEP file under the kernel's whole workload, as JSON lines (tools/kernel-stress/run.sh):
//! the import (time, peak memory, refusals, the file's entity kinds), then every solid checked,
//! weighed and meshed, and the largest ones sectioned and cut: a box through it (subtract and
//! intersect, which must add back to the solid), a bore (the same), a copy beside it and over it
//! (union, against the intersection), and two of the file's solids that overlap.
//!
//! Every op is watched: past `--timeout` seconds the op is written as `hang` and the process ends
//! with code 3, so the runner reruns with that op in `--skip`. A panic is caught and written.
//! An `Ok` whose body fails `check`, whose mesh is open, or whose volume disagrees with the
//! oracle is `wrong`: the worst kind of answer, so it is told apart from a refusal.
//!
//! `forge-stress FILE.step [--timeout S] [--max-rss-mb M] [--max-solids N] [--skip k1,k2] [--only k1,k2] [--nary]`
//! Past `--max-rss-mb` (4000) resident, the op is written as `memory` and the run ends as for a hang.

use forge_bool::{Op, boolean};
use forge_math::{Box3, Frame, Point2, Point3, UnitVec3, Vec3};
use forge_mesh::{Quality, tessellate_tolerant};
use forge_topo::{Body, check, mass};
use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

fn esc(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_graphic() || *c == ' ')
        .map(|c| if c == '"' || c == '\\' { '\'' } else { c })
        .take(240)
        .collect()
}

fn num(x: f64) -> String {
    if x.is_finite() {
        format!("{x:.6e}")
    } else {
        "null".into()
    }
}

fn emit(s: String) {
    let mut o = std::io::stdout().lock();
    let _ = writeln!(o, "{s}");
    let _ = o.flush();
}

/// Peak and current resident memory, MB, from /proc (0 where there is none).
fn rss() -> (f64, f64) {
    let s = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    let get = |k: &str| {
        s.lines()
            .find(|l| l.starts_with(k))
            .and_then(|l| l.split_whitespace().nth(1))
            .and_then(|v| v.parse::<f64>().ok())
            .unwrap_or(0.0)
            / 1024.0
    };
    (get("VmHWM:"), get("VmRSS:"))
}

/// The op in flight and when it must end, for the watchdog.
type Armed = Arc<Mutex<Option<(String, Instant)>>>;

static PANIC: Mutex<String> = Mutex::new(String::new());

struct Ctx {
    armed: Armed,
    timeout: Duration,
    skip: BTreeSet<String>,
    only: Option<BTreeSet<String>>,
    /// Every op's time, for the slow list.
    times: Vec<(f64, String)>,
}

enum Res<T> {
    Done(T),
    Panic(String),
    Skipped,
}

impl Ctx {
    /// Runs `f` as op `key`, watched; the time is kept.
    fn run<T>(&mut self, key: &str, f: impl FnOnce() -> T) -> (Res<T>, f64) {
        // `--only` picks among the ops on solids: the import and each solid's check, mass and mesh
        // always run, as the ops' choice of solids follows from them
        let always = key == "import" || [":check", ":mass", ":mesh"].iter().any(|k| key.ends_with(k));
        let unwanted = !always
            && self
                .only
                .as_ref()
                .is_some_and(|o| !o.iter().any(|p| key.ends_with(&format!(":{p}")) || key == p));
        if self.skip.contains(key) || unwanted {
            return (Res::Skipped, 0.0);
        }
        // the op in flight, for the runner if this process dies in it (an abort, a stack overflow)
        eprintln!("BEGIN {key}");
        *self.armed.lock().unwrap() = Some((key.to_string(), Instant::now() + self.timeout));
        let t = Instant::now();
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
        let ms = t.elapsed().as_secs_f64() * 1e3;
        *self.armed.lock().unwrap() = None;
        self.times.push((ms, key.to_string()));
        match r {
            Ok(v) => (Res::Done(v), ms),
            Err(_) => (Res::Panic(PANIC.lock().unwrap().clone()), ms),
        }
    }
}

/// A body's validity in a word: "" if `check` passes, else its problems by kind.
fn problems(b: &Body) -> (usize, String) {
    let p = check(b);
    let mut kinds: BTreeMap<String, usize> = BTreeMap::new();
    for x in &p {
        let d = format!("{x:?}");
        *kinds
            .entry(d.split([' ', '{', '(']).next().unwrap_or("").to_string())
            .or_default() += 1;
    }
    let first = p
        .first()
        .map(|x| {
            let d = format!("{x:?}");
            // what the problem's edge and face are: a line on planes, a spline on a cylinder..
            let id = |tag: &str| {
                d.find(tag)
                    .and_then(|i| d[i + tag.len()..].split(')').next()?.parse::<u32>().ok())
            };
            let mut what = String::new();
            if let Some(e) = id("EdgeId(").filter(|&e| (e as usize) < b.edges.len()) {
                let x = b.edge(forge_topo::EdgeId(e));
                let k = format!("{:?}", x.curve);
                what += &format!(
                    " [edge {} tol {:e} on",
                    k.split(['(', ' ', '{']).next().unwrap_or(""),
                    x.tol
                );
                for &c in &x.coedges {
                    what += &format!(" {}", b.face(b.coedge_face(c)).surface.kind());
                }
                what += "]";
            }
            if let Some(f) = id("FaceId(").filter(|&f| (f as usize) < b.faces.len()) {
                what += &format!(" [face {}]", b.face(forge_topo::FaceId(f)).surface.kind());
            }
            esc(&format!("{d}{what}"))
        })
        .unwrap_or_default();
    let k: Vec<String> = kinds.iter().map(|(k, n)| format!("{k}:{n}")).collect();
    (
        p.len(),
        if p.is_empty() {
            String::new()
        } else {
            format!("{} | {}", k.join(","), first)
        },
    )
}

struct Meshed {
    tris: usize,
    open: usize,
    pinched: usize,
    volume: f64,
}

/// `--nary`: every boolean through `forge_bool::nary` on par's threads, as the app's builds run
/// them (forge-model), its helpers sharing the op's budget; else `forge_bool::boolean`.
static NARY: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// What the last `mesh` spent of its budget (written on the solid's line as `mesh_evals`); the op
/// runs on the watchdog's thread, so not a thread-local.
static MESH_EVALS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn mesh(b: &Body) -> Result<Meshed, String> {
    // what the mesh spends of its budget, as for a boolean: an outer budget it cannot run out of
    let outer = forge_geom::budget::scoped(u64::MAX >> 1);
    let before = forge_geom::budget::left();
    let r = tessellate_tolerant(b, &Quality::default());
    let evals = before - forge_geom::budget::left();
    drop(outer);
    MESH_EVALS.store(evals, std::sync::atomic::Ordering::Relaxed);
    let m = r.map_err(|e| format!("{e:?}"))?;
    Ok(Meshed {
        tris: m.triangles.len(),
        open: m.open_edges().len(),
        pinched: m.pinched_vertices().len(),
        volume: m.volume(),
    })
}

/// `b` moved by `d`, every tolerance kept, names salted: a copy of its own (the same names twice
/// would fail `check` whatever the boolean did).
fn translated(b: &Body, d: Vec3) -> Body {
    use forge_geom::{CurveGeom, SurfaceGeom};
    let f = Frame::WORLD.with_origin(Point3::ORIGIN + d);
    let salt = forge_topo::Name::feature(0x5eed);
    let mut c = b.clone();
    for v in &mut c.vertices {
        v.point = f.point_to_world(v.point);
        v.name = v.name.salted(salt);
    }
    for e in &mut c.edges {
        e.curve = e.curve.transformed(&f);
        e.name = e.name.salted(salt);
        e.bbox = Default::default();
    }
    for x in &mut c.faces {
        x.surface = x.surface.transformed(&f);
        x.name = x.name.salted(salt);
        x.bbox = Default::default();
        x.query = Default::default();
    }
    c.hulls = Default::default();
    c
}

/// A box `hi.z - lo.z` tall standing on `lo`, its base `hi.x - lo.x` by `hi.y - lo.y` along the
/// x and y that `Frame::from_z` picks for z, which are not the world's (for z up its x is the
/// world's y and its y the world's -x). So it is not the box from `lo` to `hi`. Kept as it is: the
/// sweeps' box ops and the corpus's rows were made with it, and any box through the solid serves.
fn block(lo: Point3, hi: Point3, id: u64) -> Option<Body> {
    let p = forge_ops::Profile {
        plane: Frame::from_z(Point3::new(lo.x, lo.y, lo.z), UnitVec3::Z),
        loops: vec![forge_ops::Profile::rectangle(
            Point2::ORIGIN,
            hi.x - lo.x,
            hi.y - lo.y,
            id,
        )],
    };
    forge_ops::extrude(&p, hi.z - lo.z, id + 10).ok()
}

fn bore(at: Point3, z: UnitVec3, r: f64, len: f64) -> Option<Body> {
    let o = at - Vec3::from(z) * (0.5 * len);
    let p = forge_ops::Profile {
        plane: Frame::from_z(o, z),
        loops: vec![forge_ops::Profile::circle(Point2::ORIGIN, r, 7001)],
    };
    forge_ops::extrude(&p, len, 7000).ok()
}

thread_local! {
    /// The error bounds `mass` gave the answers judged since the last oracle: what their volumes
    /// may be off by, honestly said (a file's tolerant edges widen it), which an oracle allows.
    static BOUND: std::cell::Cell<f64> = const { std::cell::Cell::new(0.0) };
}

/// What a boolean's answer is: its status word, its volume, and what is wrong with it.
fn judge(r: Result<Body, String>) -> (String, f64, String) {
    match r {
        Err(e) => {
            let st = if e.contains("Empty") || e.contains("empty") {
                "empty"
            } else {
                "refused"
            };
            (st.into(), f64::NAN, e)
        }
        Ok(b) => {
            if b.face_ids().next().is_none() {
                return ("ok".into(), 0.0, "no faces".into());
            }
            let (n, why) = problems(&b);
            if n > 0 {
                return ("wrong".into(), f64::NAN, format!("check: {why}"));
            }
            let v = match mass::mass_rel(&b, 1e-6) {
                Ok(m) => {
                    BOUND.with(|c| c.set(c.get() + m.err[0]));
                    m.volume
                }
                Err(e) => return ("wrong".into(), f64::NAN, format!("mass: {e}")),
            };
            match mesh(&b) {
                Err(e) => ("wrong".into(), v, format!("mesh: {e}")),
                Ok(m) if m.open > 0 || m.volume <= 0.0 => (
                    "wrong".into(),
                    v,
                    format!("mesh not watertight: open {} vol {}", m.open, m.volume),
                ),
                Ok(_) => ("ok".into(), v, String::new()),
            }
        }
    }
}

struct Ops<'a> {
    cx: &'a mut Ctx,
    s: usize,
    ent: u32,
}

impl Ops<'_> {
    fn line(&self, op: &str, ms: f64, st: &str, extra: &str, d: &str) {
        emit(format!(
            "{{\"ev\":\"op\",\"s\":{},\"ent\":{},\"op\":\"{op}\",\"ms\":{ms:.1},\"st\":\"{st}\"{extra},\"d\":\"{}\"}}",
            self.s,
            self.ent,
            esc(d)
        ));
    }

    /// A boolean, judged; its volume, or NaN.
    fn boolean(&mut self, op: &str, a: &Body, b: &Body, how: Op) -> f64 {
        let key = format!("s{}:{op}", self.s);
        // the answer's check, mass and mesh are watched with it: a body a boolean got wrong can
        // hang them as well as the boolean
        let (r, ms) = self.cx.run(&key, || {
            let t = Instant::now();
            // what the boolean spends of its work budget: an outer budget it shares, read after
            // (the distribution of what succeeding ops spend sets OP near the real tail)
            let outer = forge_geom::budget::scoped(u64::MAX >> 1);
            let before = forge_geom::budget::left();
            let r = if NARY.load(std::sync::atomic::Ordering::Relaxed) {
                forge_bool::nary(&[a, b], &[how], forge_math::par::threads())
            } else {
                boolean(a, b, how, 9000)
            }
            .map_err(|e| format!("{e:?}"));
            let spent = before - forge_geom::budget::left();
            drop(outer);
            let bms = t.elapsed().as_secs_f64() * 1e3;
            MESH_EVALS.store(0, std::sync::atomic::Ordering::Relaxed);
            let j = judge(r);
            (bms, spent, j)
        });
        // what the answer's mesh spent (0 when there was no answer to mesh)
        let mesh_evals = MESH_EVALS.load(std::sync::atomic::Ordering::Relaxed);
        match r {
            Res::Skipped => f64::NAN,
            Res::Panic(m) => {
                self.line(op, ms, "panic", "", &m);
                f64::NAN
            }
            Res::Done((bms, spent, (st, v, d))) => {
                // the boolean's own time; the check of its answer after it is in `all_ms`
                self.line(
                    op,
                    bms,
                    &st,
                    &format!(
                        ",\"vol\":{},\"all_ms\":{ms:.1},\"evals\":{spent},\"mesh_evals\":{mesh_evals}",
                        num(v)
                    ),
                    &d,
                );
                if st == "empty" { 0.0 } else { v }
            }
        }
    }

    /// `a + b` against `want`, as an oracle line: within 1e-4 of `scale`, or within the error
    /// bounds `mass` said the volumes have (`bound`, the operands', and the answers' since the
    /// last oracle). A difference inside what `mass` owns to is not a wrong answer: on a file's
    /// tolerant edge (6 um off a cylinder it meets nearly tangent, TM-1) the volume is honestly
    /// uncertain by more than 1e-4.
    fn oracle(&self, op: &str, got: f64, want: f64, scale: f64, bound: f64) {
        let bound = bound + BOUND.with(|c| c.replace(0.0));
        if !got.is_finite() || !want.is_finite() {
            return;
        }
        let err = (got - want).abs() / scale.max(1e-12);
        let st = if err <= 1e-4 || (got - want).abs() <= bound {
            "ok"
        } else {
            "wrong"
        };
        self.line(
            op,
            0.0,
            st,
            &format!(
                ",\"got\":{},\"want\":{},\"rel\":{},\"bound\":{}",
                num(got),
                num(want),
                num(err),
                num(bound)
            ),
            if st == "ok" { "" } else { "volumes do not add up" },
        );
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let opt = |f: &str| {
        args.iter()
            .position(|a| a == f)
            .and_then(|i| args.get(i + 1))
            .cloned()
    };
    let path = args.first().expect("forge-stress FILE.step").clone();
    let list = |s: Option<String>| -> BTreeSet<String> {
        s.map(|x| x.split(',').filter(|k| !k.is_empty()).map(String::from).collect())
            .unwrap_or_default()
    };
    let armed: Armed = Arc::new(Mutex::new(None));
    let mut cx = Ctx {
        armed: armed.clone(),
        timeout: Duration::from_secs_f64(opt("--timeout").and_then(|t| t.parse().ok()).unwrap_or(60.0)),
        skip: list(opt("--skip")),
        only: opt("--only").map(|s| list(Some(s))),
        times: Vec::new(),
    };
    let max_solids: usize = opt("--max-solids").and_then(|t| t.parse().ok()).unwrap_or(24);
    NARY.store(
        args.iter().any(|a| a == "--nary"),
        std::sync::atomic::Ordering::Relaxed,
    );
    std::panic::set_hook(Box::new(|i| {
        let loc = i
            .location()
            .map(|l| format!(" at {}:{}", l.file(), l.line()))
            .unwrap_or_default();
        let msg = i
            .payload()
            .downcast_ref::<&str>()
            .map(|s| s.to_string())
            .or_else(|| i.payload().downcast_ref::<String>().cloned())
            .unwrap_or_default();
        *PANIC.lock().unwrap() = format!("{msg}{loc}");
    }));
    let max_rss: f64 = opt("--max-rss-mb").and_then(|t| t.parse().ok()).unwrap_or(4000.0);
    {
        let armed = armed.clone();
        std::thread::spawn(move || {
            loop {
                std::thread::sleep(Duration::from_millis(200));
                if let Some((k, when)) = armed.lock().unwrap().clone() {
                    let (hwm, cur) = rss();
                    if Instant::now() > when {
                        emit(format!("{{\"ev\":\"hang\",\"key\":\"{k}\",\"hwm_mb\":{hwm:.0}}}"));
                        std::process::exit(3);
                    }
                    // an op that eats the machine: written, and the run ends as for a hang
                    if cur > max_rss {
                        emit(format!(
                            "{{\"ev\":\"memory\",\"key\":\"{k}\",\"rss_mb\":{cur:.0}}}"
                        ));
                        std::process::exit(3);
                    }
                }
            }
        });
    }

    let text = std::fs::read(&path).expect("read the file");
    // the file's own words for its geometry, read off the text: what a reader must meet
    let mut ents: BTreeMap<String, usize> = BTreeMap::new();
    for rec in text.split(|&c| c == b';') {
        let Some(eq) = rec.iter().position(|&c| c == b'=') else {
            continue;
        };
        if !rec.iter().take(eq).any(|&c| c == b'#') {
            continue;
        }
        let rest = &rec[eq + 1..];
        let start = rest
            .iter()
            .position(|c| !c.is_ascii_whitespace())
            .unwrap_or(rest.len());
        let rest = &rest[start..];
        let name: String = if rest.first() == Some(&b'(') {
            // a complex entity: its parts' names, sorted
            let mut parts: Vec<String> = Vec::new();
            let mut depth = 0;
            let mut word = String::new();
            for &c in rest {
                match c {
                    b'(' => {
                        if depth == 1 && !word.is_empty() {
                            parts.push(std::mem::take(&mut word));
                        }
                        depth += 1;
                    }
                    b')' => depth -= 1,
                    c if depth == 1 && (c.is_ascii_alphanumeric() || c == b'_') => word.push(c as char),
                    _ => {
                        if depth == 1 {
                            word.clear();
                        }
                    }
                }
            }
            let geo = ["SURFACE", "CURVE", "BREP", "POINT"];
            parts.retain(|p| geo.iter().any(|g| p.contains(g)));
            if parts.is_empty() {
                continue;
            }
            format!("({})", parts.join("+"))
        } else {
            rest.iter()
                .take_while(|c| c.is_ascii_alphanumeric() || **c == b'_')
                .map(|&c| c as char)
                .collect()
        };
        let geo = [
            "SURFACE",
            "CURVE",
            "BREP",
            "SHELL",
            "PCURVE",
            "LINE",
            "CIRCLE",
            "ELLIPSE",
            "HYPERBOLA",
            "PARABOLA",
            "OFFSET",
            "TRIMMED",
            "SWEPT",
            "EXTRUSION",
            "REVOLUTION",
        ];
        if geo.iter().any(|g| name.contains(g)) {
            *ents.entry(name).or_default() += 1;
        }
    }
    let ej: Vec<String> = ents.iter().map(|(k, n)| format!("\"{k}\":{n}")).collect();

    let (r, read_ms) = cx.run("import", || {
        forge_step::import(&text, &forge_step::Limits::default())
    });
    drop(text);
    let (hwm, cur) = rss();
    let imp = match r {
        Res::Skipped => return,
        Res::Panic(m) => {
            emit(format!(
                "{{\"ev\":\"import\",\"st\":\"panic\",\"ms\":{read_ms:.1},\"hwm_mb\":{hwm:.0},\"d\":\"{}\",\"ents\":{{{}}}}}",
                esc(&m),
                ej.join(",")
            ));
            return;
        }
        Res::Done(Err(e)) => {
            emit(format!(
                "{{\"ev\":\"import\",\"st\":\"error\",\"ms\":{read_ms:.1},\"hwm_mb\":{hwm:.0},\"d\":\"{}\",\"ents\":{{{}}}}}",
                esc(&e.to_string()),
                ej.join(",")
            ));
            return;
        }
        Res::Done(Ok(i)) => i,
    };
    let mut surf: BTreeMap<&str, usize> = BTreeMap::new();
    let mut curv: BTreeMap<String, usize> = BTreeMap::new();
    for s in &imp.solids {
        for f in s.body.face_ids() {
            *surf.entry(s.body.face(f).surface.kind()).or_default() += 1;
        }
        for e in s.body.edge_ids() {
            let d = format!("{:?}", s.body.edge(e).curve);
            *curv
                .entry(d.split(['(', ' ', '{']).next().unwrap_or("").to_string())
                .or_default() += 1;
        }
    }
    let refs: Vec<String> = imp
        .refusals
        .iter()
        .map(|r| {
            format!(
                "{{\"ent\":{},\"kind\":\"{}\",\"why\":\"{}\"}}",
                r.entity,
                esc(&r.kind),
                esc(&r.why)
            )
        })
        .collect();
    let st = &imp.stats;
    emit(format!(
        "{{\"ev\":\"import\",\"st\":\"ok\",\"ms\":{read_ms:.1},\"hwm_mb\":{hwm:.0},\"rss_mb\":{cur:.0},\"schema\":\"{}\",\"system\":\"{}\",\"solids\":{},\"faces\":{},\"edges\":{},\"assembly\":{},\"tolerant\":{},\"maxtol\":{},\"voff\":{},\"coff\":{},\"surf\":{{{}}},\"curv\":{{{}}},\"ents\":{{{}}},\"refusals\":[{}]}}",
        esc(&imp.schema),
        esc(&imp.system),
        imp.solids.len(),
        st.faces,
        st.edges,
        st.assembly,
        st.tolerant_edges,
        num(st.max_edge_tol),
        num(st.vertex_off_curve),
        num(st.curve_off_surface),
        surf.iter()
            .map(|(k, n)| format!("\"{k}\":{n}"))
            .collect::<Vec<_>>()
            .join(","),
        curv.iter()
            .map(|(k, n)| format!("\"{k}\":{n}"))
            .collect::<Vec<_>>()
            .join(","),
        ej.join(","),
        refs.join(",")
    ));

    // every solid: check, mass, mesh
    let mut good: Vec<(usize, f64, f64, Box3)> = Vec::new();
    for (i, s) in imp.solids.iter().enumerate() {
        let b = &s.body;
        let nv = b.vertex_ids().count();
        let (nf, ne) = (b.face_ids().count(), b.edge_ids().count());
        let (r, cms) = cx.run(&format!("s{i}:check"), || problems(b));
        let (nprob, why) = match r {
            Res::Done(x) => x,
            Res::Panic(m) => (usize::MAX, format!("panic: {m}")),
            Res::Skipped => (usize::MAX, "skipped".into()),
        };
        let (r, mms) = cx.run(&format!("s{i}:mass"), || {
            mass::mass_rel(b, 1e-6).map_err(|e| e.to_string())
        });
        let (vol, area, merr, verr) = match r {
            Res::Done(Ok(m)) => (m.volume, m.area, String::new(), m.err[0]),
            Res::Done(Err(e)) => (f64::NAN, f64::NAN, e, f64::NAN),
            Res::Panic(m) => (f64::NAN, f64::NAN, format!("panic: {m}"), f64::NAN),
            Res::Skipped => (f64::NAN, f64::NAN, "skipped".into(), f64::NAN),
        };
        MESH_EVALS.store(0, std::sync::atomic::Ordering::Relaxed);
        let (r, tms) = cx.run(&format!("s{i}:mesh"), || mesh(b));
        let mesh_evals = MESH_EVALS.load(std::sync::atomic::Ordering::Relaxed);
        let (watertight, manifold, mvol, tris, mrr) = match r {
            Res::Done(Ok(m)) => (
                m.open == 0 && m.tris >= 4 && m.volume > 0.0,
                m.open == 0 && m.pinched == 0,
                m.volume,
                m.tris,
                String::new(),
            ),
            Res::Done(Err(e)) => (false, false, f64::NAN, 0, e),
            Res::Panic(m) => (false, false, f64::NAN, 0, format!("panic: {m}")),
            Res::Skipped => (false, false, f64::NAN, 0, "skipped".into()),
        };
        let bx = b.faces_box();
        emit(format!(
            "{{\"ev\":\"solid\",\"s\":{i},\"ent\":{},\"name\":\"{}\",\"faces\":{nf},\"edges\":{ne},\"verts\":{nv},\"shells\":{},\"problems\":{},\"why\":\"{}\",\"vol\":{},\"area\":{},\"merr\":\"{}\",\"watertight\":{watertight},\"manifold\":{manifold},\"mesh_vol\":{},\"tris\":{tris},\"mesh_err\":\"{}\",\"unverified\":\"{}\",\"check_ms\":{cms:.1},\"mass_ms\":{mms:.1},\"mesh_ms\":{tms:.1},\"mesh_evals\":{},\"diag\":{}}}",
            s.entity,
            esc(&s.name),
            b.shells.len(),
            if nprob == usize::MAX { -1 } else { nprob as i64 },
            esc(&why),
            num(vol),
            num(area),
            esc(&merr),
            num(mvol),
            esc(&mrr),
            esc(b.unverified.as_deref().unwrap_or("")),
            mesh_evals,
            num(bx.diagonal())
        ));
        if nprob == 0 && vol.is_finite() && vol > 0.0 && watertight {
            good.push((i, vol, verr, bx));
        }
    }

    // the heaviest solids by faces, then every other one evenly, to `max_solids`
    let mut pick: Vec<usize> = good.iter().map(|g| g.0).collect();
    pick.sort_by_key(|&i| std::cmp::Reverse(imp.solids[i].body.face_ids().count()));
    let half = max_solids / 2;
    let mut chosen: Vec<usize> = pick.iter().take(half).copied().collect();
    let rest: Vec<usize> = pick.iter().skip(half).copied().collect();
    if !rest.is_empty() {
        let want = max_solids - chosen.len();
        let step = (rest.len() as f64 / want.max(1) as f64).max(1.0);
        let mut k = 0.0;
        while (k as usize) < rest.len() && chosen.len() < max_solids {
            chosen.push(rest[k as usize]);
            k += step;
        }
    }
    let info: BTreeMap<usize, (f64, f64, Box3)> = good.iter().map(|g| (g.0, (g.1, g.2, g.3))).collect();
    for &i in &chosen {
        let s = &imp.solids[i];
        let b = &s.body;
        let (vol, verr, bx) = info[&i];
        let c = bx.center();
        let (w, h, d) = (bx.x.width(), bx.y.width(), bx.z.width());
        let small = w.min(h).min(d).max(1e-3);
        let m = 0.1 * bx.diagonal() + 1.0;
        let mut o = Ops {
            cx: &mut cx,
            s: i,
            ent: s.entity,
        };
        // sections: through the middle on each axis, and off the middle
        for (name, at, z) in [
            ("sec_x", c, UnitVec3::X),
            ("sec_y", c, UnitVec3::Y),
            ("sec_z", c, UnitVec3::Z),
            ("sec_z37", Point3::new(c.x, c.y, bx.z.lo + 0.37 * d), UnitVec3::Z),
            // the middle on x moved off by a hair: a plane through a turned part's axis meets its
            // seams and poles, and a fault there and not here is the plane's
            (
                "sec_x_off",
                Point3::new(c.x + 1.37e-4 * bx.diagonal(), c.y, c.z),
                UnitVec3::X,
            ),
        ] {
            let key = format!("s{i}:{name}");
            let (r, ms) =
                o.cx.run(&key, || forge_agent::features::section(b, &Frame::from_z(at, z)));
            match r {
                Res::Skipped => {}
                Res::Panic(p) => o.line(name, ms, "panic", "", &p),
                Res::Done(Err(e)) => o.line(name, ms, "refused", "", &e),
                Res::Done(Ok(loops)) => {
                    let open = loops.iter().filter(|l| !l.closed).count();
                    let area: f64 = loops.iter().map(|l| l.area).sum();
                    // material left of a loop, outlines up: the sum is the section's area, never less than 0
                    let st = if open > 0 || area < -1e-6 * bx.diagonal() * bx.diagonal() {
                        "wrong"
                    } else {
                        "ok"
                    };
                    let why = if open > 0 {
                        format!("{open} open loops")
                    } else if st == "wrong" {
                        "negative area".into()
                    } else {
                        String::new()
                    };
                    o.line(
                        name,
                        ms,
                        st,
                        &format!(",\"loops\":{},\"area\":{}", loops.len(), num(area)),
                        &why,
                    );
                }
            }
        }
        // a box through it (see `block`: turned from the quarter off through in y this names)
        if let Some(t) = block(
            Point3::new(c.x, bx.y.lo - m, c.z),
            Point3::new(bx.x.hi + m, bx.y.hi + m, bx.z.hi + m),
            8000,
        ) {
            let sub = o.boolean("box_sub", b, &t, Op::Subtract);
            let int = o.boolean("box_int", b, &t, Op::Intersect);
            o.oracle("box_sum", sub + int, vol, vol, verr);
        }
        // a bore along z through the middle
        if let Some(t) = bore(c, UnitVec3::Z, 0.13 * small, 3.0 * bx.diagonal()) {
            let sub = o.boolean("bore_sub", b, &t, Op::Subtract);
            let int = o.boolean("bore_int", b, &t, Op::Intersect);
            o.oracle("bore_sum", sub + int, vol, vol, verr);
        }
        // a copy beside it, touching its box, and one half over it
        // copies of it moved along x: beside (touching its box), over (half on it), and half on it
        // at an ordinary offset. The box is padded by the kernel's tolerance, so beside and over
        // stand exactly that far off their twins: the tolerance's edge, adversarial on purpose;
        // over2 (0.4137 of the width) is ordinary use
        for (tag, k) in [("beside", 1.0), ("over", 0.5), ("over2", 0.4137)] {
            let copy = translated(b, Vec3::new(k * w, 0.0, 0.0));
            // the copy must be as good as the solid, or what follows says nothing of the boolean:
            // its ops are written as the harness's (an import's tolerance), not as wrong
            let (n, why) = problems(&copy);
            if n > 0 {
                for op in ["union", "int"] {
                    o.line(
                        &format!("{op}_{tag}"),
                        0.0,
                        "harness",
                        "",
                        &format!("the copy fails check: {why}"),
                    );
                }
                continue;
            }
            let u = o.boolean(&format!("union_{tag}"), b, &copy, Op::Union);
            let x = o.boolean(&format!("int_{tag}"), b, &copy, Op::Intersect);
            o.oracle(&format!("{tag}_sum"), u + x, 2.0 * vol, vol, 2.0 * verr);
        }
    }
    // two of the file's own solids whose boxes overlap: the first few such pairs among the chosen
    let mut pairs = 0;
    'outer: for (k, &i) in chosen.iter().enumerate() {
        for &j in &chosen[k + 1..] {
            let (vi, ei, bi) = info[&i];
            let (vj, ej, bj) = info[&j];
            let ov = bi.x.lo < bj.x.hi
                && bj.x.lo < bi.x.hi
                && bi.y.lo < bj.y.hi
                && bj.y.lo < bi.y.hi
                && bi.z.lo < bj.z.hi
                && bj.z.lo < bi.z.hi;
            if !ov {
                continue;
            }
            let (a, b) = (&imp.solids[i].body, &imp.solids[j].body);
            let mut o = Ops {
                cx: &mut cx,
                s: i,
                ent: imp.solids[i].entity,
            };
            let tag = format!("pair{j}");
            let u = o.boolean(&format!("union_{tag}"), a, b, Op::Union);
            let x = o.boolean(&format!("int_{tag}"), a, b, Op::Intersect);
            o.oracle(&format!("{tag}_sum"), u + x, vi + vj, vi.max(vj), ei + ej);
            pairs += 1;
            if pairs >= 6 {
                break 'outer;
            }
        }
    }
    let (hwm, cur) = rss();
    cx.times.sort_by(|a, b| b.0.total_cmp(&a.0));
    let slow: Vec<String> = cx
        .times
        .iter()
        .take(20)
        .map(|(ms, k)| format!("[\"{k}\",{ms:.1}]"))
        .collect();
    emit(format!(
        "{{\"ev\":\"done\",\"hwm_mb\":{hwm:.0},\"rss_mb\":{cur:.0},\"chosen\":{},\"slow\":[{}]}}",
        chosen.len(),
        slow.join(",")
    ));
}
