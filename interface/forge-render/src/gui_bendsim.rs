//! The Bend Simulation environment in the view: the press, its tools and the part at the instant
//! shown (forge-brake's scene), drawn by the view's own renderer as parts, so they are lit and
//! shadowed as any part is. The design's renderer and scene are put aside while it is in and put
//! back when it leaves.

use super::Gui;
use crate::mesh::{FaceRange, Lod, PartMesh};
use crate::render::Renderer;
use crate::scene::{Placement, Scene};
use crate::shell::bendsim::Framing;
use crate::shell::env::EnvId;
use forge_brake::scene::{Body, Look, Mesh};

struct Taken {
    renderer: Renderer,
    scene: Scene,
    /// The design's view cube naming and home, put back on leaving.
    orient: crate::viewcube::Orient,
    home_view: Option<crate::camera::Camera>,
}

#[derive(Default)]
pub struct BendView {
    taken: Option<Taken>,
    /// What is drawn: (simulation's identity, step, instant) and the bodies' names.
    shown: Option<(usize, usize, u64)>,
    names: Vec<String>,
    /// The view's renderer's geometry counter after our last upload: another value means a
    /// build put its own part in the view since.
    ours: Option<u64>,
    /// The camera framed on the press once per entry.
    framed: bool,
    /// Where the part is at the instant shown (its bounds): what Follow bend keeps in view.
    part: Option<([f32; 3], [f32; 3])>,
    /// Follow bend's move under way: from (target, distance), to (target, distance), and the frames
    /// it has run; held until it is there.
    goal: Option<(Eye, Eye, u32)>,
    /// The working station's place along the bed when framed: the part moved to another
    /// station, the eye follows it there.
    station: Option<u64>,
    /// The station the eye was last brought to (framed there, or panned there by Follow bend): a
    /// step at another one pans the eye along the bed to it (David: the eye goes to each station
    /// the part moves to, and holds through the handling there).
    followed: Option<u64>,
    /// The press's parts cut away for the eye (the Work framing's near walls), and the framed
    /// subject they are cut for (none in Press Overview).
    cut: Vec<String>,
    subject: Option<([f32; 3], [f32; 3])>,
    /// The step last framed (Follow bend re-frames on a new one).
    step: Option<usize>,
    /// The part's motion at the tools through that step: (step, lo, hi).
    envelope: Option<(usize, [f32; 3], [f32; 3])>,
    /// The bodies drawn, in placement order: what see-through tests against.
    bodies: Vec<Body>,
    /// Each step's contact regions (BENDSIM-TOOLS' contacts_of), for the simulation they came
    /// from (its address): made once a step, only for a step whose collision checks fail or are
    /// near.
    contacts: Option<(usize, std::collections::HashMap<usize, Vec<forge_brake::Contact>>)>,
    /// See-through's faces drawn as ghosts (the bodies that hide the bend, whole).
    pub ghosts: Vec<crate::render::FaceColour>,
    /// Each body's see-through state: frames it has blocked running, and how far it has faded
    /// (0 solid, 1 at the set opacity).
    fade: std::collections::HashMap<String, (u8, f32)>,
    /// How far each press part is cut away (0 there, 1 gone), stepped a frame at a time over
    /// FADE_FRAMES as it is cut or put back (the coordinator: no part vanishes in one frame).
    cutfade: std::collections::HashMap<String, f32>,
    /// What is drawn and seen (the bodies but those wholly cut away).
    visible: Vec<Body>,
    /// A body is fading in or out, or waiting its frames to.
    fading: bool,
    /// The instant's bodies as the scene makes them (before the cut-away and the x-ray), and the
    /// eye and settings the drawn ones were derived for.
    raw: Vec<Body>,
    derived: Option<([u32; 3], bool, u32, bool)>,
    /// What is seen through, outlined: its silhouette edges from the eye, in the world.
    outline: Vec<([[f32; 3]; 2], f32)>,
}

/// Edges by their welded ends: how many triangles share it (an edge to draw marked half of
/// usize::MAX), its ends as first seen, whether its first triangle faces the eye, and that
/// triangle's unit normal.
type EdgeFaces = std::collections::HashMap<([i64; 3], [i64; 3]), (usize, [f32; 3], [f32; 3], bool, [f32; 3])>;

/// The cosine past which two faces meeting at an edge make a crease drawn (about 30 degrees).
const CREASE_COS: f32 = 0.866;

/// A mesh's outline from `eye`: its silhouette (edges between a triangle facing the eye and one
/// facing away), its creases (faces meeting at more than about 30 degrees: its shape, so a ghost
/// reads in depth) and its open edges (the mesh's corners welded to a hundredth of a millimetre).
pub fn silhouette(m: &Mesh, eye: [f32; 3]) -> Vec<[[f32; 3]; 2]> {
    let key = |p: [f32; 3]| p.map(|v| (v * 100.0).round() as i64);
    let mut faces = EdgeFaces::new();
    for t in &m.tri {
        let [a, b, c] = t.map(|i| m.pos[i as usize]);
        let (u, v) = (
            [b[0] - a[0], b[1] - a[1], b[2] - a[2]],
            [c[0] - a[0], c[1] - a[1], c[2] - a[2]],
        );
        let n = [
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ];
        let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
        if len < 1e-12 {
            continue;
        }
        let n = n.map(|x| x / len);
        let to = [eye[0] - a[0], eye[1] - a[1], eye[2] - a[2]];
        let front = n[0] * to[0] + n[1] * to[1] + n[2] * to[2] > 0.0;
        for (p, q) in [(a, b), (b, c), (c, a)] {
            let (kp, kq) = (key(p), key(q));
            let k = if kp <= kq { (kp, kq) } else { (kq, kp) };
            let e = faces.entry(k).or_insert((0, p, q, front, n));
            e.0 += 1;
            let crease = e.4[0] * n[0] + e.4[1] * n[1] + e.4[2] * n[2] < CREASE_COS;
            if e.0 > 1 && e.0 < usize::MAX / 2 && (e.3 != front || crease) {
                // one face each way, or a crease: drawn
                e.0 = usize::MAX / 2;
            }
        }
    }
    faces
        .into_values()
        .filter(|e| e.0 == 1 || e.0 >= usize::MAX / 2)
        .map(|e| [e.1, e.2])
        .collect()
}

/// See-through's tint, in linear light (the ghost is blended linear and shown in sRGB): a
/// ghost is drawn flat (unlit), so its own colour is first taken down to a mid tone
/// (XRAY_SHADE: the light grey of tool steel or paint, drawn flat at that, is the floor's and
/// the backdrop's and could not be seen), then a light, desaturated blue (sRGB about 150, 180,
/// 225) mixed in by XRAY_MIX (David: "make part of the opacity slightly blue"); its outline a
/// darker blue (sRGB about 60, 95, 150).
pub const XRAY_SHADE: f32 = 0.25;
pub const XRAY_BLUE: [f32; 3] = [0.3, 0.46, 0.75];
pub const XRAY_MIX: f32 = 0.3;
pub const XRAY_LINE: [f32; 3] = [0.045, 0.11, 0.3];

/// An eye as Follow bend moves it: its target and its distance.
type Eye = ([f32; 3], f32);

/// Follow bend's safe area: the part's middle may stray this far from the frame's (normalised
/// device coordinates, a half-width) before the eye pans; and the frames a move takes (half a
/// second at 30 a second), eased from rest to rest.
pub const FOLLOW_SAFE: f32 = 0.6;
pub const FOLLOW_FRAMES: u32 = 15;

/// The profile view's yaw and pitch (degrees): looking along +y, down the bend line.
pub const PROFILE_YAW: f32 = 262.0;
pub const PROFILE_PITCH: f32 = 12.0;

/// Frames a body must block the bend for, running, before it starts to fade (hysteresis).
pub const FADE_AFTER: u8 = 3;
/// Frames a fade in or out takes (about 150 ms at 60 frames a second).
pub const FADE_FRAMES: u32 = 9;

/// The view cube's naming in the simulation: FRONT the operator's side (-x), BACK the
/// backgauge's, TOP up; LEFT and RIGHT along the bed, so along every bend's line (each is made
/// along the bed): those faces give the view down the bend line.
pub fn bend_orient() -> crate::viewcube::Orient {
    crate::viewcube::Orient::with_face([-1, 0, 0], [0, -1, 0], [0, 0, 1])
}

/// Unit normal to octahedral i16x2 (as tess.rs packs them).
fn oct(n: [f32; 3]) -> [i16; 2] {
    let s = (n[0].abs() + n[1].abs() + n[2].abs()).max(1e-20);
    let (mut x, mut y) = (n[0] / s, n[1] / s);
    if n[2] < 0.0 {
        (x, y) = ((1.0 - y.abs()) * x.signum(), (1.0 - x.abs()) * y.signum());
    }
    [(x * 32767.0).round() as i16, (y * 32767.0).round() as i16]
}

/// A scene body's triangles as a part: one level, one face, no edges.
pub fn part_of(m: &Mesh) -> PartMesh {
    let mut lod = Lod {
        chord: 0.05,
        ..Lod::default()
    };
    for (p, n) in m.pos.iter().zip(&m.nrm) {
        for v in p {
            lod.vertices.extend_from_slice(&v.to_le_bytes());
        }
        for v in oct(*n) {
            lod.vertices.extend_from_slice(&v.to_le_bytes());
        }
    }
    for t in &m.tri {
        for v in t {
            lod.indices.extend_from_slice(&v.to_le_bytes());
        }
    }
    lod.faces.push(FaceRange {
        face: 0,
        first_tri: 0,
        tris: m.tri.len() as u32,
        first_vertex: 0,
        vertices: m.pos.len() as u32,
    });
    let (lo, hi) = m.bounds().unwrap_or(([0.0; 3], [0.0; 3]));
    let center = [0, 1, 2].map(|k| (lo[k] + hi[k]) * 0.5);
    let radius = (0..3).map(|k| (hi[k] - lo[k]).powi(2)).sum::<f32>().sqrt() * 0.5;
    PartMesh {
        center,
        radius: radius.max(1e-3),
        lods: vec![lod],
    }
}

/// The appearances library, parsed once (RENDER-STUDIO's: it clones a static list per call).
fn appearances() -> &'static crate::studio::appearance::Appearances {
    static A: std::sync::OnceLock<crate::studio::appearance::Appearances> = std::sync::OnceLock::new();
    A.get_or_init(crate::studio::appearance::Appearances::builtin)
}

/// A body's tint (linear rgb, metalness) from its appearance (forge_brake::scene::Look's id,
/// RENDER-STUDIO's library): ground tool steel, machine paint (no maker's livery is in the data:
/// its neutral default), black-oxide backgauge, concrete floor; the sheet in its forge-material's
/// appearance, or a neutral matte sheet when the material is not known; failing bodies red.
pub fn tint(b: &Body, material: Option<&str>) -> [f32; 4] {
    // a contact region (Collisions): red where a collision is proven, amber where near
    if b.name.starts_with("contact:") {
        return if b.hit { CONTACT_HIT } else { CONTACT_NEAR };
    }
    let t = own_tint(b, material);
    // the other stations' tools dimmed (UX-LEAD, BENDSIM-TOOLS): the station working is the
    // tools as they are, solid steel; the others there, set back, a light neutral and matte
    if b.look == Look::Tool && b.name.starts_with("station") && !b.hit {
        return [0, 1, 2, 3].map(|k| match k {
            3 => t[3] * STATION_DIM_METAL,
            _ => t[k] + (STATION_DIM_TO - t[k]) * STATION_DIM,
        });
    }
    t
}

/// A contact region's colours: a proven collision, a near one (linear rgb, matte).
pub const CONTACT_HIT: [f32; 4] = [0.95, 0.06, 0.04, 0.0];
pub const CONTACT_NEAR: [f32; 4] = [0.98, 0.62, 0.08, 0.0];

/// How far the other stations' tools are set back toward a light neutral, that neutral, and what
/// of their metal's sheen they keep.
pub const STATION_DIM: f32 = 0.8;
pub const STATION_DIM_TO: f32 = 0.85;
pub const STATION_DIM_METAL: f32 = 0.1;

fn own_tint(b: &Body, material: Option<&str>) -> [f32; 4] {
    let id = if b.hit {
        "highlight-red"
    } else {
        b.look.appearance()
    };
    let found = if b.look == Look::Sheet && !b.hit {
        appearances().viewport_tint(Some("sheet"), material, None)
    } else {
        appearances().viewport_tint(Some(id), None, None)
    };
    found.unwrap_or(match b.look {
        // no material named: a neutral, matte sheet (not the selection's blue, not a metal
        // the part was never said to be), apart from the ground steel of the tools
        Look::Sheet => [0.42, 0.47, 0.53, 0.0],
        Look::Tool => [0.66, 0.67, 0.69, 1.0],
        Look::Frame => [0.46, 0.48, 0.50, 0.15],
        Look::Gauge => [0.16, 0.16, 0.17, 0.6],
        Look::Floor => [0.30, 0.30, 0.29, 0.0],
        Look::Witness => [0.95, 0.10, 0.08, 0.0],
        // the bend being made: an annotation colour of the panel's, not a material
        Look::BendZone => [0.98, 0.60, 0.10, 0.0],
    })
}

/// Axis-aligned bounds of meshes, (lo, hi); None when empty.
fn bounds<'a>(meshes: impl Iterator<Item = &'a Mesh>) -> Option<([f32; 3], [f32; 3])> {
    let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
    for m in meshes {
        if let Some((a, c)) = m.bounds() {
            for k in 0..3 {
                lo[k] = lo[k].min(a[k]);
                hi[k] = hi[k].max(c[k]);
            }
        }
    }
    (lo[0] <= hi[0]).then_some((lo, hi))
}

/// The eye for a framing (docs/ux/STANDARD.md §2), from the 3/4 front-right-top view (the
/// operator's side, -x; the bed's right-hand end, -y), perspective, the subject filling about
/// 70 % of the view's shorter side. `Work`'s subject is the working zone: the part through the
/// whole step's motion (`envelope`: its bounds at every instant sampled) and the working
/// station's punch and die, their length cut to the part's plus a margin; held through the
/// step. `Overview`'s is the whole press. Returns the eye and the subject's bounds.
pub fn framing(
    bodies: &[Body],
    envelope: Option<([f32; 3], [f32; 3])>,
    how: Framing,
    handling: bool,
    profile: bool,
    size: (u32, u32),
) -> Option<(crate::camera::Camera, [f32; 3], [f32; 3])> {
    let (mut lo, mut hi) = match envelope {
        Some(e) => e,
        None => bounds(bodies.iter().filter(|x| x.look.is_sheet()).map(|x| &x.mesh))?,
    };
    let (ylo, yhi) = (lo[1], hi[1]);
    let mut grow = |p: [f32; 3]| {
        for k in 0..3 {
            lo[k] = lo[k].min(p[k]);
            hi[k] = hi[k].max(p[k]);
        }
    };
    match how {
        // the working zone is the part's, scaled to it (David): the punch's tip and the die's V
        // where they meet it, never the tools' height or the press
        // through the operator's handling the part is the subject alone: away from the tools
        Framing::Work if handling => {}
        Framing::Work => {
            let tip = bodies
                .iter()
                .filter(|x| x.name.starts_with("punch:") && x.name.ends_with("/tip"))
                .flat_map(|x| x.mesh.pos.iter())
                .map(|p| p[2])
                .fold(f32::INFINITY, f32::min);
            for x in bodies.iter().filter(|x| x.look == Look::Tool) {
                let die = x.name.starts_with("die:");
                let punch = x.name.starts_with("punch:") && x.name.ends_with("/tip");
                if !die && !punch {
                    continue;
                }
                let top = x.mesh.pos.iter().map(|p| p[2]).fold(f32::MIN, f32::max);
                for p in &x.mesh.pos {
                    // the die's V and shoulders (its top 12 mm); the tip's last 12 mm
                    let near = if die {
                        p[2] >= top - 12.0
                    } else {
                        p[2] <= tip + 12.0
                    };
                    if near && p[1] >= ylo && p[1] <= yhi {
                        grow(*p);
                    }
                }
            }
        }
        Framing::Overview => {
            // the bed's tool row (every station) and the part: the overview's subject (UX-LEAD),
            // not the press around it
            for x in bodies
                .iter()
                .filter(|x| matches!(x.look, Look::Tool | Look::Sheet | Look::BendZone))
            {
                if let Some((a, c)) = x.mesh.bounds() {
                    grow(a);
                    grow(c);
                }
            }
        }
    }
    let center = [
        0.5 * (lo[0] + hi[0]),
        0.5 * (lo[1] + hi[1]),
        0.5 * (lo[2] + hi[2]),
    ];
    let radius = 0.5 * ((hi[0] - lo[0]).powi(2) + (hi[1] - lo[1]).powi(2) + (hi[2] - lo[2]).powi(2)).sqrt();
    let mut cam = crate::camera::Camera::fit(center, radius);
    // the working view three-quarter from the operator's side and above; or (profile) end on,
    // down the bend line from the bed's end, a little toward the operator and above: the bend's
    // profile, as press-brake simulations show it (David to choose)
    let (yaw, pitch) = match (how, profile) {
        (Framing::Work, false) => (215f32, 28f32),
        (Framing::Work, true) => (PROFILE_YAW, PROFILE_PITCH),
        // from the operator's side and above, the row along the bed seen nearly broadside
        (Framing::Overview, _) => (200f32, 30f32),
    };
    cam.yaw = yaw.to_radians();
    cam.pitch = pitch.to_radians();
    // the subject's sphere fits the view with a margin of about 15 % (the press, and the part
    // being handled: ~70 %)
    let margin = match how {
        Framing::Work if !handling => 1.15,
        _ => 1.0 / 0.7,
    };
    cam.dist = cam.fit_dist(radius, size.0.max(1), size.1.max(1)) * margin;
    if handling {
        // the part handled is seen clear of the press (RENDER-STUDIO, the coordinator: not the
        // bed's near edge): the eye rises from the working zone's until its lines to the part's
        // middle and corners pass no press part or tool
        let mut aims = vec![center];
        aims.extend((0..8).map(|k| [0, 1, 2].map(|i| if k >> i & 1 == 0 { lo[i] } else { hi[i] })));
        let blocks: Vec<([f32; 3], [f32; 3])> = bodies
            .iter()
            .filter(|x| matches!(x.look, Look::Frame | Look::Gauge | Look::Tool))
            .filter_map(|x| x.mesh.bounds())
            .collect();
        let clear = |c: &crate::camera::Camera| {
            let eye = c.eye();
            aims.iter()
                .all(|&p| blocks.iter().all(|&(a, b)| !hits(eye, p, a, b)))
        };
        let mut p = pitch;
        while p < 75.0 && !clear(&cam) {
            p += 5.0;
            cam.pitch = p.to_radians();
        }
    }
    Some((cam, lo, hi))
}

/// Does the segment a-b pass through the box (lo, hi)? (slabs)
fn hits(a: [f32; 3], b: [f32; 3], lo: [f32; 3], hi: [f32; 3]) -> bool {
    let (mut t0, mut t1) = (0f32, 1f32);
    for k in 0..3 {
        let d = b[k] - a[k];
        if d.abs() < 1e-9 {
            if a[k] < lo[k] || a[k] > hi[k] {
                return false;
            }
            continue;
        }
        let (mut u, mut v) = ((lo[k] - a[k]) / d, (hi[k] - a[k]) / d);
        if u > v {
            std::mem::swap(&mut u, &mut v);
        }
        t0 = t0.max(u);
        t1 = t1.min(v);
        if t0 > t1 {
            return false;
        }
    }
    true
}

/// The press's parts cut away for an eye: those (frame, beams, backgauge) that stand between it
/// and the subject (its centre and corners), or that hold it, and those above the subject,
/// nearer the eye than it, in the picture (docs/ux/STANDARD.md §2: the eye outside every body,
/// the machine's near walls cut away). The tools and the part are never cut.
pub fn cutaway(
    bodies: &[Body],
    cam: &crate::camera::Camera,
    size: (u32, u32),
    lo: [f32; 3],
    hi: [f32; 3],
) -> Vec<String> {
    let eye = cam.eye();
    // the subject's nearest reach from the eye, and the view's projection: a press part above the
    // working zone and nearer the eye than any of it, in the picture, hangs in front of it (the
    // upper beam over a bend seen from in front and above): cut too
    let near_of = |a: [f32; 3], c: [f32; 3]| {
        let q = [0, 1, 2].map(|k| eye[k].clamp(a[k], c[k]) - eye[k]);
        (q[0] * q[0] + q[1] * q[1] + q[2] * q[2]).sqrt()
    };
    let subject = near_of(lo, hi);
    let mid_z = 0.5 * (lo[2] + hi[2]);
    let m = cam.view(size.0.max(1), size.1.max(1)).matrix;
    let seen = |a: [f32; 3], c: [f32; 3]| {
        (0..8).any(|k| {
            let p = [0, 1, 2].map(|i| if k >> i & 1 == 0 { a[i] } else { c[i] });
            let w = m[3] * p[0] + m[7] * p[1] + m[11] * p[2] + m[15];
            let x = (m[0] * p[0] + m[4] * p[1] + m[8] * p[2] + m[12]) / w;
            let y = (m[1] * p[0] + m[5] * p[1] + m[9] * p[2] + m[13]) / w;
            w > 0.0 && x.abs() <= 1.0 && y.abs() <= 1.0
        })
    };
    let mut aims = vec![[
        0.5 * (lo[0] + hi[0]),
        0.5 * (lo[1] + hi[1]),
        0.5 * (lo[2] + hi[2]),
    ]];
    for k in 0..8 {
        aims.push([
            if k & 1 == 0 { lo[0] } else { hi[0] },
            if k & 2 == 0 { lo[1] } else { hi[1] },
            if k & 4 == 0 { lo[2] } else { hi[2] },
        ]);
    }
    bodies
        .iter()
        .filter(|x| matches!(x.look, Look::Frame | Look::Gauge))
        .filter(|x| {
            x.mesh.bounds().is_some_and(|(a, c)| {
                // a little inside its box: a face the subject rests on is not in the way
                let (a2, c2) = (a.map(|v| v + 1.0), c.map(|v| v - 1.0));
                aims.iter().any(|&m| hits(eye, m, a2, c2))
                    || (a[2] > mid_z && near_of(a, c) < subject && seen(a, c))
            })
        })
        .map(|x| x.name.clone())
        .collect()
}

/// What the user watches (David): points on the bend being made (the zone's top, along it),
/// on the punch tip where it meets the sheet, on the free edge of the flange swinging up (the
/// coordinator), and on any check's witness. A body is seen through
/// only where it stands between the eye and these.
/// What the press overview watches: every station's tools (each body's middle and its ends
/// along the bed, at its middle height and near its working edge). The press body hiding them
/// is seen through (UX-LEAD).
pub fn overview_samples(bodies: &[Body]) -> Vec<[f32; 3]> {
    let mut out = Vec::new();
    for x in bodies.iter().filter(|x| x.look == Look::Tool) {
        let Some((a, c)) = x.mesh.bounds() else { continue };
        let die = x.name.contains("die:");
        let mx = 0.5 * (a[0] + c[0]);
        let mz = 0.5 * (a[2] + c[2]);
        // the die's top, the punch's foot: where the work is
        let edge = if die { c[2] - 0.1 * (c[2] - a[2]) } else { a[2] + 0.1 * (c[2] - a[2]) };
        for f in [0.15f32, 0.5, 0.85] {
            let y = a[1] + f * (c[1] - a[1]);
            out.push([mx, y, mz]);
            out.push([mx, y, edge]);
        }
    }
    out
}

pub fn roi_samples(bodies: &[Body]) -> Vec<[f32; 3]> {
    // up to `n` of a mesh's points, spread along its length (y: along the bed)
    fn spread(pts: Vec<[f32; 3]>, n: usize) -> Vec<[f32; 3]> {
        if pts.len() <= n {
            return pts;
        }
        let mut pts = pts;
        pts.sort_by(|a, b| {
            a[1].total_cmp(&b[1])
                .then(a[2].total_cmp(&b[2]))
                .then(a[0].total_cmp(&b[0]))
        });
        (0..n).map(|k| pts[k * (pts.len() - 1) / (n - 1)]).collect()
    }
    let mut out = Vec::new();
    out.extend(spread(zone_top(bodies), 48));
    // the tip's contact edge: its points within a millimetre of its lowest
    let tips: Vec<&Body> = bodies
        .iter()
        .filter(|x| x.name.starts_with("punch:") && x.name.ends_with("/tip"))
        .collect();
    if let Some(low) = tips
        .iter()
        .flat_map(|x| x.mesh.pos.iter())
        .map(|p| p[2])
        .reduce(f32::min)
    {
        let edge: Vec<[f32; 3]> = tips
            .iter()
            .flat_map(|x| x.mesh.pos.iter().copied())
            .filter(|p| p[2] <= low + 1.0)
            .collect();
        out.extend(spread(edge, 16));
    }
    // the flange swinging up (a piece beside the bend that rises over it): its free edge, the
    // points of it farthest from the bend, so a flange swinging toward the ram stays in sight
    let zb = bounds(
        bodies
            .iter()
            .filter(|x| x.look == Look::BendZone)
            .map(|x| &x.mesh),
    );
    if let Some((zl, zh)) = zb {
        let zc = [0, 1, 2].map(|k| 0.5 * (zl[k] + zh[k]));
        for x in bodies.iter().filter(|x| bending(x, zb)) {
            let Some((_, top)) = x.mesh.bounds() else { continue };
            if top[2] <= zh[2] + 1.0 {
                continue;
            }
            let far = |p: &[f32; 3]| (p[0] - zc[0]).powi(2) + (p[2] - zc[2]).powi(2);
            let reach = x.mesh.pos.iter().map(far).fold(0f32, f32::max);
            let edge: Vec<[f32; 3]> = x
                .mesh
                .pos
                .iter()
                .filter(|p| far(p) >= reach * 0.98)
                .copied()
                .collect();
            out.extend(spread(edge, 12));
        }
    }
    let wit: Vec<[f32; 3]> = bodies
        .iter()
        .filter(|x| x.look == Look::Witness)
        .flat_map(|x| x.mesh.pos.iter().copied())
        .collect();
    out.extend(spread(wit, 8));
    out
}

/// The bend zone's top along the bend line: at each place along it (to half a millimetre), its
/// highest point; the bend as one looks onto it, not its outside, which lies in the die's V.
pub fn zone_top(bodies: &[Body]) -> Vec<[f32; 3]> {
    let mut top: std::collections::BTreeMap<i64, [f32; 3]> = Default::default();
    for p in bodies
        .iter()
        .filter(|x| x.look == Look::BendZone)
        .flat_map(|x| x.mesh.pos.iter())
    {
        let e = top.entry((p[1] * 2.0).round() as i64).or_insert(*p);
        if p[2] > e[2] {
            *e = *p;
        }
    }
    top.into_values().collect()
}

/// How many of `aims` a mesh hides from `eye`: the segment to each, short of it by half a
/// millimetre, passing through one of its triangles (Moller-Trumbore; its box first).
pub fn roi_blocked(mesh: &Mesh, eye: [f32; 3], aims: &[[f32; 3]]) -> usize {
    let Some((lo, hi)) = mesh.bounds() else {
        return 0;
    };
    let sub = |a: [f32; 3], b: [f32; 3]| [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    let dot = |a: [f32; 3], b: [f32; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    let cross = |a: [f32; 3], b: [f32; 3]| {
        [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ]
    };
    aims.iter()
        .filter(|&&p| {
            let d = sub(p, eye);
            let len = dot(d, d).sqrt();
            if len < 1e-3 {
                return false;
            }
            // short of the aim, so a surface it lies on does not hide it
            let tmax = 1.0 - 0.5 / len;
            let end = [0, 1, 2].map(|k| eye[k] + d[k] * tmax);
            if !hits(eye, end, lo, hi) {
                return false;
            }
            mesh.tri.iter().any(|t| {
                let [a, b, c] = t.map(|i| mesh.pos[i as usize]);
                let (e1, e2) = (sub(b, a), sub(c, a));
                let h = cross(d, e2);
                let det = dot(e1, h);
                if det.abs() < 1e-12 {
                    return false;
                }
                let f = 1.0 / det;
                let s0 = sub(eye, a);
                let u = f * dot(s0, h);
                if !(0.0..=1.0).contains(&u) {
                    return false;
                }
                let q = cross(s0, e1);
                let v = f * dot(d, q);
                if v < 0.0 || u + v > 1.0 {
                    return false;
                }
                let tt = f * dot(e2, q);
                tt > 1e-6 && tt < tmax
            })
        })
        .count()
}

/// A body hides the bend when it hides at least this share of what is watched (and two points at
/// least): a corner grazing one line of sight does not.
pub const ROI_SHARE: f32 = 0.03;

/// Whether a piece of the part is the bend being made or a wall either side of it (it touches
/// the bend zone): never seen through.
fn bending(x: &Body, zone: Option<([f32; 3], [f32; 3])>) -> bool {
    x.look == Look::Sheet
        && match (zone, x.mesh.bounds()) {
            (Some((zl, zh)), Some((a, c))) => (0..3).all(|k| a[k] <= zh[k] + 0.5 && c[k] >= zl[k] - 0.5),
            _ => false,
        }
}

/// The bodies that may be seen through: anything but the bend being made and the walls either
/// side of it, a punch's tip (the contact), the floor and a witness; what is drawn red too, in its
/// red. Whether one is, is the eye's: only when it hides what is watched (`roi_blocked`), the die
/// and the bed included (seen low, a die's shoulder hides the bend in its V).
fn xray_candidate(x: &Body, zone: Option<([f32; 3], [f32; 3])>) -> bool {
    match x.look {
        Look::Sheet => !bending(x, zone),
        Look::Tool => !x.name.ends_with("/tip"),
        Look::Frame | Look::Gauge => true,
        Look::BendZone | Look::Floor | Look::Witness => false,
    }
}

impl BendView {
    /// The outline of what is seen through, as world triangles: thin (1 px) darker-blue lines
    /// facing this eye, as strong as the body has faded. The view's frame and a capture's (Render Studio's recorder) both draw them.
    pub fn outline_tris(
        &self,
        camera: &crate::camera::Camera,
        size: (u32, u32),
    ) -> Vec<crate::origin::WorldVert> {
        let mut out = Vec::new();
        for (e, level) in &self.outline {
            let [r, g, b] = XRAY_LINE;
            crate::origin::polyline(e, 1.0, [r, g, b, 0.85 * level], camera, size, &mut out);
        }
        out
    }

    /// The names of what is drawn now.
    pub fn drawn_names(&self) -> Vec<String> {
        self.bodies.iter().map(|x| x.name.clone()).collect()
    }

    /// What is drawn now, whole (not what is wholly cut away).
    pub fn drawn(&self) -> &[Body] {
        &self.visible
    }

    /// What is drawn seen through now.
    pub fn ghosted(&self) -> Vec<&Body> {
        self.ghosts
            .iter()
            .filter_map(|g| self.bodies.get(g.placement as usize))
            .collect()
    }

    /// The names of what is drawn seen through now.
    pub fn ghosted_names(&self) -> Vec<String> {
        self.ghosts
            .iter()
            .filter_map(|g| self.bodies.get(g.placement as usize))
            .map(|x| x.name.clone())
            .collect()
    }

    /// The pieces of the part that are the bend being made and the walls either side of it.
    pub fn bending_names(&self) -> Vec<String> {
        let zone = bounds(
            self.raw
                .iter()
                .filter(|x| x.look == Look::BendZone)
                .map(|x| &x.mesh),
        );
        self.raw
            .iter()
            .filter(|x| bending(x, zone))
            .map(|x| x.name.clone())
            .collect()
    }

    /// Whether every body drawn is one of the instant's, whole: none cut.
    pub fn whole(&self) -> bool {
        self.bodies.iter().all(|d| {
            self.raw
                .iter()
                .any(|x| x.name == d.name && x.mesh.pos == d.mesh.pos && x.mesh.tri == d.mesh.tri)
        })
    }

    /// The view holds the simulation's bodies, not the design.
    pub fn showing(&self) -> bool {
        self.taken.is_some()
    }
}

impl Gui {
    /// Each frame: the simulation's instant in the view while the environment is in; the
    /// design's view back once it is not.
    pub(super) fn bendsim_frame(&mut self) {
        let on = self.shell.env == Some(EnvId::BendSim)
            && self.shell.sheet.bend_sim.as_ref().is_some_and(|b| b.sim.is_ok());
        if !on {
            self.bendsim_put_back();
            return;
        }
        let Some(v) = &mut self.view else { return };
        // a build put its own part in the view: that is the design's now
        if let Some(g) = self.bend.ours
            && v.renderer.geometry() != g
        {
            self.bend.taken = None;
            self.bend.shown = None;
            self.bend.names.clear();
            self.bend.ours = None;
        }
        let b = self.shell.sheet.bend_sim.as_ref().unwrap();
        let sim = b.sim.as_ref().unwrap();
        let n = sim.plan.steps.len();
        if n == 0 {
            return;
        }
        let step = b.step.min(n - 1);
        // (the Collisions switch in the key: turned, the bodies are drawn again)
        let key = (sim as *const _ as usize, step * 2 + usize::from(b.collisions), b.t.to_bits());
        if self.bend.shown == Some(key) && b.frame.is_none() {
            self.bendsim_follow();
            self.bendsim_see_through();
            return;
        }
        let mut sc = forge_brake::scene::Scene::new(&sim.model, &sim.tooling, &sim.plan);
        if let Some(h) = b.table_height {
            sc.table_height = h;
        }
        let mut bodies = sc.bodies(step, b.t);
        // the contact regions at this instant, on the part (Collisions)
        if b.collisions {
            let id = sim as *const _ as usize;
            if self.bend.contacts.as_ref().is_none_or(|c| c.0 != id) {
                self.bend.contacts = Some((id, Default::default()));
            }
            let cache = &mut self.bend.contacts.as_mut().unwrap().1;
            let cs = cache.entry(step).or_insert_with(|| {
                let st = &sim.plan.steps[step];
                let colliding = st.checks.iter().any(|c| {
                    matches!(
                        c.rule,
                        forge_brake::Rule::ToolCollision
                            | forge_brake::Rule::ClosesOnPunch
                            | forge_brake::Rule::Withdraw
                            | forge_brake::Rule::SelfCollision
                    ) && (!c.pass || c.near.is_some())
                });
                if !colliding {
                    return Vec::new();
                }
                let mut done = vec![false; sim.model.bends.len()];
                for s in &sim.plan.steps[..step] {
                    s.mark(&mut done);
                }
                forge_brake::contacts_of(&sim.model, &sim.tooling, &done, st)
            });
            let regions = sc.contact_bodies(step, b.t, cs);
            if !regions.is_empty() {
                // the regions say where: the part's pieces keep their own colour under them (the
                // whole piece red would hide the region); the tools met stay red
                for x in bodies.iter_mut().filter(|x| x.name.starts_with("part:")) {
                    x.hit = false;
                }
            }
            bodies.extend(regions);
        }
        // the station the step works at (along the bed)
        let at = {
            let st = &sim.plan.steps[step];
            st.station
                .and_then(|k| sim.tooling.stations.get(k))
                .map(|x| x.y.to_bits())
        };
        let ask = b.frame;
        // framed on entry and when asked (the View tools, F); otherwise the eye stands still as an
        // operator's does at the brake, Follow bend only panning it down the bed (David: the
        // camera stays still for the part's small motions and mostly pans down the line)
        let first = !self.bend.framed;
        let mut eye = None;
        if first || ask.is_some() {
            let how = ask.unwrap_or(Framing::Work);
            // the part's whole motion at the tools at this station, every step made there: the eye
            // then stands still through them (David), panning only when the part moves station;
            // the operator's handling (the pull out toward the operator, the turns) is not fitted
            // (fitting it draws the eye far back): it may pass out of the frame's edge and back
            let envelope = match how {
                Framing::Work => {
                    // the step's own station (BENDSIM-TOOLS' Step.station: a punch and die may
                    // stand at two stations)
                    let here = |k: usize| {
                        let st = &sim.plan.steps[k];
                        st.station
                            .and_then(|j| sim.tooling.stations.get(j))
                            .map(|x| x.y.to_bits())
                    };
                    let samples: Vec<Vec<Body>> = (0..n)
                        .filter(|&k| here(k) == at)
                        .flat_map(|k| [0.2, 0.35, 0.5, 0.6, 0.7, 0.78, 0.9].map(|tt| (k, tt)))
                        .map(|(k, tt)| sc.bodies(k, tt))
                        .collect();
                    let env = bounds(
                        samples
                            .iter()
                            .flatten()
                            .filter(|x| x.look.is_sheet())
                            .map(|x| &x.mesh),
                    );
                    if let Some((a, c)) = env {
                        self.bend.envelope = Some((step, a, c));
                    }
                    env
                }
                Framing::Overview => None,
            };
            if let Some((cam, lo, hi)) = framing(&bodies, envelope, how, false, b.profile, v.size) {
                self.bend.subject = (how == Framing::Work).then_some((lo, hi));
                if how == Framing::Work {
                    // the cube's home is the working zone
                    v.nav.home_view = Some(cam);
                }
                eye = Some((cam, first));
            }
            self.bend.framed = true;
            self.bend.goal = None;
            self.bend.followed = at;
        }
        self.bend.station = at;
        self.bend.step = Some(step);
        // where the part is now: what Follow bend keeps in view
        self.bend.part = bounds(bodies.iter().filter(|x| x.look.is_sheet()).map(|x| &x.mesh));
        if let Some(b) = self.shell.sheet.bend_sim.as_mut() {
            b.frame = None;
        }
        match eye {
            // on entry the eye is placed; asked for, it travels there
            Some((cam, true)) => v.nav.jump_to(cam),
            Some((cam, false)) => v.nav.go_to(cam),
            None => {}
        }
        // a failure located: the eye turned on its place, at its distance
        if let Some(at) = self.shell.sheet.bend_sim.as_mut().and_then(|b| b.locate.take()) {
            let mut cam = v.nav.camera;
            cam.target = at.map(|x| x as f32);
            cam.dist = cam.dist.min(400.0);
            v.nav.jump_to(cam);
        }
        self.bend.raw = bodies;
        self.bend.derived = None;
        self.bend.shown = Some(key);
        self.bendsim_follow();
        self.bendsim_see_through();
    }

    /// Follow bend as an operator watches (David): the eye keeps its orientation and distance (the
    /// user's own, after an orbit); while the part stays in the frame's middle 60 % it does not
    /// move (sliding to the stop, lifting, the ram's stroke, a flange's swing, a turn over); when
    /// it leaves that, the eye pans along the bed (y) to it; only when it would leave the frame is
    /// it brought back whole, and only when it cannot fit does the eye draw back, once. Every move
    /// eased a fraction a frame (no cut, no overshoot, counted in frames: a recording is the same
    /// each time), and a move once begun is finished before another is chosen (no hunting).
    fn bendsim_follow(&mut self) {
        let Some(v) = &mut self.view else { return };
        let Some(b) = self.shell.sheet.bend_sim.as_mut() else {
            return;
        };
        if !b.follow || !self.bend.framed {
            self.bend.goal = None;
            return;
        }
        let Some((lo, hi)) = self.bend.part else { return };
        let mut cam = v.nav.camera;
        // the operator's handling (the pull out, the turns) starts no move: the eye stands still
        // through it and the part comes back to the tools where the eye is (David)
        let handling = forge_brake::scene::stage_at(b.t).0 == forge_brake::scene::Stage::Handle;
        // the part at another station: the eye pans along the bed to it, its height and distance
        // kept, once the handling is over (before the bend is made there)
        if self.bend.goal.is_none() && !handling && self.bend.station != self.bend.followed {
            if let Some(y) = self.bend.station.map(|y| f64::from_bits(y) as f32) {
                let mut t = cam.target;
                t[1] = y;
                self.bend.goal = Some(((cam.target, cam.dist), (t, cam.dist), 0));
            }
            self.bend.followed = self.bend.station;
        }
        if self.bend.goal.is_none() && !handling {
            let m = cam.view(v.size.0.max(1), v.size.1.max(1)).matrix;
            let ndc = |p: [f32; 3]| {
                let c = |r: usize| m[r] * p[0] + m[4 + r] * p[1] + m[8 + r] * p[2] + m[12 + r];
                let w = c(3).max(1e-6);
                (c(0) / w, c(1) / w)
            };
            let mid = [0, 1, 2].map(|k| 0.5 * (lo[k] + hi[k]));
            let (mut x0, mut x1, mut y0, mut y1) = (f32::MAX, f32::MIN, f32::MAX, f32::MIN);
            for k in 0..8 {
                let (x, y) = ndc([0, 1, 2].map(|i| if k >> i & 1 == 0 { lo[i] } else { hi[i] }));
                (x0, x1, y0, y1) = (x0.min(x), x1.max(x), y0.min(y), y1.max(y));
            }
            let span = (x1 - x0).max(y1 - y0);
            // near the frame's edge (a margin ahead of it, so the move is under way before it gets there)
            let out = x0 < -0.85 || x1 > 0.85 || y0 < -0.85 || y1 > 0.85;
            let (cx, cy) = ndc(mid);
            let from = (cam.target, cam.dist);
            if span > 1.9 {
                // cannot fit: drawn back to fit it at about three-quarters, centred
                self.bend.goal = Some((from, (mid, cam.dist * span / 1.5), 0));
            } else if out {
                self.bend.goal = Some((from, (mid, cam.dist), 0));
            } else if cx.abs() > FOLLOW_SAFE || cy.abs() > FOLLOW_SAFE {
                let mut t = cam.target;
                t[1] = mid[1];
                self.bend.goal = Some((from, (t, cam.dist), 0));
            }
        }
        let Some(((t0, d0), (t1, d1), f)) = self.bend.goal else {
            return;
        };
        // a smoothstep from rest to rest over FOLLOW_FRAMES: no jolt at the start, no overshoot
        let f = f + 1;
        let p = (f as f32 / FOLLOW_FRAMES as f32).min(1.0);
        let e = p * p * (3.0 - 2.0 * p);
        let before = cam.target;
        cam.target = [0, 1, 2].map(|k| t0[k] + (t1[k] - t0[k]) * e);
        cam.dist = d0 + (d1 - d0) * e;
        self.bend.goal = (p < 1.0).then_some(((t0, d0), (t1, d1), f));
        // the cut-away's subject moves with the eye
        if let Some((a, c)) = self.bend.subject.as_mut() {
            for k in 0..3 {
                let dk = cam.target[k] - before[k];
                a[k] += dk;
                c[k] += dk;
            }
        }
        v.nav.jump_to(cam);
        v.dirty = true;
        b.settle = b.settle.max(2);
    }

    /// What is drawn, kept up with the eye: the instant's bodies, the press's near walls cut
    /// away, and whatever stands between the eye and the bend being made seen through, each such
    /// body whole (nothing is cut), faded in and out.
    fn bendsim_see_through(&mut self) {
        let Some(v) = &mut self.view else { return };
        let Some(b) = self.shell.sheet.bend_sim.as_ref() else {
            return;
        };
        if self.bend.raw.is_empty() {
            return;
        }
        let cam = v.nav.camera;
        let eye = cam.eye();
        let overview = b.overview;
        let key = (eye.map(f32::to_bits), b.see_through, b.opacity.to_bits(), overview);
        if self.bend.derived == Some(key) && !self.bend.fading {
            return;
        }
        self.bend.derived = Some(key);
        // the cut-away, for this eye and the press as it stands at this instant (the ram comes
        // down through the picture as a bend forms: it is cut as it does)
        self.bend.cut = match self.bend.subject {
            Some((lo, hi)) => cutaway(&self.bend.raw, &cam, v.size, lo, hi),
            None => Vec::new(),
        };
        let cut = self.bend.cut.clone();
        // the cut-away faded in and out over FADE_FRAMES; a part first seen starts as it is
        let step = 1.0 / FADE_FRAMES as f32;
        let mut cutting = false;
        let raw_names: Vec<String> = self.bend.raw.iter().map(|x| x.name.clone()).collect();
        self.bend.cutfade.retain(|n, _| raw_names.contains(n));
        for n in &raw_names {
            let target = if cut.contains(n) { 1.0 } else { 0.0 };
            let e = self.bend.cutfade.entry(n.clone()).or_insert(target);
            *e = if *e < target {
                (*e + step).min(target)
            } else {
                (*e - step).max(target)
            };
            cutting |= *e != target;
        }
        let cutfade = self.bend.cutfade.clone();
        let cut_level = |n: &str| cutfade.get(n).copied().unwrap_or(0.0);
        // all drawn (a part wholly cut away an unseen ghost: it still casts its shadow, so the
        // floor's light does not jump as it goes); a part being cut is no see-through candidate
        let working: Vec<&Body> = self.bend.raw.iter().collect();
        // the x-ray, from this eye: a candidate hiding enough of what is watched (the bend's zone,
        // the punch's contact, a witness) from it
        let zone = bounds(
            working
                .iter()
                .filter(|x| x.look == Look::BendZone)
                .map(|x| &x.mesh),
        );
        // worked out even with see-through off: what is still fading out keeps fading until it is
        // solid again
        let aims = if overview {
            overview_samples(&self.bend.raw)
        } else {
            roi_samples(&self.bend.raw)
        };
        let need = ((aims.len() as f32 * ROI_SHARE).ceil() as usize).max(2);
        let see = b.see_through;
        let mut blocks: std::collections::HashSet<String> = Default::default();
        if !aims.is_empty() {
            for x in working
                .iter()
                .filter(|x| {
                    cut_level(&x.name) == 0.0
                        && if overview {
                            // the overview: only the press's body is seen through, the tools stay
                            matches!(x.look, Look::Frame | Look::Gauge)
                        } else {
                            xray_candidate(x, zone)
                        }
                })
            {
                if roi_blocked(&x.mesh, eye, &aims) >= need {
                    blocks.insert(x.name.clone());
                }
            }
        }
        // hysteresis and a fade, counted in frames (deterministic when recorded): a body must
        // block for FADE_AFTER frames running before it starts to fade, and fades in or out over
        // FADE_FRAMES frames, so a body grazing the line of sight does not flicker
        let mut moving = cutting;
        let names: Vec<String> = working.iter().map(|x| x.name.clone()).collect();
        self.bend.fade.retain(|n, _| names.contains(n));
        for n in &names {
            let e = self.bend.fade.entry(n.clone()).or_insert((0, 0.0));
            let blocking = see && blocks.contains(n);
            e.0 = if blocking { e.0.saturating_add(1) } else { 0 };
            let target = if e.0 >= FADE_AFTER { 1.0 } else { 0.0 };
            let step = 1.0 / FADE_FRAMES as f32;
            e.1 = if e.1 < target {
                (e.1 + step).min(target)
            } else {
                (e.1 - step).max(target)
            };
            moving |= e.1 != target || (blocking && e.0 < FADE_AFTER);
        }
        self.bend.fading = moving;
        // what is drawn: every body whole, never cut; one that hides the bend seen through
        let drawn: Vec<Body> = working.iter().map(|x| (*x).clone()).collect();
        let xray: Vec<(usize, f32)> = drawn
            .iter()
            .enumerate()
            .filter_map(|(k, x)| {
                let level = self.bend.fade.get(&x.name).map_or(0.0, |f| f.1);
                (level > 0.0).then_some((k, level))
            })
            .collect();
        let material = b.material.as_deref();
        let opacity = b.opacity;
        // the drawn bodies into the view: the same ones, new triangles; else a renderer afresh
        let parts: Vec<PartMesh> = drawn.iter().map(|x| part_of(&x.mesh)).collect();
        let placements: Vec<Placement> = drawn
            .iter()
            .enumerate()
            .map(|(k, x)| Placement {
                tint: tint(x, material),
                ..Placement::at(k as u32, [0.0; 3])
            })
            .collect();
        let info: Vec<([f32; 3], f32, Vec<f32>)> = parts
            .iter()
            .map(|m| (m.center, m.radius, m.lods.iter().map(|x| x.chord).collect()))
            .collect();
        let names: Vec<String> = drawn.iter().map(|x| x.name.clone()).collect();
        let mut scene = Scene::new(&info, placements);
        scene.set_explode(0.0);
        scene.ghost_shadows = true;
        if names == self.bend.names && self.bend.taken.is_some() {
            v.renderer.set_parts(&parts);
            v.renderer.set_tints(&scene);
            v.scene = scene;
        } else {
            let mut renderer = Renderer::new(
                self.device.clone(),
                self.queue.clone(),
                self.adapter.clone(),
                crate::render::OFFSCREEN,
                (16, 16),
                &parts,
                &scene,
            );
            renderer.resize(v.size);
            // the simulation is shown as it would look: studio light, floor shadow, no edges
            renderer.style = crate::render::VisualStyle::Realistic;
            renderer.look = v.renderer.look.clone();
            renderer.back_colour = v.renderer.back_colour;
            renderer.set_tints(&scene);
            let old_r = std::mem::replace(&mut v.renderer, renderer);
            let old_s = std::mem::replace(&mut v.scene, scene);
            if self.bend.taken.is_none() {
                self.bend.taken = Some(Taken {
                    renderer: old_r,
                    scene: old_s,
                    orient: v.nav.orient,
                    home_view: v.nav.home_view.take(),
                });
                v.nav.orient = bend_orient();
            }
            self.bend.names = names;
        }
        // what hides the bend as ghosts: from solid down to the opacity set as they fade, tinted
        // toward a light blue (David: an x-ray, not missing), a thin darker-blue outline coming
        // in with them
        self.bend.ghosts.clear();
        let mut outline: Vec<([[f32; 3]; 2], f32)> = Vec::new();
        for &(k, level) in &xray {
            v.scene.set_ghost(k, true);
            let t = tint(&drawn[k], material);
            // a failing body keeps its red (no blue mixed in): what fails still reads as failing
            let blue = if drawn[k].hit { 0.0 } else { XRAY_MIX };
            let c = [0, 1, 2].map(|i| {
                let t = t[i] * (1.0 - (1.0 - XRAY_SHADE) * level);
                t + (XRAY_BLUE[i] - t) * blue * level
            });
            let to8 = |x: f32| (x.clamp(0.0, 1.0) * 255.0).round() as u8;
            let alpha = 1.0 - level * (1.0 - opacity);
            self.bend.ghosts.push(crate::render::FaceColour {
                placement: k as u32,
                face: 0,
                rgba: [to8(c[0]), to8(c[1]), to8(c[2]), to8(alpha)],
            });
            outline.extend(silhouette(&drawn[k].mesh, eye).into_iter().map(|e| (e, level)));
        }
        // a press part being cut away or put back: in its own colour, fading (a cut, not an x-ray)
        for (k, x) in drawn.iter().enumerate() {
            let level = cut_level(&x.name);
            if level <= 0.0 || xray.iter().any(|&(j, _)| j == k) {
                continue;
            }
            v.scene.set_ghost(k, true);
            if level >= 1.0 {
                // wholly cut away: not drawn, its shadow kept
                continue;
            }
            let t = tint(x, material);
            let to8 = |x: f32| (x.clamp(0.0, 1.0) * 255.0).round() as u8;
            self.bend.ghosts.push(crate::render::FaceColour {
                placement: k as u32,
                face: 0,
                rgba: [to8(t[0]), to8(t[1]), to8(t[2]), to8(1.0 - level)],
            });
        }
        self.bend.visible = drawn
            .iter()
            .filter(|x| cut_level(&x.name) < 1.0)
            .cloned()
            .collect();
        self.bend.outline = outline;
        self.bend.bodies = drawn;
        self.bend.ours = Some(v.renderer.geometry());
        v.dirty = true;
        if moving && let Some(b) = self.shell.sheet.bend_sim.as_mut() {
            // frames still to draw while a body fades
            b.settle = b.settle.max(2);
        }
    }

    fn bendsim_put_back(&mut self) {
        self.bend.framed = false;
        self.bend.part = None;
        self.bend.goal = None;
        self.bend.cut.clear();
        self.bend.subject = None;
        self.bend.raw.clear();
        self.bend.derived = None;
        self.bend.shown = None;
        self.bend.names.clear();
        self.bend.ours = None;
        self.bend.bodies.clear();
        self.bend.ghosts.clear();
        self.bend.fade.clear();
        self.bend.cutfade.clear();
        self.bend.visible.clear();
        self.bend.fading = false;
        self.bend.outline.clear();
        self.bend.step = None;
        self.bend.envelope = None;
        if let (Some(t), Some(v)) = (self.bend.taken.take(), &mut self.view) {
            v.renderer = t.renderer;
            v.scene = t.scene;
            v.nav.orient = t.orient;
            v.nav.home_view = t.home_view;
            v.renderer.resize(v.size);
            v.nav.fit_all(&v.scene);
            v.dirty = true;
        }
    }
}

#[cfg(test)]
mod station_tests {
    use super::*;

    fn tool(name: &str) -> Body {
        Body {
            name: name.into(),
            look: Look::Tool,
            mesh: Mesh::default(),
            hit: false,
            strip: None,
        }
    }

    #[test]
    fn the_stations_not_working_are_set_back() {
        let working = tint(&tool("punch:trumpf/OW300-S/body"), None);
        let other = tint(&tool("station3:punch:trumpf/OW300-S/body"), None);
        let lum = |c: [f32; 4]| c[0] + c[1] + c[2];
        assert!(lum(other) > lum(working), "lighter: {other:?} vs {working:?}");
        assert!(other[3] < working[3], "less metal: {other:?} vs {working:?}");
        // a failing tool keeps its red wherever it stands
        let mut red = tool("station3:punch:trumpf/OW300-S/body");
        red.hit = true;
        let mut red_here = tool("punch:trumpf/OW300-S/body");
        red_here.hit = true;
        assert_eq!(tint(&red, None), tint(&red_here, None));
    }
}
