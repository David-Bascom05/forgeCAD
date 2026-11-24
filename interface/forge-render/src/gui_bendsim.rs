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
    /// The working station's place along the bed when framed: the part moved to another
    /// station, the eye follows it there.
    station: Option<u64>,
    /// The press's parts cut away for the framed eye (the Work framing's near walls).
    cut: Vec<String>,
    /// The step last framed (Follow bend re-frames on a new one).
    step: Option<usize>,
    /// The bodies drawn, in placement order: what see-through tests against.
    bodies: Vec<Body>,
    /// See-through: the eye and the settings it was worked out for, and the faces drawn as
    /// ghosts (bodies between the eye and the bend being made).
    seen: Option<([u32; 3], bool, u32, bool)>,
    pub ghosts: Vec<crate::render::FaceColour>,
}

/// The view cube's naming in the simulation: FRONT the operator's side (-x), BACK the
/// backgauge's, TOP up; LEFT and RIGHT along the bed, so along every bend's line (each is made
/// along the bed): those faces give the view down the bend line.
pub fn bend_orient() -> crate::viewcube::Orient {
    crate::viewcube::Orient::with_face([-1, 0, 0], [0, -1, 0], [0, 0, 1])
}

/// Does the segment a-b cross the triangle (p, q, r) strictly between its ends? (Moller-Trumbore)
fn crosses(a: [f32; 3], b: [f32; 3], p: [f32; 3], q: [f32; 3], r: [f32; 3]) -> bool {
    let sub = |u: [f32; 3], v: [f32; 3]| [u[0] - v[0], u[1] - v[1], u[2] - v[2]];
    let cross = |u: [f32; 3], v: [f32; 3]| {
        [
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ]
    };
    let dot = |u: [f32; 3], v: [f32; 3]| u[0] * v[0] + u[1] * v[1] + u[2] * v[2];
    let d = sub(b, a);
    let (e1, e2) = (sub(q, p), sub(r, p));
    let h = cross(d, e2);
    let det = dot(e1, h);
    if det.abs() < 1e-9 {
        return false;
    }
    let f = 1.0 / det;
    let s = sub(a, p);
    let u = f * dot(s, h);
    if !(0.0..=1.0).contains(&u) {
        return false;
    }
    let qv = cross(s, e1);
    let v = f * dot(d, qv);
    if v < 0.0 || u + v > 1.0 {
        return false;
    }
    let t = f * dot(e2, qv);
    t > 1e-4 && t < 0.995
}

/// The bodies to see through from `eye`: the part's own pieces (and with `press`, the frame,
/// beams and backgauge) that stand between the eye and the bend being made (its zone's corners
/// and middle) or the punch's contact with the part (its tip's lowest point). Never the bend
/// zone itself, the tools, or a body drawn red.
pub fn see_through(bodies: &[Body], eye: [f32; 3], press: bool) -> Vec<usize> {
    let mut aims: Vec<[f32; 3]> = Vec::new();
    let zone = bounds(
        bodies
            .iter()
            .filter(|x| x.look == Look::BendZone)
            .map(|x| &x.mesh),
    );
    // the walls either side of the bend being made are the work itself, not what was bent
    // before it: a piece touching the zone is never seen through
    let touches = |m: &Mesh| match (zone, m.bounds()) {
        (Some((zl, zh)), Some((a, c))) => (0..3).all(|k| a[k] <= zh[k] + 0.5 && c[k] >= zl[k] - 0.5),
        _ => false,
    };
    if let Some((lo, hi)) = zone {
        let mid = [0, 1, 2].map(|k| 0.5 * (lo[k] + hi[k]));
        aims.push(mid);
        for y in [lo[1], hi[1]] {
            aims.push([mid[0], y, mid[2]]);
            aims.push([lo[0], y, hi[2]]);
            aims.push([hi[0], y, hi[2]]);
        }
    }
    for x in bodies
        .iter()
        .filter(|x| x.name.starts_with("punch:") && x.name.ends_with("/tip"))
    {
        if let Some(p) = x.mesh.pos.iter().min_by(|a, b| a[2].total_cmp(&b[2])) {
            aims.push(*p);
        }
    }
    if aims.is_empty() {
        return Vec::new();
    }
    bodies
        .iter()
        .enumerate()
        .filter(|(_, x)| !x.hit)
        .filter(|(_, x)| {
            (x.look == Look::Sheet && !touches(&x.mesh))
                || (press && matches!(x.look, Look::Frame | Look::Gauge))
        })
        .filter(|(_, x)| {
            let m = &x.mesh;
            aims.iter().any(|&aim| {
                m.tri.iter().any(|t| {
                    crosses(
                        eye,
                        aim,
                        m.pos[t[0] as usize],
                        m.pos[t[1] as usize],
                        m.pos[t[2] as usize],
                    )
                })
            })
        })
        .map(|(k, _)| k)
        .collect()
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
/// and the subject (its centre and corners), or that hold it (docs/ux/STANDARD.md §2: the eye
/// outside every body, the machine's near walls cut away). The tools and the part are never cut.
pub fn cutaway(bodies: &[Body], eye: [f32; 3], lo: [f32; 3], hi: [f32; 3]) -> Vec<String> {
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
            })
        })
        .map(|x| x.name.clone())
        .collect()
}

impl BendView {
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
        let mut bodies = sc.bodies(step, b.t);
        // framed: on entry, when asked (the View tools, F), and when the part moves to another
        // station (a new subject); held through the step otherwise
        let at = {
            let st = &sim.plan.steps[step].setup;
            sim.tooling.station(st.punch, st.die).map(|x| x.y.to_bits())
        };
        let ask = b.frame;
        // with Follow bend, each new step is framed afresh; otherwise the eye is the user's
        let moved = self.bend.framed && b.follow && self.bend.step != Some(step);
        let first = !self.bend.framed;
        let mut eye = None;
        if first || ask.is_some() || moved {
            let how = ask.unwrap_or(Framing::Work);
            // the part's whole motion through the step, from gauging to the ram's return
            let envelope = match how {
                Framing::Work => {
                    let samples: Vec<Vec<Body>> = [0.2, 0.35, 0.5, 0.6, 0.7, 0.78, 0.9]
                        .iter()
                        .map(|&tt| sc.bodies(step, tt))
                        .collect();
                    bounds(
                        samples
                            .iter()
                            .flatten()
                            .chain(bodies.iter())
                            .filter(|x| x.look.is_sheet())
                            .map(|x| &x.mesh),
                    )
                }
                Framing::Overview => None,
            };
            if let Some((cam, lo, hi)) = framing(&bodies, envelope, how, v.size) {
                self.bend.cut = match how {
                    Framing::Work => cutaway(&bodies, cam.eye(), lo, hi),
                    Framing::Overview => Vec::new(),
                };
                if how == Framing::Work {
                    // the cube's home is the working zone
                    v.nav.home_view = Some(cam);
                }
                eye = Some((cam, first));
            }
            self.bend.framed = true;
            self.bend.station = at;
            self.bend.step = Some(step);
        }
        if !self.bend.cut.is_empty() {
            bodies.retain(|x| !self.bend.cut.contains(&x.name));
        }
        let parts: Vec<PartMesh> = bodies.iter().map(|x| part_of(&x.mesh)).collect();
        let material = b.material.as_deref();
        let placements: Vec<Placement> = bodies
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
        let names: Vec<String> = bodies.iter().map(|x| x.name.clone()).collect();
        let mut scene = Scene::new(&info, placements);
        scene.set_explode(0.0);
        if names == self.bend.names && self.bend.taken.is_some() {
            // the same bodies: new triangles, the same renderer
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
        if let Some(b) = self.shell.sheet.bend_sim.as_mut() {
            b.frame = None;
        }
        match eye {
            // on entry the eye is placed; asked for or followed, it travels there
            Some((cam, true)) => v.nav.jump_to(cam),
            Some((cam, false)) => v.nav.go_to(cam),
            None => {}
        }
        self.bend.bodies = bodies;
        self.bend.seen = None;
        // a failure located: the eye turned on its place, at its distance
        if let Some(at) = self.shell.sheet.bend_sim.as_mut().and_then(|b| b.locate.take()) {
            let mut cam = v.nav.camera;
            cam.target = at.map(|x| x as f32);
            cam.dist = cam.dist.min(400.0);
            v.nav.jump_to(cam);
        }
        self.bend.ours = Some(v.renderer.geometry());
        self.bend.shown = Some(key);
        v.dirty = true;
        self.bendsim_see_through();
    }

    /// See-through, kept up with the eye: the bodies between it and the bend being made drawn
    /// as ghosts at the panel's opacity; the rest as themselves.
    fn bendsim_see_through(&mut self) {
        let Some(v) = &mut self.view else { return };
        let Some(b) = self.shell.sheet.bend_sim.as_ref() else {
            return;
        };
        let eye = v.nav.camera.eye();
        let key = (
            eye.map(f32::to_bits),
            b.see_through,
            b.opacity.to_bits(),
            b.see_press,
        );
        if self.bend.seen == Some(key) {
            return;
        }
        self.bend.seen = Some(key);
        let ghosts = if b.see_through {
            see_through(&self.bend.bodies, eye, b.see_press)
        } else {
            Vec::new()
        };
        self.bend.ghosts.clear();
        let material = b.material.as_deref();
        // their outlines, for the panel to draw over the view (a ghost must read, not vanish)
        let outlines: Vec<Vec<[f32; 3]>> = ghosts
            .iter()
            .map(|&k| {
                let m = &self.bend.bodies[k].mesh;
                let step = (m.pos.len() / 600).max(1);
                m.pos.iter().step_by(step).copied().collect()
            })
            .collect();
        for k in 0..self.bend.bodies.len() {
            let on = ghosts.contains(&k);
            v.scene.set_ghost(k, on);
            if on {
                let c = tint(&self.bend.bodies[k], material);
                let to8 = |x: f32| (x.clamp(0.0, 1.0) * 255.0).round() as u8;
                self.bend.ghosts.push(crate::render::FaceColour {
                    placement: k as u32,
                    face: 0,
                    rgba: [to8(c[0]), to8(c[1]), to8(c[2]), to8(b.opacity)],
                });
            }
        }
        v.dirty = true;
        if let Some(b) = self.shell.sheet.bend_sim.as_mut() {
            b.outlines = outlines;
        }
    }

    fn bendsim_put_back(&mut self) {
        self.bend.framed = false;
        self.bend.cut.clear();
        self.bend.shown = None;
        self.bend.names.clear();
        self.bend.ours = None;
        self.bend.bodies.clear();
        self.bend.ghosts.clear();
        self.bend.seen = None;
        self.bend.step = None;
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
