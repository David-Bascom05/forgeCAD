//! A long chat costs what's on screen: entries out of sight are space, not drawn, and come back
//! as they're scrolled to, in their place.

use forge_render::shell::agent::{Entry, ToolState};

fn long(n: usize) -> Vec<Entry> {
    (0..n)
        .flat_map(|k| {
            [
                Entry::Note {
                    text: format!("Round {k}: the part builds, 0 errors"),
                    error: false,
                },
                Entry::Tool {
                    id: format!("t{k}"),
                    name: "Read".into(),
                    summary: format!("Read part{k}.fg"),
                    input: String::new(),
                    output: String::new(),
                    state: ToolState::Done,
                },
            ]
        })
        .collect()
}

fn drawn(h: &forge_render::gui::Headless) -> usize {
    h.gui
        .shell
        .places
        .iter()
        .filter(|p| p.0.starts_with("agent:note:"))
        .count()
}

#[test]
fn a_long_chat_draws_only_what_is_near_the_screen() {
    let Ok(mut h) = forge_render::gui::Headless::new((1400.0, 860.0), 1.0) else {
        return;
    };
    h.step();
    assert!(h.click("side:tab:Agent"));
    h.gui.shell.agent.restore_entries(long(500));
    // the first frame draws it all (heights not known yet), the next ones only what's near
    for _ in 0..4 {
        h.step();
    }
    let near = drawn(&h);
    assert!(near > 0 && near < 60, "{near} of 500 notes drawn");
    assert!(h.gui.shell.place("agent:note:990").is_some(), "the end, in sight");
    assert!(
        h.gui.shell.place("agent:note:0").is_none(),
        "the start, far above"
    );
    // the tool rows still count from the top: the last one is #500
    assert!(h.gui.shell.place("agent:tool#500").is_some());
    // scrolled to the top: the start drawn, in its place
    let chat = h.gui.shell.place("agent:note:998").unwrap();
    h.move_to(chat.center().x, chat.center().y);
    for _ in 0..200 {
        h.wheel(4000.0);
        h.step();
    }
    for _ in 0..4 {
        h.step();
    }
    let top = h.gui.shell.place("agent:note:0").expect("the start, scrolled to");
    assert!(top.top() < 400.0, "at the top of the chat: {top:?}");
    assert!(
        h.gui.shell.place("agent:note:990").is_none(),
        "the end, far below"
    );
    assert!(drawn(&h) < 60);
}
