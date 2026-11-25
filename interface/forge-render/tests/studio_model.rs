//! The Render Studio's Model tab (shell/studio_model.rs, research/render/studio-model/), headless:
//! an assembly's parts listed once each with their own materials' looks, the brush applied by a
//! click in the view, many parts given one look at once, the filter. `STUDIO_MODEL_SHOTS=DIR`
//! keeps pictures of the tab.

use forge_render::gui::{Gui, Headless};
use forge_render::render::Entity;
use forge_render::shell::studio_model::{Row, apply, rows, select};

/// The tests here one at a time: they share one (often software) GPU, and two at once time each
/// other's frames.
fn one_at_a_time() -> std::sync::MutexGuard<'static, ()> {
    static GPU: std::sync::Mutex<()> = std::sync::Mutex::new(());
    GPU.lock().unwrap_or_else(|e| e.into_inner())
}

fn settled(g: &Gui) -> bool {
    g.view.is_some() && g.fg.as_ref().is_some_and(|s| s.done == s.seq) && !g.loading()
}

fn studio(file: &str) -> Option<Headless> {
    let mut h = match Headless::new((1400.0, 860.0), 1.0) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("skipped: {e}");
            return None;
        }
    };
    h.step();
    h.gui
        .open(&format!("{}/../../examples/{file}", env!("CARGO_MANIFEST_DIR")));
    assert!(h.until(settled), "never built: {}", h.gui.shell.status);
    h.gui.env_asked = Some("render".into());
    for _ in 0..30 {
        h.step();
        if !h.gui.shell.studio.names.is_empty() {
            break;
        }
    }
    assert!(!h.gui.shell.studio.names.is_empty(), "the Studio found no parts");
    for _ in 0..5 {
        h.step();
    }
    assert_eq!(
        h.gui.shell.env.map(|e| e.key()),
        Some("render"),
        "{}",
        h.gui.shell.status
    );
    Some(h)
}

fn shot(h: &Headless, name: &str) {
    if let Ok(dir) = std::env::var("STUDIO_MODEL_SHOTS") {
        std::fs::write(format!("{dir}/{name}.png"), h.png().unwrap()).unwrap();
    }
}

#[test]
fn a_click_in_the_view_paints_the_part_with_the_brush() {
    let _gpu = one_at_a_time();
    let Some(mut h) = studio("robot/arm2.fg") else {
        return;
    };
    let st = &h.gui.shell.studio;
    assert_eq!(
        st.names,
        ["base", "finger", "link"],
        "each part once, by its file"
    );
    assert!(st.looks.iter().all(Option::is_none));
    // the brush, painting: a click in the view selects; the Studio takes it as the brush's
    h.gui.shell.studio.brush = Some("Brass".into());
    h.gui.shell.studio.painting = true;
    let v = h.gui.view.as_mut().unwrap();
    let part = v.scene.placements[0].part as usize;
    let version = h.gui.shell.studio.version;
    v.nav.selected.push((0, Entity::Face(0)));
    h.step();
    let st = &h.gui.shell.studio;
    assert_eq!(st.looks[part].as_deref(), Some("Brass"));
    assert!(st.looks.iter().filter(|l| l.is_some()).count() == 1);
    assert!(st.version > version, "the picture is made again");
    assert!(
        h.gui.view.as_ref().unwrap().nav.selected.is_empty(),
        "the click is spent"
    );
    assert!(st.said.ends_with(": Brass"), "{}", st.said);
    h.step();
    // the trace starts again: until it's readable the raster view, in the same looks, stands in
    assert!(h.gui.shell.view_over_tex.is_none(), "the noise is shown");
    assert!(h.gui.shell.studio.samples < forge_render::shell::studio_env::READABLE);
    shot(&h, "painted");
    // (the live trace goes a band of rows a frame, as the GPU allows: waited for by time)
    let start = std::time::Instant::now();
    while h.gui.shell.studio.samples < 2 * forge_render::shell::studio_env::READABLE
        && start.elapsed().as_secs() < 120
    {
        h.step();
    }
    h.step();
    assert!(
        h.gui.shell.view_over_tex.is_some() && h.gui.shell.view_over_alpha == 1.0,
        "the trace again"
    );
    shot(&h, "painted-settled");
    // not painting: a click paints nothing; it chooses that part's row (lit in the view)
    h.gui.shell.studio.painting = false;
    let other = h.gui.view.as_ref().unwrap().scene.placements[1].part as usize;
    h.gui
        .view
        .as_mut()
        .unwrap()
        .nav
        .selected
        .push((1, Entity::Face(0)));
    h.step();
    assert_eq!(h.gui.shell.studio.selected, vec![other]);
    assert_eq!(h.gui.shell.studio.looks.iter().filter(|l| l.is_some()).count(), 1);
    // rows chosen in the list stay chosen: the view's pick is taken once
    h.gui.shell.studio.selected = vec![0, 1, 2];
    h.step();
    h.step();
    assert_eq!(h.gui.shell.studio.selected, vec![0, 1, 2]);
    shot(&h, "selected");
}

#[test]
fn many_parts_take_one_look_and_the_filter_finds_them() {
    let _gpu = one_at_a_time();
    let Some(mut h) = studio("robot/arm2.fg") else {
        return;
    };
    let st = &mut h.gui.shell.studio;
    let shown: Vec<usize> = rows(st)
        .iter()
        .filter_map(|r| match r {
            Row::Part(k) => Some(*k),
            _ => None,
        })
        .collect();
    assert_eq!(shown.len(), 3);
    select(st, shown[0], false, false, &shown);
    select(st, shown[2], false, true, &shown);
    let to = st.selected.clone();
    assert_eq!(to.len(), 3);
    apply(st, &to, Some("Paint, machine gloss"));
    assert!(
        st.looks
            .iter()
            .all(|l| l.as_deref() == Some("Paint, machine gloss"))
    );
    st.filter = "fing".into();
    assert_eq!(rows(st), vec![Row::Part(1)]);
    st.filter.clear();
    // the picture is made again with them, however they were changed: the samples start over
    let start = std::time::Instant::now();
    while h.gui.shell.studio.samples < 4 && start.elapsed().as_secs() < 120 {
        h.step();
    }
    let before = h.gui.shell.studio.samples;
    assert!(before >= 4, "{before}");
    apply(&mut h.gui.shell.studio, &[0], Some("Brass"));
    h.step();
    let after = h.gui.shell.studio.samples;
    assert!(
        after <= 2,
        "the picture didn't start again: {after} samples after {before}"
    );
    shot(&h, "many");
}

/// Each part's own material gives its default look, not the assembly's: the rack's parts are
/// steel, nylon, rubber, ... (each `props(material:)`), and the Studio reads each.
#[test]
fn an_assemblys_parts_keep_their_own_materials() {
    let _gpu = one_at_a_time();
    let Some(h) = studio("rack18u/asm/rack.fg") else {
        return;
    };
    let st = &h.gui.shell.studio;
    assert!(st.names.len() > 50, "{}", st.names.len());
    let mut own: Vec<&str> = st.own.iter().flatten().map(String::as_str).collect();
    own.sort();
    own.dedup();
    assert!(own.len() >= 2, "one material for all: {own:?}");
    let groups: std::collections::BTreeSet<&str> = st.groups.iter().map(String::as_str).collect();
    assert!(groups.contains("frame") && groups.contains("hw"), "{groups:?}");
    assert!(st.counts.iter().sum::<usize>() > st.names.len());
    shot(&h, "rack");
}

/// Render Image answers at once: the next frame shows its progress (the card, the button saying
/// "Rendering…"), within 100 ms; it ends in a result, the picture with its file; a failure says
/// why. (David: "clicking Render image gives no visible feedback".)
#[test]
fn render_image_shows_its_progress_at_once_then_its_result() {
    let _gpu = one_at_a_time();
    let Some(mut h) = studio("machined/flange.fg") else {
        return;
    };
    let dir = std::env::temp_dir().join(format!("fg-studio-render-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    h.gui.shell.capture.folder = dir.to_string_lossy().into_owned();
    let st = &mut h.gui.shell.studio;
    st.size = (64, 48);
    st.spp = 64;
    // no file named: a fresh one where captures go (BUTTON-AUDITOR: not the process's folder)
    assert!(st.out.is_empty());
    // a plain Studio frame, the same view with nothing traced (the live view stopped, its last
    // denoise done), for the click frame to be measured against
    let l = h.gui.studio_live().unwrap();
    l.target = l.traced();
    for _ in 0..20 {
        h.step();
    }
    let plain = (0..5)
        .map(|_| {
            let t = std::time::Instant::now();
            h.step();
            t.elapsed()
        })
        .min()
        .unwrap();
    // `click` hovers and presses (two frames); the click lands on the release, the frame timed
    assert!(h.click("tool:Render Image"), "no Render Image on the ribbon");
    let t0 = std::time::Instant::now();
    h.step();
    let first = t0.elapsed();
    // progress in that same frame
    let r = h.gui.shell.studio.rendering.clone().expect("no progress shown");
    let info = h.gui.adapter();
    let software = info.device_type == wgpu::DeviceType::Cpu
        || ["llvmpipe", "lavapipe", "swiftshader", "warp"]
            .iter()
            .any(|s| info.name.to_lowercase().contains(s) || info.driver.to_lowercase().contains(s));
    eprintln!(
        "click frame {first:?}, plain frame {plain:?} ({}, {:?}{})",
        info.name,
        info.device_type,
        if software { ", software" } else { "" }
    );
    if !cfg!(debug_assertions) {
        // no more than a plain frame: the click adds no work of its own
        assert!(
            first <= plain + std::time::Duration::from_millis(20),
            "the click frame {first:?}, a plain frame {plain:?}"
        );
        // and at once where the GPU isn't a software rasteriser (whose every frame is slow)
        assert!(software || first.as_millis() < 100, "{first:?}");
    }
    assert!(r.of == 64 && r.what == "Rendering image", "{r:?}");
    h.step();
    assert!(
        h.gui.shell.place("tool:Rendering…").is_some(),
        "the button says it's rendering"
    );
    assert!(h.gui.shell.place("studio:cancel").is_some(), "Cancel on the card");
    shot(&h, "rendering");
    let start = std::time::Instant::now();
    while h.gui.shell.studio.result.is_none() {
        h.step();
        assert!(start.elapsed().as_secs() < 120, "the render never ended");
    }
    let res = h.gui.shell.studio.result.clone().unwrap();
    let path = res.said.clone().expect("it saved");
    let p = std::path::Path::new(&path);
    assert!(p.is_file(), "{path}");
    assert_eq!(p.parent(), Some(dir.as_path()), "{path}");
    let name = p.file_name().unwrap().to_string_lossy();
    assert!(
        name.starts_with("flange-render-") && name.ends_with(".png"),
        "{name}"
    );
    assert_eq!(res.thumb.map(|t| t.1), Some((64, 48)), "the picture shown");
    assert!(h.gui.shell.studio.rendering.is_none());
    assert_eq!(h.gui.shell.status, format!("Wrote {path}"));
    h.step();
    shot(&h, "rendered");
    // a failure is said, not silent
    h.gui.shell.studio.out = "/nonexistent-dir/x.png".into();
    h.gui.shell.studio.size = (32, 24);
    h.gui.shell.studio.spp = 8;
    h.gui
        .shell
        .studio
        .asks
        .push(forge_render::shell::studio_env::Ask::Image);
    let start = std::time::Instant::now();
    loop {
        h.step();
        if let Some(r) = &h.gui.shell.studio.result
            && r.said.is_err()
        {
            break;
        }
        assert!(start.elapsed().as_secs() < 120, "the failure was never said");
    }
    assert!(
        h.gui.shell.status.starts_with("Render failed: "),
        "{}",
        h.gui.shell.status
    );
}

/// Pictures for review, only with STUDIO_MODEL_SHOTS=DIR: the rack at 1400 and 480 wide, a filter
/// typed, applying with the pointer over a part, Overrides open.
#[test]
fn shots_for_review() {
    let _gpu = one_at_a_time();
    let Ok(dir) = std::env::var("STUDIO_MODEL_SHOTS") else {
        return;
    };
    for (w, h) in [(1400.0, 860.0), (480.0, 860.0)] {
        let mut g = match Headless::new((w, h), 1.0) {
            Ok(g) => g,
            Err(e) => {
                eprintln!("skipped: {e}");
                return;
            }
        };
        g.step();
        g.gui.open(&format!(
            "{}/../../examples/rack18u/asm/rack.fg",
            env!("CARGO_MANIFEST_DIR")
        ));
        assert!(g.until(settled));
        g.gui.env_asked = Some("render".into());
        for _ in 0..40 {
            g.step();
        }
        let snap = |g: &mut Headless, name: &str| {
            for _ in 0..4 {
                g.step();
            }
            std::fs::write(format!("{dir}/review-{}-{name}.png", w as u32), g.png().unwrap()).unwrap();
        };
        // a filter typed: the groups that hold matches open
        g.gui.shell.studio.filter = "iso7380".into();
        snap(&mut g, "filter");
        g.gui.shell.studio.filter.clear();
        // applying: the brush chosen, the pointer over a part in the view
        g.gui.shell.studio.brush = Some("Paint, machine gloss".into());
        g.gui.shell.studio.painting = true;
        let c = g.gui.shell.view_rect.center();
        g.move_to(c.x, c.y);
        snap(&mut g, "applying");
        // some looks given: Overrides open; a group open with a part chosen
        g.gui.shell.studio.painting = false;
        let n = g.gui.shell.studio.names.len();
        let frame: Vec<usize> = (0..n)
            .filter(|&k| g.gui.shell.studio.groups[k] == "frame")
            .collect();
        forge_render::shell::studio_model::apply(
            &mut g.gui.shell.studio,
            &frame,
            Some("Paint, machine gloss"),
        );
        let hw: Vec<usize> = (0..n)
            .filter(|&k| g.gui.shell.studio.groups[k] == "hw")
            .take(3)
            .collect();
        forge_render::shell::studio_model::apply(&mut g.gui.shell.studio, &hw, Some("Steel, black oxide"));
        g.gui.shell.studio.open.insert("frame".into());
        g.gui.shell.studio.selected = frame.clone();
        let start = std::time::Instant::now();
        while g.gui.shell.studio.samples < 16 && start.elapsed().as_secs() < 120 {
            g.step();
        }
        snap(&mut g, "overrides");
    }
}

/// The live trace keeps the window answering: the trace and its denoise go a band of rows a
/// frame, as many as the GPU does in the pace's budget (8 ms), so a Studio frame while it runs
/// costs at most `LIVE_MS`, or a quarter of a frame on a GPU slow enough that frames take more
/// than 120 ms, more than one with it stopped (p90 against the median), a software GPU too.
/// Before the bands, every frame traced a whole sample and denoised it: about 230 ms a frame on
/// lavapipe here, over 16 ms stopped. Release only: a debug build's own frames are no measure.
#[test]
fn the_live_trace_keeps_studio_frames_within_budget() {
    let _gpu = one_at_a_time();
    const LIVE_MS: f64 = 30.0;
    if cfg!(debug_assertions) {
        return;
    }
    let Some(mut h) = studio("machined/flange.fg") else {
        return;
    };
    let timed = |h: &mut Headless, n: usize| {
        let mut ms: Vec<f64> = (0..n)
            .map(|_| {
                let t = std::time::Instant::now();
                h.step();
                t.elapsed().as_secs_f64() * 1e3
            })
            .collect();
        ms.sort_by(f64::total_cmp);
        ms
    };
    // the pace fitted to this GPU, and a picture shown (a denoise run): then frames as they go
    let start = std::time::Instant::now();
    while h.gui.shell.studio.samples < 2 * forge_render::shell::studio_env::READABLE {
        h.step();
        assert!(
            start.elapsed().as_secs() < 180,
            "the live view never showed a picture"
        );
    }
    let traced = h.gui.studio_live().unwrap().traced();
    let live = timed(&mut h, 60);
    let more = h.gui.studio_live().unwrap().traced() - traced;
    assert!(more > 0, "nothing traced while timed");
    // stopped: the same frames, the trace done
    let l = h.gui.studio_live().unwrap();
    l.target = l.traced();
    for _ in 0..30 {
        h.step();
    }
    let idle = timed(&mut h, 30);
    let at = |v: &[f64], q: f64| v[((v.len() - 1) as f64 * q) as usize];
    let rows = h.gui.studio_live().unwrap().pace.rows;
    eprintln!(
        "studio frames: tracing median {:.1} ms, p90 {:.1}, max {:.1}; stopped median {:.1} ms; \
         {more} samples in 60 frames, {rows} rows a frame",
        at(&live, 0.5),
        at(&live, 0.9),
        at(&live, 1.0),
        at(&idle, 0.5)
    );
    let limit = LIVE_MS.max(0.25 * at(&idle, 0.5));
    assert!(
        at(&live, 0.9) - at(&idle, 0.5) <= limit,
        "a traced frame costs {:.1} ms more than a stopped one, over {limit:.0} ms (p90 {:.1} ms, \
         stopped {:.1} ms)",
        at(&live, 0.9) - at(&idle, 0.5),
        at(&live, 0.9),
        at(&idle, 0.5)
    );
}

/// The pictures UX-LEAD reviews (research/render/studio/11-13), only with STUDIO_UX_SHOTS=DIR, at
/// 1400×860 in the dark theme: the live view still refining between its denoise steps (the badge
/// says so), the Output fold's File field empty with its hint, the recording pill.
#[test]
fn shots_for_ux() {
    let Ok(dir) = std::env::var("STUDIO_UX_SHOTS") else {
        return;
    };
    let _gpu = one_at_a_time();
    let Some(mut h) = studio("machined/flange.fg") else {
        return;
    };
    let tmp = std::env::temp_dir().join(format!("fg-studio-ux-{}", std::process::id()));
    std::fs::create_dir_all(&tmp).unwrap();
    h.gui.shell.capture.folder = tmp.to_string_lossy().into_owned();
    let save =
        |h: &Headless, name: &str| std::fs::write(format!("{dir}/{name}.png"), h.png().unwrap()).unwrap();
    // 1. a picture shown (16 samples), more going in
    let start = std::time::Instant::now();
    while !(h.gui.shell.studio.samples >= 16 && h.gui.shell.studio.traced > h.gui.shell.studio.samples + 4) {
        h.step();
        assert!(
            start.elapsed().as_secs() < 180,
            "the live view never refined past 16"
        );
    }
    assert!(forge_render::shell::studio_env::refining(&h.gui.shell.studio));
    let words = forge_render::shell::studio_env::live_words(&h.gui.shell.studio);
    assert!(words.starts_with("Tracing · "), "{words}");
    // the pointer over the badge: its hover says what it does
    let v = h.gui.shell.view_rect;
    h.move_to(v.left() + 40.0, v.bottom() - 24.0);
    // (a tooltip waits for the pointer to rest: 1.5 s of the frames' clock)
    for _ in 0..90 {
        h.step();
    }
    save(&h, "11-live-refining");
    // 2. Output open: the File field empty, its hint
    h.move_to(v.center().x, v.center().y);
    assert!(h.gui.shell.studio.out.is_empty());
    assert!(h.click("studio:fold:output"), "no Output fold");
    // (the fold opens over a few frames)
    for _ in 0..30 {
        h.step();
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    save(&h, "12-render-file-hint");
    // 3. a recording under way: its pill, Cancel and Esc
    h.gui
        .capture_command("record 320x200 fps=10 seconds=8 frames")
        .unwrap();
    // (the pill fades in)
    for _ in 0..10 {
        h.step();
        std::thread::sleep(std::time::Duration::from_millis(30));
    }
    assert!(h.gui.shell.place("capture:stop").is_some(), "no recording pill");
    save(&h, "13-recording-pill");
    assert!(h.click("capture:stop"));
    h.step();
    let _ = std::fs::remove_dir_all(&tmp);
}
