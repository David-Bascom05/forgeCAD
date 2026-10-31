//! Draws with no window. A machine with no GPU (CI) has nothing to test here and says so.

use forge_render::camera::Camera;
use forge_render::mesh::PartMesh;
use forge_render::render::{self, Offscreen, Renderer, part_info};
use forge_render::scene::{self, Placement, Scene};

fn renderer(parts: &[PartMesh], scene: &Scene, size: (u32, u32)) -> Option<Renderer> {
    let instance = render::native_instance();
    let (device, queue, adapter) = match render::block_on(render::open_device(&instance, None)) {
        Ok(x) => x,
        Err(e) => {
            eprintln!("skipped: {e}");
            return None;
        }
    };
    Some(Renderer::new(
        device,
        queue,
        adapter.get_info(),
        render::OFFSCREEN,
        size,
        parts,
        scene,
    ))
}

fn crank() -> PartMesh {
    forge_render::load::open("crank", false).unwrap().0
}

#[test]
fn a_part_fills_part_of_the_frame_with_shades_and_lines() {
    let parts = [crank()];
    let mut scene = Scene::new(&part_info(&parts), vec![Placement::at(0, [0.0; 3])]);
    let size = (320, 200);
    let Some(mut r) = renderer(&parts, &scene, size) else {
        return;
    };
    let target = Offscreen::new(&r.device, size);
    let (center, radius) = scene.bounds();
    let drawn = r.frame(&target.view, &mut scene, &Camera::fit(center, radius), None);
    r.wait();
    assert_eq!(drawn.placements, 1);
    assert!(drawn.tris > 0 && drawn.segments > 0);
    let rgb = target.read_rgb(&r);
    let px = rgb.as_chunks::<3>().0;
    let part = px.iter().filter(|p| **p != px[0]).count();
    let lines = px.iter().filter(|p| p[0] < 120).count();
    assert!(part > 2000, "only {part} pixels off the background");
    assert!(lines > 50, "only {lines} dark pixels");
}

#[test]
fn looking_away_draws_nothing() {
    let parts = [crank()];
    let mut scene = Scene::new(&part_info(&parts), scene::stress(1, 50, 300.0));
    let size = (64, 64);
    let Some(mut r) = renderer(&parts, &scene, size) else {
        return;
    };
    let target = Offscreen::new(&r.device, size);
    let (center, radius) = scene.bounds();
    let mut away = Camera::fit([center[0] + 100.0 * radius, center[1], center[2]], radius);
    away.yaw = core::f32::consts::PI;
    away.pitch = 0.0;
    let drawn = r.frame(&target.view, &mut scene, &away, None);
    r.wait();
    assert_eq!(drawn.placements, 0);
}
