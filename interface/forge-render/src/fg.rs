//! Forge (.fg) parts in the app: the source, typed edits applied through forge-lang, and a worker
//! that keeps the build's stages from run to run (as `fg serve` does), so a rebuild starts at the
//! first feature that changed and an unchanged one costs milliseconds.

use crate::live::Mesher;
use crate::livegpu::Delta;
use crate::mesh::PartMesh;
use crate::render::{self, Entity, Renderer, part_info};
use crate::scene::{Placement, Scene};
use crate::shell::fg::{DimRow, FeatRow, From, Issue, ParamRow, Severity, Status};
use crate::tess;
use forge_lang::build::Cache;
use forge_lang::edit::{self, Reach};
use forge_lang::error::{Error, Op, Warning};
use forge_lang::eval::Value;
use forge_lang::values::{Dim, Src};
use forge_lang::{Measure, Options, Run, Timings};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Instant;

pub type Gpu = (wgpu::Device, wgpu::Queue, wgpu::AdapterInfo);

/// A text to build: its sequence number, the text, and whether an assembly runs its checks (the
/// overlap check and its expects).
pub type Job = (u64, String, bool);

// plain data, so the shell has it for the browser too
pub use crate::shell::asmdrag::Drive;

/// What the worker is given: a text to build, or drives set straight in the kept solve.
#[derive(Clone)]
enum Work {
    Text(Job),
    Drive(u64, Vec<Drive>),
}

/// A placement's rows from a pose: the frame's axes as columns, its origin last.
pub fn rows_of(f: &forge_math::Frame) -> crate::scene::Rows {
    let (x, y, z, o) = (f.x().get(), f.y().get(), f.z().get(), f.origin);
    [
        [x.x as f32, y.x as f32, z.x as f32, o.x as f32],
        [x.y as f32, y.y as f32, z.y as f32, o.y as f32],
        [x.z as f32, y.z as f32, z.z as f32, o.z as f32],
    ]
}

/// One part occurrence of an assembly: its path, part file, and its placement (none when the
/// part has no solid to draw).
#[derive(Clone, Debug, PartialEq)]
pub struct Occ {
    pub path: String,
    pub file: String,
    pub placement: Option<usize>,
    /// Its part's thermoform moulds, placed with it, drawn when their row's eye says.
    pub tooling: Option<usize>,
}

/// An assembly's solve for the screen and the panel. Each distinct part is meshed once (by its
/// content) and drawn at every occurrence.
#[derive(Clone, Debug, Default)]
pub struct AsmView {
    pub occ: Vec<Occ>,
    /// Each placement's mesh (an index into the meshes so far) and pose.
    pub placed: Vec<(usize, crate::scene::Rows)>,
    /// Meshes new since the last build, by index.
    pub meshes: Vec<(usize, PartMesh)>,
    /// Which meshes are placed where changed: every mesh so far, for the host to make the view
    /// again (the worker holds no GPU after the first: a device kept on a thread outlives the
    /// driver at exit).
    pub restage: Option<Vec<PartMesh>>,
    pub joints: Vec<forge_lang::asm::JointOut>,
    pub mobility: usize,
    pub bom: Vec<(String, usize)>,
    pub expects: Vec<forge_lang::asm::ExpectOut>,
    /// The overlap check's counts, when it ran: interferences, intended contacts.
    pub overlap: Option<(usize, usize)>,
    pub timings: forge_lang::asm::AsmTimings,
    /// Distinct parts and how many were meshed this time.
    pub parts: usize,
    pub meshed: usize,
    /// Each mesh's box in its own space, by index.
    pub boxes: Vec<crate::explode::Aabb>,
    /// The grounded top-level instances.
    pub grounded: Vec<String>,
    /// Every occurrence and link through subassemblies, as the documents write them.
    pub insts: Vec<forge_lang::asm::InstOut>,
    pub links: Vec<forge_lang::asm::LinkOut>,
    /// Where each link's two ends are in the world, solved.
    pub link_at: Vec<[Option<forge_math::Frame>; 2]>,
    /// Each top-level instance's pose: where a subassembly's origin is.
    pub poses: Vec<(String, forge_math::Frame)>,
    pub inspect: Arc<crate::gui::gui_inspect::InspectData>,
    /// The part files with a thermoform, as the occurrences name them.
    pub thermo_files: Vec<String>,
    /// Each body of the file and the features that made it (a body component's tree).
    pub body_chains: Vec<(String, Vec<String>)>,
}

/// What one build gave: the panel's data, and the mesh when the part's shape changed.
pub struct Built {
    pub seq: u64,
    pub ok: bool,
    pub feats: Vec<FeatRow>,
    pub params: Vec<ParamRow>,
    pub issues: Vec<Issue>,
    pub measure: Option<Measure>,
    pub timings: Timings,
    pub rebuilt: usize,
    pub reused: usize,
    /// The part's new mesh; none when the part is the one already shown, or didn't build.
    pub mesh: Option<PartMesh>,
    /// The faces the build changed, for the screen at once (the whole mesh follows).
    pub delta: Option<Delta>,
    /// When `mesh` is new: the flat pattern of the part's first sheet; none without one.
    pub flat: Option<PartMesh>,
    /// Each sheet metal part: its id, how many bends, and its flat pattern as DXF.
    pub sheets: Vec<SheetInfo>,
    /// Each thermoform's panel row and its wall-thickness map (world triangles).
    pub thermos: Vec<(crate::shell::thermo::ThermoRow, crate::thermo_view::Gfx)>,
    /// The first build's renderer and scene: pipelines are dear, made here and never in a frame.
    pub view: Option<(Renderer, Scene)>,
    /// What Forge names each face and edge, when the text changed since the last.
    pub anchors: Option<Arc<AnchorView>>,
    /// An assembly's solve: occurrences, poses, joints, BOM, checks.
    pub asm: Option<AsmView>,
    /// The worker's time for it: the build, and the mesh.
    pub build_ms: f64,
    pub mesh_ms: f64,
    /// The exact bodies and iProperties behind it, for Measure and the physical properties.
    pub inspect: Option<Arc<crate::gui::gui_inspect::InspectData>>,
    /// The last build's mesh of the same file (nothing it read has changed), shown while this
    /// one builds: the build it stands for is still to come.
    pub preview: bool,
    /// The checks ran (measures, expects, a thermoform's walls); else they follow in another.
    pub checked: bool,
}

impl Built {
    pub fn summary(&self) -> String {
        if self.preview {
            return "opening: the last build on screen while it builds".into();
        }
        if !self.checked {
            return "built; checking".into();
        }
        let mut s = match &self.measure {
            Some(m) => format!(
                "volume {} mm³ · {} faces · {} · ",
                crate::shell::fg::short((m.volume * 10.0).round() / 10.0),
                m.faces,
                if m.problems == 0 { "valid" } else { "INVALID" }
            ),
            None if self.ok => "nothing built · ".into(),
            None => "not built · ".into(),
        };
        s.push_str(&format!(
            "rebuilt {}, reused {} · {:.1} ms",
            self.rebuilt,
            self.reused,
            self.build_ms + self.mesh_ms
        ));
        s
    }
}

/// Each face and edge of the part (in the mesh's entity order, which is the body's) as Forge
/// names it, if anything does; the planes a sketch on one is drawn in; the sketches.
#[derive(Clone, Debug, Default)]
pub struct AnchorView {
    pub faces: Vec<Option<String>>,
    pub edges: Vec<Option<String>>,
    /// Each face's and edge's own name in the body (the kernel's, which a rebuild keeps for the
    /// same face whatever number it gets): what a selection is found again by.
    pub face_names: Vec<forge_topo::Name>,
    pub edge_names: Vec<forge_topo::Name>,
    /// How many faces each face anchor names.
    pub sizes: HashMap<String, usize>,
    pub planes: HashMap<String, forge_math::Frame>,
    pub sketches: Vec<forge_lang::anchors::SketchRefs>,
    /// Sheet edges a flange can stand on, with the part's edges (by index) each is.
    pub sheet_edges: Vec<(String, Vec<usize>)>,
}

/// An entity of the body `old` names, in the body `new` names: the one face or edge with the same
/// name in both (the kernel's own, kept for the same face across a rebuild), else none (gone, or
/// the name isn't one face's alone; a vertex isn't kept).
pub fn same_entity(e: Entity, old: &AnchorView, new: &AnchorView) -> Option<Entity> {
    fn one(list: &[forge_topo::Name], name: &forge_topo::Name) -> Option<u32> {
        let mut at = list
            .iter()
            .enumerate()
            .filter(|(_, n)| *n == name)
            .map(|(k, _)| k);
        match (at.next(), at.next()) {
            (Some(k), None) => Some(k as u32),
            _ => None,
        }
    }
    let (was, now, k) = match e {
        Entity::Face(k) => (&old.face_names, &new.face_names, k),
        Entity::Edge(k) => (&old.edge_names, &new.edge_names, k),
        Entity::Vertex(_) => return None,
    };
    let name = was.get(k as usize)?;
    one(was, name).filter(|&j| j == k)?;
    let j = one(now, name)?;
    Some(match e {
        Entity::Face(_) => Entity::Face(j),
        _ => Entity::Edge(j),
    })
}

impl AnchorView {
    fn of(a: &forge_lang::anchors::Anchors, body: Option<&forge_topo::Body>) -> AnchorView {
        let Some(b) = body else {
            return AnchorView {
                sketches: a.sketches.clone(),
                planes: a.planes.clone(),
                ..AnchorView::default()
            };
        };
        let edge_names: Vec<forge_topo::Name> = b.edge_ids().map(|e| b.edge(e).name).collect();
        AnchorView {
            faces: b
                .face_ids()
                .map(|f| a.faces.get(&b.face(f).name).cloned())
                .collect(),
            edges: edge_names.iter().map(|n| a.edges.get(n).cloned()).collect(),
            face_names: b.face_ids().map(|f| b.face(f).name).collect(),
            edge_names: edge_names.clone(),
            sizes: a.sizes.clone(),
            planes: a.planes.clone(),
            sketches: a.sketches.clone(),
            sheet_edges: a
                .sheet_edges
                .iter()
                .map(|s| {
                    let idx = edge_names
                        .iter()
                        .enumerate()
                        .filter(|(_, n)| s.names.contains(n))
                        .map(|(k, _)| k)
                        .collect();
                    (s.text.clone(), idx)
                })
                .collect(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct SheetInfo {
    pub id: String,
    pub bends: usize,
    pub dxf: Result<String, String>,
    /// What forge-lang reports of it (thickness, radius, rule, bends, reliefs, corners, and
    /// whatever later lanes add), as scalar facts and lists of one-line records.
    pub facts: Vec<(String, String)>,
    pub lists: Vec<(String, Vec<String>)>,
    /// Its flat pattern as drawing data, the drawing's part note, and what the flat view draws.
    pub pattern: Option<Arc<forge_sheet::Pattern>>,
    pub note: String,
    pub view: Option<crate::shell::sheet::FlatView>,
}

/// What the flat view draws of a sheet: its contours as polylines, its bends by the feature that
/// made each, its tools' centres, its box.
fn flat_view(s: &forge_lang::sheet::SheetOut, p: &forge_sheet::Pattern) -> crate::shell::sheet::FlatView {
    use crate::shell::sheet::{FlatBend, FlatView};
    use forge_sheet::Curve2;
    let pt = |q: forge_math::Point2| [q.x, q.y];
    let outline: Vec<Vec<[f64; 2]>> = p
        .contours
        .iter()
        .map(|c| {
            let mut line = vec![];
            for cv in c {
                match cv {
                    Curve2::Arc { c, r, start, sweep } => {
                        let n = ((sweep.abs() / 0.15).ceil() as usize).max(2);
                        for i in 0..=n {
                            let a = start + sweep * i as f64 / n as f64;
                            let (sn, cs) = forge_math::dmath::sin_cos(a);
                            line.push([c.x + r * cs, c.y + r * sn]);
                        }
                    }
                    Curve2::Spline { points, .. } => line.extend(points.iter().map(|q| pt(*q))),
                    Curve2::Line { a, b } => {
                        line.push(pt(*a));
                        line.push(pt(*b));
                    }
                }
            }
            line
        })
        .collect();
    let (mut lo, mut hi) = ([f64::INFINITY; 2], [f64::NEG_INFINITY; 2]);
    for q in outline.iter().flatten() {
        lo = [lo[0].min(q[0]), lo[1].min(q[1])];
        hi = [hi[0].max(q[0]), hi[1].max(q[1])];
    }
    if lo[0] > hi[0] {
        (lo, hi) = ([0.0; 2], [0.0; 2]);
    }
    FlatView {
        sheet: s.id.clone(),
        frame: Some(s.sheet.frame),
        outline,
        bends: p
            .bends
            .iter()
            .map(|b| FlatBend {
                a: pt(b.a),
                b: pt(b.b),
                angle: b.angle.to_degrees(),
                radius: b.radius,
                allowance: b.allowance,
                feature: s
                    .joints
                    .iter()
                    .find(|j| j.0 == b.joint)
                    .map_or_else(|| format!("bend {}", b.joint.0), |j| j.1.clone()),
            })
            .collect(),
        tools: p
            .hits
            .iter()
            .map(|h| (pt(h.at), h.tool.clone()))
            .chain(p.forms.iter().map(|f| (pt(f.at), f.tool.clone())))
            .collect(),
        lo,
        hi,
    }
}

/// A JSON value in a line: numbers rounded, records as `k v, ..`.
pub fn json_line(j: &forge_agent::json::Json) -> String {
    use forge_agent::json::Json;
    match j {
        Json::Null => "-".into(),
        Json::Bool(b) => b.to_string(),
        Json::Num(n) => crate::shell::fg::short(*n),
        Json::Str(s) => s.clone(),
        Json::Arr(xs) => xs.iter().map(json_line).collect::<Vec<_>>().join(" "),
        Json::Obj(fs) => fs
            .iter()
            .map(|(k, v)| format!("{k} {}", json_line(v)))
            .collect::<Vec<_>>()
            .join(", "),
    }
}

/// A sheet's report as facts and lists, by its shape alone: new fields show without code here.
/// A report's scalar facts, and its lists of one-line records.
type Report = (Vec<(String, String)>, Vec<(String, Vec<String>)>);

fn sheet_report(j: &forge_agent::json::Json) -> Report {
    use forge_agent::json::Json;
    let (mut facts, mut lists) = (vec![], vec![]);
    if let Json::Obj(fs) = j {
        for (k, v) in fs {
            match v {
                Json::Arr(xs) if xs.iter().any(|x| matches!(x, Json::Obj(_))) || xs.is_empty() => {
                    lists.push((k.clone(), xs.iter().map(json_line).collect()))
                }
                _ if k != "id" => facts.push((k.clone(), json_line(v))),
                _ => {}
            }
        }
    }
    (facts, lists)
}

/// The first sheet's flat solid (several sheets: each one's DXF, the first one shown).
/// Each thermoform of a run as the app shows it: its row and what the view draws of it.
pub fn thermo_views(run: &Run) -> Vec<(crate::shell::thermo::ThermoRow, crate::thermo_view::Gfx)> {
    run.thermos
        .iter()
        .map(|o| {
            let m = forge_thermo::placeholder().material(&o.material).cloned();
            let draft = m.map_or(3.0, |m| {
                let d = if o.tool == forge_thermo::Tool::Male {
                    m.draft_male
                } else {
                    m.draft_female
                };
                d.to_degrees() as f32
            });
            crate::thermo_view::view(o, &run.solids, draft)
        })
        .collect()
}

fn flat_body(run: &Run) -> Option<forge_topo::Body> {
    run.sheets
        .iter()
        .find_map(|s| forge_sheet::build(&s.sheet, forge_sheet::Map::Flat).ok())
}

fn shown(v: f64, d: Dim) -> f64 {
    if d == Dim::Angle { v.to_degrees() } else { v }
}

fn line_of(src: &str, at: u32) -> usize {
    src[..(at as usize).min(src.len())].matches('\n').count() + 1
}

/// The run as the panel's rows.
pub fn rows(run: &Run, src: &str) -> (Vec<FeatRow>, Vec<ParamRow>, Vec<Issue>) {
    let uses = forge_lang::report::uses(run);
    // a component's features after the document's, by their definition ids (`peg.shank`): its
    // browser rows fall under the component's own, and once however many calls it has
    let mut seen = std::collections::HashSet::new();
    let feats = run
        .feats
        .iter()
        .chain(run.components.iter().flat_map(|c| &c.feats))
        .filter(|f| seen.insert(f.id.clone()))
        .map(|f| FeatRow {
            id: f.id.clone(),
            kind: f.kind.to_string(),
            line: line_of(src, f.span.lo),
            status: match f.status {
                forge_lang::eval::Status::Ok => Status::Ok,
                forge_lang::eval::Status::Failed => Status::Failed,
                forge_lang::eval::Status::NotBuilt => Status::NotBuilt,
            },
            dims: f
                .dims
                .iter()
                .map(|d| DimRow {
                    what: d.what.to_string(),
                    value: shown(d.num.v, d.num.dim),
                    dim: d.num.dim,
                    from: match &d.num.src {
                        Src::Const => From::Const,
                        Src::Driving(p) => From::Driving(p.clone()),
                        Src::Driven(ps) => From::Driven(ps.iter().cloned().collect()),
                    },
                })
                .collect(),
            reads: f.reads.iter().cloned().collect(),
            frame: f.frame,
        })
        .collect();
    let mut seen = std::collections::HashSet::new();
    let params = run
        .shared
        .iter()
        .chain(&run.locals)
        .chain(run.components.iter().flat_map(|c| &c.locals))
        .filter(|p| seen.insert(p.key.clone()))
        .map(|p| ParamRow {
            key: p.key.clone(),
            value: match &p.value {
                Value::Num(n) => Some((shown(n.v, n.dim), n.dim)),
                _ => None,
            },
            text: src
                .get(p.rhs.lo as usize..p.rhs.hi as usize)
                .unwrap_or("")
                .to_string(),
            links: p.links.iter().cloned().collect(),
            used_by: uses.get(&p.key).cloned().unwrap_or_default(),
            structural: run.observed.contains(&p.key),
            line: line_of(src, p.span.lo),
        })
        .collect();
    // the feature an issue is in: its own span's, else its statement's
    let feature_at = |span: Option<forge_lang::syntax::Span>| {
        let s = span?;
        let inside = |o: forge_lang::syntax::Span| o.lo <= s.lo && s.hi <= o.hi;
        let comp = run.components.iter().flat_map(|c| &c.feats);
        comp.clone()
            .find(|f| inside(f.span))
            .or_else(|| run.feats.iter().find(|f| inside(f.span)))
            .or_else(|| run.feats.iter().find(|f| inside(f.stmt)))
            .map(|f| f.id.clone())
    };
    let issue = |e: &Error, severity: Severity| {
        let (line, col) = e.span().map_or((0, 0), |s| {
            let (l, c, ..) = s.lines(src);
            (l, c)
        });
        let feature = match e {
            Error::Build { id, .. } => Some(id.clone()),
            _ => feature_at(e.span()),
        };
        Issue {
            severity,
            kind: e.kind().to_string(),
            text: e.text(),
            line,
            col,
            fixes: e.fixes(),
            feature,
            asm: match e {
                Error::Asm(n) => Some(n.issue.clone()),
                _ => None,
            },
        }
    };
    let mut issues: Vec<Issue> = run.errors.iter().map(|e| issue(e, Severity::Error)).collect();
    for w in &run.warnings {
        let (kind, span, text) = match w {
            Warning::Lint(e) => {
                issues.push(issue(e, Severity::Lint));
                continue;
            }
            Warning::Sheet(n) => {
                let (line, col, ..) = n.span.lines(src);
                issues.push(Issue {
                    severity: if n.severity() == "violation" {
                        Severity::Violation
                    } else {
                        Severity::Warning
                    },
                    kind: n.kind().into(),
                    text: n.text(),
                    line,
                    col,
                    fixes: n.fixes.clone(),
                    feature: Some(n.id.clone()),
                    asm: None,
                });
                continue;
            }
            Warning::Thermo(n) => {
                let (line, col, ..) = n.span.lines(src);
                issues.push(Issue {
                    severity: if n.severity() == "violation" {
                        Severity::Violation
                    } else {
                        Severity::Warning
                    },
                    kind: n.kind().into(),
                    text: n.text(),
                    line,
                    col,
                    fixes: n.fixes.clone(),
                    feature: Some(n.id.clone()),
                    asm: None,
                });
                continue;
            }
            Warning::Asm(n) => {
                let (line, col, ..) = n.span.lines(src);
                issues.push(Issue {
                    severity: Severity::Warning,
                    kind: n.kind().into(),
                    text: n.text(),
                    line,
                    col,
                    fixes: n.fixes(),
                    feature: feature_at(Some(n.span)),
                    asm: Some(n.issue.clone()),
                });
                continue;
            }
            Warning::UnderConstrained { span, sketch, dof } => (
                "UnderConstrained",
                *span,
                format!("sketch {sketch} can still move ({dof} degrees of freedom)"),
            ),
            Warning::UnusedParam { span, param } => ("UnusedParam", *span, format!("nothing reads {param}")),
            Warning::NoEffect { span, id } => ("NoEffect", *span, format!("{id} changed nothing")),
        };
        let (line, col, ..) = span.lines(src);
        issues.push(Issue {
            severity: Severity::Warning,
            kind: kind.into(),
            text,
            line,
            col,
            fixes: vec![],
            feature: feature_at(Some(span)),
            asm: None,
        });
    }
    (feats, params, issues)
}

/// What the worker sends: a build (its rows, and a delta of the faces it changed), the faces of
/// the first build for the GPU's live set, or the whole mesh once nothing newer waits.
pub enum Msg {
    Built(Box<Built>),
    Warm(Delta),
    Final {
        seq: u64,
        mesh: PartMesh,
        flat: Option<PartMesh>,
        mesh_ms: f64,
    },
    /// Drives set in the kept solve: each occurrence's pose (the last build's order), or None
    /// when they need a build; and the solve's time.
    Posed {
        seq: u64,
        poses: Option<Vec<forge_math::Frame>>,
        ms: f64,
    },
}

fn whole(body: &forge_topo::Body, flat: Option<&forge_topo::Body>) -> (PartMesh, Option<PartMesh>) {
    let mesh = tess::build_part(body).unwrap_or_else(|e| {
        eprintln!("forge-app: can't mesh it: {e:?}");
        PartMesh::empty()
    });
    let flat = flat.map(|b| tess::build_part(b).unwrap_or_else(|_| PartMesh::empty()));
    (mesh, flat)
}

type Whole = (u64, forge_topo::Body, Option<forge_topo::Body>);

/// The whole meshes, on a thread of their own so a build never waits on one: the latest only.
fn mesh_whole(bodies: Receiver<Whole>, out: Sender<Msg>, waker: Arc<dyn Fn() + Send + Sync>) {
    while let Ok(mut w) = bodies.recv() {
        while let Ok(n) = bodies.try_recv() {
            w = n;
        }
        let t = Instant::now();
        let (mesh, flat) = whole(&w.1, w.2.as_ref());
        let mesh_ms = t.elapsed().as_secs_f64() * 1e3;
        if out
            .send(Msg::Final {
                seq: w.0,
                mesh,
                flat,
                mesh_ms,
            })
            .is_err()
        {
            return;
        }
        waker();
    }
}

/// The worker: owns the stage cache (it is not `Send`: it never leaves this thread) and builds
/// the latest text it was given. A changed part goes to the screen first as the faces that changed
/// (meshed once per face, as a drag's), then whole.
/// Where a part's last screen mesh is kept: its file, text and the meshing code.
fn shown_key(file: &str, src: &str) -> u128 {
    forge_lang::store::key("app-part-mesh", |h| {
        h.str(file);
        h.str(src);
        h.str(env!("FORGE_KERNEL_HASH"));
        h.str(&tess::ladder_key());
    })
}

/// The part's mesh (and flat pattern's) as last shown, if nothing it was built from changed.
fn shown_before(file: &str, src: &str) -> Option<(PartMesh, Option<PartMesh>)> {
    let b = forge_lang::kept::fetch("app-part-mesh", shown_key(file, src))?;
    let mut i = forge_lang::store::In(&b);
    let mesh = PartMesh::from_bytes(i.bytes()?)?;
    let flat = if i.bool()? {
        Some(PartMesh::from_bytes(i.bytes()?)?)
    } else {
        None
    };
    Some((mesh, flat))
}

fn keep_shown(
    file: &str,
    src: &str,
    from: &forge_lang::deps::Deps,
    mesh: &PartMesh,
    flat: Option<&PartMesh>,
) {
    let mut o = forge_lang::store::Out::default();
    o.bytes(&mesh.to_bytes());
    o.bool(flat.is_some());
    if let Some(f) = flat {
        o.bytes(&f.to_bytes());
    }
    forge_lang::kept::keep("app-part-mesh", shown_key(file, src), from, &o.0);
}

/// Every document's build thread, by the thread that opened it (the app's), with its stop flag:
/// a closed document's finishes the build it's in, then ends; on exit each is stopped and waited
/// for before the GPU device goes.
type Builder = (
    std::thread::ThreadId,
    Arc<AtomicBool>,
    std::thread::JoinHandle<()>,
);
static BUILDERS: std::sync::Mutex<Vec<Builder>> = std::sync::Mutex::new(Vec::new());

/// Stops the build threads this thread's documents started, and waits for each.
pub fn stop_builders() {
    let me = std::thread::current().id();
    let mine: Vec<Builder> = {
        let mut b = BUILDERS.lock().unwrap_or_else(|e| e.into_inner());
        let (mine, rest) = std::mem::take(&mut *b).into_iter().partition(|x| x.0 == me);
        *b = rest;
        mine
    };
    for (_, stop, _) in &mine {
        stop.store(true, Ordering::SeqCst);
    }
    for (_, _, h) in mine {
        let _ = h.join();
    }
}

/// This thread's documents' build threads still running.
pub fn builders_running() -> usize {
    let me = std::thread::current().id();
    let mut b = BUILDERS.lock().unwrap_or_else(|e| e.into_inner());
    b.retain(|x| !x.2.is_finished());
    b.iter().filter(|x| x.0 == me).count()
}

/// What a build read, for the host's watch; a build that read nothing (all of it warm) leaves
/// the last list. Only what forge-lang's loaders let through and read (or found missing):
/// never a path they refused.
fn note_deps(deps: &Reads, read: &forge_lang::deps::Deps) {
    let files: Vec<std::path::PathBuf> = read
        .0
        .iter()
        .filter(|(_, seen)| !matches!(seen, forge_lang::deps::Seen::There))
        .map(|(p, _)| p.clone())
        .collect();
    if !files.is_empty() {
        *deps.lock().unwrap_or_else(|e| e.into_inner()) = files;
    }
}

/// The files a session's last build read: its parts, subassemblies, libraries, fonts, imports.
pub type Reads = Arc<std::sync::Mutex<Vec<std::path::PathBuf>>>;

fn work(
    dir: Option<std::path::PathBuf>,
    file: Option<String>,
    deps: Reads,
    // the texts to build, and a new GPU picture when one's asked for
    (jobs, gpus): (Receiver<Work>, Receiver<Gpu>),
    out: Sender<Msg>,
    mut gpu: Option<Gpu>,
    // the host's: woken when a result is in, and told to stop when the document closes
    (waker, stop): (Arc<dyn Fn() + Send + Sync>, Arc<AtomicBool>),
) {
    let mut cache = Cache::default();
    let mut mesher = Mesher::default();
    // an assembly's meshes by part content, and the placements' meshes last shown
    let mut asm_meshes = AsmMeshes::default();
    let mut asm_shape: Option<Vec<usize>> = None;
    let mut shown_ids: Option<(Vec<String>, u64)> = None;
    // the text the last anchors were found for: unchanged text, unchanged anchors
    let mut anchored: Option<String> = None;
    // the body on screen, for Measure
    let mut last_body: Option<Arc<forge_topo::Body>> = None;
    let (to_mesh, bodies) = channel::<Whole>();
    let mesher_thread = {
        let (out, waker) = (out.clone(), waker.clone());
        std::thread::Builder::new()
            .name("forge-fg-mesh".into())
            .spawn(move || mesh_whole(bodies, out, waker))
            .expect("a thread")
    };
    // stopped (the document closed, the app going): no more GPU work, the mesher waited for
    let stopped = |to_mesh: Sender<Whole>, t: std::thread::JoinHandle<()>| {
        drop(to_mesh);
        let _ = t.join();
    };
    let send = |m: Msg| {
        let ok = out.send(m).is_ok();
        waker();
        ok
    };
    // a drive sent after a text, kept till that text is built
    let mut next: Option<Work> = None;
    loop {
        let first = match next.take() {
            Some(w) => w,
            // waiting, a stop is seen within a tenth of a second
            None => loop {
                if stop.load(Ordering::SeqCst) {
                    stopped(to_mesh, mesher_thread);
                    return;
                }
                match jobs.recv_timeout(std::time::Duration::from_millis(100)) {
                    Ok(w) => break w,
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
                    Err(_) => {
                        stopped(to_mesh, mesher_thread);
                        return;
                    }
                }
            },
        };
        let mut all = vec![first];
        all.extend(jobs.try_iter());
        // only the latest text matters, and the latest drive after it
        let text = all.iter().rposition(|w| matches!(w, Work::Text(_)));
        let drive = all
            .iter()
            .rposition(|w| matches!(w, Work::Drive(..)))
            .filter(|&d| text.is_none_or(|t| d > t));
        if let Some(d) = drive {
            if text.is_some() {
                next = Some(all[d].clone());
            } else if let Work::Drive(seq, drives) = &all[d] {
                let t = Instant::now();
                let poses = cache.asm.as_mut().and_then(|a| a.drive(drives));
                let ms = t.elapsed().as_secs_f64() * 1e3;
                if !send(Msg::Posed { seq: *seq, poses, ms }) {
                    return;
                }
                continue;
            }
        }
        // a new view asked for: everything goes to it whole, as at first
        if let Ok(g) = gpus.try_recv() {
            gpu = Some(g);
            asm_shape = None;
            shown_ids = None;
        }
        let Some(Work::Text(job)) = text.map(|t| all.swap_remove(t)) else {
            continue;
        };
        let t = Instant::now();
        if is_assembly(&job.1) {
            let opts = Options {
                dir: dir.clone(),
                render: true,
                // the overlap check and the expects are the slow part of a big assembly: on request
                overlap: job.2,
                expects: job.2,
                ..Options::default()
            };
            let watch = forge_lang::deps::watch();
            let (run, c) = forge_lang::run(&job.1, opts, std::mem::take(&mut cache));
            note_deps(&deps, &watch.done());
            cache = c;
            if stop.load(Ordering::SeqCst) {
                stopped(to_mesh, mesher_thread);
                return;
            }
            let build_ms = t.elapsed().as_secs_f64() * 1e3;
            let t = Instant::now();
            let (asm, view) = asm_view(&run, &mut asm_meshes, &mut gpu, &mut asm_shape);
            let mesh_ms = t.elapsed().as_secs_f64() * 1e3;
            let (feats, params, issues) = rows(&run, &job.1);
            let built = Built {
                seq: job.0,
                ok: run.ok(),
                feats,
                params,
                issues,
                measure: run.measure.clone(),
                timings: run.timings.clone(),
                rebuilt: run.rebuilt,
                reused: run.reused,
                mesh: None,
                flat: None,
                delta: None,
                sheets: vec![],
                thermos: vec![],
                view,
                anchors: None,
                inspect: asm.as_ref().map(|a| a.inspect.clone()),
                preview: false,
                checked: true,
                asm,
                build_ms,
                mesh_ms,
            };
            if !send(Msg::Built(Box::new(built))) {
                return;
            }
            continue;
        }
        // opening: the mesh last shown of this very file and all it read, at once
        let place = file
            .as_deref()
            .and_then(|f| std::fs::canonicalize(f).ok())
            .map(|p| p.to_string_lossy().to_string());
        let opening = gpu.is_some();
        let before = if opening {
            place.as_deref().and_then(|p| shown_before(p, &job.1))
        } else {
            None
        };
        // opening with nothing kept: the part as built without its checks first (a thermoform's
        // forming, the measures), on screen while they run
        let before = before.or_else(|| {
            if !opening {
                return None;
            }
            let quick = Options {
                checks: false,
                dir: dir.clone(),
                ..Options::default()
            };
            let (shape, c) = forge_lang::run(&job.1, quick, std::mem::take(&mut cache));
            cache = c;
            let b = shape.body.as_ref()?;
            Some(whole(b, flat_body(&shape).as_ref()))
        });
        if opening && let Some((mesh, flat)) = before {
            let quick = Options {
                build: false,
                dir: dir.clone(),
                ..Options::default()
            };
            let (checked, _) = forge_lang::run(&job.1, quick, Default::default());
            let (feats, params, issues) = rows(&checked, &job.1);
            let view = gpu.take().map(|(device, queue, adapter)| {
                let parts = [mesh.clone()];
                let mut scene = Scene::new(&part_info(&parts), vec![Placement::at(0, [0.0; 3])]);
                scene.set_explode(0.0);
                let r = Renderer::new(
                    device,
                    queue,
                    adapter,
                    render::OFFSCREEN,
                    (16, 16),
                    &parts,
                    &scene,
                );
                (r, scene)
            });
            let shown = Built {
                seq: job.0,
                ok: true,
                feats,
                params,
                issues,
                measure: None,
                timings: checked.timings.clone(),
                rebuilt: 0,
                reused: 0,
                mesh: Some(mesh),
                flat,
                delta: None,
                sheets: vec![],
                thermos: vec![],
                view,
                anchors: None,
                asm: None,
                build_ms: t.elapsed().as_secs_f64() * 1e3,
                mesh_ms: 0.0,
                inspect: None,
                preview: true,
                checked: false,
            };
            if !send(Msg::Built(Box::new(shown))) {
                return;
            }
        }
        // the shape first, on screen as soon as it is built; then the checks (measures, expects,
        // a thermoform's walls) with every stage reused, unless a newer text is waiting
        let mut checks = false;
        // what the shape's build rebuilt, which the checks' build (all reused) reports for it
        let mut shaped: Option<(usize, usize)> = None;
        loop {
            let t = if checks { Instant::now() } else { t };
            let opening = opening && !checks;
            let want = anchored.as_deref() != Some(job.1.as_str());
            let opts = Options {
                anchors: want,
                dir: dir.clone(),
                checks,
                ..Options::default()
            };
            let watch = forge_lang::deps::watch();
            let (run, c) = forge_lang::run(&job.1, opts, std::mem::take(&mut cache));
            let all = watch.done();
            note_deps(&deps, &all);
            let read = opening.then_some(all);
            let anchors = run
                .anchors
                .as_ref()
                .map(|a| Arc::new(AnchorView::of(a, run.body.as_ref())));
            if anchors.is_some() {
                anchored = Some(job.1.clone());
            }
            cache = c;
            if stop.load(Ordering::SeqCst) {
                stopped(to_mesh, mesher_thread);
                return;
            }
            let build_ms = t.elapsed().as_secs_f64() * 1e3;
            let t = Instant::now();
            let ids: Vec<String> = run.feats.iter().map(|f| f.id.clone()).collect();
            // the very stage on screen, and the same features: nothing to show again (a kept stage
            // served for an edit taken back is another stage than the one shown)
            let now = (ids, run.stage);
            let same = run.stage != 0 && shown_ids.as_ref() == Some(&now);
            let changed = run.body.as_ref().filter(|_| !same);
            if changed.is_some() {
                shown_ids = Some(now);
            }
            if let Some(b) = changed {
                last_body = Some(Arc::new(b.clone()));
            }
            // the first build comes whole (after a preview too, which it replaces)
            let first = opening;
            let (mut mesh, mut flat, mut delta) = (None, None, None);
            match changed {
                Some(b) if first => {
                    let (m, f) = whole(b, flat_body(&run).as_ref());
                    if let (Some(place), Some(read)) = (&place, &read) {
                        keep_shown(place, &job.1, read, &m, f.as_ref());
                    }
                    (mesh, flat) = (Some(m), f);
                }
                Some(b) => delta = Some(mesher.delta(b)),
                None => {}
            }
            let sheets = run
                .sheets
                .iter()
                .map(|s| {
                    let (facts, lists) = sheet_report(&s.json());
                    let pattern = s.pattern().map(Arc::new);
                    let note = s.note();
                    SheetInfo {
                        id: s.id.clone(),
                        bends: s.layout.as_ref().map_or(0, |l| l.bends.len()),
                        dxf: pattern
                            .as_ref()
                            .map(|p| forge_sheet::dxf_noted(p, &note))
                            .map_err(|e| forge_lang::error::words(&e)),
                        view: pattern.as_ref().ok().map(|p| flat_view(s, p)),
                        pattern: pattern.ok(),
                        note,
                        facts,
                        lists,
                    }
                })
                .collect();
            let mesh_ms = t.elapsed().as_secs_f64() * 1e3;
            let view = gpu.take().map(|(device, queue, adapter)| {
                let parts = [mesh.clone().unwrap_or_else(PartMesh::empty)];
                let mut scene = Scene::new(&part_info(&parts), vec![Placement::at(0, [0.0; 3])]);
                scene.set_explode(0.0);
                let r = Renderer::new(
                    device,
                    queue,
                    adapter,
                    render::OFFSCREEN,
                    (16, 16),
                    &parts,
                    &scene,
                );
                (r, scene)
            });
            let (feats, params, issues) = rows(&run, &job.1);
            let built = Built {
                seq: job.0,
                ok: run.ok(),
                feats,
                params,
                issues,
                measure: run.measure.clone(),
                timings: run.timings.clone(),
                rebuilt: shaped.map_or(run.rebuilt, |x| x.0),
                reused: shaped.map_or(run.reused, |x| x.1),
                mesh,
                flat,
                delta,
                sheets,
                thermos: thermo_views(&run),
                view,
                anchors,
                asm: None,
                build_ms,
                mesh_ms,
                inspect: Some(Arc::new(crate::gui::gui_inspect::InspectData {
                    bodies: last_body.iter().cloned().collect(),
                    placed: vec![(0, forge_math::Frame::WORLD)],
                    names: vec![],
                    props: run.props.clone(),
                    physical: run.physical.clone(),
                    asm: None,
                })),
                preview: false,
                checked: checks,
            };
            if !send(Msg::Built(Box::new(built))) {
                return;
            }
            if let Some(b) = changed {
                if first {
                    // the GPU's live set starts as the whole part, so later deltas carry only changes
                    if !send(Msg::Warm(mesher.delta(b))) {
                        return;
                    }
                } else if to_mesh.send((job.0, b.clone(), flat_body(&run))).is_err() {
                    return;
                }
            }
            if checks {
                break;
            }
            // something newer sent (a text, a drive): it goes first, the checks wait for it
            if let Ok(w) = jobs.try_recv() {
                next = Some(w);
                break;
            }
            checks = true;
            shaped = Some((run.rebuilt, run.reused));
        }
    }
}

// plain text, so the shell has it for the browser too
pub use crate::shell::fg::is_assembly;

/// An assembly's part meshes so far: one per distinct body (by content), in the order made.
#[derive(Default)]
pub struct AsmMeshes {
    by_key: HashMap<u64, usize>,
    pub all: Vec<PartMesh>,
    /// Each mesh's exact body.
    pub bodies: Vec<Arc<forge_topo::Body>>,
    /// Each mesh's box in its part's space: what the exploded view keeps apart.
    pub boxes: Vec<crate::explode::Aabb>,
}

/// A body's content, cheaply: every vertex's place and every face's name.
fn body_key(b: &forge_topo::Body) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    for f in b.face_ids() {
        b.face(f).name.hash(&mut h);
    }
    for v in &b.vertices {
        v.point.x.to_bits().hash(&mut h);
        v.point.y.to_bits().hash(&mut h);
        v.point.z.to_bits().hash(&mut h);
    }
    h.finish()
}

/// The assembly's view: each distinct part meshed once (new ones on threads), each occurrence
/// a placement of its part's mesh. A new renderer and scene when which meshes are placed where
/// changed; else only poses move.
fn asm_view(
    run: &Run,
    meshes: &mut AsmMeshes,
    gpu: &mut Option<Gpu>,
    shape: &mut Option<Vec<usize>>,
) -> (Option<AsmView>, Option<(Renderer, Scene)>) {
    let Some(out) = &run.asm else {
        // a failed assembly: the view stays as it was, its errors in the panel
        return (None, None);
    };
    // each distinct body: an index into the meshes, meshing the new ones (the parts', then the
    // moulds of the thermoformed ones)
    let mut bodies: Vec<Arc<forge_topo::Body>> = out.parts.clone();
    let tool_of: Vec<Option<usize>> = out
        .occ_tooling
        .iter()
        .map(|t| {
            t.as_ref()
                .map(|b| match bodies.iter().position(|x| Arc::ptr_eq(x, b)) {
                    Some(k) => k,
                    None => {
                        bodies.push(b.clone());
                        bodies.len() - 1
                    }
                })
        })
        .collect();
    let keys: Vec<u64> = bodies.iter().map(|b| body_key(b)).collect();
    let mut fresh: Vec<(u64, usize)> = vec![];
    for (k, key) in keys.iter().enumerate() {
        if !meshes.by_key.contains_key(key) && !fresh.iter().any(|f| f.0 == *key) {
            fresh.push((*key, k));
        }
    }
    let made: Vec<PartMesh> = {
        let at = std::sync::atomic::AtomicUsize::new(0);
        let mut got: Vec<(usize, PartMesh)> = std::thread::scope(|s| {
            let workers: Vec<_> = (0..4)
                .map(|_| {
                    s.spawn(|| {
                        let mut got = vec![];
                        loop {
                            let i = at.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            let Some(&(_, k)) = fresh.get(i) else { return got };
                            let m = tess::build_part_coarse(&bodies[k]).unwrap_or_else(|_| PartMesh::empty());
                            got.push((i, m));
                        }
                    })
                })
                .collect();
            workers
                .into_iter()
                .flat_map(|w| w.join().expect("a mesher panicked"))
                .collect()
        });
        got.sort_by_key(|x| x.0);
        got.into_iter().map(|x| x.1).collect()
    };
    let mut new_meshes = vec![];
    for ((key, k), m) in fresh.iter().zip(made) {
        meshes.bodies.push(bodies[*k].clone());
        meshes.by_key.insert(*key, meshes.all.len());
        new_meshes.push((meshes.all.len(), m.clone()));
        meshes.boxes.push(crate::explode::mesh_box(&m));
        meshes.all.push(m);
    }
    let mut occ = vec![];
    let mut placed = vec![];
    let mut exact = vec![];
    for (i, ((path, file, pose), part)) in out.occurrences.iter().zip(&out.occ_part).enumerate() {
        let placement = part.map(|k| {
            exact.push((meshes.by_key[&keys[k]], *pose, path.clone()));
            placed.push((meshes.by_key[&keys[k]], rows_of(pose)));
            placed.len() - 1
        });
        let tooling = tool_of.get(i).copied().flatten().map(|k| {
            placed.push((meshes.by_key[&keys[k]], rows_of(pose)));
            placed.len() - 1
        });
        occ.push(Occ {
            path: path.clone(),
            file: file.clone(),
            placement,
            tooling,
        });
    }
    let now: Vec<usize> = placed.iter().map(|p| p.0).collect();
    let restage = shape.as_ref() != Some(&now) || gpu.is_some();
    let all = (restage && gpu.is_none()).then(|| meshes.all.clone());
    let view = if restage && let Some((device, queue, adapter)) = gpu.take() {
        *shape = Some(now);
        let parts: Vec<PartMesh> = if meshes.all.is_empty() {
            vec![PartMesh::empty()]
        } else {
            meshes.all.clone()
        };
        let placements: Vec<Placement> = if placed.is_empty() {
            vec![Placement::at(0, [0.0; 3])]
        } else {
            placed
                .iter()
                .map(|(m, rows)| Placement {
                    part: *m as u32,
                    rows: *rows,
                    tint: PART_TINT,
                })
                .collect()
        };
        let mut scene = Scene::new(&part_info(&parts), placements);
        scene.set_explode(0.0);
        scene.set_part_boxes(meshes.boxes.clone());
        let r = Renderer::new(
            device,
            queue,
            adapter,
            render::OFFSCREEN,
            (16, 16),
            &parts,
            &scene,
        );
        Some((r, scene))
    } else {
        if restage {
            *shape = Some(now);
        }
        None
    };
    let parts = meshes.all.len();
    let meshed = new_meshes.len();
    let asm = AsmView {
        body_chains: run.body_chains.clone(),
        occ,
        placed,
        meshes: new_meshes,
        restage: all,
        joints: out.joints.clone(),
        mobility: out.mobility,
        bom: out.bom.clone(),
        expects: out.expects.clone(),
        overlap: out.overlap.as_ref().map(|_| (out.interferences, out.intended)),
        timings: out.timings.clone(),
        parts,
        meshed,
        boxes: meshes.boxes.clone(),
        grounded: out.grounded.clone(),
        insts: out.insts.clone(),
        links: out.links.clone(),
        link_at: out.link_at.clone(),
        poses: out.poses.clone(),
        thermo_files: out
            .part_docs
            .iter()
            .filter(|d| !d.1.thermos.is_empty())
            .map(|d| d.0.clone())
            .collect(),
        inspect: Arc::new(crate::gui::gui_inspect::InspectData {
            bodies: meshes.bodies.clone(),
            placed: exact.iter().map(|e| (e.0, e.1)).collect(),
            names: exact.into_iter().map(|e| e.2).collect(),
            props: run.props.clone(),
            physical: None,
            asm: Some(Arc::new(forge_lang::asm::AsmOut {
                occurrences: out.occurrences.clone(),
                occ_body: out.occ_body.clone(),
                part_docs: out.part_docs.clone(),
                ..Default::default()
            })),
        }),
    };
    (Some(asm), view)
}

/// A part's colour in an assembly, and a selected one's.
pub const PART_TINT: [f32; 4] = crate::palette::GREY;
pub const PICKED_TINT: [f32; 4] = [0.95, 0.55, 0.15, 0.0];
pub const CLASH_TINT: [f32; 4] = [0.90, 0.20, 0.20, 0.0];
/// A mould's colour among the parts: tooling.
pub const TOOL_TINT: [f32; 4] = [0.55, 0.58, 0.62, 0.0];

/// A .fg part open in the app.
pub struct Session {
    pub path: Option<String>,
    pub src: String,
    /// What the file holds.
    pub saved: String,
    undo: Vec<String>,
    redo: Vec<String>,
    /// Steps made from outside (the Agent tab): the undo depth each made, its label and key.
    tags: Vec<(usize, String, u64)>,
    jobs: Sender<Work>,
    /// A new GPU picture asked for: the old one was let go to save memory.
    gpus: Sender<Gpu>,
    /// What its last build read of the disk.
    pub deps: Reads,
    built: Receiver<Msg>,
    /// The last text sent to the worker, and the last one back.
    pub seq: u64,
    pub done: u64,
    sent_at: Vec<(u64, Instant)>,
    /// From sending a text to its build being back, the last time.
    pub round_ms: f64,
    /// The ops applied, in order.
    pub ops: Vec<Op>,
    /// An assembly runs its overlap check (asked for once, kept: after a drag it re-tests only
    /// what moved).
    pub overlap: bool,
    /// The last build's times: forge-lang's (its stages), and meshing for the screen.
    pub timings: Timings,
    pub build_ms: f64,
    pub mesh_ms: f64,
    /// The last build that changed the part's shape.
    pub changed: u64,
    /// The last build whose whole mesh came, and that mesh's time.
    pub whole: u64,
    pub whole_ms: f64,
    /// What's built is a dialog's preview, not the file.
    pub previewing: bool,
    /// Tells its build thread to stop: set when the document closes.
    stop: Arc<AtomicBool>,
}

impl Drop for Session {
    fn drop(&mut self) {
        // its thread finishes the build it's in, does no GPU work after, and ends
        self.stop.store(true, Ordering::SeqCst);
    }
}

impl Session {
    /// A session on this text; the first build starts at once.
    pub fn new(
        path: Option<String>,
        src: String,
        gpu: Option<Gpu>,
        waker: Arc<dyn Fn() + Send + Sync>,
    ) -> Session {
        let (jobs, rx) = channel();
        let (gpus, gpu_rx) = channel();
        let deps = Reads::default();
        let (tx, built) = channel();
        // fonts and imported files are found beside the part
        let dir = path
            .as_deref()
            .and_then(|p| std::path::Path::new(p).parent().map(|d| d.to_path_buf()));
        let stop = Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        let h = std::thread::Builder::new()
            .name("forge-fg".into())
            .spawn({
                let (file, deps) = (path.clone(), deps.clone());
                move || work(dir, file, deps, (rx, gpu_rx), tx, gpu, (waker, flag))
            })
            .expect("a thread");
        {
            let mut b = BUILDERS.lock().unwrap_or_else(|e| e.into_inner());
            b.retain(|x| !x.2.is_finished());
            b.push((std::thread::current().id(), stop.clone(), h));
        }
        let mut s = Session {
            path,
            saved: src.clone(),
            src,
            undo: vec![],
            redo: vec![],
            tags: vec![],
            jobs,
            gpus,
            deps,
            built,
            seq: 0,
            done: 0,
            sent_at: vec![],
            round_ms: 0.0,
            ops: vec![],
            overlap: false,
            timings: Timings::default(),
            build_ms: 0.0,
            mesh_ms: 0.0,
            whole: 0,
            whole_ms: 0.0,
            changed: 0,
            previewing: false,
            stop,
        };
        s.rebuild();
        s
    }

    pub fn open(path: &str, gpu: Option<Gpu>, waker: Arc<dyn Fn() + Send + Sync>) -> Result<Session, String> {
        let src = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
        Ok(Session::new(Some(path.to_string()), src, gpu, waker))
    }

    /// Its view made again from scratch, on this GPU: after it was let go while the tab was idle.
    pub fn regpu(&mut self, gpu: Gpu) -> u64 {
        let _ = self.gpus.send(gpu);
        self.rebuild()
    }

    /// An assembly's overlap check, from now on.
    pub fn check_overlap(&mut self) -> u64 {
        self.overlap = true;
        self.rebuild()
    }

    /// Builds the text as it stands (through the warm stages).
    pub fn rebuild(&mut self) -> u64 {
        self.seq += 1;
        self.sent_at.push((self.seq, Instant::now()));
        let _ = self
            .jobs
            .send(Work::Text((self.seq, self.src.clone(), self.overlap)));
        self.seq
    }

    /// Joint values set straight in the assembly's kept solve (a drag's): the poses come back as
    /// `Msg::Posed`, the text and what's built from it unchanged.
    pub fn drive(&mut self, drives: Vec<Drive>) -> u64 {
        self.seq += 1;
        self.sent_at.push((self.seq, Instant::now()));
        // what's on screen is no longer the file's
        self.previewing = true;
        let _ = self.jobs.send(Work::Drive(self.seq, drives));
        self.seq
    }

    /// What the worker has finished.
    pub fn poll(&mut self) -> Vec<Msg> {
        let got: Vec<Msg> = self.built.try_iter().collect();
        for m in &got {
            match m {
                Msg::Built(b) => {
                    // a preview is on screen, its build still to come
                    // done once checked: the shape alone, or a preview, has its checks to come
                    if !b.preview && b.checked {
                        self.done = b.seq;
                    }
                    if b.delta.is_some() || b.mesh.is_some() {
                        self.changed = b.seq;
                    }
                    // the first build comes whole
                    if b.mesh.is_some() {
                        self.whole = b.seq;
                    }
                    self.timings = b.timings.clone();
                    (self.build_ms, self.mesh_ms) = (b.build_ms, b.mesh_ms);
                    if let Some(&(_, t)) = self.sent_at.iter().find(|s| s.0 == b.seq) {
                        self.round_ms = t.elapsed().as_secs_f64() * 1e3;
                    }
                    self.sent_at.retain(|s| s.0 > b.seq);
                }
                Msg::Final { seq, mesh_ms, .. } => {
                    self.whole = *seq;
                    self.whole_ms = *mesh_ms;
                }
                Msg::Posed { seq, ms, .. } => {
                    self.done = *seq;
                    self.build_ms = *ms;
                    if let Some(&(_, t)) = self.sent_at.iter().find(|s| s.0 == *seq) {
                        self.round_ms = t.elapsed().as_secs_f64() * 1e3;
                    }
                    self.sent_at.retain(|s| s.0 > *seq);
                }
                Msg::Warm(_) => {}
            }
        }
        got
    }

    /// A refactor between this file and others (Make / Extract / Inline Component): all or
    /// nothing, the files it makes written beside this one, one undo step for this file's text.
    pub fn refactor(&mut self, r: &forge_lang::refactor::Refactor) -> Result<Vec<String>, Error> {
        let dir = self
            .path
            .as_deref()
            .and_then(|f| std::fs::canonicalize(f).ok())
            .and_then(|f| f.parent().map(|d| d.to_path_buf()))
            .unwrap_or_default();
        let read = |f: &str| std::fs::read_to_string(dir.join(f)).ok();
        let w = forge_lang::refactor::apply_checked(&self.src, r, &read)?;
        for (f, t) in &w.files {
            std::fs::write(dir.join(f), t).map_err(|e| Error::BadValue {
                span: forge_lang::syntax::Span::default(),
                what: "a file written",
                value: e.raw_os_error().unwrap_or(0) as f64,
            })?;
        }
        self.set_text(w.doc);
        Ok(w.files.into_iter().map(|f| f.0).collect())
    }

    /// Applies a typed op through forge-lang: refused, with its typed error, if the result
    /// wouldn't check; else the new text builds.
    pub fn apply(&mut self, op: Op) -> Result<Option<Reach>, Error> {
        let dir = self
            .path
            .as_deref()
            .and_then(|f| std::fs::canonicalize(f).ok())
            .and_then(|f| f.parent().map(|d| d.to_path_buf()));
        let (text, reach) = edit::apply_checked_in(&self.src, &op, dir)?;
        self.set_text(text);
        self.ops.push(op);
        Ok(reach)
    }

    /// Several ops as one edit, one undo step: refused whole if the result wouldn't check.
    pub fn apply_all(&mut self, ops: Vec<Op>) -> Result<(), Error> {
        let text = edit::apply_all_checked(&self.src, &ops)?;
        self.set_text(text);
        self.ops.extend(ops);
        Ok(())
    }

    /// A dialog's preview: these ops' text built in the file's place, the file's text kept (an
    /// op that doesn't apply previews the file as it is); None builds the file's own again.
    pub fn preview(&mut self, ops: Option<&[Op]>) -> u64 {
        let text = ops.and_then(|o| edit::apply_all_checked(&self.src, o).ok());
        self.previewing = text.is_some();
        self.seq += 1;
        self.sent_at.push((self.seq, Instant::now()));
        let _ = self.jobs.send(Work::Text((
            self.seq,
            text.unwrap_or_else(|| self.src.clone()),
            self.overlap,
        )));
        self.seq
    }

    fn set_text(&mut self, text: String) {
        self.previewing = false;
        let old = std::mem::replace(&mut self.src, text);
        self.undo.push(old);
        self.redo.clear();
        self.tags.retain(|t| t.0 < self.undo.len());
        self.rebuild();
    }

    /// Text from outside the app's own edits, as one labelled undo step: a later text under the
    /// same key, with nothing done between, joins that step. `on_disk`: the file holds it.
    pub fn external(&mut self, text: String, label: &str, key: u64, on_disk: bool) {
        if on_disk {
            self.saved = text.clone();
        }
        if text == self.src {
            return;
        }
        let joins = self
            .tags
            .last()
            .is_some_and(|t| t.2 == key && t.0 == self.undo.len() && self.redo.is_empty());
        if joins {
            self.previewing = false;
            self.src = text;
            self.rebuild();
        } else {
            self.set_text(text);
            self.tags.push((self.undo.len(), label.to_string(), key));
        }
    }

    /// The label and key of the step Undo would take back, if it came from outside.
    pub fn undo_tag(&self) -> Option<(&str, u64)> {
        let t = self.tags.iter().rev().find(|t| t.0 == self.undo.len())?;
        Some((&t.1, t.2))
    }

    pub fn undo(&mut self) -> bool {
        let Some(prev) = self.undo.pop() else { return false };
        self.redo.push(std::mem::replace(&mut self.src, prev));
        self.rebuild();
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(next) = self.redo.pop() else { return false };
        self.undo.push(std::mem::replace(&mut self.src, next));
        self.rebuild();
        true
    }

    pub fn dirty(&self) -> bool {
        self.src != self.saved
    }

    /// The file's text as it is on disk now, read again: an undo step back to what was here.
    pub fn reload(&mut self, text: String) {
        self.saved = text.clone();
        self.set_text(text);
    }

    pub fn save(&mut self) -> Result<(), String> {
        let path = self.path.as_ref().ok_or("no file to save to")?;
        std::fs::write(path, &self.src).map_err(|e| format!("{path}: {e}"))?;
        self.saved = self.src.clone();
        Ok(())
    }
}
