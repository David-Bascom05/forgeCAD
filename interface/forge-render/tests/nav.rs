//! Navigation from real winit window events, with the pick answered by the GPU where it takes one.

use forge_render::camera::{Camera, dot};
use forge_render::mesh::PartMesh;
use forge_render::nav::Nav;
use forge_render::render::{self, Offscreen, Overlay, Renderer, part_info};
use forge_render::scene::{Placement, Scene};
use winit::dpi::PhysicalPosition;
use winit::event::{
    DeviceId, ElementState, Modifiers, MouseButton, MouseScrollDelta, TouchPhase, WindowEvent,
};
use winit::keyboard::{KeyCode, ModifiersState};

const SIZE: (u32, u32) = (800, 600);

fn moved(x: f32, y: f32) -> WindowEvent {
    WindowEvent::CursorMoved {
        device_id: DeviceId::dummy(),
        position: PhysicalPosition::new(x as f64, y as f64),
    }
}

fn button(state: ElementState, button: MouseButton) -> WindowEvent {
    WindowEvent::MouseInput {
        device_id: DeviceId::dummy(),
        state,
        button,
    }
}

fn wheel(steps: f32) -> WindowEvent {
    WindowEvent::MouseWheel {
        device_id: DeviceId::dummy(),
        delta: MouseScrollDelta::LineDelta(0.0, steps),
        phase: TouchPhase::Moved,
    }
}

fn nav() -> Nav {
    Nav::new(Camera::fit([0.0; 3], 100.0), SIZE, 1.0)
}

fn near(a: (f32, f32), b: (f32, f32), tol: f32) {
    assert!((a.0 - b.0).abs() < tol && (a.1 - b.1).abs() < tol, "{a:?} {b:?}");
}

#[test]
fn a_left_drag_orbits_and_a_middle_drag_pans_and_shift_middle_orbits() {
    let mut n = nav();
    let start = n.camera;
    n.event(&moved(400.0, 300.0));
    n.event(&button(ElementState::Pressed, MouseButton::Left));
    assert!(n.take_pick().is_some());
    assert!(n.event(&moved(460.0, 300.0)));
    n.event(&button(ElementState::Released, MouseButton::Left));
    assert!(n.camera.yaw < start.yaw);
    assert_eq!(n.camera.pitch, start.pitch);

    let was = n.camera;
    n.event(&button(ElementState::Pressed, MouseButton::Middle));
    n.event(&moved(460.0, 360.0));
    n.event(&button(ElementState::Released, MouseButton::Middle));
    assert_eq!((n.camera.yaw, n.camera.pitch), (was.yaw, was.pitch));
    assert_ne!(n.camera.target, was.target);

    n.event(&WindowEvent::ModifiersChanged(Modifiers::from(
        ModifiersState::SHIFT,
    )));
    let was = n.camera;
    n.event(&button(ElementState::Pressed, MouseButton::Middle));
    n.event(&moved(520.0, 360.0));
    assert_ne!(n.camera.yaw, was.yaw);
}

#[test]
fn a_press_too_small_to_be_a_drag_moves_nothing() {
    let mut n = nav();
    let start = n.camera;
    n.event(&moved(400.0, 300.0));
    n.event(&button(ElementState::Pressed, MouseButton::Left));
    n.event(&moved(402.0, 301.0));
    n.event(&button(ElementState::Released, MouseButton::Left));
    assert_eq!(n.camera, start);
}

#[test]
fn the_wheel_zooms_at_the_cursor_with_nothing_under_it() {
    let mut n = nav();
    let px = (150.0, 120.0);
    n.event(&moved(px.0, px.1));
    let p = n.camera.point_at(px, n.camera.dist, SIZE.0, SIZE.1);
    let d = n.camera.dist;
    n.event(&wheel(3.0));
    assert!(n.camera.dist < d);
    near(n.camera.pixel_of(p, SIZE.0, SIZE.1).unwrap(), px, 0.1);
}

#[test]
fn clicking_a_cube_face_arrives_at_that_view_smoothly_and_the_wheel_stops_it() {
    let mut n = nav();
    let (at, side) = n.cube_box();
    let mid = (at.0 + side / 2.0, at.1 + side / 2.0);
    let start = n.camera;
    n.event(&moved(mid.0, mid.1));
    assert_ne!(n.cube_hover, [0; 3]);
    let region = n.cube_hover;
    n.event(&button(ElementState::Pressed, MouseButton::Left));
    n.event(&button(ElementState::Released, MouseButton::Left));
    assert!(n.tick(1000.0));
    assert_eq!(n.camera, start);
    assert!(n.tick(1100.0));
    assert_ne!(n.camera, start);
    assert!(!n.tick(1400.0));
    let want = forge_render::nav::region_view(start, region);
    assert!((n.camera.yaw - want.yaw).abs() < 1e-5 && (n.camera.pitch - want.pitch).abs() < 1e-5);

    // from above, the middle of the cube is its top: going there looks straight down
    let mut n = nav();
    n.camera.pitch = 1.2;
    n.event(&moved(mid.0, mid.1));
    assert_eq!(n.cube_hover, [0, 0, 1]);
    n.event(&button(ElementState::Pressed, MouseButton::Left));
    n.event(&button(ElementState::Released, MouseButton::Left));
    n.tick(0.0);
    n.tick(100.0);
    let mid_flight = n.camera;
    n.event(&moved(100.0, 100.0));
    n.event(&wheel(1.0));
    assert!(!n.tick(200.0), "the wheel stopped it");
    assert_eq!(n.camera.pitch, mid_flight.pitch);

    let mut n = nav();
    n.camera.pitch = 1.2;
    n.event(&moved(mid.0, mid.1));
    n.event(&button(ElementState::Pressed, MouseButton::Left));
    n.event(&button(ElementState::Released, MouseButton::Left));
    n.tick(0.0);
    n.tick(1000.0);
    assert!((n.camera.pitch - core::f32::consts::FRAC_PI_2).abs() < 1e-6);
    let (f, _, _) = n.camera.axes();
    assert!(dot(f, [0.0, 0.0, -1.0]) > 0.9999);
}

#[test]
fn a_drag_starting_on_the_cube_orbits_and_lands_on_no_view() {
    let mut n = nav();
    let (at, side) = n.cube_box();
    let mid = (at.0 + side / 2.0, at.1 + side / 2.0);
    let start = n.camera;
    n.event(&moved(mid.0, mid.1));
    n.event(&button(ElementState::Pressed, MouseButton::Left));
    n.event(&moved(mid.0 - 50.0, mid.1));
    n.event(&button(ElementState::Released, MouseButton::Left));
    assert_ne!(n.camera.yaw, start.yaw);
    assert!(!n.tick(0.0));
}

fn block() -> Vec<PartMesh> {
    // a box 100 x 100 x 20, its top at z = 20
    let m = forge_agent::bench::model("extrude 20 rect -50 -50 100 100");
    assert!(m.error.is_none());
    vec![forge_render::tess::build_part(m.body().unwrap()).unwrap()]
}

fn with_gpu(parts: &[PartMesh], scene: &Scene) -> Option<Renderer> {
    let instance = render::native_instance();
    let (device, queue, adapter) = render::block_on(render::open_device(&instance, None)).ok()?;
    Some(Renderer::new(
        device,
        queue,
        adapter.get_info(),
        render::OFFSCREEN,
        SIZE,
        parts,
        scene,
    ))
}

/// Answers the pick the view wants, if it wants one, from the GPU.
fn answer(n: &mut Nav, r: &mut Renderer, scene: &mut Scene) {
    if let Some(px) = n.take_pick() {
        r.pick_start(scene, &n.camera, px);
        r.wait();
        let (at, hit, _) = r.pick_poll(scene).expect("it was waited for");
        n.on_pick(at, hit);
    }
}

#[test]
fn hover_press_and_drag_turn_about_the_real_surface_under_the_cursor() {
    let parts = block();
    let mut scene = Scene::new(&part_info(&parts), vec![Placement::at(0, [0.0; 3])]);
    let Some(mut r) = with_gpu(&parts, &scene) else {
        eprintln!("skipped: no GPU");
        return;
    };
    let target = Offscreen::new(&r.device, SIZE);
    let mut cam = Camera::fit([0.0, 0.0, 10.0], 80.0);
    cam.pitch = 1.2;
    let mut n = Nav::new(cam, SIZE, 1.0);
    r.frame(&target.view, &mut scene, &n.camera, Overlay::default());

    // somewhere on the top face, off centre: ray and plane z = 20 say where it is
    let px = (470.0, 260.0);
    let top = {
        let a = n.camera.point_at(px, 100.0, SIZE.0, SIZE.1);
        let e = n.camera.eye();
        let t = (20.0 - e[2]) / (a[2] - e[2]);
        [0, 1, 2].map(|k| e[k] + (a[k] - e[k]) * t)
    };
    n.event(&moved(px.0, px.1));
    answer(&mut n, &mut r, &mut scene);

    // the zoom keeps the real surface point on its pixel, not a point at the target's depth
    let before = n.camera.pixel_of(top, SIZE.0, SIZE.1).unwrap();
    near(before, px, 0.5);
    n.event(&wheel(2.0));
    near(n.camera.pixel_of(top, SIZE.0, SIZE.1).unwrap(), before, 1.0);

    // a left drag from there swings the eye round that same point
    let eye_to_top = |c: &Camera| {
        let e = c.eye();
        let v = [top[0] - e[0], top[1] - e[1], top[2] - e[2]];
        dot(v, v).sqrt()
    };
    let was = eye_to_top(&n.camera);
    n.event(&button(ElementState::Pressed, MouseButton::Left));
    answer(&mut n, &mut r, &mut scene);
    n.event(&moved(px.0 + 80.0, px.1 + 30.0));
    n.event(&moved(px.0 + 140.0, px.1 + 10.0));
    answer(&mut n, &mut r, &mut scene);
    n.event(&button(ElementState::Released, MouseButton::Left));
    assert!(
        (eye_to_top(&n.camera) - was).abs() < was * 0.01,
        "{} {was}",
        eye_to_top(&n.camera)
    );
}

#[test]
fn a_click_selects_and_the_selection_can_be_zoomed_to() {
    let parts = block();
    let places = vec![Placement::at(0, [0.0; 3]), Placement::at(0, [400.0, 0.0, 0.0])];
    let mut scene = Scene::new(&part_info(&parts), places);
    let Some(mut r) = with_gpu(&parts, &scene) else {
        eprintln!("skipped: no GPU");
        return;
    };
    let (c, rad) = scene.bounds();
    let mut n = Nav::new(Camera::fit(c, rad), SIZE, 1.0);
    let px = n.camera.pixel_of([400.0, 0.0, 20.0], SIZE.0, SIZE.1).unwrap();
    n.event(&moved(px.0, px.1));
    answer(&mut n, &mut r, &mut scene);
    n.event(&button(ElementState::Pressed, MouseButton::Left));
    n.event(&button(ElementState::Released, MouseButton::Left));
    assert_eq!(n.selected.first().map(|s| s.0), Some(1));

    assert!(n.key(KeyCode::KeyF, &scene));
    n.tick(0.0);
    n.tick(1000.0);
    let (center, radius) = scene.sphere(1);
    assert_eq!(n.camera.target, center);
    assert_eq!(n.camera.dist, n.camera.fit_dist(radius, SIZE.0, SIZE.1));

    assert!(n.key(KeyCode::Home, &scene));
    n.tick(2000.0);
    n.tick(3000.0);
    assert_eq!(n.camera.target, c);
    assert!(n.key(KeyCode::Escape, &scene));
    assert!(n.selected.is_empty());

    // nothing under the cursor: the click selects nothing
    n.event(&moved(5.0, 590.0));
    answer(&mut n, &mut r, &mut scene);
    n.event(&button(ElementState::Pressed, MouseButton::Left));
    n.event(&button(ElementState::Released, MouseButton::Left));
    assert!(n.selected.is_empty());
}

#[test]
fn keys_step_the_section_and_explode_and_ctrl_wheel_moves_the_cut() {
    let parts = block();
    let scene = Scene::new(&part_info(&parts), vec![Placement::at(0, [0.0; 3])]);
    let mut n = nav();
    assert!(n.key(KeyCode::KeyS, &scene));
    let at = n.section.expect("on").1;
    assert_eq!(n.section_plane(), Some([1.0, 0.0, 0.0, -at]));
    n.event(&WindowEvent::ModifiersChanged(Modifiers::from(
        ModifiersState::CONTROL,
    )));
    let d = n.camera.dist;
    n.event(&wheel(2.0));
    assert_eq!(n.camera.dist, d, "the camera stayed");
    assert!(n.section.unwrap().1 > at);
    n.key(KeyCode::KeyS, &scene);
    assert_eq!(n.section.unwrap().0, 1);
    n.key(KeyCode::KeyS, &scene);
    n.key(KeyCode::KeyS, &scene);
    assert_eq!(n.section, None);

    assert_eq!(n.explode_tick(0.0), None);
    n.key(KeyCode::KeyE, &scene);
    let mut last = 0.0;
    let mut now = 0.0;
    while let Some(t) = n.explode_tick(now) {
        assert!(t >= last);
        last = t;
        now += 16.0;
    }
    assert_eq!(last, 1.0);
    assert!(now < 700.0);
}
