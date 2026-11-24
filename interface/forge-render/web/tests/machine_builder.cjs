// The Machine Builder in the browser (research/cam/machine-builder.md §4): opened over a part
// kept by `forgecad web`, a trunnion made from its template, jogged into its head/cradle
// contact (drawn red), checked, and saved to the project's cam/machines/, where the machine's
// own files (and a native `forgecad machine check`) see it.
//   node web/tests/machine_builder.cjs OUT   (OUT: web/build.sh's output)
//   FORGE_WEB_SHOTS=DIR keeps the pictures.
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const { execFileSync } = require("node:child_process");
const t = require("./lib.cjs");

const PART = `shared L = 60
shared W = 30
shared t = 6
sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (L, W))
}
base = extrude(plan, t)
`;

async function machine(p, cmd) {
  await p.evaluate((c) => window.forgecad.fc.app_machine(c), cmd);
  const st = await t.until(p, (s) => (s.status ?? "").startsWith(`machine: [${cmd}] `), `machine ${cmd}`, 120000);
  // drawn after the answer
  await Promise.race([t.frames(p, st, 2).catch(() => null), new Promise((r) => setTimeout(r, 3000))]);
  return st.status.slice(`machine: [${cmd}] `.length);
}

(async () => {
  const app = path.resolve(process.argv[2] ?? "web/out");
  const dir = t.tmpdir("machine-builder");
  fs.writeFileSync(path.join(dir, "plate.fg"), PART);
  const { url, proc } = await t.serve(dir, app);
  const b = await t.browser();
  try {
    const p = await t.page(b, `${url}?open=plate.fg&offscreen=1280x800`);
    await t.until(p, (s) => s.built?.checked, "the first build");
    await t.loaded(p);

    console.log(`ok - the part (${t.shot("web-00-part.png", await t.pixels(p))})`);
    let s = await machine(p, "open");
    assert.match(s, /^generic-vmc3/, s);
    let f = await t.pixels(p);
    console.log(`ok - opened on generic-vmc3 (${t.shot("web-01-opened.png", f)})`);

    s = await machine(p, "template trunnion web-trunnion");
    assert.match(s, /^web-trunnion \| New `web-trunnion`/, s);
    assert.doesNotMatch(s, /contact/, s);
    await machine(p, "tab joints");
    f = await t.pixels(p);
    console.log(`ok - a trunnion from its template (${t.shot("web-02-trunnion.png", f)})`);

    for (const [j, v] of [["X", -153.586], ["Y", -300], ["Z", -350]]) await machine(p, `jog ${j} ${v}`);
    s = await machine(p, "jog B -120");
    assert.match(s, /contact: .*(cradle\/head|head\/cradle) HIT/, s);
    f = await t.pixels(p);
    // drawn posed: the frame moved with the jog. KNOWN ISSUE (research/cam/machine-builder.md §7):
    // in the browser the bodies are drawn unlit (black), so the hit's red is not checked here;
    // natively (tests/machine_builder.rs) it is.
    assert.ok(f.changed > 5000, `the jogged machine drawn: ${f.changed} bytes changed`);
    let red = 0;
    for (let i = 0; f.rgba && i < f.rgba.length; i += 4) if (f.rgba[i] > 180 && f.rgba[i + 1] < 110 && f.rgba[i + 2] < 110) red++;
    console.log(`ok - jogged into the head/cradle contact, ${f.changed} bytes changed, ${red} red pixels (${t.shot("web-03-collision.png", f)})`);

    s = await machine(p, "check");
    f = await t.pixels(p);
    console.log(`ok - checked (${t.shot("web-04-check.png", f)})`);

    s = await machine(p, "save project");
    assert.match(s, /Saved \/project\/cam\/machines\/web-trunnion\.toml \(project level\)/, s);
    await p.evaluate(() => window.forgecad.settled());
    const file = path.join(dir, "cam/machines/web-trunnion.toml");
    assert.ok(fs.existsSync(file), "saved through the page to the machine's folder");
    assert.ok(fs.existsSync(path.join(dir, "cam/machines/parts/web-trunnion/cradle.fg")), "its parts came along");
    f = await t.pixels(p);
    console.log(`ok - saved to ${file} (${t.shot("web-05-saved.png", f)})`);

    // the native CLI reads the browser's file as the same machine
    const out = execFileSync(t.bins.forgecad, ["machine", "list"], { cwd: dir }).toString();
    assert.match(out, /web-trunnion\s+table-table\s+fanuc\s+project/, out);
    console.log("ok - `forgecad machine list` in the project lists it at the project level");
    t.check(p);
  } finally {
    await b.close();
    proc.kill();
  }
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
