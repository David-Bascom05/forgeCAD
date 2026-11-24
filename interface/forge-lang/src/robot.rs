//! The `robot` document kind (research/export/urdf-editor.md): what the language knows of it
//! itself — that a source is one, and the words its statements and values are called by. Its
//! reader, evaluator and edits are `forge_urdf::robot`, kept out of this crate (the language's
//! own code generation is the speed gate's).

use crate::syntax::{self, Expr, ExprKind, StmtKind};

/// A robot document's first statement.
pub const HEADER: &str = "robot";

/// Its statement calls (`NAME = call(..)` or `call(..)`).
pub const STATEMENTS: [&str; 16] = [
    "source",
    "link",
    "frame",
    "joint",
    "root",
    "cut",
    "freeze",
    "merge",
    "drop",
    "visual",
    "collision",
    "collisions",
    "inertial",
    "transmission",
    "actuation",
    "shared",
];

/// Its value calls.
pub const VALUES: [&str; 6] = ["pose", "box", "cylinder", "sphere", "capsule", "mesh"];

/// Names a robot document's values are read as before any binding (`hull`, `none`): binding one
/// would be shadowed silently, so it is refused.
pub const RESERVED: [&str; 2] = ["hull", "none"];

/// Whether a source is a robot document.
pub fn is_robot(src: &str) -> bool {
    match syntax::parse(src) {
        Ok(p) => matches!(
            p.first().map(|s| &s.kind),
            Some(StmtKind::Expr(Expr { kind: ExprKind::Name(n), .. })) if n == HEADER
        ),
        Err(_) => false,
    }
}
