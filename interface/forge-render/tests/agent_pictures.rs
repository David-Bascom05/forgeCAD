//! Pictures with a message: the view attached (a picture of it, no marks), pictures pasted or
//! dropped (PNG, JPEG, GIF, WebP, by their bytes), each with its ×, all of them sent to the agent
//! as image blocks the text reads after, and the message saying how many went.
// a fake agent CLI as a shell script: Unix only
#![cfg(unix)]

use forge_agent::json::Json;
use forge_render::agent::{Picture, claude};
use forge_render::shell::agent::{AgentAct, AgentTab, Entry};
use forge_render::shell::agent_settings::{AgentSettings, Kind};
use std::path::{Path, PathBuf};

const PNG: &[u8] = &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 13];
const JPEG: &[u8] = &[0xFF, 0xD8, 0xFF, 0xE0, 0, 16];

#[test]
fn a_picture_is_known_by_its_bytes_not_its_name() {
    assert_eq!(Picture::sniff(PNG.to_vec()).map(|p| p.media), Some("image/png"));
    assert_eq!(Picture::sniff(JPEG.to_vec()).map(|p| p.media), Some("image/jpeg"));
    assert_eq!(
        Picture::sniff(b"GIF89a....".to_vec()).map(|p| p.media),
        Some("image/gif")
    );
    assert_eq!(
        Picture::sniff(b"RIFF\0\0\0\0WEBPVP8 ".to_vec()).map(|p| p.media),
        Some("image/webp")
    );
    assert_eq!(Picture::sniff(b"<svg onload=x>".to_vec()), None);
    assert_eq!(Picture::sniff(vec![]), None);
}

#[test]
fn pictures_go_first_each_as_an_image_block() {
    let pics = vec![Picture::png(PNG.to_vec()), Picture::sniff(JPEG.to_vec()).unwrap()];
    let c = claude::content("what's this?".into(), &pics);
    let Json::Arr(blocks) = c else { panic!("{c:?}") };
    let kinds: Vec<_> = blocks
        .iter()
        .map(|b| b.get("type").and_then(Json::str).unwrap())
        .collect();
    assert_eq!(kinds, ["image", "image", "text"]);
    let media: Vec<_> = blocks[..2]
        .iter()
        .map(|b| {
            b.get("source")
                .and_then(|s| s.get("media_type"))
                .and_then(Json::str)
                .unwrap()
        })
        .collect();
    assert_eq!(media, ["image/png", "image/jpeg"]);
    // none: the text as it is
    assert_eq!(claude::content("hi".into(), &[]), Json::from("hi"));
}

#[test]
fn the_tab_keeps_pictures_for_the_next_message_and_refuses_what_isnt_one() {
    let mut tab = AgentTab::default();
    assert!(tab.add_picture(PNG.to_vec()));
    assert!(tab.add_picture(JPEG.to_vec()));
    assert!(!tab.add_picture(b"%PDF-1.7".to_vec()));
    assert!(
        matches!(tab.entries.last(), Some(Entry::Note { text, error: true }) if text.contains("PNG, JPEG, GIF or WebP"))
    );
    let mut big = PNG.to_vec();
    big.resize(forge_render::agent::PICTURE_BYTES + 1, 0);
    assert!(!tab.add_picture(big));
    assert!(matches!(tab.entries.last(), Some(Entry::Note { text, .. }) if text.contains("up to 5 MB")));
    assert_eq!(tab.pictures.len(), 2);
    tab.entries.clear();
    // a picture alone is a message
    tab.send();
    let Some(AgentAct::Send { pictures, text, .. }) = tab.out.pop() else {
        panic!()
    };
    assert_eq!((text.as_str(), pictures.len()), ("", 2));
    assert!(tab.pictures.is_empty(), "sent, not kept for the next");
    assert!(matches!(
        tab.entries.last(),
        Some(Entry::User { pictures: 2, .. })
    ));
    assert_eq!(forge_render::shell::agent::pictures_word(2), "2 pictures");
}

#[test]
fn attach_view_sends_the_view_as_markup_does_and_only_once() {
    let mut tab = AgentTab::default();
    tab.input = "is this right?".into();
    tab.attach_view = true;
    tab.send();
    assert!(matches!(
        tab.out.last(),
        Some(AgentAct::Send { markup: true, .. })
    ));
    assert!(!tab.attach_view);
}

/// A stand-in `claude`: each message it's sent goes to `heard.jsonl`; the turn ends at once.
fn fake(dir: &Path) -> PathBuf {
    let init = r#"{"type":"system","subtype":"init","session_id":"s","model":"m"}"#;
    let end = r#"{"type":"result","subtype":"success","session_id":"s"}"#;
    let script = dir.join("claude");
    std::fs::write(
        &script,
        format!(
            "#!/bin/sh\necho '{init}'\nwhile read line; do\n  echo \"$line\" >> '{d}/heard.jsonl'\n  echo '{end}'\ndone\n",
            d = dir.display()
        ),
    )
    .unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    script
}

#[test]
fn a_dropped_picture_reaches_claude_code_with_the_message() {
    let dir = std::env::temp_dir().join(format!("forgecad-pictures-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let dir = std::fs::canonicalize(dir).unwrap();
    std::fs::write(dir.join("plate.fg"), "param d = 12\n").unwrap();
    let Ok(mut h) = forge_render::gui::Headless::new((1400.0, 860.0), 1.0) else {
        return;
    };
    h.gui.agent.program = Some(fake(&dir).display().to_string());
    h.gui.agent.cwd = Some(dir.clone());
    h.gui.agent.settings_file = Some(dir.join("agent.toml"));
    h.step();
    AgentSettings {
        kind: Kind::ClaudeCode,
        ..Default::default()
    }
    .write_extra(&mut h.gui.shell.settings.extra);
    h.gui.open(dir.join("plate.fg").to_str().unwrap());
    h.settle();
    // not the agent's while another tab shows
    assert!(!h.gui.agent_picture(PNG.to_vec()));
    assert!(h.click("side:tab:Agent"));
    for _ in 0..3 {
        h.step();
    }
    assert_eq!(h.gui.shell.agent.tab, forge_render::shell::agent::SideTab::Agent);
    assert!(
        h.gui.agent_picture(PNG.to_vec()),
        "{:?}",
        h.gui.shell.agent.entries
    );
    assert!(h.gui.agent_picture(JPEG.to_vec()));
    h.step();
    assert!(h.gui.shell.place("agent:picture:1").is_some(), "each a chip");
    if let Ok(shots) = std::env::var("FORGE_UI_SHOTS") {
        assert!(h.click("agent:input"));
        h.text("what's wrong with this?");
        for _ in 0..6 {
            h.step();
        }
        std::fs::create_dir_all(&shots).unwrap();
        std::fs::write(
            Path::new(&shots).join("pictures-1400-dark-tray.png"),
            h.png().unwrap(),
        )
        .unwrap();
        h.gui.shell.agent.input.clear();
    }
    // its × leaves it out
    assert!(h.click("agent:picture:1:x"));
    h.step();
    assert_eq!(h.gui.shell.agent.pictures.len(), 1);
    assert!(h.click("agent:input"));
    h.text("what's wrong with this?");
    assert!(h.click("agent:send"));
    let t = std::time::Instant::now();
    let heard = loop {
        h.step();
        let s = std::fs::read_to_string(dir.join("heard.jsonl")).unwrap_or_default();
        if !s.is_empty() || t.elapsed().as_secs() > 30 {
            break s;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    };
    let msg = Json::parse(heard.lines().next().unwrap()).unwrap();
    let content = msg.get("message").and_then(|m| m.get("content")).unwrap();
    let Json::Arr(blocks) = content else {
        panic!("{content:?}")
    };
    assert_eq!(blocks[0].get("type").and_then(Json::str), Some("image"));
    assert_eq!(blocks.len(), 2, "the one picture, then the text");
    assert!(matches!(
        h.gui
            .shell
            .agent
            .entries
            .iter()
            .find(|e| matches!(e, Entry::User { .. })),
        Some(Entry::User { pictures: 1, .. })
    ));
    let _ = std::fs::remove_dir_all(dir);
}
