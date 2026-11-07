//! Project-aware agent tools use the existing history and release implementation.
use forge_agent::json::Json;
use forge_render::agent::{Event, Watch, apptools::AppTools, project};
use std::path::{Path, PathBuf};
use std::sync::Arc;

const PART: &str = "shared w = 40\nproperties(part_number: \"AG-100\")\nsketch plan(on: xy) {\n outline = rect(corner: (0, 0), size: (w, 30))\n}\nbase = extrude(plan, 10)\n";
fn scratch() -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let p = std::env::temp_dir().join(format!(
        "forgecad-agent-project-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&p).unwrap();
    p.canonicalize().unwrap()
}
fn init(root: &Path) -> forge_history::History {
    let mut h = forge_history::History::turn_on(root).unwrap();
    h.set_identity("Agent Test", "agent-test@example.com").unwrap();
    h
}
fn app(root: &Path) -> AppTools {
    let (tools, _) = AppTools::new(Arc::new(|| {}));
    tools.set_dir(root);
    tools
}
fn ask(app: &AppTools, name: &str, fields: Vec<(&str, Json)>) -> Json {
    let reply = app.call(name, Json::obj(fields));
    assert!(!reply.error, "{name}: {}", reply.text);
    Json::parse(&reply.text).unwrap()
}

#[test]
fn tools_organize_nested_sources_and_watch_them_from_the_repo_root() {
    let root = scratch();
    let _h = init(&root);
    let tools = app(&root);
    let mut watch = Watch::new(&root);
    ask(
        &tools,
        "project_mkdir",
        vec![("path", "assemblies/components".into())],
    );
    ask(
        &tools,
        "project_write",
        vec![("file", "parts/bracket.fg".into()), ("text", PART.into())],
    );
    ask(
        &tools,
        "project_write",
        vec![("file", "docs/README.md".into()), ("text", "Design notes".into())],
    );
    assert_eq!(project::root(&root.join("parts/bracket.fg")), root);
    let events = watch.check();
    assert_eq!(events.len(), 2);
    assert!(events.iter().any(
        |e| matches!(e, Event::FileChanged { path, before: None, .. } if path.ends_with("docs/README.md"))
    ));
    let info = ask(&tools, "project_info", vec![]).to_string();
    assert!(info.contains("parts/bracket.fg") && info.contains("docs/README.md"));
    let read = ask(&tools, "project_read", vec![("file", "parts/bracket.fg".into())]);
    assert_eq!(read.get("text").and_then(Json::str), Some(PART));
    let refused = tools.call(
        "project_write",
        Json::obj(vec![("file", "parts/bracket.fg".into()), ("text", "oops".into())]),
    );
    assert!(refused.error);
    ask(
        &tools,
        "project_write",
        vec![
            ("file", "parts/bracket.fg".into()),
            ("old", PART.into()),
            ("text", PART.replace("w = 40", "w = 44").into()),
        ],
    );
    let stale = tools.call(
        "project_write",
        Json::obj(vec![
            ("file", "parts/bracket.fg".into()),
            ("old", PART.into()),
            ("text", "oops".into()),
        ]),
    );
    assert!(stale.error);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn project_paths_refuse_escapes_metadata_and_links_and_watch_ignores_outputs() {
    let root = scratch();
    let h = init(&root);
    let outside = scratch();
    std::fs::write(outside.join("p.fg"), PART).unwrap();
    std::os::unix::fs::symlink(&outside, root.join("linked")).unwrap();
    let tools = app(&root);
    for path in [
        "../outside.fg",
        "/tmp/outside.fg",
        ".git/config",
        "linked/p.fg",
        "releases/a.fg",
        "exports/a.fg",
        "parts/../../a.fg",
    ] {
        assert!(
            tools
                .call(
                    "project_write",
                    Json::obj(vec![("file", path.into()), ("text", PART.into())])
                )
                .error,
            "{path}"
        );
        assert!(
            tools
                .call("fg_versions", Json::obj(vec![("file", path.into())]))
                .error,
            "{path}"
        );
    }
    std::fs::write(root.join(".gitignore"), "ignored/\n").unwrap();
    for dir in ["ignored", "releases", "exports", ".secret", "target"] {
        std::fs::create_dir(root.join(dir)).unwrap();
        std::fs::write(root.join(dir).join("p.fg"), PART).unwrap();
    }
    assert!(project::files(&root).is_empty());
    assert!(h.head().unwrap().is_none());
    std::fs::remove_dir_all(root).unwrap();
    std::fs::remove_dir_all(outside).unwrap();
}

#[test]
fn version_tools_checkpoint_all_turn_files_and_leave_other_work_untouched() {
    let root = scratch();
    let h = init(&root);
    std::fs::create_dir(root.join("parts")).unwrap();
    std::fs::write(root.join("parts/a.fg"), PART).unwrap();
    std::fs::write(root.join("parts/b.fg"), PART).unwrap();
    std::fs::write(root.join("unrelated.txt"), "baseline").unwrap();
    h.save_group(
        &[
            root.join("parts/a.fg"),
            root.join("parts/b.fg"),
            root.join("unrelated.txt"),
        ],
        "Baseline",
        None,
    )
    .unwrap();
    // Work already dirty when the turn starts must not be swept into the agent's commit.
    std::fs::write(root.join("unrelated.txt"), "other work").unwrap();
    let tools = app(&root);
    for file in ["parts/a.fg", "parts/b.fg"] {
        ask(
            &tools,
            "project_write",
            vec![
                ("file", file.into()),
                ("old", PART.into()),
                ("text", PART.replace("w = 40", "w = 44").into()),
            ],
        );
    }
    let version = ask(
        &tools,
        "fg_create_version",
        vec![("file", "parts/a.fg".into()), ("name", "Wider".into())],
    );
    assert_eq!(version.get("n").and_then(Json::num), Some(1.0));
    let head = h.head().unwrap().unwrap();
    assert_eq!(
        h.repo.file_at(&head, "unrelated.txt").unwrap().unwrap(),
        b"baseline"
    );
    assert!(
        String::from_utf8(h.repo.file_at(&head, "parts/b.fg").unwrap().unwrap())
            .unwrap()
            .contains("w = 44")
    );
    let history = ask(
        &tools,
        "fg_history",
        vec![("file", "parts/a.fg".into()), ("all", true.into())],
    )
    .to_string();
    assert!(history.contains("agent") && history.contains("parts/b.fg"));
    ask(&tools, "fg_versions", vec![("file", "parts/a.fg".into())]);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn versions_compare_restore_and_releases_use_the_shared_implementation() {
    let root = scratch();
    let _h = init(&root);
    let tools = app(&root);
    ask(
        &tools,
        "project_write",
        vec![("file", "parts/a.fg".into()), ("text", PART.into())],
    );
    ask(
        &tools,
        "fg_create_version",
        vec![("file", "parts/a.fg".into()), ("name", "Initial".into())],
    );
    ask(
        &tools,
        "project_write",
        vec![
            ("file", "parts/a.fg".into()),
            ("old", PART.into()),
            ("text", PART.replace("w = 40", "w = 44").into()),
        ],
    );
    ask(
        &tools,
        "fg_create_version",
        vec![("file", "parts/a.fg".into()), ("name", "Wider".into())],
    );
    let diff = ask(
        &tools,
        "fg_compare",
        vec![
            ("file", "parts/a.fg".into()),
            ("base", "v1".into()),
            ("target", "v2".into()),
        ],
    );
    assert!(diff.to_string().contains("44"));
    ask(
        &tools,
        "fg_restore",
        vec![("file", "parts/a.fg".into()), ("to", "v1".into())],
    );
    assert_eq!(std::fs::read_to_string(root.join("parts/a.fg")).unwrap(), PART);
    assert!(
        tools
            .call(
                "fg_release",
                Json::obj(vec![
                    ("file", "parts/a.fg".into()),
                    ("reason", "First article".into())
                ])
            )
            .error
    );
    assert!(!root.join("releases").exists());
    let rel = ask(
        &tools,
        "fg_release",
        vec![
            ("file", "parts/a.fg".into()),
            ("reason", "First article".into()),
            ("confirmed", true.into()),
        ],
    );
    assert_eq!(rel.get("revision").and_then(Json::str), Some("A"));
    assert!(root.join("releases/AG-100/A/a-A.step").exists());
    ask(&tools, "fg_releases", vec![("file", "parts/a.fg".into())]);
    let regenerated = ask(
        &tools,
        "fg_regenerate_release",
        vec![("file", "parts/a.fg".into()), ("revision", "A".into())],
    );
    assert_eq!(regenerated.get("result").and_then(Json::str), Some("identical"));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn enabling_history_is_explicit_and_schemas_reach_both_protocols() {
    let root = scratch();
    let tools = app(&root);
    assert!(tools.call("project_enable_history", Json::obj(vec![])).error);
    assert!(!root.join(".git").exists());
    ask(
        &tools,
        "project_enable_history",
        vec![
            ("confirmed", true.into()),
            ("name", "Test User".into()),
            ("email", "user@example.com".into()),
        ],
    );
    assert!(root.join(".git").exists());
    let specs = forge_render::agent::apptools::openai_specs()
        .into_iter()
        .map(|j| j.to_string())
        .collect::<String>();
    for name in [
        "project_info",
        "project_write",
        "fg_versions",
        "fg_history",
        "fg_create_version",
        "fg_compare",
        "fg_restore",
        "fg_releases",
        "fg_release",
        "fg_regenerate_release",
    ] {
        assert!(
            forge_render::agent::apptools::schemas()
                .iter()
                .any(|s| s.0 == name)
        );
        assert!(specs.contains(&format!("app_{name}")));
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn the_app_records_a_nested_multi_file_agent_turn_as_one_history_save() {
    use forge_render::shell::agent_settings::{AgentSettings, Kind};
    use std::os::unix::fs::PermissionsExt;
    let Ok(mut view) = forge_render::gui::Headless::new((1400.0, 900.0), 1.0) else {
        return;
    };
    let root = scratch();
    let history = init(&root);
    std::fs::create_dir(root.join("parts")).unwrap();
    std::fs::write(root.join("parts/a.fg"), PART).unwrap();
    std::fs::write(root.join("unrelated.txt"), "baseline").unwrap();
    history
        .save(
            &root.join("parts/a.fg"),
            &[root.join("unrelated.txt")],
            "Baseline",
            None,
        )
        .unwrap();
    std::fs::write(root.join("unrelated.txt"), "outside this turn").unwrap();
    let program = root.join("fake-codex");
    std::fs::write(&program,r#"#!/bin/sh
cat >/dev/null
mkdir -p ../docs
cp ../parts/a.fg ../parts/b.fg
printf 'Project notes' > ../docs/README.md
printf '{"type":"thread.started","thread_id":"project-thread"}\n'
printf '{"type":"item.completed","item":{"id":"m","type":"agent_message","text":"Created a part and project notes."}}\n'
printf '{"type":"turn.completed","usage":{}}\n'
"#).unwrap();
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).unwrap();
    view.gui.agent.program = Some(program.display().to_string());
    view.gui.agent.cwd = Some(root.join(".agent-run"));
    view.gui.agent.settings_file = Some(root.join("unused-agent.toml"));
    view.step();
    AgentSettings {
        kind: Kind::Codex,
        trusted: vec![root.display().to_string()],
        ..Default::default()
    }
    .write_extra(&mut view.gui.shell.settings.extra);
    view.gui.open(root.join("parts/a.fg").to_str().unwrap());
    view.settle();
    assert!(view.click("side:tab:Agent"));
    view.step();
    assert!(view.click("agent:input"));
    view.step();
    view.text("Create a second part and project notes");
    view.step();
    assert!(view.click("agent:send"));
    let started = std::time::Instant::now();
    loop {
        view.step();
        if !view.gui.shell.agent.running && !view.gui.agent.turns.done.is_empty() {
            break;
        }
        assert!(
            started.elapsed().as_secs() < 90,
            "agent turn finished: {:?}",
            view.gui.shell.agent.entries
        );
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    assert_eq!(view.gui.agent.turns.done[0].files.len(), 2);
    let log = history.log().unwrap();
    assert_eq!(log.len(), 2, "baseline and one agent save: {log:?}");
    assert!(log[0].paths.contains(&"parts/b.fg".into()) && log[0].paths.contains(&"docs/README.md".into()));
    assert_eq!(log[0].agent.as_deref(), Some("agent"));
    assert_eq!(
        history
            .repo
            .file_at(&log[0].id, "unrelated.txt")
            .unwrap()
            .unwrap(),
        b"baseline"
    );
    drop(view);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_paused_history_cannot_issue_a_release_from_stale_content() {
    let root = scratch();
    let h = init(&root);
    std::fs::write(root.join("a.fg"), PART).unwrap();
    h.save(&root.join("a.fg"), &[], "Baseline", None).unwrap();
    std::fs::write(root.join(".git/MERGE_HEAD"), h.head().unwrap().unwrap().hex()).unwrap();
    std::fs::write(root.join("a.fg"), PART.replace("w = 40", "w = 44")).unwrap();
    assert!(
        forge_lang::release::release(&root.join("a.fg"), "First article", "", false, Some("agent")).is_err()
    );
    assert!(forge_lang::release::records(&h, Some("a.fg")).unwrap().is_empty());
    assert!(!root.join("releases").exists());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
#[ignore = "uses installed Codex and the user's model access"]
fn codex_live_organizes_project_files_and_creates_a_version_through_mcp() {
    use forge_render::agent::{Backend, Out, claude::Config, codex::Codex};
    let root = scratch();
    let history = init(&root);
    std::fs::create_dir(root.join("parts")).unwrap();
    let part = root.join("parts/a.fg");
    std::fs::write(&part, PART).unwrap();
    history.save(&part, &[], "Baseline", None).unwrap();
    let (tools, _calls) = AppTools::new(Arc::new(|| {}));
    tools.set_dir(&root);
    let socket = root.join("app.sock");
    forge_render::agent::mcp::serve(&socket, tools).unwrap();
    let (tx, rx) = std::sync::mpsc::channel();
    let mut codex = Codex::new(
        Config {
            program: "codex".into(),
            model: String::new(),
            file: part.clone(),
            system: forge_render::agent::system_prompt(&part),
            cwd: root.join(".agent-run"),
            mcp: Some(forge_render::agent::mcp::config(
                Path::new(env!("CARGO_BIN_EXE_forge-app")),
                &socket,
            )),
        },
        Out {
            tx,
            wake: Arc::new(|| {}),
        },
        None,
    );
    codex.send("Use the ForgeCAD project tools to create docs/agent-check.md with the exact text `Project tools ready`. Then create a named version of parts/a.fg called Agent smoke. Do not edit the part or issue a release. Use MCP tools, no shell commands.".into(),None);
    let mut events = vec![];
    loop {
        let event = rx
            .recv_timeout(std::time::Duration::from_secs(120))
            .expect("live turn finishes");
        let done = matches!(event, Event::Done { .. } | Event::Problem { .. });
        events.push(event);
        if done {
            break;
        }
    }
    assert!(
        matches!(
            events.last(),
            Some(Event::Done {
                error: None,
                stopped: false
            })
        ),
        "{events:?}"
    );
    assert_eq!(
        std::fs::read_to_string(root.join("docs/agent-check.md"))
            .unwrap()
            .trim(),
        "Project tools ready"
    );
    let versions = history.versions(Some("parts/a.fg")).unwrap();
    assert!(
        versions
            .iter()
            .any(|v| v.name == "Agent smoke" && v.agent.as_deref() == Some("agent")),
        "{events:?}"
    );
    let at = versions.iter().find(|v| v.name == "Agent smoke").unwrap();
    assert!(
        history
            .repo
            .file_at(&at.save, "docs/agent-check.md")
            .unwrap()
            .is_some()
    );
    assert!(!root.join("releases").exists());
    drop(codex);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_version_of_a_new_nested_assembly_includes_its_preexisting_untracked_inputs() {
    let root = scratch();
    let history = init(&root);
    std::fs::create_dir(root.join("parts")).unwrap();
    std::fs::create_dir(root.join("assemblies")).unwrap();
    std::fs::write(root.join("parts/a.fg"), PART).unwrap();
    let assembly = "assembly\na = part(\"../parts/a.fg\", ground: true)\n";
    std::fs::write(root.join("assemblies/top.fg"), assembly).unwrap();
    let tools = app(&root);
    let built = tools.call("fg_run", Json::obj(vec![("file", "assemblies/top.fg".into())]));
    assert!(!built.error, "{}", built.text);
    ask(
        &tools,
        "fg_create_version",
        vec![
            ("file", "assemblies/top.fg".into()),
            ("name", "Assembly baseline".into()),
        ],
    );
    let version = history
        .versions(Some("assemblies/top.fg"))
        .unwrap()
        .pop()
        .unwrap();
    assert!(
        history
            .repo
            .file_at(&version.save, "parts/a.fg")
            .unwrap()
            .is_some()
    );
    let saved =
        forge_lang::compare::build_at(&history, "assemblies/top.fg", Some(&version.save), "v1").unwrap();
    assert!(saved.run.errors.is_empty(), "{:?}", saved.run.errors);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn an_agent_turn_refreshes_a_release_badge_even_without_file_edits() {
    use forge_render::shell::agent_settings::{AgentSettings, Kind};
    use std::os::unix::fs::PermissionsExt;
    let Ok(mut view) = forge_render::gui::Headless::new((1400.0, 900.0), 1.0) else {
        return;
    };
    let root = scratch();
    let history = init(&root);
    let part = root.join("a.fg");
    std::fs::write(&part, PART).unwrap();
    history.save(&part, &[], "Baseline", None).unwrap();
    let program = root.join("fake-codex");
    std::fs::write(
        &program,
        "#!/bin/sh\ncat >/dev/null\nsleep 0.3\nprintf '{\"type\":\"turn.completed\",\"usage\":{}}\\n'\n",
    )
    .unwrap();
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).unwrap();
    view.gui.agent.program = Some(program.display().to_string());
    view.gui.agent.cwd = Some(root.join(".agent-run"));
    view.gui.agent.settings_file = Some(root.join("unused-agent.toml"));
    view.step();
    AgentSettings {
        kind: Kind::Codex,
        trusted: vec![root.display().to_string()],
        ..Default::default()
    }
    .write_extra(&mut view.gui.shell.settings.extra);
    view.gui.open(part.to_str().unwrap());
    view.settle();
    assert!(view.gui.shell.versions.revision.is_empty());
    assert!(view.click("side:tab:Agent"));
    view.step();
    assert!(view.click("agent:input"));
    view.step();
    view.text("Read the history");
    view.step();
    assert!(view.click("agent:send"));
    view.step();
    // A metadata-only release, as a version tool makes, must be noticed when the turn ends.
    forge_lang::release::release(&part, "Test article", "", false, Some("agent")).unwrap();
    let started = std::time::Instant::now();
    while view.gui.shell.agent.running {
        assert!(started.elapsed().as_secs() < 30);
        view.step();
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    view.step();
    view.step();
    assert_eq!(view.gui.shell.versions.revision, "A");
    assert!(
        view.gui.agent.turns.done.is_empty(),
        "metadata-only turn has no file diff"
    );
    drop(view);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn empty_projects_can_chat_then_create_the_first_part_without_resetting_the_conversation() {
    use forge_render::shell::{
        agent::Entry,
        agent_settings::{AgentSettings, Kind},
    };
    use std::os::unix::fs::PermissionsExt;
    for with_git in [false, true] {
        let Ok(mut view) = forge_render::gui::Headless::new((1400.0, 900.0), 1.0) else {
            return;
        };
        let root = scratch();
        let _history = with_git.then(|| init(&root));
        let program = root.join("fake-codex");
        let script = format!(
            r#"#!/bin/sh
cat > prompt.txt
printf '%s\n' "$@" >> arguments.txt
printf '{{"type":"thread.started","thread_id":"project-chat"}}\n'
if [ -f chatted ]; then
    mkdir -p ../parts
    cat > ../parts/first.fg <<'PART'
{PART}
PART
else
    touch chatted
fi
printf '{{"type":"item.completed","item":{{"id":"m","type":"agent_message","text":"We can work on this project."}}}}\n'
printf '{{"type":"turn.completed","usage":{{}}}}\n'
"#
        );
        std::fs::write(&program, script).unwrap();
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).unwrap();
        view.gui.agent.project_dir = Some(root.clone());
        view.gui.agent.program = Some(program.display().to_string());
        view.gui.agent.cwd = Some(root.join(".agent-run"));
        view.gui.agent.settings_file = Some(root.join("unused-agent.toml"));
        view.step();
        AgentSettings {
            kind: Kind::Codex,
            trusted: vec![root.display().to_string()],
            ..Default::default()
        }
        .write_extra(&mut view.gui.shell.settings.extra);
        view.step();
        assert!(view.gui.agent_path().is_none());
        assert_eq!(view.gui.shell.agent.project.as_deref(), root.to_str());
        assert!(view.click("side:tab:Agent"));
        view.step();
        for (turn, message) in [
            "Let's discuss the telescope design",
            "Create the first part in parts/first.fg",
            "Describe the project folder",
        ]
        .into_iter()
        .enumerate()
        {
            assert!(view.click("agent:input"));
            view.step();
            view.text(message);
            view.step();
            assert_eq!(
                view.gui.shell.agent.input, message,
                "input enabled without a part"
            );
            assert!(view.click("agent:send"));
            let started = std::time::Instant::now();
            loop {
                view.step();
                let replies = view
                    .gui
                    .shell
                    .agent
                    .entries
                    .iter()
                    .filter(|e| matches!(e, Entry::Agent { .. }))
                    .count();
                if !view.gui.shell.agent.running && replies > turn {
                    break;
                }
                assert!(
                    started.elapsed().as_secs() < 60,
                    "turn {turn} failed: {:?}",
                    view.gui.shell.agent.entries
                );
                std::thread::sleep(std::time::Duration::from_millis(2));
            }
            assert!(view.gui.shell.agent.problem.is_none());
            if turn == 0 {
                assert!(
                    view.gui.agent_path().is_none(),
                    "chat doesn't fabricate a document"
                );
                assert!(
                    !project::files(&root)
                        .iter()
                        .any(|p| p.extension().is_some_and(|e| e == "fg"))
                );
                assert!(view.gui.agent.turns.done.is_empty());
            } else {
                assert_eq!(
                    view.gui.agent_path().as_deref(),
                    root.join("parts/first.fg").to_str()
                );
            }
            assert_eq!(view.gui.shell.agent.project.as_deref(), root.to_str());
        }
        let arguments = std::fs::read_to_string(root.join(".agent-run/arguments.txt")).unwrap();
        assert_eq!(arguments.lines().filter(|s| *s == "resume").count(), 2);
        assert_eq!(
            arguments.lines().filter(|s| *s == root.to_str().unwrap()).count(),
            3,
            "project boundary stays at the launch folder"
        );
        assert!(
            !view
                .gui
                .shell
                .agent
                .entries
                .iter()
                .any(|e| matches!(e,Entry::Note{text,..} if text.contains("new conversation")))
        );
        drop(view);
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn directory_context_uses_the_directory_and_a_project_prompt_without_a_fake_document() {
    let root = scratch();
    assert_eq!(project::root(&root), root);
    let prompt = forge_render::agent::system_prompt(&root);
    assert!(prompt.contains("No saved CAD document is open"));
    assert!(prompt.contains("Do not require the user to open or save a part"));
    assert!(!prompt.contains("The open file is"));
    assert!(project::files(&root).is_empty());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
#[ignore = "uses installed Codex and the user's model access"]
fn codex_live_can_discuss_an_empty_project_in_the_app_without_an_open_part() {
    use forge_render::shell::{
        agent::Entry,
        agent_settings::{AgentSettings, Kind},
    };
    let Ok(mut view) = forge_render::gui::Headless::new((1400.0, 900.0), 1.0) else {
        return;
    };
    let root = scratch();
    let _history = init(&root);
    view.gui.agent.project_dir = Some(root.clone());
    view.gui.agent.cwd = Some(root.join(".agent-run"));
    view.gui.agent.settings_file = Some(root.join("unused-agent.toml"));
    view.gui.agent.mcp_exe = Some(PathBuf::from(env!("CARGO_BIN_EXE_forge-app")));
    view.step();
    AgentSettings {
        kind: Kind::Codex,
        trusted: vec![root.display().to_string()],
        ..Default::default()
    }
    .write_extra(&mut view.gui.shell.settings.extra);
    view.step();
    assert!(view.gui.agent_path().is_none());
    assert!(view.click("side:tab:Agent"));
    view.step();
    assert!(view.click("agent:input"));
    view.step();
    view.text("This is an empty telescope project. Call project_info to inspect the project, then suggest a first mechanical design step in one sentence. Discuss only; do not create or edit any files.");
    view.step();
    assert!(view.click("agent:send"));
    let started = std::time::Instant::now();
    loop {
        view.step();
        if !view.gui.shell.agent.running {
            break;
        }
        assert!(
            started.elapsed().as_secs() < 120,
            "live chat completes: {:?}",
            view.gui.shell.agent.entries
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(
        view.gui.shell.agent.problem.is_none(),
        "{:?}",
        view.gui.shell.agent.problem
    );
    assert!(
        view.gui
            .shell
            .agent
            .entries
            .iter()
            .any(|e| matches!(e,Entry::Agent{text} if !text.trim().is_empty())),
        "{:?}",
        view.gui.shell.agent.entries
    );
    assert!(
        view.gui
            .shell
            .agent
            .entries
            .iter()
            .any(|e| matches!(e,Entry::Tool{name,..} if name.contains("project_info"))),
        "{:?}",
        view.gui.shell.agent.entries
    );
    assert!(
        !view
            .gui
            .shell
            .agent
            .entries
            .iter()
            .any(|e| matches!(e, Entry::Note { error: true, .. })),
        "{:?}",
        view.gui.shell.agent.entries
    );
    assert!(view.gui.agent_path().is_none());
    assert!(
        project::files(&root).is_empty(),
        "discussion mustn't fabricate a part"
    );
    drop(view);
    std::fs::remove_dir_all(root).unwrap();
}
