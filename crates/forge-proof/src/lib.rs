//! Ways to prove the kernel beyond "it built": metamorphic relations between results (`props`),
//! certificates every solid must carry (`cert`), and the B-rep checked point by point against
//! the CSG tree it came from. A quick tier runs in `cargo test`; the deep tier is the `meta`
//! example, which shrinks each failure to a minimal history and writes it as a part file.

pub mod cert;
pub mod map;
pub mod props;
