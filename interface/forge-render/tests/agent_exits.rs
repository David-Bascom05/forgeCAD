//! BUTTON-AUDITOR's agent-tab findings: every window the tab opens has a way out (a button and
//! Esc), and markup says in words what it is until the first mark.

fn shell() -> Option<forge_render::gui::Headless> {
    let mut h = forge_render::gui::Headless::new((1400.0, 900.0), 1.0).ok()?;
    h.step();
    assert!(h.click("side:tab:Agent"));
    h.step();
    Some(h)
}

/// `FORGE_UI_SHOTS=DIR` writes what each looks like.
fn shot(h: &mut forge_render::gui::Headless, name: &str) {
    if let Ok(dir) = std::env::var("FORGE_UI_SHOTS") {
        // past the fade-in
        for _ in 0..30 {
            h.step();
        }
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(format!("{dir}/{name}.png"), h.png().unwrap()).unwrap();
    }
}

#[test]
fn resume_closes_by_cancel_and_by_esc() {
    let Some(mut h) = shell() else {
        return;
    };
    h.gui.shell.agent.resume_open = true;
    h.step();
    h.step();
    shot(&mut h, "agent-resume-cancel");
    assert!(h.click("agent:resume:cancel"));
    h.step();
    assert!(!h.gui.shell.agent.resume_open, "Cancel closes Resume");
    h.gui.shell.agent.resume_open = true;
    h.step();
    h.step();
    h.key(egui::Key::Escape);
    h.step();
    assert!(!h.gui.shell.agent.resume_open, "Esc closes Resume");
}

#[test]
fn the_switcher_closes_by_close_and_by_esc() {
    let Some(mut h) = shell() else {
        return;
    };
    let before = h.gui.shell.agent.settings.clone();
    assert!(h.click("agent:switch"));
    h.step();
    assert!(h.gui.shell.agent.switcher);
    h.step();
    shot(&mut h, "agent-switcher-close");
    assert!(h.click("agent:switch:close"));
    h.step();
    assert!(!h.gui.shell.agent.switcher, "Close closes the switcher");
    assert!(h.click("agent:switch"));
    h.step();
    assert!(h.gui.shell.agent.switcher);
    h.key(egui::Key::Escape);
    h.step();
    assert!(!h.gui.shell.agent.switcher, "Esc closes the switcher");
    // and neither changed the backend
    assert_eq!(h.gui.shell.agent.settings, before);
}

#[test]
fn markup_says_what_it_is_until_the_first_mark() {
    let Some(mut h) = shell() else {
        return;
    };
    assert!(h.click("agent:markup"));
    h.step();
    h.step();
    assert!(h.gui.shell.agent.markup.on);
    let hint = h.placed("markup:hint").expect("markup says what it is");
    let strip = h.placed("markup:done").unwrap();
    assert!(hint.top() > strip.bottom(), "under the strip");
    shot(&mut h, "agent-markup-hint");
    // the first mark: the words go
    let s = h.placed("markup:surface:0").unwrap();
    let a = s.center();
    h.gui
        .shell
        .agent
        .markup
        .marks
        .push(forge_render::shell::markup::Mark {
            tool: forge_render::shell::markup::Tool::Pen,
            ink: 0,
            pts: vec![a, a + egui::vec2(40.0, 30.0)],
            text: String::new(),
        });
    h.step();
    h.step();
    assert!(h.placed("markup:hint").is_none(), "gone once there's a mark");
}
