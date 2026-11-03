//! Typed errors, warnings and the edit operations that fix them. Nothing here is a free-text
//! message: every variant carries a span and the data a caller needs, and every error lists the
//! typed edits (`Op`) that would remove it. The text an agent reads is rendered from the data.

use crate::syntax::{Key, Seg, Span, Unit};
use crate::values::{Dim, Ty};
use forge_agent::json::Json;

/// A param: shared (no owner) or local to the feature path `owner` (`boss`, `lug.hub`, `bolt[2]`).
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ParamKey {
    pub owner: Vec<Seg>,
    pub name: String,
}

impl PartialOrd for Seg {
    fn partial_cmp(&self, o: &Seg) -> Option<std::cmp::Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for Seg {
    fn cmp(&self, o: &Seg) -> std::cmp::Ordering {
        (&self.name, &self.index).cmp(&(&o.name, &o.index))
    }
}

pub fn path_text(p: &[Seg]) -> String {
    p.iter()
        .map(|s| {
            if s.index.is_empty() {
                s.name.clone()
            } else {
                let i: Vec<String> = s.index.iter().map(Key::to_string).collect();
                format!("{}[{}]", s.name, i.join(","))
            }
        })
        .collect::<Vec<_>>()
        .join(".")
}

impl std::fmt::Display for ParamKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.owner.is_empty() {
            write!(f, "{}", self.name)
        } else {
            write!(f, "{}.{}", path_text(&self.owner), self.name)
        }
    }
}

impl ParamKey {
    pub fn shared(name: &str) -> ParamKey {
        ParamKey {
            owner: vec![],
            name: name.into(),
        }
    }
    pub fn is_shared(&self) -> bool {
        self.owner.is_empty()
    }
    /// Parses `wall`, `boss.d`, `bolt[2].d`, `lug.hub.r`.
    pub fn parse(s: &str) -> Option<ParamKey> {
        let mut segs = parse_path(s)?;
        let last = segs.pop()?;
        if !last.index.is_empty() {
            return None;
        }
        Some(ParamKey {
            owner: segs,
            name: last.name,
        })
    }
}

/// `bolt[2].hub` as segments.
pub fn parse_path(s: &str) -> Option<Vec<Seg>> {
    let mut out = Vec::new();
    for part in s.split('.') {
        let (name, index) = match part.split_once('[') {
            Some((n, rest)) => {
                let inner = rest.strip_suffix(']')?;
                let idx: Vec<Key> = inner
                    .split(',')
                    .map(|x| {
                        let x = x.trim();
                        x.parse()
                            .map(Key::Ix)
                            .unwrap_or_else(|_| Key::Label(x.to_string()))
                    })
                    .collect();
                (n, idx)
            }
            None => (part, vec![]),
        };
        if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return None;
        }
        out.push(Seg {
            name: name.into(),
            index,
        });
    }
    Some(out)
}

/// A literal value as a param definition writes it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Literal {
    pub value: f64,
    pub dim: Dim,
}

impl Literal {
    pub fn text(&self) -> String {
        let v = crate::values::fmt(self.value);
        match self.dim {
            Dim::Angle => format!("{v}deg"),
            Dim::Count => format!("count({v})"),
            Dim::Ratio => format!("ratio({v})"),
            _ => v,
        }
    }
}

/// Where a lifted literal's new param lives.
#[derive(Clone, Debug, PartialEq)]
pub enum Scope {
    Shared,
    Local(Vec<Seg>),
}

/// A typed edit of a document. Each is a pure function of (document, op): it changes one
/// defined span (or inserts at one point) and leaves every other byte as it was.
#[derive(Clone, Debug, PartialEq)]
pub enum Op {
    /// The param's whole right-hand side becomes this literal.
    SetParam { param: ParamKey, value: Literal },
    /// `param FEATURE.name = value`, put in above the feature's statement.
    AddLocalParam {
        feature: Vec<Seg>,
        name: String,
        value: Literal,
    },
    /// `param F.x = v` becomes `shared x = v`; uses inside F read the shared param by the same name.
    MakeShared { param: ParamKey },
    /// `param bolt[2].d = <bolt.d's value>` below `param bolt.d`: that instance then has its own.
    SplitInstance { id: Vec<Seg>, param: ParamKey },
    /// A statement put in above the statement making `before` (None: at the end).
    AddFeature { before: Option<String>, text: String },
    /// The statement making `id` taken out.
    DeleteFeature { id: String },
    /// The statement making `id` moved above the one making `before` (None: to the end).
    MoveFeature { id: String, before: Option<String> },
    /// A loose number replaced by a new param's name, the param defined above.
    LiftLiteral { span: Span, name: String, scope: Scope },
}

impl Op {
    pub fn json(&self) -> Json {
        let lit = |l: &Literal| Json::obj(vec![("value", l.value.into()), ("dim", l.dim.name().into())]);
        let path = |p: &[Seg]| Json::Str(path_text(p));
        let opt = |s: &Option<String>| s.clone().map_or(Json::Null, Json::Str);
        match self {
            Op::SetParam { param, value } => Json::obj(vec![
                ("op", "SetParam".into()),
                ("param", param.to_string().into()),
                ("value", lit(value)),
            ]),
            Op::AddLocalParam { feature, name, value } => Json::obj(vec![
                ("op", "AddLocalParam".into()),
                ("feature", path(feature)),
                ("name", name.as_str().into()),
                ("value", lit(value)),
            ]),
            Op::MakeShared { param } => Json::obj(vec![
                ("op", "MakeShared".into()),
                ("param", param.to_string().into()),
            ]),
            Op::SplitInstance { id, param } => Json::obj(vec![
                ("op", "SplitInstance".into()),
                ("id", path(id)),
                ("param", param.to_string().into()),
            ]),
            Op::AddFeature { before, text } => Json::obj(vec![
                ("op", "AddFeature".into()),
                ("before", opt(before)),
                ("text", text.as_str().into()),
            ]),
            Op::DeleteFeature { id } => {
                Json::obj(vec![("op", "DeleteFeature".into()), ("id", id.as_str().into())])
            }
            Op::MoveFeature { id, before } => Json::obj(vec![
                ("op", "MoveFeature".into()),
                ("id", id.as_str().into()),
                ("before", opt(before)),
            ]),
            Op::LiftLiteral { span, name, scope } => Json::obj(vec![
                ("op", "LiftLiteral".into()),
                ("span", span_json(*span)),
                ("name", name.as_str().into()),
                (
                    "scope",
                    match scope {
                        Scope::Shared => "shared".into(),
                        Scope::Local(p) => path(p),
                    },
                ),
            ]),
        }
    }

    /// Reads an op from its JSON form (what `json` writes).
    pub fn from_json(j: &Json) -> Option<Op> {
        let s = |k: &str| j.get(k).and_then(Json::str).map(str::to_string);
        let lit = |k: &str| {
            let v = j.get(k)?;
            Some(Literal {
                value: v.get("value")?.num()?,
                dim: Dim::from_name(v.get("dim")?.str()?)?,
            })
        };
        Some(match s("op")?.as_str() {
            "SetParam" => Op::SetParam {
                param: ParamKey::parse(&s("param")?)?,
                value: lit("value")?,
            },
            "AddLocalParam" => Op::AddLocalParam {
                feature: parse_path(&s("feature")?)?,
                name: s("name")?,
                value: lit("value")?,
            },
            "MakeShared" => Op::MakeShared {
                param: ParamKey::parse(&s("param")?)?,
            },
            "SplitInstance" => Op::SplitInstance {
                id: parse_path(&s("id")?)?,
                param: ParamKey::parse(&s("param")?)?,
            },
            "AddFeature" => Op::AddFeature {
                before: s("before"),
                text: s("text")?,
            },
            "DeleteFeature" => Op::DeleteFeature { id: s("id")? },
            "MoveFeature" => Op::MoveFeature {
                id: s("id")?,
                before: s("before"),
            },
            "LiftLiteral" => {
                let sp = j.get("span")?;
                let span = Span {
                    lo: sp.get("lo")?.num()? as u32,
                    hi: sp.get("hi")?.num()? as u32,
                };
                let scope = match s("scope")?.as_str() {
                    "shared" => Scope::Shared,
                    p => Scope::Local(parse_path(p)?),
                };
                Op::LiftLiteral {
                    span,
                    name: s("name")?,
                    scope,
                }
            }
            _ => return None,
        })
    }
}

pub fn span_json(s: Span) -> Json {
    Json::obj(vec![("lo", (s.lo as f64).into()), ("hi", (s.hi as f64).into())])
}

#[derive(Clone, Debug, PartialEq)]
pub enum SyntaxError {
    BadNumber { span: Span },
    UnknownUnit { span: Span },
    OpenString { span: Span },
    BadChar { span: Span },
    Expected { what: &'static str, span: Span },
    ParamWithoutOwner { span: Span, name: String },
    AssignToNonName { span: Span },
    KeywordNotName { span: Span },
    PositionalAfterKeyword { span: Span },
}

/// The static rules. Each can be switched off (`Rules`), so loosening one is a small change.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rule {
    /// Numbers live only in `shared` and `param` definitions.
    LiteralsInParams,
    /// A local param is seen only by its feature and what that makes.
    ScopedParams,
    /// World planes and coordinates only for the first feature.
    WorldOnlyFirst,
    /// No function calls itself, directly or through others.
    NoRecursion,
    /// Every feature is named by an assignment.
    NamedFeatures,
    /// Loop instances are keyed by a range's index or a keyed loop's label, never a list position.
    KeyedIds,
    /// No selection by where things lie (extreme along a direction).
    NoPositionalSelection,
    /// No name is bound twice, in any enclosing scope.
    SingleAssignment,
}

impl Rule {
    pub fn name(self) -> &'static str {
        match self {
            Rule::LiteralsInParams => "literals-in-params",
            Rule::ScopedParams => "scoped-params",
            Rule::WorldOnlyFirst => "world-only-first",
            Rule::NoRecursion => "no-recursion",
            Rule::NamedFeatures => "named-features",
            Rule::KeyedIds => "keyed-ids",
            Rule::NoPositionalSelection => "no-positional-selection",
            Rule::SingleAssignment => "single-assignment",
        }
    }
}

/// A sketch entity's kind, for errors about what a constraint takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntKind {
    Point,
    Line,
    Circle,
    Arc,
    Spline,
    Loop,
}

/// What an expect line measures.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Metric {
    Volume,
    Area,
    Faces,
    Edges,
    Bodies,
    Valid,
    Mass,
    BboxSize,
}

impl Metric {
    pub fn name(self) -> &'static str {
        match self {
            Metric::Volume => "volume",
            Metric::Area => "area",
            Metric::Faces => "faces",
            Metric::Edges => "edges",
            Metric::Bodies => "bodies",
            Metric::Valid => "valid",
            Metric::Mass => "mass",
            Metric::BboxSize => "bbox",
        }
    }
}

/// An anchor role a feature exports.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Start,
    End,
    Side,
    Axis,
    Faces,
    Edges,
    Floor,
    Wall,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Error {
    Syntax(SyntaxError),
    /// A static rule broken.
    LooseLiteral {
        span: Span,
        value: f64,
        unit: Unit,
        owner: Option<Vec<Seg>>,
        rule: Rule,
    },
    WorldAfterFirst {
        span: Span,
        first: String,
        rule: Rule,
    },
    Recursion {
        span: Span,
        func: String,
        rule: Rule,
    },
    UnnamedFeature {
        span: Span,
        kind: &'static str,
        rule: Rule,
    },
    /// A feature or entity made in a loop over a plain list: its position is no id.
    PositionalId {
        span: Span,
        rule: Rule,
    },
    /// A selection by where things lie, which an unrelated edit can retarget.
    PositionalSelection {
        span: Span,
        rule: Rule,
    },
    /// A function that ran out of its call budget (recursion allowed).
    Fuel {
        span: Span,
        func: String,
        budget: usize,
    },
    ParamOutOfScope {
        span: Span,
        param: ParamKey,
        used_in: Vec<Seg>,
        value: Literal,
        rule: Rule,
    },
    /// A type error: the value is not of the type the place needs.
    WrongType {
        span: Span,
        what: &'static str,
        want: Ty,
        found: Ty,
    },
    /// Two dimensions that do not combine (a Length plus an Angle, a size as a count).
    DimMismatch {
        span: Span,
        op: &'static str,
        left: Dim,
        right: Dim,
    },
    WrongDim {
        span: Span,
        what: &'static str,
        want: Dim,
        found: Dim,
    },
    NotWhole {
        span: Span,
        value: f64,
    },
    BadValue {
        span: Span,
        what: &'static str,
        value: f64,
    },
    UnknownName {
        span: Span,
        name: String,
    },
    UnknownMember {
        span: Span,
        ty: Ty,
        member: String,
    },
    UnknownArg {
        span: Span,
        func: &'static str,
        arg: String,
    },
    MissingArg {
        span: Span,
        func: &'static str,
        arg: &'static str,
    },
    TooManyArgs {
        span: Span,
        func: String,
        max: usize,
    },
    NotCallable {
        span: Span,
        found: Ty,
    },
    Rebind {
        span: Span,
        name: String,
        first: Span,
    },
    /// A param or name that would hide a built-in function or constant.
    ShadowsBuiltin {
        span: Span,
        name: String,
    },
    ParamTwice {
        span: Span,
        param: ParamKey,
        first: Span,
    },
    DuplicateId {
        span: Span,
        id: String,
        first: Span,
    },
    IndexOut {
        span: Span,
        index: i64,
        len: usize,
    },
    ReturnOutsideFn {
        span: Span,
    },
    /// A sketch-only call outside a sketch block, or a 3D one inside.
    WrongPlace {
        span: Span,
        func: &'static str,
        needs_sketch: bool,
    },
    OtherSketch {
        span: Span,
        sketch: String,
        here: String,
    },
    WrongEntity {
        span: Span,
        want: EntKind,
        found: EntKind,
    },
    /// The sketch's regions: none, or several when the feature names none.
    Regions {
        span: Span,
        sketch: String,
        count: usize,
    },
    SketchConflict {
        span: Span,
        sketch: String,
        rows: Vec<Span>,
    },
    SketchNotSolved {
        span: Span,
        sketch: String,
        residual: f64,
    },
    SketchKernel {
        span: Span,
        sketch: String,
        error: forge_sketch::SketchError,
    },
    SketchProfile {
        span: Span,
        sketch: String,
        error: forge_sketch::ExtractError,
    },
    /// The kernel refused the feature.
    Build {
        span: Span,
        id: String,
        error: forge_model::RegenError,
    },
    /// A role anchor whose faces the part no longer has.
    NoFace {
        span: Span,
        id: String,
        role: Role,
    },
    NotPlanar {
        span: Span,
        id: String,
        role: Role,
    },
    /// A selection that picked nothing.
    EmptySelection {
        span: Span,
    },
    /// An expect line the part fails.
    ExpectFailed {
        span: Span,
        metric: Metric,
        measured: f64,
        cmp: crate::syntax::Cmp,
        limit: f64,
        tol: f64,
    },
    UnknownMetric {
        span: Span,
        metric: String,
    },
    /// An expect line whose limit is the measured value to the last digit: copied from a run, so
    /// it checks nothing the author meant.
    ExpectCopied {
        span: Span,
        metric: Metric,
        value: f64,
    },
    /// An edit op naming a param or feature the document lacks, or a name already taken.
    UnknownParam {
        param: ParamKey,
    },
    UnknownFeature {
        id: String,
    },
    NotAStatement {
        id: String,
    },
    NameTaken {
        name: String,
    },
    /// Adding, deleting or moving statements would change what a reader reads: `reader` (the
    /// statement at `span`) mentions `name`.
    StillRead {
        name: String,
        reader: String,
        span: Span,
    },
    /// A function's local, a loop variable or a sketch's entity named as a top-level name.
    ShadowsTopLevel {
        span: Span,
        name: String,
    },
    /// The op's value has another dimension than the param it sets.
    EditDim {
        param: ParamKey,
        want: Dim,
        found: Dim,
    },
    /// Sheet metal: the kernel's refusals and the language's own (`sheet.rs`).
    Sheet(Box<crate::sheet::SheetErr>),
}

impl Error {
    pub fn span(&self) -> Option<Span> {
        use Error::*;
        match self {
            Syntax(s) => Some(match s {
                SyntaxError::BadNumber { span }
                | SyntaxError::UnknownUnit { span }
                | SyntaxError::OpenString { span }
                | SyntaxError::BadChar { span }
                | SyntaxError::Expected { span, .. }
                | SyntaxError::ParamWithoutOwner { span, .. }
                | SyntaxError::AssignToNonName { span }
                | SyntaxError::KeywordNotName { span }
                | SyntaxError::PositionalAfterKeyword { span } => *span,
            }),
            LooseLiteral { span, .. }
            | WorldAfterFirst { span, .. }
            | Recursion { span, .. }
            | UnnamedFeature { span, .. }
            | PositionalId { span, .. }
            | PositionalSelection { span, .. }
            | Fuel { span, .. }
            | ParamOutOfScope { span, .. }
            | WrongType { span, .. }
            | DimMismatch { span, .. }
            | WrongDim { span, .. }
            | NotWhole { span, .. }
            | BadValue { span, .. }
            | UnknownName { span, .. }
            | UnknownMember { span, .. }
            | UnknownArg { span, .. }
            | MissingArg { span, .. }
            | TooManyArgs { span, .. }
            | NotCallable { span, .. }
            | Rebind { span, .. }
            | ShadowsBuiltin { span, .. }
            | ParamTwice { span, .. }
            | DuplicateId { span, .. }
            | IndexOut { span, .. }
            | ReturnOutsideFn { span }
            | WrongPlace { span, .. }
            | OtherSketch { span, .. }
            | WrongEntity { span, .. }
            | Regions { span, .. }
            | SketchConflict { span, .. }
            | SketchNotSolved { span, .. }
            | SketchKernel { span, .. }
            | SketchProfile { span, .. }
            | Build { span, .. }
            | NoFace { span, .. }
            | NotPlanar { span, .. }
            | EmptySelection { span }
            | ExpectFailed { span, .. }
            | UnknownMetric { span, .. }
            | ExpectCopied { span, .. }
            | StillRead { span, .. }
            | ShadowsTopLevel { span, .. } => Some(*span),
            UnknownParam { .. }
            | UnknownFeature { .. }
            | NotAStatement { .. }
            | NameTaken { .. }
            | EditDim { .. } => None,
            Sheet(e) => e.span(),
        }
    }

    /// The variant's name: what tests and callers match on.
    pub fn kind(&self) -> &'static str {
        use Error::*;
        match self {
            Syntax(_) => "Syntax",
            LooseLiteral { .. } => "LooseLiteral",
            WorldAfterFirst { .. } => "WorldAfterFirst",
            Recursion { .. } => "Recursion",
            UnnamedFeature { .. } => "UnnamedFeature",
            PositionalId { .. } => "PositionalId",
            PositionalSelection { .. } => "PositionalSelection",
            Fuel { .. } => "Fuel",
            ParamOutOfScope { .. } => "ParamOutOfScope",
            WrongType { .. } => "WrongType",
            DimMismatch { .. } => "DimMismatch",
            WrongDim { .. } => "WrongDim",
            NotWhole { .. } => "NotWhole",
            BadValue { .. } => "BadValue",
            UnknownName { .. } => "UnknownName",
            UnknownMember { .. } => "UnknownMember",
            UnknownArg { .. } => "UnknownArg",
            MissingArg { .. } => "MissingArg",
            TooManyArgs { .. } => "TooManyArgs",
            NotCallable { .. } => "NotCallable",
            Rebind { .. } => "Rebind",
            ShadowsBuiltin { .. } => "ShadowsBuiltin",
            ParamTwice { .. } => "ParamTwice",
            DuplicateId { .. } => "DuplicateId",
            IndexOut { .. } => "IndexOut",
            ReturnOutsideFn { .. } => "ReturnOutsideFn",
            WrongPlace { .. } => "WrongPlace",
            OtherSketch { .. } => "OtherSketch",
            WrongEntity { .. } => "WrongEntity",
            Regions { .. } => "Regions",
            SketchConflict { .. } => "SketchConflict",
            SketchNotSolved { .. } => "SketchNotSolved",
            SketchKernel { .. } => "SketchKernel",
            SketchProfile { .. } => "SketchProfile",
            Build { .. } => "Build",
            NoFace { .. } => "NoFace",
            NotPlanar { .. } => "NotPlanar",
            EmptySelection { .. } => "EmptySelection",
            ExpectFailed { .. } => "ExpectFailed",
            UnknownMetric { .. } => "UnknownMetric",
            ExpectCopied { .. } => "ExpectCopied",
            UnknownParam { .. } => "UnknownParam",
            UnknownFeature { .. } => "UnknownFeature",
            NotAStatement { .. } => "NotAStatement",
            NameTaken { .. } => "NameTaken",
            StillRead { .. } => "StillRead",
            ShadowsTopLevel { .. } => "ShadowsTopLevel",
            EditDim { .. } => "EditDim",
            Sheet(e) => e.kind(),
        }
    }

    /// The typed edits that would remove this error (possibly none).
    pub fn fixes(&self) -> Vec<Op> {
        use Error::*;
        match self {
            LooseLiteral {
                span, owner, value, ..
            } => {
                let name = format!("v{}", crate::values::fmt(*value).replace(['.', '-'], "_"));
                let mut out = vec![];
                if let Some(o) = owner {
                    out.push(Op::LiftLiteral {
                        span: *span,
                        name: name.clone(),
                        scope: Scope::Local(o.clone()),
                    });
                }
                out.push(Op::LiftLiteral {
                    span: *span,
                    name,
                    scope: Scope::Shared,
                });
                out
            }
            ParamOutOfScope {
                param,
                used_in,
                value,
                ..
            } => vec![
                Op::AddLocalParam {
                    feature: used_in.clone(),
                    name: param.name.clone(),
                    value: *value,
                },
                Op::MakeShared { param: param.clone() },
            ],
            Sheet(e) => e.fixes(),
            _ => vec![],
        }
    }

    /// A one-line rendering of the data, for people and agents reading text.
    pub fn text(&self) -> String {
        use Error::*;
        match self {
            Syntax(s) => match s {
                SyntaxError::BadNumber { .. } => "a malformed number".into(),
                SyntaxError::UnknownUnit { .. } => "unknown unit (mm, deg)".into(),
                SyntaxError::OpenString { .. } => "a string not closed".into(),
                SyntaxError::BadChar { .. } => "a character Forge does not use".into(),
                SyntaxError::Expected { what, .. } => format!("expected {what}"),
                SyntaxError::ParamWithoutOwner { name, .. } => {
                    format!("param {name} has no owner: `param FEATURE.{name} = ..` or `shared {name} = ..`")
                }
                SyntaxError::AssignToNonName { .. } => "only a name can be assigned".into(),
                SyntaxError::KeywordNotName { .. } => "a keyword argument is a name".into(),
                SyntaxError::PositionalAfterKeyword { .. } => {
                    "a positional argument after a keyword one".into()
                }
            },
            LooseLiteral { value, rule, .. } => format!(
                "loose number {}: numbers live in shared/param definitions [{}]",
                crate::values::fmt(*value),
                rule.name()
            ),
            WorldAfterFirst { first, rule, .. } => {
                format!(
                    "world placement after the first feature ({first}); place on an anchor [{}]",
                    rule.name()
                )
            }
            Recursion { func, rule, .. } => format!("{func} calls itself [{}]", rule.name()),
            PositionalId { rule, .. } => format!(
                "made in a loop over a plain list: loop over a range (ids name[i]) or a keyed map {{a: .., b: ..}} (ids name[a]) [{}]",
                rule.name()
            ),
            PositionalSelection { rule, .. } => format!(
                "selection by position (extreme) retargets when unrelated features move; select by role or between faces [{}]",
                rule.name()
            ),
            Fuel { func, budget, .. } => format!("{func} ran past its budget of {budget} calls"),
            UnnamedFeature { kind, rule, .. } => {
                format!(
                    "a {kind} feature needs a name: `name = {kind}(..)` [{}]",
                    rule.name()
                )
            }
            ParamOutOfScope {
                param, used_in, rule, ..
            } => format!(
                "{param} is local to {}, used in {} [{}]",
                path_text(&param.owner),
                if used_in.is_empty() {
                    "top level".into()
                } else {
                    path_text(used_in)
                },
                rule.name()
            ),
            WrongType {
                what, want, found, ..
            } => format!("{what}: needs {want:?}, found {found:?}"),
            DimMismatch { op, left, right, .. } => format!("{left:?} {op} {right:?}: dimensions differ"),
            WrongDim {
                what, want, found, ..
            } => format!("{what}: needs a {want:?}, found a {found:?}"),
            NotWhole { value, .. } => format!("{value} is not a whole number"),
            BadValue { what, value, .. } => format!("{what} cannot be {}", crate::values::fmt(*value)),
            UnknownName { name, .. } => format!("unknown name {name}"),
            UnknownMember { ty, member, .. } => format!("{ty:?} has no member {member}"),
            UnknownArg { func, arg, .. } => format!("{func} takes no argument {arg}"),
            MissingArg { func, arg, .. } => format!("{func} needs {arg}"),
            TooManyArgs { func, max, .. } => format!("{func} takes at most {max} arguments"),
            NotCallable { found, .. } => format!("a {found:?} is not a function"),
            Rebind { name, .. } => format!("{name} is already defined"),
            ShadowsBuiltin { name, .. } => {
                format!("{name} would hide the built-in {name}; pick another name")
            }
            ParamTwice { param, .. } => format!("{param} is defined twice"),
            DuplicateId { id, .. } => format!("feature id {id} is made twice"),
            IndexOut { index, len, .. } => format!("index {index} out of 0..{len}"),
            ReturnOutsideFn { .. } => "return outside a function".into(),
            WrongPlace {
                func, needs_sketch, ..
            } => {
                if *needs_sketch {
                    format!("{func} belongs in a sketch block")
                } else {
                    format!("{func} does not belong in a sketch block")
                }
            }
            OtherSketch { sketch, here, .. } => format!("a point of sketch {sketch} used in sketch {here}"),
            WrongEntity { want, found, .. } => format!("needs a {want:?}, found a {found:?}"),
            Regions { sketch, count, .. } => format!("sketch {sketch} has {count} regions; name one"),
            SketchConflict { sketch, rows, .. } => format!("sketch {sketch}: {} rows conflict", rows.len()),
            SketchNotSolved { sketch, residual, .. } => {
                format!("sketch {sketch} does not solve ({residual:e})")
            }
            SketchKernel { sketch, error, .. } => format!("sketch {sketch}: {error:?}"),
            SketchProfile { sketch, error, .. } => format!("sketch {sketch}: {error:?}"),
            Build { id, error, .. } => format!("{id}: the kernel refused it: {error:?}"),
            NoFace { id, role, .. } => format!("{id}.{role:?}: no such face on the part here"),
            NotPlanar { id, role, .. } => format!("{id}.{role:?} is not planar"),
            EmptySelection { .. } => "the selection picks nothing".into(),
            ExpectFailed {
                metric,
                measured,
                cmp,
                limit,
                tol,
                ..
            } => format!(
                "expect {} {cmp:?} {} +- {}: measured {}",
                metric.name(),
                crate::values::fmt(*limit),
                crate::values::fmt(*tol),
                crate::values::fmt(*measured)
            ),
            UnknownMetric { metric, .. } => format!("unknown metric {metric}"),
            ExpectCopied { metric, value, .. } => format!(
                "expect {} is the measured {} exactly: state the intended value and a tolerance",
                metric.name(),
                crate::values::fmt(*value)
            ),
            UnknownParam { param } => format!("no param {param}"),
            UnknownFeature { id } => format!("no feature {id}"),
            NotAStatement { id } => format!("{id} is made inside a loop or call, not by its own statement"),
            NameTaken { name } => format!("{name} is taken"),
            StillRead { name, reader, .. } => {
                format!("{reader} reads {name}: the edit would change what it reads")
            }
            ShadowsTopLevel { name, .. } => format!("{name} is a top-level name; a local may not hide it"),
            EditDim { param, want, found } => format!("{param} is a {want:?}; the value is a {found:?}"),
            Sheet(e) => e.text(),
        }
    }

    pub fn json(&self, src: &str) -> Json {
        let mut f: Vec<(&str, Json)> = vec![("kind", self.kind().into())];
        if let Some(sp) = self.span() {
            let (l0, c0, l1, c1) = sp.lines(src);
            f.push((
                "span",
                Json::obj(vec![
                    ("lo", (sp.lo as f64).into()),
                    ("hi", (sp.hi as f64).into()),
                    ("line", (l0 as f64).into()),
                    ("col", (c0 as f64).into()),
                    ("end_line", (l1 as f64).into()),
                    ("end_col", (c1 as f64).into()),
                ]),
            ));
        }
        f.extend(self.data());
        f.push(("text", self.text().into()));
        f.push(("fixes", Json::Arr(self.fixes().iter().map(Op::json).collect())));
        Json::obj(f)
    }

    /// The variant's fields as JSON.
    fn data(&self) -> Vec<(&'static str, Json)> {
        use Error::*;
        let s = |x: &str| Json::Str(x.into());
        match self {
            LooseLiteral { value, rule, .. } => vec![("value", (*value).into()), ("rule", s(rule.name()))],
            WorldAfterFirst { first, rule, .. } => vec![("first", s(first)), ("rule", s(rule.name()))],
            Recursion { func, rule, .. } => vec![("func", s(func)), ("rule", s(rule.name()))],
            UnnamedFeature { kind, rule, .. } => vec![("feature_kind", s(kind)), ("rule", s(rule.name()))],
            ParamOutOfScope {
                param, used_in, rule, ..
            } => vec![
                ("param", s(&param.to_string())),
                ("used_in", s(&path_text(used_in))),
                ("rule", s(rule.name())),
            ],
            WrongType {
                what, want, found, ..
            } => vec![
                ("what", s(what)),
                ("want", s(&format!("{want:?}"))),
                ("found", s(&format!("{found:?}"))),
            ],
            DimMismatch { op, left, right, .. } => {
                vec![
                    ("op", s(op)),
                    ("left", s(left.name())),
                    ("right", s(right.name())),
                ]
            }
            WrongDim {
                what, want, found, ..
            } => {
                vec![
                    ("what", s(what)),
                    ("want", s(want.name())),
                    ("found", s(found.name())),
                ]
            }
            NotWhole { value, .. } => vec![("value", (*value).into())],
            BadValue { what, value, .. } => vec![("what", s(what)), ("value", (*value).into())],
            UnknownName { name, .. } => vec![("name", s(name))],
            UnknownMember { ty, member, .. } => vec![("type", s(&format!("{ty:?}"))), ("member", s(member))],
            UnknownArg { func, arg, .. } => vec![("func", s(func)), ("arg", s(arg))],
            MissingArg { func, arg, .. } => vec![("func", s(func)), ("arg", s(arg))],
            DuplicateId { id, .. } | Build { id, .. } => vec![("id", s(id))],
            NoFace { id, role, .. } | NotPlanar { id, role, .. } => {
                vec![("id", s(id)), ("role", s(&format!("{role:?}")))]
            }
            ExpectFailed {
                metric,
                measured,
                limit,
                tol,
                ..
            } => vec![
                ("metric", s(metric.name())),
                ("measured", (*measured).into()),
                ("limit", (*limit).into()),
                ("tol", (*tol).into()),
            ],
            Regions { sketch, count, .. } => vec![("sketch", s(sketch)), ("count", (*count as f64).into())],
            SketchConflict { sketch, rows, .. } => vec![
                ("sketch", s(sketch)),
                ("rows", Json::Arr(rows.iter().map(|r| span_json(*r)).collect())),
            ],
            UnknownParam { param } | EditDim { param, .. } => vec![("param", s(&param.to_string()))],
            UnknownFeature { id } | NotAStatement { id } => vec![("id", s(id))],
            StillRead { name, reader, .. } => vec![("name", s(name)), ("reader", s(reader))],
            ShadowsTopLevel { name, .. } => vec![("name", s(name))],
            Sheet(e) => e.data(),
            _ => vec![],
        }
    }
}

/// Typed warnings: the build went on.
#[derive(Clone, Debug, PartialEq)]
pub enum Warning {
    /// A rule set to lint: the same typed data and fixes as the error, not blocking.
    Lint(Error),
    /// A sketch with freedom left: its points that can still move.
    UnderConstrained { span: Span, sketch: String, dof: usize },
    /// A param nothing reads.
    UnusedParam { span: Span, param: ParamKey },
    /// A feature that changed nothing (a cut that misses, an add inside the part).
    NoEffect { span: Span, id: String },
    /// A sheet's kernel warning or shop violation, with fixes (`sheet.rs`).
    Sheet(Box<crate::sheet::SheetNote>),
}

impl Warning {
    pub fn json(&self, src: &str) -> Json {
        if let Warning::Lint(e) = self {
            let mut j = e.json(src);
            if let Json::Obj(f) = &mut j {
                f.insert(1, ("severity".into(), "lint".into()));
            }
            return j;
        }
        if let Warning::Sheet(n) = self {
            return n.json(src);
        }
        let (kind, span, mut data): (&str, Span, Vec<(&str, Json)>) = match self {
            Warning::Lint(_) | Warning::Sheet(_) => unreachable!("above"),
            Warning::UnderConstrained { span, sketch, dof } => (
                "UnderConstrained",
                *span,
                vec![("sketch", sketch.as_str().into()), ("dof", (*dof as f64).into())],
            ),
            Warning::UnusedParam { span, param } => {
                ("UnusedParam", *span, vec![("param", param.to_string().into())])
            }
            Warning::NoEffect { span, id } => ("NoEffect", *span, vec![("id", id.as_str().into())]),
        };
        let (l, c, ..) = span.lines(src);
        data.insert(0, ("kind", kind.into()));
        data.insert(1, ("severity", "warning".into()));
        data.push(("line", (l as f64).into()));
        data.push(("col", (c as f64).into()));
        data.push(("fixes", Json::Arr(vec![])));
        Json::obj(data)
    }
}
