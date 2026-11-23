// The ForgeCAD app in the page: the desktop's editor (webapp.rs) on a canvas, its files kept by
// storage.js. Served by `forgecad web DIR`, the project is DIR on that machine; served as static
// files, it is kept in this browser (OPFS, else IndexedDB), with import and export.
//   ?open=PATH       the project file to open first
//   ?project=NAME    the browser-kept project (default "default")
//   ?token=T         `forgecad web`'s token, when it was started with one (kept for the tab)
//   ?storage=memory  nothing kept (tests)
//   ?offscreen=WxH  frames drawn to a texture, not the canvas (a headless browser's software
//                    WebGPU can't show a canvas); forgecad.pixels() reads them
//   ?noapp           no canvas: the kernel and files only (the parity test)

import init, * as fc from "./forge_render.js";
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
    const first = got.map((f) => cleanPath(f.path)).find((p) => /\.(fg|dwg)$/i.test(p));
    if (first) fc.app_open(`${MOUNT}/${first}`);
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

await init();
fc.app_init();
const store = await backend();
let mirror = null;
const problems = [];
// the app's own settings and state (`/home`): kept in this browser, whatever serves the files
const home = store ? new Mirror(await localBackend("_app"), "/home", fc, (e) => console.error(`forge-app: ${e}`)) : null;
await home?.load();
window.forge_fs_changed = (path, bytes) => {
  if (mirror?.owns(path)) mirror.changed(path, bytes);
  else if (home?.owns(path)) home.changed(path, bytes);
};
if (store) {
  mirror = new Mirror(store, MOUNT, fc, (e) => {
    problems.push(String(e));
    console.error(`forge-app: ${e}`);
    if (e.name === "StaleBase") alert(`${e.message}\nYour edit is kept in this tab; reload to see the other.`);
  });
  await mirror.load();
}
// a new browser project starts with the examples (a machine's folder is never written to unasked)
if (files().length === 0 && store?.kind !== "server") {
  for (const p of await examples()) {
    const bytes = new Uint8Array(await (await fetch(`examples/${p}`)).arrayBuffer());
    fc.fs_write(`${MOUNT}/examples/${p}`, bytes);
  }
}
window.forge_pick = openDialog;
window.forge_save_as = (name) => {
  const n = prompt("Save as (a path in the project)", name || "part.fg");
  if (n) fc.app_save_as(`${MOUNT}/${cleanPath(n)}`);
};
window.forgecad = {
  fc,
  mirror,
  problems,
  state: () => JSON.parse(fc.app_state()),
  settled: () => Promise.all([mirror?.settled(), home?.settled()]),
  files,
};
const first = q.get("open") ?? files().filter((p) => p.endsWith(".fg")).sort()[0];
// ?noapp: the files and the kernel only, no canvas (the parity test)
if (!q.has("noapp")) fc.start_app(first ? `${MOUNT}/${cleanPath(first)}` : undefined, q.get("offscreen") ?? undefined);
say("started");
// the app's next frame as {width, height, rgba} (a headless browser can't capture the canvas)
window.forgecad.pixels = () => new Promise((ok) => fc.app_pixels(ok));
