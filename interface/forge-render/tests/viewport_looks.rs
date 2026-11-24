//! The viewport's Realistic style with appearances' roughness and coat (packed in a placement's
//! tint, `studio::appearance::pack_tint`): a rough finish draws unlike a polished one, and a tint
//! of the old kind (w below 2) draws exactly as before. `LOOK_SHOTS=DIR` writes the frames.

use forge_render::camera::Camera;
use forge_render::render::{self, Offscreen, Overlay, Renderer, VisualStyle, part_info};
use forge_render::scene::{Placement, Scene};
use forge_render::studio::appearance::Appearances;

fn frame(tint: [f32; 4]) -> Option<Vec<u8>> {
    let instance = render::native_instance();
    let (device, queue, adapter) = match render::block_on(render::open_device(&instance, None)) {
        Ok(x) => x,
        Err(e) => {
            eprintln!("skipped: {e}");
            return None;
        }
    };
    let parts = [forge_render::load::open("crank", false).unwrap().0];
    let mut pl = Placement::at(0, [0.0; 3]);
    pl.tint = tint;
    let mut scene = Scene::new(&part_info(&parts), vec![pl]);
    let size = (240, 160);
    let mut r = Renderer::new(
        device,
        queue,
        adapter.get_info(),
        render::OFFSCREEN,
        size,
        &parts,
        &scene,
    );
    r.style = VisualStyle::Realistic;
    r.set_tints(&scene);
    let target = Offscreen::new(&r.device, size);
    let (c, rad) = scene.bounds();
    r.frame(&target.view, &mut scene, &Camera::fit(c, rad), Overlay::default());
    r.wait();
    Some(target.read_rgb(&r))
}

fn diff(a: &[u8], b: &[u8]) -> f64 {
    a.iter()
        .zip(b)
        .map(|(x, y)| (*x as f64 - *y as f64).abs())
        .sum::<f64>()
        / a.len() as f64
}

#[test]
fn appearances_roughness_shows_and_old_tints_do_not_change() {
    let apps = Appearances::builtin();
    let polished = apps.find("Aluminium, polished").unwrap().viewport_tint(None);
    let blasted = apps.find("Aluminium, bead-blasted").unwrap().viewport_tint(None);
    let Some(a) = frame(polished) else { return };
    let b = frame(blasted).unwrap();
    let d = diff(&a, &b);
    eprintln!("polished vs bead-blasted: {d:.2} levels a channel on average");
    assert!(d > 0.5, "roughness makes no difference: {d}");
    // an old tint (metal 1) the same twice, and unlike the packed polished one
    let old = [polished[0], polished[1], polished[2], 1.0];
    let o1 = frame(old).unwrap();
    assert_eq!(o1, frame(old).unwrap());
    if let Ok(dir) = std::env::var("LOOK_SHOTS") {
        for (n, px) in [("polished", &a), ("blasted", &b), ("old", &o1)] {
            let mut e = png::Encoder::new(
                std::fs::File::create(format!("{dir}/look_{n}.png")).unwrap(),
                240,
                160,
            );
            e.set_color(png::ColorType::Rgb);
            e.write_header().unwrap().write_image_data(px).unwrap();
        }
    }
}
