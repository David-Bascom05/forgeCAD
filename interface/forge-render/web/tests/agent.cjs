// The page's agent: the person's own Claude Code through `forgecad web DIR --agent`
// (research/agent-tab/bridge-threat-model.md), here a stand-in `claude` (FORGECAD_CLAUDE) that
// asks to edit the part, edits it if allowed, and replies with markup that must stay text.
// node web/tests/agent.cjs OUT (from interface/forge-render; OUT is web/build.sh's).
// FORGE_WEB_SHOTS=DIR keeps the page's pictures.
const t = require("./lib.cjs");
const assert = require("node:assert");
const fs = require("node:fs");
const path = require("node:path");

const app = process.argv[2];
const PART = `shared L = 60
shared W = 30
param extrude1.d = 12
sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (L, W))
}
extrude1 = extrude(plan, d)
`;
const INJECT = `Done. <script>window.pwned=1</script> <img src=x onerror="window.pwned=2">`;

/// The stand-in: for each message, an Edit asked about, made if allowed, then a reply.
function fake(dir) {
  const tool = JSON.stringify({ type: "assistant", message: { id: "m-1", content: [{ type: "tool_use", id: "t-1", name: "Edit", input: { file_path: "plate.fg", old_string: "= 12", new_string: "= 14" } }] }, parent_tool_use_id: null });
  const ask = JSON.stringify({ type: "control_request", request_id: "req-1", request: { subtype: "can_use_tool", tool_name: "Edit", input: { file_path: "plate.fg", old_string: "= 12", new_string: "= 14" }, permission_suggestions: [{ type: "setMode", mode: "acceptEdits", destination: "session" }], tool_use_id: "t-1" } });
  const res = JSON.stringify({ type: "user", message: { content: [{ type: "tool_result", tool_use_id: "t-1", content: "ok" }] }, parent_tool_use_id: null });
  const said = JSON.stringify({ type: "assistant", message: { id: "m-2", content: [{ type: "text", text: INJECT }] }, parent_tool_use_id: null });
  const end = JSON.stringify({ type: "result", subtype: "success", session_id: "s" });
  const q = (s) => `'${s.replace(/'/g, `'\\''`)}'`;
  const script = path.join(dir, "claude");
  fs.writeFileSync(script, `#!/bin/sh
echo '{"type":"system","subtype":"init","session_id":"s","model":"m","permissionMode":"default"}'
while read line; do
  case "$line" in *'"type":"user"'*) ;; *) continue;; esac
  echo ${q(tool)}
  echo ${q(ask)}
  read reply
  case "$reply" in *'"allow"'*) sed -i 's/= 12/= 14/' plate.fg;; esac
  echo ${q(res)}
  echo ${q(said)}
  echo ${q(end)}
done
`);
  fs.chmodSync(script, 0o755);
  return script;
}

/// The frame an ask draws (the page is idle after it: no frames come unasked).
async function settle(p) {
  const f0 = (await p.evaluate(() => window.forgecad.state())).frames;
  const t0 = Date.now();
  while (Date.now() - t0 < 5000) {
    if ((await p.evaluate(() => window.forgecad.state())).frames > f0) return;
    await new Promise((r) => setTimeout(r, 50));
  }
}
async function click(p, name) {
  await t.until(p, (s) => s.places?.some((x) => x.name === name), `\`${name}\` on screen`, 60000);
  await p.evaluate((n) => window.forgecad.fc.app_click(n, 0, false), name);
  await settle(p);
}
const agentState = (s) => s.agent ?? { entries: [] };

(async () => {
  const root = t.tmpdir("agent");
  const dir = path.join(root, "proj");
  fs.mkdirSync(dir, { recursive: true });
  fs.writeFileSync(path.join(dir, "plate.fg"), PART);
  process.env.FORGECAD_CLAUDE = fake(root);
  const { url, proc } = await t.serve(dir, app, ["--agent"]);
  const [base, frag] = url.split("#");
  assert.match(frag ?? "", /^agent=[0-9a-f]{32}$/, "the agent's token in the fragment");
  const b = await t.browser();
  try {
    const p = await t.page(b, `${base}?open=plate.fg&offscreen=1280x800#${frag}`);
    // every request the page makes: to its own server only
    const origin = new URL(base).origin;
    const elsewhere = [];
    p.on("request", (r) => {
      if (!r.url().startsWith(origin) && !r.url().startsWith("data:") && !r.url().startsWith("blob:")) elsewhere.push(r.url());
    });
    const csp = [];
    p.on("console", (m) => /Content Security Policy/.test(m.text()) && csp.push(m.text()));
    await p.waitForFunction(() => window.forgecad?.loaded, null, { timeout: 600000 });
    await t.loaded(p);
    // the token: off the address bar, in no storage
    const kept = await p.evaluate(() => ({
      hash: location.hash,
      local: JSON.stringify({ ...localStorage }),
      session: JSON.stringify({ ...sessionStorage }),
    }));
    assert.equal(kept.hash, "", "the token is taken off the address bar");
    assert.ok(!kept.local.includes(frag.slice(6)) && !kept.session.includes(frag.slice(6)), "and kept in no storage");
    console.log("ok - the agent's token is held in memory only");

    await click(p, "side:tab:Agent");
    await click(p, "agent:input");
    await p.evaluate(() => window.forgecad.fc.app_type("make the plate thicker", true));
    await settle(p);
    let st = await t.until(p, (s) => agentState(s).entries.some((e) => e.kind === "permission" && e.state === "Waiting"), "the question");
    const tool = agentState(st).entries.find((e) => e.kind === "tool");
    assert.ok(tool, "the Edit's row");
    if (process.env.FORGE_WEB_SHOTS) t.shot("agent-web-waiting.png", await t.pixels(p));
    console.log("ok - Claude Code's question is a card in the page");

    await click(p, "agent:perm:req-1:allow");
    st = await t.until(p, (s) => !agentState(s).running && agentState(s).entries.some((e) => e.kind === "diff"), "the turn's diff");
    assert.match(fs.readFileSync(path.join(dir, "plate.fg"), "utf8"), /extrude1\.d = 14/, "edited on the machine");
    st = await t.until(p, (s) => s.done >= s.seq && s.built?.checked, "the page's part built again");
    const reply = agentState(st).entries.find((e) => e.kind === "agent");
    assert.equal(reply?.text, INJECT, "the reply's markup is text, as it came");
    const pwned = await p.evaluate(() => window.pwned);
    assert.equal(pwned, undefined, "no script ran from the agent's reply");
    if (process.env.FORGE_WEB_SHOTS) t.shot("agent-web-after-turn.png", await t.pixels(p));
    console.log("ok - allowed: the edit made on the machine is a diff in the page; its markup stayed text");

    // the turn's undo, saved back to the machine (the page took the agent's version: no StaleBase)
    const diff = agentState(st).entries.find((e) => e.kind === "diff");
    await click(p, `agent:undo:${diff.turn}:${diff.index}`);
    await t.until(p, (s) => agentState(s).entries.some((e) => e.kind === "diff" && e.undone), "the undo");
    await p.evaluate(() => window.forgecad.fc.app_save());
    await p.evaluate(() => window.forgecad.settled());
    const problems = await p.evaluate(() => window.forgecad.problems);
    assert.deepEqual(problems, [], "the save went through");
    assert.match(fs.readFileSync(path.join(dir, "plate.fg"), "utf8"), /extrude1\.d = 12/, "undone on the machine");
    console.log("ok - Undo takes the agent's change back, and saves to the machine");

    assert.deepEqual(elsewhere, [], "no request left for another host");
    assert.deepEqual(csp, [], "nothing the page needs is refused by its CSP");
    t.check(p);
    console.log("ok - the page asked nothing of any host but its own; its CSP refused nothing it needs");
  } finally {
    await b.close();
    proc.kill();
  }
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
