//! The Render Studio environment in the window, headless: entered on a part, the view shows the
//! tracer's picture and its samples climb; a Render Image writes its file; Finish gives the view
//! back. `STUDIO_SHOTS=DIR` writes the window (PNG) for research/render/studio/.

use forge_render::gui::{Gui, Headless};
use forge_render::shell::env::EnvId;
use forge_render::shell::studio_env::Ask;

fn settled(g: &Gui) -> bool {
    g.view.is_some() && g.fg.as_ref().is_some_and(|s| s.done == s.seq) && !g.loading()
}

#[test]
fn the_studio_traces_the_view_and_renders_a_file() {
    let mut h = match Headless::new((1400.0, 860.0), 1.0) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("skipped: {e}");
            return;
        }
    };
    h.step();
    let part = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/machined/flange.fg");
    h.gui.open(part);
    assert!(h.until(settled), "never built: {}", h.gui.shell.status);
    let shaded = h.gui.shell.view_tex;
    h.gui.shell.enter_env(EnvId::RenderStudio).unwrap();
    for _ in 0..30 {
        h.step();
    }
    let st = &h.gui.shell.studio;
    assert!(
        st.samples >= 10,
        "samples {}, env {:?}, status {}, names {:?}",
        st.samples,
        h.gui.shell.env,
        h.gui.shell.status,
        st.names
    );
    assert_eq!(st.names.len(), 1);
    assert_ne!(
        h.gui.shell.view_tex, shaded,
        "the view shows the tracer's picture"
    );
    if let Ok(dir) = std::env::var("STUDIO_SHOTS") {
        for _ in 0..60 {
            h.step();
        }
        std::fs::write(format!("{dir}/studio-env.png"), h.png().unwrap()).unwrap();
    }
    // a final render, small, to a file
    let out = std::env::temp_dir().join(format!("fg-studio-{}.png", std::process::id()));
    let st = &mut h.gui.shell.studio;
    st.out = out.to_string_lossy().into_owned();
    st.size = (64, 40);
    st.spp = 16;
    st.looks[0] = Some("Brass".into());
    st.version += 1;
    st.asks.push(Ask::Image);
    let start = std::time::Instant::now();
    while !out.exists() && start.elapsed().as_secs() < 120 {
        h.step();
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(out.exists(), "no file; queue says {}", h.gui.shell.studio.queue);
    let bytes = std::fs::read(&out).unwrap();
    assert!(bytes.starts_with(b"\x89PNG"));
    let _ = std::fs::remove_file(&out);
    // Finish: the shaded view again
    h.gui.shell.exit_env();
    h.step();
    h.step();
    assert_eq!(h.gui.shell.view_tex, shaded);
}

/// The environment as David sees it, for research/render/studio/: an assembly in mixed finishes
/// under the bundled HDRI, at the window's usual size and at 1024x700. Only with STUDIO_SHOTS.
#[test]
fn shots_of_the_environment() {
    let Ok(dir) = std::env::var("STUDIO_SHOTS") else {
        return;
    };
    for (w, h) in [(1400.0, 860.0), (1024.0, 700.0)] {
        let mut h_ = match Headless::new((w, h), 1.0) {
            Ok(h) => h,
            Err(e) => {
                eprintln!("skipped: {e}");
                return;
            }
        };
        h_.step();
        let asm = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/sheetmetal/tray_asm.fg"
        );
        h_.gui.open(asm);
        assert!(h_.until(settled), "never built: {}", h_.gui.shell.status);
        h_.gui.shell.enter_env(EnvId::RenderStudio).unwrap();
        h_.step();
        h_.step();
        let looks = [
            "Aluminium, bead-blasted",
            "Steel, black oxide",
            "Powder coat",
            "Stainless steel, brushed",
        ];
        let st = &mut h_.gui.shell.studio;
        for (k, l) in st.looks.iter_mut().enumerate() {
            *l = Some(looks[k % looks.len()].into());
        }
        st.version += 1;
        let start = std::time::Instant::now();
        while h_.gui.shell.studio.samples < 256 && start.elapsed().as_secs() < 300 {
            h_.step();
        }
        let png = h_.png().unwrap();
        std::fs::write(format!("{dir}/studio-env-{}x{}.png", w as u32, h as u32), png).unwrap();
    }
}
