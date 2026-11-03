//! The errors of the wired features (holes, threads, patterns, work features, bodies, text,
//! imports, projections): typed, each with a span and its data, held in `Error::Feat`.

use crate::error::Op;
use crate::syntax::Span;
use crate::values::Ty;
use forge_agent::json::Json;

#[derive(Clone, Debug, PartialEq)]
pub enum FeatError {
    /// None, or more than one, of a call's exclusive forms (`given` of them).
    Forms {
        span: Span,
        func: &'static str,
        forms: &'static [&'static str],
        given: usize,
    },
    /// Two options that do not go together.
    Conflict {
        span: Span,
        func: &'static str,
        a: &'static str,
        b: &'static str,
    },
    /// An option given where it cannot apply: never dropped silently.
    NotApplicable {
        span: Span,
        func: &'static str,
        arg: &'static str,
        because: Because,
    },
    /// A word argument that is none of the allowed ones.
    UnknownWord {
        span: Span,
        arg: &'static str,
        word: String,
        allowed: &'static [&'static str],
    },
    /// A screw or thread size the tables do not hold.
    UnknownSize { span: Span, size: String },
    /// A reference that must be exactly one entity (a face, an edge, a point) is `count`.
    NotOne {
        span: Span,
        what: &'static str,
        count: usize,
    },
    /// A value worked out here (a point, axis or plane value) where the kernel needs a reference
    /// it can follow through edits: a work feature, a sketch, a sketch point, a face or an edge.
    NotARef { span: Span, found: Ty },
    /// A sketch entity with no name: a reference by name cannot follow it.
    Unnamed { span: Span, sketch: String },
    /// A face or edge no earlier feature's own solid holds (no feature to refer to it through).
    NoOwner { span: Span },
    /// A feature that makes no body, where a body is needed.
    NotABody { span: Span, id: String },
    /// A general curve projected with no `approx:`: no sketch entity holds its image exactly.
    NeedsApprox { span: Span },
    /// Geometry with no image a sketch can hold (an axis not seen end-on ..): the reason.
    NoImage { span: Span, why: String },
    /// A file a feature reads (a font, a STEP file).
    File {
        span: Span,
        file: String,
        problem: FileProblem,
    },
}

/// Why an option cannot apply.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Because {
    /// A through hole ends nowhere: no drill point, no bottom.
    Through,
    /// Only a standard screw size has fits, conventions and seats.
    NoScrew,
    /// The screw's tables hold no such seat.
    NoStandardSeat,
    /// The option is for another form of the call.
    OtherForm,
    /// A thread on a hole takes its size from the hole.
    HoleGivesSize,
    /// The file names no solid this op can take.
    NotSolid,
}

#[derive(Clone, Debug, PartialEq)]
pub enum FileProblem {
    /// Not found beside the part file (or no part file to look beside).
    Missing,
    /// Not a kind of file this feature reads (by its extension).
    WrongKind,
    /// Read, but refused: the reader's reason.
    Unreadable(String),
}

impl FeatError {
    pub fn span(&self) -> Span {
        use FeatError::*;
        match self {
            Forms { span, .. }
            | Conflict { span, .. }
            | NotApplicable { span, .. }
            | UnknownWord { span, .. }
            | UnknownSize { span, .. }
            | NotOne { span, .. }
            | NotARef { span, .. }
            | Unnamed { span, .. }
            | NoOwner { span }
            | NotABody { span, .. }
            | NeedsApprox { span }
            | NoImage { span, .. }
            | File { span, .. } => *span,
        }
    }

    pub fn kind(&self) -> &'static str {
        use FeatError::*;
        match self {
            Forms { .. } => "Forms",
            Conflict { .. } => "Conflict",
            NotApplicable { .. } => "NotApplicable",
            UnknownWord { .. } => "UnknownWord",
            UnknownSize { .. } => "UnknownSize",
            NotOne { .. } => "NotOne",
            NotARef { .. } => "NotARef",
            Unnamed { .. } => "Unnamed",
            NoOwner { .. } => "NoOwner",
            NotABody { .. } => "NotABody",
            NeedsApprox { .. } => "NeedsApprox",
            NoImage { .. } => "NoImage",
            File { .. } => "File",
        }
    }

    /// None of these has an edit that removes it without a choice only the author can make.
    pub fn fixes(&self) -> Vec<Op> {
        vec![]
    }

    pub fn text(&self) -> String {
        use FeatError::*;
        match self {
            Forms {
                func, forms, given, ..
            } => format!(
                "{func} takes exactly one of {}; {given} given",
                forms.join(", ")
            ),
            Conflict { func, a, b, .. } => format!("{func}: {a} and {b} do not go together"),
            NotApplicable {
                func, arg, because, ..
            } => format!("{func}: {arg} cannot apply: {}", because.text()),
            UnknownWord {
                arg, word, allowed, ..
            } => format!("{arg}: \"{word}\" is none of {}", allowed.join(", ")),
            UnknownSize { size, .. } => {
                format!("\"{size}\" is no size the tables hold (M6, M8x1, 1/4-20, #10-24 ..)")
            }
            NotOne { what, count, .. } => format!("{what} must be one entity; it is {count}"),
            NotARef { found, .. } => format!(
                "a {found:?} worked out here, where the feature needs something it can follow: a work feature, a sketch or sketch point, a face or an edge"
            ),
            Unnamed { sketch, .. } => {
                format!("an entity of sketch {sketch} with no name; name it to refer to it")
            }
            NoOwner { .. } => "no earlier feature's own solid holds this face or edge".into(),
            NotABody { id, .. } => format!("{id} makes no body"),
            NeedsApprox { .. } => {
                "a general curve has no exact image in a sketch: give approx: (a tolerance) to fit a spline within it".into()
            }
            NoImage { why, .. } => why.clone(),
            File { file, problem, .. } => match problem {
                FileProblem::Missing => format!("{file}: not found beside the part file"),
                FileProblem::WrongKind => format!("{file}: not a kind of file this reads"),
                FileProblem::Unreadable(why) => format!("{file}: {why}"),
            },
        }
    }

    pub fn data(&self) -> Vec<(&'static str, Json)> {
        use FeatError::*;
        let s = |x: &str| Json::Str(x.into());
        let list = |xs: &[&str]| Json::Arr(xs.iter().map(|x| s(x)).collect());
        match self {
            Forms {
                func, forms, given, ..
            } => vec![
                ("func", s(func)),
                ("forms", list(forms)),
                ("given", (*given as f64).into()),
            ],
            Conflict { func, a, b, .. } => vec![("func", s(func)), ("a", s(a)), ("b", s(b))],
            NotApplicable {
                func, arg, because, ..
            } => vec![
                ("func", s(func)),
                ("arg", s(arg)),
                ("because", s(&format!("{because:?}"))),
            ],
            UnknownWord {
                arg, word, allowed, ..
            } => vec![("arg", s(arg)), ("word", s(word)), ("allowed", list(allowed))],
            UnknownSize { size, .. } => vec![("size", s(size))],
            NotOne { what, count, .. } => vec![("what", s(what)), ("count", (*count as f64).into())],
            NotARef { found, .. } => vec![("found", s(&format!("{found:?}")))],
            Unnamed { sketch, .. } => vec![("sketch", s(sketch))],
            NoOwner { .. } => vec![],
            NotABody { id, .. } => vec![("id", s(id))],
            NeedsApprox { .. } => vec![],
            NoImage { why, .. } => vec![("why", s(why))],
            File { file, problem, .. } => vec![
                ("file", s(file)),
                (
                    "problem",
                    s(match problem {
                        FileProblem::Missing => "Missing",
                        FileProblem::WrongKind => "WrongKind",
                        FileProblem::Unreadable(_) => "Unreadable",
                    }),
                ),
            ],
        }
    }
}

impl Because {
    fn text(self) -> &'static str {
        match self {
            Because::Through => "a through hole ends nowhere",
            Because::NoScrew => "it needs a standard screw size (screw:)",
            Because::NoStandardSeat => "the screw's tables hold no such seat",
            Because::OtherForm => "it belongs to another form of the call",
            Because::HoleGivesSize => "a hole's thread takes its size from the hole",
            Because::NotSolid => "the file holds no solid",
        }
    }
}

impl From<FeatError> for crate::error::Error {
    fn from(e: FeatError) -> Self {
        crate::error::Error::Feat(e)
    }
}
