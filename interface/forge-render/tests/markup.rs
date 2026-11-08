//! Markup for the agent: marks drawn over the view go with the message as a picture and as the
//! names of what they're on, and stay with the message in the chat.

use forge_render::shell::agent::Entry;
use forge_render::shell::markup::Tool;
use std::path::PathBuf;

const PART: &str = "param sk1.x1 = -40
param sk1.y2 = -30
param sk1.w3 = 80
param sk1.h4 = 60
sketch sk1(on: xy) {
  r1 = rect(corner: (x1, y2), size: (w3, h4))
}
param extrude1.d = 12
extrude1 = extrude(sk1, d)
";

fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("forgecad-markup-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn drag(h: &mut forge_render::gui::Headless, pts: &[(f32, f32)]) {
    h.move_to(pts[0].0, pts[0].1);
    h.step();
    h.button(true);
    h.step();
    for p in &pts[1..] {
        h.move_to(p.0, p.1);
        h.step();
    }
    h.button(false);
    h.step();
}

/// A stand-in `claude` for the app: what it's sent, kept in `got.json`; a turn that ends at once.
fn stand_in(h: &mut forge_render::gui::Headless, dir: &std::path::Path) {
    let script = dir.join("claude");
    std::fs::write(
        &script,
        format!(
            "#!/bin/sh\nread -r line\nprintf '%s' \"$line\" > '{0}/got.json'\necho '{{\"type\":\"result\",\"subtype\":\"success\",\"is_error\":false,\"result\":\"ok\"}}'\nwhile read line; do :; done\n",
            dir.display()
        ),
    )
    .unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    h.gui.agent.program = Some(script.display().to_string());
    h.gui.agent.cwd = Some(dir.join("cwd"));
    h.gui.agent.settings_file = Some(dir.join("agent.toml"));
    std::fs::write(
        dir.join("agent.toml"),
        format!(
            "backend = \"claude-code\"\ntrusted = [\"{}\"]\n",
            dir.canonicalize().unwrap().display()
        ),
    )
    .unwrap();
}

/// What the stand-in was sent, once it has been.
fn sent(h: &mut forge_render::gui::Headless, dir: &std::path::Path) -> String {
    let t = std::time::Instant::now();
    let got = dir.join("got.json");
    loop {
        h.step();
        if let Ok(text) = std::fs::read_to_string(&got)
            && forge_agent::json::Json::parse(&text).is_ok()
        {
            return text;
        }
        assert!(t.elapsed().as_secs() < 60, "markup message did not arrive");
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
}

#[test]
fn marks_go_with_the_message_as_a_picture_and_the_names_of_what_they_are_on() {
    let Ok(mut h) = forge_render::gui::Headless::new((1200.0, 800.0), 1.0) else {
        return;
    };
    let dir = scratch("send");
    let file = dir.join("plate.fg");
    std::fs::write(&file, PART).unwrap();
    stand_in(&mut h, &dir);
    h.step();
    h.gui.open(file.to_str().unwrap());
    h.settle();
    assert!(h.click("side:tab:Agent"));
    h.step();
    h.key(egui::Key::K);
    h.step();
    assert!(h.gui.shell.agent.markup.on, "K turns markup on");
    assert!(h.gui.shell.place("markup:pen").is_some(), "its tool strip");
    let v = h.gui.shell.view_rect;
    let c = v.center();
    // a rectangle round the whole view, an arrow to its middle, a word at the arrow's tail
    assert!(h.click("markup:rectangle"));
    h.step();
    drag(
        &mut h,
        &[
            (v.left() + 12.0, v.top() + 60.0),
            (v.right() - 12.0, v.bottom() - 12.0),
        ],
    );
    assert!(h.click("markup:arrowmark"));
    h.step();
    drag(
        &mut h,
        &[(c.x - 200.0, c.y - 150.0), (c.x - 100.0, c.y - 75.0), (c.x, c.y)],
    );
    assert!(h.click("markup:text"));
    h.step();
    h.move_to(c.x - 260.0, c.y - 170.0);
    h.step();
    h.button(true);
    h.step();
    h.button(false);
    h.step();
    h.step();
    h.text("thicker");
    h.step();
    let m = &h.gui.shell.agent.markup;
    assert_eq!(
        m.marks.iter().map(|m| m.tool).collect::<Vec<_>>(),
        [Tool::Rect, Tool::Arrow]
    );
    assert_eq!(m.typing.as_ref().map(|t| t.1.as_str()), Some("thicker"));
    // the view kept drawing: the camera's not been touched by the pen
    assert!(h.click("agent:send"));
    let sent = sent(&mut h, &dir);
    assert!(
        sent.contains(r#""type":"image""#) && sent.contains("image/png"),
        "{sent}"
    );
    assert!(sent.contains("Marked on the screen"), "{sent}");
    assert!(sent.contains("1. rectangle, red"), "{sent}");
    assert!(sent.contains("around the solid (`solid`)"), "{sent}");
    assert!(sent.contains("2. arrow, red"), "{sent}");
    assert!(sent.contains("pointing to face `extrude1.end`"), "{sent}");
    assert!(
        sent.contains("3. text \\\"thicker\\\"") && sent.contains("at the tail of mark 2"),
        "{sent}"
    );
    // the marks went with it, kept with the message
    assert!(h.gui.shell.agent.markup.marks.is_empty());
    let kept = h.gui.shell.agent.entries.iter().find_map(|e| match e {
        Entry::User { markup, .. } => markup.clone(),
        _ => None,
    });
    let kept = kept.expect("the message keeps its marks");
    assert_eq!(kept.marks.len(), 3);
    assert!(kept.thumb.w <= 200 && kept.thumb.h <= 200 && kept.thumb.w > 0);
    // Esc leaves markup; the thumbnail shows the marks again
    h.key(egui::Key::Escape);
    h.step();
    assert!(!h.gui.shell.agent.markup.on);
    h.step();
    assert!(h.click("agent:markup:0"));
    h.step();
    assert_eq!(h.gui.shell.agent.markup.shown, Some(0));
    drop(h);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn marks_on_a_drawing_name_its_views_and_annotations() {
    let Ok(mut h) = forge_render::gui::Headless::new((1200.0, 800.0), 1.0) else {
        return;
    };
    let dir = scratch("dwg");
    let ex = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/machined");
    for f in ["flange.fg", "flange_dwg.fg"] {
        std::fs::copy(ex.join(f), dir.join(f)).unwrap();
    }
    stand_in(&mut h, &dir);
    h.step();
    h.gui.open(dir.join("flange_dwg.fg").to_str().unwrap());
    // the sheet made: a view on it to name
    let t = std::time::Instant::now();
    let mut at = None;
    while at.is_none() && t.elapsed().as_secs() < 60 {
        h.step();
        let v = h.gui.shell.view_rect;
        let d = h.gui.shell.drawing.as_ref().unwrap();
        at = (0..1600)
            .map(|k| {
                egui::pos2(
                    v.left() + v.width() * (k % 40) as f32 / 40.0,
                    v.top() + v.height() * (k / 40) as f32 / 40.0,
                )
            })
            .find(|p| d.markup_at(*p, 0.0).is_some_and(|n| n.starts_with("view")));
    }
    let at = at.expect("a view on the sheet");
    let v = h.gui.shell.view_rect;
    assert!(h.click("side:tab:Agent"));
    h.step();
    h.key(egui::Key::K);
    h.step();
    assert!(h.gui.shell.agent.markup.on);
    assert!(h.click("markup:ellipse"));
    h.step();
    drag(&mut h, &[(at.x - 40.0, at.y - 40.0), (at.x + 40.0, at.y + 40.0)]);
    assert!(h.click("markup:arrowmark"));
    h.step();
    drag(&mut h, &[(v.left() + 20.0, v.top() + 80.0), (at.x, at.y)]);
    assert!(h.click("agent:send"));
    let sent = sent(&mut h, &dir);
    assert!(sent.contains(r#""type":"image""#), "{sent}");
    assert!(
        sent.contains("1. ellipse, red") && sent.contains("around view `"),
        "{sent}"
    );
    assert!(
        sent.contains("2. arrow, red") && sent.contains("pointing to "),
        "{sent}"
    );
    drop(h);
    let _ = std::fs::remove_dir_all(dir);
}

/// Where each placement shows in the view, in window points: its pixels' box, and one of its
/// faces' middle pixels.
fn shown(h: &mut forge_render::gui::Headless) -> Vec<(u32, egui::Rect, egui::Pos2)> {
    let v = h.gui.view.as_mut().unwrap();
    let cam = v.nav.camera;
    let ids = v.renderer.view_ids(&mut v.scene, &cam);
    v.mark_dirty();
    let at = h.gui.shell.view_rect.min;
    let mut out: Vec<(u32, egui::Rect, Vec<egui::Pos2>)> = Vec::new();
    for y in 0..ids.h {
        for x in 0..ids.w {
            let Some((pl, e)) = ids.at(x, y) else { continue };
            let p = at + egui::vec2(x as f32, y as f32);
            match out.iter_mut().find(|o| o.0 == pl) {
                Some(o) => {
                    o.1.extend_with(p);
                    if matches!(e, forge_render::render::Entity::Face(_)) {
                        o.2.push(p);
                    }
                }
                None => out.push((pl, egui::Rect::from_min_max(p, p), vec![p])),
            }
        }
    }
    out.into_iter()
        .map(|(pl, r, f)| (pl, r, f[f.len() / 2]))
        .collect()
}

#[test]
#[ignore = "a real Claude Code call: run by hand, signed in"]
fn the_agent_reads_a_circled_part_an_arrow_and_a_word() {
    let Ok(mut h) = forge_render::gui::Headless::new((1400.0, 900.0), 1.0) else {
        return;
    };
    let dir = scratch("real");
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/unified/nested_boxes.fg");
    let file = dir.join("nested_boxes.fg");
    std::fs::copy(src, &file).unwrap();
    // claude itself, what it's sent kept to look at
    let script = dir.join("claude");
    std::fs::write(
        &script,
        format!(
            "#!/bin/sh\ntee '{}/stdin.jsonl' | exec claude \"$@\"\n",
            dir.display()
        ),
    )
    .unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    h.gui.agent.program = Some(script.display().to_string());
    h.gui.agent.mcp_exe = Some(env!("CARGO_BIN_EXE_forge-app").into());
    h.gui.agent.cwd = Some(dir.join("cwd"));
    h.gui.agent.settings_file = Some(dir.join("agent.toml"));
    std::fs::write(
        dir.join("agent.toml"),
        format!(
            "backend = \"claude-code\"\nclaude_model = \"sonnet\"\ntrusted = [\"{}\"]\n",
            dir.canonicalize().unwrap().display()
        ),
    )
    .unwrap();
    h.step();
    h.gui.open(file.to_str().unwrap());
    let t = std::time::Instant::now();
    while t.elapsed().as_secs() < 120 {
        h.step();
        let ready = h.gui.fg.as_ref().is_some_and(|s| s.done == s.seq)
            && h.gui.fg_asm.as_ref().is_some_and(|a| a.0.len() >= 4);
        if ready {
            break;
        }
    }
    for _ in 0..200 {
        h.step();
    }
    let occ = h.gui.fg_asm.as_ref().unwrap().0.clone();
    eprintln!(
        "{:?}",
        occ.iter().map(|o| (&o.path, o.placement)).collect::<Vec<_>>()
    );
    let pl = |path: &str| {
        occ.iter()
            .filter(|o| o.path == path || o.path.starts_with(&format!("{path}.")))
            .find_map(|o| o.placement)
            .unwrap() as u32
    };
    let (b2, b1) = (pl("b2"), pl("b1"));
    let seen = shown(&mut h);
    let r2 = seen.iter().find(|s| s.0 == b2).unwrap().1;
    let f1 = seen.iter().find(|s| s.0 == b1).unwrap().2;
    assert!(h.click("side:tab:Agent"));
    h.step();
    assert!(h.click("status:markup"));
    h.step();
    assert!(h.click("markup:ellipse"));
    h.step();
    let e = r2.expand(25.0);
    drag(
        &mut h,
        &[
            (e.left(), e.top()),
            (e.center().x, e.center().y),
            (e.right(), e.bottom()),
        ],
    );
    // a long pen stroke, each frame timed: drawing must keep 60 fps
    assert!(h.click("markup:pen"));
    h.step();
    let v = h.gui.shell.view_rect;
    let (cx, cy) = (v.right() - 150.0, v.bottom() - 120.0);
    h.move_to(cx + 60.0, cy);
    h.step();
    h.button(true);
    h.step();
    let mut times = Vec::new();
    for k in 1..=400 {
        let a = k as f32 / 400.0 * 6.0;
        h.move_to(cx + 60.0 * a.cos(), cy + 40.0 * a.sin());
        let t = std::time::Instant::now();
        h.step();
        times.push(t.elapsed().as_secs_f64() * 1e3);
    }
    h.button(false);
    h.step();
    times.sort_by(f64::total_cmp);
    eprintln!(
        "pen frames: median {:.2} ms, p99 {:.2} ms, max {:.2} ms over {}",
        times[times.len() / 2],
        times[times.len() * 99 / 100],
        times[times.len() - 1],
        times.len()
    );
    assert!(h.click("markup:arrowmark"));
    h.step();
    let tail = (f1.x - 120.0, f1.y - 110.0);
    drag(
        &mut h,
        &[tail, ((tail.0 + f1.x) / 2.0, (tail.1 + f1.y) / 2.0), (f1.x, f1.y)],
    );
    assert!(h.click("markup:text"));
    h.step();
    h.move_to(tail.0 - 40.0, tail.1 - 22.0);
    h.step();
    h.button(true);
    h.step();
    h.button(false);
    h.step();
    h.step();
    h.text("thicker");
    h.step();
    std::fs::write(dir.join("drawn.ppm"), h.ppm()).unwrap();
    assert!(h.click("agent:input"));
    h.step();
    h.text(
        "Don't change anything. Tell me exactly what my marks refer to, by their stable names, and what you see in the picture.",
    );
    h.step();
    assert!(h.click("agent:send"));
    let t = std::time::Instant::now();
    h.step();
    while h.gui.shell.agent.running && t.elapsed().as_secs() < 300 {
        h.step();
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    for _ in 0..30 {
        h.step();
    }
    std::fs::write(dir.join("after.ppm"), h.ppm()).unwrap();
    let log = format!("{:#?}", h.gui.shell.agent.entries);
    std::fs::write(dir.join("entries.txt"), &log).unwrap();
    eprintln!("{}\n{log}", dir.display());
    let said = h.gui.shell.agent.entries.iter().find_map(|e| match e {
        Entry::User { markup: Some(m), .. } => Some(m.said.clone()),
        _ => None,
    });
    let said = said.unwrap();
    assert!(said.contains("around occurrence `b2"), "{said}");
    assert!(
        said.contains("occurrence `b1") && said.contains("› face `"),
        "{said}"
    );
}

/// Claude Code itself for the app, what it's sent kept in `stdin.jsonl`.
fn real_claude(h: &mut forge_render::gui::Headless, dir: &std::path::Path) {
    let script = dir.join("claude");
    std::fs::write(
        &script,
        format!(
            "#!/bin/sh\ntee '{}/stdin.jsonl' | exec claude \"$@\"\n",
            dir.display()
        ),
    )
    .unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    h.gui.agent.program = Some(script.display().to_string());
    h.gui.agent.mcp_exe = Some(env!("CARGO_BIN_EXE_forge-app").into());
    h.gui.agent.cwd = Some(dir.join("cwd"));
    h.gui.agent.settings_file = Some(dir.join("agent.toml"));
    std::fs::write(
        dir.join("agent.toml"),
        format!(
            "backend = \"claude-code\"\nclaude_model = \"sonnet\"\ntrusted = [\"{}\"]\n",
            dir.canonicalize().unwrap().display()
        ),
    )
    .unwrap();
}

/// A message to the real agent, till its turn is over; the window before and after kept.
fn ask(h: &mut forge_render::gui::Headless, dir: &std::path::Path, text: &str) -> String {
    std::fs::write(dir.join("drawn.ppm"), h.ppm()).unwrap();
    assert!(h.click("agent:input"));
    h.step();
    h.text(text);
    h.step();
    assert!(h.click("agent:send"));
    let t = std::time::Instant::now();
    h.step();
    while h.gui.shell.agent.running && t.elapsed().as_secs() < 300 {
        h.step();
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    for _ in 0..30 {
        h.step();
    }
    std::fs::write(dir.join("after.ppm"), h.ppm()).unwrap();
    let log = format!("{:#?}", h.gui.shell.agent.entries);
    std::fs::write(dir.join("entries.txt"), &log).unwrap();
    eprintln!("{}", dir.display());
    h.gui
        .shell
        .agent
        .entries
        .iter()
        .find_map(|e| match e {
            Entry::User { markup: Some(m), .. } => Some(m.said.clone()),
            _ => None,
        })
        .unwrap()
}

#[test]
#[ignore = "a real Claude Code call: run by hand, signed in"]
fn the_agent_reads_a_hole_in_a_section_a_dimension_and_the_title_block() {
    let Ok(mut h) = forge_render::gui::Headless::new((1400.0, 900.0), 1.0) else {
        return;
    };
    let dir = scratch("real-dwg");
    let ex = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/machined");
    for f in ["flange.fg", "flange_dwg.fg"] {
        std::fs::copy(ex.join(f), dir.join(f)).unwrap();
    }
    real_claude(&mut h, &dir);
    h.step();
    h.gui.open(dir.join("flange_dwg.fg").to_str().unwrap());
    // where on the screen the sheet has a hole in the section, a dimension, the title
    let find = |h: &forge_render::gui::Headless, want: &dyn Fn(&str) -> bool| {
        let v = h.gui.shell.view_rect;
        let d = h.gui.shell.drawing.as_ref().unwrap();
        (0..200 * 130)
            .map(|k| {
                egui::pos2(
                    v.left() + v.width() * (k % 200) as f32 / 200.0,
                    v.top() + v.height() * (k / 200) as f32 / 130.0,
                )
            })
            .find(|p| d.markup_at(*p, 0.0).is_some_and(|n| want(&n)))
    };
    // the section's middle edge: the flange's bore, cut through
    let grid = |h: &forge_render::gui::Headless| -> Vec<(egui::Pos2, String)> {
        let v = h.gui.shell.view_rect;
        let d = h.gui.shell.drawing.as_ref().unwrap();
        (0..240 * 160)
            .map(|k| {
                egui::pos2(
                    v.left() + v.width() * (k % 240) as f32 / 240.0,
                    v.top() + v.height() * (k / 240) as f32 / 160.0,
                )
            })
            .filter_map(|p| d.markup_at(p, 0.0).map(|n| (p, n)))
            .filter(|(_, n)| n.contains("(section)"))
            .collect()
    };
    let t = std::time::Instant::now();
    let mut cut = vec![];
    while cut
        .iter()
        .all(|(_, n): &(egui::Pos2, String)| n.starts_with("view"))
        && t.elapsed().as_secs() < 120
    {
        for _ in 0..20 {
            h.step();
        }
        cut = grid(&h);
    }
    let mid = cut.iter().fold(egui::Vec2::ZERO, |a, (p, _)| a + p.to_vec2()) / cut.len().max(1) as f32;
    let (hole, name) = cut
        .iter()
        .filter(|(_, n)| !n.starts_with("view"))
        .min_by(|a, b| {
            (a.0.to_vec2() - mid)
                .length()
                .total_cmp(&(b.0.to_vec2() - mid).length())
        })
        .cloned()
        .expect("an edge in the section view");
    eprintln!("the section's middle edge: {name}");
    let hole = egui::pos2(hole.x, hole.y);
    let dim = find(&h, &|n| n.starts_with("dimension")).expect("a dimension");
    let title = find(&h, &|n| n.contains("field TITLE")).expect("the title block");
    assert!(h.click("side:tab:Agent"));
    h.step();
    h.key(egui::Key::K);
    h.step();
    assert!(h.click("markup:arrowmark"));
    h.step();
    drag(
        &mut h,
        &[
            (hole.x - 90.0, hole.y - 70.0),
            (hole.x - 45.0, hole.y - 35.0),
            (hole.x, hole.y),
        ],
    );
    assert!(h.click("markup:ellipse"));
    h.step();
    drag(
        &mut h,
        &[(dim.x - 30.0, dim.y - 14.0), (dim.x + 30.0, dim.y + 14.0)],
    );
    assert!(h.click("markup:text"));
    h.step();
    h.move_to(title.x - 30.0, title.y - 40.0);
    h.step();
    h.button(true);
    h.step();
    h.button(false);
    h.step();
    h.step();
    h.text("say BEARING HOUSING");
    h.step();
    assert!(h.click("markup:arrowmark"));
    h.step();
    drag(&mut h, &[(title.x - 10.0, title.y - 25.0), (title.x, title.y)]);
    let said = ask(
        &mut h,
        &dir,
        "Don't change anything. Tell me exactly what each of my marks refers to, by the names you'd use with your tools, and what you see in the picture.",
    );
    assert!(
        said.contains("pointing to ") && said.contains("(section)"),
        "{said}"
    );
    assert!(said.contains("around dimension"), "{said}");
    assert!(said.contains("title block field TITLE"), "{said}");
}
