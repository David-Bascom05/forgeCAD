//! Photo and Record (shell/capture.rs, gui_record.rs), headless: a photo at the size asked,
//! opaque, the same view twice the same bytes; a turntable recording's PNG frames the same twice;
//! an MP4 by ffmpeg (where there is one) with every frame in it.

use forge_render::gui::{Gui, Headless};
use forge_render::shell::capture::{Capture, Format, Size};

/// The names in a folder ending so, sorted.
fn listed(dir: &std::path::Path, end: &str) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(end))
        .collect();
    v.sort();
    v
}

fn settled(g: &Gui) -> bool {
    g.view.is_some() && g.fg.as_ref().is_some_and(|s| s.done == s.seq) && !g.loading()
}

fn open() -> Option<Headless> {
    let mut h = match Headless::new((1000.0, 700.0), 1.0) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("skipped: {e}");
            return None;
        }
    };
    h.step();
    h.gui.open(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/machined/flange.fg"
    ));
    assert!(h.until(settled), "never built: {}", h.gui.shell.status);
    Some(h)
}

fn until_done(h: &mut Headless) {
    let start = std::time::Instant::now();
    h.step();
    while h.gui.capture_busy() {
        h.step();
        assert!(start.elapsed().as_secs() < 300, "capture never ended");
    }
}

fn scratch(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("fg-capture-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn a_photo_is_the_view_at_the_size_asked() {
    let Some(mut h) = open() else { return };
    let dir = scratch("photo");
    h.gui.shell.capture.folder = dir.to_string_lossy().into_owned();
    h.gui.shell.capture.photo_size = Size::Custom(640, 400);
    for _ in 0..2 {
        h.gui.shell.capture.asks.push(Capture::Photo);
        until_done(&mut h);
    }
    let files = listed(&dir, ".png");
    assert_eq!(files.len(), 2, "{files:?}");
    assert!(
        files[0].starts_with("flange-view-2"),
        "named document-environment-time: {files:?}"
    );
    let a = std::fs::read(dir.join(&files[0])).unwrap();
    let b = std::fs::read(dir.join(&files[1])).unwrap();
    assert_eq!(a, b, "the same view, the same photo");
    let mut r = png::Decoder::new(std::io::Cursor::new(&a)).read_info().unwrap();
    assert_eq!((r.info().width, r.info().height), (640, 400));
    let mut px = vec![0; r.output_buffer_size().unwrap()];
    r.next_frame(&mut px).unwrap();
    let first = &px[..4];
    assert!(px.chunks(4).any(|p| p != first), "the part is in it");
    assert!(px.chunks(4).all(|p| p[3] == 255));
    assert!(
        h.gui.shell.capture.last.starts_with("Photo saved: flange-view-"),
        "{}",
        h.gui.shell.capture.last
    );
    if let Ok(out) = std::env::var("PHOTO_SHOTS") {
        std::fs::write(format!("{out}/photo.png"), &a).unwrap();
    }
    assert!(h.gui.photo_start((0, 10), "x.png").is_err());
}

fn record(h: &mut Headless, dir: &std::path::Path, format: Format) {
    let c = &mut h.gui.shell.capture;
    c.folder = dir.to_string_lossy().into_owned();
    c.size = Size::Custom(160, 96);
    c.fps = 10;
    c.orbit_seconds = 1.0;
    c.format = format;
    c.asks.push(Capture::Record);
    until_done(h);
}

#[test]
fn a_turntable_records_the_same_frames_twice() {
    let Some(mut h) = open() else { return };
    let dir = scratch("frames");
    record(&mut h, &dir, Format::Frames);
    record(&mut h, &dir, Format::Frames);
    let dirs = listed(&dir, "");
    assert_eq!(dirs.len(), 2, "{dirs:?}");
    let read = |k: usize| {
        let d = dir.join(&dirs[k]);
        (0..10)
            .map(|f| std::fs::read(d.join(format!("frame_{f:05}.png"))).unwrap())
            .collect::<Vec<_>>()
    };
    let (a, b) = (read(0), read(1));
    assert_eq!(a, b, "a recording is deterministic");
    assert_ne!(a[0], a[3], "the camera turned");
    assert!(
        !dir.join("flange-turntable-frames-1")
            .join("frame_00010.png")
            .exists()
    );
    assert!(
        h.gui.shell.capture.last.starts_with("Recorded 10 frames"),
        "{}",
        h.gui.shell.capture.last
    );
}

#[test]
fn a_turntable_records_an_mp4_with_ffmpeg() {
    let Some(ff) = forge_render::record::ffmpeg::locate() else {
        eprintln!("skipped: no ffmpeg");
        return;
    };
    let Some(mut h) = open() else { return };
    let dir = scratch("mp4");
    record(&mut h, &dir, Format::Mp4);
    let mp4s = listed(&dir, ".mp4");
    assert_eq!(mp4s.len(), 1, "{}", h.gui.shell.capture.last);
    let mp4 = dir.join(&mp4s[0]);
    assert!(
        h.gui.shell.capture.last.starts_with("Video saved: flange-view-"),
        "{}",
        h.gui.shell.capture.last
    );
    let probe = ff.with_file_name("ffprobe");
    if probe.is_file() {
        let o = std::process::Command::new(probe)
            .args(["-v", "error", "-count_frames", "-select_streams", "v:0"])
            .args([
                "-show_entries",
                "stream=codec_name,nb_read_frames,width,height",
                "-of",
                "csv=p=0",
            ])
            .arg(&mp4)
            .output()
            .unwrap();
        let line = String::from_utf8_lossy(&o.stdout).trim().to_string();
        assert!(
            line.starts_with("h264,160,96,10") || line.starts_with("mpeg4,160,96,10"),
            "{line}"
        );
        if let Ok(out) = std::env::var("PHOTO_SHOTS") {
            std::fs::copy(&mp4, format!("{out}/turntable.mp4")).unwrap();
        }
    }
}
