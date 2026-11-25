#!/usr/bin/env python3
"""Fan-out latency: a writer appends ops to one doc (durably, as oplog_bench.py does), 5 ms apart;
a subscriber learns of each through the store's own change feed and holds the op. Latency = from
the start of the append to the subscriber holding the op (includes the durable commit).
  postgres  LISTEN/NOTIFY (NOTIFY in the insert's transaction, the op in the payload)
  fdb       a watch on the doc's head key, then a range read of the new ops
  mongo     a change stream on the ops collection filtered by doc (fullDocument in the event)
usage: fanout_bench.py [candidates...]   (env DBBIN: the FDB cluster file's directory)"""
import json, os, select, statistics, sys, threading, time

N = int(os.environ.get("N", 300))
GAP = float(os.environ.get("GAP", 0.005))
OP = json.dumps({"op": "SetParam", "param": "t", "value": {"value": 3.15, "dim": "Length"}}).encode()

def pct(xs, p):
    xs = sorted(xs); return xs[min(len(xs) - 1, int(p / 100 * len(xs)))]

def run(name, subscribe, append):
    sent, got = {}, {}
    ready = threading.Event()
    t = threading.Thread(target=subscribe, args=(ready, got), daemon=True)
    t.start(); ready.wait(10); time.sleep(0.3)
    for seq in range(1, N + 1):
        sent[seq] = time.perf_counter()
        append(seq)
        time.sleep(GAP)
    deadline = time.time() + 10
    while len(got) < N and time.time() < deadline:
        time.sleep(0.05)
    lat = [(got[s] - sent[s]) * 1e3 for s in sent if s in got]
    print(json.dumps({"candidate": name, "n": len(lat), "missing": N - len(lat),
                      "p50_ms": round(pct(lat, 50), 3), "p99_ms": round(pct(lat, 99), 3), "max_ms": round(max(lat), 3)}), flush=True)

def postgres():
    import psycopg2
    def conn():
        return psycopg2.connect(host="/var/tmp", port=5433, user="postgres", dbname="postgres")
    c = conn(); cur = c.cursor()
    cur.execute("drop table if exists fops; create table fops(doc bigint, seq bigint, payload bytea, primary key(doc, seq))"); c.commit()
    def subscribe(ready, got):
        l = conn(); l.autocommit = True; lc = l.cursor(); lc.execute("LISTEN ops"); ready.set()
        while True:
            if select.select([l], [], [], 5) == ([], [], []): continue
            l.poll()
            while l.notifies:
                n = l.notifies.pop(0); seq = int(n.payload.split(":", 2)[1]); got[seq] = time.perf_counter()
    def append(seq):
        cur.execute("insert into fops values (1,%s,%s); select pg_notify('ops', '1:' || %s || ':' || %s)", (seq, OP, seq, OP.decode())); c.commit()
    run("postgres16 LISTEN/NOTIFY", subscribe, append)

def fdb_():
    import fdb
    fdb.api_version(730)
    db = fdb.open(os.path.join(os.environ["DBBIN"], "fdb.cluster"))
    db.clear_range(b"", b"\xff")
    head = fdb.tuple.pack(("h", 1))
    @fdb.transactional
    def tx(tr, seq):
        tr[fdb.tuple.pack(("o", 1, seq))] = OP
        tr[head] = seq.to_bytes(8, "little")
    @fdb.transactional
    def arm(tr):
        return tr.watch(head)
    @fdb.transactional
    def read_since(tr, seq):
        return [(fdb.tuple.unpack(kv.key)[2], kv.value) for kv in tr.get_range(fdb.tuple.pack(("o", 1, seq + 1)), fdb.tuple.pack(("o", 2)))]
    def subscribe(ready, got):
        last = 0
        w = arm(db); ready.set()
        while True:
            w.wait()
            w = arm(db)
            for seq, _ in read_since(db, last):
                got[seq] = time.perf_counter(); last = max(last, seq)
    run("foundationdb 7.3.63 watch + range read", subscribe, lambda seq: tx(db, seq))

def mongo():
    import pymongo
    from pymongo import WriteConcern
    c = pymongo.MongoClient("mongodb://127.0.0.1:27099/?directConnection=true")
    c.drop_database("fan")
    ops = c["fan"].get_collection("ops", write_concern=WriteConcern(w=1, j=True))
    ops.insert_one({"doc": 0, "seq": 0, "p": b""})
    def subscribe(ready, got):
        with c["fan"].ops.watch([{"$match": {"operationType": "insert", "fullDocument.doc": 1}}]) as st:
            ready.set()
            for ev in st:
                got[ev["fullDocument"]["seq"]] = time.perf_counter()
    run("mongodb 8.0.4 change stream", subscribe, lambda seq: ops.insert_one({"doc": 1, "seq": seq, "p": OP}))

ALL = {"postgres": postgres, "fdb": fdb_, "mongo": mongo}
for k in (sys.argv[1:] or ALL):
    ALL[k]()
