//! An agent's tool calls run forge-lang on a thread that lives as long as the conversation (each
//! MCP connection's own, for Claude Code and Codex). forge-lang keeps every sketch a run evaluates on
//! its thread until the host frees them, so each call kept a few KB for the life of the process.
//! The calls free them as they end: many calls hold no more than one.

use std::sync::Arc;

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

#[test]
fn an_agents_tool_calls_keep_no_sketches_on_their_thread() {
    let dir = std::env::temp_dir().join(format!("forgecad-agent-kept-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("p.fg"), PART).unwrap();
    let (tools, _rx) = forge_render::agent::apptools::AppTools::new(Arc::new(|| {}));
    tools.set_dir(&dir);
    // one thread for every call, as an MCP connection's is
    let kept = std::thread::spawn(move || {
        let args = forge_agent::json::Json::parse(r#"{"file":"p.fg"}"#).unwrap();
        let mut kept = vec![];
        for _ in 0..20 {
            for name in ["fg_run", "fg_check"] {
                let r = tools.call(name, args.clone());
                assert!(!r.error, "{name}: {}", r.text);
            }
            kept.push(forge_keep::kept());
        }
        kept
    })
    .join()
    .unwrap();
    assert!(
        kept.iter().all(|k| *k == 0),
        "sketches kept on the tool thread after each call: {kept:?}"
    );
    let _ = std::fs::remove_dir_all(dir);
}
