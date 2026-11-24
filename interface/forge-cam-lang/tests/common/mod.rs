//! What the per-feature test files share (LANG-PROOFS: new tests go in a file per feature, not
//! appended to cam.rs, so branches touching forge-cam-lang do not all conflict there).
#![allow(dead_code)]
use std::path::{Path, PathBuf};

/// The machined examples' folder (the parts the documents name).
pub fn dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/machined")
}

/// A flange document, placed, with two tools and one setup `op1`, and `body` after.
pub fn doc(body: &str) -> String {
    format!(
        "machining\nf = part(\"flange.fg\", group: \"N\")\nm = machine(\"generic-vmc3\")\nt1 = tool(\"flat-10\", t: 1, stickout: 32, holder: \"bt40-er32\")\nt2 = tool(\"flat-6\", t: 2, stickout: 26, holder: \"bt40-er32\")\nop1 = setup(f, machine: m, stock: box(margin: (3, 3, 2), below: 3), origin: \"top-center\", tol: 0.01, at: (0, 0, -250))\n{body}"
    )
}
