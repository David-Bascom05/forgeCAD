// The Render Studio's tracer in the browser (WebGPU, software in CI) against the golden picture
// the native test keeps: the same scene, seed and samples (src/studio/golden.rs), so the WGSL runs
// the same on both. Needs only the wasm build (web/build.sh OUT), served here as static files.
//   node web/tests/studio.cjs OUT
const assert = require("node:assert/strict");
const fs = require("node:fs");
const http = require("node:http");
const path = require("node:path");
const zlib = require("node:zlib");
const { chromium } = require("playwright");

/// An 8-bit RGBA PNG's pixels (what the native test writes: one IDAT stream, filters 0-4).
function readPng(file) {
  const b = fs.readFileSync(file);
  let at = 8;
  let w = 0;
  let h = 0;
  const idat = [];
  while (at < b.length) {
    const len = b.readUInt32BE(at);
    const type = b.toString("ascii", at + 4, at + 8);
    const data = b.subarray(at + 8, at + 8 + len);
    if (type === "IHDR") {
      w = data.readUInt32BE(0);
      h = data.readUInt32BE(4);
      assert.equal(data[8], 8, "8 bits");
      assert.equal(data[9], 6, "RGBA");
    } else if (type === "IDAT") idat.push(data);
    at += 12 + len;
  }
  const raw = zlib.inflateSync(Buffer.concat(idat));
  const out = Buffer.alloc(w * h * 4);
  const row = w * 4;
  for (let y = 0; y < h; y++) {
    const f = raw[y * (row + 1)];
    for (let x = 0; x < row; x++) {
      const v = raw[y * (row + 1) + 1 + x];
      const a = x >= 4 ? out[y * row + x - 4] : 0;
      const up = y > 0 ? out[(y - 1) * row + x] : 0;
      const c = x >= 4 && y > 0 ? out[(y - 1) * row + x - 4] : 0;
      let p;
      if (f === 0) p = 0;
      else if (f === 1) p = a;
      else if (f === 2) p = up;
      else if (f === 3) p = (a + up) >> 1;
      else {
        const e = a + up - c;
        const pa = Math.abs(e - a);
        const pb = Math.abs(e - up);
        const pc = Math.abs(e - c);
        p = pa <= pb && pa <= pc ? a : pb <= pc ? up : c;
      }
      out[y * row + x] = (v + p) & 255;
    }
  }
  return { w, h, rgba: out };
}

/// As `golden::compare`: RMSE and the 99th percentile of the per-channel difference.
function compare(got, want) {
  const d = [];
  for (let k = 0; k < want.length; k++) if (k % 4 !== 3) d.push(Math.abs(got[k] - want[k]));
  const rmse = Math.sqrt(d.reduce((s, x) => s + x * x, 0) / d.length);
  d.sort((x, y) => x - y);
  return { rmse, p99: d[Math.floor((d.length * 99) / 100)] };
}

const types = { ".js": "text/javascript", ".wasm": "application/wasm", ".html": "text/html" };

(async () => {
  const out = path.resolve(process.argv[2] ?? "web/out");
  const page = `<!doctype html><meta charset="utf-8"><script type="module">
    import init, { studio_golden } from "./forge_render.js";
    await init();
    window.done = studio_golden().then((px) => Array.from(px), (e) => "error: " + e);
  </script>`;
  const server = http.createServer((q, r) => {
    const f = q.url === "/" ? null : path.join(out, decodeURIComponent(q.url.split("?")[0]));
    if (!f) {
      r.writeHead(200, { "content-type": "text/html" });
      return r.end(page);
    }
    fs.readFile(f, (e, data) => {
      if (e) {
        r.writeHead(404);
        return r.end();
      }
      r.writeHead(200, { "content-type": types[path.extname(f)] ?? "application/octet-stream" });
      r.end(data);
    });
  });
  await new Promise((ok) => server.listen(0, "127.0.0.1", ok));
  const url = `http://127.0.0.1:${server.address().port}/`;
  const b = await chromium.launch({
    args: ["--enable-unsafe-webgpu", "--enable-features=Vulkan", "--use-angle=swiftshader", "--use-webgpu-adapter=swiftshader", "--disable-vulkan-surface"],
  });
  try {
    const p = await b.newPage();
    const problems = [];
    p.on("console", (m) => /panicked/.test(m.text()) && problems.push(m.text()));
    p.on("pageerror", (e) => problems.push(e.message));
    await p.goto(url);
    await p.waitForFunction(() => window.done !== undefined, null, { timeout: 120000 });
    const px = await p.evaluate(() => window.done);
    assert.ok(Array.isArray(px), `the studio failed in the browser: ${px} ${problems.join("\n")}`);
    const golden = readPng(path.join(__dirname, "../../tests/fixtures/studio_golden.png"));
    assert.equal(px.length, golden.w * golden.h * 4);
    const got = Buffer.from(px);
    const { rmse, p99 } = compare(got, golden.rgba);
    if (rmse > 0) fs.writeFileSync(path.join(out, "studio_golden_browser.png"), require("./png.cjs")(golden.w, golden.h, got));
    console.log(`golden in the browser: rmse ${rmse.toFixed(2)}, p99 ${p99} (8-bit levels)`);
    // the native test's bounds (golden.rs MAX_RMSE, MAX_P99)
    assert.ok(rmse < 6 && p99 < 40, `rmse ${rmse}, p99 ${p99}: see ${out}/studio_golden_browser.png`);
    console.log("ok - the browser traces the golden scene");
  } finally {
    await b.close();
    server.close();
  }
})().catch((e) => {
  console.error(`not ok - ${e.stack ?? e}`);
  process.exit(1);
});
