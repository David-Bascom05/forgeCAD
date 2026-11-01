//! Navigation from real window events: left drag orbits about the point under the cursor, middle
//! drag pans it with the cursor, the wheel zooms at it; shift + middle orbits too. The right button
//! is the marking menu's: a click brings it up, a flick chooses from it. The view
//! cube's faces, edges and corners go to their views, smoothly, its ring spins about up. Nothing
//! here touches the window or the GPU: what's under the cursor comes back through `on_pick`.

use crate::camera::{Camera, PITCH_END, dot};
use crate::render::{Entity, Mark, PickHit};
use crate::scene::Scene;
use crate::viewcube::{self, Hit, Orient, Place};
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::keyboard::{KeyCode, ModifiersState};

/// The opening view, and Home's: the cube's front, right, top corner, dead on.
pub const HOME_CORNER: [i8; 3] = [1, -1, 1];
/// A press that moves less than this is a click (pixels).
const CLICK_SLOP: f32 = 4.0;
/// How long a view change animates (ms).
const TRANSITION_MS: f64 = 280.0;
const CUBE_MARGIN_PX: f32 = 8.0;
/// A wheel step of the section, as a share of the view's distance.
const SECTION_STEP: f32 = 0.01;
/// How long exploding or coming back together takes (ms).
const EXPLODE_MS: f64 = 500.0;
/// What the left button drags in the view.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NavTool {
    #[default]
    Orbit,
    Pan,
    Zoom,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Drag {
    Orbit,
    Pan,
    Zoom,
    Cube,
    Ring,
    /// The right button: nothing moves; where it ends says what's meant.
    Mark,
}

struct Transition {
    from: Camera,
    to: Camera,
    start: Option<f64>,
}

pub struct Nav {
    pub camera: Camera,
    pub size: (u32, u32),
    pub scale: f32,
    cursor: (f32, f32),
    modifiers: ModifiersState,
    drag: Option<Drag>,
    press_at: (f32, f32),
    press_button: MouseButton,
    moved: bool,
    /// What's under the cursor, as of a pick there.
    hit: Option<PickHit>,
    hit_at: Option<(f32, f32)>,
    pivot: Option<[f32; 3]>,
    pan_depth: Option<f32>,
    want_pick: Option<(f32, f32)>,
    want_home: bool,
    pub tool: NavTool,
    /// Clicks that select, so far: the host's other things in the view can answer one.
    pub clicks: u32,
    transition: Option<Transition>,
    /// What of the view cube's under the cursor.
    pub cube_hover: Option<Hit>,
    /// The view its home button goes to; none: fit the part from the opening angle.
    pub home_view: Option<Camera>,
    /// It was right-clicked here: the host's menu goes up, once.
    pub cube_menu: Option<(f32, f32)>,
    /// What part of the cube that click was on.
    pub cube_menu_region: Option<[i8; 3]>,
    /// How the part's axes are named on the cube.
    pub orient: Orient,
    /// The right button in the view, from here to there: taken once by the host.
    pub marked: Option<((f32, f32), (f32, f32))>,
    /// The section: which axis it's across (0 x, 1 y, 2 z), and where along it.
    pub section: Option<(usize, f32)>,
    /// How far apart the placements are heading, and are: both 0 to 1.
    pub explode_to: f32,
    pub explode_at: f32,
    explode_time: Option<f64>,
    /// What's been clicked, in order.
    pub selected: Vec<(u32, Entity)>,
}

/// The view from a region of the cube, with the part's axes named this way: the named TOP is up, and
/// straight onto TOP or BOTTOM, FRONT is below or above it.
pub fn region_view_as(camera: Camera, region: [i8; 3], orient: &Orient) -> Camera {
    if *orient == Orient::default() {
        return region_view(camera, region);
    }
    let v = region.map(|x| x as f32);
    let l = dot(v, v).sqrt();
    let f = v.map(|x| -x / l);
    let top = orient.world([0, 0, 1]).map(|x| x as f32);
    let up = match dot(f, top) {
        d if d < -0.999 => orient.world([0, 1, 0]).map(|x| x as f32),
        d if d > 0.999 => orient.world([0, -1, 0]).map(|x| x as f32),
        _ => top,
    };
    let r = [
        f[1] * up[2] - f[2] * up[1],
        f[2] * up[0] - f[0] * up[2],
        f[0] * up[1] - f[1] * up[0],
    ];
    let n = dot(r, r).sqrt();
    let mut c = camera;
    c.look(f, r.map(|x| x / n));
    c
}

/// The view from a region of the cube.
pub fn region_view(mut camera: Camera, region: [i8; 3]) -> Camera {
    let [a, b, c] = region.map(|v| v as f32);
    let flat = (a * a + b * b).sqrt();
    camera.pitch = c.atan2(flat).clamp(-PITCH_END, PITCH_END);
    camera.yaw = if flat > 0.0 {
        b.atan2(a)
    } else {
        // straight up or down: FRONT below TOP, and above BOTTOM, so their words read upright
        -core::f32::consts::FRAC_PI_2
    };
    camera.roll = 0.0;
    camera
}

impl Nav {
    pub fn new(camera: Camera, size: (u32, u32), scale: f32) -> Nav {
        Nav {
            camera,
            size,
            scale,
            cursor: (0.0, 0.0),
            modifiers: ModifiersState::empty(),
            drag: None,
            press_at: (0.0, 0.0),
            press_button: MouseButton::Left,
            moved: false,
            hit: None,
            hit_at: None,
            pivot: None,
            pan_depth: None,
            want_pick: None,
            want_home: false,
            tool: NavTool::Orbit,
            clicks: 0,
            transition: None,
            cube_hover: None,
            home_view: None,
            cube_menu: None,
            cube_menu_region: None,
            orient: Orient::default(),
            marked: None,
            selected: Vec::new(),
            section: None,
            explode_to: 0.0,
            explode_at: 0.0,
            explode_time: None,
        }
    }

    /// Where the view cube is: the top right of the view.
    pub fn cube_place(&self) -> Place {
        let side = viewcube::WIDGET_PX * self.scale;
        Place {
            at: (
                self.size.0 as f32 - side - CUBE_MARGIN_PX * self.scale,
                CUBE_MARGIN_PX * self.scale,
            ),
            side,
            scale: self.scale,
        }
    }

    fn cube_at(&self, px: (f32, f32)) -> Option<Hit> {
        viewcube::hit(&self.camera, &self.cube_place(), px)
    }

    /// The pixel a pick is wanted for, once.
    pub fn take_pick(&mut self) -> Option<(f32, f32)> {
        self.want_pick.take()
    }

    /// What's marked this frame: the selection, and what's under the cursor unless dragging.
    pub fn marks(&self) -> Vec<Mark> {
        let mut out: Vec<Mark> = self
            .selected
            .iter()
            .map(|&(placement, entity)| Mark {
                placement,
                entity,
                selected: true,
            })
            .collect();
        if self.drag.is_none()
            // the last answer stands till the next one comes: the mark doesn't blink as the pointer moves
            && let Some(h) = self.hit.filter(|_| self.hit_at.is_some())
            && !self.selected.contains(&(h.placement, h.entity))
        {
            out.push(Mark {
                placement: h.placement,
                entity: h.entity,
                selected: false,
            });
        }
        out
    }

    /// What's marked, in words for the overlay.
    pub fn marked_text(&self) -> String {
        let marks = self.marks();
        let Some(m) = marks.iter().rfind(|m| !m.selected).or(marks.last()) else {
            return String::new();
        };
        let (kind, id) = match m.entity {
            Entity::Face(i) => ("face", i),
            Entity::Edge(i) => ("edge", i),
            Entity::Vertex(i) => ("vertex", i),
        };
        let chosen = marks.iter().filter(|m| m.selected).count();
        format!("{kind} {id} of part {} {chosen} selected", m.placement)
    }

    /// What's under the pixel a pick was made for; true if that changes what's drawn.
    pub fn on_pick(&mut self, px: (f32, f32), hit: Option<PickHit>) -> bool {
        let before = self.marks();
        self.hit = hit;
        self.hit_at = Some(px);
        // the answer for this drag's press, even if the drag has moved on since
        if self
            .drag
            .is_some_and(|d| !matches!(d, Drag::Cube | Drag::Ring | Drag::Mark))
            && px == self.press_at
        {
            self.pivot = hit.map(|h| h.point);
            self.pan_depth = hit.map(|h| h.depth);
        }
        self.marks() != before
    }

    fn known_hit(&self) -> Option<PickHit> {
        (self.hit_at == Some(self.cursor)).then_some(self.hit).flatten()
    }

    /// What one window event does; true if the view needs drawing again.
    pub fn event(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::ModifiersChanged(m) => {
                self.modifiers = m.state();
                false
            }
            WindowEvent::Resized(s) => {
                self.size = (s.width.max(1), s.height.max(1));
                true
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.scale = *scale_factor as f32;
                true
            }
            WindowEvent::MouseInput { state, button, .. } => match state {
                // a held button hides the hover mark, so these redraw
                ElementState::Pressed => self.press(*button) || self.known_hit().is_some(),
                ElementState::Released => self.release() || self.known_hit().is_some(),
            },
            WindowEvent::CursorMoved { position, .. } => {
                self.moved_to((position.x as f32, position.y as f32))
            }
            WindowEvent::CursorLeft { .. } => {
                let was = self.cube_hover.is_some();
                self.cube_hover = None;
                self.hit_at = None;
                self.drag = None;
                was
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let steps = match delta {
                    MouseScrollDelta::LineDelta(_, y) => *y,
                    MouseScrollDelta::PixelDelta(p) => p.y as f32 / 40.0,
                };
                self.wheel(steps)
            }
            _ => false,
        }
    }

    fn press(&mut self, button: MouseButton) -> bool {
        self.transition = None;
        self.press_at = self.cursor;
        self.press_button = button;
        self.moved = false;
        if let Some(h) = self.cube_at(self.cursor) {
            if button == MouseButton::Right {
                self.cube_menu = Some(self.cursor);
                self.cube_menu_region = match h {
                    Hit::Region(r) => Some(r),
                    _ => None,
                };
                return false;
            }
            self.drag = Some(if h == Hit::Ring { Drag::Ring } else { Drag::Cube });
            return false;
        }
        let shift = self.modifiers.shift_key();
        self.drag = Some(match button {
            MouseButton::Left => match self.tool {
                NavTool::Orbit => Drag::Orbit,
                NavTool::Pan => Drag::Pan,
                NavTool::Zoom => Drag::Zoom,
            },
            MouseButton::Right => Drag::Mark,
            MouseButton::Middle if shift => Drag::Orbit,
            _ => Drag::Pan,
        });
        // what's under the cursor may be known already from hovering
        if self.hit_at == Some(self.cursor) {
            self.pivot = self.hit.map(|h| h.point);
            self.pan_depth = self.hit.map(|h| h.depth);
        } else {
            self.pivot = None;
            self.pan_depth = None;
            self.want_pick = Some(self.cursor);
        }
        false
    }

    fn release(&mut self) -> bool {
        let drag = self.drag.take();
        if drag == Some(Drag::Mark) {
            self.marked = Some((self.press_at, self.cursor));
            self.moved = false;
            return false;
        }
        if self.moved {
            self.want_pick = Some(self.cursor);
            return false;
        }
        match drag {
            Some(Drag::Cube) => {
                let to = match self.cube_at(self.cursor) {
                    Some(Hit::Region(r)) => region_view_as(self.camera, r, &self.orient),
                    Some(Hit::Roll(clockwise)) => {
                        let mut c = self.camera;
                        let turn = core::f32::consts::FRAC_PI_2;
                        c.roll = ((c.roll + if clockwise { turn } else { -turn }) / turn).round() * turn;
                        c
                    }
                    Some(Hit::Step(x, y)) => {
                        // the eye swings a quarter turn to the side that was shown
                        let (f, r, u) = self.camera.axes();
                        let (nf, nr) = match (x, y) {
                            (1, _) => (r.map(|v| -v), f),
                            (-1, _) => (r, f.map(|v| -v)),
                            (_, 1) => (u.map(|v| -v), r),
                            _ => (u, r),
                        };
                        let mut c = self.camera;
                        c.look(nf, nr);
                        c
                    }
                    Some(Hit::Home) => {
                        self.want_home = true;
                        return true;
                    }
                    _ => return false,
                };
                self.go_to(to);
                true
            }
            // a click with a tool up still selects
            Some(Drag::Orbit | Drag::Pan | Drag::Zoom) if matches!(self.press_button, MouseButton::Left) => {
                self.clicks += 1;
                let got = self.known_hit().map(|h| (h.placement, h.entity));
                if self.modifiers.control_key() || self.modifiers.shift_key() {
                    // adds to the selection, or takes off what's in it
                    if let Some(g) = got {
                        match self.selected.iter().position(|s| *s == g) {
                            Some(at) => {
                                self.selected.remove(at);
                            }
                            None => self.selected.push(g),
                        }
                    }
                } else {
                    self.selected = got.into_iter().collect();
                }
                true
            }
            _ => false,
        }
    }

    fn moved_to(&mut self, at: (f32, f32)) -> bool {
        let (dx, dy) = (at.0 - self.cursor.0, at.1 - self.cursor.1);
        self.cursor = at;
        let Some(drag) = self.drag else {
            let hover = self.cube_at(at);
            let changed = hover != self.cube_hover;
            self.cube_hover = hover;
            if hover.is_none() {
                self.want_pick = Some(at);
            }
            // what's marked was for the pixel left behind
            return changed || self.hit.is_some();
        };
        if !self.moved {
            let (px, py) = (at.0 - self.press_at.0, at.1 - self.press_at.1);
            if px * px + py * py < CLICK_SLOP * CLICK_SLOP {
                return false;
            }
            self.moved = true;
        }
        match drag {
            Drag::Mark => return false,
            Drag::Orbit => {
                let pivot = self.pivot.unwrap_or(self.camera.target);
                self.camera.orbit_about(pivot, dx, dy);
            }
            Drag::Cube => {
                self.camera.orbit_about(self.camera.target, dx, dy);
            }
            Drag::Ring => {
                // the view turns about the screen's axis, following the cursor round the ring
                let p = self.cube_place();
                let c = (p.at.0 + p.side * 0.5, p.at.1 + p.side * 0.5);
                let a = (at.1 - c.1).atan2(at.0 - c.0) - (at.1 - dy - c.1).atan2(at.0 - dx - c.0);
                self.camera.roll += a;
            }
            Drag::Zoom => {
                let at = self
                    .camera
                    .point_at(self.press_at, self.camera.dist, self.size.0, self.size.1);
                self.camera.zoom_at(at, -dy * 0.05);
            }
            Drag::Pan => {
                let depth = self.pan_depth.unwrap_or(self.camera.dist);
                self.camera.pan(dx, dy, depth, self.size.1);
            }
        }
        true
    }

    /// The cutting plane in the renderer's form.
    pub fn section_plane(&self) -> Option<[f32; 4]> {
        self.section.map(|(axis, at)| {
            let mut p = [0.0; 4];
            p[axis] = 1.0;
            p[3] = -at;
            p
        })
    }

    fn wheel(&mut self, steps: f32) -> bool {
        // ctrl + wheel moves the section along its axis, by a share of the view
        if self.modifiers.control_key()
            && let Some((_, at)) = &mut self.section
        {
            *at += steps * self.camera.dist * SECTION_STEP;
            self.hit_at = None;
            return true;
        }
        self.transition = None;
        // the point under the cursor, or the target's depth there, stays put
        let at = self.known_hit().map_or_else(
            || {
                self.camera
                    .point_at(self.cursor, self.camera.dist, self.size.0, self.size.1)
            },
            |h| h.point,
        );
        self.camera.zoom_at(at, steps);
        if self.hit_at != Some(self.cursor) {
            self.want_pick = Some(self.cursor);
        }
        true
    }

    /// Starts a smooth move to a view; any mouse press or wheel stops it where it is.
    pub fn go_to(&mut self, to: Camera) {
        self.transition = Some(Transition {
            from: self.camera,
            to,
            start: None,
        });
    }

    pub fn fit_all(&mut self, scene: &Scene) {
        let (c, r) = scene.bounds();
        self.fit_sphere(c, r);
    }

    pub fn fit_sphere(&mut self, center: [f32; 3], radius: f32) {
        let mut to = self.camera;
        to.target = center;
        to.fit_to(radius, self.size.0, self.size.1);
        self.go_to(to);
    }

    /// The opening view of everything.
    pub fn home(&mut self, scene: &Scene) {
        let (c, r) = scene.bounds();
        let to = self.home_view.unwrap_or_else(|| {
            let mut to = Camera::fit(c, r);
            to = region_view_as(to, self.orient.world(HOME_CORNER), &self.orient);
            if self.camera.parallel {
                to.set_parallel(true, r);
            }
            to.fit_to(r, self.size.0, self.size.1);
            to
        });
        self.go_to(to);
    }

    /// A key's view change, if it has one; true if the view needs drawing again.
    pub fn key(&mut self, key: KeyCode, scene: &Scene) -> bool {
        match key {
            KeyCode::Home | KeyCode::F6 => self.home(scene),
            KeyCode::KeyA => self.fit_all(scene),
            KeyCode::KeyF => {
                let Some(&(s, _)) = self.selected.last() else {
                    return false;
                };
                let (c, r) = scene.sphere(s as usize);
                self.fit_sphere(c, r);
            }
            // S: x, y, z, off
            KeyCode::KeyS => {
                let (c, _) = scene.bounds();
                self.section = match self.section {
                    None => Some((0, c[0])),
                    Some((0, _)) => Some((1, c[1])),
                    Some((1, _)) => Some((2, c[2])),
                    Some(_) => None,
                };
                self.hit_at = None;
            }
            KeyCode::KeyE => {
                self.explode_to = 1.0 - self.explode_to;
                self.explode_time = None;
            }
            KeyCode::Escape => {
                let had = !self.selected.is_empty();
                self.selected.clear();
                return had;
            }
            _ => return false,
        }
        true
    }

    /// Advances exploding to this time; true while it's on its way, with where it's got to.
    pub fn explode_tick(&mut self, now: f64) -> Option<f32> {
        if self.explode_at == self.explode_to {
            return None;
        }
        let last = self.explode_time.unwrap_or(now - 8.0);
        self.explode_time = Some(now);
        let step = ((now - last) / EXPLODE_MS) as f32;
        self.explode_at = if self.explode_to > self.explode_at {
            (self.explode_at + step).min(self.explode_to)
        } else {
            (self.explode_at - step).max(self.explode_to)
        };
        self.hit_at = None;
        let u = self.explode_at;
        Some(u * u * (3.0 - 2.0 * u))
    }

    /// Where the pointer last was in the view.
    pub fn cursor(&self) -> (f32, f32) {
        self.cursor
    }

    /// The cube's home button was pressed: the scene's there for it now.
    pub fn home_if_asked(&mut self, scene: &Scene) -> bool {
        if std::mem::take(&mut self.want_home) {
            self.home(scene);
            return true;
        }
        false
    }

    /// This world face takes this name, keeping what's up on the screen up. Back to the same view,
    /// its word's now that.
    pub fn set_face_as(&mut self, face: [i8; 3], name: [i8; 3]) {
        // the axis nearest the screen's up that's square to the face; else nearest its right
        let (_, right, up) = self.camera.axes();
        let square = |v: [f32; 3]| -> Option<[i8; 3]> {
            let mut best = None;
            let mut most = 0.0;
            for k in 0..3 {
                if face[k] != 0 {
                    continue;
                }
                if v[k].abs() > most {
                    most = v[k].abs();
                    let mut a = [0i8; 3];
                    a[k] = if v[k] < 0.0 { -1 } else { 1 };
                    best = Some(a);
                }
            }
            best.filter(|_| most > 1e-3)
        };
        let axis = square(up).or_else(|| square(right)).unwrap_or([0, 0, 1]);
        self.orient = Orient::with_face(face, name, axis);
    }

    /// Between a parallel and a perspective view, smoothly.
    pub fn set_parallel(&mut self, on: bool, scene: &Scene) {
        let mut to = self.camera;
        to.set_parallel(on, scene.bounds().1);
        self.go_to(to);
    }

    /// Advances a view change to this time; true while one is under way.
    pub fn tick(&mut self, now: f64) -> bool {
        let Some(t) = &mut self.transition else {
            return false;
        };
        let start = *t.start.get_or_insert(now);
        let u = (((now - start) / TRANSITION_MS) as f32).clamp(0.0, 1.0);
        let eased = u * u * (3.0 - 2.0 * u);
        self.camera = t.from.lerp(&t.to, eased);
        if u >= 1.0 {
            self.camera = t.to;
            self.transition = None;
            self.hit_at = None;
            return false;
        }
        true
    }
}
