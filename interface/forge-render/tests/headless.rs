//! Draws with no window. A machine with no GPU (CI) has nothing to test here and says so.

use forge_render::camera::Camera;
use forge_render::mesh::PartMesh;
use forge_render::render::{self, Offscreen, Overlay, Renderer, part_info};
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
    let drawn = r.frame(
        &target.view,
        &mut scene,
        &Camera::fit(center, radius),
        Overlay::default(),
    );
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
    let drawn = r.frame(&target.view, &mut scene, &away, Overlay::default());
    r.wait();
    assert_eq!(drawn.placements, 0);
}

/// How many pixels differ between two pictures.
fn differ(a: &[u8], b: &[u8]) -> usize {
    let (a, b) = (a.as_chunks::<3>().0, b.as_chunks::<3>().0);
    a.iter().zip(b).filter(|(x, y)| x != y).count()
}

/// A part dragged is drawn from its faces in their own slots: about what its mesh draws, and a
/// new value changes the picture, the worker's numbers coming back on the frame.
#[test]
fn a_dragged_part_draws_from_its_faces_and_follows_the_value() {
    use forge_agent::live::Event;
    use forge_agent::tools::Part;
    use forge_render::live::{self, Out};
    let src = "w = 60\nbase: extrude 10 rect 0 0 w 40\nhole 8 -xy at 15 20 10 thru\nfillet 2 (edges of base along +z)\n";
    let part = Part::new(src);
    let dim = part.dims().into_iter().find(|d| d.name == "w").unwrap();
    let mesh = forge_render::tess::build_part(part.model.body().unwrap()).unwrap();
    let parts = [mesh];
    let mut scene = Scene::new(&part_info(&parts), vec![Placement::at(0, [0.0; 3])]);
    let size = (320, 200);
    let Some(mut r) = renderer(&parts, &scene, size) else {
        return;
    };
    let target = Offscreen::new(&r.device, size);
    let (center, radius) = scene.bounds();
    let cam = Camera::fit(center, radius * 1.5);
    let mut away = Camera::fit([center[0] + 100.0 * radius, center[1], center[2]], radius);
    away.yaw = core::f32::consts::PI;
    away.pitch = 0.0;
    r.frame(&target.view, &mut scene, &away, Overlay::default());
    r.wait();
    let blank = target.read_rgb(&r);
    r.frame(&target.view, &mut scene, &cam, Overlay::default());
    r.wait();
    let whole = target.read_rgb(&r);

    let mut l = live::start(part);
    let next = |l: &mut forge_agent::live::Live<Out>, r: &mut Renderer, scene: &mut Scene, seq: u64| {
        for _ in 0..20_000 {
            for e in l.poll() {
                if let Event::Frame(f) = e {
                    assert!(f.failure.is_none(), "{:?}", f.failure);
                    if let Some(Out::Delta(d)) = f.out {
                        r.live_apply(0, &d);
                        scene.set_part(0, d.center, d.radius, vec![d.chord]);
                        if f.seq == seq {
                            return d;
                        }
                    }
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        panic!("the worker never answered");
    };
    let warm = next(&mut l, &mut r, &mut scene, 0);
    assert_eq!(warm.added.len(), warm.faces);
    r.live_show(true);
    r.frame(&target.view, &mut scene, &cam, Overlay::default());
    r.wait();
    let faces = target.read_rgb(&r);
    let (a, b) = (differ(&whole, &blank) as f64, differ(&faces, &blank) as f64);
    assert!(
        a > 2000.0 && (a - b).abs() / a < 0.03,
        "{a} pixels from the mesh, {b} from the faces"
    );

    let seq = l.drag(dim.at.op("90".into()));
    let moved = next(&mut l, &mut r, &mut scene, seq);
    assert!(moved.meshed < moved.faces, "{} of {}", moved.meshed, moved.faces);
    assert!(moved.radius > warm.radius);
    r.frame(&target.view, &mut scene, &cam, Overlay::default());
    r.wait();
    let after = target.read_rgb(&r);
    let c = differ(&after, &blank) as f64;
    assert!(c > b * 1.1, "{c} pixels of part, from {b}");
}
