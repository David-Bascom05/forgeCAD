//! The evaluator: statements in order, each feature built as it is made. Ids come from the
//! assigned name, the call path and loop indices only. Params are scoped to their feature;
//! shared params are seen everywhere. Every number carries where it came from.

use crate::build::{Builder, Cache, history_id};
use crate::error::{EntKind, Error, Literal, ParamKey, Rule, Warning, path_text};
use crate::sketch::Sk;
use crate::syntax::{self, Arg, BinOp, BlockKind, Expr, ExprKind, Key, Seg, Span, Stmt, StmtKind, Unit};
use crate::values::{self, Dim, Num, Src, Ty};
use forge_math::{Frame, Point3, UnitVec3, Vec3};
use forge_sketch::EntityId;
use forge_topo::Name;
use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap};
use std::rc::Rc;

/// How a rule is enforced: not at all, as a typed lint (a warning with fixes, never blocking),
/// or as a typed error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    Off,
    Lint,
    Error,
}

/// Which rules hold, each its own switch. The hard ones are what editability needs (proven in
/// research/forge-lang); the lints are style and intent.
#[derive(Clone, Copy, Debug)]
pub struct Rules {
    pub literals_in_params: Level,
    pub scoped_params: Level,
    pub world_only_first: Level,
    pub no_recursion: Level,
    pub named_features: Level,
    pub keyed_ids: Level,
    pub no_positional_selection: Level,
    pub single_assignment: Level,
    pub expect_copied: Level,
}

impl Default for Rules {
    fn default() -> Rules {
        Rules {
            literals_in_params: Level::Lint,
            scoped_params: Level::Lint,
            world_only_first: Level::Lint,
            no_recursion: Level::Off,
            named_features: Level::Error,
            keyed_ids: Level::Error,
            no_positional_selection: Level::Error,
            single_assignment: Level::Error,
            expect_copied: Level::Lint,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Plane {
    pub frame: Frame,
    /// A world plane (`xy`, `xz`, `yz`): only the first feature may use one.
    pub world: Option<&'static str>,
}

#[derive(Clone, Debug)]
pub struct Faces {
    pub names: Vec<Name>,
    /// The feature they belong to and the role, for errors.
    pub owner: String,
    pub role: crate::error::Role,
    /// Their plane, z out of the part, x and origin carried from the owner's sketch.
    pub plane: Option<Frame>,
}

/// What an extrude-like feature's anchors are computed from.
#[derive(Clone, Debug)]
pub struct Shape {
    pub sketch: Rc<RefCell<Sk>>,
    /// Along the sketch normal, where the far end lies (None: up to a face, or through).
    pub far: Option<f64>,
    pub near: f64,
    pub cut: bool,
    pub segs: Vec<EntityId>,
}

#[derive(Clone, Debug)]
pub struct Feat {
    pub id: String,
    pub kind: &'static str,
    pub idx: usize,
    pub hid: u64,
    pub shape: Option<Shape>,
    /// A revolve's or hole's axis.
    pub axis: Option<(Point3, UnitVec3)>,
    /// The sketch an extrude or revolve made its faces from (`side(entity)` names them).
    pub prof: Option<Rc<RefCell<Sk>>>,
}

/// A sketch entity: the sketch and the entity (a point, a curve) or a loop of curves.
#[derive(Clone)]
pub struct Ent {
    pub sk: Rc<RefCell<Sk>>,
    pub id: EntityId,
    pub kind: EntKind,
    pub loop_segs: Rc<Vec<EntityId>>,
}

impl std::fmt::Debug for Ent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Ent({:?} {:?})", self.kind, self.id)
    }
}

#[derive(Clone, Debug)]
pub enum ExtentV {
    Sym(f64),
    Two(f64, f64),
    Thru,
    ThruAll,
    Next,
    UpTo(Rc<Faces>, f64),
    /// Up to a work plane (its place in the history), then an offset.
    UpToPlane(usize, f64),
}

#[derive(Clone, Debug)]
pub struct FnDef {
    pub name: String,
    pub params: Vec<syntax::FnParam>,
    pub body: Rc<Vec<Stmt>>,
    pub span: Span,
    /// A function of a used file (`uses.rs`): its body sees that file's names, not the caller's.
    pub lib: Option<std::rc::Weak<crate::uses::Lib>>,
}

#[derive(Clone, Debug)]
pub enum Value {
    Num(Num),
    Bool(bool),
    Str(Rc<str>),
    List(Rc<Vec<Value>>),
    /// Values by key: a keyed literal `{a: .., b: ..}`, or what a keyed or range loop bound.
    Map(Rc<Vec<(Key, Value)>>),
    Tuple(Rc<Vec<Value>>),
    Func(Rc<FnDef>),
    Builtin(&'static str),
    /// A direction an anchor gives.
    Dir(Vec3),
    /// A world axis direction (X, Y, Z): for selecting, never for placing after the first feature.
    WorldDir(Vec3),
    /// A line in space (from `shift`, `hline`, `vline`), and where its position came from.
    Line(Point3, UnitVec3, Src),
    Plane(Rc<Plane>),
    Axis(Point3, UnitVec3),
    Point(Point3),
    Sketch(Rc<RefCell<Sk>>),
    Ent(Ent),
    /// A hull item: a centre (coordinates or a point) and a radius.
    Disc(Rc<(Value, Num)>),
    Feature(Rc<Feat>),
    Faces(Rc<Faces>),
    Edges(Rc<Vec<Name>>),
    Extent(ExtentV),
    /// Sheet metal: features, edges, unfolds, and shop data (`sheet.rs`).
    Sheet(Rc<crate::sheet::SheetVal>),
    /// A work feature's datum, a pattern's layout, a body motion (wired.rs).
    Wired(Rc<crate::wired::W>),
    /// An assembly's instance, reference, joint or check (`asm.rs`).
    Asm(Rc<crate::asm::AsmVal>),
    /// Thermoforming's materials and shops (`thermo.rs`).
    Thermo(Rc<crate::thermo::ThermoVal>),
    /// What `check` cannot know without building (a selection, a face's plane).
    Unresolved,
}

impl Value {
    pub fn ty(&self) -> Ty {
        match self {
            Value::Num(_) => Ty::Num,
            Value::Bool(_) => Ty::Bool,
            Value::Str(_) => Ty::Str,
            Value::List(_) => Ty::List,
            Value::Map(_) => Ty::Map,
            Value::Tuple(t) if t.len() == 2 && t.iter().all(|x| matches!(x, Value::Num(_))) => Ty::Vec2,
            Value::Tuple(_) => Ty::Tuple,
            Value::Func(_) | Value::Builtin(_) => Ty::Function,
            Value::Dir(_) | Value::WorldDir(_) => Ty::Dir,
            Value::Line(..) => Ty::Line,
            Value::Plane(_) => Ty::Plane,
            Value::Axis(..) => Ty::Axis,
            Value::Point(_) => Ty::Point,
            Value::Sketch(_) => Ty::Sketch,
            Value::Ent(_) => Ty::Entity,
            Value::Disc(_) => Ty::Disc,
            Value::Feature(_) => Ty::Feature,
            Value::Faces(_) => Ty::Faces,
            Value::Edges(_) => Ty::Edges,
            Value::Extent(_) => Ty::Extent,
            Value::Sheet(s) => s.ty(),
            Value::Wired(w) => w.ty(),
            Value::Asm(a) => a.ty(),
            Value::Thermo(t) => t.ty(),
            Value::Unresolved => Ty::Unresolved,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ParamInfo {
    pub key: ParamKey,
    pub span: Span,
    /// The right-hand side: what `SetParam` replaces.
    pub rhs: Span,
    pub value: Value,
    /// The params its definition reads: links, shown as relations.
    pub links: BTreeSet<ParamKey>,
    pub used: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Ok,
    Failed,
    NotBuilt,
}

/// A dimension a feature took: the argument, its value, and where it came from.
#[derive(Clone, Debug)]
pub struct Dimension {
    pub what: &'static str,
    pub num: Num,
}

#[derive(Clone, Debug)]
pub struct FeatInfo {
    pub id: String,
    pub kind: &'static str,
    pub span: Span,
    pub idx: usize,
    pub status: Status,
    pub dims: Vec<Dimension>,
    /// The statement that makes it: what feature edit ops address.
    pub stmt: Span,
    pub top_level: bool,
    /// Every param its evaluation read (its arguments, its sketch rows, its selections).
    pub reads: BTreeSet<ParamKey>,
    /// A sketch's plane, as built.
    pub frame: Option<Frame>,
    /// The part's volume and centroid after it, when asked for (`Options::volumes`).
    pub mass: Option<[f64; 4]>,
    /// A hash of the kernel feature it became, when asked for: equal exactly when the feature is.
    pub fingerprint: Option<u64>,
}

#[derive(Clone, Debug, Default)]
struct Ctx {
    /// The call path: `lug[2]` while evaluating the body of `lug[2] = boss(..)`.
    prefix: Vec<Seg>,
    /// Loop keys since the last call boundary; None for a loop over a plain list, whose
    /// positions may not name anything.
    loops: Vec<Option<Key>>,
    /// The feature path whose local params are visible here.
    owner: Vec<Seg>,
    /// An assignment's name, waiting for the call that makes the feature.
    pending: Option<(String, Span)>,
    in_fn: usize,
    /// While evaluating a param definition: links may name other features' params.
    in_param: bool,
    top_stmt: Option<Span>,
}

pub struct Eval<'s> {
    pub src: &'s str,
    pub rules: Rules,
    pub shared: Vec<ParamInfo>,
    pub locals: Vec<ParamInfo>,
    pub fns: HashMap<String, Rc<FnDef>>,
    scopes: Vec<HashMap<String, (Value, Span)>>,
    pub b: Builder,
    pub feats: Vec<FeatInfo>,
    ids: HashMap<String, Span>,
    pub errors: Vec<Error>,
    pub warnings: Vec<Warning>,
    ctx: Ctx,
    sketch: Option<Rc<RefCell<Sk>>>,
    /// Inside a path block: its last segment and current end point.
    path: Option<PathState>,
    pub expects: Vec<Stmt>,
    /// Each expect's local names where it stood (loop variables, a function's arguments).
    expect_scopes: Vec<Vec<HashMap<String, (Value, Span)>>>,
    ret: Option<Value>,
    /// Inside a sketch: the name (with loop indices) the entity a call makes takes, and the call.
    pub ent_name: Option<(String, Span)>,
    /// Params whose values were observed (a branch, a count, an index, a comparison, a solver's
    /// input): editing one of these re-runs evaluation; any other is re-read.
    pub observed: BTreeSet<ParamKey>,
    /// Calls made for the top-level statement being evaluated: its budget.
    calls: usize,
    /// Params read since the last feature was made.
    pub reads: BTreeSet<ParamKey>,
    /// Measure the part's volume after every feature.
    pub volumes: bool,
    /// Run the slow checks (a thermoform's wall simulation); off, the shape alone.
    pub checks: bool,
    /// Whether the built-in being called was given something unknown.
    pub poisoned: bool,
    /// The part's sheets (`sheet.rs`).
    pub sheets: crate::sheet::Sheets,
    /// The part's thermoforms (`thermo.rs`).
    pub thermos: Vec<crate::thermo::ThermoOut>,
    /// Thermoforms' modelled parts whose mass is worked out after the run, beside the part's own
    /// measure (`run` does; any other evaluation works it out in place): each one's place in
    /// `thermos`, its memo key and the part.
    pub masses_owed: Option<Vec<(usize, String, forge_topo::Body)>>,
    /// Threads drawn as standards draw them (`Options::cosmetic_threads`): not modelled, only
    /// recorded, so a drawing's hidden-line pass sees the plain envelope.
    pub cosmetic_threads: bool,
    pub threads: Vec<crate::holes::Cosmetic>,
    /// Where the files features read (fonts, imports) are found: the part file's directory.
    pub dir: Option<std::path::PathBuf>,
    /// An assembly document's instances, joints and checks (`asm.rs`); None in a part.
    pub asm: Option<crate::asm::Doc>,
    /// The session's root, canonical: no file outside it is read (fonts, imports, parts).
    pub root: Option<std::path::PathBuf>,
    /// The files `use` read (`uses.rs`), kept while their functions may be called.
    pub libs: Vec<Rc<crate::uses::Lib>>,
    /// The files being used, outermost first: a file already here is a `use` cycle.
    pub use_chain: Vec<std::path::PathBuf>,
    /// The same files as the documents name them, for what errors say (never a real path).
    pub use_files: Vec<String>,
    /// Inside a used file's function: that file's names.
    lib: Option<Rc<crate::uses::Lib>>,
    /// Inside a used file's function: the call in this document, where what it makes is shown.
    lib_call: Option<Span>,
    /// The document's `properties(..)` (`props.rs`).
    pub props: crate::props::Props,
    /// The inline components' own evaluations, in the order the document makes them (`asm.rs`).
    pub comps: Vec<crate::asm::Comp<'s>>,
    /// Inside a component's call: the calls it is under (`box(d=1)/`), what its own in-file
    /// components' keys start with; empty for the document.
    pub comp_path: String,
    /// The whole run's budget, shared by every evaluation it makes (each component's call, at
    /// any depth): function calls, and component evaluations.
    pub(crate) total: Rc<std::cell::Cell<(usize, usize)>>,
    /// How deep in calls this evaluation starts (a component's, its caller's depth).
    pub(crate) depth0: usize,
}

#[derive(Clone)]
pub struct PathState {
    pub start: EntityId,
    pub at: EntityId,
    pub last: Option<EntityId>,
    pub segs: Vec<EntityId>,
    pub name: String,
}

pub type R<T> = Result<T, Error>;

/// Function calls one top-level statement may make (recursion is allowed within it).
pub const CALL_BUDGET: usize = 100_000;
/// Function calls a whole run may make, its components' evaluations with it.
pub const TOTAL_CALLS: usize = 1_000_000;
/// Component evaluations a whole run may make (a wide tree of calls stops here, with Fuel).
pub const TOTAL_COMPONENTS: usize = 2_000;

/// How deep calls may nest (the evaluator's own stack is the limit).
pub const DEPTH_BUDGET: usize = 128;

pub const WORDS: [&str; 6] = ["thru", "thru_all", "next", "X", "Y", "Z"];

impl<'s> Eval<'s> {
    /// What the top level bound, by name (for `anchors`).
    pub(crate) fn top_level(&self) -> Vec<(String, Value)> {
        self.scopes[0]
            .iter()
            .map(|(k, v)| (k.clone(), v.0.clone()))
            .collect()
    }

    pub fn new(src: &'s str, rules: Rules, building: bool, cache: Cache) -> Eval<'s> {
        Eval {
            src,
            rules,
            shared: vec![],
            locals: vec![],
            fns: HashMap::new(),
            scopes: vec![HashMap::new()],
            b: Builder::new(building, cache),
            feats: vec![],
            ids: HashMap::new(),
            errors: vec![],
            warnings: vec![],
            ctx: Ctx::default(),
            sketch: None,
            path: None,
            expects: vec![],
            expect_scopes: vec![],
            ret: None,
            ent_name: None,
            observed: BTreeSet::new(),
            calls: 0,
            reads: BTreeSet::new(),
            volumes: false,
            checks: true,
            poisoned: false,
            sheets: Default::default(),
            thermos: vec![],
            masses_owed: None,
            cosmetic_threads: false,
            threads: vec![],
            dir: None,
            asm: None,
            root: None,
            libs: vec![],
            use_chain: vec![],
            use_files: vec![],
            lib: None,
            lib_call: None,
            props: Default::default(),
            comps: vec![],
            comp_path: String::new(),
            total: Default::default(),
            depth0: 0,
        }
    }

    /// An evaluator for a part of this document (a component's call, its defaults, a check of
    /// its definition) over the same text: the session's fence and every context a read of a
    /// file obeys go with it (root, directory, the use chain, its used files), with the shared
    /// params and functions it may see.
    pub(crate) fn child(&self, building: bool, cache: Cache) -> Eval<'s> {
        let mut c = Eval::new(self.src, self.rules, building, cache);
        c.root = self.root.clone();
        c.dir = self.dir.clone();
        c.volumes = self.volumes;
        c.use_chain = self.use_chain.clone();
        c.use_files = self.use_files.clone();
        c.libs = self.libs.clone();
        c.shared = self.shared.clone();
        c.fns = self.fns.clone();
        c.comp_path = self.comp_path.clone();
        c.total = self.total.clone();
        c.depth0 = self.depth0 + self.ctx.in_fn;
        // the component definitions above it, as functions are: a block may place them
        for (n, (v, sp)) in &self.scopes[0] {
            if let Value::Asm(a) = v
                && matches!(&**a, crate::asm::AsmVal::Def(_))
            {
                c.scopes[0].insert(n.clone(), (v.clone(), *sp));
            }
        }
        c
    }

    /// Puts back expect `k`'s local names; `leave_expect` with the mark takes them away again.
    pub(crate) fn enter_expect(&mut self, k: usize) -> usize {
        let mark = self.scopes.len();
        if let Some(sc) = self.expect_scopes.get(k) {
            self.scopes.extend(sc.iter().cloned());
        }
        mark
    }

    pub(crate) fn leave_expect(&mut self, mark: usize) {
        self.scopes.truncate(mark.max(1));
    }

    /// One top-level statement, as `run` takes it (a used file's).
    pub(crate) fn run_one(&mut self, s: &Stmt) -> R<()> {
        self.ctx.top_stmt = Some(s.full);
        self.calls = 0;
        self.stmt(s)
    }

    /// The name an assignment gives the call at `span`, if the call is that assignment's.
    pub(crate) fn binding(&mut self, span: Span) -> Option<String> {
        match self.ctx.pending.take() {
            Some((n, sp)) if sp == span => Some(n),
            other => {
                self.ctx.pending = other;
                None
            }
        }
    }

    /// The name a block statement gives the id it takes next (`take_id`).
    pub(crate) fn ctx_pending(&mut self, name: &str, span: Span) {
        self.ctx.pending = Some((name.into(), span));
    }

    pub(crate) fn bind_name(&mut self, name: &str, v: Value, span: Span) -> R<()> {
        self.bind(name, v, span)
    }

    pub(crate) fn in_sketch(&self) -> bool {
        self.sketch.is_some()
    }

    /// Evaluates `f` with `name` bound to `v` in a scope of its own (a name no source can write).
    pub(crate) fn with_bound<T>(&mut self, names: Vec<(String, Value)>, f: impl FnOnce(&mut Self) -> T) -> T {
        let mut sc = HashMap::new();
        for (n, v) in names {
            sc.insert(n, (v, Span { lo: 0, hi: 0 }));
        }
        self.scopes.push(sc);
        let r = f(self);
        self.scopes.pop();
        r
    }

    /// At a document's top level: not in a function, loop, sketch or path.
    pub(crate) fn at_top(&self) -> bool {
        self.ctx.in_fn == 0 && self.ctx.loops.is_empty() && self.sketch.is_none() && self.path.is_none()
    }

    /// The top-level names and their values, by name.
    pub fn globals(&self) -> Vec<(String, Value)> {
        let mut out: Vec<(String, Value)> = self.scopes[0]
            .iter()
            .map(|(k, v)| (k.clone(), v.0.clone()))
            .collect();
        out.sort_by(|a, b| a.0.cmp(&b.0));
        out
    }

    /// The feature path being made (the call path and loop indices), for a record of its own.
    pub fn record(&mut self, id: &str, kind: &'static str, span: Span, dims: Vec<Dimension>) {
        // made by a used file's function: shown at the call in this document
        let span = self.lib_call.unwrap_or(span);
        self.feats.push(FeatInfo {
            id: id.into(),
            kind,
            span,
            idx: usize::MAX,
            status: Status::NotBuilt,
            dims,
            stmt: self.ctx.top_stmt.unwrap_or(span),
            top_level: self.ctx.prefix.is_empty() && self.ctx.loops.is_empty(),
            reads: std::mem::take(&mut self.reads),
            frame: None,
            mass: None,
            fingerprint: None,
        });
    }

    // ---- the static pass ---------------------------------------------------------------------

    /// Rules checked before evaluation: loose numbers, recursion.
    pub fn check_static(&mut self, prog: &[Stmt]) {
        crate::props::check_placement(prog, &mut self.errors);
        crate::asm::cycle_check(prog, &mut self.errors);
        if self.rules.literals_in_params != Level::Off {
            for s in prog {
                let owner = match &s.kind {
                    StmtKind::Assign { name, .. } | StmtKind::Block { name, .. } => Some(vec![Seg {
                        name: name.clone(),
                        index: vec![],
                    }]),
                    _ => None,
                };
                self.literals_stmt(s, &owner);
            }
        }
        if self.rules.no_recursion != Level::Off {
            self.recursion(prog);
        }
        // hard: no local (a function's parameter or binding, a loop or comprehension variable, a
        // sketch's entity) has a top-level name
        let mut top = crate::edit::bound(prog);
        for s in prog {
            if let StmtKind::Param { owner, .. } = &s.kind {
                top.extend(owner.first().map(|o| o.name.clone()));
            }
        }
        let mut locals = vec![];
        locals_of(prog, true, &mut locals);
        for (name, span) in locals {
            if top.contains(&name) {
                self.errors.push(Error::ShadowsTopLevel { span, name });
            }
        }
    }

    fn literals_stmt(&mut self, s: &Stmt, owner: &Option<Vec<Seg>>) {
        match &s.kind {
            StmtKind::Shared { .. } | StmtKind::Param { .. } | StmtKind::Expect { .. } => {}
            StmtKind::Fn { body, params, .. } => {
                for p in params {
                    if let Some(d) = &p.default {
                        self.literals_expr(d, false, &None);
                    }
                }
                for b in body {
                    self.literals_stmt(b, &None);
                }
            }
            StmtKind::For { iter, body, .. } => {
                self.literals_expr(iter, true, owner);
                for b in body {
                    let o = match (&b.kind, owner) {
                        (StmtKind::Assign { name, .. } | StmtKind::Block { name, .. }, None) => {
                            Some(vec![Seg {
                                name: name.clone(),
                                index: vec![],
                            }])
                        }
                        _ => owner.clone(),
                    };
                    self.literals_stmt(b, &o);
                }
            }
            StmtKind::If { cond, then, other } => {
                self.literals_expr(cond, false, owner);
                for b in then.iter().chain(other.iter().flatten()) {
                    self.literals_stmt(b, owner);
                }
            }
            StmtKind::Return(e) | StmtKind::Expr(e) | StmtKind::Assign { value: e, .. } => {
                self.literals_expr(e, false, owner)
            }
            // its own evaluation lints its body; its `at:` is a written pose
            StmtKind::Block {
                kind: BlockKind::Component,
                ..
            } => {}
            StmtKind::Block { args, body, .. } => {
                for a in args {
                    self.literals_expr(&a.value, false, owner);
                }
                for b in body {
                    self.literals_stmt(b, owner);
                }
            }
        }
    }

    fn literals_expr(&mut self, e: &Expr, count: bool, owner: &Option<Vec<Seg>>) {
        let go = |this: &mut Self, x: &Expr, c: bool| this.literals_expr(x, c, owner);
        match &e.kind {
            ExprKind::Num(v, u) => {
                let allowed = match u {
                    Unit::Deg => [0.0, 90.0, 180.0, 360.0].contains(&v.abs()),
                    _ => [0.0, 1.0, 2.0].contains(&v.abs()) || (count && v.fract() == 0.0 && *v >= 0.0),
                };
                if !allowed {
                    let err = Error::LooseLiteral {
                        span: e.span,
                        value: *v,
                        unit: *u,
                        owner: owner.clone(),
                        rule: Rule::LiteralsInParams,
                    };
                    if let Err(x) = self.rule(self.rules.literals_in_params, err) {
                        self.errors.push(x);
                    }
                }
            }
            ExprKind::Str(_) | ExprKind::Bool(_) | ExprKind::Name(_) => {}
            ExprKind::List(xs) | ExprKind::Tuple(xs) => {
                for x in xs {
                    go(self, x, false);
                }
            }
            ExprKind::Map(xs) => {
                for (_, x) in xs {
                    go(self, x, false);
                }
            }
            ExprKind::Comp { item, iter, cond, .. } => {
                go(self, item, false);
                go(self, iter, true);
                if let Some(c) = cond {
                    go(self, c, false);
                }
            }
            ExprKind::Range { lo, hi, .. } => {
                go(self, lo, true);
                go(self, hi, true);
            }
            ExprKind::Bin(_, a, b) => {
                go(self, a, count);
                go(self, b, count);
            }
            ExprKind::Neg(a) | ExprKind::Not(a) => go(self, a, count),
            ExprKind::Call(f, args) => {
                go(self, f, false);
                // iProperties are data about the part, not its design numbers
                if matches!(&f.kind, ExprKind::Name(n) if n == "properties") {
                    return;
                }
                // an assembly's written poses are where the solve starts, not design numbers
                if crate::asm::written_pose(f) {
                    return;
                }
                // a standard part's length and sheet pick a catalog row, like its size string
                let catalog = matches!(&f.kind, ExprKind::Name(n) if n == "std");
                for a in args {
                    if a.name.as_ref().is_some_and(|(n, _)| n == "at") && crate::asm::places(f) {
                        continue;
                    }
                    if catalog
                        && a.name
                            .as_ref()
                            .is_some_and(|(n, _)| n == "length" || n == "sheet")
                    {
                        continue;
                    }
                    let c = a.name.as_ref().is_some_and(|(n, _)| n == "n" || n == "count");
                    go(self, &a.value, c);
                }
            }
            ExprKind::Attr(o, _) => go(self, o, false),
            ExprKind::Index(o, i) => {
                go(self, o, false);
                go(self, i, true);
            }
            ExprKind::If { cond, then, other } => {
                go(self, cond, false);
                go(self, then, count);
                go(self, other, count);
            }
        }
    }

    fn recursion(&mut self, prog: &[Stmt]) {
        let mut calls: HashMap<String, (Span, BTreeSet<String>)> = HashMap::new();
        for s in prog {
            if let StmtKind::Fn { name, body, .. } = &s.kind {
                let mut names = BTreeSet::new();
                syntax::walk(body, &mut |st| {
                    let mut add = |e: &Expr| called(e, &mut names);
                    match &st.kind {
                        StmtKind::Assign { value, .. } | StmtKind::Expr(value) | StmtKind::Return(value) => {
                            add(value)
                        }
                        StmtKind::For { iter, .. } => add(iter),
                        StmtKind::If { cond, .. } => add(cond),
                        StmtKind::Block { args, .. } => args.iter().for_each(|a| add(&a.value)),
                        _ => {}
                    }
                });
                calls.insert(name.clone(), (s.span, names));
            }
        }
        for (f, (span, _)) in &calls {
            let mut seen = BTreeSet::new();
            let mut todo: Vec<String> = calls[f].1.iter().cloned().collect();
            while let Some(g) = todo.pop() {
                if &g == f {
                    let err = Error::Recursion {
                        span: *span,
                        func: f.clone(),
                        rule: Rule::NoRecursion,
                    };
                    if let Err(x) = self.rule(self.rules.no_recursion, err) {
                        self.errors.push(x);
                    }
                    break;
                }
                if seen.insert(g.clone())
                    && let Some((_, n)) = calls.get(&g)
                {
                    todo.extend(n.iter().cloned());
                }
            }
        }
        fn called(e: &Expr, out: &mut BTreeSet<String>) {
            match &e.kind {
                ExprKind::Call(f, args) => {
                    if let ExprKind::Name(n) = &f.kind {
                        out.insert(n.clone());
                    }
                    called(f, out);
                    args.iter().for_each(|a| called(&a.value, out));
                }
                ExprKind::List(xs) | ExprKind::Tuple(xs) => xs.iter().for_each(|x| called(x, out)),
                ExprKind::Map(xs) => xs.iter().for_each(|x| called(&x.1, out)),
                ExprKind::Bin(_, a, b) | ExprKind::Index(a, b) => {
                    called(a, out);
                    called(b, out);
                }
                ExprKind::Neg(a) | ExprKind::Not(a) | ExprKind::Attr(a, _) => called(a, out),
                ExprKind::Comp { item, iter, cond, .. } => {
                    called(item, out);
                    called(iter, out);
                    if let Some(c) = cond {
                        called(c, out);
                    }
                }
                ExprKind::Range { lo, hi, .. } => {
                    called(lo, out);
                    called(hi, out);
                }
                ExprKind::If { cond, then, other } => {
                    called(cond, out);
                    called(then, out);
                    called(other, out);
                }
                _ => {}
            }
        }
    }

    // ---- statements --------------------------------------------------------------------------

    pub fn run(&mut self, prog: &[Stmt]) {
        for s in prog {
            self.ctx.top_stmt = Some(s.full);
            self.calls = 0;
            if let Err(e) = self.stmt(s) {
                self.errors.push(e);
                // a failed statement's name stands for nothing known, so errors don't cascade
                if let StmtKind::Assign { name, .. } | StmtKind::Block { name, .. } = &s.kind
                    && !self.scopes[0].contains_key(name)
                {
                    self.scopes[0].insert(name.clone(), (Value::Unresolved, s.span));
                }
            }
        }
        // an error inside a loop is one error, however many times the loop ran into it
        let mut seen = std::collections::HashSet::new();
        self.errors.retain(|e| seen.insert((e.kind(), e.span())));
        self.expect_reads();
        self.close_observed();
        for p in self.shared.iter().chain(&self.locals) {
            if !p.used {
                self.warnings.push(Warning::UnusedParam {
                    span: p.span,
                    param: p.key.clone(),
                });
            }
        }
    }

    /// Marks the params `expect` lines read as used: they are evaluated after the build (and
    /// only when it finished), but a param that states the design's intent is not unused. They
    /// are read here as the expect line will read them, and what that reading says is dropped.
    fn expect_reads(&mut self) {
        let (errors, warnings) = (self.errors.len(), self.warnings.len());
        let observed = self.observed.clone();
        let saved = self.ctx.owner.clone();
        self.ctx.owner = vec![];
        for (k, s) in self.expects.clone().iter().enumerate() {
            if let StmtKind::Expect { args, cmp, at, .. } = &s.kind {
                let mark = self.enter_expect(k);
                let tol = cmp
                    .iter()
                    .flat_map(|(_, v, t)| std::iter::once(v).chain(t.iter().map(|t| &t.0)));
                // `at P = v`: v is read, and P is set for the check (the param is the check's)
                let values = at.iter().map(|(_, v)| v);
                for e in args.iter().chain(tol).chain(values) {
                    let _ = self.expr(e);
                }
                for (path, _) in at {
                    if let Some((last, owner)) = path.split_last() {
                        let key = ParamKey {
                            owner: owner.to_vec(),
                            name: last.name.clone(),
                        };
                        for p in self.shared.iter_mut().chain(self.locals.iter_mut()) {
                            if p.key == key {
                                p.used = true;
                            }
                        }
                    }
                }
                self.leave_expect(mark);
            }
        }
        self.ctx.owner = saved;
        self.errors.truncate(errors);
        self.warnings.truncate(warnings);
        self.observed = observed;
    }

    fn bind_loop(&mut self, name: &str, v: Value, span: Span) -> R<()> {
        self.bind(name, v, span)
    }

    /// Binds a name once: rebinding, or shadowing a name, a param or a built-in, is an error.
    fn bind(&mut self, name: &str, v: Value, span: Span) -> R<()> {
        // a work plane's plane, for the app to draw
        if let Value::Wired(w) = &v
            && let Some(f) = w.plane()
            && let Some(last) = self.feats.last_mut()
            && last.kind == "workplane"
            && last.frame.is_none()
        {
            last.frame = Some(f);
        }
        if crate::builtins::builtin(name).is_some() || crate::builtins::constant(name).is_some() {
            return Err(Error::ShadowsBuiltin {
                span,
                name: name.into(),
            });
        }
        if self.rules.single_assignment != Level::Off {
            let mut first = None;
            for sc in &self.scopes {
                if let Some((_, f)) = sc.get(name) {
                    first = Some(*f);
                }
            }
            let here = self.ctx.owner.clone();
            if let Some(p) = self
                .shared
                .iter()
                .chain(&self.locals)
                .find(|p| p.key.name == name && Self::visible(&p.key.owner, &here).is_some())
            {
                first = Some(p.span);
            }
            if let Some(first) = first {
                let err = Error::Rebind {
                    span,
                    name: name.into(),
                    first,
                };
                self.rule(self.rules.single_assignment, err)?;
            }
        }
        if self.scopes.len() == 1
            && let Some(p) = self.shared.iter().find(|p| p.key.name == name)
        {
            return Err(Error::Rebind {
                span,
                name: name.into(),
                first: p.span,
            });
        }
        let scope = self.scopes.last_mut().expect("a scope");
        if let Some((_, first)) = scope.get(name) {
            return Err(Error::Rebind {
                span,
                name: name.into(),
                first: *first,
            });
        }
        scope.insert(name.into(), (v, span));
        Ok(())
    }

    /// An observed param's definition is observed too: the params it is made of, through every
    /// link (`m = n + 1` observed: `n` is structural as well).
    fn close_observed(&mut self) {
        let mut todo: Vec<ParamKey> = self.observed.iter().cloned().collect();
        while let Some(k) = todo.pop() {
            let links: Vec<ParamKey> = self
                .shared
                .iter()
                .chain(&self.locals)
                .filter(|p| p.key == k)
                .flat_map(|p| p.links.iter().cloned())
                .collect();
            for l in links {
                if self.observed.insert(l.clone()) {
                    todo.push(l);
                }
            }
        }
    }

    pub fn observe(&mut self, n: &Num) {
        self.observed.extend(n.src.params());
    }

    pub fn observe_value(&mut self, v: &Value) {
        let mut ps = BTreeSet::new();
        collect_params(v, &mut ps);
        self.observed.extend(ps);
    }

    /// A rule broken: an error, a lint, or nothing, as the rule's level says.
    pub fn rule(&mut self, level: Level, e: Error) -> R<()> {
        match level {
            Level::Off => Ok(()),
            Level::Lint => {
                // the same lint at the same place once, not once per loop instance
                let w = Warning::Lint(e);
                if !self.warnings.contains(&w) {
                    self.warnings.push(w);
                }
                Ok(())
            }
            Level::Error => Err(e),
        }
    }

    /// The path a name has here, for seeing params (a loop over a plain list adds no key).
    fn here(&self, name: &str) -> Vec<Seg> {
        let mut p = self.ctx.prefix.clone();
        p.push(Seg {
            name: name.into(),
            index: self.ctx.loops.iter().flatten().cloned().collect(),
        });
        p
    }

    /// The loop keys an id made here takes: an error inside a loop over a plain list, whose
    /// positions would shift when the list is edited.
    fn id_keys(&self, span: Span) -> R<Vec<Key>> {
        if self.ctx.loops.iter().any(Option::is_none) {
            return Err(Error::PositionalId {
                span,
                rule: Rule::KeyedIds,
            });
        }
        Ok(self.ctx.loops.iter().flatten().cloned().collect())
    }

    pub fn stmt(&mut self, s: &Stmt) -> R<()> {
        match &s.kind {
            StmtKind::Shared { name, value } | StmtKind::Param { name, value, .. }
                if crate::builtins::builtin(name).is_some() || crate::builtins::constant(name).is_some() =>
            {
                let _ = value;
                Err(Error::ShadowsBuiltin {
                    span: s.span,
                    name: name.clone(),
                })
            }
            StmtKind::Shared { name, value } => {
                if let Some(p) = self.shared.iter().find(|p| &p.key.name == name) {
                    return Err(Error::ParamTwice {
                        span: s.span,
                        param: p.key.clone(),
                        first: p.span,
                    });
                }
                let (v, links) = self.param_value(value, &[])?;
                self.shared.push(ParamInfo {
                    key: ParamKey::shared(name),
                    span: s.span,
                    rhs: value.span,
                    value: v,
                    links,
                    used: false,
                });
                Ok(())
            }
            StmtKind::Param { owner, name, value } => {
                let key = ParamKey {
                    owner: owner.clone(),
                    name: name.clone(),
                };
                if let Some(p) = self.locals.iter().find(|p| p.key == key) {
                    return Err(Error::ParamTwice {
                        span: s.span,
                        param: key,
                        first: p.span,
                    });
                }
                let (v, links) = self.param_value(value, owner)?;
                self.locals.push(ParamInfo {
                    key,
                    span: s.span,
                    rhs: value.span,
                    value: v,
                    links,
                    used: false,
                });
                Ok(())
            }
            StmtKind::Fn { name, params, body } => {
                self.fns.insert(
                    name.clone(),
                    Rc::new(FnDef {
                        name: name.clone(),
                        params: params.clone(),
                        body: Rc::new(body.clone()),
                        span: s.span,
                        lib: None,
                    }),
                );
                Ok(())
            }
            StmtKind::For { vars, iter, body } => {
                // a loop's header reads what its statement would: the sketch's own names inside
                // a sketch, else the enclosing scope's
                let saved_owner = self.ctx.owner.clone();
                if self.sketch.is_none() {
                    self.ctx.owner = self.ctx.prefix.clone();
                }
                let it = self.expr(iter);
                self.ctx.owner = saved_owner;
                let it = it?;
                // ids inside: a range's values, a keyed map's labels; a plain list gives none
                let ranged = matches!(iter.kind, ExprKind::Range { .. });
                let items: Vec<(Option<Key>, Value)> = match it {
                    Value::Map(m) => m.iter().map(|(k, v)| (Some(k.clone()), v.clone())).collect(),
                    Value::List(xs) => xs
                        .iter()
                        .enumerate()
                        .map(|(k, v)| match (ranged, v) {
                            (true, Value::Num(n)) => (Some(Key::Ix(n.v.round() as i64)), v.clone()),
                            _ if self.rules.keyed_ids == Level::Off => (Some(Key::Ix(k as i64)), v.clone()),
                            _ => (None, v.clone()),
                        })
                        .collect(),
                    other => {
                        return Err(Error::WrongType {
                            span: iter.span,
                            what: "a loop's range, keyed map or list",
                            want: Ty::List,
                            found: other.ty(),
                        });
                    }
                };
                if ranged {
                    let vals: Vec<Value> = items.iter().map(|x| x.1.clone()).collect();
                    self.observe_value(&Value::List(Rc::new(vals)));
                }
                let mut collected: Vec<Collected> = vec![];
                for (key, item) in items {
                    self.scopes.push(HashMap::new());
                    self.ctx.loops.push(key.clone());
                    let r = (|| {
                        match vars.len() {
                            1 => self.bind(&vars[0], item, iter.span)?,
                            _ => {
                                let k = match &key {
                                    Some(Key::Label(l)) => Value::Str(l.as_str().into()),
                                    Some(Key::Ix(i)) => Value::Num(Num::konst(*i as f64, Dim::Count)),
                                    None => Value::Unresolved,
                                };
                                self.bind(&vars[0], k, iter.span)?;
                                self.bind(&vars[1], item, iter.span)?;
                            }
                        }
                        for b in body {
                            self.stmt(b)?;
                            if self.ret.is_some() {
                                break;
                            }
                        }
                        Ok(())
                    })();
                    self.ctx.loops.pop();
                    let scope = self.scopes.pop().expect("loop scope");
                    if let Err(e) = r {
                        self.errors.push(e);
                    }
                    // every name the body bound, keyed as the loop is
                    let mut names: Vec<(&String, &(Value, Span))> =
                        scope.iter().filter(|(k, _)| !vars.contains(k)).collect();
                    names.sort_by_key(|x| (x.1.1.lo, x.0.clone()));
                    for (n, (v, sp)) in names {
                        match collected.iter_mut().find(|c| &c.0 == n) {
                            Some(c) => c.1.push((key.clone(), v.clone())),
                            None => collected.push((n.clone(), vec![(key.clone(), v.clone())], *sp)),
                        }
                    }
                    if self.ret.is_some() {
                        break;
                    }
                }
                for (n, vs, sp) in collected {
                    let v = if vs.iter().all(|x| x.0.is_some()) {
                        Value::Map(Rc::new(
                            vs.into_iter().map(|(k, v)| (k.expect("keyed"), v)).collect(),
                        ))
                    } else {
                        Value::List(Rc::new(vs.into_iter().map(|x| x.1).collect()))
                    };
                    self.bind_loop(&n, v, sp)?;
                }
                Ok(())
            }
            StmtKind::If { cond, then, other } => {
                let c = self.expr(cond)?;
                let Value::Bool(c) = c else {
                    return Err(Error::WrongType {
                        span: cond.span,
                        what: "an if condition",
                        want: Ty::Bool,
                        found: c.ty(),
                    });
                };
                let body = if c { Some(then) } else { other.as_ref() };
                for b in body.into_iter().flatten() {
                    self.stmt(b)?;
                    if self.ret.is_some() {
                        break;
                    }
                }
                Ok(())
            }
            StmtKind::Return(e) => {
                if self.ctx.in_fn == 0 {
                    return Err(Error::ReturnOutsideFn { span: s.span });
                }
                let v = self.expr(e)?;
                self.ret = Some(v);
                Ok(())
            }
            StmtKind::Expect { .. } => {
                self.expects.push(s.clone());
                // the names it sees here (a loop's variable), for reading it after the run
                self.expect_scopes.push(self.scopes[1..].to_vec());
                Ok(())
            }
            StmtKind::Block {
                kind,
                name,
                args,
                body,
            } => match kind {
                BlockKind::Sketch => self.sketch_block(s, name, args, body),
                BlockKind::Path => self.path_block(s, name, args, body),
                BlockKind::Component => crate::asm::component_block(self, s, name, args, body),
                // a drawing of this document: drawn only when asked (`drawing::block`)
                BlockKind::Drawing => Ok(()),
            },
            StmtKind::Assign { name, value } => {
                let saved = self.ctx.owner.clone();
                if self.sketch.is_none() {
                    self.ctx.owner = self.here(name);
                }
                if matches!(value.kind, ExprKind::Call(..)) {
                    if self.sketch.is_some() {
                        let path = path_text(&[Seg {
                            name: name.clone(),
                            index: self.id_keys(s.span)?,
                        }]);
                        self.ent_name = Some((path, value.span));
                    } else {
                        self.ctx.pending = Some((name.clone(), value.span));
                    }
                }
                let r = self.expr(value);
                self.ctx.pending = None;
                self.ent_name = None;
                self.ctx.owner = saved;
                let v = r?;
                if let (Some(sk), Value::Ent(e)) = (&self.sketch, &v) {
                    // entities take the name they are bound to, for stable face names
                    let mut sk = sk.borrow_mut();
                    for x in sk.ents.iter_mut() {
                        if x.0 == e.id && x.1.starts_with('~') {
                            x.1 = name.clone();
                        }
                    }
                }
                self.bind(name, v, s.span)
            }
            StmtKind::Expr(e) => {
                if self.sketch.is_none() {
                    self.ctx.owner = self.ctx.prefix.clone();
                }
                self.expr(e)?;
                Ok(())
            }
        }
    }

    /// A param definition's value: shared params, the owner's earlier locals, and (as explicit
    /// links) other features' params by their qualified name.
    fn param_value(&mut self, e: &Expr, owner: &[Seg]) -> R<(Value, BTreeSet<ParamKey>)> {
        let saved = (self.ctx.owner.clone(), self.ctx.in_param);
        self.ctx.owner = owner.to_vec();
        self.ctx.in_param = true;
        let r = self.expr(e);
        self.ctx.owner = saved.0;
        self.ctx.in_param = saved.1;
        let v = settle(r?);
        let mut links = BTreeSet::new();
        collect_params(&v, &mut links);
        Ok((v, links))
    }

    fn iterable(&mut self, v: Value, span: Span) -> R<Vec<Value>> {
        match v {
            Value::List(xs) => Ok((*xs).clone()),
            Value::Map(m) => Ok(m.iter().map(|x| x.1.clone()).collect()),
            other => Err(Error::WrongType {
                span,
                what: "a loop's range or list",
                want: Ty::List,
                found: other.ty(),
            }),
        }
    }

    // ---- names and params --------------------------------------------------------------------

    fn visible(owner: &[Seg], here: &[Seg]) -> Option<usize> {
        if owner.len() > here.len() {
            return None;
        }
        let mut score = 0;
        for (o, h) in owner.iter().zip(here) {
            if o.name != h.name {
                return None;
            }
            if !o.index.is_empty() {
                if o.index != h.index {
                    return None;
                }
                score += 1;
            }
        }
        Some(owner.len() * 2 + score)
    }

    fn param_num(&mut self, which: (bool, usize), span: Span) -> Value {
        let p = if which.0 {
            &mut self.shared[which.1]
        } else {
            &mut self.locals[which.1]
        };
        p.used = true;
        let key = p.key.clone();
        driving(&p.value, &key, span)
    }

    fn lookup(&mut self, name: &str, span: Span) -> R<Value> {
        // loop variables, arguments and sketch entities first, then this feature's own params,
        // then top-level names, then shared params
        for scope in self.scopes[1..].iter().rev() {
            if let Some((v, _)) = scope.get(name) {
                return Ok(v.clone());
            }
        }
        // a used file's function sees that file's names and the built-ins, nothing of the caller
        if let Some(lib) = self.lib.clone() {
            if let Some(v) = lib.names.borrow().get(name) {
                return Ok(v.clone());
            }
            if let Some(v) = crate::builtins::constant(name) {
                return Ok(v);
            }
            if let Some(b) = crate::builtins::builtin(name) {
                return Ok(Value::Builtin(b));
            }
            return Err(Error::UnknownName {
                span,
                name: name.into(),
            });
        }
        let here = self.ctx.owner.clone();
        let best = self
            .locals
            .iter()
            .enumerate()
            .filter(|(_, p)| p.key.name == name)
            .filter_map(|(k, p)| Self::visible(&p.key.owner, &here).map(|s| (s, k)))
            .max();
        if let Some((_, k)) = best {
            return Ok(self.param_num((false, k), span));
        }
        if let Some((v, _)) = self.scopes[0].get(name) {
            return Ok(v.clone());
        }
        if let Some(k) = self.shared.iter().position(|p| p.key.name == name) {
            return Ok(self.param_num((true, k), span));
        }
        if let Some(f) = self.fns.get(name) {
            return Ok(Value::Func(f.clone()));
        }
        if let Some(v) = crate::builtins::constant(name) {
            return Ok(v);
        }
        if let Some(v) = crate::asm::constant(name) {
            return Ok(v);
        }
        if let Some(b) = crate::builtins::builtin(name) {
            return Ok(Value::Builtin(b));
        }
        if let Some(k) = self.locals.iter().position(|p| p.key.name == name) {
            let err = self.out_of_scope(self.locals[k].key.clone(), span);
            self.rule(self.rules.scoped_params, err)?;
            return Ok(self.param_num((false, k), span));
        }
        Err(Error::UnknownName {
            span,
            name: name.into(),
        })
    }

    fn out_of_scope(&self, param: ParamKey, span: Span) -> Error {
        let p = self.locals.iter().find(|p| p.key == param).expect("a local");
        let value = match &p.value {
            Value::Num(n) => Literal {
                value: if n.dim == Dim::Angle {
                    n.v.to_degrees()
                } else {
                    n.v
                },
                dim: n.dim,
            },
            _ => Literal {
                value: 0.0,
                dim: Dim::Length,
            },
        };
        Error::ParamOutOfScope {
            span,
            param,
            used_in: self
                .ctx
                .owner
                .iter()
                .map(|s| Seg {
                    name: s.name.clone(),
                    index: vec![],
                })
                .collect(),
            value,
            rule: Rule::ScopedParams,
        }
    }

    // ---- expressions -------------------------------------------------------------------------

    pub fn expr(&mut self, e: &Expr) -> R<Value> {
        match &e.kind {
            ExprKind::Num(v, u) => Ok(Value::Num(match u {
                Unit::Deg => Num::konst(v.to_radians(), Dim::Angle),
                Unit::Mm => Num::konst(*v, Dim::Length),
                // in a param too: a bare number takes the other operand's dimension, so `W / 2`
                // is a Length; what is still bare when the param is made settles to Length
                Unit::None => Num::konst(*v, Dim::Any),
            })),
            ExprKind::Str(s) => Ok(Value::Str(s.as_str().into())),
            ExprKind::Bool(b) => Ok(Value::Bool(*b)),
            ExprKind::Name(n) => self.lookup(n, e.span),
            ExprKind::List(xs) => {
                let v: Vec<Value> = xs.iter().map(|x| self.expr(x)).collect::<R<_>>()?;
                Ok(Value::List(Rc::new(v)))
            }
            ExprKind::Tuple(xs) => {
                let v: Vec<Value> = xs.iter().map(|x| self.expr(x)).collect::<R<_>>()?;
                Ok(Value::Tuple(Rc::new(v)))
            }
            ExprKind::Map(entries) => {
                let mut v: Vec<(Key, Value)> = vec![];
                for (k, x) in entries {
                    if v.iter().any(|(q, _)| *q == Key::Label(k.clone())) {
                        return Err(Error::Rebind {
                            span: x.span,
                            name: k.clone(),
                            first: e.span,
                        });
                    }
                    v.push((Key::Label(k.clone()), self.expr(x)?));
                }
                Ok(Value::Map(Rc::new(v)))
            }
            ExprKind::Comp {
                item,
                vars,
                iter,
                cond,
            } => {
                let it = self.expr(iter)?;
                let items = self.iterable(it, iter.span)?;
                let mut out = vec![];
                for (k, x) in items.into_iter().enumerate() {
                    self.scopes.push(HashMap::new());
                    let r = (|| {
                        match vars.len() {
                            1 => self.bind(&vars[0], x, iter.span)?,
                            _ => {
                                self.bind(&vars[0], Value::Num(Num::konst(k as f64, Dim::Count)), iter.span)?;
                                self.bind(&vars[1], x, iter.span)?;
                            }
                        }
                        if let Some(c) = cond {
                            match self.expr(c)? {
                                Value::Bool(false) => return Ok(None),
                                Value::Bool(true) => {}
                                o => {
                                    return Err(Error::WrongType {
                                        span: c.span,
                                        what: "a filter",
                                        want: Ty::Bool,
                                        found: o.ty(),
                                    });
                                }
                            }
                        }
                        self.expr(item).map(Some)
                    })();
                    self.scopes.pop();
                    if let Some(v) = r? {
                        out.push(v);
                    }
                }
                Ok(Value::List(Rc::new(out)))
            }
            ExprKind::Range { lo, hi, inclusive } => {
                let a = self.num(lo, "a range's start")?;
                let b = self.num(hi, "a range's end")?;
                for (x, sp) in [(&a, lo.span), (&b, hi.span)] {
                    if !matches!(x.dim, Dim::Count | Dim::Any) {
                        return Err(Error::WrongDim {
                            span: sp,
                            what: "a range bound",
                            want: Dim::Count,
                            found: x.dim,
                        });
                    }
                    if !x.is_whole() {
                        return Err(Error::NotWhole { span: sp, value: x.v });
                    }
                }
                let (a0, b0) = (a.v.round() as i64, b.v.round() as i64 + i64::from(*inclusive));
                // a range is control: its bounds are observed (however many times it runs, none
                // too), its values constants, no data's provenance
                self.observe(&a);
                self.observe(&b);
                let v = (a0..b0)
                    .map(|k| Value::Num(Num::konst(k as f64, Dim::Count)))
                    .collect();
                Ok(Value::List(Rc::new(v)))
            }
            ExprKind::Bin(op, a, b) => self.binop(*op, a, b, e.span),
            ExprKind::Neg(a) => match self.expr(a)? {
                Value::Num(n) => Ok(Value::Num(values::neg(&n))),
                Value::Dir(d) => Ok(Value::Dir(-d)),
                Value::WorldDir(d) => Ok(Value::WorldDir(-d)),
                Value::Tuple(t) => {
                    let v = t
                        .iter()
                        .map(|x| match x {
                            Value::Num(n) => Ok(Value::Num(values::neg(n))),
                            o => Err(Error::WrongType {
                                span: a.span,
                                what: "negation",
                                want: Ty::Num,
                                found: o.ty(),
                            }),
                        })
                        .collect::<R<Vec<_>>>()?;
                    Ok(Value::Tuple(Rc::new(v)))
                }
                o => Err(Error::WrongType {
                    span: a.span,
                    what: "negation",
                    want: Ty::Num,
                    found: o.ty(),
                }),
            },
            ExprKind::Not(a) => match self.expr(a)? {
                Value::Bool(b) => Ok(Value::Bool(!b)),
                o => Err(Error::WrongType {
                    span: a.span,
                    what: "not",
                    want: Ty::Bool,
                    found: o.ty(),
                }),
            },
            ExprKind::If { cond, then, other } => match self.expr(cond)? {
                Value::Bool(true) => self.expr(then),
                Value::Bool(false) => self.expr(other),
                o => Err(Error::WrongType {
                    span: cond.span,
                    what: "a condition",
                    want: Ty::Bool,
                    found: o.ty(),
                }),
            },
            ExprKind::Index(o, i) => {
                let ov = self.expr(o)?;
                if matches!(ov, Value::Unresolved) {
                    return Ok(Value::Unresolved);
                }
                let iv = self.num(i, "an index")?;
                self.observe(&iv);
                if !matches!(iv.dim, Dim::Count | Dim::Any) {
                    return Err(Error::WrongDim {
                        span: i.span,
                        what: "an index",
                        want: Dim::Count,
                        found: iv.dim,
                    });
                }
                if let Value::Asm(a) = &ov {
                    return crate::asm::index(a, iv.v.round() as i64, e.span);
                }
                if let Value::Map(m) = &ov {
                    let k = iv.v.round() as i64;
                    return m
                        .iter()
                        .find(|(key, _)| *key == Key::Ix(k))
                        .map(|x| x.1.clone())
                        .ok_or(Error::IndexOut {
                            span: i.span,
                            index: k,
                            len: m.len(),
                        });
                }
                let items = match &ov {
                    Value::List(x) | Value::Tuple(x) => x.clone(),
                    Value::Ent(en) if !en.loop_segs.is_empty() => {
                        let segs = en.loop_segs.clone();
                        let k = iv.v.round() as i64;
                        let Some(&id) = usize::try_from(k).ok().and_then(|k| segs.get(k)) else {
                            return Err(Error::IndexOut {
                                span: i.span,
                                index: k,
                                len: segs.len(),
                            });
                        };
                        let kind = en.sk.borrow().kind(id);
                        return Ok(Value::Ent(Ent {
                            sk: en.sk.clone(),
                            id,
                            kind,
                            loop_segs: Rc::new(vec![]),
                        }));
                    }
                    o => {
                        return Err(Error::WrongType {
                            span: e.span,
                            what: "indexing",
                            want: Ty::List,
                            found: o.ty(),
                        });
                    }
                };
                let k = iv.v.round() as i64;
                let item = usize::try_from(k)
                    .ok()
                    .and_then(|k| items.get(k))
                    .cloned()
                    .ok_or(Error::IndexOut {
                        span: i.span,
                        index: k,
                        len: items.len(),
                    })?;
                // the element as it is: the index is control (observed above), not its data
                Ok(item)
            }
            ExprKind::Attr(o, name) => self.attr(o, name, e.span),
            ExprKind::Call(f, args) => self.call(f, args, e.span),
        }
    }

    pub fn num(&mut self, e: &Expr, what: &'static str) -> R<Num> {
        match self.expr(e)? {
            Value::Num(n) => Ok(n),
            o => Err(Error::WrongType {
                span: e.span,
                what,
                want: Ty::Num,
                found: o.ty(),
            }),
        }
    }

    fn binop(&mut self, op: BinOp, a: &Expr, b: &Expr, span: Span) -> R<Value> {
        if matches!(op, BinOp::And | BinOp::Or) {
            let x = self.expr(a)?;
            let Value::Bool(x) = x else {
                return Err(Error::WrongType {
                    span: a.span,
                    what: op.text(),
                    want: Ty::Bool,
                    found: x.ty(),
                });
            };
            if (op == BinOp::And && !x) || (op == BinOp::Or && x) {
                return Ok(Value::Bool(x));
            }
            return match self.expr(b)? {
                Value::Bool(y) => Ok(Value::Bool(y)),
                o => Err(Error::WrongType {
                    span: b.span,
                    what: op.text(),
                    want: Ty::Bool,
                    found: o.ty(),
                }),
            };
        }
        let x = self.expr(a)?;
        let y = self.expr(b)?;
        if matches!(
            op,
            BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge
        ) {
            self.observe_value(&x);
            self.observe_value(&y);
        }
        arith(op, &x, &y, span, a.span, b.span)
    }

    fn attr(&mut self, o: &Expr, name: &str, span: Span) -> R<Value> {
        // a qualified param in a definition: an explicit link to another feature's param
        if self.ctx.in_param
            && let Some(path) = expr_path(o)
        {
            let key = ParamKey {
                owner: path,
                name: name.into(),
            };
            if let Some(k) = self.locals.iter().position(|p| p.key == key) {
                return Ok(self.param_num((false, k), span));
            }
        }
        let v = self.expr(o)?;
        crate::builtins::member(self, v, name, span)
    }

    fn call(&mut self, f: &Expr, args: &[Arg], span: Span) -> R<Value> {
        // methods of sheet values: `S.edge(entity)`, `U.of(op)`
        if let ExprKind::Attr(o, m) = &f.kind
            && crate::sheet::METHODS.contains(&m.as_str())
            && let Value::Sheet(s) = self.expr(o)?
        {
            return crate::sheet::method(self, &s, m, args, span);
        }
        // a call on an assembly's instance or reference (`yoke.plane(at: boss.end.center, normal:
        // boss.end.normal)`): the part's, its arguments evaluated in the part file
        if let ExprKind::Attr(o, _) = &f.kind
            && self.asm.as_ref().is_some_and(|d| !d.insts.is_empty())
            && let Value::Asm(a) = self.expr(o)?
        {
            let text: String = self.src[o.span.hi as usize..span.hi as usize]
                .trim_start_matches(|c: char| c == '.' || c.is_whitespace())
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
            return crate::asm::part_call_on(&a, text, span);
        }
        // `E.side(entity)`: the face one entity of E's profile made
        if let ExprKind::Attr(o, m) = &f.kind
            && m == "side"
        {
            let ov = self.expr(o)?;
            if let Value::Feature(feat) = &ov
                && feat.prof.is_some()
            {
                let [a] = args else {
                    return Err(Error::TooManyArgs {
                        span,
                        func: "side".into(),
                        max: 1,
                    });
                };
                let ev = self.expr(&a.value)?;
                let Value::Ent(e) = ev else {
                    return Err(Error::WrongType {
                        span: a.value.span,
                        what: "side(entity)",
                        want: Ty::Entity,
                        found: ev.ty(),
                    });
                };
                return crate::builtins::side(self, feat, &e, span);
            }
        }
        let fv = self.expr(f)?;
        match fv {
            Value::Builtin(name) => {
                // an error that comes only from inputs nothing could know (an earlier failure,
                // or `check`, which builds nothing) is no error: the result is unknown too
                let outer = std::mem::replace(&mut self.poisoned, false);
                let r = crate::builtins::call(self, name, args, span);
                let poisoned = std::mem::replace(&mut self.poisoned, outer);
                match r {
                    Err(e)
                        if poisoned
                            && !matches!(e, Error::UnnamedFeature { .. } | Error::DuplicateId { .. }) =>
                    {
                        Ok(Value::Unresolved)
                    }
                    r => r,
                }
            }
            Value::Func(def) => self.call_fn(&def, args, span),
            Value::Asm(a)
                if matches!(
                    &*a,
                    crate::asm::AsmVal::Def(_) | crate::asm::AsmVal::Foreign { .. }
                ) =>
            {
                match &*a {
                    crate::asm::AsmVal::Def(d) => crate::asm::call_def(self, d, args, span),
                    crate::asm::AsmVal::Foreign { name, file } => {
                        crate::asm::call_foreign(self, &name.clone(), &file.clone(), args, span)
                    }
                    _ => unreachable!("matched"),
                }
            }
            Value::Unresolved => Ok(Value::Unresolved),
            o => Err(Error::NotCallable {
                span: f.span,
                found: o.ty(),
            }),
        }
    }

    fn call_fn(&mut self, def: &Rc<FnDef>, args: &[Arg], span: Span) -> R<Value> {
        let Some(lib) = def.lib.as_ref().and_then(|w| w.upgrade()) else {
            let here = self.lib.clone();
            return self.call_fn_here(def, args, span, here);
        };
        // a used file's function: its defaults and body see that file's names; what goes wrong
        // inside is shown at this call, naming the file
        let outer = (self.lib.replace(lib.clone()), self.lib_call);
        if self.lib_call.is_none() {
            self.lib_call = Some(span);
        }
        let r = self.call_fn_here(def, args, span, outer.0.clone());
        (self.lib, self.lib_call) = outer;
        r.map_err(|e| crate::uses::in_file(&lib, span, e))
    }

    /// `caller`: the names the arguments are read in (the caller's used file, or none).
    fn call_fn_here(
        &mut self,
        def: &Rc<FnDef>,
        args: &[Arg],
        span: Span,
        caller: Option<Rc<crate::uses::Lib>>,
    ) -> R<Value> {
        // recursion is allowed; each top-level statement has a budget of calls, and the whole run
        // (every component's evaluation with it) one of its own
        self.calls += 1;
        let (calls, comps) = self.total.get();
        self.total.set((calls + 1, comps));
        if self.calls > CALL_BUDGET || calls + 1 > TOTAL_CALLS || self.depth0 + self.ctx.in_fn >= DEPTH_BUDGET
        {
            return Err(Error::Fuel {
                span,
                func: def.name.clone(),
                budget: CALL_BUDGET,
            });
        }
        if args.len() > def.params.len() {
            return Err(Error::TooManyArgs {
                span,
                func: def.name.clone(),
                max: def.params.len(),
            });
        }
        let mut scope: HashMap<String, (Value, Span)> = HashMap::new();
        for (k, a) in args.iter().enumerate() {
            let pname = match &a.name {
                Some((n, nsp)) => {
                    if !def.params.iter().any(|p| &p.name == n) {
                        return Err(Error::UnknownArg {
                            span: *nsp,
                            func: "a function",
                            arg: n.clone(),
                        });
                    }
                    n.clone()
                }
                None => def.params[k].name.clone(),
            };
            // arguments are the caller's: read in the caller's names
            let inner = std::mem::replace(&mut self.lib, caller.clone());
            let v = self.expr(&a.value);
            self.lib = inner;
            scope.insert(pname, (v?, a.value.span));
        }
        for p in &def.params {
            if !scope.contains_key(&p.name) {
                match &p.default {
                    Some(d) => {
                        let v = self.expr(d)?;
                        scope.insert(p.name.clone(), (v, d.span));
                    }
                    None => {
                        return Err(Error::MissingArg {
                            span,
                            func: "a function",
                            arg: "every parameter without a default",
                        });
                    }
                }
            }
        }
        // a call made by an assignment names the features its body makes
        let named = match self.ctx.pending.take() {
            Some((n, sp)) if sp == span => Some(n),
            other => {
                self.ctx.pending = other;
                None
            }
        };
        let saved = std::mem::take(&mut self.ctx);
        self.ctx.in_fn = saved.in_fn + 1;
        self.ctx.top_stmt = saved.top_stmt;
        self.ctx.prefix = saved.prefix.clone();
        if let Some(n) = &named {
            if saved.loops.iter().any(Option::is_none) {
                self.ctx = saved;
                return Err(Error::PositionalId {
                    span,
                    rule: Rule::KeyedIds,
                });
            }
            self.ctx.prefix.push(Seg {
                name: n.clone(),
                index: saved.loops.iter().flatten().cloned().collect(),
            });
        }
        self.ctx.owner = self.ctx.prefix.clone();
        // a body sees its arguments and the top-level names, nothing local to the caller
        let saved_scopes = self.scopes.split_off(1);
        self.scopes.push(scope);
        let mut r = Ok(());
        for s in def.body.iter() {
            r = self.stmt(s);
            if r.is_err() || self.ret.is_some() {
                break;
            }
        }
        self.scopes.truncate(1);
        self.scopes.extend(saved_scopes);
        self.ctx = saved;
        r?;
        Ok(self.ret.take().unwrap_or(Value::Unresolved))
    }

    // ---- features ----------------------------------------------------------------------------

    /// The id the feature being made takes: the assignment's name, under the call path, with
    /// the loop indices. A feature made anywhere else has no name.
    pub fn take_id(&mut self, kind: &'static str, span: Span) -> R<String> {
        match self.ctx.pending.take() {
            Some((n, sp)) if sp == span => {
                let mut path = self.ctx.prefix.clone();
                path.push(Seg {
                    name: n.clone(),
                    index: self.id_keys(span)?,
                });
                let id = path_text(&path);
                if let Some(first) = self.ids.get(&id) {
                    return Err(Error::DuplicateId {
                        span,
                        id,
                        first: *first,
                    });
                }
                self.ids.insert(id.clone(), span);
                Ok(id)
            }
            other => {
                self.ctx.pending = other;
                if self.rules.named_features == Level::Error {
                    Err(Error::UnnamedFeature {
                        span,
                        kind,
                        rule: Rule::NamedFeatures,
                    })
                } else {
                    let id = format!("{kind}@{}", span.lo);
                    Ok(id)
                }
            }
        }
    }

    /// Adds a made feature to the history and builds it.
    pub fn push_feature(
        &mut self,
        id: &str,
        kind: &'static str,
        f: forge_model::Feature,
        span: Span,
        dims: Vec<Dimension>,
    ) -> R<(usize, u64)> {
        crate::sheet::guard(self, &f, id, span)?;
        let hid = history_id(id);
        self.push_made(id, kind, span, dims, hid, |b| {
            b.push(f, hid).map_err(|error| Error::Build {
                span,
                id: id.into(),
                error,
            })
        })
    }

    /// Adds a feature to the history and builds it by `push`, keeping its record.
    pub fn push_made(
        &mut self,
        id: &str,
        kind: &'static str,
        span: Span,
        dims: Vec<Dimension>,
        hid: u64,
        push: impl FnOnce(&mut crate::build::Builder) -> R<usize>,
    ) -> R<(usize, u64)> {
        let was_building = self.b.building;
        let r = push(&mut self.b);
        let idx = self.b.features.len() - 1;
        let status = match (&r, was_building) {
            (Ok(_), true) => Status::Ok,
            (Ok(_), false) => Status::NotBuilt,
            (Err(_), _) => Status::Failed,
        };
        self.feats.push(FeatInfo {
            id: id.into(),
            kind,
            span,
            idx,
            status,
            dims,
            stmt: self.ctx.top_stmt.unwrap_or(span),
            top_level: self.ctx.prefix.is_empty() && self.ctx.loops.is_empty(),
            reads: std::mem::take(&mut self.reads),
            frame: None,
            mass: None,
            fingerprint: None,
        });
        if self.volumes && status == Status::Ok {
            let v = self
                .b
                .body()
                .and_then(|b| forge_topo::mass::mass(b).ok())
                .map(|m| [m.volume, m.centroid.x, m.centroid.y, m.centroid.z]);
            use std::hash::{Hash, Hasher};
            let mut h = std::collections::hash_map::DefaultHasher::new();
            format!("{:?}", self.b.features[idx]).hash(&mut h);
            let last = self.feats.last_mut().expect("pushed");
            last.mass = v;
            last.fingerprint = Some(h.finish());
        }
        r?;
        if status == Status::Ok && idx > 0 && self.no_effect(idx) {
            self.warnings.push(Warning::NoEffect { span, id: id.into() });
        }
        Ok((idx, hid))
    }

    fn no_effect(&self, idx: usize) -> bool {
        let n = self.b.stages.len();
        if n < 2 || n != idx + 1 || self.b.features[idx].is_param_like() || moves_faces(&self.b.features[idx])
        {
            return false;
        }
        let (a, b) = (&self.b.stages[n - 2], &self.b.stages[n - 1]);
        // the same faces by name, and as many edges: the feature changed nothing
        // (and the corners where they were: a draft or a face offset keeps every name)
        let names = |s: &crate::build::Stage| -> Option<(Vec<u64>, usize, Vec<[i64; 3]>)> {
            let b = s.view()?;
            let mut v: Vec<u64> = b.face_ids().map(|f| b.face(f).name.0).collect();
            v.sort_unstable();
            let q = |x: f64| (x * 1e5).round() as i64;
            let mut c: Vec<[i64; 3]> = b
                .vertex_ids()
                .map(|x| {
                    let p = b.vertex(x).point;
                    [q(p.x), q(p.y), q(p.z)]
                })
                .collect();
            c.sort_unstable();
            Some((v, b.edge_ids().count(), c))
        };
        matches!((names(a), names(b)), (Some(x), Some(y)) if x == y)
    }

    /// The first feature's id, if one exists (world placement is allowed only before it).
    pub fn world_check(&mut self, world: bool, span: Span) -> R<()> {
        if let (true, Some(first)) = (world, self.feats.first()) {
            let err = Error::WorldAfterFirst {
                span,
                first: first.id.clone(),
                rule: Rule::WorldOnlyFirst,
            };
            return self.rule(self.rules.world_only_first, err);
        }
        Ok(())
    }

    pub fn out_of_scope_rule(&mut self, param: ParamKey, span: Span) -> R<()> {
        let err = self.out_of_scope(param, span);
        self.rule(self.rules.scoped_params, err)
    }

    fn sketch_block(&mut self, s: &Stmt, name: &str, args: &[Arg], body: &[Stmt]) -> R<()> {
        if self.sketch.is_some() {
            return Err(Error::WrongPlace {
                span: s.span,
                func: "sketch",
                needs_sketch: false,
            });
        }
        let saved_owner = self.ctx.owner.clone();
        self.ctx.owner = self.here(name);
        self.ctx.pending = Some((name.into(), s.span));
        let r = self.sketch_inner(s, name, args, body);
        self.ctx.owner = saved_owner;
        self.ctx.pending = None;
        let v = r?;
        self.bind(name, v, s.span)
    }

    fn sketch_inner(&mut self, s: &Stmt, _name: &str, args: &[Arg], body: &[Stmt]) -> R<Value> {
        let id = self.take_id("sketch", s.span)?;
        let a = crate::builtins::Args::bind(self, "sketch", &["on", "at"], args, s.span)?;
        let on = a.req("on")?;
        // a plane nothing could know yet (check builds nothing): the rows are still checked
        let known = !crate::builtins::unknown_in(&on);
        let mut frame = if known {
            crate::builtins::plane_of(self, on, a.span("on"), "on")?
        } else {
            Frame::WORLD
        };
        if let Some(at) = a.opt("at").filter(|v| known && !crate::builtins::unknown_in(v)) {
            let p = crate::builtins::point_of(self, at, a.span("at"))?;
            let d = (p - frame.origin).dot(frame.z().get());
            frame = frame.with_origin(p - frame.z().get() * d);
        }
        let sk = Rc::new(RefCell::new(Sk::new(id.clone(), s.span, frame)));
        self.sketch = Some(sk.clone());
        self.scopes.push(HashMap::new());
        let mut r = Ok(());
        for b in body {
            r = self.stmt(b);
            if r.is_err() {
                break;
            }
        }
        let scope = self.scopes.pop().expect("sketch scope");
        self.sketch = None;
        r?;
        {
            let mut skb = sk.borrow_mut();
            let mut named: Vec<(String, Value)> = scope.into_iter().map(|(k, (v, _))| (k, v)).collect();
            named.sort_by(|a, b| a.0.cmp(&b.0));
            skb.named = named;
        }
        sk.borrow_mut().finish()?;
        if sk.borrow().free {
            let ps = sk.borrow().params.clone();
            self.observed.extend(ps);
        }
        {
            let skb = sk.borrow();
            if skb.dof > 0 {
                self.warnings.push(Warning::UnderConstrained {
                    span: s.span,
                    sketch: id.clone(),
                    dof: skb.dof,
                });
            }
        }
        let f = forge_model::Feature::Sketch(Box::new(sk.borrow().model()));
        let (idx, _) = self.push_feature(&id, "sketch", f, s.span, vec![])?;
        sk.borrow_mut().idx = idx;
        let reads = sk.borrow().params.clone();
        let last = self.feats.last_mut().expect("pushed");
        last.frame = Some(sk.borrow().frame);
        last.reads.extend(reads);
        Ok(Value::Sketch(sk))
    }

    fn path_block(&mut self, s: &Stmt, name: &str, args: &[Arg], body: &[Stmt]) -> R<()> {
        let Some(skr) = self.sketch.clone() else {
            return Err(Error::WrongPlace {
                span: s.span,
                func: "path",
                needs_sketch: true,
            });
        };
        let a = crate::builtins::Args::bind(self, "path", &["start", "closed"], args, s.span)?;
        let start = crate::builtins::p2_of(self, a.req("start")?, a.span("start"))?;
        let start = skb_p2(&skr, start, s.span)?;
        self.path = Some(PathState {
            start,
            at: start,
            last: None,
            segs: vec![],
            name: name.into(),
        });
        let mut r = Ok(());
        for b in body {
            r = self.stmt(b);
            if r.is_err() {
                break;
            }
        }
        let st = self.path.take().expect("path state");
        r?;
        let first = st.segs.first().copied().ok_or(Error::BadValue {
            span: s.span,
            what: "the number of path segments",
            value: 0.0,
        })?;
        let v = Value::Ent(Ent {
            sk: skr,
            id: first,
            kind: EntKind::Loop,
            loop_segs: Rc::new(st.segs),
        });
        self.bind(name, v, s.span)
    }

    pub fn sketch_ref(&self) -> Option<Rc<RefCell<Sk>>> {
        self.sketch.clone()
    }

    pub fn path_mut(&mut self) -> Option<&mut PathState> {
        self.path.as_mut()
    }

    pub fn owner(&self) -> &[Seg] {
        &self.ctx.owner
    }
}

fn skb_p2(sk: &Rc<RefCell<Sk>>, p: crate::sketch::P2, span: Span) -> R<EntityId> {
    sk.borrow_mut().p2(p, span)
}

/// A param's value as read by name: the param drives it; the items of a list or tuple param are
/// driven by it (one param, many values: no single handle).
pub(crate) fn driving(v: &Value, key: &ParamKey, span: Span) -> Value {
    let _ = span;
    fn inner(v: &Value, key: &ParamKey) -> Value {
        match v {
            Value::Num(n) => Value::Num(Num {
                v: n.v,
                dim: n.dim,
                src: Src::Driven([key.clone()].into()),
            }),
            Value::List(xs) => Value::List(Rc::new(xs.iter().map(|x| inner(x, key)).collect())),
            Value::Tuple(xs) => Value::Tuple(Rc::new(xs.iter().map(|x| inner(x, key)).collect())),
            Value::Sheet(s) => Value::Sheet(Rc::new(s.driven(key))),
            o => o.clone(),
        }
    }
    match v {
        Value::Num(n) => Value::Num(Num {
            v: n.v,
            dim: n.dim,
            src: Src::Driving(key.clone()),
        }),
        o => inner(o, key),
    }
}

pub fn collect_params(v: &Value, out: &mut BTreeSet<ParamKey>) {
    match v {
        Value::Num(n) => out.extend(n.src.params()),
        Value::List(xs) | Value::Tuple(xs) => xs.iter().for_each(|x| collect_params(x, out)),
        Value::Map(m) => m.iter().for_each(|x| collect_params(&x.1, out)),
        Value::Sheet(s) => s.params(out),
        _ => {}
    }
}

/// `a.b[2].c` as a feature path, if the expression is one.
fn expr_path(e: &Expr) -> Option<Vec<Seg>> {
    match &e.kind {
        ExprKind::Name(n) => Some(vec![Seg {
            name: n.clone(),
            index: vec![],
        }]),
        ExprKind::Attr(o, n) => {
            let mut p = expr_path(o)?;
            p.push(Seg {
                name: n.clone(),
                index: vec![],
            });
            Some(p)
        }
        ExprKind::Index(o, i) => {
            let mut p = expr_path(o)?;
            let ExprKind::Num(v, Unit::None) = i.kind else {
                return None;
            };
            p.last_mut()?.index.push(Key::Ix(v as i64));
            Some(p)
        }
        _ => None,
    }
}

/// A param's value with every bare number in it made a Length.
fn settle(v: Value) -> Value {
    match v {
        Value::Num(n) if n.dim == Dim::Any => Value::Num(Num {
            dim: Dim::Length,
            ..n
        }),
        Value::List(xs) => Value::List(Rc::new(xs.iter().cloned().map(settle).collect())),
        Value::Tuple(xs) => Value::Tuple(Rc::new(xs.iter().cloned().map(settle).collect())),
        Value::Map(m) => Value::Map(Rc::new(
            m.iter().map(|(k, x)| (k.clone(), settle(x.clone()))).collect(),
        )),
        o => o,
    }
}

pub fn arith(op: BinOp, x: &Value, y: &Value, span: Span, sa: Span, sb: Span) -> R<Value> {
    let wrong = |sp: Span, v: &Value| Error::WrongType {
        span: sp,
        what: op.text(),
        want: Ty::Num,
        found: v.ty(),
    };
    match (x, y) {
        (Value::Num(a), Value::Num(b)) => {
            let n = match op {
                BinOp::Add => values::add(a, b, false, span)?,
                BinOp::Sub => values::add(a, b, true, span)?,
                BinOp::Mul => values::mul(a, b, span)?,
                BinOp::Div => values::div(a, b, span)?,
                BinOp::Pow => values::pow(a, b, span)?,
                _ => {
                    values::add(a, b, true, span)?;
                    let r = match op {
                        BinOp::Eq => (a.v - b.v).abs() < 1e-9,
                        BinOp::Ne => (a.v - b.v).abs() >= 1e-9,
                        BinOp::Lt => a.v < b.v,
                        BinOp::Le => a.v <= b.v,
                        BinOp::Gt => a.v > b.v,
                        _ => a.v >= b.v,
                    };
                    return Ok(Value::Bool(r));
                }
            };
            Ok(Value::Num(n))
        }
        // vectors: elementwise with vectors, scaled by numbers
        (Value::Tuple(a), Value::Tuple(b)) if matches!(op, BinOp::Add | BinOp::Sub) && a.len() == b.len() => {
            let v = a
                .iter()
                .zip(b.iter())
                .map(|(p, q)| arith(op, p, q, span, sa, sb))
                .collect::<R<Vec<_>>>()?;
            Ok(Value::Tuple(Rc::new(v)))
        }
        (Value::Tuple(a), Value::Num(_)) if matches!(op, BinOp::Mul | BinOp::Div) => {
            let v = a
                .iter()
                .map(|p| arith(op, p, y, span, sa, sb))
                .collect::<R<Vec<_>>>()?;
            Ok(Value::Tuple(Rc::new(v)))
        }
        (Value::Num(_), Value::Tuple(b)) if op == BinOp::Mul => {
            let v = b
                .iter()
                .map(|q| arith(op, x, q, span, sa, sb))
                .collect::<R<Vec<_>>>()?;
            Ok(Value::Tuple(Rc::new(v)))
        }
        (Value::Dir(d), Value::Num(n)) if op == BinOp::Mul => Ok(Value::Dir(*d * n.v)),
        (Value::List(a), Value::List(b)) if op == BinOp::Add => {
            let mut v = (**a).clone();
            v.extend(b.iter().cloned());
            Ok(Value::List(Rc::new(v)))
        }
        (Value::Faces(a), Value::Faces(b)) if op == BinOp::Add => {
            let mut names = a.names.clone();
            for n in &b.names {
                if !names.contains(n) {
                    names.push(*n);
                }
            }
            Ok(Value::Faces(Rc::new(Faces {
                names,
                owner: format!("{} + {}", a.owner, b.owner),
                role: crate::error::Role::Faces,
                plane: None,
            })))
        }
        (Value::Edges(a), Value::Edges(b)) if op == BinOp::Add => {
            let mut v = (**a).clone();
            for n in b.iter() {
                if !v.contains(n) {
                    v.push(*n);
                }
            }
            Ok(Value::Edges(Rc::new(v)))
        }
        (Value::Unresolved, _) | (_, Value::Unresolved) => Ok(Value::Unresolved),
        (Value::Num(_), o) => Err(wrong(sb, o)),
        (o, _) => Err(wrong(sa, o)),
    }
}

trait ParamLike {
    fn is_param_like(&self) -> bool;
}

impl ParamLike for forge_model::Feature {
    fn is_param_like(&self) -> bool {
        matches!(
            self,
            forge_model::Feature::Sketch(_)
                | forge_model::Feature::Param(_)
                | forge_model::Feature::Plane(_)
                | forge_model::Feature::Work(_)
        )
    }
}

/// A feature that moves faces it keeps the names of (a direct edit, a body moved): names alone
/// cannot say it changed nothing.
fn moves_faces(f: &forge_model::Feature) -> bool {
    use forge_model::{BodyFeature, Feature};
    match f {
        Feature::Edit(_) => true,
        Feature::Body(b) => match &**b {
            BodyFeature::Move { .. } => true,
            BodyFeature::In { feature, .. } => moves_faces(feature),
            _ => false,
        },
        _ => false,
    }
}

pub fn unit_or_z(v: Vec3) -> UnitVec3 {
    v.normalize().unwrap_or(UnitVec3::Z)
}

/// The local names statements bind, with where: function parameters and bodies, loop and
/// comprehension variables, a sketch's or path's names. A loop at top level binds top-level names.
fn locals_of(stmts: &[Stmt], top: bool, out: &mut Vec<(String, Span)>) {
    fn comp(e: &Expr, out: &mut Vec<(String, Span)>) {
        match &e.kind {
            ExprKind::Comp {
                item,
                vars,
                iter,
                cond,
            } => {
                out.extend(vars.iter().map(|v| (v.clone(), e.span)));
                comp(item, out);
                comp(iter, out);
                if let Some(c) = cond {
                    comp(c, out);
                }
            }
            ExprKind::List(xs) | ExprKind::Tuple(xs) => xs.iter().for_each(|x| comp(x, out)),
            ExprKind::Map(xs) => xs.iter().for_each(|x| comp(&x.1, out)),
            ExprKind::Bin(_, a, b) | ExprKind::Index(a, b) | ExprKind::Range { lo: a, hi: b, .. } => {
                comp(a, out);
                comp(b, out);
            }
            ExprKind::Neg(a) | ExprKind::Not(a) | ExprKind::Attr(a, _) => comp(a, out),
            ExprKind::Call(f, args) => {
                comp(f, out);
                args.iter().for_each(|a| comp(&a.value, out));
            }
            ExprKind::If { cond, then, other } => {
                comp(cond, out);
                comp(then, out);
                comp(other, out);
            }
            _ => {}
        }
    }
    for s in stmts {
        match &s.kind {
            StmtKind::Assign { name, value } => {
                if !top {
                    out.push((name.clone(), s.span));
                }
                comp(value, out);
            }
            StmtKind::Expr(v)
            | StmtKind::Return(v)
            | StmtKind::Shared { value: v, .. }
            | StmtKind::Param { value: v, .. } => comp(v, out),
            // a component's names are its own document's
            StmtKind::Block {
                kind: BlockKind::Component,
                ..
            } => {}
            StmtKind::Block { name, args, body, .. } => {
                if !top {
                    out.push((name.clone(), s.span));
                }
                args.iter().for_each(|a| comp(&a.value, out));
                locals_of(body, false, out);
            }
            StmtKind::For { vars, iter, body } => {
                out.extend(vars.iter().map(|v| (v.clone(), s.span)));
                comp(iter, out);
                locals_of(body, top, out);
            }
            StmtKind::If { cond, then, other } => {
                comp(cond, out);
                locals_of(then, top, out);
                locals_of(other.as_deref().unwrap_or(&[]), top, out);
            }
            StmtKind::Fn { params, body, .. } => {
                out.extend(params.iter().map(|p| (p.name.clone(), s.span)));
                locals_of(body, false, out);
            }
            StmtKind::Expect { .. } => {}
        }
    }
}

/// A name a loop body bound: its value at each key, and where it was bound.
type Collected = (String, Vec<(Option<Key>, Value)>, Span);
