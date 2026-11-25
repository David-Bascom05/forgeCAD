//! A conversation survives the app closing: the Agent tab reopens the project's last one, its
//! Diff cards whole (their lines and Undo), and an Undo after a restart takes the turn back only
//! while the file is as the turn left it, never over what's been done since.
// a fake agent CLI as a shell script: Unix only
#![cfg(unix)]

use forge_render::agent::conversations::{self, Conversation, TURN_FILE_BYTES};
use forge_render::agent::{FileRec, TurnRec};
use forge_render::shell::agent::Entry;
use forge_render::shell::agent_settings::{AgentSettings, Kind};
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

fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("forgecad-history-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    std::fs::canonicalize(d).unwrap()
}

fn rec(path: &str, before: Option<&str>, after: &str) -> FileRec {
    FileRec {
        path: path.into(),
        before: before.map(str::to_string),
        after: after.into(),
        key: 1,
        undone: false,
        trash: None,
        opened_from: None,
        restored: false,
    }
}

#[test]
fn a_saved_conversation_keeps_its_turns_and_diff_cards() {
    let dir = scratch("roundtrip");
    let store = dir.join("store");
    let mut c = Conversation::new(&dir, &AgentSettings::default(), "thicker").unwrap();
    let big = "x".repeat(TURN_FILE_BYTES + 1);
    c.turns = vec![TurnRec {
        label: "Agent: thicker".into(),
        files: vec![
            rec("/p/plate.fg", Some("param d = 12\n"), "param d = 14\n"),
            rec("/p/huge.fg", Some("small"), &big),
        ],
    }];
    c.entries = vec![
        Entry::User {
            text: "thicker".into(),
            refs: vec![],
            markup: None,
            pictures: 0,
        },
        Entry::Diff {
            turn: 0,
            index: 0,
            created: false,
            file: "plate.fg".into(),
            lines: vec![],
            added: 1,
            removed: 1,
            undone: false,
        },
        Entry::Diff {
            turn: 0,
            index: 1,
            created: false,
            file: "huge.fg".into(),
            lines: vec![],
            added: 1,
            removed: 1,
            undone: false,
        },
    ];
    c.save(&store).unwrap();
    let back = conversations::load(&store, &dir, &c.id).unwrap();
    // the card whole: its lines from the turn's before and after
    match &back.entries[1] {
        Entry::Diff { lines, file, .. } => {
            assert_eq!(file, "plate.fg");
            assert!(!lines.is_empty());
        }
        e => panic!("{e:?}"),
    }
    // too large to keep: said as a note, with no Undo to offer
    assert!(
        matches!(&back.entries[2], Entry::Note { text, .. } if text == "Changed huge.fg (+1 −1) · too large to undo after a resume")
    );
    let f = &back.turns[0].files[0];
    assert_eq!(
        (f.before.as_deref(), f.after.as_str()),
        (Some("param d = 12\n"), "param d = 14\n")
    );
    assert!(f.restored);
    assert!(!back.turns[0].files[1].restored, "the huge one isn't undoable");
    let _ = std::fs::remove_dir_all(dir);
}

/// A stand-in `claude`: each message, edits plate.fg 12 → 14 and ends the turn.
fn fake(dir: &Path) -> PathBuf {
    let init = r#"{"type":"system","subtype":"init","session_id":"s-1","model":"m"}"#;
    let tool = r#"{"type":"assistant","message":{"id":"m-1","content":[{"type":"tool_use","id":"t-1","name":"Edit","input":{"file_path":"plate.fg","old_string":"= 12","new_string":"= 14"}}]},"parent_tool_use_id":null}"#;
    let res = r#"{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"t-1","content":"ok"}]},"parent_tool_use_id":null}"#;
    let end = r#"{"type":"result","subtype":"success","session_id":"s-1"}"#;
    let script = dir.join("claude");
    std::fs::write(
        &script,
        format!(
            "#!/bin/sh\necho '{init}'\nwhile read line; do\n  echo '{tool}'\n  sed -i 's/= 12$/= 14/' '{d}/plate.fg'\n  echo '{res}'\n  echo '{end}'\ndone\n",
            d = dir.display()
        ),
    )
    .unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    script
}

fn app(dir: &Path) -> Option<forge_render::gui::Headless> {
    let mut h = forge_render::gui::Headless::new((1400.0, 860.0), 1.0).ok()?;
    h.gui.agent.program = Some(fake(dir).display().to_string());
    h.gui.agent.cwd = Some(dir.join("cwd"));
    h.gui.agent.settings_file = Some(dir.join("agent.toml"));
    h.gui.agent.project_dir = Some(dir.to_path_buf());
    h.gui.agent.conversations_dir = Some(dir.join("store"));
    h.step();
    AgentSettings {
        kind: Kind::ClaudeCode,
        ..Default::default()
    }
    .write_extra(&mut h.gui.shell.settings.extra);
    h.gui.open(dir.join("plate.fg").to_str().unwrap());
    h.settle();
    assert!(h.click("side:tab:Agent"));
    for _ in 0..3 {
        h.step();
    }
    Some(h)
}

fn wait(h: &mut forge_render::gui::Headless, what: impl Fn(&forge_render::gui::Gui) -> bool) -> bool {
    let t = std::time::Instant::now();
    while t.elapsed().as_secs() < 60 {
        h.step();
        if what(&h.gui) {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    false
}

fn diff_undone(g: &forge_render::gui::Gui) -> Option<bool> {
    g.shell.agent.entries.iter().find_map(|e| match e {
        Entry::Diff { undone, .. } => Some(*undone),
        _ => None,
    })
}

#[test]
fn after_a_restart_the_last_chat_is_back_and_its_undo_works_while_the_file_is_as_it_left_it() {
    let dir = scratch("restart");
    std::fs::write(dir.join("plate.fg"), PART).unwrap();
    let Some(mut h) = app(&dir) else { return };
    assert!(h.click("agent:input"));
    h.text("make it thicker");
    assert!(h.click("agent:send"));
    assert!(wait(&mut h, |g| !g.shell.agent.running
        && diff_undone(g) == Some(false)));
    assert!(
        std::fs::read_to_string(dir.join("plate.fg"))
            .unwrap()
            .contains("= 14")
    );
    drop(h);

    // the app again: the last conversation comes back with its card whole
    let Some(mut h) = app(&dir) else { return };
    assert_eq!(
        diff_undone(&h.gui),
        Some(false),
        "the Diff card is back, not a note"
    );
    if let Ok(shots) = std::env::var("FORGE_UI_SHOTS") {
        for _ in 0..12 {
            h.step();
        }
        std::fs::create_dir_all(&shots).unwrap();
        let png = h.png().unwrap();
        std::fs::write(Path::new(&shots).join("history-1400-dark-reopened.png"), png).unwrap();
    }
    // where it was resumed: a divider saying when it was from, not a note saved with it
    let (at, said) = h.gui.shell.agent.resumed.clone().unwrap();
    assert_eq!(at, h.gui.shell.agent.entries.len());
    assert_eq!(said, "Resumed · just now");
    assert!(h.gui.shell.place("agent:resumed:new").is_some());
    assert!(
        !h.gui
            .shell
            .agent
            .entries
            .iter()
            .any(|e| matches!(e, Entry::Note { .. })),
        "no note"
    );
    assert!(h.gui.shell.place("agent:undo:0:0").is_some(), "with its Undo");
    assert!(h.click("agent:undo:0:0"));
    h.step();
    assert_eq!(diff_undone(&h.gui), Some(true));
    assert!(
        h.gui.fg.as_ref().unwrap().src.contains("param extrude1.d = 12"),
        "taken back"
    );
    // and Redo puts it back
    assert!(h.click("agent:undo:0:0"), "the same button, now Redo");
    h.step();
    assert_eq!(diff_undone(&h.gui), Some(false));
    assert!(h.gui.fg.as_ref().unwrap().src.contains("param extrude1.d = 14"));
    // the part edited since: its Undo refuses, and says where to go instead
    let mut s = h.gui.fg.as_ref().unwrap().src.clone();
    s = s.replace("param sk1.w3 = 80", "param sk1.w3 = 90");
    std::fs::write(dir.join("plate.fg"), &s).unwrap();
    h.gui.open(dir.join("plate.fg").to_str().unwrap());
    h.settle();
    h.gui.shell.agent.tab = forge_render::shell::agent::SideTab::Agent;
    h.step();
    h.gui.agent_flip(0, 0, true).unwrap_err();
    assert!(
        std::fs::read_to_string(dir.join("plate.fg"))
            .unwrap()
            .contains("sk1.w3 = 90"),
        "nothing lost"
    );
    // its New chat: a fresh one, the old one kept in Chats
    assert!(h.click("agent:resumed:new"));
    h.step();
    assert!(h.gui.shell.agent.entries.is_empty());
    assert_eq!(h.gui.shell.agent.resumed, None);
    let _ = std::fs::remove_dir_all(dir);
}
