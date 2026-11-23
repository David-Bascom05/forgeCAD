// Shared by the browser tests: Chromium with WebGPU (a software adapter where there's no GPU),
// `forgecad web` started on a folder, waits on the app's state.
const { chromium } = require("playwright");
const { spawn } = require("node:child_process");
const fs = require("node:fs");
const path = require("node:path");

const repo = path.resolve(__dirname, "../../../..");
const env = (k, d) => process.env[k] ?? d;
/// Where cargo puts a workspace's builds, as cargo says (CARGO_TARGET_DIR, a config's target-dir).
function targetDir(dir) {
  const { execFileSync } = require("node:child_process");
  const meta = execFileSync("cargo", ["metadata", "--format-version", "1", "--no-deps"], { cwd: dir, maxBuffer: 1 << 26 });
  return JSON.parse(meta).target_directory;
}
const bins = {
  forgecad: env("FORGECAD", path.join(targetDir(repo), "release/forgecad")),
  parity: env("FORGE_PARITY", path.join(targetDir(path.join(repo, "interface/forge-render")), "release/forge-parity")),
};

async function browser() {
  return chromium.launch({
    args: [
      "--enable-unsafe-webgpu",
      "--enable-features=Vulkan",
      "--use-angle=swiftshader",
      "--use-webgpu-adapter=swiftshader",
      "--disable-vulkan-surface",
    ],
  });
}

/// A page with its console kept (and any panic or page error failing the test at once).
async function page(b, url) {
  const p = await b.newPage({ viewport: { width: 1280, height: 800 } });
  p.problems = [];
  const heard = (m) => {
    const t = m.text();
    if (process.env.VERBOSE) console.log(`  console: ${t.slice(0, 300)}`);
    if (/panicked|forge worker/.test(t)) p.problems.push(t);
  };
  p.on("console", heard);
  // the threaded build's Web Workers: a panic or a death on one is the page's too
  p.on("worker", (w) => w.on?.("console", heard));
  p.on("pageerror", (e) => p.problems.push(`page error: ${e.message}`));
  await p.goto(url);
  return p;
}

function check(p) {
  if (p.problems.length) throw new Error(p.problems.join("\n"));
}

/// Waits until `f(state)` holds; the state then.
async function until(p, f, what, ms = 600000) {
  const t0 = Date.now();
  let st = null;
  while (Date.now() - t0 < ms) {
    check(p);
    st = await p.evaluate(() => window.forgecad?.state?.() ?? null).catch(() => null);
    if (st && f(st)) return st;
    await new Promise((r) => setTimeout(r, 250));
  }
  throw new Error(`timed out waiting for ${what}; last state ${JSON.stringify(st)}`);
}

/// Every project file in the page (they follow the app's first frame) and the rebuild that comes
/// after them done: the state then.
async function loaded(p) {
  await p.evaluate(() => window.forgecad.loaded);
  return until(p, (s) => s.done >= s.seq && s.built?.checked && s.built.seq >= s.seq, "the rebuild after every file");
}

/// `forgecad web DIR --app APP` on a free port; resolves with its URL once it listens.
function serve(dir, app, extra = []) {
  return new Promise((ok, fail) => {
    const port = 20000 + Math.floor(Math.random() * 20000);
    const s = spawn(bins.forgecad, ["web", dir, "--app", app, "--port", String(port), ...extra], {
      stdio: ["ignore", "pipe", "inherit"],
    });
    s.on("error", fail);
    s.stdout.on("data", (d) => {
      const m = /at (http:\S+)/.exec(String(d));
      if (m) ok({ url: m[1], proc: s });
    });
  });
}

/// The native build of each file, as forge-parity reports it, by file.
function native(files) {
  const { execFileSync } = require("node:child_process");
  // capped: a native hang is said, not waited out (the sync call holds the run's own timers)
  const out = execFileSync(bins.parity, files, { maxBuffer: 1 << 26, timeout: Number(env("NATIVE_MS", 600000)) }).toString();
  return Object.fromEntries(out.trim().split("\n").map((l) => {
    const j = JSON.parse(l);
    return [j.file, j];
  }));
}

/// What must be the same, natively and in the page.
const SAME = ["ok", "volume_bits", "area_bits", "faces", "edges", "bodies", "problems"];

function differs(a, b) {
  const d = SAME.filter((k) => JSON.stringify(a?.[k]) !== JSON.stringify(b?.[k]));
  if ((a?.errors?.length ?? 0) !== (b?.errors?.length ?? 0)) d.push("errors");
  return d;
}

/// The app's frame (offscreen mode): {width, height, rgba}, and how many colours it has.
/// The app's next frame (offscreen mode), looked at in the page (a frame is megabytes: carried
/// out as numbers it took minutes): {width, height, colours, changed} (bytes changed from the
/// frame read before it, -1 for the first), and its RGBA only when $FORGE_WEB_SHOTS asks to keep
/// pictures.
async function pixels(p) {
  const keep = !!process.env.FORGE_WEB_SHOTS;
  const r = await p.evaluate(async (keep) => {
    const f = await window.forgecad.pixels();
    if (!f) return null;
    const rgba = f.rgba;
    const colours = new Set();
    for (let i = 0; i < rgba.length; i += 4 * 7) colours.add((rgba[i] << 16) | (rgba[i + 1] << 8) | rgba[i + 2]);
    const last = window.forge_last_frame;
    let changed = -1;
    if (last && last.length === rgba.length) {
      changed = 0;
      for (let i = 0; i < rgba.length; i++) if (rgba[i] !== last[i]) changed++;
    }
    window.forge_last_frame = rgba;
    let b64 = null;
    if (keep) {
      let s = "";
      for (let i = 0; i < rgba.length; i += 0x8000) s += String.fromCharCode(...rgba.subarray(i, i + 0x8000));
      b64 = btoa(s);
    }
    return { width: f.width, height: f.height, colours: colours.size, changed, b64 };
  }, keep);
  if (!r) throw new Error("the frame didn't read back");
  return { ...r, rgba: r.b64 && Uint8Array.from(Buffer.from(r.b64, "base64")) };
}

/// Frames drawn after the state `st` was read: waits for `n` more (a change on screen is drawn).
async function frames(p, st, n = 2) {
  return until(p, (s) => s.frames >= st.frames + n, `${n} frames drawn`);
}

/// Where the tests keep what the app drew: $FORGE_WEB_SHOTS, else the system's temp folder.
function shot(name, f) {
  if (!f.rgba) return "not kept: FORGE_WEB_SHOTS=DIR keeps it";
  const dir = process.env.FORGE_WEB_SHOTS;
  fs.mkdirSync(dir, { recursive: true });
  const at = path.join(dir, name);
  fs.writeFileSync(at, require("./png.cjs")(f.width, f.height, f.rgba));
  return at;
}

function tmpdir(tag) {
  return fs.mkdtempSync(path.join(require("node:os").tmpdir(), `forgecad-${tag}-`));
}

module.exports = { repo, bins, browser, page, check, until, loaded, frames, serve, native, differs, tmpdir, pixels, shot, SAME };
