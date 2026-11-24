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
use forge_brake::scene::{Body, Look, Mesh, stage_at};

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
    /// The working station's place along the bed when framed: the part moved to another
    /// station, the eye follows it there.
    station: Option<u64>,
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
    /// See-through's faces drawn as ghosts (the x-ray window's pieces).
    pub ghosts: Vec<crate::render::FaceColour>,
    /// Each body's see-through state: frames it has blocked running, and how far it has faded
    /// (0 solid, 1 at the set opacity).
    fade: std::collections::HashMap<String, (u8, f32)>,
    /// A body is fading in or out, or waiting its frames to.
    fading: bool,
    /// The instant's bodies as the scene makes them (before the cut-away and the x-ray), and the
    /// eye and settings the drawn ones were derived for.
    raw: Vec<Body>,
    derived: Option<([u32; 3], bool, u32)>,
    /// What is seen through, outlined: its silhouette edges from the eye, in the world.
    outline: Vec<([[f32; 3]; 2], f32)>,
}

/// A mesh's silhouette from `eye`: its edges between a triangle facing the eye and one facing
/// away, and its open edges (the mesh's corners welded to a hundredth of a millimetre).
pub fn silhouette(m: &Mesh, eye: [f32; 3]) -> Vec<[[f32; 3]; 2]> {
    use std::collections::HashMap;
    let key = |p: [f32; 3]| p.map(|v| (v * 100.0).round() as i64);
    let mut faces: HashMap<([i64; 3], [i64; 3]), (usize, [f32; 3], [f32; 3], bool)> = HashMap::new();
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
        let to = [eye[0] - a[0], eye[1] - a[1], eye[2] - a[2]];
        let front = n[0] * to[0] + n[1] * to[1] + n[2] * to[2] > 0.0;
        for (p, q) in [(a, b), (b, c), (c, a)] {
            let (kp, kq) = (key(p), key(q));
            let k = if kp <= kq { (kp, kq) } else { (kq, kp) };
            let e = faces.entry(k).or_insert((0, p, q, front));
            e.0 += 1;
            if e.0 > 1 && e.3 != front {
                // one face each way: on the silhouette
                e.3 = !front;
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
            for x in bodies.iter().filter(|x| !matches!(x.look, Look::Floor)) {
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
    let (yaw, pitch) = match how {
        Framing::Work => (215f32, 28f32),
        Framing::Overview => (220f32, 20f32),
    };
    cam.yaw = yaw.to_radians();
    cam.pitch = pitch.to_radians();
    // the subject's sphere fits the view with a margin of about 15 % (the press: ~70 %)
    let margin = match how {
        Framing::Work => 1.15,
        Framing::Overview => 1.0 / 0.7,
    };
    cam.dist = cam.fit_dist(radius, size.0.max(1), size.1.max(1)) * margin;
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

/// The x-ray window: where on the screen the bend being made and the punch's contact are, grown
/// by a margin (normalised device coordinates: xmin, xmax, ymin, ymax), and how far from the
/// eye they begin (clip w): what is nearer than that, inside the window, hides them.
pub fn xray_window(bodies: &[Body], m: &[f32; 16]) -> Option<([f32; 4], f32)> {
    let zone = bounds(
        bodies
            .iter()
            .filter(|x| x.look == Look::BendZone)
            .map(|x| &x.mesh),
    )?;
    let (lo, hi) = zone;
    let mut aims: Vec<[f32; 3]> = (0..8)
        .map(|k| [0, 1, 2].map(|i| if k >> i & 1 == 0 { lo[i] } else { hi[i] }))
        .collect();
    for x in bodies
        .iter()
        .filter(|x| x.name.starts_with("punch:") && x.name.ends_with("/tip"))
    {
        if let Some(p) = x.mesh.pos.iter().min_by(|a, b| a[2].total_cmp(&b[2])) {
            aims.push(*p);
        }
    }
    let clip = |p: [f32; 3], r: usize| m[r] * p[0] + m[4 + r] * p[1] + m[8 + r] * p[2] + m[12 + r];
    let (mut x0, mut x1, mut y0, mut y1, mut w0) = (f32::MAX, f32::MIN, f32::MAX, f32::MIN, f32::MAX);
    for p in &aims {
        let w = clip(*p, 3);
        if w <= 1e-6 {
            return None;
        }
        let (x, y) = (clip(*p, 0) / w, clip(*p, 1) / w);
        x0 = x0.min(x);
        x1 = x1.max(x);
        y0 = y0.min(y);
        y1 = y1.max(y);
        w0 = w0.min(w);
    }
    // a margin round it: a quarter of its size, at least a sixteenth of the view
    let (mx, my) = ((0.25 * (x1 - x0)).max(0.125), (0.25 * (y1 - y0)).max(0.125));
    Some(([x0 - mx, x1 + mx, y0 - my, y1 + my], w0 * 0.995))
}

/// A mesh split by the x-ray window's frustum: (inside it and nearer the eye than `wmax`,
/// the rest). Each triangle is clipped against the window's sides and its depth in clip space,
/// where they are planes (Sutherland-Hodgman); positions and normals carried linearly.
pub fn xray_split(mesh: &Mesh, m: &[f32; 16], win: [f32; 4], wmax: f32) -> (Mesh, Mesh) {
    type V = ([f32; 3], [f32; 3]);
    let clip = |p: [f32; 3], r: usize| m[r] * p[0] + m[4 + r] * p[1] + m[8 + r] * p[2] + m[12 + r];
    let planes: [Box<dyn Fn([f32; 3]) -> f32>; 5] = [
        Box::new(move |p| clip(p, 0) - win[0] * clip(p, 3)),
        Box::new(move |p| win[1] * clip(p, 3) - clip(p, 0)),
        Box::new(move |p| clip(p, 1) - win[2] * clip(p, 3)),
        Box::new(move |p| win[3] * clip(p, 3) - clip(p, 1)),
        Box::new(move |p| wmax - clip(p, 3)),
    ];
    let lerp = |a: V, b: V, t: f32| -> V {
        (
            [0, 1, 2].map(|k| a.0[k] + (b.0[k] - a.0[k]) * t),
            [0, 1, 2].map(|k| a.1[k] + (b.1[k] - a.1[k]) * t),
        )
    };
    let push = |out: &mut Mesh, poly: &[V]| {
        for k in 1..poly.len().saturating_sub(1) {
            let base = out.pos.len() as u32;
            for v in [poly[0], poly[k], poly[k + 1]] {
                out.pos.push(v.0);
                out.nrm.push(v.1);
            }
            out.tri.push([base, base + 1, base + 2]);
        }
    };
    let (mut inside, mut outside) = (Mesh::default(), Mesh::default());
    for t in &mesh.tri {
        let mut poly: Vec<V> = t
            .iter()
            .map(|&i| (mesh.pos[i as usize], mesh.nrm[i as usize]))
            .collect();
        for f in &planes {
            let d: Vec<f32> = poly.iter().map(|v| f(v.0)).collect();
            if d.iter().all(|&x| x >= 0.0) {
                continue;
            }
            let (mut keep, mut drop) = (Vec::new(), Vec::new());
            for i in 0..poly.len() {
                let j = (i + 1) % poly.len();
                let (a, b, da, db) = (poly[i], poly[j], d[i], d[j]);
                if da >= 0.0 {
                    keep.push(a);
                } else {
                    drop.push(a);
                }
                if (da >= 0.0) != (db >= 0.0) {
                    let c = lerp(a, b, da / (da - db));
                    keep.push(c);
                    drop.push(c);
                }
            }
            push(&mut outside, &drop);
            poly = keep;
            if poly.len() < 3 {
                break;
            }
        }
        push(&mut inside, &poly);
    }
    (inside, outside)
}

/// The bodies an x-ray may see through: anything but the bend being made, the walls either
/// side of it, a punch's tip (the contact), the floor, a witness and what is drawn red.
fn xray_candidate(x: &Body, zone: Option<([f32; 3], [f32; 3])>) -> bool {
    let touches = |m: &Mesh| match (zone, m.bounds()) {
        (Some((zl, zh)), Some((a, c))) => (0..3).all(|k| a[k] <= zh[k] + 0.5 && c[k] >= zl[k] - 0.5),
        _ => false,
    };
    !x.hit
        && match x.look {
            Look::Sheet => !touches(&x.mesh),
            Look::Tool => !x.name.ends_with("/tip"),
            Look::Frame | Look::Gauge => true,
            Look::BendZone | Look::Floor | Look::Witness => false,
        }
}

impl BendView {
    /// The outline of what is seen through, as world triangles: thin (1 px) low-contrast lines
    /// facing this eye, as strong as the body has faded. The view's frame and a capture's (Render Studio's recorder) both draw them.
    pub fn outline_tris(
        &self,
        camera: &crate::camera::Camera,
        size: (u32, u32),
    ) -> Vec<crate::origin::WorldVert> {
        let mut out = Vec::new();
        for (e, level) in &self.outline {
            crate::origin::polyline(e, 1.0, [0.78, 0.81, 0.86, 0.45 * level], camera, size, &mut out);
        }
        out
    }

    /// The names of what is drawn now: a body seen through inside the x-ray window is drawn as
    /// its solid rest (its own name) and its window piece (`name#xray`).
    pub fn drawn_names(&self) -> Vec<String> {
        self.bodies.iter().map(|x| x.name.clone()).collect()
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
        let key = (sim as *const _ as usize, step, b.t.to_bits());
        if self.bend.shown == Some(key) && b.frame.is_none() {
            self.bendsim_see_through();
            return;
        }
        let mut sc = forge_brake::scene::Scene::new(&sim.model, &sim.tooling, &sim.plan);
        if let Some(h) = b.table_height {
            sc.table_height = h;
        }
        let bodies = sc.bodies(step, b.t);
        // framed: on entry, when asked (the View tools, F), and when the part moves to another
        // station (a new subject); held through the step otherwise
        let at = {
            let st = &sim.plan.steps[step].setup;
            sim.tooling.station(st.punch, st.die).map(|x| x.y.to_bits())
        };
        let ask = b.frame;
        // with Follow bend, each new step is framed afresh, and through the operator's handling
        // the eye widens to keep the part in view, settling back on the working zone as it
        // returns to the tools; otherwise the eye is the user's
        let moved = self.bend.framed && b.follow && self.bend.step != Some(step);
        let handling = b.follow && stage_at(b.t).0 == forge_brake::scene::Stage::Handle;
        let first = !self.bend.framed;
        let mut eye = None;
        if first || ask.is_some() || moved || (handling && self.bend.framed) {
            let how = ask.unwrap_or(Framing::Work);
            // the part's whole motion through the step at the tools, from gauging to the ram's
            // return (kept for the step), and where the part is now
            let envelope = match how {
                Framing::Work => {
                    let kept = self.bend.envelope.filter(|e| e.0 == step).map(|e| (e.1, e.2));
                    let env = kept.or_else(|| {
                        let samples: Vec<Vec<Body>> = [0.2, 0.35, 0.5, 0.6, 0.7, 0.78, 0.9]
                            .iter()
                            .map(|&tt| sc.bodies(step, tt))
                            .collect();
                        bounds(
                            samples
                                .iter()
                                .flatten()
                                .filter(|x| x.look.is_sheet())
                                .map(|x| &x.mesh),
                        )
                    });
                    if let Some((a, c)) = env {
                        self.bend.envelope = Some((step, a, c));
                    }
                    let now = bounds(bodies.iter().filter(|x| x.look.is_sheet()).map(|x| &x.mesh));
                    match (env, now) {
                        (Some((a, c)), Some((p, q))) => Some((
                            [0, 1, 2].map(|k| a[k].min(p[k])),
                            [0, 1, 2].map(|k| c[k].max(q[k])),
                        )),
                        (e, n) => e.or(n),
                    }
                }
                Framing::Overview => None,
            };
            if let Some((cam, lo, hi)) = framing(&bodies, envelope, how, v.size) {
                self.bend.subject = (how == Framing::Work).then_some((lo, hi));
                if how == Framing::Work && !handling {
                    // the cube's home is the working zone
                    v.nav.home_view = Some(cam);
                }
                // tracking the handling is continuous: placed, not travelled to
                eye = Some((cam, first || (handling && !moved && ask.is_none())));
            }
            self.bend.framed = true;
            self.bend.station = at;
            self.bend.step = Some(step);
        }
        if let Some(b) = self.shell.sheet.bend_sim.as_mut() {
            b.frame = None;
        }
        match eye {
            // on entry the eye is placed; asked for or followed, it travels there
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
        self.bendsim_see_through();
    }

    /// What is drawn, kept up with the eye: the instant's bodies, the press's near walls cut
    /// away, and whatever stands between the eye and the bend being made seen through inside the
    /// x-ray window round the bend and the punch's contact (solid outside it), faded in and out.
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
        let key = (eye.map(f32::to_bits), b.see_through, b.opacity.to_bits());
        if self.bend.derived == Some(key) && !self.bend.fading {
            return;
        }
        self.bend.derived = Some(key);
        let m = cam.view(v.size.0.max(1), v.size.1.max(1)).matrix;
        // the cut-away, for this eye and the press as it stands at this instant (the ram comes
        // down through the picture as a bend forms: it is cut as it does)
        self.bend.cut = match self.bend.subject {
            Some((lo, hi)) => cutaway(&self.bend.raw, &cam, v.size, lo, hi),
            None => Vec::new(),
        };
        let cut = self.bend.cut.clone();
        let working: Vec<&Body> = self.bend.raw.iter().filter(|x| !cut.contains(&x.name)).collect();
        // the x-ray: each candidate split by the window; what lies inside it, nearer than the
        // bend, blocks it
        let zone = bounds(
            working
                .iter()
                .filter(|x| x.look == Look::BendZone)
                .map(|x| &x.mesh),
        );
        // the window is worked out even with see-through off: what is still fading out keeps
        // its window piece until it is solid again
        let window = xray_window(&self.bend.raw, &m);
        let see = b.see_through;
        let mut split: std::collections::HashMap<String, (Mesh, Mesh)> = Default::default();
        if let Some((win, wmax)) = window {
            for x in working.iter().filter(|x| xray_candidate(x, zone)) {
                let (inside, outside) = xray_split(&x.mesh, &m, win, wmax);
                if !inside.tri.is_empty() {
                    split.insert(x.name.clone(), (inside, outside));
                }
            }
        }
        // hysteresis and a fade, counted in frames (deterministic when recorded): a body must
        // block for FADE_AFTER frames running before it starts to fade, and fades in or out over
        // FADE_FRAMES frames, so a body grazing the line of sight does not flicker
        let mut moving = false;
        let names: Vec<String> = working.iter().map(|x| x.name.clone()).collect();
        self.bend.fade.retain(|n, _| names.contains(n));
        for n in &names {
            let e = self.bend.fade.entry(n.clone()).or_insert((0, 0.0));
            let blocking = see && split.contains_key(n);
            if !split.contains_key(n) {
                // nothing of it in the window: nothing to show; it comes in again from solid
                *e = (0, 0.0);
                continue;
            }
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
        // what is drawn: a seen-through body as its solid rest and its window piece
        let mut drawn: Vec<Body> = Vec::new();
        let mut xray: Vec<(usize, f32)> = Vec::new();
        for x in &working {
            let level = self.bend.fade.get(&x.name).map_or(0.0, |f| f.1);
            match split.get(&x.name) {
                Some((inside, outside)) if level > 0.0 => {
                    if !outside.tri.is_empty() {
                        drawn.push(Body {
                            mesh: outside.clone(),
                            ..(*x).clone()
                        });
                    }
                    xray.push((drawn.len(), level));
                    drawn.push(Body {
                        name: format!("{}#xray", x.name),
                        mesh: inside.clone(),
                        ..(*x).clone()
                    });
                }
                _ => drawn.push((*x).clone()),
            }
        }
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
        // the window pieces as ghosts: from solid down to the opacity set as they fade, a thin,
        // low-contrast outline coming in with them
        self.bend.ghosts.clear();
        let mut outline: Vec<([[f32; 3]; 2], f32)> = Vec::new();
        for &(k, level) in &xray {
            v.scene.set_ghost(k, true);
            let c = tint(&drawn[k], material);
            let to8 = |x: f32| (x.clamp(0.0, 1.0) * 255.0).round() as u8;
            let alpha = 1.0 - level * (1.0 - opacity);
            self.bend.ghosts.push(crate::render::FaceColour {
                placement: k as u32,
                face: 0,
                rgba: [to8(c[0]), to8(c[1]), to8(c[2]), to8(alpha)],
            });
            outline.extend(silhouette(&drawn[k].mesh, eye).into_iter().map(|e| (e, level)));
        }
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
