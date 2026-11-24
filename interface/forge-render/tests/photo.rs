//! A photo of the view (gui_record.rs), headless: the size asked, an opaque PNG, and the same
//! view twice gives the same bytes.

use forge_render::gui::{Gui, Headless};

fn settled(g: &Gui) -> bool {
    g.view.is_some() && g.fg.as_ref().is_some_and(|s| s.done == s.seq) && !g.loading()
}

fn photo(h: &mut Headless, path: &std::path::Path, size: (u32, u32)) -> Vec<u8> {
    h.gui.photo_start(size, &path.to_string_lossy()).unwrap();
    let start = std::time::Instant::now();
    loop {
        h.step();
        if let Some(done) = h.gui.photo_poll() {
            assert_eq!(done.unwrap(), path.to_string_lossy());
            break;
        }
        assert!(start.elapsed().as_secs() < 60, "no photo");
    }
    std::fs::read(path).unwrap()
}

#[test]
fn a_photo_is_the_view_at_the_size_asked() {
    let mut h = match Headless::new((1000.0, 700.0), 1.0) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("skipped: {e}");
            return;
        }
    };
    h.step();
    h.gui.open(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/machined/flange.fg"
    ));
    assert!(h.until(settled), "never built: {}", h.gui.shell.status);
    let dir = std::env::temp_dir().join(format!("fg-photo-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let a = photo(&mut h, &dir.join("a.png"), (640, 400));
    let b = photo(&mut h, &dir.join("b.png"), (640, 400));
    assert_eq!(a, b, "the same view, the same photo");
    let d = png::Decoder::new(std::io::Cursor::new(&a));
    let r = d.read_info().unwrap();
    let info = r.info();
    assert_eq!((info.width, info.height), (640, 400));
    // the part is in it: not one colour
    let mut r = png::Decoder::new(std::io::Cursor::new(&a)).read_info().unwrap();
    let mut px = vec![0; r.output_buffer_size().unwrap()];
    r.next_frame(&mut px).unwrap();
    let first = &px[..4];
    assert!(px.chunks(4).any(|p| p != first));
    assert!(px.chunks(4).all(|p| p[3] == 255));
    if let Ok(out) = std::env::var("PHOTO_SHOTS") {
        std::fs::write(format!("{out}/photo.png"), &a).unwrap();
    }
    assert!(h.gui.photo_start((0, 10), "x.png").is_err());
    let _ = std::fs::remove_dir_all(&dir);
}
