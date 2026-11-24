// Paths that reached std calls which panic in a browser (`SystemTime::now`: "time not
// implemented on this platform", then `unreachable` and a dead page), clicked as a person would:
//   - Versions > Turn on history (forge-git's temp file names read the time of day)
//   - Agent > Send (a new conversation's id read the time of day)
// and a lock taken twice on the page's one thread ("cannot recursively acquire mutex"): a job
// started while its caller held the lock the job takes, which natively runs on another thread
// but in the single-threaded page runs before `plat::spawn` returns:
//   - Thermoform > Check Forming (the thickness map's job, thermo_view.rs `want_heat`)
//   node web/tests/nopanic.cjs OUT   (OUT: web/build.sh's output)
// The page is 1280x800 (offscreen), so the chrome is where these coordinates say.
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const t = require("./lib.cjs");

const PART = `sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (10, 10))
}
base = extrude(plan, 5)
`;
const at = {
  agentTab: [91, 190],
  versionsTab: [157, 190],
  turnOnHistory: [57, 250],
  agentBox: [130, 690],
  agentSend: [229, 724],
  // with a thermoformed part open
  thermoformTab: [465, 55],
  checkForming: [127, 110],
};
const pause = (ms) => new Promise((r) => setTimeout(r, ms));

(async () => {
  const app = path.resolve(process.argv[2] ?? "web/out");
  const dir = t.tmpdir("nopanic");
  fs.writeFileSync(path.join(dir, "a.fg"), PART);
  // the thermoform example, with what it reads beside it
  fs.cpSync(path.join(t.repo, "examples/thermoform"), path.join(dir, "thermoform"), { recursive: true });
  const { url, proc } = await t.serve(dir, app);
  const b = await t.browser();
  try {
    const p = await t.page(b, `${url}?open=a.fg&offscreen=1280x800`);
    await t.until(p, (s) => s.built?.checked, "the first build");
    await t.loaded(p);
    const click = async (xy) => {
      await p.mouse.click(...xy);
      await pause(800);
      t.check(p);
    };

    await click(at.versionsTab);
    await click(at.turnOnHistory);
    await t.until(p, (s) => /History is on/.test(s.status), "history turned on", 30000);
    console.log("ok - Versions: history turned on");

    await click(at.agentTab);
    await click(at.agentBox);
    await p.keyboard.type("make it taller");
    await click(at.agentSend);
    await pause(2000);
    t.check(p);
    console.log("ok - Agent: a message sent without a panic");

    await p.evaluate(() => window.forgecad.fc.app_open("/project/thermoform/cover.fg"));
    await t.until(p, (s) => /cover\.fg$/.test(s.path ?? "") && s.done >= s.seq && s.built?.checked, "cover.fg built", 120000);
    await pause(2000);
    await click(at.thermoformTab);
    await click(at.checkForming);
    await pause(8000);
    t.check(p);
    console.log("ok - Thermoform: Check Forming without a panic");
  } finally {
    await b.close();
    proc.kill();
    fs.rmSync(dir, { recursive: true, force: true });
  }
})().catch((e) => {
  console.error(`not ok - ${e.stack ?? e}`);
  process.exit(1);
});
