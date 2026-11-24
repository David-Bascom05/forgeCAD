//! A clip of the soft jaws (David's report): the bracket's first side, then the jaws cut with no
//! part in them (it is not in the machine yet), the part seated in the finished jaws, and the
//! second side machined in them. The whole window, frame by frame, through the app's encoder.
//! `FORGE_CLIPS=DIR cargo test --release --test camsim_clips_jaws`

use forge_render::gui::{Gui, Headless};
use std::path::Path;

#[test]
fn the_bracket_in_soft_jaws_clip() {
    let src = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/machined"));
    let dir = std::env::temp_dir().join(format!("fg-clip-jaws-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for f in ["bracket.fg", "bracket_cam.fg"] {
        std::fs::copy(src.join(f), dir.join(f)).unwrap();
    }
    let mut h = match Headless::new((1400.0, 860.0), 1.0) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("skipped: {e}");
            return;
        }
    };
    h.step();
    h.gui.open(dir.join("bracket_cam.fg").to_str().unwrap());
    let ready = |g: &Gui| {
        g.shell
            .cam
            .as_ref()
            .is_some_and(|w| w.run.is_some() && w.running.is_none() && !w.want_run)
            && g.shell.camsim.is_some()
    };
    let t = std::time::Instant::now();
    while t.elapsed().as_secs_f64() < 180.0 && !h.until(ready) {}
    for _ in 0..20 {
        h.step();
    }
    let total = h.gui.shell.camsim.as_ref().unwrap().sim.tl.total;
    h.gui
        .camsim_command("t=0;tab=program;colour=material;part=ghost;played=1;follow=0;fit=stock")
        .unwrap();
    let out = std::env::var("FORGE_CLIPS").ok();
    let rgba = |h: &Headless| {
        let png = h.png().unwrap();
        let mut r = png::Decoder::new(std::io::Cursor::new(png)).read_info().unwrap();
        let mut buf = vec![0; r.output_buffer_size().unwrap()];
        let info = r.next_frame(&mut buf).unwrap();
        buf.truncate(info.buffer_size());
        let px = match info.color_type {
            png::ColorType::Rgba => buf,
            png::ColorType::Rgb => buf.chunks(3).flat_map(|c| [c[0], c[1], c[2], 255]).collect(),
            c => panic!("a window PNG of {c:?}"),
        };
        (px, (info.width, info.height))
    };
    let mut video = out.as_ref().map(|d| {
        std::fs::create_dir_all(d).unwrap();
        let ff = forge_render::record::ffmpeg::locate().expect("ffmpeg, for FORGE_CLIPS");
        let (_, size) = rgba(&h);
        let p = Path::new(d).join("camsim-bracket-soft-jaws.mp4");
        forge_render::record::ffmpeg::Video::start(&ff, p.to_str().unwrap(), size, 30).unwrap()
    });
    // the whole program in 24 s, each setup refitted as it starts (each has its own frame)
    let n = if video.is_some() { 720 } else { 6 };
    let mut setup = usize::MAX;
    for k in 0..n {
        let tt = total * k as f64 / (n - 1) as f64;
        h.gui.camsim_command(&format!("t={tt}")).unwrap();
        let now = h.gui.shell.camsim.as_ref().unwrap().setup_now();
        if now != setup {
            h.gui.camsim_command("fit=stock").unwrap();
            setup = now;
        }
        h.step();
        h.step();
        if let Some(v) = video.as_mut() {
            v.push(&rgba(&h).0).unwrap();
        }
    }
    if let Some(v) = video {
        eprintln!("{}", v.finish().unwrap());
    }
}
