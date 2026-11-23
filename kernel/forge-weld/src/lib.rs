//! Weldments: structural members swept along the segments of a layout, their standard sections,
//! and the one rule for how they meet (frame.rs), all exact from lines, arcs and planes.

pub mod frame;
pub mod library;
pub mod section;

pub use frame::{
    At, Corner, Cut, Cutting, End, EndCut, FrameError, MemberSpec, Node, Path, Plan, cutting, plan, reach,
};
pub use library::{designations, lookup};
pub use section::{Kind, Section, SectionError};
