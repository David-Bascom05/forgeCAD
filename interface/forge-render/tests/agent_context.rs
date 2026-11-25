//! What goes with a message to the agent: the selection, offered as a chip until taken off; what
//! `@` names (the part's features and params, the project's files), picked from a list; and the
//! open file's build state.
// a fake agent CLI as a shell script: Unix only
#![cfg(unix)]

use forge_render::shell::agent::{
    AgentAct, AgentTab, Candidate, MENTION_ROWS, Mention, matches, mention_word,
};
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
    let d = std::env::temp_dir().join(format!("forgecad-context-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn c(name: &str) -> Candidate {
    Candidate {
        name: name.into(),
        what: String::new(),
        ..Default::default()
    }
}

#[test]
fn the_word_after_at_is_what_the_list_matches() {
    assert_eq!(mention_word("thicken @ext"), Some("ext"));
    assert_eq!(mention_word("@"), Some(""));
    assert_eq!(mention_word("see @parts/pla"), Some("parts/pla"));
    assert_eq!(
        mention_word("mail a@b"),
        None,
        "an @ inside a word isn't a mention"
    );
    assert_eq!(mention_word("@ext "), None, "a space ends it");
    assert_eq!(mention_word("why?@x"), None);
    let all: Vec<Candidate> = ["sk1", "extrude1", "sk1.x1", "base_ext", "parts/plate.fg"]
        .into_iter()
        .map(c)
        .collect();
    let names = |v: Vec<Candidate>| v.into_iter().map(|c| c.name).collect::<Vec<_>>();
    // starting with it first, then holding it, each in order; case aside
    assert_eq!(names(matches(&all, "EXT")), ["extrude1", "base_ext"]);
    assert_eq!(names(matches(&all, "sk1")), ["sk1", "sk1.x1"]);
    assert_eq!(matches(&all, "").len(), all.len().min(MENTION_ROWS));
    let many: Vec<Candidate> = (0..40).map(|k| c(&format!("f{k}"))).collect();
    assert_eq!(matches(&many, "f").len(), MENTION_ROWS);
}

#[test]
fn the_selection_goes_with_a_message_until_its_chip_is_taken_off() {
    let mut tab = AgentTab::default();
    tab.selection = Some("extrude1".into());
    assert_eq!(tab.suggested(), Some("extrude1"));
    tab.input = "make it thicker".into();
    tab.send();
    assert!(matches!(&tab.out[0], AgentAct::Send { refs, .. } if refs == &["extrude1".to_string()]));
    // taken off: not offered again for that selection
    tab.running = false;
    tab.dismissed = Some("extrude1".into());
    assert_eq!(tab.suggested(), None);
    tab.input = "and the other".into();
    tab.send();
    assert!(matches!(&tab.out[1], AgentAct::Send { refs, .. } if refs.is_empty()));
    // something else selected: offered
    tab.selection = Some("sk1".into());
    assert_eq!(tab.suggested(), Some("sk1"));
    // already attached by @: not twice
    tab.refs = vec!["sk1".into()];
    assert_eq!(tab.suggested(), None);
}

#[test]
fn a_pick_replaces_the_word_typed() {
    let mut tab = AgentTab::default();
    tab.input = "fillet @ext".into();
    tab.mention = Some(Mention {
        sigil: '@',
        word: "ext".into(),
        items: vec![c("extrude1"), c("base_ext")],
        at: 1,
        all: vec![],
    });
    assert!(tab.take_mention());
    assert_eq!(tab.input, "fillet @base_ext ");
    assert!(tab.mention.is_none());
    assert!(!tab.take_mention());
}

#[test]
fn a_part_note_says_how_it_builds() {
    let mut fg = forge_render::shell::fg::FgPanel {
        file: "/p/plate.fg".into(),
        ..Default::default()
    };
    assert_eq!(
        forge_render::gui::gui_agent::build_note(&fg).as_deref(),
        Some("plate.fg builds now with no errors or warnings")
    );
    fg.issues = (0..5)
        .map(|k| forge_render::shell::fg::Issue {
            severity: if k == 0 {
                forge_render::shell::fg::Severity::Error
            } else {
                forge_render::shell::fg::Severity::Warning
            },
            kind: "x".into(),
            text: format!("problem {k}\nmore"),
            line: k + 1,
            col: 0,
            fixes: vec![],
            feature: None,
            asm: None,
            detail: None,
        })
        .collect();
    let note = forge_render::gui::gui_agent::build_note(&fg).unwrap();
    assert_eq!(
        note,
        "plate.fg now has 1 error, 4 warnings: line 1: problem 0; line 2: problem 1; line 3: problem 2; and 2 more"
    );
}

/// A stand-in `claude` that keeps the first message it's sent and ends the turn.
fn fake_claude(dir: &Path) -> PathBuf {
    let script = dir.join("claude");
    std::fs::write(
        &script,
        format!(
            "#!/bin/sh\nread line\necho \"$line\" > '{}/said.json'\necho '{{\"type\":\"system\",\"subtype\":\"init\",\"session_id\":\"s\",\"model\":\"m\"}}'\necho '{{\"type\":\"result\",\"subtype\":\"success\",\"session_id\":\"s\"}}'\nwhile read line; do :; done\n",
            dir.display()
        ),
    )
    .unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    script
}

#[test]
fn in_the_app_at_completes_and_the_selection_and_build_go_with_the_message() {
    let Ok(mut h) = forge_render::gui::Headless::new((1400.0, 860.0), 1.0) else {
        return;
    };
    let dir = scratch("app");
    std::fs::write(dir.join("plate.fg"), PART).unwrap();
    std::fs::create_dir_all(dir.join("parts")).unwrap();
    std::fs::write(dir.join("parts/bracket.fg"), PART).unwrap();
    h.gui.agent.program = Some(fake_claude(&dir).display().to_string());
    h.gui.agent.cwd = Some(dir.join("cwd"));
    h.gui.agent.settings_file = Some(dir.join("agent.toml"));
    h.gui.agent.project_dir = Some(dir.clone());
    h.step();
    h.gui.open(dir.join("plate.fg").to_str().unwrap());
    h.settle();
    assert!(h.click("side:tab:Agent"));
    h.gui.shell.fg.as_mut().unwrap().selected = Some("extrude1".into());
    h.step();
    h.step();
    assert!(
        h.gui.shell.place("agent:offered").is_some(),
        "the selection is offered"
    );
    assert!(h.click("agent:input"));
    h.text("make @ext");
    h.step();
    h.step();
    assert!(h.gui.shell.place("agent:mention:extrude1").is_some());
    h.key(egui::Key::Enter);
    h.step();
    assert_eq!(h.gui.shell.agent.input, "make @extrude1 ");
    // a file, by its path in the project
    h.text("like @brack");
    h.step();
    h.step();
    assert!(h.gui.shell.place("agent:mention:parts/bracket.fg").is_some());
    h.key(egui::Key::Tab);
    h.step();
    assert_eq!(h.gui.shell.agent.input, "make @extrude1 like @parts/bracket.fg ");
    // Esc closes the list and leaves the text
    h.text("@sk");
    h.step();
    h.step();
    assert!(
        h.gui
            .shell
            .agent
            .mention
            .as_ref()
            .is_some_and(|m| !m.items.is_empty())
    );
    h.key(egui::Key::Escape);
    h.step();
    assert!(h.gui.shell.agent.mention.is_none());
    assert!(h.gui.shell.agent.input.ends_with("@sk"));
    h.step();
    assert!(
        h.gui.shell.agent.mention.is_none(),
        "stays closed until the text changes"
    );
    h.text(" thicker");
    h.key(egui::Key::Enter);
    let t = std::time::Instant::now();
    while t.elapsed().as_secs() < 60 && !dir.join("said.json").exists() {
        h.step();
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let said = std::fs::read_to_string(dir.join("said.json")).unwrap();
    let j = forge_agent::json::Json::parse(said.trim()).unwrap();
    let text = j
        .get("message")
        .and_then(|m| m.get("content"))
        .and_then(forge_agent::json::Json::str)
        .unwrap()
        .to_string();
    assert!(
        text.starts_with("make @extrude1 like @parts/bracket.fg @sk thicker"),
        "{text}"
    );
    assert!(text.contains("(Selected in the app: `extrude1`)"), "{text}");
    assert!(
        text.contains("(plate.fg builds now with no errors or warnings)"),
        "{text}"
    );
    let _ = std::fs::remove_dir_all(dir);
}

/// UX review pictures: the selection offered on the box, and the `@` list open, at 1400×860 and
/// 480 px, dark and light, over a long chat. `FORGE_UI_SHOTS=DIR` writes them.
#[test]
fn context_pictures() {
    use forge_render::shell::agent::{Entry, ToolState};
    for (w, tag) in [(1400.0, "1400"), (480.0, "480")] {
        for dark in [true, false] {
            let Ok(mut h) = forge_render::gui::Headless::new((w, 860.0), 1.0) else {
                return;
            };
            let dir = scratch(&format!("shots-{tag}-{dark}"));
            std::fs::write(dir.join("plate.fg"), PART).unwrap();
            std::fs::create_dir_all(dir.join("parts")).unwrap();
            std::fs::write(dir.join("parts/bracket.fg"), PART).unwrap();
            h.gui.agent.program = Some(fake_claude(&dir).display().to_string());
            h.gui.agent.cwd = Some(dir.join("cwd"));
            h.gui.agent.settings_file = Some(dir.join("agent.toml"));
            h.gui.agent.project_dir = Some(dir.clone());
            let mut s = h.gui.shell.settings.clone();
            s.dark = dark;
            h.gui.shell.use_settings(s);
            h.step();
            h.gui.open(dir.join("plate.fg").to_str().unwrap());
            h.settle();
            assert!(h.click("side:tab:Agent"));
            let mut long = Vec::new();
            for k in 0..12 {
                long.push(Entry::User {
                    text: format!("Round {k}: fillet the top edges 2 mm and check the wall"),
                    refs: vec!["extrude1".into()],
                    markup: None,
                    pictures: 0,
                });
                long.push(Entry::Tool {
                    id: format!("old{k}"),
                    name: "mcp__forgecad__fg_run".into(),
                    summary: "Built plate.fg".into(),
                    input: String::new(),
                    output: "0 errors, 0 warnings".into(),
                    state: ToolState::Done,
                });
                long.push(Entry::Agent {
                    text: "Added `fillet1` (2 mm) on the top edges; the part builds, 0 errors.".into(),
                });
            }
            h.gui.shell.agent.restore_entries(long);
            // a face picked in the view, by a click on the part's top (the pick path, so the view
            // shows it picked); a selected feature at 480 px trips fgdim.rs's clamp on main
            let top = if tag == "1400" {
                (835.0, 450.0)
            } else {
                (378.0, 478.0)
            };
            h.click_at(top.0, top.1);
            for _ in 0..4 {
                h.step();
            }
            let picked = h.gui.shell.agent_selection();
            assert!(
                picked.as_deref().is_some_and(|p| p.starts_with("extrude1")),
                "{picked:?}"
            );
            let theme = if dark { "dark" } else { "light" };
            assert!(h.click("agent:input"));
            h.text("thicken ");
            shot(&mut h, &format!("context-{tag}-{theme}-selection-offered"));
            // the pointer on the chip: the view lights what it names
            let chip = h.gui.shell.place("agent:offered").unwrap();
            h.move_to(chip.center().x, chip.center().y);
            for _ in 0..4 {
                h.step();
            }
            assert_eq!(h.gui.shell.agent.hover_ref, picked);
            assert!(!h.gui.fg_lit_faces().is_empty(), "the chip's faces lit");
            shot(&mut h, &format!("context-{tag}-{theme}-chip-hover"));
            assert!(h.click("agent:input"));
            h.text("@e");
            shot(&mut h, &format!("context-{tag}-{theme}-at-list"));
            drop(h);
            let _ = std::fs::remove_dir_all(dir);
        }
    }
}

fn shot(h: &mut forge_render::gui::Headless, name: &str) {
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

#[test]
fn a_chip_says_in_words_what_it_names() {
    use forge_render::shell::agent::describe;
    assert_eq!(describe("extrude1.end"), "End face of extrude1");
    assert_eq!(describe("cover.body.start"), "Start face of cover.body");
    assert_eq!(describe("extrude1"), "extrude1");
    assert_eq!(describe("rack/ear.weird"), "rack/ear.weird");
}

#[test]
fn several_picked_go_as_one_chip_and_all_of_them_are_attached() {
    let mut tab = forge_render::shell::agent::AgentTab::default();
    tab.selection = Some("extrude1.side".into());
    tab.selection_all = vec!["extrude1.end".into(), "extrude1.side".into()];
    assert_eq!(tab.suggested_all(), ["extrude1.end", "extrude1.side"]);
    tab.input = "fillet these".into();
    tab.send();
    assert!(
        matches!(&tab.out[0], AgentAct::Send { refs, .. } if refs == &["extrude1.end".to_string(), "extrude1.side".to_string()])
    );
    // one picked: itself
    tab.selection_all.clear();
    tab.selection = Some("sk1".into());
    assert_eq!(tab.suggested_all(), ["sk1"]);
}

#[test]
fn shift_clicked_faces_are_offered_together() {
    let dir = std::env::temp_dir().join(format!("forgecad-multi-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("plate.fg"), PART).unwrap();
    let Ok(mut h) = forge_render::gui::Headless::new((1400.0, 860.0), 1.0) else {
        return;
    };
    h.step();
    h.gui.open(dir.join("plate.fg").to_str().unwrap());
    h.settle();
    assert!(h.click("side:tab:Agent"));
    h.click_at(835.0, 450.0);
    for _ in 0..4 {
        h.step();
    }
    h.shift(true);
    h.click_at(760.0, 560.0);
    h.shift(false);
    for _ in 0..4 {
        h.step();
    }
    let all = h.gui.shell.agent.selection_all.clone();
    assert_eq!(all.len(), 2, "{all:?}");
    assert!(all.iter().all(|n| n.contains("extrude1")), "{all:?}");
    assert!(h.gui.shell.place("agent:offered").is_some());
    if let Ok(shots) = std::env::var("FORGE_UI_SHOTS") {
        let chip = h.gui.shell.place("agent:offered").unwrap().center();
        h.move_to(chip.x, chip.y);
        for _ in 0..8 {
            h.step();
        }
        std::fs::create_dir_all(&shots).unwrap();
        std::fs::write(
            std::path::Path::new(&shots).join("context-1400-dark-multi-offered.png"),
            h.png().unwrap(),
        )
        .unwrap();
    }
    let _ = std::fs::remove_dir_all(dir);
}
