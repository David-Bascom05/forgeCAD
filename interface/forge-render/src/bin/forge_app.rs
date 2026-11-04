//! The ForgeCAD app. With `--shot`, `--bench`: no window, input by script.

use forge_render::gui::Headless;

const USAGE: &str = "forge-app [PART | file.fc] [--size WxH] [--times]
  [--shot FILE.ppm [--scale S] [--light] [--tab N] [--select NAME] [--click NAME].. [--rclick NAME].. [--wait] [--at X Y] [--sketch ROW;ROW..] [--edit NAME] [--draw TOOL X Y] [--view YAW PITCH] [--marking]]
  steps, in order: --click NAME --dclick NAME --rclick NAME --wait --type TEXT --key NAME (enter, escape, tab, backspace)
  --hover X Y (the view's pick there) --clickat X Y --drag X Y X2 Y2 --snap FILE.ppm --save
  --viewnow YAW PITCH --home
  [--present fifo|mailbox|immediate] [--run-for SECONDS] [--spin]
  [--bench]
PART: a bench part or a .fc file. --times: input-to-present on exit. --shot: one frame, no window.
--bench: what clicks and frames cost, no window.";

enum Step {
    Click(String),
    DoubleClick(String),
    RightClick(String),
    Wait,
    Type(String),
    Key(String),
    Hover(f32, f32),
    ClickAt(f32, f32),
    Drag(f32, f32, f32, f32),
    Snap(String),
    Save,
    View(f32, f32),
    Home,
}

struct Args {
    file: Option<String>,
    size: (f32, f32),
    scale: f32,
    light: bool,
    shot: Option<String>,
    tab: usize,
    select: Option<String>,
    /// Clicks by name and waits for the Forge file's builds and checks, in order.
    steps: Vec<Step>,
    at: Option<(f32, f32)>,
    sketch: Option<String>,
    view: Option<(f32, f32)>,
    edit: Option<String>,
    draw: Option<(String, f64, f64)>,
    /// Shows the marking menu at the view's middle, for a shot.
    marking: bool,
    bench: bool,
    times: bool,
    present: Option<String>,
    run_for: Option<f64>,
    spin: bool,
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
        steps: Vec::new(),
        at: None,
        sketch: None,
        view: None,
        edit: None,
        draw: None,
        marking: false,
        bench: false,
        times: false,
        present: None,
        run_for: None,
        spin: false,
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
            "--click" => a.steps.push(Step::Click(v()?)),
            "--rclick" => a.steps.push(Step::RightClick(v()?)),
            "--dclick" => a.steps.push(Step::DoubleClick(v()?)),
            "--wait" => a.steps.push(Step::Wait),
            "--type" => a.steps.push(Step::Type(v()?)),
            "--key" => a.steps.push(Step::Key(v()?)),
            "--snap" => a.steps.push(Step::Snap(v()?)),
            "--save" => a.steps.push(Step::Save),
            "--home" => a.steps.push(Step::Home),
            "--viewnow" => {
                let y: f32 = v()?.parse().map_err(|_| USAGE)?;
                let p: f32 = it.next().and_then(|w| w.parse().ok()).ok_or(USAGE)?;
                a.steps.push(Step::View(y.to_radians(), p.to_radians()));
            }
            "--hover" | "--clickat" | "--drag" => {
                let n = if k == "--drag" { 4 } else { 2 };
                let mut xs = vec![];
                for _ in 0..n {
                    xs.push(
                        it.next()
                            .and_then(|w| w.parse::<f32>().ok())
                            .ok_or(format!("{k} takes {n} numbers"))?,
                    );
                }
                a.steps.push(match k.as_str() {
                    "--hover" => Step::Hover(xs[0], xs[1]),
                    "--clickat" => Step::ClickAt(xs[0], xs[1]),
                    _ => Step::Drag(xs[0], xs[1], xs[2], xs[3]),
                });
            }
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
            "--marking" => a.marking = true,
            "--bench" => a.bench = true,
            "--times" => a.times = true,
            "--present" => a.present = Some(v()?),
            "--run-for" => a.run_for = Some(v()?.parse().map_err(|_| USAGE)?),
            "--spin" => a.spin = true,
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
    // what a first look costs: each dialog's and menu's first frames against its second time
    let timed = |h: &mut Headless, name: &str, open: &dyn Fn(&mut Headless), shut: &dyn Fn(&mut Headless)| {
        let mut cost = [0.0f64; 2];
        for c in &mut cost {
            open(h);
            for _ in 0..3 {
                let o = h.step();
                *c = c.max(o.times.ui + o.times.tess + o.times.draw);
            }
            shut(h);
            for _ in 0..2 {
                h.step();
            }
        }
        println!("first look at {name}: {:.2} ms, again {:.2} ms", cost[0], cost[1]);
    };
    // a commit's frame: what taking in the new part costs, by what it is
    for part in ["bracket", "gearbox"] {
        h.gui.open(part);
        h.settle();
        for _ in 0..20 {
            h.step();
        }
        let Some(id) = h
            .gui
            .shell
            .rows
            .iter()
            .find(|r| r.kind == "extrude")
            .map(|r| r.id)
        else {
            continue;
        };
        let start = std::time::Instant::now();
        h.gui.shell.out.push(forge_render::shell::Action::Suppress(id));
        let (mut worst, mut ui, mut view, mut tess, mut draw) = (0.0f64, 0.0f64, 0.0f64, 0.0f64, 0.0f64);
        while start.elapsed().as_secs_f64() < 6.0 {
            let o = h.step();
            let t = o.times;
            if t.events > worst {
                worst = t.events;
            }
            ui = ui.max(t.ui);
            view = view.max(t.view);
            tess = tess.max(t.tess);
            draw = draw.max(t.draw);
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        println!(
            "a commit on {part}: worst worker's answers {worst:.2}, ui {ui:.2}, view {view:.2}, tess {tess:.2}, draw {draw:.2} ms"
        );
    }
    use forge_render::shell::Action as A;
    let shut: &dyn Fn(&mut Headless) = &|h| h.gui.shell.out.push(A::DialogCancel);
    for kind in ["extrude", "hole", "thread", "fillet", "pattern", "coil"] {
        let kind: &'static str = kind;
        timed(h, kind, &move |h| h.gui.shell.out.push(A::NewFeature(kind)), shut);
    }
    timed(
        h,
        "marking menu",
        &|h| {
            let c = h.gui.shell.view_rect.center();
            h.gui.shell.marking = Some(c);
        },
        &|h| h.gui.shell.marking = None,
    );
    timed(h, "Sketch tab", &|h| h.gui.shell.tab = 1, &|h| {
        h.gui.shell.tab = 0
    });
    timed(h, "Manage tab", &|h| h.gui.shell.tab = 4, &|h| {
        h.gui.shell.tab = 0
    });

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

/// Frames until the Forge file's builds and checks are done, or a long while.
/// The pointer to a place in the window, and frames till the view's pick there is back.
fn hover(h: &mut Headless, x: f32, y: f32) {
    h.move_to(x, y);
    for _ in 0..200 {
        h.step();
        std::thread::sleep(std::time::Duration::from_millis(5));
        if h.gui.view.as_ref().is_none_or(|v| !v.nav.pick_pending()) {
            break;
        }
    }
    for _ in 0..3 {
        h.step();
    }
}

fn wait(h: &mut Headless) {
    let t = std::time::Instant::now();
    while t.elapsed().as_secs() < 900 {
        h.step();
        let busy = h.gui.fg.as_ref().is_some_and(|s| s.done != s.seq)
            || h.gui
                .shell
                .fg
                .as_ref()
                .and_then(|p| p.asm.as_ref())
                .is_some_and(|a| a.checking);
        if !busy {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    // and what animates (the exploded view) the time it takes
    for _ in 0..40 {
        h.step();
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
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
        return forge_render::app::run(
            a.file,
            (a.size.0 as f64, a.size.1 as f64),
            a.times,
            a.present,
            a.run_for,
            a.spin,
        );
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
    for step in &a.steps {
        let (right, c) = match step {
            Step::Click(c) | Step::DoubleClick(c) => (false, c),
            Step::RightClick(c) => (true, c),
            Step::Wait => {
                wait(&mut h);
                continue;
            }
            Step::Type(t) => {
                h.text(t);
                h.step();
                h.step();
                continue;
            }
            Step::Key(k) => {
                let key = match k.as_str() {
                    "enter" => egui::Key::Enter,
                    "escape" => egui::Key::Escape,
                    "tab" => egui::Key::Tab,
                    "delete" => egui::Key::Delete,
                    _ => egui::Key::Backspace,
                };
                h.key(key);
                h.step();
                h.step();
                continue;
            }
            Step::Hover(x, y) => {
                hover(&mut h, *x, *y);
                continue;
            }
            Step::ClickAt(x, y) => {
                hover(&mut h, *x, *y);
                h.button(true);
                h.step();
                h.button(false);
                for _ in 0..4 {
                    h.step();
                }
                continue;
            }
            Step::Drag(x, y, x2, y2) => {
                hover(&mut h, *x, *y);
                h.button(true);
                h.step();
                for k in 1..=10 {
                    let t = k as f32 / 10.0;
                    h.move_to(x + (x2 - x) * t, y + (y2 - y) * t);
                    h.step();
                }
                h.button(false);
                for _ in 0..4 {
                    h.step();
                }
                continue;
            }
            Step::Snap(f) => {
                for _ in 0..3 {
                    h.step();
                }
                std::fs::write(f, h.ppm()).map_err(|e| e.to_string())?;
                continue;
            }
            Step::View(yaw, pitch) => {
                if let Some(v) = &mut h.gui.view {
                    v.nav.camera.yaw = *yaw;
                    v.nav.camera.pitch = *pitch;
                    v.mark_dirty();
                }
                for _ in 0..3 {
                    h.step();
                }
                continue;
            }
            Step::Home => {
                if let Some(v) = &mut h.gui.view {
                    v.nav.home(&v.scene);
                    v.mark_dirty();
                }
                // the camera's move there takes its time
                for _ in 0..60 {
                    h.step();
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                continue;
            }
            Step::Save => {
                h.gui.shell.fg_command(forge_render::shell::FgCmd::Save);
                h.step();
                h.step();
                continue;
            }
        };
        // by its name, or the first whose name starts so
        let found = h.gui.shell.place(c).or_else(|| {
            h.gui
                .shell
                .places
                .iter()
                .find(|p| p.0.starts_with(c.as_str()))
                .map(|p| p.1)
        });
        let Some(r) = found else {
            return Err(format!("nothing called {c}"));
        };
        h.move_to(r.center().x, r.center().y);
        h.step();
        if right {
            h.right_click_here();
        } else {
            h.button(true);
            h.step();
            h.button(false);
            if matches!(step, Step::DoubleClick(_)) {
                h.step();
                h.button(true);
                h.step();
                h.button(false);
            }
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
    if a.marking {
        h.gui.shell.marking = Some(h.gui.shell.view_rect.center());
        for _ in 0..3 {
            h.step();
        }
        let c = h.gui.shell.view_rect.center();
        h.move_to(c.x + 40.0, c.y - 40.0);
        for _ in 0..3 {
            h.step();
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
