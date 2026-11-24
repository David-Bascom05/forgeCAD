// Photo and Record in the browser (gui_record.rs, research/render/record.md): a photo downloads as
// a PNG at the size asked; a turntable records through WebCodecs' H.264 encoder into the app's own
// MP4 muxer (record/mp4.rs) and downloads, at odd sides grown to even ones; ffprobe, where there is
// one, reads it back as H.264 with every frame; then the bend simulation's timeline the same way.
// A browser without avc1 gets a PNG zip instead.
//   node web/tests/record.cjs OUT   (OUT: web/build.sh's output, served as static files)
// RECORD_OUT=DIR keeps what was downloaded.
const assert = require("node:assert/strict");
const { execFileSync } = require("node:child_process");
const fs = require("node:fs");
const http = require("node:http");
const os = require("node:os");
const path = require("node:path");
const t = require("./lib.cjs");

const CHANNEL = `shared t = 2
shared r = 2
shared k = ratio(0.4)
shared h = 30
sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (100, 200))
}
base = sheet(plan, t: t, r: r, bend: kfactor(k))
right = flange(base.edge(plan.outline[1]), height: h)
left = flange(base.edge(plan.outline[3]), height: h)
`;

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

    // FORGE_WEB_SHOTS=DIR: the settings window, as the browser draws it
    if (process.env.FORGE_WEB_SHOTS) {
      // Capture is on the View tab in the design view
      await p.evaluate(() => window.forgecad.fc.app_click("tab:View", 0, false));
      await t.pixels(p);
      await t.until(p, (s) => s.places.some((q) => q.name === "tool:Capture Settings"), "Capture on the View tab");
      await p.evaluate(() => window.forgecad.fc.app_click("tool:Capture Settings", 0, false));
      await t.pixels(p);
      console.log(`  ${t.shot("web-capture-settings.png", await t.pixels(p))}`);
      await p.evaluate(() => window.forgecad.fc.app_click("tool:Capture Settings", 0, false));
      await t.pixels(p);
    }
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

    // an environment's timeline: the bend simulation of a channel, 2 bends of 4 s, at 10 fps
    await p.evaluate((text) => {
      window.forgecad.fc.fs_write("/project/rec/channel.fg", new TextEncoder().encode(text));
      window.forgecad.fc.app_open("/project/rec/channel.fg");
      window.forgecad.fc.app_env("bendsim");
    }, CHANNEL);
    await t.until(p, (s) => s.env === "bendsim" && s.done >= s.seq, "in Bend Simulation");
    if (process.env.FORGE_WEB_SHOTS) {
      // the pill, part way through a recording
      await p.evaluate(() => window.forgecad.fc.app_capture("record 640x360 fps=10"));
      await t.until(p, (s) => s.capture?.frame?.[0] >= 10, "a recording under way");
      console.log(`  ${t.shot("web-recording.png", await t.pixels(p))}`);
      await p.evaluate(() => window.forgecad.fc.app_click("capture:stop", 0, false));
      await t.until(p, (s) => !s.capture.busy, "stopped");
    }
    const bend = await download(p, "record 640x360 fps=10");
    assert.match(bend.name, /^channel-bendsim-\d{8}-\d{6}\.mp4$/, bend.said);
    assert.match(bend.said, /\(80 frames, WebCodecs H\.264\)$/);
    const bendMp4 = path.join(keep, "bendsim.mp4");
    fs.writeFileSync(bendMp4, bend.bytes);
    try {
      probe = execFileSync("ffprobe", ["-v", "error", "-count_frames", "-select_streams", "v:0",
        "-show_entries", "stream=codec_name,width,height,nb_read_frames", "-of", "csv=p=0", bendMp4]).toString().trim();
      assert.equal(probe, "h264,640,360,80");
    } catch (e) {
      if (e.code !== "ENOENT") throw e;
    }
    console.log(`ok - the bend simulation recorded: ${bend.said}; ffprobe: ${probe}; ${bendMp4}`);
    t.check(p);
  } finally {
    await b.close();
    served.close();
  }
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
