//! Audit pictures of the Agent tab (research/agent-tab): `FORGE_UI_SHOTS=DIR` writes them.
#![cfg(unix)]

use forge_render::gui::Headless;
use forge_render::shell::agent::{Entry, SideTab, ToolState};
use std::path::{Path, PathBuf};

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

fn fixture(name: &str) -> String {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/agent").join(name)).unwrap()
}

fn wait(h: &mut Headless, what: impl Fn(&forge_render::gui::Gui) -> bool) -> bool {
    let t = std::time::Instant::now();
    while t.elapsed().as_secs() < 120 {
        h.step();
        if what(&h.gui) {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    false
}

fn shot(h: &mut Headless, name: &str) {
    for _ in 0..12 {
        h.step();
    }
    if let Ok(dir) = std::env::var("FORGE_UI_SHOTS") {
        std::fs::create_dir_all(&dir).unwrap();
        if let Ok(png) = h.png() {
            std::fs::write(Path::new(&dir).join(format!("{name}.png")), png).unwrap();
        }
    }
}

fn fake_claude(dir: &Path) -> PathBuf {
    let lines: Vec<String> = fixture("edit_turn.jsonl").lines().map(str::to_string).collect();
    let cut = lines.iter().position(|l| l.contains("has been updated successfully")).unwrap();
    std::fs::write(dir.join("head.jsonl"), lines[..cut].join("\n") + "\n").unwrap();
    std::fs::write(dir.join("tail.jsonl"), lines[cut..].join("\n") + "\n").unwrap();
    let script = dir.join("claude");
    std::fs::write(
        &script,
        format!(
            "#!/bin/sh\nread line\ncat '{0}/head.jsonl'\nsed -i 's/^param extrude1.d = 12$/param extrude1.d = 14/' '{0}/plate.fg'\ncat '{0}/tail.jsonl'\nwhile read line; do :; done\n",
            dir.display()
        ),
    )
    .unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    script
}

#[test]
fn agent_tab_audit_shots() {
    let Ok(mut h) = Headless::new((1400.0, 900.0), 1.0) else {
        return;
    };
    let dir = std::env::temp_dir().join(format!("forgecad-agent-audit-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("plate.fg");
    std::fs::write(&file, PART).unwrap();
    h.gui.agent.program = Some(fake_claude(&dir).display().to_string());
    h.gui.agent.cwd = Some(dir.join("cwd"));
    h.gui.agent.settings_file = Some(dir.join("agent.toml"));
    h.step();
    h.gui.open(file.to_str().unwrap());
    h.settle();
    assert!(h.click("side:tab:Agent"));
    shot(&mut h, "01-empty");
    assert!(h.click("agent:input"));
    h.text("make the base plate thicker");
    h.step();
    shot(&mut h, "02-typed");
    let t = std::time::Instant::now();
    assert!(h.click("agent:send"));
    assert!(wait(&mut h, |g| !g.shell.agent.running
        && g.shell.agent.entries.iter().any(|e| matches!(e, Entry::Diff { .. }))));
    eprintln!("fake turn round trip: {:?}", t.elapsed());
    shot(&mut h, "03-after-turn");
    // open every tool row
    let ids: Vec<String> = h
        .gui
        .shell
        .agent
        .entries
        .iter()
        .filter_map(|e| match e {
            Entry::Tool { id, .. } => Some(id.clone()),
            _ => None,
        })
        .collect();
    for id in ids {
        h.gui.shell.agent.open.insert(id);
    }
    shot(&mut h, "04-tools-open");
    // a long transcript: frame time
    let mut long = Vec::new();
    for k in 0..400 {
        long.push(Entry::User { text: format!("request {k}: make the plate {k} mm thicker and fillet every convex edge"), refs: vec![], markup: None });
        long.push(Entry::Tool { id: format!("t{k}"), name: "Edit".into(), summary: "Edit plate.fg".into(), input: "x".into(), output: "ok".into(), state: ToolState::Done });
        long.push(Entry::Agent { text: "Done: raised `extrude1.d` to 14 mm; the part builds with no new warnings.\n\n```\nparam extrude1.d = 14\n```".repeat(2) });
    }
    h.gui.shell.agent.restore_entries(long);
    for _ in 0..5 {
        h.step();
    }
    let t = std::time::Instant::now();
    for _ in 0..30 {
        h.step();
    }
    eprintln!("frame with 1200 entries: {:?}", t.elapsed() / 30);
    h.gui.shell.agent.restore_entries(vec![]);
    for _ in 0..5 {
        h.step();
    }
    let t = std::time::Instant::now();
    for _ in 0..30 {
        h.step();
    }
    eprintln!("frame with 0 entries: {:?}", t.elapsed() / 30);
    // not installed
    drop(h);
    let Ok(mut h) = Headless::new((1400.0, 900.0), 1.0) else {
        return;
    };
    h.gui.agent.program = Some("/nonexistent/claude".into());
    h.gui.agent.cwd = Some(dir.join("cwd"));
    h.gui.agent.settings_file = Some(dir.join("agent.toml"));
    h.step();
    h.gui.open(file.to_str().unwrap());
    h.settle();
    assert!(h.click("side:tab:Agent"));
    assert!(h.click("agent:input"));
    h.text("hello");
    assert!(h.click("agent:send"));
    assert!(wait(&mut h, |g| g.shell.agent.problem.is_some()));
    shot(&mut h, "05-not-installed");
    assert_eq!(h.gui.shell.agent.tab, SideTab::Agent);
    let _ = std::fs::remove_dir_all(dir);
}
