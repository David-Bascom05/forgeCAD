// What the page writes for a person: File > Export STL hands the part's mesh to the browser's
// downloads, and the weldment's Cut List > Export CSV writes the list beside the part, kept where
// the project is. Before: File's menu never opened in a page (a click's press and release came in
// one frame, and the menu took the press that opened it for a press outside it, shutting it in
// the frame it opened); its Export items were left out of the page's menu, which had nothing to
// do with them; and Export CSV said "Couldn't write ...: operation not supported on this
// platform" (it wrote through std's files, which a page has none of).
//   node web/tests/export.cjs OUT   (OUT: web/build.sh's output)
// The page is 1280x800 (offscreen), so the chrome is where these coordinates say.
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const t = require("./lib.cjs");

const at = {
  file: [33, 55],
  exportStl: [60, 217],
  weldmentsTab: [363, 55],
  cutList: [343, 110],
  exportCsv: [603, 710],
};
const pause = (ms) => new Promise((r) => setTimeout(r, ms));

(async () => {
  const app = path.resolve(process.argv[2] ?? "web/out");
  const dir = t.tmpdir("export");
  fs.cpSync(path.join(t.repo, "examples/weldments"), path.join(dir, "weldments"), { recursive: true });
  const { url, proc } = await t.serve(dir, app);
  const b = await t.browser();
  try {
    const p = await t.page(b, `${url}?open=weldments/gate_frame.fg&offscreen=1280x800`);
    await t.until(p, (s) => s.built?.checked, "the first build");
    await t.loaded(p);

    // File > Export STL: a binary STL in the downloads
    await p.mouse.click(...at.file);
    await pause(800);
    const got = p.waitForEvent("download", { timeout: 120000 });
    await p.mouse.click(...at.exportStl);
    const d = await got;
    assert.equal(d.suggestedFilename(), "gate_frame.stl");
    const stl = fs.readFileSync(await d.path());
    const tris = stl.readUInt32LE(80);
    assert.ok(tris > 100, `the STL has ${tris} triangles`);
    assert.equal(stl.length, 84 + 50 * tris, "a binary STL's length");
    const said = (await p.evaluate(() => window.forgecad.state())).status;
    assert.match(said, /^Exported gate_frame\.stl: /, said);
    console.log(`ok - File > Export STL downloaded gate_frame.stl (${tris} triangles): ${said}`);
    t.check(p);

    // Weldments > Cut List > Export CSV: written beside the part, on the machine serving the page
    await p.mouse.click(...at.weldmentsTab);
    await pause(800);
    await p.mouse.click(...at.cutList);
    await pause(1500);
    await p.mouse.click(...at.exportCsv);
    const csvSaid = (
      await t.until(p, (s) => /Cut list written|Couldn't write/.test(s.status), "the cut list's export", 60000)
    ).status;
    assert.match(csvSaid, /^Cut list written to /, csvSaid);
    await p.evaluate(() => window.forgecad.settled());
    const csv = fs.readFileSync(path.join(dir, "weldments/gate_frame-cutlist.csv"), "utf8");
    assert.match(csv, /RHS 60x40x3/, csv);
    console.log(`ok - Cut List > Export CSV kept with the project (${csv.trim().split("\n").length} lines)`);
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
