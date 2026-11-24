// Photo and Record in the browser (gui_record.rs, research/render/record.md): a photo downloads as
// a PNG at the size asked; a turntable records through WebCodecs' H.264 encoder into the app's own
// MP4 muxer (record/mp4.rs) and downloads, at odd sides grown to even ones; ffprobe, where there is
// one, reads it back as H.264 with every frame. A browser without avc1 gets a PNG zip instead.
//   node web/tests/record.cjs OUT   (OUT: web/build.sh's output, served as static files)
// RECORD_OUT=DIR keeps what was downloaded.
const assert = require("node:assert/strict");
const { execFileSync } = require("node:child_process");
const fs = require("node:fs");
const http = require("node:http");
const os = require("node:os");
const path = require("node:path");
const t = require("./lib.cjs");

function serveStatic(root) {
  const types = { ".html": "text/html", ".js": "text/javascript", ".wasm": "application/wasm" };
  return new Promise((ok) => {
    const s = http.createServer((q, r) => {
      const u = decodeURIComponent(new URL(q.url, "http://x").pathname);
      const f = path.join(root, u === "/" ? "index.html" : u);
      if (!f.startsWith(root) || !fs.existsSync(f) || fs.statSync(f).isDirectory()) {
        r.writeHead(404);
        return r.end();
      }
      r.writeHead(200, { "content-type": types[path.extname(f)] ?? "application/octet-stream" });
      fs.createReadStream(f).pipe(r);
    });
    s.listen(0, "127.0.0.1", () => ok({ url: `http://127.0.0.1:${s.address().port}/`, close: () => s.close() }));
  });
}

/// What the app hands over next as a download: its name and bytes.
async function download(p, words) {
  const before = (await p.evaluate(() => window.forgecad.state())).capture?.last;
  await p.evaluate((w) => {
    window.got = null;
    window.forge_download = (name, bytes) => (window.got = { name, bytes: Array.from(bytes) });
    window.forgecad.fc.app_capture(w);
  }, words);
  // done: what it said changed (a capture's words name a fresh file, or say why not)
  await t.until(p, (s) => s.capture && !s.capture.busy && s.capture.last && s.capture.last !== before, `${words}: done`, 300000);
  const st = await p.evaluate(() => window.forgecad.state());
  const got = await p.evaluate(() => window.got);
  assert.ok(got, `${words}: nothing downloaded (${st.capture.last})`);
  return { name: got.name, bytes: Buffer.from(got.bytes), said: st.capture.last };
}

(async () => {
  const app = path.resolve(process.argv[2] ?? "web/out");
  const keep = process.env.RECORD_OUT ?? fs.mkdtempSync(path.join(os.tmpdir(), "fg-record-"));
  fs.mkdirSync(keep, { recursive: true });
  const served = await serveStatic(app);
  const b = await t.browser();
  try {
    const p = await t.page(b, `${served.url}?offscreen=1000x700&storage=memory`);
    await t.until(p, (s) => s.ready, "the app");
    await p.evaluate(() => window.forgecad.loaded);
    await p.evaluate(() => window.forgecad.fc.app_open("/project/examples/machined/flange.fg"));
    await t.until(p, (s) => s.built?.checked && s.done >= s.seq, "the build");

    const photo = await download(p, "photo 320x200");
    assert.match(photo.name, /^flange-view-\d{8}-\d{6}\.png$/);
    assert.equal(photo.bytes.readUInt32BE(16), 320);
    assert.equal(photo.bytes.readUInt32BE(20), 200);
    fs.writeFileSync(path.join(keep, "photo.png"), photo.bytes);
    console.log(`ok - photo: ${photo.said}`);

    // isConfigSupported answers on its own time
    await p.waitForFunction(() => window.forge_avc1 === true, null, { timeout: 5000 }).catch(() => {});
    const avc1 = await p.evaluate(() => window.forge_avc1);
    const rec = await download(p, "record 161x97 fps=10 seconds=1");
    if (!avc1) {
      assert.match(rec.name, /\.zip$/, rec.said);
      assert.match(rec.said, /no H\.264 video encoder/);
      console.log(`ok - no avc1 in this browser: ${rec.said}`);
      return;
    }
    assert.match(rec.name, /^flange-view-\d{8}-\d{6}\.mp4$/, rec.said);
    assert.match(rec.said, /^Video downloaded: .* \(10 frames, WebCodecs H\.264\)$/);
    assert.equal(rec.bytes.toString("ascii", 4, 8), "ftyp");
    const mp4 = path.join(keep, "turntable.mp4");
    fs.writeFileSync(mp4, rec.bytes);
    let probe = "no ffprobe here";
    try {
      probe = execFileSync("ffprobe", ["-v", "error", "-count_frames", "-select_streams", "v:0",
        "-show_entries", "stream=codec_name,width,height,nb_read_frames", "-of", "csv=p=0", mp4]).toString().trim();
      assert.equal(probe, "h264,162,98,10");
    } catch (e) {
      if (e.code !== "ENOENT") throw e;
    }
    console.log(`ok - record: ${rec.said}; ffprobe: ${probe}; ${mp4}`);
    t.check(p);
  } finally {
    await b.close();
    served.close();
  }
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
