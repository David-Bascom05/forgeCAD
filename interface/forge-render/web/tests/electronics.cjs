// Open each .fg through the web app, select a real browser row and capture the shared editor.
// Run against the static ForgeCAD web build with FORGE_WEB_SHOTS set to the screenshot folder.
const t = require("./lib.cjs");
async function verifyAttachedPdf(page) {
  const path = "/project/examples/electronics/analog/analog-symbols.fg";
  const pdf = "%PDF-1.4\n%%EOF";
  await page.evaluate(({path,pdf}) => {
    const fc = window.forgecad.fc;
    fc.app_attach_pdf(path, "Amplifier", new TextEncoder().encode(pdf));
    fc.app_save();
  }, {path,pdf});
  await page.waitForFunction(({path,pdf}) => {
    const fc = window.forgecad.fc;
    const source = new TextDecoder().decode(fc.fs_read(path));
    const ref = source.match(/datasheets\/[a-f0-9]{64}\.pdf/);
    if (!ref) return false;
    const asset = path.slice(0,path.lastIndexOf("/") + 1) + ref[0];
    return new TextDecoder().decode(fc.fs_read(asset)) === pdf;
  }, {path,pdf}, {timeout:15000});
}
async function capture(browser, base, width, height, file, row, name) {
  const page = await t.page(browser, base + "/?offscreen=" + width + "x" + height + "&storage=memory");
  try {
    await t.until(page, s => s.ready, "app ready");
    await page.evaluate(() => window.forgecad.loaded);
    await page.evaluate(f => window.forgecad.fc.app_open("/project/examples/electronics/analog/" + f + ".fg"), file);
    const state = await t.until(page, s => s.places.some(p => p.name === row), row, 45000);
    await page.evaluate(id => window.forgecad.fc.app_click(id, 0, false), row);
    // Input arrives on successive frames. Wait for the selected row to acquire the
    // same highlight as its active sheet or symbol before reading back the whole window.
    const parent = file === "analog-symbols" ? "electronics:schchip:Amplifier" : "electronics:sheet:main";
    const rectangles = [row,parent].map(id => state.places.find(p => p.name === id).rect);
    await page.waitForFunction(async rects => {
      const image = await window.forgecad.pixels();
      if (!image) return false;
      const colour = ([x,y,w]) => {
        const offset = 4 * (Math.floor(y + 3) * image.width + Math.floor(x + w - 6));
        const p = image.rgba;
        return (p[offset] << 16) | (p[offset + 1] << 8) | p[offset + 2];
      };
      return colour(rects[0]) === colour(rects[1]);
    }, rectangles, {timeout:20000});
    const frame = await t.pixels(page);
    if (frame.width !== width || frame.height !== height || frame.colours < 12) throw Error("Incomplete editor frame");
    console.log(t.shot("web-" + name + ".png", frame));
    if (name === "symbol") await verifyAttachedPdf(page);
  } finally {
    await page.close();
  }
}
async function run(browser, base) {
  await capture(browser, base, 1600, 1000, "sensor-interface", "electronics:schchip:U1", "schematic");
  await capture(browser, base, 1400, 860, "analog-symbols", "electronics:schpin:VDD", "symbol");
  await capture(browser, base, 1024, 700, "sensor-interface", "electronics:schchip:C2", "compact");
}
module.exports = run;
if (require.main === module) (async () => {
  const browser = await t.browser();
  try { await run(browser, process.argv[2]); } finally { await browser.close(); }
})().catch(e => {console.error(e); process.exitCode = 1;});
