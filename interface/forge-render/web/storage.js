// Where a browser project's files are kept. One interface, three places:
//   server: `forgecad web DIR` (the files on that machine; every file has an ETag, and a write
//           made against an old one is refused as StaleBase, as `forgecad edit --base` is)
//   opfs:   the browser's Origin Private File System
//   idb:    IndexedDB, where there is no OPFS
// Each backend: list() -> [{path, version}], read(path) -> {bytes, version},
// write(path, bytes, base) -> version (base: the version the write was made from; null for a new
// file, undefined to not check), remove(path, base), rename(from, to).
// Paths are relative to the project, `/`-separated, never with `..`.

export class StaleBase extends Error {
  constructor(path, current) {
    super(`${path} changed since it was read (now ${current ?? "gone"}): reload it`);
    this.name = "StaleBase";
    this.path = path;
    this.current = current;
  }
}

/// A path the project may hold: relative, no `..`, no empty parts.
export function cleanPath(p) {
  const parts = String(p).replaceAll("\\", "/").split("/").filter((s) => s && s !== ".");
  if (parts.length === 0 || parts.some((s) => s === "..")) throw new Error(`not a project path: ${p}`);
  return parts.join("/");
}

/// A version for bytes held locally: their SHA-256, hex.
async function digest(bytes) {
  const h = new Uint8Array(await crypto.subtle.digest("SHA-256", bytes));
  return Array.from(h.slice(0, 16), (b) => b.toString(16).padStart(2, "0")).join("");
}

export class ServerBackend {
  constructor(base = "", token = null) {
    this.kind = "server";
    this.base = base.replace(/\/$/, "");
    this.token = token;
  }
  headers(extra = {}) {
    return this.token ? { Authorization: `Bearer ${this.token}`, ...extra } : extra;
  }
  url(p) {
    return `${this.base}/api/files/${cleanPath(p).split("/").map(encodeURIComponent).join("/")}`;
  }
  async check(r, what) {
    if (r.ok) return r;
    let body = {};
    try {
      body = await r.json();
    } catch {}
    if (r.status === 412) throw new StaleBase(what, body.etag ?? null);
    throw new Error(`${what}: ${r.status} ${body.error ?? r.statusText}`);
  }
  async info() {
    const r = await fetch(`${this.base}/api/info`, { headers: this.headers() });
    return (await this.check(r, "info")).json();
  }
  async list() {
    const r = await fetch(`${this.base}/api/files`, { headers: this.headers() });
    const j = await (await this.check(r, "list")).json();
    return j.files.map((f) => ({ path: f.path, version: f.etag, size: f.size }));
  }
  async read(p) {
    const r = await this.check(await fetch(this.url(p), { headers: this.headers() }), p);
    return { bytes: new Uint8Array(await r.arrayBuffer()), version: r.headers.get("ETag") };
  }
  async write(p, bytes, base) {
    const h = {};
    if (base === null) h["If-None-Match"] = "*";
    else if (base !== undefined) h["If-Match"] = base;
    const r = await fetch(this.url(p), { method: "PUT", headers: this.headers(h), body: bytes });
    return (await (await this.check(r, p)).json()).etag;
  }
  async remove(p, base) {
    const h = base ? { "If-Match": base } : {};
    await this.check(await fetch(this.url(p), { method: "DELETE", headers: this.headers(h) }), p);
  }
  async rename(from, to) {
    const r = await fetch(`${this.base}/api/rename`, {
      method: "POST",
      headers: this.headers({ "Content-Type": "application/json" }),
      body: JSON.stringify({ from: cleanPath(from), to: cleanPath(to) }),
    });
    return (await (await this.check(r, from)).json()).etag;
  }
}

/// The Origin Private File System, under one folder per project.
export class OpfsBackend {
  constructor(root) {
    this.kind = "opfs";
    this.root = root;
  }
  static async open(project = "default") {
    const top = await navigator.storage.getDirectory();
    const projects = await top.getDirectoryHandle("projects", { create: true });
    return new OpfsBackend(await projects.getDirectoryHandle(project, { create: true }));
  }
  async dirOf(p, create) {
    const parts = cleanPath(p).split("/");
    let d = this.root;
    for (const s of parts.slice(0, -1)) d = await d.getDirectoryHandle(s, { create });
    return [d, parts.at(-1)];
  }
  async list() {
    const out = [];
    const walk = async (d, pre) => {
      for await (const [name, h] of d.entries()) {
        if (h.kind === "directory") await walk(h, `${pre}${name}/`);
        else {
          const f = await h.getFile();
          out.push({ path: `${pre}${name}`, version: `${f.size}-${f.lastModified}`, size: f.size });
        }
      }
    };
    await walk(this.root, "");
    return out;
  }
  async read(p) {
    const [d, name] = await this.dirOf(p, false);
    const f = await (await d.getFileHandle(name)).getFile();
    const bytes = new Uint8Array(await f.arrayBuffer());
    return { bytes, version: await digest(bytes) };
  }
  async write(p, bytes, base) {
    const [d, name] = await this.dirOf(p, true);
    if (base !== undefined) {
      let now = null;
      try {
        now = (await this.read(p)).version;
      } catch {}
      if (now !== base) throw new StaleBase(p, now);
    }
    const w = await (await d.getFileHandle(name, { create: true })).createWritable();
    await w.write(bytes);
    await w.close();
    return digest(bytes);
  }
  async remove(p) {
    const [d, name] = await this.dirOf(p, false);
    await d.removeEntry(name, { recursive: true });
  }
  async rename(from, to) {
    const { bytes } = await this.read(from);
    const v = await this.write(to, bytes);
    await this.remove(from);
    return v;
  }
}

/// IndexedDB: one object store of path -> bytes per project.
export class IdbBackend {
  constructor(db, store) {
    this.kind = "idb";
    this.db = db;
    this.store = store;
  }
  static open(project = "default") {
    return new Promise((ok, fail) => {
      const r = indexedDB.open("forgecad", 1);
      r.onupgradeneeded = () => r.result.createObjectStore("files");
      r.onsuccess = () => ok(new IdbBackend(r.result, project));
      r.onerror = () => fail(r.error);
    });
  }
  key(p) {
    return `${this.store}/${cleanPath(p)}`;
  }
  tx(mode, f) {
    return new Promise((ok, fail) => {
      const t = this.db.transaction("files", mode);
      const got = f(t.objectStore("files"));
      t.oncomplete = () => ok(got?.result);
      t.onerror = () => fail(t.error);
    });
  }
  async list() {
    const keys = await this.tx("readonly", (s) => s.getAllKeys());
    const pre = `${this.store}/`;
    return keys.filter((k) => k.startsWith(pre)).map((k) => ({ path: k.slice(pre.length), version: null }));
  }
  async read(p) {
    const bytes = await this.tx("readonly", (s) => s.get(this.key(p)));
    if (!bytes) throw new Error(`${p}: no such file`);
    return { bytes: new Uint8Array(bytes), version: await digest(bytes) };
  }
  async write(p, bytes, base) {
    if (base !== undefined) {
      let now = null;
      try {
        now = (await this.read(p)).version;
      } catch {}
      if (now !== base) throw new StaleBase(p, now);
    }
    await this.tx("readwrite", (s) => s.put(bytes.slice(), this.key(p)));
    return digest(bytes);
  }
  async remove(p) {
    await this.tx("readwrite", (s) => s.delete(this.key(p)));
  }
  async rename(from, to) {
    const { bytes } = await this.read(from);
    const v = await this.write(to, bytes);
    await this.remove(from);
    return v;
  }
}

/// The browser's own storage: OPFS where it works, else IndexedDB.
export async function localBackend(project = "default") {
  try {
    if (navigator.storage?.getDirectory) return await OpfsBackend.open(project);
  } catch {}
  return IdbBackend.open(project);
}

/// Keeps a backend in step with the app's filesystem: what the app writes under `mount` goes to
/// the backend in order, each write made against the version last read or written, so a file
/// changed meanwhile elsewhere is refused (StaleBase) rather than overwritten.
export class Mirror {
  constructor(backend, mount, fc, onError) {
    this.backend = backend;
    this.mount = mount.replace(/\/$/, "");
    this.fc = fc;
    this.onError = onError;
    this.versions = new Map();
    this.queue = Promise.resolve();
    this.pending = 0;
  }
  /// Every file of the backend into the app's filesystem.
  async load() {
    const files = await this.backend.list();
    for (const f of files) {
      const { bytes, version } = await this.backend.read(f.path);
      this.versions.set(f.path, version);
      this.fc.fs_load(`${this.mount}/${f.path}`, bytes);
    }
    return files.map((f) => f.path);
  }
  owns(path) {
    return path.startsWith(`${this.mount}/`);
  }
  /// The app wrote (bytes) or removed (null) a file.
  changed(path, bytes) {
    const rel = path.slice(this.mount.length + 1);
    const copy = bytes ? bytes.slice() : null;
    this.pending += 1;
    this.queue = this.queue
      .then(async () => {
        const base = this.versions.has(rel) ? this.versions.get(rel) : null;
        if (copy) {
          this.versions.set(rel, await this.backend.write(rel, copy, base ?? null));
        } else if (this.versions.has(rel)) {
          await this.backend.remove(rel, base);
          this.versions.delete(rel);
        } else {
          // a directory: everything under it
          for (const k of [...this.versions.keys()].filter((k) => k.startsWith(`${rel}/`))) {
            await this.backend.remove(k, this.versions.get(k));
            this.versions.delete(k);
          }
        }
      })
      .catch((e) => this.onError(e))
      .finally(() => (this.pending -= 1));
  }
  /// Resolves when every change so far is kept.
  settled() {
    return this.queue;
  }
  /// A file read again from the backend (after a StaleBase), into the app.
  async reload(rel) {
    const { bytes, version } = await this.backend.read(rel);
    this.versions.set(rel, version);
    this.fc.fs_load(`${this.mount}/${rel}`, bytes);
  }
}
