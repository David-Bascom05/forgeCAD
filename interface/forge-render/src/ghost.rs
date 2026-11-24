//! Unknown geometry, shown as such (docs/ux/STANDARD.md §4): a see-through envelope with a dashed
//! outline, and a label saying what is unknown. Never a solid that looks like the product.
//!
//! The envelope goes into a frame's translucent `world` triangles (the part hides it where it is
//! nearer); the label is drawn over the view by `shell::ux::ghost_labels`.

use crate::camera::Camera;
use crate::frame::Bounds;
use crate::origin::{WorldVert, polyline};

/// The envelope's fill and outline, straight alpha (light and dark themes both read it).
pub const FILL: [f32; 4] = [0.62, 0.70, 0.80, 0.12];
pub const LINE: [f32; 4] = [0.75, 0.82, 0.90, 0.85];

/// What a ghost says, and where its leader points.
#[derive(Clone, Debug, PartialEq)]
pub struct Label {
    pub at: [f32; 3],
    /// "Height · unknown", "Frame · no geometry published"
    pub text: String,
}

/// A ghost box: its faces at `FILL`, its twelve edges dashed at `LINE`, each dash about 8 px.
/// Returns the label anchor: the box's top front-right corner as the camera sees it.
pub fn ghost_box(b: Bounds, camera: &Camera, size: (u32, u32), out: &mut Vec<WorldVert>) -> [f32; 3] {
    let c = b.corners();
    // faces: -x +x -y +y -z +z, as corner indices (bit 0 x, bit 1 y, bit 2 z)
    const FACES: [[usize; 4]; 6] = [
        [0, 2, 6, 4],
        [1, 5, 7, 3],
        [0, 4, 5, 1],
        [2, 3, 7, 6],
        [0, 1, 3, 2],
        [4, 6, 7, 5],
    ];
    for f in FACES {
        for k in [0, 1, 2, 0, 2, 3] {
            out.push(WorldVert {
                at: c[f[k]],
                colour: FILL,
            });
        }
    }
    const EDGES: [(usize, usize); 12] = [
        (0, 1),
        (2, 3),
        (4, 5),
        (6, 7),
        (0, 2),
        (1, 3),
        (4, 6),
        (5, 7),
        (0, 4),
        (1, 5),
        (2, 6),
        (3, 7),
    ];
    let eye = camera.eye();
    for (i, j) in EDGES {
        dashed(c[i], c[j], camera, size, eye, out);
    }
    // the corner nearest the eye, on top: where a leader reads best
    c[4..]
        .iter()
        .copied()
        .min_by(|p, q| dist2(*p, eye).total_cmp(&dist2(*q, eye)))
        .unwrap_or(b.max)
}

fn dist2(a: [f32; 3], b: [f32; 3]) -> f32 {
    (0..3).map(|k| (a[k] - b[k]) * (a[k] - b[k])).sum()
}

/// A dashed line a→b: dashes about 8 px long with 6 px gaps at the segment's middle distance.
fn dashed(
    a: [f32; 3],
    b: [f32; 3],
    camera: &Camera,
    size: (u32, u32),
    eye: [f32; 3],
    out: &mut Vec<WorldVert>,
) {
    let len = dist2(a, b).sqrt();
    if len <= 0.0 {
        return;
    }
    let mid = [0, 1, 2].map(|k| (a[k] + b[k]) * 0.5);
    let px = dist2(mid, eye).sqrt() * (camera.fovy * 0.5).tan() * 2.0 / size.1.max(1) as f32;
    let (dash, gap) = (8.0 * px, 6.0 * px);
    let n = ((len / (dash + gap)).floor() as usize).clamp(1, 400);
    let step = len / n as f32;
    let d = (dash / (dash + gap)) * step;
    let at = |s: f32| [0, 1, 2].map(|k| a[k] + (b[k] - a[k]) * (s / len));
    for i in 0..n {
        let s = i as f32 * step + (step - d) * 0.5;
        polyline(&[at(s), at(s + d)], 1.5, LINE, camera, size, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::{Framing, ViewRect, frame};

    #[test]
    fn a_ghost_is_see_through_and_dashed() {
        let b = Bounds::new([0.0, 0.0, 0.0], [150.0, 60.0, 100.0]);
        let cam = frame(b, ViewRect::full(1400.0, 860.0), Framing::default(), &[]).camera;
        let mut out = Vec::new();
        let at = ghost_box(b, &cam, (1400, 860), &mut out);
        assert!(
            out.len() > 36 + 12 * 6,
            "faces and several dashes per edge: {}",
            out.len()
        );
        assert!(out.iter().all(|v| v.colour[3] < 0.9));
        assert!(out[..36].iter().all(|v| v.colour == FILL));
        assert_eq!(at[2], 100.0, "label on the top");
    }
}
