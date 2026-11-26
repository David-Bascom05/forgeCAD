// Inspect > Wall in the page. Where jobs run on the page's one thread (the single-threaded build)
// the panel waits for Measure thinnest wall, and the search then runs a few milliseconds a frame:
// the page keeps drawing, the panel says how far, and Cancel stops it. Before: the whole part's
// wall was worked out as the panel opened, on the page's thread: the welded gate frame held the
// tab for over five minutes. The threaded build works it out on a worker as the panel opens.
//   node web/tests/wall.cjs OUT   (OUT: web/build.sh's output)
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const t = require("./lib.cjs");

const PLATE = `sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (60, 30))
}
base = extrude(plan, 6)
`;
const at = { inspectTab: [449, 55], wall: [103, 87] };
const pause = (ms) => new Promise((r) => setTimeout(r, ms));
const shows = (s, name) => s.places.some((p) => p.name === name);
const place = (s, name) => {
  const r = s.places.find((p) => p.name === name)?.rect;
  assert.ok(r, `${name} not on screen`);
  return [r[0] + r[2] / 2, r[1] + r[3] / 2];
};

(async () => {
  const app = path.resolve(process.argv[2] ?? "web/out");
  const dir = t.tmpdir("wall");
  fs.writeFileSync(path.join(dir, "plate.fg"), PLATE);
  fs.cpSync(path.join(t.repo, "examples/weldments"), path.join(dir, "weldments"), { recursive: true });
  const { url, proc } = await t.serve(dir, app);
  const b = await t.browser();
  try {
    const p = await t.page(b, `${url}?open=weldments/gate_frame.fg&offscreen=1280x800`);
    await t.until(p, (s) => s.built?.checked, "the first build");
    await t.loaded(p);
    const threaded = await p.evaluate(() => window.forgecad.threaded);
    await p.mouse.click(...at.inspectTab);
    await pause(800);
    await p.mouse.click(...at.wall);
    if (threaded) {
      const s = await t.until(p, (s) => s.wall?.at != null, "the wall, on a worker", 300000);
      console.log(`ok - threaded: the gate frame's wall as the panel opened (${s.wall.at.toFixed(3)} mm)`);
      t.check(p);
      return;
    }
    let s = await t.until(p, (s) => s.wall != null && shows(s, "measure:close:close"), "the Wall panel", 30000);
    assert.ok(s.wall.held && !s.wall.busy, `the panel measured unasked: ${JSON.stringify(s.wall)}`);
    // asked: it works a few milliseconds a frame, and says how far
    await p.mouse.click(...place(s, "measure:wall:go"));
    s = await t.until(
      p,
      (s) => s.wall?.busy && s.wall.done > 0 && shows(s, "measure:wall:cancel"),
      "the search under way",
      120000,
    );
    const f0 = s.frames;
    await pause(1500);
    s = await p.evaluate(() => window.forgecad.state());
    assert.ok(s.frames > f0 + 5, `the page stopped drawing while it searched: ${f0} -> ${s.frames} frames`);
    assert.ok(s.wall.busy, "the gate frame's search ended at once");
    const done = s.wall.done;
    // Cancel: it stops, and waits for the button again
    await p.mouse.click(...place(s, "measure:wall:cancel"));
    s = await t.until(p, (s) => s.wall && !s.wall.busy, "the search stopped", 30000);
    assert.ok(s.wall.held && s.wall.at == null, `after Cancel: ${JSON.stringify(s.wall)}`);
    console.log(`ok - the gate frame's wall waited, ran ${Math.round(done * 100)}% between frames (${s.frames - f0} frames drawn), and stopped`);

    // a small part: asked, measured (the panel shut first: Wall again opens it)
    await p.keyboard.press("Escape");
    await t.until(p, (s) => s.wall == null, "the panel shut", 30000);
    await p.evaluate(() => window.forgecad.fc.app_open("/project/plate.fg"));
    await t.until(p, (s) => s.path === "/project/plate.fg" && s.built?.checked, "the plate");
    await p.mouse.click(...at.inspectTab);
    await pause(800);
    await p.mouse.click(...at.wall);
    s = await t.until(p, (s) => s.wall?.held && shows(s, "measure:wall:go"), "the plate's Wall panel", 30000);
    await p.mouse.click(...place(s, "measure:wall:go"));
    s = await t.until(p, (s) => s.wall?.at != null, "the plate's wall", 60000);
    assert.ok(Math.abs(s.wall.at - 6) < 1e-9, `the plate's wall: ${s.wall.at}`);
    console.log("ok - the plate's wall, asked: 6 mm");
    t.check(p);
  } finally {
    await b.close();
    proc.kill();
    fs.rmSync(dir, { recursive: true, force: true });
  }
})().catch((e) => {
  console.error(`not ok - ${e.stack ?? e}`);
  process.exit(1);
});
