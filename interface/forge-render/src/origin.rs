//! The part's origin: its three planes, three axes and centre point. Drawn as translucent world
//! triangles, picked by what's on the screen, used as references by name (`xy`, `x`).

use crate::camera::{Camera, along, dot};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Item {
    YzPlane,
    XzPlane,
    XyPlane,
    XAxis,
    YAxis,
    ZAxis,
    Center,
}

pub const ITEMS: [Item; 7] = [
    Item::YzPlane,
    Item::XzPlane,
    Item::XyPlane,
    Item::XAxis,
    Item::YAxis,
    Item::ZAxis,
    Item::Center,
];

/// One corner of a world triangle, linear colour.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WorldVert {
    pub at: [f32; 3],
    pub colour: [f32; 4],
}

/// How far the axes reach and the planes spread, from what's in view.
pub fn reach(center: [f32; 3], radius: f32) -> f32 {
    (dot(center, center).sqrt() + radius) * 0.9
}

impl Item {
    pub fn name(self) -> &'static str {
        match self {
            Item::YzPlane => "YZ Plane",
            Item::XzPlane => "XZ Plane",
            Item::XyPlane => "XY Plane",
            Item::XAxis => "X Axis",
            Item::YAxis => "Y Axis",
            Item::ZAxis => "Z Axis",
            Item::Center => "Center Point",
        }
    }

    /// How a part file names it.
    pub fn reference(self) -> &'static str {
        match self {
            Item::YzPlane => "yz",
            Item::XzPlane => "xz",
            Item::XyPlane => "xy",
            Item::XAxis => "x",
            Item::YAxis => "y",
            Item::ZAxis => "z",
            Item::Center => "origin",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            Item::YzPlane | Item::XzPlane | Item::XyPlane => "plane",
            Item::XAxis | Item::YAxis | Item::ZAxis => "axis",
            Item::Center => "point",
        }
    }

    /// Its colour as sRGB: red, green, blue for x, y, z; a plane takes its normal's.
    pub fn colour(self) -> u32 {
        match self {
            Item::YzPlane | Item::XAxis => 0xE0554A,
            Item::XzPlane | Item::YAxis => 0x3FAE6E,
            Item::XyPlane | Item::ZAxis => 0x5A9BE0,
            Item::Center => 0xE0A21B,
        }
    }

    /// The unit direction of an axis.
    fn axis(self) -> Option<[f32; 3]> {
        match self {
            Item::XAxis => Some([1.0, 0.0, 0.0]),
            Item::YAxis => Some([0.0, 1.0, 0.0]),
            Item::ZAxis => Some([0.0, 0.0, 1.0]),
            _ => None,
        }
    }

    /// A plane's two directions.
    fn plane(self) -> Option<([f32; 3], [f32; 3])> {
        match self {
            Item::YzPlane => Some(([0.0, 1.0, 0.0], [0.0, 0.0, 1.0])),
            Item::XzPlane => Some(([1.0, 0.0, 0.0], [0.0, 0.0, 1.0])),
            Item::XyPlane => Some(([1.0, 0.0, 0.0], [0.0, 1.0, 0.0])),
            _ => None,
        }
    }
}

/// Its triangles, for this view. `strong`: hovered or selected.
pub fn tris(
    item: Item,
    reach: f32,
    camera: &Camera,
    size: (u32, u32),
    strong: bool,
    out: &mut Vec<WorldVert>,
) {
    let rgb = crate::viewcube::rgb(item.colour());
    let colour = |a: f32| [rgb[0], rgb[1], rgb[2], a];
    let mut quad = |p: [[f32; 3]; 4], c: [f32; 4]| {
        for k in [0, 1, 2, 0, 2, 3] {
            out.push(WorldVert { at: p[k], colour: c });
        }
    };
    let (_, r, u) = camera.axes();
    // a pixel's width here, about the origin
    let mm_px = camera.dist * (camera.fovy * 0.5).tan() * 2.0 / size.1 as f32;
    if let Some((a, b)) = item.plane() {
        let at = |x: f32, y: f32| along(along([0.0; 3], a, x * reach), b, y * reach);
        quad(
            [at(-1.0, -1.0), at(1.0, -1.0), at(1.0, 1.0), at(-1.0, 1.0)],
            colour(if strong { 0.30 } else { 0.14 }),
        );
        let edge = mm_px * if strong { 1.25 } else { 0.75 };
        let c = colour(if strong { 0.95 } else { 0.55 });
        for (p, q, n) in [
            (at(-1.0, -1.0), at(1.0, -1.0), b),
            (at(-1.0, 1.0), at(1.0, 1.0), b),
            (at(-1.0, -1.0), at(-1.0, 1.0), a),
            (at(1.0, -1.0), at(1.0, 1.0), a),
        ] {
            quad(
                [
                    along(p, n, -edge),
                    along(q, n, -edge),
                    along(q, n, edge),
                    along(p, n, edge),
                ],
                c,
            );
        }
    } else if let Some(a) = item.axis() {
        let half = mm_px * if strong { 1.5 } else { 1.0 };
        // a ribbon square to the eye
        let f = camera.axes().0;
        let side = {
            let s = [
                a[1] * f[2] - a[2] * f[1],
                a[2] * f[0] - a[0] * f[2],
                a[0] * f[1] - a[1] * f[0],
            ];
            let l = dot(s, s).sqrt().max(1e-5);
            s.map(|v| v / l)
        };
        let (p, q) = (along([0.0; 3], a, -reach), along([0.0; 3], a, reach));
        quad(
            [
                along(p, side, -half),
                along(q, side, -half),
                along(q, side, half),
                along(p, side, half),
            ],
            colour(if strong { 1.0 } else { 0.75 }),
        );
    } else {
        let h = mm_px * if strong { 5.0 } else { 3.5 };
        quad(
            [
                along([0.0; 3], r, -h),
                along([0.0; 3], u, -h),
                along([0.0; 3], r, h),
                along([0.0; 3], u, h),
            ],
            colour(1.0),
        );
    }
}

/// What of these is under a pixel. A point or an axis within a few pixels; a plane only if `planes`
/// (where nothing of the part is hit).
pub fn hit(
    items: &[Item],
    reach: f32,
    camera: &Camera,
    size: (u32, u32),
    px: (f32, f32),
    planes: bool,
) -> Option<Item> {
    const NEAR: f32 = 6.0;
    let on = |p: [f32; 3]| camera.pixel_of(p, size.0, size.1);
    if items.contains(&Item::Center)
        && let Some(c) = on([0.0; 3])
        && (c.0 - px.0).hypot(c.1 - px.1) < NEAR * 1.5
    {
        return Some(Item::Center);
    }
    let mut best = None;
    let mut near = NEAR;
    for &i in items {
        let Some(a) = i.axis() else { continue };
        let (Some(p), Some(q)) = (on(along([0.0; 3], a, -reach)), on(along([0.0; 3], a, reach))) else {
            continue;
        };
        let (dx, dy) = (q.0 - p.0, q.1 - p.1);
        let l = (dx * dx + dy * dy).max(1e-6);
        let t = (((px.0 - p.0) * dx + (px.1 - p.1) * dy) / l).clamp(0.0, 1.0);
        let d = (p.0 + dx * t - px.0).hypot(p.1 + dy * t - px.1);
        if d < near {
            near = d;
            best = Some(i);
        }
    }
    if best.is_some() || !planes {
        return best;
    }
    // the nearest plane the ray goes through inside its square
    let eye = camera.eye();
    let through = camera.point_at(px, camera.dist, size.0, size.1);
    let d = [0, 1, 2].map(|k| through[k] - eye[k]);
    let mut got: Option<(f32, Item)> = None;
    for &i in items {
        let Some((a, b)) = i.plane() else { continue };
        let n = [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ];
        let denom = dot(n, d);
        if denom.abs() < 1e-7 {
            continue;
        }
        let t = -dot(n, eye) / denom;
        let q = along(eye, d, t);
        if t > 0.0 && dot(q, a).abs() < reach && dot(q, b).abs() < reach && got.is_none_or(|g| t < g.0) {
            got = Some((t, i));
        }
    }
    got.map(|g| g.1)
}

fn minus(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

/// How long a pixel is at this place in the world.
fn pixel_there(p: [f32; 3], camera: &Camera, size: (u32, u32)) -> f32 {
    let to = minus(p, camera.eye());
    (to[0] * to[0] + to[1] * to[1] + to[2] * to[2]).sqrt() * (camera.fovy * 0.5).tan() * 2.0 / size.1 as f32
}

/// A line through these points, this many pixels wide, facing the eye.
pub fn polyline(
    points: &[[f32; 3]],
    px: f32,
    colour: [f32; 4],
    camera: &Camera,
    size: (u32, u32),
    out: &mut Vec<WorldVert>,
) {
    for w in points.windows(2) {
        let (a, b) = (w[0], w[1]);
        let way = minus(b, a);
        let view = minus(a, camera.eye());
        let n = [
            way[1] * view[2] - way[2] * view[1],
            way[2] * view[0] - way[0] * view[2],
            way[0] * view[1] - way[1] * view[0],
        ];
        let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
        if len < 1e-9 {
            continue;
        }
        let half = |p: [f32; 3]| pixel_there(p, camera, size) * px * 0.5 / len;
        let (ha, hb) = (half(a), half(b));
        let corners = [
            along(a, n, -ha),
            along(b, n, -hb),
            along(b, n, hb),
            along(a, n, ha),
        ];
        for k in [0, 1, 2, 0, 2, 3] {
            out.push(WorldVert {
                at: corners[k],
                colour,
            });
        }
    }
}

/// A dot, this many pixels across, facing the eye.
pub fn disc(
    p: [f32; 3],
    px: f32,
    colour: [f32; 4],
    camera: &Camera,
    size: (u32, u32),
    out: &mut Vec<WorldVert>,
) {
    let (_, r, u) = camera.axes();
    let radius = pixel_there(p, camera, size) * px * 0.5;
    let rim = |k: usize| {
        let (s, c) = (k as f32 * core::f32::consts::TAU / 8.0).sin_cos();
        along(along(p, r, c * radius), u, s * radius)
    };
    for k in 0..8 {
        for at in [p, rim(k), rim(k + 1)] {
            out.push(WorldVert { at, colour });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cam() -> Camera {
        let mut c = Camera::fit([0.0; 3], 50.0);
        c.yaw = 0.6;
        c.pitch = 0.5;
        c
    }

    #[test]
    fn an_axis_or_the_point_is_found_by_its_pixels_and_a_plane_only_where_allowed() {
        let (c, size) = (cam(), (800, 600));
        let r = 50.0;
        let at = |p: [f32; 3]| c.pixel_of(p, size.0, size.1).unwrap();
        assert_eq!(hit(&ITEMS, r, &c, size, at([0.0; 3]), true), Some(Item::Center));
        assert_eq!(
            hit(&ITEMS, r, &c, size, at([30.0, 0.0, 0.0]), true),
            Some(Item::XAxis)
        );
        assert_eq!(
            hit(&ITEMS, r, &c, size, at([0.0, 0.0, -30.0]), false),
            Some(Item::ZAxis)
        );
        let on_xy = at([20.0, -30.0, 0.0]);
        assert_eq!(
            hit(&[Item::XyPlane], r, &c, size, on_xy, true),
            Some(Item::XyPlane)
        );
        assert_eq!(hit(&[Item::XyPlane], r, &c, size, on_xy, false), None);
        // what isn't shown isn't found
        assert_eq!(hit(&[Item::YAxis], r, &c, size, at([30.0, 0.0, 0.0]), true), None);
    }

    #[test]
    fn every_item_draws_whole_triangles() {
        let mut v = Vec::new();
        for i in ITEMS {
            let n = v.len();
            tris(i, 50.0, &cam(), (800, 600), true, &mut v);
            assert!(v.len() > n && (v.len() - n) % 3 == 0, "{i:?}");
        }
    }
}
