//! The `machining` document: CAM in .fg (research/cam/build.md §6).
//!
//! A machining document is a .fg file whose first statement is the bare name `machining`. Its
//! statements are the language's own (`shared`, `param`, `NAME = call(..)`), read by the same
//! parser; the calls are CAM's built-ins:
//!
//! ```text
//! machining
//! shared tol = 0.01
//! f     = part("flange.fg", group: "N")             # the part, its ISO 513 material group
//! m     = machine("generic-vmc3")
//! t1    = tool("face-50", t: 1, stickout: 30, holder: "bt40-er32")
//! t2    = tool("flat-6", t: 2, stickout: 25, holder: "bt40-er32")
//! op1   = setup(f, machine: m, stock: box(margin: (3, 3, 2)), origin: "top-center", tol: tol)
//! face  = facing(op1, tool: t1, step: 1)
//! prof  = contour(op1, tool: t2, side: "outside", from: 12, to: 0, step: 4)
//! bore  = pocket(op1, f.bore_h, tool: t2, stepover: 2.5, step: 3)
//! bolts = drill(op1, f.bolts, plan: "auto")
//! prog  = program(op1, post: "fanuc", number: 1001)
//! ```
//!
//! Every operation's path is posted, read back (the round trip), and the path the text means is
//! what is certified: the cutter against the part (gouge), the shank and holder against the part
//! and the stock, rapids against both. An op whose checks are not all `pass` is refused, with the
//! check, where, and a fix. The reply is `forgecad.cam/1`.
use forge_lang::syntax::{self, Arg, Expr, ExprKind, StmtKind};
pub mod holes;

use forge_agent::json::Json;
use forge_cam::ops::{self, CheckOut, Heights, OpPath, PMove, Phase, Stock, ToolUse};
use forge_cam_geom::check::Face;
use forge_cam_geom::region::{Loop, Region, Seg, p2};
use forge_post::{CycleKind, Rec};
use forge_tooling::{Kind, Library, feeds};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Whether a source is a machining document.
pub fn is_machining(src: &str) -> bool {
    match syntax::parse(src) {
        Ok(p) => {
            matches!(p.first().map(|s| &s.kind), Some(StmtKind::Expr(Expr { kind: ExprKind::Name(n), .. })) if n == "machining")
        }
        Err(_) => false,
    }
}

#[derive(Clone, Debug)]
enum V {
    Num(f64),
    Str(String),
    Tuple(Vec<V>),
    Part(usize),
    Machine(String),
    Tool(usize),
    Setup(usize),
    Holes(usize, String),
    /// A part's sketch, by name.
    Sketch(usize, String),
    Stock(StockSpec),
    Vise(ViseSpec),
    /// Soft jaws: their index, and the jaws as a part.
    Jaws(usize, usize),
    #[allow(dead_code)]
    Op(usize),
}

#[derive(Clone, Debug)]
struct ViseSpec {
    name: String,
    axis: usize,
    clamp: Option<f64>,
    parallel: f64,
    clearance: f64,
}

#[derive(Clone, Debug)]
enum StockSpec {
    Margin([f64; 3]),
    Box { min: [f64; 3], max: [f64; 3] },
}

struct PartIn {
    /// The part's source and where it is, to read its sketches.
    src: String,
    pdir: std::path::PathBuf,
    /// The workholding a setup of this part gets unless it names one (soft jaws, held to be cut).
    hold: Option<forge_cam::fixture::Fixture>,
    /// This is part `k` turned over about x (a later setup's view of it).
    turn_of: Option<usize>,
    file: String,
    faces: Vec<Face>,
    holes: Vec<holes::HoleInfo>,
    bbox: ([f64; 3], [f64; 3]),
    group: String,
}

struct SetupIn {
    name: String,
    part: usize,
    machine: String,
    stock: Stock,
    origin: [f64; 3],
    wcs: u32,
    tol: f64,
    clearance: f64,
    heights: Heights,
    /// The work offset: the machine coordinates of the WCS origin (as G54 holds them; the
    /// tool's length comes from H). None: not placed (G5 then checks the spans only).
    at: Option<[f64; 3]>,
    /// The in-process stock as the ops so far left it.
    ip: forge_cam::stock::InProcess,
    /// The workholding, if the setup names one.
    fixture: Option<forge_cam::fixture::Fixture>,
    /// The setup this one continues, the part turned over.
    from: Option<usize>,
}

/// One operation, as made and as checked.
pub struct OpOut {
    pub name: String,
    pub kind: &'static str,
    pub setup: usize,
    pub tool: Option<usize>,
    pub paths: Vec<OpPath>,
    pub feeds: Option<feeds::Feeds>,
    pub checks: Vec<CheckOut>,
    pub pairs: usize,
    pub cells: usize,
    pub status: &'static str,
    pub refusal: Option<String>,
    pub fix: Option<String>,
    pub moves: usize,
    pub length_cut: f64,
    pub length_rapid: f64,
    pub time_s: f64,
    pub least: f64,
    pub plan: Vec<String>,
    /// The stock it is asked to leave on the part (δ).
    pub leave: f64,
    /// The target it is certified against when not the part: P ∖ Δ for declared removals.
    target: Option<Vec<Face>>,
    /// Declared removals (Δ): what each is and where.
    pub removals: Vec<String>,
    /// The stock this op starts from (G2, G3 are checked against it).
    stock_before: Option<forge_cam::stock::InProcess>,
    /// What the op provably clears: (region, or the whole box; its final floor).
    voids: Vec<(Option<Region>, f64)>,
}

pub struct ProgramOut {
    pub name: String,
    pub setups: Vec<usize>,
    pub post: String,
    pub text: Option<String>,
    pub lines: usize,
    pub error: Option<String>,
    pub round_trip: bool,
}

/// G4 for one setup: in each height band, the final stock less the part grown by the
/// tolerance: what is left. Empty bands are omitted.
#[derive(Clone, Debug)]
pub struct Leftover {
    pub setup: usize,
    pub z0: f64,
    pub z1: f64,
    /// Each left-over region's box, xmin ymin xmax ymax.
    pub boxes: Vec<[f64; 4]>,
    /// "held by fixture" (the band is within the jaws' grip: no op of this setup may reach it)
    /// or "uncut".
    pub why: &'static str,
}

pub struct Cam {
    pub leftover: Vec<Leftover>,
    pub leftover_done: bool,
    pub ok: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    parts: Vec<PartIn>,
    tools: Vec<ToolUse>,
    setups: Vec<SetupIn>,
    pub ops: Vec<OpOut>,
    pub programs: Vec<ProgramOut>,
    lib: Library,
    jaws: Vec<forge_cam::fixture::SoftJaws>,
}

impl PartIn {
    /// Its lowest plus its highest z: turning over about x maps z to h − z.
    fn h(&self) -> f64 {
        self.bbox.0[2] + self.bbox.1[2]
    }
}

fn num(v: &V) -> Option<f64> {
    if let V::Num(x) = v { Some(*x) } else { None }
}

fn tuple3(v: &V) -> Option<[f64; 3]> {
    match v {
        V::Tuple(t) if t.len() == 3 => Some([num(&t[0])?, num(&t[1])?, num(&t[2])?]),
        _ => None,
    }
}

impl Cam {
    /// The part turned over about x, made once.
    fn turned_part(&mut self, i: usize) -> Result<usize, String> {
        if let Some(k) = self.parts.iter().position(|p| p.turn_of == Some(i)) {
            return Ok(k);
        }
        let p = &self.parts[i];
        let h = p.h();
        let faces =
            forge_cam::lower::turned(&p.faces, h).map_err(|e| format!("turning the part over: {e:?}"))?;
        let (lo, hi) = p.bbox;
        let turned = PartIn {
            src: p.src.clone(),
            pdir: p.pdir.clone(),
            hold: None,
            turn_of: Some(i),
            file: p.file.clone(),
            faces,
            // a hole is drilled from the side it opens on, in the setup that sees it so
            holes: vec![],
            bbox: ([lo[0], -hi[1], h - hi[2]], [hi[0], -lo[1], h - lo[2]]),
            group: p.group.clone(),
        };
        self.parts.push(turned);
        Ok(self.parts.len() - 1)
    }

    fn eval(&self, e: &Expr, env: &BTreeMap<String, V>) -> Result<V, String> {
        Ok(match &e.kind {
            ExprKind::Num(x, _) => V::Num(*x),
            ExprKind::Str(s) => V::Str(s.clone()),
            ExprKind::Name(n) => env.get(n).cloned().ok_or_else(|| format!("unknown name `{n}`"))?,
            ExprKind::Tuple(xs) | ExprKind::List(xs) => {
                V::Tuple(xs.iter().map(|x| self.eval(x, env)).collect::<Result<_, _>>()?)
            }
            ExprKind::Neg(x) => V::Num(-num(&self.eval(x, env)?).ok_or("`-` of a non-number")?),
            ExprKind::Bin(op, a, b) => {
                let (a, b) = (
                    num(&self.eval(a, env)?).ok_or("arithmetic on a non-number")?,
                    num(&self.eval(b, env)?).ok_or("arithmetic on a non-number")?,
                );
                V::Num(match op {
                    syntax::BinOp::Add => a + b,
                    syntax::BinOp::Sub => a - b,
                    syntax::BinOp::Mul => a * b,
                    syntax::BinOp::Div => a / b,
                    _ => return Err("only + - * / in a machining document".into()),
                })
            }
            ExprKind::Attr(x, name) => match self.eval(x, env)? {
                V::Part(i) => {
                    if self.parts[i].holes.iter().any(|h| &h.name == name) {
                        V::Holes(i, name.clone())
                    } else if forge_lang::sketch_view(
                        &self.parts[i].src,
                        name,
                        forge_lang::Options {
                            dir: Some(self.parts[i].pdir.clone()),
                            ..Default::default()
                        },
                    )
                    .is_some()
                    {
                        V::Sketch(i, name.clone())
                    } else {
                        let have: Vec<&str> = self.parts[i].holes.iter().map(|h| h.name.as_str()).collect();
                        return Err(format!(
                            "the part has no hole feature `{name}` (it has: {})",
                            have.join(", ")
                        ));
                    }
                }
                _ => return Err(format!("`.{name}` of something that is not a part")),
            },
            ExprKind::Call(f, args) => {
                let ExprKind::Name(fname) = &f.kind else {
                    return Err("a call of something that is not a built-in".into());
                };
                if fname == "box" {
                    let named = self.named(args, env)?;
                    if let Some(m) = named.get("margin").and_then(tuple3) {
                        return Ok(V::Stock(StockSpec::Margin(m)));
                    }
                    if let (Some(lo), Some(hi)) = (
                        named.get("min").and_then(tuple3),
                        named.get("max").and_then(tuple3),
                    ) {
                        return Ok(V::Stock(StockSpec::Box { min: lo, max: hi }));
                    }
                    return Err("box(margin: (x, y, z)) or box(min: (..), max: (..))".into());
                }
                if fname == "vise" {
                    let a = self.named(args, env)?;
                    let name = match a.get("#0") {
                        Some(V::Str(s)) => s.clone(),
                        _ => {
                            return Err(
                                "vise(\"NAME\", across: \"x\", clamp: MM, parallel: MM, clearance: MM)"
                                    .into(),
                            );
                        }
                    };
                    self.lib.vise(&name).map_err(|e| format!("{e:?}"))?;
                    let axis = match get_str(&a, "across", Some("x"))?.as_str() {
                        "x" => 0,
                        "y" => 1,
                        o => {
                            return Err(format!(
                                "across: \"x\" or \"y\" (the jaws close along it), not `{o}`"
                            ));
                        }
                    };
                    return Ok(V::Vise(ViseSpec {
                        name,
                        axis,
                        clamp: match a.get("clamp") {
                            None => None,
                            Some(V::Num(x)) => Some(*x),
                            Some(_) => return Err("`clamp` must be a number".into()),
                        },
                        parallel: get_num(&a, "parallel", Some(6.0))?,
                        clearance: get_num(&a, "clearance", Some(1.0))?,
                    }));
                }
                return Err(format!(
                    "`{fname}` is a statement's built-in here: give it its own name"
                ));
            }
            _ => return Err("an expression a machining document does not take".into()),
        })
    }

    fn named(&self, args: &[Arg], env: &BTreeMap<String, V>) -> Result<BTreeMap<String, V>, String> {
        let mut m = BTreeMap::new();
        let mut k = 0;
        for a in args {
            let v = self.eval(&a.value, env)?;
            match &a.name {
                Some((n, _)) => {
                    m.insert(n.clone(), v);
                }
                None => {
                    m.insert(format!("#{k}"), v);
                    k += 1;
                }
            }
        }
        Ok(m)
    }
}

fn get_num(m: &BTreeMap<String, V>, k: &str, default: Option<f64>) -> Result<f64, String> {
    match m.get(k) {
        Some(V::Num(x)) => Ok(*x),
        Some(_) => Err(format!("`{k}` must be a number")),
        None => default.ok_or_else(|| format!("missing `{k}:`")),
    }
}
fn get_str(m: &BTreeMap<String, V>, k: &str, default: Option<&str>) -> Result<String, String> {
    match m.get(k) {
        Some(V::Str(x)) => Ok(x.clone()),
        Some(_) => Err(format!("`{k}` must be a string")),
        None => default
            .map(str::to_string)
            .ok_or_else(|| format!("missing `{k}:`")),
    }
}

/// Runs a machining document (its file's directory finds the part).
pub fn run(src: &str, dir: &Path) -> Cam {
    let mut cam = Cam {
        leftover: vec![],
        leftover_done: false,
        ok: true,
        errors: vec![],
        warnings: vec![],
        parts: vec![],
        tools: vec![],
        setups: vec![],
        ops: vec![],
        programs: vec![],
        lib: Library::builtin(),
        jaws: vec![],
    };
    let prog = match syntax::parse(src) {
        Ok(p) => p,
        Err(e) => {
            cam.ok = false;
            cam.errors.push(format!("{e:?}"));
            return cam;
        }
    };
    if !is_machining(src) {
        cam.ok = false;
        cam.errors
            .push("not a machining document: its first statement must be `machining`".into());
        return cam;
    }
    // a project library beside the document
    let proj = dir.join("cam");
    if let Ok(rd) = forge_dirs::fs::read_dir(&proj) {
        let mut files: Vec<PathBuf> = rd
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x == "toml"))
            .collect();
        files.sort();
        for f in files {
            if let Ok(s) = forge_dirs::fs::read_to_string(&f)
                && let Err(e) = cam.lib.add(&f.display().to_string(), &s)
            {
                cam.errors.push(e.to_string());
                cam.ok = false;
            }
        }
    }
    let mut env: BTreeMap<String, V> = BTreeMap::new();
    let mut params: BTreeMap<String, BTreeMap<String, Expr>> = BTreeMap::new();
    for st in prog.iter().skip(1) {
        let r: Result<(), String> = (|| {
            match &st.kind {
                StmtKind::Shared { name, value } => {
                    let v = cam.eval(value, &env)?;
                    env.insert(name.clone(), v);
                }
                StmtKind::Param { owner, name, value } => {
                    let o = owner
                        .iter()
                        .map(|s| format!("{s:?}"))
                        .collect::<Vec<_>>()
                        .join(".");
                    let o = o.trim_matches('"').to_string();
                    params.entry(o).or_default().insert(name.clone(), value.clone());
                }
                StmtKind::Assign { name, value } => {
                    let ExprKind::Call(f, args) = &value.kind else {
                        let v = cam.eval(value, &env)?;
                        env.insert(name.clone(), v);
                        return Ok(());
                    };
                    let ExprKind::Name(fname) = &f.kind else {
                        return Err("a call of a non-built-in".into());
                    };
                    // the statement's own params, by their short names
                    let mut local = env.clone();
                    for (owner, ps) in &params {
                        if owner.ends_with(name.as_str()) {
                            for (k, e) in ps {
                                let v = cam.eval(e, &env)?;
                                local.insert(k.clone(), v);
                            }
                        }
                    }
                    let a = cam.named(args, &local)?;
                    // the clock only when asked: std's Instant panics in a browser
                    let t0 = std::env::var("FORGE_CAM_TIME").is_ok().then(std::time::Instant::now);
                    let v = cam.call(name, fname, &a, dir)?;
                    if let Some(t0) = t0 {
                        eprintln!("{name}: {:?}", t0.elapsed());
                    }
                    env.insert(name.clone(), v);
                }
                StmtKind::Expr(_) => {}
                _ => return Err("a statement a machining document does not take".into()),
            }
            Ok(())
        })();
        if let Err(e) = r {
            cam.ok = false;
            cam.errors
                .push(format!("line {}: {e}", line_of(src, st.span.lo as usize)));
        }
    }
    // an op no program posted has no certificate: its checks run on the posted path only
    let posted: Vec<usize> = cam
        .programs
        .iter()
        .filter(|p| p.round_trip)
        .flat_map(|p| p.setups.clone())
        .collect();
    for o in &mut cam.ops {
        if o.status == "built" && !posted.contains(&o.setup) {
            o.status = "unposted";
            o.refusal =
                Some("not certified: no program(..) posts this setup (checks run on the posted path)".into());
        }
    }
    cam.leftover();
    for s in &cam.setups {
        if s.at.is_none() {
            cam.warnings.push(format!(
                "setup `{}` is not placed (`at:` the work offset): G5 checked each axis's span against the travel, not the positions",
                s.name
            ));
        }
    }
    if cam.ops.iter().any(|o| o.status != "built") || cam.programs.iter().any(|p| p.error.is_some()) {
        cam.ok = false;
    }
    cam
}

fn line_of(src: &str, at: usize) -> usize {
    src[..at.min(src.len())].bytes().filter(|b| *b == b'\n').count() + 1
}

impl Cam {
    /// G4: S_end ⊆ (P ⊕ B_{tol}) ∪ L, L listed. In each band between the stock's and the part's
    /// heights: the stock's material region less the part's section grown by tol, exactly.
    fn leftover(&mut self) {
        let mut all_ok = true;
        for (si, s) in self.setups.iter().enumerate() {
            // a setup a later one continues (the part turned over) ends in that one: its stock,
            // turned, is where the later one starts, so G4 is decided at the chain's end
            if self.setups.iter().any(|t| t.from == Some(si)) {
                continue;
            }
            let part = &self.parts[s.part];
            // G4 grows the part by δ + ε: the stock the chain's ops were asked to leave (the
            // most of them), and the tolerance
            let mut chain = vec![si];
            while let Some(k) = self.setups[*chain.last().unwrap()].from {
                chain.push(k);
            }
            let delta = self
                .ops
                .iter()
                .filter(|o| chain.contains(&o.setup))
                .map(|o| o.leave)
                .fold(0.0, f64::max);
            let grow = delta + s.tol;
            let mut zs = s.ip.levels();
            for f in &part.faces {
                if let Some(forge_cam_geom::slab::Prism::Flat { z, .. }) = &f.prism {
                    zs.push(*z);
                }
            }
            if let Some(f) = &s.fixture {
                zs.push(f.jaw_top);
            }
            zs.retain(|z| *z >= s.ip.min[2] && *z <= s.ip.max[2]);
            zs.sort_by(|a, b| a.partial_cmp(b).unwrap());
            // exact: a band ends at the jaws' top exactly, so none straddles it
            zs.dedup();
            for w in zs.windows(2) {
                let zm = 0.5 * (w[0] + w[1]);
                let Ok(Some(mat)) = s.ip.material(zm) else {
                    if s.ip.material(zm).is_err() {
                        all_ok = false;
                    }
                    continue;
                };
                // the part's section for this band, and how far it may be grown. Its own, by
                // δ + ε; or, where it has none (outside the part's heights) or none can be taken
                // (a band thinner than the section's resolution at a face), a section at z′ just
                // past either end, by √((δ + ε)² − dz²), dz the band's farthest height from z′: a
                // point within that of (x, y, z′) ∈ P is within δ + ε of P (P ⊕ B is
                // three-dimensional), whatever z′ is
                let e = 0.5 * grow;
                let mut tries = vec![(zm, grow)];
                for zp in [w[0] - e, w[1] + e] {
                    let dz = (zp - w[0]).abs().max((zp - w[1]).abs());
                    let g2 = grow * grow - dz * dz;
                    if g2 > 0.0 {
                        tries.push((zp, (g2.sqrt() * (1.0 - 1e-12)).max(0.0)));
                    }
                }
                let mut found = None;
                let mut failed = false;
                for (zp, g) in tries {
                    match forge_cam::section::section(&part.faces, zp, 1e-5) {
                        Ok(sec) => {
                            if std::env::var("FORGE_CAM_DEBUG").is_ok() {
                                eprintln!("G4 band {w:?}: section at {zp}, grown {g}");
                            }
                            found = Some((sec, g));
                            break;
                        }
                        Err(forge_cam::section::SectionError::Empty) => {}
                        Err(e) => {
                            if std::env::var("FORGE_CAM_DEBUG").is_ok() {
                                eprintln!("G4 at z {zp}: section {e:?}");
                            }
                            failed = true;
                        }
                    }
                }
                let left = match found {
                    Some((sec, g)) => match forge_cam_geom::offset::offset(&sec, -g) {
                        Ok((grown, _)) => forge_cam_geom::boolean::boolean(
                            &mat,
                            &forge_cam_geom::boolean::region_of(&grown),
                            forge_cam_geom::boolean::Op::Diff,
                        ),
                        Err(e) => Err(e),
                    },
                    None if failed => {
                        all_ok = false;
                        continue;
                    }
                    // no part at or near this height: all the material is left over
                    None => Ok(mat
                        .loops
                        .iter()
                        .map(|l| forge_cam_geom::offset::OLoop { segs: l.segs.clone() })
                        .collect()),
                };
                if let Err(e) = &left
                    && std::env::var("FORGE_CAM_DEBUG").is_ok()
                {
                    eprintln!("G4 band {w:?}: {e:?}");
                }
                match left {
                    Ok(ls) if !ls.is_empty() => {
                        let boxes = ls
                            .iter()
                            .map(|l| {
                                let mut b =
                                    [f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY];
                                for sg in &l.segs {
                                    for p in [sg.start(), sg.end()] {
                                        b = [b[0].min(p.x), b[1].min(p.y), b[2].max(p.x), b[3].max(p.y)];
                                    }
                                    // an arc may bulge past its ends: its circle's box bounds it
                                    if let Seg::Arc { a, c, .. } = *sg {
                                        let r = ((a.x - c.x).powi(2) + (a.y - c.y).powi(2)).sqrt();
                                        b = [
                                            b[0].min(c.x - r),
                                            b[1].min(c.y - r),
                                            b[2].max(c.x + r),
                                            b[3].max(c.y + r),
                                        ];
                                    }
                                }
                                b
                            })
                            .collect();
                        // held by the fixture: under the jaws' top, and within the fixture's
                        // clearance of one of its pieces (where no tool may go)
                        let held = |b: &[f64; 4]| match &s.fixture {
                            Some(f) if w[1] <= f.jaw_top => f.boxes().iter().any(|(_, lo, hi)| {
                                let c = f.clearance;
                                b[0] <= hi[0] + c
                                    && b[2] >= lo[0] - c
                                    && b[1] <= hi[1] + c
                                    && b[3] >= lo[1] - c
                                    && w[0] <= hi[2] + c
                                    && w[1] >= lo[2] - c
                            }),
                            _ => false,
                        };
                        let boxes: Vec<[f64; 4]> = boxes;
                        let (hb, ub): (Vec<[f64; 4]>, Vec<[f64; 4]>) =
                            boxes.into_iter().partition(|b| held(b));
                        for (boxes, why) in [(hb, "held by fixture"), (ub, "uncut")] {
                            if !boxes.is_empty() {
                                self.leftover.push(Leftover {
                                    setup: si,
                                    z0: w[0],
                                    z1: w[1],
                                    boxes,
                                    why,
                                });
                            }
                        }
                    }
                    Ok(_) => {}
                    Err(_) => all_ok = false,
                }
            }
        }
        self.leftover_done = all_ok;
    }

    fn call(&mut self, name: &str, f: &str, a: &BTreeMap<String, V>, dir: &Path) -> Result<V, String> {
        match f {
            "part" => {
                let file = get_str(a, "#0", None)?;
                let path = dir.join(&file);
                let src = forge_dirs::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
                let (run, _) = forge_lang::run(&src, forge_lang::Options::default(), Default::default());
                let holes = holes::hole_view(&src, forge_lang::Options::default());
                if !run.ok() {
                    return Err(format!(
                        "the part `{file}` does not build: {:?}",
                        run.errors.first()
                    ));
                }
                let body = run.body.ok_or("the part has no body")?;
                let faces = forge_cam::lower::lower(&body)
                    .map_err(|e| format!("the part is outside this phase's CAM domain: {e:?}"))?;
                let mut lo = [f64::INFINITY; 3];
                let mut hi = [f64::NEG_INFINITY; 3];
                for fc in &faces {
                    for k in 0..3 {
                        lo[k] = lo[k].min(fc.bbox[k].lo + fc.mu);
                        hi[k] = hi[k].max(fc.bbox[k].hi - fc.mu);
                    }
                }
                self.parts.push(PartIn {
                    src: src.clone(),
                    pdir: path.parent().map(|p| p.to_path_buf()).unwrap_or_default(),
                    hold: None,
                    turn_of: None,
                    file,
                    faces,
                    holes,
                    bbox: (lo, hi),
                    group: get_str(a, "group", Some("N"))?,
                });
                Ok(V::Part(self.parts.len() - 1))
            }
            "machine" => {
                let n = get_str(a, "#0", None)?;
                self.lib.machine(&n).map_err(|e| e.to_string())?;
                Ok(V::Machine(n))
            }
            "tool" => {
                let id = get_str(a, "#0", None)?;
                let t = self.lib.tool(&id).map_err(|e| e.to_string())?.clone();
                let tn = get_num(a, "t", None)? as u32;
                let stickout = get_num(a, "stickout", Some(t.lcf + 5.0))?;
                let holder = match a.get("holder") {
                    Some(V::Str(h)) => Some(self.lib.holder(h).map_err(|e| e.to_string())?.clone()),
                    _ => None,
                };
                let shape = t
                    .shape(stickout, holder.as_ref())
                    .map_err(|e| format!("tool `{id}`: {e:?}"))?;
                self.tools.push(ToolUse {
                    t: tn,
                    tool: t,
                    holder: holder.map(|h| h.id),
                    stickout,
                    shape,
                });
                Ok(V::Tool(self.tools.len() - 1))
            }
            "setup" => {
                let mut part = match a.get("#0") {
                    Some(V::Part(i)) | Some(V::Jaws(_, i)) => *i,
                    _ => return Err("setup(PART, ..)".into()),
                };
                // a later setup: the part turned over about x, as the earlier one left it
                let from = match a.get("from") {
                    None => None,
                    Some(V::Setup(k)) => {
                        if self.setups[*k].part != part {
                            return Err("from: a setup of the same part".into());
                        }
                        Some(*k)
                    }
                    Some(_) => {
                        return Err("from: SETUP (the setup this one continues, the part turned over)".into());
                    }
                };
                if from.is_some() {
                    part = self.turned_part(part)?;
                }
                let machine = match a.get("machine") {
                    Some(V::Machine(m)) => m.clone(),
                    _ => "generic-vmc3".into(),
                };
                let (plo, phi) = self.parts[part].bbox;
                let stock = match a.get("stock") {
                    None if from.is_some() => {
                        let ip = self.setups[from.unwrap()].ip.turned(self.parts[part].h());
                        Stock {
                            min: ip.min,
                            max: ip.max,
                        }
                    }
                    Some(V::Stock(StockSpec::Margin(m))) => Stock {
                        min: [plo[0] - m[0], plo[1] - m[1], plo[2]],
                        max: [phi[0] + m[0], phi[1] + m[1], phi[2] + m[2]],
                    },
                    Some(V::Stock(StockSpec::Box { min, max })) => Stock { min: *min, max: *max },
                    None => Stock { min: plo, max: phi },
                    _ => return Err("stock: box(..)".into()),
                };
                let origin = match a.get("origin") {
                    Some(V::Str(s)) if s == "top-center" => [
                        0.5 * (stock.min[0] + stock.max[0]),
                        0.5 * (stock.min[1] + stock.max[1]),
                        stock.max[2],
                    ],
                    Some(V::Str(s)) if s == "top-corner" => [stock.min[0], stock.min[1], stock.max[2]],
                    Some(v) => tuple3(v)
                        .ok_or("origin: \"top-center\", \"top-corner\" or (x, y, z) in the part's frame")?,
                    None => [
                        0.5 * (stock.min[0] + stock.max[0]),
                        0.5 * (stock.min[1] + stock.max[1]),
                        stock.max[2],
                    ],
                };
                let fixture = match a.get("fixture") {
                    None => self.parts[part].hold.clone(),
                    Some(V::Vise(v)) => {
                        let l = self.lib.vise(&v.name).map_err(|e| format!("{e:?}"))?;
                        let size = forge_cam::fixture::ViseSize {
                            jaw_width: l.jaw_width,
                            jaw_height: l.jaw_height,
                            jaw_thickness: l.jaw_thickness,
                            max_opening: l.max_opening,
                            bed_depth: l.bed_depth,
                        };
                        let clamp = v
                            .clamp
                            .ok_or("fixture: vise(.., clamp: MM): how much of the stock the jaws grip")?;
                        Some(
                            forge_cam::fixture::vise(
                                &v.name,
                                size,
                                &stock,
                                v.axis,
                                clamp,
                                v.parallel,
                                v.clearance,
                            )
                            .map_err(|e| format!("fixture: {e}"))?,
                        )
                    }
                    Some(V::Jaws(j, _)) => {
                        if from.is_none() {
                            return Err("fixture: soft jaws hold the part as an earlier setup left it: give `from: SETUP`".into());
                        }
                        Some(self.jaws[*j].fixture.clone())
                    }
                    Some(_) => return Err("fixture: vise(\"NAME\", across:, clamp:)".into()),
                };
                let clear = stock.max[2] + get_num(a, "clear", Some(10.0))?;
                let retract = stock.max[2] + get_num(a, "retract", Some(3.0))?;
                self.setups.push(SetupIn {
                    name: name.into(),
                    part,
                    machine,
                    stock,
                    origin,
                    wcs: get_num(a, "wcs", Some(0.0))? as u32,
                    at: match a.get("at") {
                        None => None,
                        Some(v) => {
                            Some(tuple3(v).ok_or("at: (x, y, z), the WCS origin in machine coordinates")?)
                        }
                    },
                    tol: get_num(a, "tol", Some(0.01))?,
                    clearance: get_num(a, "clearance", Some(0.1))?,
                    heights: Heights { clear, retract },
                    ip: match from {
                        // what the earlier setup provably cleared, turned over, inside this stock
                        Some(k) if a.get("stock").is_none() => self.setups[k].ip.turned(self.parts[part].h()),
                        _ => forge_cam::stock::InProcess::new(stock.min, stock.max),
                    },
                    fixture,
                    from,
                });
                Ok(V::Setup(self.setups.len() - 1))
            }
            "facing" | "contour" | "pocket" | "clear" | "drill" | "engrave" | "thread_mill"
            | "edge_break" => self.op(name, f, a),
            "soft_jaw" => self.soft_jaw(name, a),
            "program" => {
                let su: Vec<usize> = match a.get("#0") {
                    Some(V::Setup(i)) => vec![*i],
                    Some(V::Tuple(t)) => t
                        .iter()
                        .filter_map(|v| if let V::Setup(i) = v { Some(*i) } else { None })
                        .collect(),
                    _ => return Err("program(SETUP or [SETUPS], post: \"fanuc\", number: N)".into()),
                };
                let post = get_str(a, "post", None).or_else(|_| -> Result<String, String> {
                    Ok(self
                        .lib
                        .machine(&self.setups[su[0]].machine)
                        .map(|m| m.control.clone())
                        .unwrap_or_else(|_| "fanuc".into()))
                })?;
                let number = get_num(a, "number", Some(1000.0))? as u32;
                let out = self.post(name, &su, &post, number);
                self.programs.push(out);
                Ok(V::Num(0.0))
            }
            _ => Err(format!(
                "`{f}` is not a machining built-in (part, machine, tool, setup, facing, contour, pocket, drill, engrave, thread_mill, edge_break, program, box)"
            )),
        }
    }

    /// Soft jaws for the part as setup `from:` left it, turned over: the pocket is what the jaw
    /// tool cuts of the part's sections in the grip band grown by `fit`; refused unless the
    /// part, and everything `from:` left in the band, seats in it, and the seat carries at least
    /// `min_contact` (default a quarter of the part's largest section).
    fn soft_jaw(&mut self, name: &str, a: &BTreeMap<String, V>) -> Result<V, String> {
        let v = match a.get("#0") {
            Some(V::Vise(v)) => v.clone(),
            _ => return Err("soft_jaw(vise(NAME, across:), blank:, from: SETUP, grip:, fit:, tool:)".into()),
        };
        let k = match a.get("from") {
            Some(V::Setup(k)) => *k,
            _ => return Err("soft_jaw(.., from: SETUP): the setup whose result the jaws hold".into()),
        };
        let tool_r = match a.get("tool") {
            Some(V::Tool(t)) => 0.5 * self.tools[*t].tool.dc,
            _ => return Err("soft_jaw(.., tool: TOOL): the tool that cuts the jaws' pocket".into()),
        };
        let vl = self.lib.vise(&v.name).map_err(|e| format!("{e:?}"))?.clone();
        let bl = self
            .lib
            .jaw_blank(&get_str(a, "blank", None)?)
            .map_err(|e| format!("{e:?}"))?
            .clone();
        let grip = get_num(a, "grip", None)?;
        let fit = get_num(a, "fit", Some(0.02))?;
        let relief = match a.get("relief") {
            None => false,
            Some(V::Str(x)) if x == "corners" => true,
            Some(_) => {
                return Err(
                    "relief: \"corners\" (a relief of the tool's radius at each of the part's corners)"
                        .into(),
                );
            }
        };
        let orig = self.setups[k].part;
        let tp = self.turned_part(orig)?;
        let part = &self.parts[tp];
        let h = part.h();
        let (seat, top) = (part.bbox.0[2], part.bbox.0[2] + grip);
        // the band's sub-bands: between the part's flat faces' heights and the levels of the
        // stock the earlier setup left (turned)
        let mut zs = vec![seat, top];
        for f in &part.faces {
            if let Some(forge_cam_geom::slab::Prism::Flat { z, .. }) = &f.prism
                && *z > seat
                && *z < top
            {
                zs.push(*z);
            }
        }
        for z in self.setups[k].ip.levels() {
            if h - z > seat && h - z < top {
                zs.push(h - z);
            }
        }
        zs.sort_by(|a, b| a.partial_cmp(b).unwrap());
        zs.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
        // the jaws grip what the earlier setup left in the band (it contains the part, and the
        // true stock): its proven material, turned over
        let mut band = vec![];
        for w in zs.windows(2) {
            let zm = 0.5 * (w[0] + w[1]);
            let m = self.setups[k]
                .ip
                .material(h - zm)
                .map_err(|e| {
                    format!(
                        "soft_jaw: the stock {} left at z {:.3}: {e}",
                        self.setups[k].name,
                        h - zm
                    )
                })?
                .ok_or_else(|| {
                    format!(
                        "soft_jaw: nothing to grip at z {:.3} of {}",
                        h - zm,
                        self.setups[k].name
                    )
                })?;
            band.push(forge_cam::stock::mirror_y(&m));
        }
        let part = &self.parts[tp];
        // reliefs: a disk of the jaw tool's radius at each convex corner of the part's sections
        // in the band and of what the earlier setup left there
        let mut relief_at = vec![];
        if relief {
            for w in zs.windows(2) {
                let zm = 0.5 * (w[0] + w[1]);
                if let Ok(sec) = forge_cam::section::section(&part.faces, zm, 1e-5) {
                    relief_at.extend(forge_cam::fixture::corners(&sec));
                }
            }
            for b in &band {
                relief_at.extend(forge_cam::fixture::corners(b));
            }
            relief_at.sort_by(|a, b| (a.x, a.y).partial_cmp(&(b.x, b.y)).unwrap());
            relief_at.dedup();
        }
        let height = part.bbox.1[2] - part.bbox.0[2];
        let sj = forge_cam::fixture::soft_jaws(
            name,
            forge_cam::fixture::ViseSize {
                jaw_width: vl.jaw_width,
                jaw_height: vl.jaw_height,
                jaw_thickness: vl.jaw_thickness,
                max_opening: vl.max_opening,
                bed_depth: vl.bed_depth,
            },
            forge_cam::fixture::BlankSize {
                width: bl.width,
                height: bl.height,
                thickness: bl.thickness,
            },
            v.axis,
            &band,
            seat,
            grip,
            height,
            fit,
            tool_r,
            &relief_at,
            v.clearance,
        )
        .map_err(|e| format!("soft_jaw: {e}"))?;
        // nothing the earlier setup left above the part's top may lie in the blank: turned over,
        // it would be inside the jaws below the seat, where the part could not go down
        // (softJaw_needs_grip)
        {
            let ip = &self.setups[k].ip;
            let top_k = h - seat;
            let b = sj.block;
            let blank = forge_cam_geom::region::Region {
                loops: vec![forge_cam_geom::region::Loop {
                    segs: {
                        let pt = |x: f64, y: f64| forge_cam_geom::region::p2(x, y);
                        let (a, bb, c, d) = (
                            pt(b.min[0], b.min[1]),
                            pt(b.max[0], b.min[1]),
                            pt(b.max[0], b.max[1]),
                            pt(b.min[0], b.max[1]),
                        );
                        vec![
                            Seg::Line { a, b: bb },
                            Seg::Line { a: bb, b: c },
                            Seg::Line { a: c, b: d },
                            Seg::Line { a: d, b: a },
                        ]
                    },
                }],
            };
            let mut zs: Vec<f64> = ip.levels().into_iter().filter(|z| *z >= top_k).collect();
            zs.insert(0, top_k);
            zs.sort_by(|a, b| a.partial_cmp(b).unwrap());
            zs.dedup();
            for w in zs.windows(2) {
                let zm = 0.5 * (w[0] + w[1]);
                let m = ip
                    .material(zm)
                    .map_err(|e| format!("soft_jaw: the stock {} left: {e}", self.setups[k].name))?;
                if let Some(m) = m {
                    let inside = forge_cam_geom::boolean::boolean(
                        &forge_cam::stock::mirror_y(&m),
                        &blank,
                        forge_cam_geom::boolean::Op::Inter,
                    )
                    .map_err(|e| format!("soft_jaw: {e:?}"))?;
                    if !inside.is_empty() {
                        return Err(format!(
                            "soft_jaw: `{}` leaves stock above the part's top (z {:.3} to {:.3}, its frame), which would sit in the jaws below the seat: face it to z {top_k:.3} in `{}` (facing(.., to: {top_k:.3}))",
                            self.setups[k].name, w[0], w[1], self.setups[k].name
                        ));
                    }
                }
            }
        }
        let largest = band.iter().map(forge_cam::fixture::area).fold(0.0, f64::max);
        let min_contact = get_num(a, "min_contact", Some(0.25 * largest))?;
        if sj.contact < min_contact {
            return Err(format!(
                "soft_jaw: the part's seat in the jaws is {:.1} mm², under the {min_contact:.1} mm² asked: grip where the part is wider, or a lower min_contact",
                sj.contact
            ));
        }
        let b = sj.block;
        self.parts.push(PartIn {
            src: String::new(),
            pdir: Default::default(),
            hold: Some(sj.hold.clone()),
            turn_of: None,
            file: format!("{name} (soft jaws, {})", bl.name),
            faces: sj.part.clone(),
            holes: vec![],
            bbox: (b.min, b.max),
            group: "N".into(),
        });
        self.jaws.push(sj);
        Ok(V::Jaws(self.jaws.len() - 1, self.parts.len() - 1))
    }

    /// The tool's tip along a sketch's curves (lines, arcs, circles; not construction), on a
    /// plane facing +z, in levels down to `depth` below it. What it clears for the stock: each
    /// line's and circle's band of the tip's radius, from the floor up (a V tool clears at least
    /// that: wider above).
    #[allow(clippy::too_many_arguments)]
    fn engrave(
        &self,
        su: usize,
        a: &BTreeMap<String, V>,
        t: &ToolUse,
        ti: usize,
        h: Heights,
        feed: (f64, f64),
        step: f64,
        voids: &mut Vec<(Option<Region>, f64)>,
    ) -> Result<Vec<OpPath>, String> {
        let (pi, sk) = match a.get("#1") {
            Some(V::Sketch(p, n)) if *p == self.setups[su].part => (*p, n.clone()),
            Some(V::Sketch(..)) => {
                return Err("engrave: the sketch is on the part the other way up: engrave it in a setup that holds it so".into());
            }
            _ => return Err("engrave(SETUP, PART.SKETCH, tool:, depth:)".into()),
        };
        let part = &self.parts[pi];
        let opts = || forge_lang::Options {
            dir: Some(part.pdir.clone()),
            ..Default::default()
        };
        // the sketch's plane, from the part's query (r9-rounded: toolpath placement only; the
        // path is certified as posted)
        let ans = forge_lang::query::ask(&part.src, Some(part.pdir.clone()), None, &format!("frame {sk}"));
        if !ans.ok {
            return Err(format!("engrave: {}", ans.text.trim()));
        }
        let v3 = |k: &str| -> Option<[f64; 3]> {
            let Json::Obj(fs) = &ans.json else { return None };
            let (_, Json::Arr(xs)) = fs.iter().find(|(n, _)| n == k)? else {
                return None;
            };
            let n = |j: &Json| if let Json::Num(x) = j { Some(*x) } else { None };
            Some([n(&xs[0])?, n(&xs[1])?, n(&xs[2])?])
        };
        let (o, x, y, z) = match (v3("origin"), v3("x"), v3("y"), v3("z")) {
            (Some(o), Some(x), Some(y), Some(z)) => (o, x, y, z),
            _ => return Err("engrave: the sketch's frame".into()),
        };
        if (z[2] - 1.0).abs() > 1e-9 || z[0].abs() > 1e-9 || z[1].abs() > 1e-9 {
            return Err("engrave: the sketch's plane must face +z in this setup".into());
        }
        let view = forge_lang::sketch_view(&part.src, &sk, opts()).ok_or("engrave: the sketch")?;
        let s = &view.s;
        let w = |p: forge_math::Point2| [o[0] + x[0] * p.x + y[0] * p.y, o[1] + x[1] * p.x + y[1] * p.y];
        // each curve: its start, and its moves as (to, Some((centre, cw)) for an arc)
        type Piece = ([f64; 2], Vec<([f64; 2], Option<([f64; 2], bool)>)>);
        let mut curves: Vec<Piece> = vec![];
        // the sketch's handedness in the part's xy: x × y along +z keeps arcs' sense
        let flip = x[0] * y[1] - x[1] * y[0] < 0.0;
        let tip = 0.5 * t.tool.tip_d.max(0.0);
        let mut bands: Vec<Region> = vec![];
        for id in s.entity_ids() {
            if s.is_construction(id) {
                continue;
            }
            match s.entity(id) {
                Some(forge_sketch::Entity::Line { a: pa, b: pb }) => {
                    let (p0, p1) = (w(s.point(pa)), w(s.point(pb)));
                    curves.push((p0, vec![(p1, None)]));
                    if tip > 0.0 {
                        bands.push(stadium(p0, p1, tip));
                    }
                }
                Some(forge_sketch::Entity::Arc {
                    center,
                    start,
                    end,
                    ccw,
                }) => {
                    let (c, p0, p1) = (w(s.point(center)), w(s.point(start)), w(s.point(end)));
                    let ccw = ccw != flip;
                    // in pieces of at most a quarter turn, as posted
                    let r = ((p0[0] - c[0]).powi(2) + (p0[1] - c[1]).powi(2)).sqrt();
                    let a0 = forge_math::dmath::atan2(p0[1] - c[1], p0[0] - c[0]);
                    let mut a1 = forge_math::dmath::atan2(p1[1] - c[1], p1[0] - c[0]);
                    let tau = std::f64::consts::TAU;
                    if ccw {
                        while a1 <= a0 {
                            a1 += tau;
                        }
                    } else {
                        while a1 >= a0 {
                            a1 -= tau;
                        }
                    }
                    let n = ((a1 - a0).abs() / std::f64::consts::FRAC_PI_2).ceil().max(1.0) as usize;
                    let mut mv = vec![];
                    for k in 1..=n {
                        let to = if k == n {
                            p1
                        } else {
                            let (sn, cs) = forge_math::dmath::sin_cos(a0 + (a1 - a0) * k as f64 / n as f64);
                            [c[0] + r * cs, c[1] + r * sn]
                        };
                        mv.push((to, Some((c, !ccw))));
                    }
                    curves.push((p0, mv));
                }
                Some(forge_sketch::Entity::Circle { center, r }) => {
                    let c = w(s.point(center));
                    let r = s.params()[r];
                    let p0 = [c[0] + r, c[1]];
                    let mv = (1..=4)
                        .map(|k| {
                            let to = match k {
                                1 => [c[0], c[1] + r],
                                2 => [c[0] - r, c[1]],
                                3 => [c[0], c[1] - r],
                                _ => p0,
                            };
                            (to, Some((c, false)))
                        })
                        .collect();
                    curves.push((p0, mv));
                    if tip > 0.0 && r > tip {
                        bands.push(ring(c, r, tip));
                    }
                }
                _ => {}
            }
        }
        if curves.is_empty() {
            return Err(format!(
                "engrave: the sketch `{sk}` has no lines, arcs or circles to follow"
            ));
        }
        let top = get_num(a, "from", Some(o[2]))?;
        let depth = get_num(a, "depth", None)?;
        // a NaN depth is refused too
        if depth.is_nan() || depth <= 0.0 {
            return Err("engrave: depth > 0".into());
        }
        let bottom = o[2] - depth;
        let mut mv = vec![];
        for z in ops::levels(top, bottom, step) {
            for (p0, ms) in &curves {
                mv.push(PMove::Rapid {
                    to: [p0[0], p0[1], h.retract],
                });
                mv.push(PMove::Line {
                    to: [p0[0], p0[1], z],
                    f: feed.1,
                    phase: Phase::Plunge,
                });
                let mut at = *p0;
                for (to, arc) in ms {
                    mv.push(match arc {
                        None => PMove::Line {
                            to: [to[0], to[1], z],
                            f: feed.0,
                            phase: Phase::Cut,
                        },
                        Some((c, cw)) => PMove::Arc {
                            to: [to[0], to[1], z],
                            c: *c,
                            cw: *cw,
                            f: feed.0,
                        },
                    });
                    at = *to;
                }
                mv.push(PMove::Line {
                    to: [at[0], at[1], h.retract],
                    f: feed.0,
                    phase: Phase::Link,
                });
            }
        }
        let first = curves[0].0;
        mv.push(PMove::Rapid {
            to: [first[0], first[1], h.clear],
        });
        // the bands it clears, united (none is claimed if the union cannot be formed)
        let mut u: Option<Region> = None;
        for b in bands {
            u = match u {
                None => Some(b),
                Some(r) => forge_cam_geom::boolean::boolean(&r, &b, forge_cam_geom::boolean::Op::Union)
                    .ok()
                    .map(|l| forge_cam_geom::boolean::region_of(&l)),
            };
            if u.is_none() {
                break;
            }
        }
        if let Some(u) = u {
            voids.push((Some(u), bottom));
        }
        Ok(vec![OpPath {
            tool: ti,
            start: [first[0], first[1], h.clear],
            moves: mv,
            rpm: 0.0,
            coolant: forge_post::Coolant::Flood,
        }])
    }

    /// Thread milling a hole's recorded thread, the model not carrying it: a declared removal Δ
    /// (cosmetic_thread) per place, the ring between the bore and the thread's major diameter
    /// over the thread's length, and the op certified against a target bounding a superset of
    /// P ∖ Δ: the bore's wall cut back below the band, Δ's outer wall and its floor added, the
    /// part's other faces as they are. The path: down the drilled bore, in to the radius, up a
    /// helix of the pitch (G3 climbing: a right-hand thread), back to the axis, out.
    #[allow(clippy::too_many_arguments)]
    fn thread_mill(
        &self,
        su: usize,
        a: &BTreeMap<String, V>,
        t: &ToolUse,
        ti: usize,
        h: Heights,
        feed: (f64, f64),
        out: &mut OpOut,
        voids: &mut Vec<(Option<Region>, f64)>,
    ) -> Result<Vec<OpPath>, String> {
        use forge_cam_geom::check::Surf;
        let (pi, hn) = match a.get("#1") {
            Some(V::Holes(p, n)) if *p == self.setups[su].part => (*p, n.clone()),
            Some(V::Holes(..)) => {
                return Err("the hole is milled from the side it opens on: in a setup that holds the part that way up".into());
            }
            _ => return Err("thread_mill(SETUP, PART.HOLE, tool:, length:)".into()),
        };
        let part = &self.parts[pi];
        let hole = part.holes.iter().find(|x| x.name == hn).unwrap().clone();
        let Some((major, pitch)) = hole.thread else {
            return Err(format!(
                "the hole `{hn}` records no thread: give it `thread:` (or a screw with fit: \"tap\")"
            ));
        };
        if hole.dir != [0.0, 0.0, -1.0] {
            return Err(format!("the hole `{hn}` is not along −z: it needs its own setup"));
        }
        if t.tool.kind != Kind::ThreadMill {
            return Err(format!("thread_mill: `{}` is not a thread mill", t.tool.id));
        }
        if (t.tool.tp - pitch).abs() > 1e-9 {
            out.fix = Some(format!("a thread mill of pitch {pitch}"));
            return Err(format!(
                "thread_mill: the tool's pitch {} is not the thread's {pitch}",
                t.tool.tp
            ));
        }
        let (rb, rm, rt) = (0.5 * hole.d, 0.5 * major, 0.5 * t.tool.dc);
        if rt >= rb {
            out.fix = Some(format!("a thread mill under {:.3} mm across", hole.d));
            return Err(format!(
                "thread_mill: the tool ({} mm) does not go down the bore ({} mm)",
                t.tool.dc, hole.d
            ));
        }
        let rc = rm - rt;
        let depth = match hole.depth {
            Some(d) => d,
            None => hole.places[0][2] - part.bbox.0[2],
        };
        let len = get_num(a, "length", Some(depth - 2.0 * pitch))?;
        if !(len > 0.0 && len <= depth) {
            return Err(format!(
                "thread_mill: length {len} must be over 0 and at most the hole's depth {depth}"
            ));
        }
        // the target: P ∖ Δ's boundary (a superset's), for every place
        let mut faces: Vec<Face> = vec![];
        let mut extra: Vec<Face> = vec![];
        let mut id = 6000u32;
        let bands: Vec<([f64; 2], f64, f64)> = hole
            .places
            .iter()
            .map(|p| ([p[0], p[1]], p[2] - len, p[2]))
            .collect();
        for f in &part.faces {
            let mut kept = Some(f.clone());
            // a flat face at the band's heights (the mouth's, an inner shoulder's): Δ's disk
            // taken out of it
            if let Some(forge_cam_geom::slab::Prism::Flat { z, region, .. }) = &f.prism
                && bands.iter().any(|(_, zb, zt)| *z >= zb - 1e-9 && *z <= zt + 1e-9)
            {
                let mut rg = trim_region(region);
                for (c, zb, zt) in &bands {
                    if *z < zb - 1e-9 || *z > zt + 1e-9 {
                        continue;
                    }
                    let a0 = p2(c[0] + rm, c[1]);
                    let disk = Region {
                        loops: vec![Loop {
                            segs: vec![Seg::Arc {
                                a: a0,
                                b: a0,
                                c: p2(c[0], c[1]),
                                ccw: true,
                            }],
                        }],
                    };
                    rg = match rg.and_then(|r| {
                        forge_cam_geom::boolean::boolean(&r, &disk, forge_cam_geom::boolean::Op::Diff).ok()
                    }) {
                        Some(l) if !l.is_empty() => Some(forge_cam_geom::boolean::region_of(&l)),
                        _ => None,
                    };
                }
                kept = match rg {
                    Some(r) => Some(forge_cam::stock::flat(&r, *z, f.mu, f.id)),
                    // nothing of the face is left, or it could not be cut: the face as it is
                    // (a superset: never less strict)
                    None => Some(f.clone()),
                };
            }
            if let Surf::Cyl { o, x, y, ax, r } = f.surf
                && ax[0] == 0.0
                && ax[1] == 0.0
                && (r - rb).abs() <= 1e-6
            {
                for (c, zb, zt) in &bands {
                    if (o[0] - c[0]).abs() > 1e-6 || (o[1] - c[1]).abs() > 1e-6 {
                        continue;
                    }
                    // heights: o.z + ax.z q; the wall kept below the band only
                    let (q_at_zb, q_at_zt) = ((zb - o[2]) * ax[2], (zt - o[2]) * ax[2]);
                    let (qlo_band, qhi_band) = (q_at_zb.min(q_at_zt), q_at_zb.max(q_at_zt));
                    let below = if ax[2] > 0.0 {
                        forge_cam_geom::ival::Ival::new(f.q.lo, f.q.hi.min(qlo_band))
                    } else {
                        forge_cam_geom::ival::Ival::new(f.q.lo.max(qhi_band), f.q.hi)
                    };
                    kept = if below.lo < below.hi {
                        Some(Face::new(f.surf, f.p, below, None, f.mu, f.id))
                    } else {
                        None
                    };
                    // Δ's outer wall over the band, the same sector at the major radius
                    let band = forge_cam_geom::ival::Ival::new(qlo_band.max(f.q.lo.min(qlo_band)), qhi_band);
                    extra.push(Face::new(
                        Surf::Cyl { o, x, y, ax, r: rm },
                        f.p,
                        band,
                        None,
                        f.mu,
                        id,
                    ));
                    id += 1;
                }
            }
            if let Some(k) = kept {
                faces.push(k);
            }
        }
        for (c, zb, zt) in &bands {
            // Δ's floor: the ring at the band's bottom
            extra.push(forge_cam::stock::flat(
                &ring(*c, 0.5 * (rb + rm), 0.5 * (rm - rb)),
                *zb,
                1e-9,
                id,
            ));
            id += 1;
            out.removals.push(format!(
                "cosmetic_thread M{major}x{pitch} of `{hn}` at ({:.3}, {:.3}): the ring {rb:.3}..{rm:.3} mm from z {zb:.3} to {zt:.3}",
                c[0], c[1]
            ));
        }
        faces.extend(extra);
        out.target = Some(faces);
        // the path, each place in turn
        let mut mv = vec![];
        let first = bands[0].0;
        for (c, zb, zt) in &bands {
            mv.push(PMove::Rapid {
                to: [c[0], c[1], h.retract],
            });
            mv.push(PMove::Line {
                to: [c[0], c[1], *zb],
                f: feed.1,
                phase: Phase::Plunge,
            });
            mv.push(PMove::Line {
                to: [c[0] + rc, c[1], *zb],
                f: feed.0,
                phase: Phase::Cut,
            });
            let turns = ((zt - zb) / pitch - 1e-9).ceil().max(1.0) as usize;
            for k in 1..=4 * turns {
                let p = match k % 4 {
                    1 => [c[0], c[1] + rc],
                    2 => [c[0] - rc, c[1]],
                    3 => [c[0], c[1] - rc],
                    _ => [c[0] + rc, c[1]],
                };
                mv.push(PMove::Arc {
                    to: [p[0], p[1], zb + pitch * k as f64 / 4.0],
                    c: *c,
                    cw: false,
                    f: feed.0,
                });
            }
            let zend = zb + pitch * turns as f64;
            mv.push(PMove::Line {
                to: [c[0], c[1], zend],
                f: feed.0,
                phase: Phase::Cut,
            });
            mv.push(PMove::Line {
                to: [c[0], c[1], h.retract],
                f: feed.0,
                phase: Phase::Link,
            });
            // what it clears: the disk of the major radius, from the band's bottom up
            let a0 = p2(c[0] + rm, c[1]);
            let disk = Region {
                loops: vec![Loop {
                    segs: vec![Seg::Arc {
                        a: a0,
                        b: a0,
                        c: p2(c[0], c[1]),
                        ccw: true,
                    }],
                }],
            };
            voids.push((Some(disk), *zb));
        }
        mv.push(PMove::Rapid {
            to: [first[0], first[1], h.clear],
        });
        Ok(vec![OpPath {
            tool: ti,
            start: [first[0], first[1], h.clear],
            moves: mv,
            rpm: 0.0,
            coolant: forge_post::Coolant::Flood,
        }])
    }

    /// Edge break (a declared removal, the master's option (a)): the part's top edges at `at:`
    /// (default its top) broken by a 45° chamfer of `size:` s with a 90° chamfer mill. Δ is the
    /// band of the top within s of its edges, below the 45° surface z' = z − s + d, d the
    /// distance inside the top's outline R. The op is certified against P ∖ Δ: the top eroded
    /// to E = R ⊖ s, the walls cut to z − s, and the chamfer surfaces from each piece of E down
    /// to the same piece moved out by s (planes for lines, cones for arcs; a line's ends at a
    /// sharp corner meet the next line's on the corner's bisector). The path: the tool's axis on
    /// R ⊕ c with c = r_tip + e at depth z − s − e (e = 0.2 mm), where the tool's flank is that
    /// surface; every point of R ⊕ c is c from R, so the flank never goes below it. In this
    /// phase the top must be the part's top, with only vertical walls through the band, and the
    /// pieces of E must meet tangentially wherever an arc meets (a sharp corner between lines is
    /// taken; one with an arc is refused).
    #[allow(clippy::too_many_arguments)]
    fn edge_break(
        &self,
        su: usize,
        a: &BTreeMap<String, V>,
        t: &ToolUse,
        ti: usize,
        h: Heights,
        feed: (f64, f64),
        out: &mut OpOut,
    ) -> Result<Vec<OpPath>, String> {
        use forge_cam_geom::check::{Surf, TSeg, Trim};
        use forge_cam_geom::ival::Ival;
        let part = &self.parts[self.setups[su].part];
        if t.tool.kind != Kind::Chamfer || (t.tool.angle - 90.0).abs() > 1e-9 {
            out.fix = Some("a 90° chamfer mill (chamfer-10-90)".into());
            return Err(format!("edge_break: `{}` is not a 90° chamfer mill", t.tool.id));
        }
        let ztop = part.bbox.1[2];
        let z = get_num(a, "at", Some(ztop))?;
        let s = get_num(a, "size", None)?;
        let (r0, rt) = (0.5 * t.tool.tip_d, 0.5 * t.tool.dc);
        const E: f64 = 0.2;
        let c = r0 + E;
        if s.is_nan() || s <= 0.0 {
            return Err("edge_break: size must be over 0".into());
        }
        if c + s >= rt {
            out.fix = Some(format!("a chamfer mill over {:.3} mm across", 2.0 * (c + s)));
            return Err(format!(
                "edge_break: a {s} mm break needs the flank out to {:.3} mm; the tool's is {rt}",
                c + s
            ));
        }
        if (z - ztop).abs() > 1e-9 {
            return Err(format!(
                "edge_break: at z = {z} is not the part's top ({ztop}); this phase breaks the top's edges only"
            ));
        }
        let zc = z - s;
        let mu = 1e-9;
        // the top's outline R: the union of the flat faces at z
        let mut tops: Option<Region> = None;
        let mut faces: Vec<Face> = vec![];
        for f in &part.faces {
            if let Some(forge_cam_geom::slab::Prism::Flat { z: fz, region, .. }) = &f.prism
                && (fz - z).abs() <= 1e-9
            {
                let rg = trim_region(region)
                    .ok_or_else(|| format!("edge_break: the top face {} has an open trim", f.id))?;
                tops = Some(match tops {
                    None => rg,
                    Some(r) => forge_cam_geom::boolean::region_of(
                        &forge_cam_geom::boolean::boolean(&r, &rg, forge_cam_geom::boolean::Op::Union)
                            .map_err(|e| format!("edge_break: the top's faces: {e:?}"))?,
                    ),
                });
                continue;
            }
            if f.bbox[2].hi <= zc + f.mu {
                faces.push(f.clone());
                continue;
            }
            // through the band: a vertical wall, kept below z − s
            match f.surf {
                Surf::Cyl { o, ax, .. } if ax[0] == 0.0 && ax[1] == 0.0 && ax[2].abs() == 1.0 => {
                    // heights o.z + ax.z q
                    let qc = (zc - o[2]) * ax[2];
                    let below = if ax[2] > 0.0 {
                        Ival::new(f.q.lo, f.q.hi.min(qc))
                    } else {
                        Ival::new(f.q.lo.max(qc), f.q.hi)
                    };
                    if below.lo < below.hi {
                        faces.push(Face::new(f.surf, f.p, below, None, f.mu, f.id));
                    }
                }
                Surf::Plane { o, u, v } if u[2] == 0.0 && v[0] == 0.0 && v[1] == 0.0 && v[2].abs() == 1.0 => {
                    // vertical over a rectangle of (p, q) (q the height): kept below z − s
                    let mut ok = true;
                    let (mut p0, mut p1, mut q0, mut q1) =
                        (f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY, f64::NEG_INFINITY);
                    match &f.trim {
                        Some(tr) if tr.segs.len() == 4 => {
                            for sg in &tr.segs {
                                match *sg {
                                    TSeg::Line { a, b } if a[0] == b[0] || a[1] == b[1] => {
                                        for pt in [a, b] {
                                            p0 = p0.min(pt[0]);
                                            p1 = p1.max(pt[0]);
                                            q0 = q0.min(pt[1]);
                                            q1 = q1.max(pt[1]);
                                        }
                                    }
                                    _ => ok = false,
                                }
                            }
                        }
                        _ => ok = false,
                    }
                    if !ok {
                        return Err(format!(
                            "edge_break: the wall {} through the break's band is not a rectangle; this phase breaks a top over plain walls",
                            f.id
                        ));
                    }
                    // heights o.z + v.z q
                    let qc = (zc - o[2]) * v[2];
                    let (q0, q1) = if v[2] > 0.0 {
                        (q0, q1.min(qc))
                    } else {
                        (q0.max(qc), q1)
                    };
                    if q0 < q1 {
                        let c4 = [[p0, q0], [p1, q0], [p1, q1], [p0, q1]];
                        let segs = (0..4)
                            .map(|k| TSeg::Line {
                                a: c4[k],
                                b: c4[(k + 1) % 4],
                            })
                            .collect();
                        faces.push(Face::new(
                            f.surf,
                            Ival::new(p0 - f.mu, p1 + f.mu),
                            Ival::new(q0 - f.mu, q1 + f.mu),
                            Some(Trim { segs }),
                            f.mu,
                            f.id,
                        ));
                    }
                }
                _ => {
                    return Err(format!(
                        "edge_break: face {} reaches into the break's band (z {zc:.3} to {z:.3}) and is not a vertical wall; this phase breaks a top over vertical walls",
                        f.id
                    ));
                }
            }
        }
        let rg = tops.ok_or_else(|| format!("edge_break: the part has no flat face at z = {z}"))?;
        // E = R ⊖ s, as many loops as R (no feature narrower than 2s)
        let (el, _) = forge_cam_geom::offset::offset(&rg, s).map_err(|e| format!("edge_break: {e:?}"))?;
        let el: Vec<_> = el
            .into_iter()
            .filter(|l| !(l.segs.len() == 1 && matches!(l.segs[0], Seg::Line { a, b } if a == b)))
            .collect();
        if el.len() != rg.loops.len() {
            out.fix = Some(format!(
                "a break under half the top's narrowest feature (size < {s})"
            ));
            return Err(format!(
                "edge_break: the top eroded by {s} has {} loops where the top has {}: a feature is narrower than the break",
                el.len(),
                rg.loops.len()
            ));
        }
        let er = forge_cam_geom::boolean::region_of(&el);
        let mut id = 7000u32;
        faces.push(forge_cam::stock::flat(&er, z, mu, id));
        id += 1;
        // the chamfer surfaces, piece by piece of E
        let rt2 = std::f64::consts::FRAC_1_SQRT_2;
        let dir = |sg: &Seg, at_end: bool| -> [f64; 2] {
            match *sg {
                Seg::Line { a, b } => {
                    let (dx, dy) = (b.x - a.x, b.y - a.y);
                    let l = (dx * dx + dy * dy).sqrt();
                    [dx / l, dy / l]
                }
                Seg::Arc { a, b, c, ccw } => {
                    let p = if at_end { b } else { a };
                    let (rx, ry) = (p.x - c.x, p.y - c.y);
                    let l = (rx * rx + ry * ry).sqrt();
                    if ccw { [-ry / l, rx / l] } else { [ry / l, -rx / l] }
                }
            }
        };
        let cross = |u: [f64; 2], v: [f64; 2]| u[0] * v[1] - u[1] * v[0];
        let dotp = |u: [f64; 2], v: [f64; 2]| u[0] * v[0] + u[1] * v[1];
        for l in &er.loops {
            let n = l.segs.len();
            for (i, sg) in l.segs.iter().enumerate() {
                let prev = &l.segs[(i + n - 1) % n];
                let next = &l.segs[(i + 1) % n];
                // a junction: tangent (no miter), or a sharp corner between two lines
                let junction = |p: &Seg, q: &Seg, at: Seg| -> Result<Option<[f64; 2]>, String> {
                    let (u0, u1) = (dir(p, true), dir(q, false));
                    if cross(u0, u1).abs() <= 1e-9 && dotp(u0, u1) > 0.0 {
                        return Ok(None);
                    }
                    let pt = at.start();
                    match (p, q) {
                        (Seg::Line { .. }, Seg::Line { .. }) if cross(u0, u1) > 0.0 => {
                            // outward normals (inside on the left): the corner's bisector
                            let (n0, n1) = ([u0[1], -u0[0]], [u1[1], -u1[0]]);
                            let k = s / (1.0 + dotp(n0, n1));
                            Ok(Some([pt.x + k * (n0[0] + n1[0]), pt.y + k * (n0[1] + n1[1])]))
                        }
                        _ => Err(format!(
                            "edge_break: the top's corner at ({:.3}, {:.3}) is sharp where an arc meets (or turns inward); this phase breaks sharp corners between lines only",
                            pt.x, pt.y
                        )),
                    }
                };
                match *sg {
                    Seg::Line { a: pa, b: pb } => {
                        let u2 = dir(sg, false);
                        let nn = [u2[1], -u2[0]];
                        let len = ((pb.x - pa.x).powi(2) + (pb.y - pa.y).powi(2)).sqrt();
                        let qb = s * std::f64::consts::SQRT_2;
                        // bottom ends: moved out by s, or the corner
                        let ca = junction(prev, sg, *sg)?;
                        let cb = junction(sg, next, *next)?;
                        let along = |p: [f64; 2]| (p[0] - pa.x) * u2[0] + (p[1] - pa.y) * u2[1];
                        let pa_b = ca.map(along).unwrap_or(0.0);
                        let pb_b = cb.map(along).unwrap_or(len);
                        let poly = [[0.0, 0.0], [len, 0.0], [pb_b, qb], [pa_b, qb]];
                        let segs: Vec<TSeg> = (0..4)
                            .map(|k| TSeg::Line {
                                a: poly[k],
                                b: poly[(k + 1) % 4],
                            })
                            .collect();
                        // o + p u + q v: u along the piece, v down the 45° slope outward; the
                        // quadrilateral runs anticlockwise in (p, q) when u × v points out of the
                        // part (up and out)
                        faces.push(Face::new(
                            Surf::Plane {
                                o: [pa.x, pa.y, z],
                                u: [u2[0], u2[1], 0.0],
                                v: [nn[0] * rt2, nn[1] * rt2, -rt2],
                            },
                            Ival::new(pa_b.min(0.0) - mu, pb_b.max(len) + mu),
                            Ival::new(-mu, qb + mu),
                            Some(Trim { segs }),
                            mu,
                            id,
                        ));
                        id += 1;
                    }
                    Seg::Arc {
                        a: pa,
                        b: pb,
                        c: cc,
                        ccw,
                    } => {
                        junction(prev, sg, *sg)?;
                        junction(sg, next, *next)?;
                        let r1 = ((pa.x - cc.x).powi(2) + (pa.y - cc.y).powi(2)).sqrt();
                        let full = pa == pb;
                        // pieces of at most a quarter turn
                        let t0 = forge_math::dmath::atan2(pa.y - cc.y, pa.x - cc.x);
                        let mut t1 = if full {
                            t0 + if ccw {
                                std::f64::consts::TAU
                            } else {
                                -std::f64::consts::TAU
                            }
                        } else {
                            forge_math::dmath::atan2(pb.y - cc.y, pb.x - cc.x)
                        };
                        if !full {
                            if ccw {
                                while t1 <= t0 {
                                    t1 += std::f64::consts::TAU;
                                }
                            } else {
                                while t1 >= t0 {
                                    t1 -= std::f64::consts::TAU;
                                }
                            }
                        }
                        let m = (((t1 - t0).abs() / (0.5 * std::f64::consts::PI)).ceil() as usize).max(1);
                        for k in 0..m {
                            let (u0, u1) = (
                                t0 + (t1 - t0) * k as f64 / m as f64,
                                t0 + (t1 - t0) * (k + 1) as f64 / m as f64,
                            );
                            let um = 0.5 * (u0 + u1);
                            let hh = 0.5 * (u1 - u0).abs();
                            let (sm, cm) = forge_math::dmath::sin_cos(um);
                            // tan of a quarter of the sweep, widened a little (a superset)
                            let th = forge_math::dmath::sin(0.5 * hh) / forge_math::dmath::cos(0.5 * hh)
                                * (1.0 + 1e-12);
                            // ccw (the centre inside): the radius grows going down; cw shrinks
                            let kk = if ccw { 1.0 } else { -1.0 };
                            faces.push(Face::new(
                                Surf::Cone {
                                    o: [cc.x, cc.y, z],
                                    x: [cm, sm, 0.0],
                                    y: [-sm, cm, 0.0],
                                    ax: [0.0, 0.0, -1.0],
                                    r0: r1,
                                    k: kk,
                                },
                                Ival::new(-th, th),
                                Ival::new(0.0, s),
                                None,
                                mu,
                                id,
                            ));
                            id += 1;
                        }
                    }
                }
            }
        }
        out.target = Some(faces);
        out.removals.push(format!(
            "edge_break {s} x 45° of the top at z {z:.3}: the band within {s} of its edges, from z {zc:.3} up, below the 45° surface"
        ));
        // the path: the axis on R ⊕ c, the tip at z − s − e
        let (pl, _) = forge_cam_geom::offset::offset(&rg, -c).map_err(|e| format!("edge_break: {e:?}"))?;
        let np = pl
            .iter()
            .filter(|l| !(l.segs.len() == 1 && matches!(l.segs[0], Seg::Line { a, b } if a == b)))
            .count();
        if np != rg.loops.len() {
            out.fix = Some(format!(
                "a chamfer mill with a smaller tip (its axis runs {c:.3} mm off the edge)"
            ));
            return Err(format!(
                "edge_break: the tool's axis path R ⊕ {c:.3} has {np} loops where the top has {}: a hole is too small for the tool",
                rg.loops.len()
            ));
        }
        let zt = zc - E;
        ops::contour(
            &rg,
            ops::Cut {
                tool: t,
                tidx: ti,
                h,
                feed,
            },
            ops::Span {
                top: zt,
                bottom: zt,
                step: 1.0,
            },
            c - rt,
            true,
        )
        .map(|p| vec![p])
        .map_err(|e| format!("{e:?}"))
    }

    fn feeds_for(&self, su: usize, t: &ToolUse, ae: f64, ap: f64) -> Option<feeds::Feeds> {
        let g = self.lib.group(&self.parts[self.setups[su].part].group)?;
        let m = self.lib.machine(&self.setups[su].machine).ok()?;
        Some(feeds::feeds(
            g,
            &feeds::Cut {
                d: t.tool.dc,
                flutes: t.tool.nof.max(1),
                ball: t.tool.kind == Kind::Ball,
                carbide: t.tool.material == "carbide",
                ap,
                ae,
            },
            m.rpm,
        ))
    }

    fn op(&mut self, name: &str, kind: &str, a: &BTreeMap<String, V>) -> Result<V, String> {
        let su = match a.get("#0") {
            Some(V::Setup(i)) => *i,
            _ => return Err(format!("{kind}(SETUP, ..)")),
        };
        let s = &self.setups[su];
        let part = &self.parts[s.part];
        let mut out = OpOut {
            name: name.into(),
            kind: match kind {
                "facing" => "facing",
                "contour" => "contour",
                "pocket" => "pocket",
                "clear" => "clear",
                "engrave" => "engrave",
                "thread_mill" => "thread_mill",
                "edge_break" => "edge_break",
                _ => "drill",
            },
            setup: su,
            tool: None,
            paths: vec![],
            feeds: None,
            checks: vec![],
            pairs: 0,
            cells: 0,
            status: "built",
            refusal: None,
            fix: None,
            moves: 0,
            length_cut: 0.0,
            length_rapid: 0.0,
            time_s: 0.0,
            least: f64::INFINITY,
            plan: vec![],
            leave: get_num(a, "leave", Some(0.0))?,
            target: None,
            removals: vec![],
            stock_before: Some(s.ip.clone()),
            voids: vec![],
        };
        let mut voids: Vec<(Option<Region>, f64)> = vec![];
        let paths: Result<Vec<OpPath>, String> = match kind {
            "drill" => self.plan_holes(su, a, &mut out, &mut voids),
            _ => {
                let ti = match a.get("tool") {
                    Some(V::Tool(i)) => *i,
                    _ => return Err(format!("{kind}: missing `tool:`")),
                };
                out.tool = Some(ti);
                let t = &self.tools[ti];
                let d = t.tool.dc;
                let step = get_num(a, "step", Some(0.5 * d))?;
                // a helical entry at this angle (degrees) where it fits; else plunges
                let ramp = match a.get("ramp") {
                    None => None,
                    Some(V::Num(x)) if *x > 0.0 && *x <= 30.0 => Some(*x),
                    Some(_) => return Err("ramp: an angle in degrees, over 0 and at most 30".into()),
                };
                let fd = match kind {
                    "facing" => self.feeds_for(su, t, 0.7 * d, step.min(t.tool.lcf)),
                    "pocket" => self.feeds_for(su, t, get_num(a, "stepover", Some(0.4 * d))?, step),
                    _ => self.feeds_for(su, t, 0.1 * d, step),
                };
                let (vf, vp) = match fd {
                    Some(fe) => (fe.vf, 0.3 * fe.vf),
                    None => (300.0, 100.0),
                };
                out.feeds = fd;
                let h = s.heights;
                match kind {
                    "engrave" => self.engrave(su, a, t, ti, h, (vf, vp), step, &mut voids),
                    "thread_mill" => self.thread_mill(su, a, t, ti, h, (vf, vp), &mut out, &mut voids),
                    "edge_break" => self.edge_break(su, a, t, ti, h, (vf, vp), &mut out),
                    "facing" => {
                        let z = get_num(a, "to", Some(part.bbox.1[2]))?;
                        let p = ops::facing(
                            &s.stock,
                            ops::Cut {
                                tool: t,
                                tidx: ti,
                                h,
                                feed: (vf, vp),
                            },
                            ops::Span {
                                top: s.stock.max[2],
                                bottom: z,
                                step,
                            },
                        );
                        voids.push((None, z));
                        Ok(vec![p])
                    }
                    "clear" => {
                        // everything between the stock's outline and the part's section, from a
                        // height down to another: a pocket with the part as islands
                        let top = get_num(a, "from", Some(s.stock.max[2]))?;
                        let bottom = get_num(a, "to", None)?;
                        let zmid = 0.5 * (top + bottom);
                        let r = 0.5 * d;
                        let sec = forge_cam::section::section(&part.faces, zmid, 1e-5)
                            .map_err(|e| format!("the part's section at z = {zmid}: {e:?}"))?;
                        let islands = outer_or_inner(&sec, true)?;
                        // the tool may pass wholly outside the stock: a strip between the part
                        // and the stock's face narrower than the tool is still cleared
                        let g = 2.0 * r;
                        let (x0, y0, x1, y1) = (
                            s.stock.min[0] - g,
                            s.stock.min[1] - g,
                            s.stock.max[0] + g,
                            s.stock.max[1] + g,
                        );
                        let (pa, pb, pc, pd) = (p2(x0, y0), p2(x1, y0), p2(x1, y1), p2(x0, y1));
                        let mut loops = vec![Loop {
                            segs: vec![
                                Seg::Line { a: pa, b: pb },
                                Seg::Line { a: pb, b: pc },
                                Seg::Line { a: pc, b: pd },
                                Seg::Line { a: pd, b: pa },
                            ],
                        }];
                        for l in &islands.loops {
                            loops.push(l.reversed());
                        }
                        let rg = Region { loops };
                        let stepover = get_num(a, "stepover", Some(0.4 * d))?;
                        let p = ops::pocket(
                            &rg,
                            ops::Cut {
                                tool: t,
                                tidx: ti,
                                h,
                                feed: (vf, vp),
                            },
                            ops::Span { top, bottom, step },
                            stepover,
                            get_num(a, "leave", Some(0.0))?,
                            ramp,
                        )
                        .map_err(|e| format!("{e:?}"))?;
                        if stepover <= r
                            && let Some(o) = opening(&rg, r)
                        {
                            voids.push((Some(o), bottom));
                        }
                        Ok(vec![p])
                    }
                    "contour" => {
                        let top = get_num(a, "from", Some(part.bbox.1[2]))?;
                        let bottom = get_num(a, "to", Some(part.bbox.0[2]))?;
                        let side = get_str(a, "side", Some("outside"))?;
                        let leave = get_num(a, "leave", Some(0.0))?;
                        let zmid = 0.5 * (top + bottom);
                        let sec = forge_cam::section::section(&part.faces, zmid, 1e-5)
                            .map_err(|e| format!("the part's section at z = {zmid}: {e:?}"))?;
                        let rg = outer_or_inner(&sec, side == "outside")?;
                        if side == "outside" {
                            // the band the loop sweeps is cleared
                            let r = 0.5 * d;
                            if let Ok((c, _)) = forge_cam_geom::offset::offset(&rg, -(r + leave)) {
                                let cr = forge_cam_geom::boolean::region_of(&c);
                                if std::env::var("FORGE_CAM_DEBUG").is_ok() {
                                    let o = forge_cam_geom::offset::offset(&cr, -r).map(|x| x.0.len());
                                    let i = forge_cam_geom::offset::offset(&cr, r).map(|x| x.0.len());
                                    eprintln!("band: c {} loops; outer {o:?} inner {i:?}", c.len());
                                }
                                if let (Ok((outer, _)), Ok((inner, _))) = (
                                    forge_cam_geom::offset::offset(&cr, -r),
                                    forge_cam_geom::offset::offset(&cr, r),
                                ) && let Ok(band) = forge_cam_geom::boolean::boolean(
                                    &forge_cam_geom::boolean::region_of(&outer),
                                    &forge_cam_geom::boolean::region_of(&inner),
                                    forge_cam_geom::boolean::Op::Diff,
                                ) {
                                    voids.push((Some(forge_cam_geom::boolean::region_of(&band)), bottom));
                                }
                            }
                            ops::contour(
                                &rg,
                                ops::Cut {
                                    tool: t,
                                    tidx: ti,
                                    h,
                                    feed: (vf, vp),
                                },
                                ops::Span { top, bottom, step },
                                leave,
                                true,
                            )
                            .map(|p| vec![p])
                            .map_err(|e| format!("{e:?}"))
                        } else {
                            ops::contour(
                                &rg,
                                ops::Cut {
                                    tool: t,
                                    tidx: ti,
                                    h,
                                    feed: (vf, vp),
                                },
                                ops::Span { top, bottom, step },
                                leave,
                                false,
                            )
                            .map(|p| vec![p])
                            .map_err(|e| format!("{e:?}"))
                        }
                    }
                    _ => {
                        // with no hole named: every hole and slot of the part's section between
                        // two heights (its inner loops, each a pocket; islands inside them kept)
                        if a.get("#1").is_none() {
                            let top = get_num(a, "from", None)?;
                            let bottom = get_num(a, "to", None)?;
                            let zmid = 0.5 * (top + bottom);
                            let sec = forge_cam::section::section(&part.faces, zmid, 1e-5)
                                .map_err(|e| format!("the part's section at z = {zmid}: {e:?}"))?;
                            let holes = outer_or_inner(&sec, false)?;
                            let stepover = get_num(a, "stepover", Some(0.4 * d))?;
                            let leave = get_num(a, "leave", Some(0.0))?;
                            let mut paths = vec![];
                            for l in &holes.loops {
                                let rg = Region {
                                    loops: vec![l.clone()],
                                };
                                paths.push(
                                    ops::pocket(
                                        &rg,
                                        ops::Cut {
                                            tool: t,
                                            tidx: ti,
                                            h,
                                            feed: (vf, vp),
                                        },
                                        ops::Span { top, bottom, step },
                                        stepover,
                                        leave,
                                        ramp,
                                    )
                                    .map_err(|e| format!("{e:?}"))?,
                                );
                                let o = if stepover <= 0.5 * d {
                                    opening(&rg, 0.5 * d)
                                } else {
                                    None
                                };
                                if let Some(o) = o {
                                    voids.push((Some(o), bottom));
                                }
                            }
                            Ok(paths)
                        } else {
                            let (pi, hn) = match a.get("#1") {
                            Some(V::Holes(p, n)) if *p == self.setups[su].part => (*p, n.clone()),
                            Some(V::Holes(..)) => return Err("the hole is drilled from the side it opens on: in a setup that holds the part that way up".into()),
                            _ => return Err("pocket(SETUP, PART.HOLE, tool: ..) or pocket(SETUP, from:, to:, tool: ..)".into()),
                        };
                            let hole = self.parts[pi]
                                .holes
                                .iter()
                                .find(|x| x.name == hn)
                                .unwrap()
                                .clone();
                            if hole.dir != [0.0, 0.0, -1.0] {
                                return Err(format!(
                                    "the hole `{hn}` is not drilled along −z: it needs its own setup"
                                ));
                            }
                            let stepover = get_num(a, "stepover", Some(0.4 * d))?;
                            let mut paths = vec![];
                            for p in &hole.places {
                                let r = 0.5 * hole.d;
                                let st = p2(p[0] + r, p[1]);
                                let rg = Region {
                                    loops: vec![Loop {
                                        segs: vec![Seg::Arc {
                                            a: st,
                                            b: st,
                                            c: p2(p[0], p[1]),
                                            ccw: true,
                                        }],
                                    }],
                                };
                                let bottom = match hole.depth {
                                    Some(dd) => p[2] - dd,
                                    None => self.parts[pi].bbox.0[2] - 0.5,
                                };
                                paths.push(
                                    ops::pocket(
                                        &rg,
                                        ops::Cut {
                                            tool: t,
                                            tidx: ti,
                                            h,
                                            feed: (vf, vp),
                                        },
                                        ops::Span {
                                            top: p[2],
                                            bottom,
                                            step,
                                        },
                                        stepover,
                                        get_num(a, "leave", Some(0.0))?,
                                        ramp,
                                    )
                                    .map_err(|e| format!("{e:?}"))?,
                                );
                                if stepover <= 0.5 * d
                                    && let Some(o) = opening(&rg, 0.5 * d)
                                {
                                    voids.push((Some(o), bottom));
                                }
                            }
                            Ok(paths)
                        }
                    }
                }
            }
        };
        let paths = match paths {
            Ok(p) => p,
            Err(e) => {
                out.status = "refused";
                out.refusal = Some(e);
                self.ops.push(out);
                return Ok(V::Op(self.ops.len() - 1));
            }
        };
        out.paths = paths;
        out.voids = voids.clone();
        for (v, floor) in voids {
            match v {
                None => self.setups[su].ip.face_to(floor),
                Some(r) => self.setups[su].ip.cut(r, floor),
            }
        }
        self.ops.push(out);
        Ok(V::Op(self.ops.len() - 1))
    }

    /// The hole plan from the feature: spot, then drill (peck past 3 D), then tap if threaded.
    fn plan_holes(
        &mut self,
        su: usize,
        a: &BTreeMap<String, V>,
        out: &mut OpOut,
        voids: &mut Vec<(Option<Region>, f64)>,
    ) -> Result<Vec<OpPath>, String> {
        let (pi, hn) = match a.get("#1") {
            Some(V::Holes(p, n)) if *p == self.setups[su].part => (*p, n.clone()),
            Some(V::Holes(..)) => {
                return Err("the hole is drilled from the side it opens on: in a setup that holds the part that way up".into());
            }
            _ => return Err("drill(SETUP, PART.HOLE, plan: \"auto\")".into()),
        };
        let hole = self.parts[pi]
            .holes
            .iter()
            .find(|x| x.name == hn)
            .unwrap()
            .clone();
        if hole.dir != [0.0, 0.0, -1.0] {
            return Err(format!(
                "the hole `{hn}` is not drilled along −z: it needs its own setup"
            ));
        }
        let through = hole.depth.is_none();
        let depth = match hole.depth {
            Some(d) => d,
            None => hole.places[0][2] - self.parts[pi].bbox.0[2] + 1.0,
        };
        // the drill: the tap drill for a thread, else the hole's own diameter
        let drill_d = match hole.thread {
            Some((major, pitch)) => major - pitch,
            None => hole.d,
        };
        let find = |lib: &Library, k: Kind, d: f64| {
            lib.tools
                .iter()
                .find(|t| t.kind == k && (t.dc - d).abs() < 1e-9)
                .cloned()
        };
        let drill = find(&self.lib, Kind::Drill, (drill_d * 100.0).round() / 100.0)
            .or_else(|| find(&self.lib, Kind::Drill, drill_d));
        let Some(drill) = drill else {
            out.fix = Some(format!(
                "add a drill of {drill_d} mm to the project's cam/*.tools.toml"
            ));
            return Err(format!(
                "NeedTool: no {drill_d} mm drill in the library for `{hn}`"
            ));
        };
        let spot = self
            .lib
            .tools
            .iter()
            .find(|t| t.kind == Kind::Spot && t.dc >= drill_d)
            .cloned();
        // plan "auto": spot, drill, tap a thread; "drill": spot and drill, the thread left to a
        // thread mill
        let plan_kind = get_str(a, "plan", Some("auto"))?;
        if plan_kind != "auto" && plan_kind != "drill" {
            return Err("plan: \"auto\" (spot, drill, tap a thread) or \"drill\" (no tap)".into());
        }
        let tap = match hole.thread {
            Some(_) if plan_kind == "drill" => None,
            Some((major, pitch)) => {
                let t = self
                    .lib
                    .tools
                    .iter()
                    .find(|t| {
                        t.kind == Kind::Tap && (t.dc - major).abs() < 1e-9 && (t.tp - pitch).abs() < 1e-9
                    })
                    .cloned();
                if t.is_none() {
                    out.fix = Some(format!("add a tap M{major}x{pitch}"));
                    return Err(format!("NeedTool: no tap {major} x {pitch} for `{hn}`"));
                }
                t
            }
            None => None,
        };
        let s = &self.setups[su];
        let h = s.heights;
        let mut paths = vec![];
        let mut plan = vec![];
        let base_t = 10 + self.tools.len() as u32;
        let add = |this: &mut Cam, t: forge_tooling::Tool, n: u32| -> Result<usize, String> {
            let holder = this.lib.holder("bt40-er32").ok().cloned();
            let stick = t.lcf + 10.0;
            let shape = t.shape(stick, holder.as_ref()).map_err(|e| format!("{e:?}"))?;
            this.tools.push(ToolUse {
                t: n,
                tool: t,
                holder: holder.map(|h| h.id),
                stickout: stick,
                shape,
            });
            Ok(this.tools.len() - 1)
        };
        if let Some(sp) = spot {
            let ti = add(self, sp, base_t)?;
            let t = &self.tools[ti];
            let f = self
                .feeds_for(su, t, t.tool.dc, 1.0)
                .map(|x| 0.3 * x.vf)
                .unwrap_or(100.0);
            // a 90° spot to just under the drill's radius: the chamfer ring the drill centres in
            let (sn, cs) = forge_math::dmath::sin_cos((0.5 * t.tool.angle).to_radians());
            let dspot = 0.45 * drill_d / (sn / cs).max(1e-9);
            let mut p = ops::holes(
                &hole.places,
                CycleKind::Drill,
                ops::Cut {
                    tool: t,
                    tidx: ti,
                    h,
                    feed: (f, f),
                },
                0.0,
                0.0,
            );
            for m in &mut p.moves {
                if let PMove::Cycle { z, r, .. } = m {
                    *z = *r - 2.0 - dspot;
                }
            }
            plan.push(format!("spot {} to {:.2} mm", t.tool.id, dspot));
            paths.push(p);
        }
        let ti = add(self, drill, base_t + 1)?;
        let t = &self.tools[ti];
        let peck = depth > 3.0 * t.tool.dc;
        let f = self
            .feeds_for(su, t, t.tool.dc, t.tool.dc)
            .map(|x| 0.1 * x.vf.min(2000.0))
            .unwrap_or(100.0);
        let kind = if peck { CycleKind::Peck } else { CycleKind::Drill };
        paths.push(ops::holes(
            &hole.places,
            kind,
            ops::Cut {
                tool: t,
                tidx: ti,
                h,
                feed: (f, f),
            },
            depth,
            if peck { t.tool.dc } else { 0.0 },
        ));
        // each hole's circle is cleared down to where the full diameter reaches
        for p in &hole.places {
            let r = 0.5 * t.tool.dc;
            let st = p2(p[0] + r, p[1]);
            voids.push((
                Some(Region {
                    loops: vec![Loop {
                        segs: vec![Seg::Arc {
                            a: st,
                            b: st,
                            c: p2(p[0], p[1]),
                            ccw: true,
                        }],
                    }],
                }),
                p[2] - depth,
            ));
        }
        plan.push(format!(
            "{} {} {}{}",
            if peck { "peck" } else { "drill" },
            t.tool.id,
            if through { "through" } else { "to depth" },
            if peck {
                format!(", q {}", t.tool.dc)
            } else {
                String::new()
            }
        ));
        out.tool = Some(ti);
        if let Some(tp) = tap {
            let ti = add(self, tp, base_t + 2)?;
            let t = &self.tools[ti];
            let rpm = 300.0;
            let f = t.tool.tp * rpm;
            let mut p = ops::holes(
                &hole.places,
                CycleKind::Tap,
                ops::Cut {
                    tool: t,
                    tidx: ti,
                    h,
                    feed: (f, f),
                },
                depth.min(hole.depth.unwrap_or(depth)),
                0.0,
            );
            p.rpm = rpm;
            paths.push(p);
            plan.push(format!("tap {} (pitch {}) at {rpm} rpm", t.tool.id, t.tool.tp));
        }
        out.plan = plan;
        // the plan's paths are posted one after the other, each with its tool
        Ok(paths)
    }
}

/// The opening of a region by a disc of radius r, (R ⊖ B_r) ⊕ B_r: what a pocket with
/// stepover ≤ r provably clears.
/// A flat face's trim (its pieces, loop after loop) as a region: each loop closes where a piece
/// ends at its loop's start.
fn trim_region(segs: &[forge_cam_geom::check::TSeg]) -> Option<Region> {
    use forge_cam_geom::check::TSeg;
    let mut loops = vec![];
    let mut cur: Vec<Seg> = vec![];
    for t in segs {
        let s = match *t {
            TSeg::Line { a, b } => Seg::Line {
                a: p2(a[0], a[1]),
                b: p2(b[0], b[1]),
            },
            TSeg::Arc { a, b, c, ccw, .. } => Seg::Arc {
                a: p2(a[0], a[1]),
                b: p2(b[0], b[1]),
                c: p2(c[0], c[1]),
                ccw,
            },
        };
        cur.push(s);
        if cur[0].start() == s.end() {
            loops.push(Loop {
                segs: std::mem::take(&mut cur),
            });
        }
    }
    if !cur.is_empty() || loops.is_empty() {
        return None;
    }
    Some(Region { loops })
}

/// The points within w of the segment p0 p1: a stadium (two lines, two half circles in quarter
/// arcs), anticlockwise.
fn stadium(p0: [f64; 2], p1: [f64; 2], w: f64) -> Region {
    let (dx, dy) = (p1[0] - p0[0], p1[1] - p0[1]);
    let l = (dx * dx + dy * dy).sqrt();
    let (ux, uy) = (dx / l * w, dy / l * w);
    let (nx, ny) = (-uy, ux);
    let q = |p: [f64; 2], a: f64, b: f64| p2(p[0] + a, p[1] + b);
    let (a0, a1, a2) = (q(p1, -nx, -ny), q(p1, ux, uy), q(p1, nx, ny));
    let (b0, b1, b2) = (q(p0, nx, ny), q(p0, -ux, -uy), q(p0, -nx, -ny));
    let (c1, c0) = (p2(p1[0], p1[1]), p2(p0[0], p0[1]));
    Region {
        loops: vec![Loop {
            segs: vec![
                Seg::Line { a: b2, b: a0 },
                Seg::Arc {
                    a: a0,
                    b: a1,
                    c: c1,
                    ccw: true,
                },
                Seg::Arc {
                    a: a1,
                    b: a2,
                    c: c1,
                    ccw: true,
                },
                Seg::Line { a: a2, b: b0 },
                Seg::Arc {
                    a: b0,
                    b: b1,
                    c: c0,
                    ccw: true,
                },
                Seg::Arc {
                    a: b1,
                    b: b2,
                    c: c0,
                    ccw: true,
                },
            ],
        }],
    }
}

/// The points within w of the circle about c of radius r (r > w): a ring.
fn ring(c: [f64; 2], r: f64, w: f64) -> Region {
    let circ = |rr: f64, ccw: bool| {
        let a = p2(c[0] + rr, c[1]);
        Loop {
            segs: vec![Seg::Arc {
                a,
                b: a,
                c: p2(c[0], c[1]),
                ccw,
            }],
        }
    };
    Region {
        loops: vec![circ(r + w, true), circ(r - w, false)],
    }
}

fn opening(rg: &Region, r: f64) -> Option<Region> {
    let dbg = |e: &dyn std::fmt::Debug| {
        if std::env::var("FORGE_CAM_DEBUG").is_ok() {
            eprintln!("opening: {e:?}");
        }
    };
    let (c, _) = forge_cam_geom::offset::offset(rg, r).map_err(|e| dbg(&e)).ok()?;
    // the tool centres' region; an isolated point of it (where the tool just fits) clears the
    // disk of radius r about it, so it is grown as that disk, apart from the rest
    let (pts, rest): (Vec<_>, Vec<_>) = c
        .into_iter()
        .partition(|l| l.segs.len() == 1 && matches!(l.segs[0], Seg::Line { a, b } if a == b));
    let mut out: Option<Region> = None;
    if !rest.is_empty() {
        let cr = forge_cam_geom::boolean::region_of(&rest);
        let (o, _) = forge_cam_geom::offset::offset(&cr, -r)
            .map_err(|e| dbg(&e))
            .ok()?;
        out = Some(forge_cam_geom::boolean::region_of(&o));
    }
    for l in pts {
        let c = l.segs[0].start();
        let a = p2(c.x + r, c.y);
        let disk = Region {
            loops: vec![Loop {
                segs: vec![Seg::Arc {
                    a,
                    b: a,
                    c,
                    ccw: true,
                }],
            }],
        };
        out = Some(match out {
            None => disk,
            Some(o) => forge_cam_geom::boolean::region_of(
                &forge_cam_geom::boolean::boolean(&o, &disk, forge_cam_geom::boolean::Op::Union)
                    .map_err(|e| dbg(&e))
                    .ok()?,
            ),
        });
    }
    out
}

/// The section's outer loops alone (outside work), or its islands as pocket boundaries.
fn outer_or_inner(sec: &Region, outer: bool) -> Result<Region, String> {
    let mut cx = forge_cam_geom::real::Ctx::new();
    let ex = forge_cam_geom::region::validate(&mut cx, sec).map_err(|e| format!("{e:?}"))?;
    let mut loops = vec![];
    for l in 0..sec.loops.len() {
        let ccw = forge_cam_geom::region::loop_ccw(&mut cx, &ex, l);
        if ccw == outer {
            loops.push(if outer {
                sec.loops[l].clone()
            } else {
                sec.loops[l].reversed()
            });
        }
    }
    if loops.is_empty() {
        return Err(if outer {
            "no outer boundary at that height".into()
        } else {
            "no inner boundary (hole) at that height".into()
        });
    }
    Ok(Region { loops })
}

impl Cam {
    /// G5 for one op's posted path: with the setup placed (`at:`), every position (an arc by its
    /// circle's box, a superset) as each joint sees it (the spindle's Z: the tip's plus the tool's
    /// length, stick-out and holder gauge) within the joint's travel, proven in outward-rounded
    /// arithmetic; unplaced, each axis's span within its travel's width. The first violation.
    fn axis_limits(&self, oi: usize, posted: &OpPath) -> Option<CheckOut> {
        let s = &self.setups[self.ops[oi].setup];
        let m = self.lib.machine(&s.machine).ok()?;
        let t = &self.tools[posted.tool];
        let gauge = t
            .holder
            .as_ref()
            .and_then(|h| self.lib.holder(h).ok())
            .map_or(0.0, |h| h.gauge_length);
        let len = t.stickout + gauge;
        // the path's box in program coordinates (part frame less the WCS origin)
        let mut lo = [f64::INFINITY; 3];
        let mut hi = [f64::NEG_INFINITY; 3];
        let mut add = |p: [f64; 3]| {
            for k in 0..3 {
                lo[k] = lo[k].min(p[k] - s.origin[k]);
                hi[k] = hi[k].max(p[k] - s.origin[k]);
            }
        };
        add(posted.start);
        let mut at = posted.start;
        for mv in &posted.moves {
            match *mv {
                PMove::Rapid { to } | PMove::Line { to, .. } => {
                    add(to);
                    at = to;
                }
                PMove::Arc { to, c, .. } => {
                    let r = ((at[0] - c[0]).powi(2) + (at[1] - c[1]).powi(2)).sqrt() * (1.0 + 1e-12);
                    add([c[0] - r, c[1] - r, to[2].min(at[2])]);
                    add([c[0] + r, c[1] + r, to[2].max(at[2])]);
                    add(to);
                    at = to;
                }
                PMove::Cycle { .. } => {}
            }
        }
        use forge_cam_geom::ival::Ival;
        for k in 0..3 {
            let Some(j) = m.joints.iter().find(|j| {
                let a = j.axis;
                (0..3).all(|q| if q == k { a[q].abs() == 1.0 } else { a[q] == 0.0 })
            }) else {
                continue;
            };
            let sign = j.axis[k];
            let extra = if k == 2 { len } else { 0.0 };
            let fail = |value: f64, limit: f64, what: String| CheckOut {
                rule: "axis_limit",
                verdict: "fail",
                value,
                limit,
                at: [f64::NAN; 3],
                tool_part: "machine",
                other: what,
                phase: "path",
                move_index: 0,
            };
            match s.at {
                Some(o) => {
                    // the joint's coordinate at the box's ends: sign (p + at) + extra
                    for (p, end) in [(lo[k], "least"), (hi[k], "greatest")] {
                        let v = Ival::pt(p).add(Ival::pt(o[k])).add(Ival::pt(extra)).scale(sign);
                        if !(v.lo >= j.range[0] && v.hi <= j.range[1]) {
                            let lim = if v.lo < j.range[0] { j.range[0] } else { j.range[1] };
                            return Some(fail(
                                if v.lo < j.range[0] { v.lo } else { v.hi },
                                lim,
                                format!(
                                    "joint {}: the path's {end} reach is outside its travel {:?}",
                                    j.name, j.range
                                ),
                            ));
                        }
                    }
                }
                None => {
                    let span = Ival::pt(hi[k]).sub(Ival::pt(lo[k])).hi;
                    let width = j.range[1] - j.range[0];
                    if span > width {
                        return Some(fail(
                            span,
                            width,
                            format!(
                                "joint {}: the path spans {span:.3} mm, the travel {width:.3} mm",
                                j.name
                            ),
                        ));
                    }
                }
            }
        }
        None
    }

    /// Posts the setups' ops, reads the text back, and certifies the path the text means.
    fn post(&mut self, name: &str, setups: &[usize], post: &str, number: u32) -> ProgramOut {
        let mut po = ProgramOut {
            setups: setups.to_vec(),
            name: name.into(),
            post: post.into(),
            text: None,
            lines: 0,
            error: None,
            round_trip: false,
        };
        let d = match forge_post::Dialect::get(post) {
            Ok(d) => d,
            Err(e) => {
                po.error = Some(format!("{e:?}"));
                return po;
            }
        };
        let mut recs = vec![];
        let mut current_tool: Option<u32> = None;
        // each op's records, and which records are whose
        let mut spans: Vec<(usize, usize, usize, usize)> = vec![];
        let op_ids: Vec<usize> = (0..self.ops.len())
            .filter(|&i| setups.contains(&self.ops[i].setup) && !self.ops[i].paths.is_empty())
            .collect();
        for &oi in &op_ids {
            let op = &self.ops[oi];
            let s = &self.setups[op.setup];
            let o = s.origin;
            let w = |p: [f64; 3]| [p[0] - o[0], p[1] - o[1], p[2] - o[2]];
            recs.push(Rec::Comment(format!("{} ({})", op.name, op.kind)));
            if current_tool.is_none() {
                recs.push(Rec::Wcs(s.wcs));
            }
            // a hole plan's several tools: one path each
            for (pi, p) in op.paths.iter().enumerate() {
                let start = recs.len();
                let t = &self.tools[p.tool];
                if current_tool != Some(t.t) {
                    if current_tool.is_some() {
                        recs.push(Rec::SpindleStop);
                        recs.push(Rec::Coolant(forge_post::Coolant::Off));
                    }
                    recs.push(Rec::ToolChange {
                        t: t.t,
                        h: t.t,
                        name: t.tool.id.clone(),
                    });
                    current_tool = Some(t.t);
                }
                let rpm = if p.rpm > 0.0 {
                    p.rpm
                } else {
                    op.feeds.map(|f| f.rpm).unwrap_or(3000.0)
                };
                recs.push(Rec::Spindle { rpm, cw: true });
                recs.push(Rec::Coolant(p.coolant));
                recs.push(Rec::Rapid(w(p.start)));
                for m in &p.moves {
                    recs.push(match *m {
                        PMove::Rapid { to } => Rec::Rapid(w(to)),
                        PMove::Line { to, f, .. } => Rec::Line(w(to), f),
                        PMove::Arc { to, c, cw, f } => Rec::Arc {
                            to: w(to),
                            c: [c[0] - o[0], c[1] - o[1]],
                            cw,
                            f,
                        },
                        PMove::Cycle {
                            kind,
                            holes,
                            z,
                            r,
                            clear,
                            q,
                            dwell,
                            f,
                        } => Rec::Cycle {
                            kind,
                            holes: vec![[holes[0] - o[0], holes[1] - o[1]]],
                            z: z - o[2],
                            r: r - o[2],
                            clear: clear - o[2],
                            q,
                            dwell,
                            f,
                        },
                    });
                }
                spans.push((oi, pi, start, recs.len()));
            }
        }
        for &oi in &op_ids {
            let op = &mut self.ops[oi];
            (
                op.moves,
                op.pairs,
                op.cells,
                op.length_cut,
                op.length_rapid,
                op.time_s,
            ) = (0, 0, 0, 0.0, 0.0, 0.0);
        }
        recs.push(Rec::SpindleStop);
        recs.push(Rec::Coolant(forge_post::Coolant::Off));
        if post != "grbl" {
            recs.push(Rec::Home);
        }
        let program = forge_post::Program {
            number,
            title: name.into(),
            recs,
        };
        let q = match forge_post::quantize(&program, &d) {
            Ok(q) => q,
            Err(e) => {
                po.error = Some(format!("{e:?}"));
                return po;
            }
        };
        match forge_post::round_trip(&q, &d) {
            Ok((txt, _)) => {
                po.round_trip = true;
                po.lines = txt.lines().count();
                po.text = Some(txt);
            }
            Err(e) => {
                po.error = Some(e);
                return po;
            }
        }
        // certify what the text means: each op's records, quantised, as motion, back in the
        // part's frame
        let scale = 10f64.powi(d.decimals as i32);
        let fscale = 10f64.powi(d.feed_decimals as i32);
        for (oi, pi, a, b) in spans {
            let s = &self.setups[self.ops[oi].setup];
            let o = s.origin;
            let sub = forge_post::QProgram {
                number: 0,
                title: String::new(),
                recs: q.recs[a..b].to_vec(),
                decimals: d.decimals,
                feed_decimals: d.feed_decimals,
            };
            let ms = forge_post::motions(&sub);
            let back = |p: [i64; 3]| {
                [
                    p[0] as f64 / scale + o[0],
                    p[1] as f64 / scale + o[1],
                    p[2] as f64 / scale + o[2],
                ]
            };
            let path = &self.ops[oi].paths[pi];
            let mut moves = vec![];
            let mut start: Option<[f64; 3]> = None;
            let (mut lc, mut lr, mut time) = (0.0, 0.0, 0.0);
            let mut at = [0.0; 3];
            for m in &ms {
                let (to, ph, f) = match m {
                    forge_post::Motion::Rapid(p) => (back(*p), Phase::Rapid, 0.0),
                    forge_post::Motion::Feed(p, f) => (back(*p), Phase::Cut, *f as f64 / fscale),
                    forge_post::Motion::Arc { to, c, cw, f, .. } => {
                        let t = back(*to);
                        if start.is_none() {
                            start = Some(t);
                        }
                        let r = ((at[0] - (c[0] as f64 / scale + o[0])).powi(2)
                            + (at[1] - (c[1] as f64 / scale + o[1])).powi(2))
                        .sqrt();
                        lc += r * 0.5 * std::f64::consts::PI;
                        moves.push(PMove::Arc {
                            to: t,
                            c: [c[0] as f64 / scale + o[0], c[1] as f64 / scale + o[1]],
                            cw: *cw,
                            f: *f as f64 / fscale,
                        });
                        at = t;
                        continue;
                    }
                    _ => continue,
                };
                if start.is_none() {
                    start = Some(to);
                    at = to;
                    continue;
                }
                let l = ((to[0] - at[0]).powi(2) + (to[1] - at[1]).powi(2) + (to[2] - at[2]).powi(2)).sqrt();
                if ph == Phase::Rapid {
                    lr += l;
                    time += l / 10_000.0 * 60.0;
                } else {
                    lc += l;
                    time += if f > 0.0 { l / f * 60.0 } else { 0.0 };
                }
                // a feed move straight down is a plunge; a rapid straight back up its column, from
                // where a plunge ended, retraces positions already checked: a link
                let ph = if ph == Phase::Cut && to[0] == at[0] && to[1] == at[1] && to[2] < at[2] {
                    Phase::Plunge
                } else {
                    ph
                };
                let ph = if ph == Phase::Rapid
                    && to[0] == at[0]
                    && to[1] == at[1]
                    && to[2] > at[2]
                    && matches!(
                        moves.last(),
                        Some(PMove::Line {
                            phase: Phase::Plunge,
                            ..
                        })
                    ) {
                    Phase::Link
                } else {
                    ph
                };
                moves.push(if ph == Phase::Rapid {
                    PMove::Rapid { to }
                } else {
                    PMove::Line { to, f, phase: ph }
                });
                at = to;
            }
            let posted = OpPath {
                tool: path.tool,
                start: start.unwrap_or(path.start),
                moves,
                rpm: path.rpm,
                coolant: path.coolant,
            };
            let tool = self.tools[path.tool].clone();
            let part = &self.parts[s.part];
            // the stock at each move: the op's start, then after each completed level (a
            // retract after cutting at depth d), its voids cleared down to d (levels go down)
            let base = self.ops[oi]
                .stock_before
                .clone()
                .unwrap_or_else(|| forge_cam::stock::InProcess::new(s.stock.min, s.stock.max));
            let faces_of = |ip: &forge_cam::stock::InProcess| {
                let t0 = std::time::Instant::now();
                let r = ip.faces();
                if std::env::var("FORGE_CAM_DEBUG").is_ok() {
                    eprintln!(
                        "{}: stock faces {:?} in {:?} (voids {}, under {})",
                        s.name,
                        r.as_ref().map(|f| f.len()),
                        t0.elapsed(),
                        ip.voids.len(),
                        ip.under.len()
                    );
                }
                r.unwrap_or_else(|e| {
                    if std::env::var("FORGE_CAM_DEBUG").is_ok() {
                        eprintln!("stock faces: {e}");
                    }
                    s.stock.faces()
                })
            };
            let mut stages = vec![(0usize, faces_of(&base))];
            let mut deepest = f64::INFINITY;
            let mut cut_z: Option<f64> = None;
            for (i, m) in posted.moves.iter().enumerate() {
                match *m {
                    // a level's depth: the lowest point it cut (the feed back up is not one)
                    PMove::Line {
                        to,
                        phase: Phase::Cut,
                        ..
                    }
                    | PMove::Arc { to, .. } => cut_z = Some(cut_z.map_or(to[2], |c: f64| c.min(to[2]))),
                    PMove::Line {
                        phase: Phase::Link, ..
                    }
                    | PMove::Rapid { .. } => {
                        if let Some(z) = cut_z.take()
                            && z < deepest
                            && !self.ops[oi].voids.is_empty()
                        {
                            deepest = z;
                            let mut ip = base.clone();
                            for (v, floor) in &self.ops[oi].voids {
                                let f = floor.max(z);
                                match v {
                                    None => ip.face_to(f),
                                    Some(r) => ip.cut(r.clone(), f),
                                }
                            }
                            if std::env::var("FORGE_CAM_DEBUG").is_ok() {
                                let m = ip.material(z + 0.1).map(|m| {
                                    m.map(|m| {
                                        (
                                            m.loops.len(),
                                            m.loops.iter().map(|l| l.segs.len()).collect::<Vec<_>>(),
                                        )
                                    })
                                });
                                eprintln!("stage at {z}: levels {:?} material above {m:?}", ip.levels());
                            }
                            stages.push((i, faces_of(&ip)));
                        }
                    }
                    _ => {}
                }
            }
            if std::env::var("FORGE_CAM_DEBUG").is_ok() {
                eprintln!(
                    "{}: voids {:?} stages {:?}",
                    self.ops[oi].name,
                    self.ops[oi]
                        .voids
                        .iter()
                        .map(|v| (v.0.as_ref().map(|r| r.loops.len()), v.1))
                        .collect::<Vec<_>>(),
                    stages.iter().map(|s| (s.0, s.1.len())).collect::<Vec<_>>()
                );
            }
            let (ffaces, fclear) = match &s.fixture {
                Some(f) => (f.faces(), f.clearance),
                None => (vec![], 0.0),
            };
            let target = self.ops[oi].target.as_ref().unwrap_or(&part.faces);
            let mut sum = ops::certify_fixtured(
                &posted,
                &tool,
                target,
                &stages,
                &ffaces,
                fclear,
                s.tol,
                s.clearance,
            );
            if let Some(f) = &s.fixture {
                for c in &mut sum.checks {
                    if c.rule == "fixture_collision"
                        && let Some(id) = c.other.strip_prefix("fixture face ").and_then(|x| x.parse().ok())
                        && let Some(p) = f.piece_of(id)
                    {
                        c.other = format!("{} ({p})", c.other);
                    }
                }
            }
            // G5: every position the posted path reaches, as the machine's joints see it
            if let Some(c) = self.axis_limits(oi, &posted) {
                sum.clear = false;
                sum.checks.insert(0, c);
            }
            // the lowest the tool's tip goes in a move that met the fixture
            let ends: Vec<[f64; 3]> = std::iter::once(posted.start)
                .chain(posted.moves.iter().scan(posted.start, |at, m| {
                    match *m {
                        PMove::Rapid { to } | PMove::Line { to, .. } | PMove::Arc { to, .. } => *at = to,
                        PMove::Cycle { .. } => {}
                    }
                    Some(*at)
                }))
                .collect();
            let low_fix = sum
                .checks
                .iter()
                .filter(|c| c.rule == "fixture_collision")
                .filter_map(|c| Some(ends.get(c.move_index)?[2].min(ends.get(c.move_index + 1)?[2])))
                .fold(f64::INFINITY, f64::min);
            let op = &mut self.ops[oi];
            op.moves += posted.moves.len();
            op.pairs += sum.pairs;
            op.cells += sum.cells;
            op.least = op.least.min(sum.least);
            op.length_cut += lc;
            op.length_rapid += lr;
            op.time_s += time;
            op.checks.extend(sum.checks);
            if !sum.clear && op.status == "built" {
                op.status = "refused";
                let c = &op.checks[0];
                op.refusal = Some(format!(
                    "{} {} at move {} ({}): {} vs {} on {}",
                    c.rule, c.verdict, c.move_index, c.phase, c.value, c.limit, c.other
                ));
                op.fix = Some(match c.rule {
                    "axis_limit" => "SetArg at: a work offset that keeps the program within travel, or the part placed elsewhere on the table (or a shorter tool for Z)".into(),
                    "fixture_collision" => {
                        let f = s.fixture.as_ref().expect("a fixture check has a fixture");
                        let seat = s.stock.min[2];
                        let clamp = f.jaw_top - seat;
                        let fit = low_fix - f.clearance - seat;
                        if low_fix.is_finite() && low_fix < f.jaw_top + f.clearance && fit > 0.0 {
                            format!(
                                "SetArg clamp: grip at most {fit:.2} mm (now {clamp:.2}): raise the stock in the vise; or leave below z = {:.2} to a second setup; or SetArg leave",
                                f.jaw_top + f.clearance
                            )
                        } else {
                            format!(
                                "the cut reaches the jaws (top z = {:.2}): split the setup (machine this in a second setup, the part turned over), or SetArg clear: a higher clearance height for the rapids",
                                f.jaw_top
                            )
                        }
                    }
                    "holder_collision" if c.other.starts_with("stock") => "SetArg tool: a tool whose flutes reach the depth (lcf above the stock top), or clear the stock above first".into(),
                    "holder_collision" => "SetArg stickout: a longer stick-out, or a smaller holder".into(),
                    "rapid_clear" => "SetArg clear: a higher clearance height".into(),
                    _ => "SetArg leave: leave stock, or choose a smaller tool".into(),
                });
            }
        }
        if self
            .ops
            .iter()
            .any(|o| setups.contains(&o.setup) && o.status != "built")
        {
            po.error = Some("an operation was refused: the program is not released".into());
            po.text = None;
        }
        po
    }

    /// Whether a program's setups are all placed (`at:`), so G5 checked its positions.
    fn placed(&self, p: &ProgramOut) -> bool {
        p.setups.iter().all(|&i| self.setups[i].at.is_some())
    }

    /// "full": every check proven, G5 on every position; "except G5": released, but a posted
    /// setup is not placed, so G5 checked spans only; "refused": not released.
    pub fn certification(&self) -> &'static str {
        if !self.ok {
            "refused"
        } else if self.programs.iter().all(|p| self.placed(p)) {
            "full"
        } else {
            "except G5"
        }
    }

    /// The reply, `forgecad.cam/1`.
    pub fn json(&self) -> Json {
        let n = |x: f64| Json::Num(x);
        let v3 = |p: [f64; 3]| Json::Arr(vec![n(p[0]), n(p[1]), n(p[2])]);
        let setups = self
            .setups
            .iter()
            .map(|s| {
                Json::obj(vec![
                    ("name", Json::Str(s.name.clone())),
                    ("part", Json::Str(self.parts[s.part].file.clone())),
                    ("machine", Json::Str(s.machine.clone())),
                    (
                        "wcs",
                        Json::obj(vec![
                            ("origin", v3(s.origin)),
                            ("axes", Json::Str("part".into())),
                            ("offset", n(s.wcs as f64)),
                        ]),
                    ),
                    (
                        "stock",
                        Json::obj(vec![
                            ("kind", Json::Str("box".into())),
                            ("min", v3(s.stock.min)),
                            ("max", v3(s.stock.max)),
                        ]),
                    ),
                    ("tol", n(s.tol)),
                    ("clearance", n(s.clearance)),
                    ("at", s.at.map(v3).unwrap_or(Json::Null)),
                    (
                        "fixture",
                        match &s.fixture {
                            None => Json::Null,
                            Some(f) => Json::obj(vec![
                                ("kind", Json::Str("vise".into())),
                                ("name", Json::Str(f.name.clone())),
                                ("across", Json::Str(if f.axis == 0 { "x" } else { "y" }.into())),
                                ("jaw_top", n(f.jaw_top)),
                                ("clamp", n(f.jaw_top - s.stock.min[2])),
                                ("clearance", n(f.clearance)),
                                (
                                    "pieces",
                                    Json::Arr(
                                        f.boxes()
                                            .into_iter()
                                            .map(|(k, lo, hi)| {
                                                Json::obj(vec![
                                                    ("name", Json::Str(k)),
                                                    ("min", v3(lo)),
                                                    ("max", v3(hi)),
                                                ])
                                            })
                                            .collect(),
                                    ),
                                ),
                            ]),
                        },
                    ),
                    (
                        "heights",
                        Json::obj(vec![
                            ("clear", n(s.heights.clear)),
                            ("retract", n(s.heights.retract)),
                        ]),
                    ),
                ])
            })
            .collect();
        let ops = self
            .ops
            .iter()
            .map(|o| {
                let tool = o.tool.map(|i| {
                    let t = &self.tools[i];
                    Json::obj(vec![
                        ("id", Json::Str(t.tool.id.clone())),
                        ("T", n(t.t as f64)),
                        ("H", n(t.t as f64)),
                        ("type", Json::Str(t.tool.kind.name().into())),
                        ("d", n(t.tool.dc)),
                        ("holder", t.holder.clone().map(Json::Str).unwrap_or(Json::Null)),
                        ("stickout", n(t.stickout)),
                    ])
                });
                let feeds = o.feeds.map(|f| {
                    Json::obj(vec![
                        ("rpm", n(f.rpm)),
                        ("vc", n(f.vc)),
                        ("fz", n(f.fz)),
                        ("hex", n(f.hex)),
                        ("f", n(f.vf)),
                        ("power_kw", n(f.power)),
                    ])
                });
                let checks: Vec<Json> = o
                    .checks
                    .iter()
                    .map(|c| {
                        Json::obj(vec![
                            ("rule", Json::Str(c.rule.into())),
                            ("verdict", Json::Str(c.verdict.into())),
                            ("certified", true.into()),
                            ("value", n(c.value)),
                            ("limit", n(c.limit)),
                            ("unit", Json::Str("mm".into())),
                            ("at", v3(c.at)),
                            ("tool_part", Json::Str(c.tool_part.into())),
                            ("other", Json::Str(c.other.clone())),
                            ("phase", Json::Str(c.phase.into())),
                            ("move", n(c.move_index as f64)),
                        ])
                    })
                    .collect();
                let summary = Json::obj(vec![
                    ("rule", Json::Str("gouge, holder_collision, rapid_clear".into())),
                    (
                        "verdict",
                        Json::Str(if o.status == "built" { "pass" } else { "fail" }.into()),
                    ),
                    ("certified", true.into()),
                    ("pairs", n(o.pairs as f64)),
                    ("cells", n(o.cells as f64)),
                    (
                        "least_cutter_distance",
                        if o.least.is_finite() {
                            n(o.least)
                        } else {
                            Json::Null
                        },
                    ),
                    (
                        "against",
                        Json::Str(
                            "the path the posted text means (round trip), not the unrounded one".into(),
                        ),
                    ),
                ]);
                Json::obj(vec![
                    ("name", Json::Str(o.name.clone())),
                    ("kind", Json::Str(o.kind.into())),
                    ("setup", Json::Str(self.setups[o.setup].name.clone())),
                    ("status", Json::Str(o.status.into())),
                    ("tool", tool.unwrap_or(Json::Null)),
                    (
                        "plan",
                        Json::Arr(o.plan.iter().map(|s| Json::Str(s.clone())).collect()),
                    ),
                    (
                        "removals",
                        Json::Arr(o.removals.iter().map(|s| Json::Str(s.clone())).collect()),
                    ),
                    ("feeds", feeds.unwrap_or(Json::Null)),
                    (
                        "path",
                        Json::obj(vec![
                            ("moves", n(o.moves as f64)),
                            ("length_cut", n(o.length_cut)),
                            ("length_rapid", n(o.length_rapid)),
                            ("time_s", n(o.time_s)),
                        ]),
                    ),
                    ("certificate", summary),
                    ("checks", Json::Arr(checks)),
                    (
                        "leftover",
                        Json::obj(vec![(
                            "status",
                            Json::Str("not computed in this phase (G4 pending)".into()),
                        )]),
                    ),
                    ("refusal", o.refusal.clone().map(Json::Str).unwrap_or(Json::Null)),
                ])
            })
            .collect();
        let blocking: Vec<Json> = self
            .ops
            .iter()
            .filter(|o| o.status != "built")
            .map(|o| {
                Json::obj(vec![
                    ("op", Json::Str(o.name.clone())),
                    ("why", Json::Str(o.refusal.clone().unwrap_or_default())),
                ])
            })
            .collect();
        let fixes: Vec<Json> = self
            .ops
            .iter()
            .filter_map(|o| {
                o.fix.as_ref().map(|f| {
                    Json::obj(vec![
                        ("op", Json::Str(o.name.clone())),
                        ("fix", Json::Str(f.clone())),
                    ])
                })
            })
            .collect();
        let suggestions: Vec<Json> = self
            .ops
            .iter()
            .filter(|o| o.refusal.as_ref().is_some_and(|r| r.starts_with("NeedTool")))
            .map(|o| {
                Json::obj(vec![
                    ("kind", Json::Str("NeedTool".into())),
                    ("op", Json::Str(o.name.clone())),
                    ("why", Json::Str(o.refusal.clone().unwrap())),
                ])
            })
            .collect();
        let programs = self
            .programs
            .iter()
            .map(|p| {
                Json::obj(vec![
                    ("name", Json::Str(p.name.clone())),
                    ("post", Json::Str(p.post.clone())),
                    ("lines", n(p.lines as f64)),
                    ("round_trip", p.round_trip.into()),
                    (
                        "certified",
                        Json::Str(
                            if p.error.is_some() {
                                "no"
                            } else if self.placed(p) {
                                "full"
                            } else {
                                "except G5 (unchecked: no `at:`)"
                            }
                            .into(),
                        ),
                    ),
                    ("error", p.error.clone().map(Json::Str).unwrap_or(Json::Null)),
                ])
            })
            .collect();
        Json::obj(vec![
            ("ok", self.ok.into()),
            ("certification", Json::Str(self.certification().into())),
            ("schema", Json::Str("forgecad.cam/1".into())),
            ("errors", Json::Arr(self.errors.iter().map(|e| Json::Str(e.clone())).collect())),
            ("warnings", Json::Arr(self.warnings.iter().map(|e| Json::Str(e.clone())).collect())),
            ("setups", Json::Arr(setups)),
            ("ops", Json::Arr(ops)),
            ("blocking", Json::Arr(blocking)),
            ("fixes", Json::Arr(fixes)),
            ("suggestions", Json::Arr(suggestions)),
            ("programs", Json::Arr(programs)),
            (
                "leftover",
                Json::obj(vec![
                    ("computed", self.leftover_done.into()),
                    ("rule", Json::Str("G4: final stock ⊆ (part ⊕ B_{leave + tol}) ∪ these regions, by z band".into())),
                    (
                        "regions",
                        Json::Arr(
                            self.leftover
                                .iter()
                                .map(|l| {
                                    Json::obj(vec![
                                        ("setup", Json::Str(self.setups[l.setup].name.clone())),
                                        ("z", Json::Arr(vec![Json::Num(l.z0), Json::Num(l.z1)])),
                                        ("boxes", Json::Arr(l.boxes.iter().map(|b| Json::Arr(b.iter().map(|x| Json::Num(*x)).collect())).collect())),
                                        ("why", Json::Str(l.why.into())),
                                    ])
                                })
                                .collect(),
                        ),
                    ),
                ]),
            ),
            (
                "scope",
                Json::Str("Proven: the cutter, shank and holder clear the exact part (within tol for the cutter on cuts), rapids clear part and stock, and every body of the tool keeps the fixture's clearance from the fixture as modelled (a placed vise's boxes), on the posted path. Not claimed: cutting physics, the machine's fidelity, the true in-process stock beyond the proven-cleared voids.".into()),
            ),
        ])
    }

    /// The setup sheet, as HTML.
    pub fn sheet(&self) -> String {
        let esc = |s: &str| s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
        let mut h = String::from(
            "<!DOCTYPE html>\n<html><head><meta charset=\"utf-8\"><title>Setup sheet</title><style>body{font-family:sans-serif;margin:2em}table{border-collapse:collapse;margin:1em 0}td,th{border:1px solid #999;padding:4px 8px;text-align:left}th{background:#eee}.refused{color:#a00}</style></head><body>\n<h1>Setup sheet</h1>\n",
        );
        for s in &self.setups {
            h.push_str(&format!(
                "<h2>Setup {}</h2>\n<p>Part <b>{}</b> on <b>{}</b>. WCS G{} at ({:.3}, {:.3}, {:.3}) of the part's frame. Stock box {:.2} × {:.2} × {:.2} mm. Tolerance {} mm; clearance {} mm; clear height {:.2}, retract {:.2} (part frame).</p>\n",
                esc(&s.name),
                esc(&self.parts[s.part].file),
                esc(&s.machine),
                54 + s.wcs,
                s.origin[0],
                s.origin[1],
                s.origin[2],
                s.stock.max[0] - s.stock.min[0],
                s.stock.max[1] - s.stock.min[1],
                s.stock.max[2] - s.stock.min[2],
                s.tol,
                s.clearance,
                s.heights.clear,
                s.heights.retract
            ));
            if let Some(f) = &s.fixture {
                h.push_str(&format!(
                    "<p>Held in the vise <b>{}</b>, jaws closed along {}, gripping {:.2} mm of the stock (jaw top z = {:.2}, part frame), on parallels; every move keeps {} mm from the vise as modelled. Measure the vise before trusting the clearance.</p>\n",
                    esc(&f.name),
                    if f.axis == 0 { "x" } else { "y" },
                    f.jaw_top - s.stock.min[2],
                    f.jaw_top,
                    f.clearance
                ));
            }
        }
        h.push_str("<h2>Tools</h2>\n<table><tr><th>T</th><th>tool</th><th>type</th><th>D</th><th>flutes</th><th>holder</th><th>stick-out</th><th>source</th></tr>\n");
        for t in &self.tools {
            h.push_str(&format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>\n",
                t.t,
                esc(&t.tool.id),
                t.tool.kind.name(),
                t.tool.dc,
                t.tool.nof,
                esc(t.holder.as_deref().unwrap_or("-")),
                t.stickout,
                esc(&format!("{}; {}", t.tool.source, t.tool.note))
            ));
        }
        h.push_str("</table>\n<h2>Operations</h2>\n<table><tr><th>op</th><th>kind</th><th>tool</th><th>rpm</th><th>feed</th><th>moves</th><th>cut mm</th><th>time</th><th>certificate</th></tr>\n");
        for o in &self.ops {
            let t = o
                .tool
                .map(|i| format!("T{} {}", self.tools[i].t, self.tools[i].tool.id))
                .unwrap_or_default();
            h.push_str(&format!(
                "<tr class=\"{}\"><td>{}</td><td>{}{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{:.0}</td><td>{:.0} s</td><td>{}</td></tr>\n",
                if o.status == "built" { "" } else { "refused" },
                esc(&o.name),
                o.kind,
                if o.plan.is_empty() { String::new() } else { format!(" ({})", esc(&o.plan.join("; "))) },
                esc(&t),
                o.feeds.map(|f| format!("{:.0}", f.rpm)).unwrap_or("-".into()),
                o.feeds.map(|f| format!("{:.0}", f.vf)).unwrap_or("-".into()),
                o.moves,
                o.length_cut,
                o.time_s,
                if o.status == "built" { "pass (certified)".to_string() } else { format!("refused: {}", esc(o.refusal.as_deref().unwrap_or(""))) }
            ));
        }
        h.push_str("</table>\n<p>Feeds and speeds are starting points from generic data: verify with the tool maker. The certificate covers the geometry only.</p>\n</body></html>\n");
        h
    }

    /// The first program's text, if released.
    pub fn program_text(&self) -> Option<(&str, &str)> {
        self.programs
            .iter()
            .find_map(|p| p.text.as_deref().map(|t| (t, p.post.as_str())))
    }
}
