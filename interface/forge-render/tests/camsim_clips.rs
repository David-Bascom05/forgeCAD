//! Clips of the simulation running in the app, for people to watch: the whole 1400×860 window,
//! driven by the simulation's own words (`Gui::camsim_command`), each frame handed to the app's own
//! encoder (`record::ffmpeg::Video`, the recorder's). `FORGE_CLIPS=DIR` makes them (MP4, ffmpeg
//! found as the recorder finds it); without it these only check that the frames can be driven.
//!
//! `FORGE_CLIPS=DIR cargo test --release --test camsim_clips -- --test-threads 1`

use forge_render::gui::{Gui, Headless};
use std::path::{Path, PathBuf};

const FPS: u32 = 30;
const SIZE: (f32, f32) = (1400.0, 860.0);

fn machined() -> PathBuf {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/machined")).to_path_buf()
}

fn copied(tag: &str, files: &[&str]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("fg-camsim-clip-{}-{tag}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for f in files {
        std::fs::copy(machined().join(f), dir.join(f)).unwrap();
    }
    dir
}

fn ready(g: &Gui) -> bool {
    g.shell
        .cam
        .as_ref()
        .is_some_and(|w| w.run.is_some() && w.running.is_none() && !w.want_run)
        && g.shell.camsim.is_some()
}

fn open(file: &Path) -> Option<Headless> {
    let mut h = match Headless::new(SIZE, 1.0) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("skipped: {e}");
            return None;
        }
    };
    h.step();
    h.gui.open(file.to_str().unwrap());
    let t = std::time::Instant::now();
    while t.elapsed().as_secs_f64() < 180.0 && !h.until(ready) {}
    assert!(ready(&h.gui), "never ran: {}", h.gui.shell.status);
    // the part's own view loads after the run
    for _ in 0..20 {
        h.step();
    }
    Some(h)
}

/// The window as RGBA rows, from its PNG.
fn rgba(h: &Headless) -> (Vec<u8>, (u32, u32)) {
    let png = h.png().unwrap();
    let dec = png::Decoder::new(std::io::Cursor::new(png));
    let mut r = dec.read_info().unwrap();
    let mut buf = vec![0; r.output_buffer_size().unwrap()];
    let info = r.next_frame(&mut buf).unwrap();
    buf.truncate(info.buffer_size());
    let px = match info.color_type {
        png::ColorType::Rgba => buf,
        png::ColorType::Rgb => buf.chunks(3).flat_map(|c| [c[0], c[1], c[2], 255]).collect(),
        c => panic!("a window PNG of {c:?}"),
    };
    (px, (info.width, info.height))
}

/// A clip: `shots` are (seconds of clip, the commands that set the frame at fraction u of that
/// stretch). Frames go to the app's encoder at FPS.
type Stretch<'a> = (f64, &'a dyn Fn(f64) -> String);

fn clip(h: &mut Headless, name: &str, stretches: &[Stretch]) {
    let out = std::env::var("FORGE_CLIPS").ok();
    let mut video = None;
    if let Some(dir) = &out {
        std::fs::create_dir_all(dir).unwrap();
        let ff = forge_render::record::ffmpeg::locate().expect("ffmpeg, for FORGE_CLIPS");
        let (_, size) = rgba(h);
        let path = Path::new(dir).join(format!("{name}.mp4"));
        video =
            Some(forge_render::record::ffmpeg::Video::start(&ff, path.to_str().unwrap(), size, FPS).unwrap());
    }
    // without FORGE_CLIPS: a few frames of each stretch, to check they can be driven
    let t0 = std::time::Instant::now();
    for (secs, at) in stretches {
        let n = if video.is_some() {
            (secs * FPS as f64).round() as usize
        } else {
            3
        };
        for k in 0..n {
            let u = if n > 1 { k as f64 / (n - 1) as f64 } else { 1.0 };
            h.gui.camsim_command(&at(u)).unwrap();
            h.step();
            h.step();
            if let Some(v) = video.as_mut() {
                let (px, _) = rgba(h);
                v.push(&px).unwrap();
            }
        }
    }
    if let Some(v) = video {
        let frames = v.frames;
        let path = v.finish().unwrap();
        eprintln!(
            "{path}: {frames} frames ({:.1} s), made in {:.0} s",
            frames as f64 / FPS as f64,
            t0.elapsed().as_secs_f64()
        );
    }
}

/// The flange, its whole program at speed: skim, top, prof, bore, bolts, the material coming off
/// close up with the camera on the cut, then the end compared against the part (green: in
/// tolerance).
#[test]
fn the_flange_clip() {
    let dir = copied("flange", &["flange.fg", "flange_cam.fg"]);
    let Some(mut h) = open(&dir.join("flange_cam.fg")) else {
        return;
    };
    let total = h.gui.shell.camsim.as_ref().unwrap().sim.tl.total;
    h.gui
        .camsim_command("t=0;tab=program;colour=material;part=hidden;tool=1;played=1;follow=0;fit=stock")
        .unwrap();
    // the program in 22 s: each op's share of the clip is its share of the program's time
    let play = move |u: f64| format!("t={}", u * total);
    let end = |_u: f64| "at=1;colour=compare;part=hidden;paths=0".to_string();
    clip(
        &mut h,
        "camsim-flange",
        &[(1.0, &|_| "t=0".into()), (22.0, &play), (4.0, &end)],
    );
}

/// A piece of the mould core's ball-end finishing (steep and shallow), close on the cut.
#[test]
fn the_mould_core_finishing_clip() {
    let dir = copied("mould", &["mold_core.fg", "mold_core_finish_cam.fg"]);
    let Some(mut h) = open(&dir.join("mold_core_finish_cam.fg")) else {
        return;
    };
    let st = h.gui.shell.camsim.as_ref().unwrap();
    let ops = &st.sim.tl.ops;
    let k = ops
        .iter()
        .position(|o| o.name == "ss")
        .unwrap_or(1.min(ops.len() - 1));
    let (t0, t1) = (ops[k].t0, ops[k].t1);
    h.gui
        .camsim_command(&format!(
            "t={t0};tab=program;colour=material;part=hidden;tool=1;played=1;follow=0;fit=stock"
        ))
        .unwrap();
    // lower than the default (25°): the steep walls being finished face the camera, and the
    // holder stands above them, not in front
    h.gui.camsim_command("paths=0;fit=stock").unwrap();
    if let Some(v) = h.gui.view.as_mut() {
        v.nav.camera.pitch = 15f32.to_radians();
        v.nav.camera.dist *= 1.05;
        v.mark_dirty();
    }
    let play = move |u: f64| format!("t={}", t0 + u * (t1 - t0));
    clip(&mut h, "camsim-mould-finish", &[(14.0, &play)]);
}
