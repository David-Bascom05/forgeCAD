//! Press-brake simulation for sheet parts (design: research/bendsim/README.md).
//!
//! - [`tooling`]: punches, dies, the press and its backgauge as data, with a standard library in our
//!   own numbers ([`library`]).
//! - [`model`]: the part as rigid walls and bend zones, and its exact partial refolds: every bend
//!   at any angle in [0, its turn], neutral length kept (audit2 §2.0).
//! - [`sim`]: one bend on the press, placed in air bending, checked through its whole motion;
//!   clearances are certified by interval branch and bound, never sampled or meshed.
//! - [`plan`]: a bend order and tooling that pass every check, or each bend's blocking checks.
pub mod bb;
pub mod catalog;
pub mod geo;
pub mod library;
pub mod model;
pub mod num;
pub mod plan;
pub mod scene;
pub mod schema;
pub mod sim;
pub mod tooling;
pub mod view;

pub use model::{Model, ModelError};
pub use plan::{Blocking, Options, Plan, Suggestion, plan, simulate};
pub use sim::{Check, PartRef, Phase, Rule, Setup, Severity, Step, Surface};
pub use tooling::{
    Backgauge, Die, Finger, Press, Punch, PunchKind, Segment, Springback, Style, Tooling, ToolingError,
};
