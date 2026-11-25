#!/usr/bin/env python3
"""Op-log storage benchmark for research/hosted/DESIGN.md (throwaway; measures, decides nothing alone).

Workload (from real forgeCAD typed ops, see MEASUREMENTS.md): one row per accepted edit op
  (doc, seq, author, base_version, op_json ~= the sizes of real ops), every append DURABLE before
  the client is acked (the edit must survive a crash once a collaborator may have seen it).
Measures, per candidate, on one disk:
  append_seq   one writer, N appends, latency p50/p99 (each a durable commit)
  append_conc  W writers on W different docs at once: total ops/s and p99 (group commit shows here)
  read_since   a doc with H ops: read the ops after seq H-K (a client catching up), p50/p99
  snapshot     write one snapshot blob (the doc text + metadata, S bytes), then read latest
               snapshot + ops since (the "open a doc" read), p50/p99
Usage: oplog_bench.py OUTDIR [candidates...]
"""
import json, os, random, statistics, subprocess, sys, threading, time

OUT = sys.argv[1]
os.makedirs(OUT, exist_ok=True)
N_SEQ = int(os.environ.get("N_SEQ", 1000))
W = int(os.environ.get("W", 16))
N_CONC = int(os.environ.get("N_CONC", 200))
H = int(os.environ.get("H", 5000))
K = int(os.environ.get("K", 200))
SNAP = int(os.environ.get("SNAP", 200_000))
REPS = int(os.environ.get("REPS", 200))

random.seed(1)
OPS = [
    '{"op":"SetParam","param":"t","value":{"value":3.15,"dim":"Length"}}',
    '{"op":"SetParam","param":"top.r","value":{"value":8.4,"dim":"Length"}}',
    '{"op":"AddFeature","before":null,"text":"edge = chamfer(edges(of: base, along: Z), 1)"}',
    '{"op":"Rename","from":"round","to":"blend"}',
    '{"op":"ReplaceFeature","id":"bolts","text":"bolts = pattern(of: bolt, layout: lin(along: plan.outline[0].dir, n: count(3), step: step))"}',
    '{"op":"MoveFeature","id":"round","before":"bolts"}',
]
def op(i):
    return json.dumps({"author": "u%d" % (i % 7), "base": "%016x" % random.getrandbits(64), "op": OPS[i % len(OPS)]}).encode()

def pct(xs, p):
    xs = sorted(xs); return xs[min(len(xs) - 1, int(p / 100 * len(xs)))]

def summary(xs):
    return {"n": len(xs), "p50_ms": round(pct(xs, 50) * 1e3, 3), "p99_ms": round(pct(xs, 99) * 1e3, 3), "mean_ms": round(statistics.mean(xs) * 1e3, 3)}

# ---------------------------------------------------------------- candidates
class Postgres:
    name = "postgres16 (synchronous_commit=on, fsync=on)"
    def __init__(self):
        import psycopg2
        self.pg = psycopg2
        c = self.conn(); cur = c.cursor()
        cur.execute("drop table if exists ops; drop table if exists snaps;")
        cur.execute("create table ops(doc bigint, seq bigint, payload bytea, primary key(doc, seq))")
        cur.execute("create table snaps(doc bigint, seq bigint, payload bytea, primary key(doc, seq))")
        c.commit(); c.close()
    def conn(self):
        return self.pg.connect(host="/var/tmp", port=5433, user="postgres", dbname="postgres")
    def writer(self):
        c = self.conn()
        cur = c.cursor()
        def append(doc, seq, p):
            cur.execute("insert into ops values (%s,%s,%s)", (doc, seq, p)); c.commit()
        return append
    def reader(self):
        c = self.conn(); c.autocommit = True; cur = c.cursor()
        def since(doc, seq):
            cur.execute("select seq, payload from ops where doc=%s and seq>%s order by seq", (doc, seq)); return cur.fetchall()
        def snap_w(doc, seq, b):
            cur.execute("insert into snaps values (%s,%s,%s)", (doc, seq, b))
        def snap_r(doc):
            cur.execute("select seq, payload from snaps where doc=%s order by seq desc limit 1", (doc,)); s = cur.fetchone()
            return s, since(doc, s[0])
        return since, snap_w, snap_r

class Sqlite:
    def __init__(self, sync="FULL", mod="sqlite3"):
        self.sync, self.mod = sync, mod
        self.name = ("libsql-embedded" if mod == "libsql" else "sqlite %s" % __import__("sqlite3").sqlite_version) + " WAL synchronous=%s" % sync
        self.path = os.path.join(OUT, "ops-%s-%s.db" % (mod, sync))
        for s in ("", "-wal", "-shm"):
            try: os.remove(self.path + s)
            except FileNotFoundError: pass
        c = self.conn()
        c.execute("create table ops(doc integer, seq integer, payload blob, primary key(doc, seq)) without rowid")
        c.execute("create table snaps(doc integer, seq integer, payload blob, primary key(doc, seq)) without rowid")
        c.commit()
        self.lock = threading.Lock()  # SQLite: one writer at a time (the doc server's single writer)
    def conn(self):
        if self.mod == "libsql":
            import libsql_experimental as m
        else:
            import sqlite3 as m
        c = m.connect(self.path, check_same_thread=False) if self.mod == "sqlite3" else m.connect(self.path)
        c.execute("pragma journal_mode=wal"); c.execute("pragma synchronous=%s" % self.sync)
        return c
    def writer(self):
        c = self.conn()
        def append(doc, seq, p):
            with self.lock:
                c.execute("insert into ops values (?,?,?)", (doc, seq, p)); c.commit()
        return append
    def reader(self):
        c = self.conn()
        def since(doc, seq):
            return c.execute("select seq, payload from ops where doc=? and seq>? order by seq", (doc, seq)).fetchall()
        def snap_w(doc, seq, b):
            with self.lock:
                c.execute("insert into snaps values (?,?,?)", (doc, seq, b)); c.commit()
        def snap_r(doc):
            s = c.execute("select seq, payload from snaps where doc=? order by seq desc limit 1", (doc,)).fetchone()
            return s, since(doc, s[0])
        return since, snap_w, snap_r

class FileLog:
    """One append-only file per doc: [u32 len][u64 seq][payload], fdatasync per append (or one
    fdatasync per batch in group mode, a single flusher thread). An in-memory index of offsets
    per doc (what a doc server keeps while the doc is open) serves read_since."""
    def __init__(self, group=False):
        self.group = group
        self.name = "append-only file per doc, fdatasync " + ("per batch (group commit)" if group else "per append")
        self.dir = os.path.join(OUT, "filelog-%d" % group)
        subprocess.run(["rm", "-rf", self.dir]); os.makedirs(self.dir)
        self.fds, self.index, self.lock = {}, {}, threading.Lock()
        if group:
            self.pending, self.cv = [], threading.Condition()
            threading.Thread(target=self.flusher, daemon=True).start()
    def fd(self, doc):
        with self.lock:
            if doc not in self.fds:
                self.fds[doc] = os.open(os.path.join(self.dir, "%d.log" % doc), os.O_WRONLY | os.O_CREAT | os.O_APPEND, 0o644)
                self.index[doc] = []
            return self.fds[doc]
    def flusher(self):
        while True:
            with self.cv:
                while not self.pending: self.cv.wait()
                batch, self.pending = self.pending, []
            for fd in {b[0] for b in batch}: os.fdatasync(fd)
            for b in batch: b[1].set()
    def writer(self):
        def append(doc, seq, p):
            fd = self.fd(doc)
            rec = len(p).to_bytes(4, "little") + seq.to_bytes(8, "little") + p
            off = os.lseek(fd, 0, os.SEEK_END); os.write(fd, rec)
            self.index[doc].append((seq, off, len(rec)))
            if self.group:
                e = threading.Event()
                with self.cv: self.pending.append((fd, e)); self.cv.notify()
                e.wait()
            else:
                os.fdatasync(fd)
        return append
    def reader(self):
        def since(doc, seq):
            idx = self.index[doc]
            lo = next(i for i, x in enumerate(idx) if x[0] > seq) if idx[-1][0] > seq else len(idx)
            if lo == len(idx): return []
            with open(os.path.join(self.dir, "%d.log" % doc), "rb") as f:
                f.seek(idx[lo][1]); data = f.read()
            out, o = [], 0
            while o < len(data):
                n = int.from_bytes(data[o:o + 4], "little"); out.append((int.from_bytes(data[o + 4:o + 12], "little"), data[o + 12:o + 12 + n])); o += 12 + n
            return out
        def snap_w(doc, seq, b):
            p = os.path.join(self.dir, "%d.snap.%d" % (doc, seq)); t = p + ".tmp"
            with open(t, "wb") as f: f.write(b); f.flush(); os.fdatasync(f.fileno())
            os.rename(t, p); self.snaps = getattr(self, "snaps", {}); self.snaps[doc] = (seq, p)
        def snap_r(doc):
            seq, p = self.snaps[doc]
            with open(p, "rb") as f: b = f.read()
            return (seq, b), since(doc, seq)
        return since, snap_w, snap_r

class Redis:
    name = "redis 7.0 streams, appendonly yes, appendfsync always"
    def __init__(self):
        import redis
        self.dir = os.path.join(OUT, "redis"); subprocess.run(["rm", "-rf", self.dir]); os.makedirs(self.dir)
        self.proc = subprocess.Popen(["redis-server", "--port", "6390", "--dir", self.dir, "--appendonly", "yes", "--appendfsync", "always", "--save", ""], stdout=subprocess.DEVNULL)
        time.sleep(1); self.r = redis
    def writer(self):
        c = self.r.Redis(port=6390)
        def append(doc, seq, p):
            c.xadd("ops:%d" % doc, {"p": p}, id="%d-1" % seq)
        return append
    def reader(self):
        c = self.r.Redis(port=6390)
        def since(doc, seq):
            return c.xrange("ops:%d" % doc, min="(%d-1" % seq)
        def snap_w(doc, seq, b):
            c.set("snap:%d" % doc, seq.to_bytes(8, "little") + b)
        def snap_r(doc):
            v = c.get("snap:%d" % doc); seq = int.from_bytes(v[:8], "little")
            return (seq, v[8:]), since(doc, seq)
        return since, snap_w, snap_r
    def close(self):
        self.proc.terminate(); self.proc.wait()

CANDS = {
    "postgres": Postgres,
    "sqlite-full": lambda: Sqlite("FULL"),
    "sqlite-normal": lambda: Sqlite("NORMAL"),
    "libsql-full": lambda: Sqlite("FULL", "libsql"),
    "filelog": lambda: FileLog(False),
    "filelog-group": lambda: FileLog(True),
    "redis-always": Redis,
}

def run(key):
    c = CANDS[key]()
    res = {"candidate": c.name}
    # append_seq
    a = c.writer(); lat = []
    for i in range(1, N_SEQ + 1):
        t = time.perf_counter(); a(1, i, op(i)); lat.append(time.perf_counter() - t)
    res["append_seq"] = summary(lat)
    # append_conc: W writers, docs 100..100+W
    lats, lock = [], threading.Lock()
    def w(doc):
        a = c.writer(); mine = []
        for i in range(1, N_CONC + 1):
            t = time.perf_counter(); a(doc, i, op(i)); mine.append(time.perf_counter() - t)
        with lock: lats.extend(mine)
    ts = [threading.Thread(target=w, args=(100 + k,)) for k in range(W)]
    t0 = time.perf_counter(); [t.start() for t in ts]; [t.join() for t in ts]; el = time.perf_counter() - t0
    res["append_conc"] = dict(summary(lats), writers=W, ops_per_s=round(W * N_CONC / el))
    # history doc 2 with H ops (not timed), then read_since and snapshot
    a = c.writer()
    for i in range(1, H + 1): a(2, i, op(i))
    since, snap_w, snap_r = c.reader()
    lat = []
    for _ in range(REPS):
        t = time.perf_counter(); rows = since(2, H - K); lat.append(time.perf_counter() - t)
    assert len(rows) == K, (key, len(rows))
    res["read_since_%d" % K] = summary(lat)
    blob = os.urandom(SNAP)
    t = time.perf_counter(); snap_w(2, H - K, blob); res["snapshot_write_ms"] = round((time.perf_counter() - t) * 1e3, 3)
    lat = []
    for _ in range(REPS):
        t = time.perf_counter(); s, rows = snap_r(2); lat.append(time.perf_counter() - t)
    assert len(rows) == K and len(s[1]) == SNAP
    res["open_snapshot_plus_%d" % K] = summary(lat)
    if hasattr(c, "close"): c.close()
    return res

if __name__ == "__main__":
    # the disk's own floor: fdatasync of a 150-byte append
    fd = os.open(os.path.join(OUT, "floor"), os.O_WRONLY | os.O_CREAT | os.O_APPEND); lat = []
    for i in range(N_SEQ):
        t = time.perf_counter(); os.write(fd, b"x" * 150); os.fdatasync(fd); lat.append(time.perf_counter() - t)
    out = {"floor_fdatasync_150B": summary(lat), "params": dict(N_SEQ=N_SEQ, W=W, N_CONC=N_CONC, H=H, K=K, SNAP=SNAP, REPS=REPS)}
    print(json.dumps(out), flush=True)
    for k in (sys.argv[2:] or CANDS):
        r = run(k); print(json.dumps(r), flush=True)
