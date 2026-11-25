// A small WASI preview1 for running forge-det's wasm32-wasip1 build in a browser (and, to check
// this file itself, in Node): args, environment, clocks, random bytes, stdout/stderr by line, and
// a read-only in-memory file system holding files at their real absolute paths, with directories
// derived from them. Enough for a program that reads documents, their sibling files (an
// assembly's parts, STEP imports, fonts) and lists directories. Anything else answers ENOSYS
// and is counted, so a missing call shows up as a reported gap, never as a silent difference.
const E = { SUCCESS: 0, BADF: 8, EXIST: 20, INVAL: 28, ISDIR: 31, NOENT: 44, NOSYS: 52, NOTDIR: 54, ROFS: 69 };
const DIR = 3, REG = 4, CHR = 2;

export class Exit extends Error {
  constructor(code) { super("exit " + code); this.code = code; }
}

function norm(p) {
  const out = [];
  for (const s of p.split("/")) {
    if (s === "" || s === ".") continue;
    if (s === "..") out.pop(); else out.push(s);
  }
  return "/" + out.join("/");
}

/// files: Map(absolute path → Uint8Array). Returns { imports, start(instance) → exit code, missing }.
export function makeWasi({ args, env, files, stdout, stderr }) {
  const enc = new TextEncoder();
  const dec = new TextDecoder();
  // directories: every prefix of every file path
  const dirs = new Map([["/", new Set()]]);
  for (const f of files.keys()) {
    const parts = f.split("/").filter(Boolean);
    let p = "";
    for (let i = 0; i < parts.length; i++) {
      const parent = p || "/";
      p += "/" + parts[i];
      if (!dirs.has(parent)) dirs.set(parent, new Set());
      dirs.get(parent).add(parts[i]);
      if (i < parts.length - 1 && !dirs.has(p)) dirs.set(p, new Set());
    }
  }
  const fds = new Map();
  fds.set(3, { path: "/", dir: true, pre: true });
  let nextFd = 4;
  const out = { 1: "", 2: "" };
  const sink = { 1: stdout, 2: stderr };
  const missing = new Map();
  let inst;
  const mem = () => new DataView(inst.exports.memory.buffer);
  const bytes = () => new Uint8Array(inst.exports.memory.buffer);
  const str = (ptr, len) => dec.decode(bytes().slice(ptr, ptr + len));
  const argv = args.map((a) => enc.encode(a + "\0"));
  const envv = Object.entries(env).map(([k, v]) => enc.encode(`${k}=${v}\0`));
  const t0 = performance.now();
  const stat = (path) => {
    if (dirs.has(path)) return { type: DIR, size: 0 };
    const f = files.get(path);
    return f ? { type: REG, size: f.length } : null;
  };
  const writeStat = (ptr, s, ino) => {
    const m = mem();
    for (let i = 0; i < 64; i += 8) m.setBigUint64(ptr + i, 0n, true);
    m.setBigUint64(ptr + 8, BigInt(ino), true);
    m.setUint8(ptr + 16, s.type);
    m.setBigUint64(ptr + 24, 1n, true);
    m.setBigUint64(ptr + 32, BigInt(s.size), true);
  };
  const inoOf = (path) => { let h = 7; for (const c of path) h = (h * 31 + c.charCodeAt(0)) >>> 0; return h + 1; };
  const resolve = (dirfd, ptr, len) => {
    const d = fds.get(dirfd);
    if (!d || !d.dir) return null;
    const p = str(ptr, len);
    return norm(p.startsWith("/") ? p : d.path + "/" + p);
  };
  const iovs = (ptr, n) => {
    const m = mem();
    const v = [];
    for (let i = 0; i < n; i++) v.push([m.getUint32(ptr + 8 * i, true), m.getUint32(ptr + 8 * i + 4, true)]);
    return v;
  };
  const flush = (fd, all) => {
    let s = out[fd];
    let k;
    while ((k = s.indexOf("\n")) >= 0) { sink[fd](s.slice(0, k)); s = s.slice(k + 1); }
    if (all && s) { sink[fd](s); s = ""; }
    out[fd] = s;
  };
  const w = {
    args_sizes_get(pc, pb) { const m = mem(); m.setUint32(pc, argv.length, true); m.setUint32(pb, argv.reduce((a, b) => a + b.length, 0), true); return 0; },
    args_get(pp, pb) { const m = mem(); const b = bytes(); for (let i = 0; i < argv.length; i++) { m.setUint32(pp + 4 * i, pb, true); b.set(argv[i], pb); pb += argv[i].length; } return 0; },
    environ_sizes_get(pc, pb) { const m = mem(); m.setUint32(pc, envv.length, true); m.setUint32(pb, envv.reduce((a, b) => a + b.length, 0), true); return 0; },
    environ_get(pp, pb) { const m = mem(); const b = bytes(); for (let i = 0; i < envv.length; i++) { m.setUint32(pp + 4 * i, pb, true); b.set(envv[i], pb); pb += envv[i].length; } return 0; },
    clock_res_get(id, p) { mem().setBigUint64(p, 1000n, true); return 0; },
    clock_time_get(id, prec, p) {
      const ns = id === 0 ? BigInt(Date.now()) * 1000000n : BigInt(Math.round((performance.now() - t0) * 1e6));
      mem().setBigUint64(p, ns, true); return 0;
    },
    random_get(p, n) { const b = bytes(); for (let o = 0; o < n; o += 65536) { const k = Math.min(65536, n - o); const t = new Uint8Array(k); crypto.getRandomValues(t); b.set(t, p + o); } return 0; },
    proc_exit(code) { throw new Exit(code); },
    sched_yield() { return 0; },
    fd_write(fd, ip, n, pw) {
      let total = 0;
      for (const [p, l] of iovs(ip, n)) {
        if (fd === 1 || fd === 2) out[fd] += dec.decode(bytes().slice(p, p + l));
        else return E.BADF;
        total += l;
      }
      if (fd === 1 || fd === 2) flush(fd, false);
      mem().setUint32(pw, total, true); return 0;
    },
    fd_read(fd, ip, n, pr) {
      const f = fds.get(fd);
      if (fd === 0) { mem().setUint32(pr, 0, true); return 0; }
      if (!f) return E.BADF;
      if (f.dir) return E.ISDIR;
      let total = 0;
      for (const [p, l] of iovs(ip, n)) {
        const chunk = f.data.subarray(f.pos, f.pos + l);
        bytes().set(chunk, p); f.pos += chunk.length; total += chunk.length;
        if (chunk.length < l) break;
      }
      mem().setUint32(pr, total, true); return 0;
    },
    fd_pread(fd, ip, n, off, pr) {
      const f = fds.get(fd);
      if (!f || f.dir) return E.BADF;
      let pos = Number(off), total = 0;
      for (const [p, l] of iovs(ip, n)) {
        const chunk = f.data.subarray(pos, pos + l);
        bytes().set(chunk, p); pos += chunk.length; total += chunk.length;
        if (chunk.length < l) break;
      }
      mem().setUint32(pr, total, true); return 0;
    },
    fd_seek(fd, off, whence, pn) {
      const f = fds.get(fd);
      if (!f || f.dir) return E.BADF;
      const base = whence === 0 ? 0 : whence === 1 ? f.pos : f.data.length;
      const p = base + Number(off);
      if (p < 0) return E.INVAL;
      f.pos = p; mem().setBigUint64(pn, BigInt(p), true); return 0;
    },
    fd_tell(fd, pn) { const f = fds.get(fd); if (!f || f.dir) return E.BADF; mem().setBigUint64(pn, BigInt(f.pos), true); return 0; },
    fd_close(fd) { if (fd > 3 && fds.delete(fd)) return 0; return fd <= 3 ? 0 : E.BADF; },
    fd_fdstat_get(fd, p) {
      const m = mem();
      let type;
      if (fd <= 2) type = CHR; else { const f = fds.get(fd); if (!f) return E.BADF; type = f.dir ? DIR : REG; }
      m.setUint8(p, type); m.setUint16(p + 2, 0, true);
      m.setBigUint64(p + 8, 0xffffffffn, true); m.setBigUint64(p + 16, 0xffffffffn, true); return 0;
    },
    fd_fdstat_set_flags() { return 0; },
    fd_filestat_get(fd, p) {
      if (fd <= 2) { writeStat(p, { type: CHR, size: 0 }, fd + 1); return 0; }
      const f = fds.get(fd); if (!f) return E.BADF;
      writeStat(p, stat(f.path), inoOf(f.path)); return 0;
    },
    path_filestat_get(dirfd, flags, pp, pl, p) {
      const path = resolve(dirfd, pp, pl); if (path === null) return E.BADF;
      const s = stat(path); if (!s) return E.NOENT;
      writeStat(p, s, inoOf(path)); return 0;
    },
    fd_prestat_get(fd, p) {
      const f = fds.get(fd); if (!f || !f.pre) return E.BADF;
      const m = mem(); m.setUint8(p, 0); m.setUint32(p + 4, enc.encode(f.path).length, true); return 0;
    },
    fd_prestat_dir_name(fd, p, l) {
      const f = fds.get(fd); if (!f || !f.pre) return E.BADF;
      bytes().set(enc.encode(f.path).subarray(0, l), p); return 0;
    },
    path_open(dirfd, dirflags, pp, pl, oflags, rb, ri, fdflags, pfd) {
      const path = resolve(dirfd, pp, pl); if (path === null) return E.BADF;
      if (oflags & 1 || oflags & 8) { if (!stat(path)) return E.ROFS; }
      const s = stat(path); if (!s) return E.NOENT;
      if (oflags & 2 && s.type !== DIR) return E.NOTDIR;
      const fd = nextFd++;
      fds.set(fd, s.type === DIR ? { path, dir: true } : { path, dir: false, data: files.get(path), pos: 0 });
      mem().setUint32(pfd, fd, true); return 0;
    },
    fd_readdir(fd, buf, len, cookie, pused) {
      const f = fds.get(fd); if (!f) return E.BADF; if (!f.dir) return E.NOTDIR;
      const names = [".", "..", ...[...(dirs.get(f.path) || [])].sort()];
      const m = mem(); const b = bytes();
      let o = 0;
      for (let i = Number(cookie); i < names.length && o < len; i++) {
        const name = enc.encode(names[i]);
        const child = names[i] === "." ? f.path : names[i] === ".." ? norm(f.path + "/..") : norm(f.path + "/" + names[i]);
        const s = stat(child) || { type: DIR };
        const ent = new Uint8Array(24 + name.length);
        const dv = new DataView(ent.buffer);
        dv.setBigUint64(0, BigInt(i + 1), true);
        dv.setBigUint64(8, BigInt(inoOf(child)), true);
        dv.setUint32(16, name.length, true);
        dv.setUint8(20, s.type);
        ent.set(name, 24);
        const k = Math.min(ent.length, len - o);
        b.set(ent.subarray(0, k), buf + o); o += k;
      }
      m.setUint32(pused, o, true); return 0;
    },
    path_readlink() { return E.INVAL; },
    fd_sync() { return 0; }, fd_datasync() { return 0; }, fd_advise() { return 0; },
  };
  const imports = { wasi_snapshot_preview1: new Proxy(w, {
    get(t, name) {
      if (name in t) return t[name];
      return () => { missing.set(name, (missing.get(name) || 0) + 1); return E.NOSYS; };
    },
  }) };
  return {
    imports,
    missing,
    start(instance) {
      inst = instance;
      let code = 0;
      try { instance.exports._start(); } catch (e) { if (e instanceof Exit) code = e.code; else { flush(1, true); flush(2, true); throw e; } }
      flush(1, true); flush(2, true);
      return code;
    },
  };
}
