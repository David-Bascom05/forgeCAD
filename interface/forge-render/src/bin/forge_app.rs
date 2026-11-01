//! The ForgeCAD app. With `--shot`, `--bench`: no window, input by script.

use forge_render::gui::Headless;

const USAGE: &str = "forge-app [PART | file.fc] [--size WxH] [--times]
  [--shot FILE.ppm [--scale S] [--light] [--tab N] [--select NAME] [--click NAME].. [--at X Y] [--sketch ROW;ROW..] [--edit NAME] [--draw TOOL X Y] [--view YAW PITCH]]
  [--present fifo|mailbox|immediate]
  [--bench]
PART: a bench part or a .fc file. --times: input-to-present on exit. --shot: one frame, no window.
--bench: what clicks and frames cost, no window.";

struct Args {
    file: Option<String>,
    size: (f32, f32),
    scale: f32,
    light: bool,
    shot: Option<String>,
    tab: usize,
    select: Option<String>,
    clicks: Vec<String>,
    at: Option<(f32, f32)>,
    sketch: Option<String>,
    view: Option<(f32, f32)>,
    edit: Option<String>,
    draw: Option<(String, f64, f64)>,
    bench: bool,
    times: bool,
    present: Option<String>,
}

fn args() -> Result<Args, String> {
    let mut a = Args {
        file: None,
        size: (1600.0, 900.0),
        scale: 1.0,
        light: false,
        shot: None,
        tab: 0,
        select: None,
        clicks: Vec::new(),
        at: None,
        sketch: None,
        view: None,
        edit: None,
        draw: None,
        bench: false,
        times: false,
        present: None,
    };
    let mut it = std::env::args().skip(1);
    while let Some(k) = it.next() {
        let mut v = || it.next().ok_or(format!("{k} takes a value\n{USAGE}"));
        match k.as_str() {
            "--size" => {
                let s = v()?;
                let (w, h) = s.split_once('x').ok_or(USAGE)?;
                a.size = (w.parse().map_err(|_| USAGE)?, h.parse().map_err(|_| USAGE)?);
            }
            "--scale" => a.scale = v()?.parse().map_err(|_| USAGE)?,
            "--light" => a.light = true,
            "--shot" => a.shot = Some(v()?),
            "--tab" => a.tab = v()?.parse().map_err(|_| USAGE)?,
            "--select" => a.select = Some(v()?),
            "--click" => a.clicks.push(v()?),
            "--at" => {
                let x = v()?.parse().map_err(|_| USAGE)?;
                let mut v = || it.next().ok_or(format!("{k} takes X Y\n{USAGE}"));
                a.at = Some((x, v()?.parse().map_err(|_| USAGE)?));
            }
            "--sketch" => a.sketch = Some(v()?),
            "--edit" => a.edit = Some(v()?),
            "--draw" => {
                let tool = v()?;
                let x = it.next().and_then(|w| w.parse().ok()).ok_or(USAGE)?;
                let y = it.next().and_then(|w| w.parse().ok()).ok_or(USAGE)?;
                a.draw = Some((tool, x, y));
            }
            "--view" => {
                let y: f32 = v()?.parse().map_err(|_| USAGE)?;
                let mut v = || it.next().ok_or(format!("{k} takes YAW PITCH\n{USAGE}"));
                let p: f32 = v()?.parse().map_err(|_| USAGE)?;
                a.view = Some((y.to_radians(), p.to_radians()));
            }
            "--bench" => a.bench = true,
            "--times" => a.times = true,
            "--present" => a.present = Some(v()?),
            s if s.starts_with("--") => return Err(USAGE.into()),
            part => a.file = Some(part.into()),
        }
    }
    Ok(a)
}

fn pct(mut v: Vec<f64>, p: f64) -> f64 {
    v.sort_by(f64::total_cmp);
    v[((v.len() - 1) as f64 * p) as usize]
}

fn bench(h: &mut Headless) {
    // opening another part: no frame waits for it, the one that takes it up is the worst
    h.gui.open("bracket");
    let (mut worst, start) = (0.0f64, std::time::Instant::now());
    while h.gui.is_loading() && start.elapsed().as_secs() < 30 {
        let t = std::time::Instant::now();
        h.step();
        worst = worst.max(t.elapsed().as_secs_f64() * 1e3);
    }
    let t = std::time::Instant::now();
    h.step();
    worst = worst.max(t.elapsed().as_secs_f64() * 1e3);
    println!("worst frame while a part opens: {worst:.2} ms");

    let now = forge_render::viewer::now_ms;
    let names: Vec<String> = h
        .gui
        .shell
        .rows
        .iter()
        .map(|r| format!("tree:{}", r.name))
        .collect();
    let report = |what: &str, v: Vec<f64>| {
        println!(
            "{what}: p50 {:.2} p95 {:.2} max {:.2} ms",
            pct(v.clone(), 0.5),
            pct(v.clone(), 0.95),
            pct(v, 1.0)
        );
    };
    let mut clicks = Vec::new();
    for k in 0..200 {
        let name = &names[k % names.len().max(1)];
        h.click(name);
        let t = now();
        h.step();
        clicks.push(now() - t);
    }
    report("click tree row to GPU done", clicks);
    let mut tabs = Vec::new();
    for k in 0..200 {
        h.click(&format!("tab:{}", forge_render::shell::TABS[k % 5]));
        let t = now();
        h.step();
        tabs.push(now() - t);
    }
    report("click ribbon tab to GPU done", tabs);
    let mut orbit = Vec::new();
    for k in 0..200 {
        if let Some(v) = &mut h.gui.view {
            v.nav.camera.yaw += 0.01;
            v.mark_dirty();
        }
        let t = now();
        let o = h.step();
        assert!(o.times.view_drawn, "frame {k}");
        orbit.push(now() - t);
    }
    report("orbit frame, chrome and viewport, GPU done", orbit);
    // a number dragged: the cursor moves, then the frame with the new shape
    let draggable = h
        .gui
        .view
        .as_ref()
        .and_then(|v| v.facts.handles.iter().find(|d| d.draggable).map(|d| d.item));
    let row = h
        .gui
        .shell
        .rows
        .iter()
        .find(|r| Some(r.id) == draggable)
        .map(|r| format!("tree:{}", r.name));
    if let Some(name) = row {
        h.click(&name);
        h.until(|g| g.shell.dim.is_some());
        h.step();
        h.step();
        if let Some(c) = h.gui.shell.place("dim:arrow").map(|r| r.center()) {
            h.move_to(c.x, c.y);
            h.step();
            h.button(true);
            h.step();
            let mut drag = Vec::new();
            for k in 0..120 {
                let d = 40.0 * (k as f32 * 0.052).sin();
                let was = h.gui.shell.dim.as_ref().map(|e| e.value);
                let t = now();
                h.move_to(c.x + d, c.y + d * 0.6);
                let mut drawn = h.step().times.view_drawn;
                if h.gui.shell.dim.as_ref().map(|e| e.value) == was {
                    continue;
                }
                // a value that can't build draws no new shape
                let failed = |h: &Headless| h.gui.shell.dim.as_ref().is_some_and(|e| e.error.is_some());
                while !drawn && !failed(h) && now() - t < 2000.0 {
                    std::thread::sleep(std::time::Duration::from_micros(200));
                    drawn = h.step().times.view_drawn;
                }
                if drawn {
                    drag.push(now() - t);
                }
            }
            report("drag a number: cursor to the frame with its shape", drag);
            h.key(egui::Key::Escape);
            h.step();
            h.button(false);
            h.until(|g| g.view.as_ref().is_some_and(|v| !v.live_drag));
        }
    }
    let o = h.step();
    let o2 = h.step();
    println!(
        "idle: asks to draw again {:?} then {:?}; viewport drawn {} ",
        o.again_in, o2.again_in, o2.times.view_drawn
    );
}

fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let a = args()?;
    if a.shot.is_none() && !a.bench {
        return forge_render::app::run(a.file, (a.size.0 as f64, a.size.1 as f64), a.times, a.present);
    }
    let mut h = Headless::new(a.size, a.scale)?;
    if a.light {
        h.gui.shell.theme = forge_render::shell::theme::LIGHT;
    }
    h.gui.shell.tab = a.tab;
    h.step();
    h.gui.open(a.file.as_deref().unwrap_or("bracket"));
    h.settle();
    for _ in 0..5 {
        h.step();
    }
    for c in &a.clicks {
        if !h.click(c) {
            return Err(format!("nothing called {c}"));
        }
        h.step();
        h.step();
    }
    if let Some(s) = &a.select
        && !h.click(&format!("tree:{s}"))
    {
        return Err(format!("no tree row {s}"));
    }
    h.step();
    h.step();
    for _ in 0..3 {
        h.step();
    }
    if let (Some((yaw, pitch)), Some(v)) = (a.view, &mut h.gui.view) {
        v.nav.camera.yaw = yaw;
        v.nav.camera.pitch = pitch;
        v.mark_dirty();
        h.step();
        h.step();
    }
    if let Some(name) = &a.edit {
        // a feature's sketch, in sketch mode
        let id = h.gui.shell.rows.iter().find(|r| r.name == *name).map(|r| r.id);
        let Some(id) = id else {
            return Err(format!("no feature {name}"));
        };
        h.gui.shell.out.push(forge_render::shell::Action::EditInline(id));
        h.until(|g| g.shell.sketch.is_some());
        for _ in 0..40 {
            h.step();
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
    if let Some(rows) = &a.sketch {
        // on the XY plane, with these rows
        h.gui.shell.origin_selected = Some(forge_render::origin::Item::XyPlane);
        h.gui.shell.out.push(forge_render::shell::Action::StartSketch);
        h.until(|g| g.shell.sketch.is_some());
        if let Some(m) = &mut h.gui.shell.sketch {
            for r in rows.split(';').filter(|r| !r.trim().is_empty()) {
                m.add(r.trim().to_string());
            }
        }
        for _ in 0..40 {
            h.step();
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
    if let (Some((tool, x, y)), Some(m)) = (&a.draw, &mut h.gui.shell.sketch) {
        use forge_render::shell::sketching::SkTool as T;
        m.set_tool(Some(match tool.as_str() {
            "rect" => T::Rectangle,
            "circle" => T::Circle,
            "arc" => T::Arc,
            _ => T::Line,
        }));
        m.click((*x, *y), 0.0);
    }
    if let Some((x, y)) = a.at {
        h.move_to(x, y);
        for _ in 0..3 {
            h.step();
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
    if a.bench {
        bench(&mut h);
    }
    if let Some(s) = &a.shot {
        std::fs::write(s, h.ppm()).map_err(|e| e.to_string())?;
    }
    Ok(())
}
