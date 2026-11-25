// The ForgeCAD app in the page: the desktop's editor (webapp.rs) on a canvas, its files kept by
// storage.js. Served by `forgecad web DIR`, the project is DIR on that machine; served as static
// files, it is kept in this browser (OPFS, else IndexedDB), with import and export.
//   ?open=PATH       the project file to open first
//   ?env=NAME        the environment to open it in (thermoform, bendsim, render, ...: shell/env)
//   ?project=NAME    the browser-kept project (default "default")
//   ?token=T         `forgecad web`'s token, when it was started with one (kept for the tab)
//   #agent=T         `forgecad web --agent`'s agent token: held in memory only, taken off the
//                    address bar; a reload pairs again from the link forgecad web printed
//   ?storage=memory  nothing kept (tests)
//   ?offscreen=WxH  frames drawn to a texture, not the canvas (a headless browser's software
//                    WebGPU can't show a canvas); forgecad.pixels() reads them
//   ?threads=0       the single-threaded build even where the page is cross-origin isolated
//   ?noapp           no canvas: the kernel and files only (the parity test)

import { Mirror, ServerBackend, localBackend, cleanPath } from "./storage.js";
import { zip, unzip } from "./zip.js";

const q = new URLSearchParams(location.search);
const MOUNT = "/project";
const say = (s) => console.log(`forge-app: ${s}`);

function token() {
  const t = q.get("token");
  if (t) sessionStorage.setItem("forgecad-token", t);
  return t ?? sessionStorage.getItem("forgecad-token");
}

/// The server's files when `forgecad web` serves the page, else the browser's own.
async function backend() {
  if (q.get("storage") === "memory") return null;
  const server = new ServerBackend("", token());
  try {
    const info = await server.info();
    if (info.server === "forgecad web") {
      say(`files from ${info.root} on the machine serving this page`);
      return server;
    }
  } catch {}
  const local = await localBackend(q.get("project") ?? "default");
  say(`files kept in this browser (${local.kind})`);
  return local;
}

/// The bundled examples, for an empty project.
async function examples() {
  try {
    return (await (await fetch("examples/list.txt")).text()).trim().split("\n").filter(Boolean);
  } catch {
    return [];
  }
}

function download(name, blob) {
  const a = document.createElement("a");
  a.href = URL.createObjectURL(blob);
  a.download = name;
  a.click();
  setTimeout(() => URL.revokeObjectURL(a.href), 1000);
}

const files = () => fc.fs_list(MOUNT).map((p) => p.slice(MOUNT.length + 1));

async function importFiles(list) {
  for (const f of list) {
    const path = cleanPath(f.path);
    // through the app's filesystem, so the mirror keeps it
    fc.fs_write(`${MOUNT}/${path}`, f.bytes);
  }
}

/// File > Open: the project's files, the examples, the computer's files, a zip.
function openDialog() {
  const old = document.getElementById("forge-open");
  if (old) old.remove();
  const d = document.createElement("dialog");
  d.id = "forge-open";
  d.style.cssText = "font: 13px system-ui, sans-serif; min-width: 380px; max-height: 70vh; overflow: auto";
  const list = files().filter((p) => /\.(fg|dwg)$/i.test(p)).sort();
  d.innerHTML = `<form method="dialog"><h3 style="margin:0 0 8px">Open</h3>
    <div id="forge-open-list"></div>
    <p><label>From this computer: <input type="file" id="forge-open-file" multiple accept=".fg,.dwg,.ttf,.otf,.step,.stp,.zip"></label></p>
    <p><label>A folder: <input type="file" id="forge-open-dir" webkitdirectory></label></p>
    <p><button value="export" id="forge-export">Export the project (.zip)</button> <button value="cancel">Cancel</button></p></form>`;
  const ul = d.querySelector("#forge-open-list");
  for (const p of list) {
    const b = document.createElement("button");
    b.textContent = p;
    b.style.cssText = "display:block; width:100%; text-align:left; margin:2px 0";
    b.dataset.path = p;
    b.onclick = (e) => {
      e.preventDefault();
      d.close();
      fc.app_open(`${MOUNT}/${p}`);
    };
    ul.append(b);
  }
  const take = async (input, folder) => {
    const got = [];
    for (const f of input.files) {
      const bytes = new Uint8Array(await f.arrayBuffer());
      if (/\.zip$/i.test(f.name)) got.push(...(await unzip(bytes)));
      else got.push({ path: folder ? f.webkitRelativePath : f.name, bytes });
    }
    await importFiles(got);
    d.close();
    const paths = got.map((f) => cleanPath(f.path));
    const first = paths.find((p) => /\.(fg|dwg)$/i.test(p));
    if (first) fc.app_open(`${MOUNT}/${first}`);
    // nothing to open (a STEP model, a font: read by a part): said, not silently added
    else if (paths.length) fc.app_say(`Added ${paths.length > 3 ? `${paths.length} files` : paths.join(", ")} to the project`);
  };
  d.querySelector("#forge-open-file").onchange = (e) => take(e.target, false);
  d.querySelector("#forge-open-dir").onchange = (e) => take(e.target, true);
  d.querySelector("#forge-export").onclick = (e) => {
    e.preventDefault();
    const all = fc.fs_list(MOUNT).map((p) => ({ path: p.slice(MOUNT.length + 1), bytes: fc.fs_read(p) }));
    download("project.zip", zip(all));
  };
  document.body.append(d);
  d.showModal();
}

// the threaded build (threads/, the build on a Web Worker) where the page may share memory
// (cross-origin isolated: `forgecad web` sends the headers) and it was built; else the
// single-threaded one. Both build the same bits.
async function threadedBuild() {
  if (!self.crossOriginIsolated || q.get("threads") === "0") return false;
  try {
    return (await fetch("threads/forge_render.js", { method: "HEAD" })).ok;
  } catch {
    return false;
  }
}
const threaded = await threadedBuild();
const fc = await import(threaded ? "./threads/forge_render.js" : "./forge_render.js");
await fc.default();
if (threaded) (await import("./threads/threads.js")).threadsOn(() => fc.app_wake());
say(threaded ? "the threaded build: builds on a Web Worker" : "the single-threaded build");
fc.app_init();
const store = await backend();
let mirror = null;
const problems = [];
// the app's own settings and state (`/home`): kept in this browser, whatever serves the files
const home = store ? new Mirror(await localBackend("_app"), "/home", fc, (e) => console.error(`forge-app: ${e}`)) : null;
// a file the app made (a photo, a recording, File > Export STL / 3MF / OBJ), to the downloads
window.forge_download = (name, bytes) => download(name, new Blob([bytes.slice()]));

// MP4 recording (gui_record.rs, record/web.rs): WebCodecs' H.264 encoder, its frames handed back
// to the app, which writes the MP4 itself (record/mp4.rs). No B-frames, AVCC NAL units, the avcC
// record from decoderConfig.description. window.forge_avc1: whether this browser has one at all.
const avcCodec = (w, h, fps) => {
  // H.264 constrained baseline at the least level the size and rate fit (Table A-1: MaxFS, MaxMBPS)
  const mbs = Math.ceil(w / 16) * Math.ceil(h / 16);
  const levels = [[0x1f, 3600, 108000], [0x20, 5120, 216000], [0x28, 8192, 245760], [0x2a, 8704, 522240],
    [0x32, 22080, 589824], [0x33, 36864, 983040], [0x34, 36864, 2073600]];
  const l = levels.find(([, fs, mbps]) => mbs <= fs && mbs * fps <= mbps) ?? levels[levels.length - 1];
  return `avc1.42E0${l[0].toString(16).padStart(2, "0")}`;
};
window.forge_avc1 = false;
if (typeof VideoEncoder === "function") {
  VideoEncoder.isConfigSupported({ codec: avcCodec(1280, 720, 30), width: 1280, height: 720, avc: { format: "avc" } })
    .then((s) => (window.forge_avc1 = !!s.supported), () => {});
}
const videos = new Map();
let videoNext = 1;
window.forge_video_open = (w, h, fps) => {
  if (!window.forge_avc1) return 0;
  const v = { chunks: [], avcc: null, state: 0, error: "", fps };
  try {
    v.enc = new VideoEncoder({
      output: (chunk, meta) => {
        const data = new Uint8Array(chunk.byteLength);
        chunk.copyTo(data);
        v.chunks.push([chunk.type === "key", data]);
        const d = meta?.decoderConfig?.description;
        if (d) v.avcc = d instanceof ArrayBuffer ? new Uint8Array(d.slice(0))
          : new Uint8Array(d.buffer.slice(d.byteOffset, d.byteOffset + d.byteLength));
      },
      error: (e) => { v.state = -1; v.error = String(e); },
    });
    v.enc.configure({ codec: avcCodec(w, h, fps), width: w, height: h, framerate: fps,
      bitrate: Math.round(w * h * fps * 0.15), latencyMode: "quality", avc: { format: "avc" } });
  } catch (e) {
    return 0;
  }
  videos.set(videoNext, v);
  return videoNext++;
};
window.forge_video_frame = (id, rgba, w, h, k) => {
  const v = videos.get(id);
  if (!v || v.state < 0) return;
  try {
    const f = new VideoFrame(rgba.slice(), { format: "RGBA", codedWidth: w, codedHeight: h,
      timestamp: Math.round((k * 1e6) / v.fps), duration: Math.round(1e6 / v.fps) });
    v.enc.encode(f, { keyFrame: k % v.fps === 0 });
    f.close();
  } catch (e) { v.state = -1; v.error = String(e); }
};
// frames still being encoded: the app waits rather than queue the whole video in memory
window.forge_video_backlog = (id) => videos.get(id)?.enc.encodeQueueSize ?? 0;
window.forge_video_finish = (id) => {
  const v = videos.get(id);
  if (v && v.state === 0) v.enc.flush().then(() => { if (v.state === 0) v.state = 1; }, (e) => { v.state = -1; v.error = String(e); });
};
// 0 encoding, 1 done, -1 failed
window.forge_video_state = (id) => videos.get(id)?.state ?? -1;
window.forge_video_error = (id) => videos.get(id)?.error ?? "no such video";
// what was made: [u32 avcC length][avcC][u32 frames]([u8 key][u32 length][AVCC bytes])..., little-endian
window.forge_video_take = (id) => {
  const v = videos.get(id);
  videos.delete(id);
  if (!v) return new Uint8Array();
  try { v.enc.close(); } catch {}
  const avcc = v.avcc ?? new Uint8Array();
  const n = 8 + avcc.length + v.chunks.reduce((a, [, d]) => a + 5 + d.length, 0);
  const out = new Uint8Array(n), dv = new DataView(out.buffer);
  let at = 0;
  dv.setUint32(at, avcc.length, true); out.set(avcc, at + 4); at += 4 + avcc.length;
  dv.setUint32(at, v.chunks.length, true); at += 4;
  for (const [key, d] of v.chunks) { out[at] = key ? 1 : 0; dv.setUint32(at + 1, d.length, true); out.set(d, at + 5); at += 5 + d.length; }
  return out;
};
window.forge_video_cancel = (id) => { const v = videos.get(id); videos.delete(id); try { v?.enc.close(); } catch {} };

window.forge_fs_changed = (path, bytes) => {
  if (mirror?.owns(path)) mirror.changed(path, bytes);
  else if (home?.owns(path)) home.changed(path, bytes);
};
if (store) {
  mirror = new Mirror(store, MOUNT, fc, (e, path) => {
    problems.push(String(e));
    console.error(`forge-app: ${e}`);
    // the app says so too: never a "saved" or "History is on" over a write that was lost
    if (path) fc.app_store_failed(path, e.message ?? String(e));
    if (e.name === "StaleBase") alert(`${e.message}\nYour edit is kept in this tab; reload to see the other.`);
  });
}
window.forge_pick = openDialog;
// Files the app exports (NC programs, a setup sheet): the browser's save picker where it has one
// (and the page may use it: a save needs the user's click, so it falls back when refused), else a
// download; several go in one zip. window.forge_exported says what was saved last.
window.forge_export = async (json) => {
  const { name, files } = JSON.parse(json);
  const enc = new TextEncoder();
  const one = files.length === 1;
  const fname = one ? files[0].path : name;
  const blob = one
    ? new Blob([enc.encode(files[0].text)], { type: "text/plain" })
    : zip(files.map((f) => ({ path: f.path, bytes: enc.encode(f.text) })));
  if (window.showSaveFilePicker && !window.FORGE_DOWNLOAD_ONLY) {
    try {
      const h = await window.showSaveFilePicker({ suggestedName: fname });
      const w = await h.createWritable();
      await w.write(blob);
      await w.close();
      window.forge_exported = h.name;
      return;
    } catch (e) {
      if (e.name === "AbortError") return;
    }
  }
  download(fname, blob);
  window.forge_exported = fname;
};
window.forge_save_as = (name) => {
  const n = prompt("Save as (a path in the project)", name || "part.fg");
  if (!n) return;
  // a name outside the project is refused, and the app says so (it threw, and nothing said)
  try {
    fc.app_save_as(`${MOUNT}/${cleanPath(n)}`);
  } catch {
    // (cleanPath refuses only a path that climbs out with `..`)
    fc.app_say(`Couldn't save as ${n}: it isn't in this project.`);
  }
};
// the link's file, when it is a path the project may hold; else the app starts with none and
// says why (`?open=../x` threw here, before the app started: a dead page)
let asked = null;
let refused = null;
if (q.get("open")) {
  try {
    asked = cleanPath(q.get("open"));
  } catch {
    refused = `Couldn't open ${q.get("open")}: it isn't in this project.`;
  }
}
// the app first: the project's files follow behind its first frame (hundreds of them took a
// round trip each before it showed). A linked file opens once they have all come: opened before,
// what it reads (a machining document's part, a drawing's) was missing from its first build
// ("0/5 ops certified" on a good part, its export refused, until a rebuild)
await home?.load();
// ?noapp: the files and the kernel only, no canvas (the parity test)
if (!q.has("noapp")) {
  fc.start_app(undefined, q.get("offscreen") ?? undefined);
  if (asked) fc.app_waiting(asked);
  else if (q.get("env")) fc.app_env(q.get("env"));
  if (refused) fc.app_say(refused);
}
say("started");
const loaded = (async () => {
  await mirror?.load();
  // a new browser project starts with the examples (a machine's folder is never written to unasked)
  if (files().length === 0 && store?.kind !== "server") {
    const list = await examples();
    const got = await Promise.all(
      list.map(async (p) => [p, new Uint8Array(await (await fetch(`examples/${p}`)).arrayBuffer())]),
    );
    for (const [p, bytes] of got) fc.fs_write(`${MOUNT}/examples/${p}`, bytes);
  }
  if (q.has("noapp")) return;
  if (asked) {
    // (what the person opened or made meanwhile is kept: the app says the link's file wasn't)
    fc.app_open_linked(`${MOUNT}/${asked}`, asked, q.get("env") ?? undefined);
  } else if (!refused) {
    // (a link refused: nothing else opened in its place, the status saying why)
    const first = files().filter((p) => p.endsWith(".fg")).sort()[0];
    if (first) fc.app_open(`${MOUNT}/${first}`);
  }
  say("every file in");
})();
// The agent: the person's own Claude Code on the machine serving this page (`forgecad web DIR
// --agent`, interface/forge-cli/src/agent_bridge.rs). Its token is held here only, never stored;
// the page's id (which page owns the session) lasts the tab, so a reload keeps it.
const agentToken = (() => {
  const m = /(?:^#|&)agent=([0-9a-f]{32})(?:&|$)/.exec(location.hash);
  if (m) history.replaceState(null, "", location.pathname + location.search);
  return m ? m[1] : null;
})();
let agentClient = sessionStorage.getItem("forgecad-agent-client");
if (!agentClient) {
  agentClient = crypto.randomUUID();
  sessionStorage.setItem("forgecad-agent-client", agentClient);
}
let agentStream = null;
const agentHeaders = () => ({ Authorization: `Bearer ${agentToken}`, "X-Forge-Client": agentClient });
window.forge_agent_ready = () => agentToken !== null;
/// A call: its status and body, as JSON text (status 0: it didn't reach the server).
window.forge_agent_call = async (name, body) => {
  try {
    const r = await fetch(`/api/agent/${name}`, {
      method: "POST",
      headers: { ...agentHeaders(), "Content-Type": "application/json" },
      body,
    });
    return JSON.stringify({ status: r.status, body: await r.text() });
  } catch (e) {
    return JSON.stringify({ status: 0, body: JSON.stringify({ error: String(e) }) });
  }
};
/// The session's lines from the `from`th, each to onLine as it comes; a new call ends the last.
window.forge_agent_events = (from, onLine) => {
  agentStream?.abort();
  const ctl = new AbortController();
  agentStream = ctl;
  const said = (e) => onLine(JSON.stringify({ type: "forge_bridge", error: e }));
  (async () => {
    try {
      const r = await fetch(`/api/agent/events?from=${from}`, { headers: agentHeaders(), signal: ctl.signal });
      if (!r.ok) return said(`The agent's stream was refused (HTTP ${r.status})`);
      const rd = r.body.getReader();
      const dec = new TextDecoder();
      let buf = "";
      for (;;) {
        const { value, done } = await rd.read();
        if (done) break;
        buf += dec.decode(value, { stream: true });
        let k;
        while ((k = buf.indexOf("\n")) >= 0) {
          const line = buf.slice(0, k).trim();
          buf = buf.slice(k + 1);
          if (line) onLine(line);
        }
      }
      if (!ctl.signal.aborted) said("The connection to forgecad web closed");
    } catch (e) {
      if (!ctl.signal.aborted) said(`The connection to forgecad web failed: ${e}`);
    }
  })();
};
/// A file the agent wrote, already in the app's filesystem: its version, so the next save is
/// made against it and isn't refused as stale.
window.forge_agent_adopt = (rel, etag) => mirror?.versions.set(rel, etag);
window.forgecad = {
  fc,
  threaded,
  mirror,
  problems,
  loaded,
  state: () => JSON.parse(fc.app_state()),
  settled: () => Promise.all([mirror?.settled(), home?.settled()]),
  files,
};
// the app's next frame as {width, height, rgba} (a headless browser can't capture the canvas)
window.forgecad.pixels = () => new Promise((ok) => fc.app_pixels(ok));
