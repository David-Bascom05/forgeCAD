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
    Stock(StockSpec),
    #[allow(dead_code)]
    Op(usize),
}

#[derive(Clone, Debug)]
enum StockSpec {
    Margin([f64; 3]),
    Box { min: [f64; 3], max: [f64; 3] },
}

struct PartIn {
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
    /// The in-process stock as the ops so far left it.
    ip: forge_cam::stock::InProcess,
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
    if let Ok(rd) = std::fs::read_dir(&proj) {
        let mut files: Vec<PathBuf> = rd
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x == "toml"))
            .collect();
        files.sort();
        for f in files {
            if let Ok(s) = std::fs::read_to_string(&f)
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
                    let t0 = std::time::Instant::now();
                    let v = cam.call(name, fname, &a, dir)?;
                    if std::env::var("FORGE_CAM_TIME").is_ok() {
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
            let part = &self.parts[s.part];
            let mut zs = s.ip.levels();
            for f in &part.faces {
                if let Some(forge_cam_geom::slab::Prism::Flat { z, .. }) = &f.prism {
                    zs.push(*z);
                }
            }
            zs.retain(|z| *z >= s.ip.min[2] && *z <= s.ip.max[2]);
            zs.sort_by(|a, b| a.partial_cmp(b).unwrap());
            zs.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
            for w in zs.windows(2) {
                let zm = 0.5 * (w[0] + w[1]);
                let Ok(Some(mat)) = s.ip.material(zm) else {
                    if s.ip.material(zm).is_err() {
                        all_ok = false;
                    }
                    continue;
                };
                let left = match forge_cam::section::section(&part.faces, zm, 1e-5) {
                    Ok(sec) => match forge_cam_geom::offset::offset(&sec, -s.tol) {
                        Ok((grown, _)) => forge_cam_geom::boolean::boolean(
                            &mat,
                            &forge_cam_geom::boolean::region_of(&grown),
                            forge_cam_geom::boolean::Op::Diff,
                        ),
                        Err(e) => Err(e),
                    },
                    // no part at this height: all the material is left over
                    Err(forge_cam::section::SectionError::Empty) => Ok(mat
                        .loops
                        .iter()
                        .map(|l| forge_cam_geom::offset::OLoop { segs: l.segs.clone() })
                        .collect()),
                    Err(_) => {
                        all_ok = false;
                        continue;
                    }
                };
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
                                }
                                b
                            })
                            .collect();
                        self.leftover.push(Leftover {
                            setup: si,
                            z0: w[0],
                            z1: w[1],
                            boxes,
                        });
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
                let src = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
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
                let part = match a.get("#0") {
                    Some(V::Part(i)) => *i,
                    _ => return Err("setup(PART, ..)".into()),
                };
                let machine = match a.get("machine") {
                    Some(V::Machine(m)) => m.clone(),
                    _ => "generic-vmc3".into(),
                };
                let (plo, phi) = self.parts[part].bbox;
                let stock = match a.get("stock") {
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
                let clear = stock.max[2] + get_num(a, "clear", Some(10.0))?;
                let retract = stock.max[2] + get_num(a, "retract", Some(3.0))?;
                self.setups.push(SetupIn {
                    name: name.into(),
                    part,
                    machine,
                    stock,
                    origin,
                    wcs: get_num(a, "wcs", Some(0.0))? as u32,
                    tol: get_num(a, "tol", Some(0.01))?,
                    clearance: get_num(a, "clearance", Some(0.1))?,
                    heights: Heights { clear, retract },
                    ip: forge_cam::stock::InProcess::new(stock.min, stock.max),
                });
                Ok(V::Setup(self.setups.len() - 1))
            }
            "facing" | "contour" | "pocket" | "clear" | "drill" => self.op(name, f, a),
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
                "`{f}` is not a machining built-in (part, machine, tool, setup, facing, contour, pocket, drill, program, box)"
            )),
        }
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
                    "facing" => {
                        let z = get_num(a, "to", Some(part.bbox.1[2]))?;
                        let p = ops::facing(&s.stock, t, ti, h, s.stock.max[2], z, step, (vf, vp));
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
                        let g = r;
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
                            t,
                            ti,
                            h,
                            top,
                            bottom,
                            step,
                            stepover,
                            get_num(a, "leave", Some(0.0))?,
                            (vf, vp),
                        )
                        .map_err(|e| format!("{e:?}"))?;
                        if stepover <= r {
                            if let Some(o) = opening(&rg, r) {
                                voids.push((Some(o), bottom));
                            }
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
                                ) {
                                    if let Ok(band) = forge_cam_geom::boolean::boolean(
                                        &forge_cam_geom::boolean::region_of(&outer),
                                        &forge_cam_geom::boolean::region_of(&inner),
                                        forge_cam_geom::boolean::Op::Diff,
                                    ) {
                                        voids.push((Some(forge_cam_geom::boolean::region_of(&band)), bottom));
                                    }
                                }
                            }
                            ops::contour(&rg, t, ti, h, top, bottom, step, leave, true, (vf, vp))
                                .map(|p| vec![p])
                                .map_err(|e| format!("{e:?}"))
                        } else {
                            ops::contour(&rg, t, ti, h, top, bottom, step, leave, false, (vf, vp))
                                .map(|p| vec![p])
                                .map_err(|e| format!("{e:?}"))
                        }
                    }
                    _ => {
                        // a pocket of a hole feature: its circle, its depth
                        let (pi, hn) = match a.get("#1") {
                            Some(V::Holes(p, n)) => (*p, n.clone()),
                            _ => return Err("pocket(SETUP, PART.HOLE, tool: ..)".into()),
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
                                    t,
                                    ti,
                                    h,
                                    p[2],
                                    bottom,
                                    step,
                                    stepover,
                                    get_num(a, "leave", Some(0.0))?,
                                    (vf, vp),
                                )
                                .map_err(|e| format!("{e:?}"))?,
                            );
                            if stepover <= 0.5 * d {
                                if let Some(o) = opening(&rg, 0.5 * d) {
                                    voids.push((Some(o), bottom));
                                }
                            }
                        }
                        Ok(paths)
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
            Some(V::Holes(p, n)) => (*p, n.clone()),
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
        let tap = match hole.thread {
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
            let dspot = 0.45 * drill_d / (0.5 * t.tool.angle).to_radians().tan().max(1e-9);
            let mut p = ops::holes(&hole.places, CycleKind::Drill, t, ti, h, 0.0, 0.0, f);
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
            t,
            ti,
            h,
            depth,
            if peck { t.tool.dc } else { 0.0 },
            f,
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
                t,
                ti,
                h,
                depth.min(hole.depth.unwrap_or(depth)),
                0.0,
                f,
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
fn opening(rg: &Region, r: f64) -> Option<Region> {
    let (c, _) = forge_cam_geom::offset::offset(rg, r).ok()?;
    if c.is_empty() {
        return None;
    }
    let cr = forge_cam_geom::boolean::region_of(&c);
    let (o, _) = forge_cam_geom::offset::offset(&cr, -r).ok()?;
    Some(forge_cam_geom::boolean::region_of(&o))
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
                    forge_post::Motion::Arc { to, c, cw, f } => {
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
                ip.faces().unwrap_or_else(|e| {
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
            let sum = ops::certify_staged(&posted, &tool, &part.faces, &stages, s.tol, s.clearance);
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
                    ("error", p.error.clone().map(Json::Str).unwrap_or(Json::Null)),
                ])
            })
            .collect();
        Json::obj(vec![
            ("ok", self.ok.into()),
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
                    ("rule", Json::Str("G4: final stock ⊆ (part ⊕ B_tol) ∪ these regions, by z band".into())),
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
                                        ("why", Json::Str("uncut".into())),
                                    ])
                                })
                                .collect(),
                        ),
                    ),
                ]),
            ),
            (
                "scope",
                Json::Str("Proven: the cutter, shank and holder clear the exact part (within tol for the cutter on cuts), rapids clear part and stock, on the posted path. Not claimed: cutting physics, the machine's fidelity, the true in-process stock beyond the proven-cleared voids.".into()),
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
