//! Inventor-style navigation, from real window events: left drag orbits about the point under the
//! cursor, middle or right drag pans it with the cursor, the wheel zooms at it; shift + middle
//! orbits too. The view cube's faces, edges and corners go to their views, smoothly. Nothing here
//! touches the window or the GPU: what's under the cursor comes back through `on_pick`.

use crate::camera::{Camera, PITCH_END};
use crate::render::PickHit;
use crate::scene::Scene;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::keyboard::{KeyCode, ModifiersState};

/// A press that moves less than this is a click (pixels).
const CLICK_SLOP: f32 = 4.0;
/// How long a view change animates (ms).
const TRANSITION_MS: f64 = 280.0;
/// The view cube's square, from the top right corner (pixels, before the window's scale).
const CUBE_PX: f32 = 120.0;
const CUBE_MARGIN_PX: f32 = 12.0;
/// How far a cube's corner sits from its middle, as a share of the viewport's half (render.wgsl).
pub const CUBE_SCALE: f32 = 1.1;
/// How near a cube face's rim counts as its edge or corner (render.wgsl has it too).
const CUBE_RIM: f32 = 0.17;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Drag {
    Orbit,
    Pan,
    Cube,
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
    moved: bool,
    /// What's under the cursor, as of a pick there.
    hit: Option<PickHit>,
    hit_at: Option<(f32, f32)>,
    pivot: Option<[f32; 3]>,
    pan_depth: Option<f32>,
    want_pick: Option<(f32, f32)>,
    transition: Option<Transition>,
    /// The cube's region under the cursor: each axis -1, 0 or 1; all zero is none.
    pub cube_hover: [i8; 3],
    /// The placement last clicked.
    pub selected: Option<u32>,
}

/// Where a pixel falls on the view cube, as a region; None if off it.
pub fn cube_region(camera: &Camera, px: (f32, f32), box_at: (f32, f32), box_px: f32) -> Option<[i8; 3]> {
    let (sx, sy) = (
        ((px.0 - box_at.0) / box_px * 2.0 - 1.0) * CUBE_SCALE,
        (1.0 - (px.1 - box_at.1) / box_px * 2.0) * CUBE_SCALE,
    );
    if sx.abs() > CUBE_SCALE || sy.abs() > CUBE_SCALE {
        return None;
    }
    let (f, r, u) = camera.axes();
    let o = [0, 1, 2].map(|k| r[k] * sx + u[k] * sy);
    // the ray o + f t through the box of half side 0.5
    let (mut near, mut far) = (f32::MIN, f32::MAX);
    for k in 0..3 {
        if f[k].abs() < 1e-6 {
            if o[k].abs() > 0.5 {
                return None;
            }
            continue;
        }
        let (a, b) = ((-0.5 - o[k]) / f[k], (0.5 - o[k]) / f[k]);
        near = near.max(a.min(b));
        far = far.min(a.max(b));
    }
    if near > far {
        return None;
    }
    let q = [0, 1, 2].map(|k| o[k] + f[k] * near);
    Some(q.map(|v| {
        if v > 0.5 - CUBE_RIM {
            1
        } else if v < -0.5 + CUBE_RIM {
            -1
        } else {
            0
        }
    }))
}

/// The view from a region of the cube.
pub fn region_view(mut camera: Camera, region: [i8; 3]) -> Camera {
    let [a, b, c] = region.map(|v| v as f32);
    let flat = (a * a + b * b).sqrt();
    camera.pitch = c.atan2(flat).clamp(-PITCH_END, PITCH_END);
    camera.yaw = if flat > 0.0 {
        b.atan2(a)
    } else {
        // straight up or down: squared to the nearest side
        (camera.yaw / core::f32::consts::FRAC_PI_2).round() * core::f32::consts::FRAC_PI_2
    };
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
            moved: false,
            hit: None,
            hit_at: None,
            pivot: None,
            pan_depth: None,
            want_pick: None,
            transition: None,
            cube_hover: [0; 3],
            selected: None,
        }
    }

    /// The cube's square: top left corner, side, in pixels.
    pub fn cube_box(&self) -> ((f32, f32), f32) {
        let side = CUBE_PX * self.scale;
        (
            (
                self.size.0 as f32 - side - CUBE_MARGIN_PX * self.scale,
                CUBE_MARGIN_PX * self.scale,
            ),
            side,
        )
    }

    fn region_under(&self, px: (f32, f32)) -> Option<[i8; 3]> {
        let (at, side) = self.cube_box();
        cube_region(&self.camera, px, at, side).filter(|r| *r != [0; 3])
    }

    /// The pixel a pick is wanted for, once.
    pub fn take_pick(&mut self) -> Option<(f32, f32)> {
        self.want_pick.take()
    }

    /// What's under the pixel a pick was made for.
    pub fn on_pick(&mut self, px: (f32, f32), hit: Option<PickHit>) {
        self.hit = hit;
        self.hit_at = Some(px);
        // the answer for this drag's press, even if the drag has moved on since
        if self.drag.is_some_and(|d| d != Drag::Cube) && px == self.press_at {
            self.pivot = hit.map(|h| h.point);
            self.pan_depth = hit.map(|h| h.depth);
        }
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
                ElementState::Pressed => self.press(*button),
                ElementState::Released => self.release(),
            },
            WindowEvent::CursorMoved { position, .. } => {
                self.moved_to((position.x as f32, position.y as f32))
            }
            WindowEvent::CursorLeft { .. } => {
                let was = self.cube_hover != [0; 3];
                self.cube_hover = [0; 3];
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
        self.moved = false;
        if self.region_under(self.cursor).is_some() {
            self.drag = Some(Drag::Cube);
            return false;
        }
        let shift = self.modifiers.shift_key();
        self.drag = Some(match button {
            MouseButton::Left => Drag::Orbit,
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
        if self.moved {
            self.want_pick = Some(self.cursor);
            return false;
        }
        match drag {
            Some(Drag::Cube) => {
                if let Some(r) = self.region_under(self.cursor) {
                    self.go_to(region_view(self.camera, r));
                    return true;
                }
                false
            }
            Some(Drag::Orbit) => {
                let was = self.selected;
                self.selected = self.known_hit().map(|h| h.placement);
                was != self.selected
            }
            _ => false,
        }
    }

    fn moved_to(&mut self, at: (f32, f32)) -> bool {
        let (dx, dy) = (at.0 - self.cursor.0, at.1 - self.cursor.1);
        self.cursor = at;
        let Some(drag) = self.drag else {
            let hover = self.region_under(at).unwrap_or([0; 3]);
            let changed = hover != self.cube_hover;
            self.cube_hover = hover;
            if hover == [0; 3] {
                self.want_pick = Some(at);
            }
            return changed;
        };
        if !self.moved {
            let (px, py) = (at.0 - self.press_at.0, at.1 - self.press_at.1);
            if px * px + py * py < CLICK_SLOP * CLICK_SLOP {
                return false;
            }
            self.moved = true;
        }
        match drag {
            Drag::Orbit => {
                let pivot = self.pivot.unwrap_or(self.camera.target);
                self.camera.orbit_about(pivot, dx, dy);
            }
            Drag::Cube => {
                self.camera.orbit_about(self.camera.target, dx, dy);
            }
            Drag::Pan => {
                let depth = self.pan_depth.unwrap_or(self.camera.dist);
                self.camera.pan(dx, dy, depth, self.size.1);
            }
        }
        true
    }

    fn wheel(&mut self, steps: f32) -> bool {
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
        to.dist = to.fit_dist(radius, self.size.0, self.size.1);
        self.go_to(to);
    }

    /// The opening view of everything.
    pub fn home(&mut self, scene: &Scene) {
        let (c, r) = scene.bounds();
        let mut to = Camera::fit(c, r);
        to.dist = to.fit_dist(r, self.size.0, self.size.1);
        self.go_to(to);
    }

    /// A key's view change, if it has one; true if the view needs drawing again.
    pub fn key(&mut self, key: KeyCode, scene: &Scene) -> bool {
        match key {
            KeyCode::Home | KeyCode::F6 => self.home(scene),
            KeyCode::KeyA => self.fit_all(scene),
            KeyCode::KeyF => {
                let Some(s) = self.selected else { return false };
                let (c, r) = scene.sphere(s as usize);
                self.fit_sphere(c, r);
            }
            KeyCode::Escape => {
                let was = self.selected.take();
                return was.is_some();
            }
            _ => return false,
        }
        true
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
