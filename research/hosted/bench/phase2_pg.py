#!/usr/bin/env python3
"""Phase-2 Postgres store measurements for research/hosted/DESIGN.md §17 (throwaway; measures,
decides nothing alone). Postgres 16 on a unix socket (PGHOST, PGPORT; defaults /var/tmp, 5433),
synchronous_commit=on and fsync=on unless the server says otherwise.

The schema is §17.1's: accounts, projects, members (role), documents (head_seq: the sequencer's
fence), ops (primary key (doc, seq); unique (doc, client, op_id): the retry dedup), snapshots,
jobs. Workloads:
  append       one op per transaction as the sequencer would commit it: the author's role
               checked, the head fenced (head_seq = seq - 1 -> seq), the op inserted; 1 writer
               and W writers on W docs. p50/p99 ms and ops/s.
  append_bare  the same insert alone (no role check, no fence): what the checks cost.
  retry        a retried op: the dedup lookup that answers AlreadyCommitted. p50/p99.
  fence        a second sequencer for the same doc (after a failover, a stale one): its append
               must fail; counts must be 0 accepted.
  open         a document's latest snapshot + the ops after it (K of them). p50/p99.
  queue        compute jobs: N enqueued (unique by kind + input hash), Q workers claim with
               FOR UPDATE SKIP LOCKED, complete; jobs/s, claim p50/p99, double claims (must be 0).
  wakeup       a worker idle on LISTEN; enqueue + NOTIFY in one transaction; ms until it wakes.
usage: phase2_pg.py OUT.json"""
import json, os, random, select, statistics, sys, threading, time
import psycopg2

OUT = sys.argv[1]
HOST = os.environ.get("PGHOST", "/var/tmp")
PORT = int(os.environ.get("PGPORT", 5433))
N = int(os.environ.get("N", 2000))
W = int(os.environ.get("W", 16))
N_CONC = int(os.environ.get("N_CONC", 300))
K = int(os.environ.get("K", 200))
JOBS = int(os.environ.get("JOBS", 4000))
random.seed(1)
OPS = [
    '{"op":"SetParam","param":"t","value":{"value":3.15,"dim":"Length"}}',
    '{"op":"SetParam","param":"top.r","value":{"value":8.4,"dim":"Length"}}',
    '{"op":"Rename","from":"boss","to":"boss_left"}',
    '{"op":"AddFeature","after":"cut2","text":"fillet f3 = fillet(edges: top.edges, r: 1.5)"}',
]
SNAP_TEXT = "x" * 12_000  # a 12 KB document (the corpus median is ~4 KB; §1)


def conn():
    c = psycopg2.connect(host=HOST, port=PORT, user="postgres", dbname="phase2")
    c.autocommit = False
    return c


def pct(xs, p):
    xs = sorted(xs)
    return round(xs[min(len(xs) - 1, int(p / 100 * len(xs)))] * 1e3, 3)


def summary(xs):
    return {"n": len(xs), "p50_ms": pct(xs, 50), "p99_ms": pct(xs, 99), "mean_ms": round(statistics.mean(xs) * 1e3, 3)}


SCHEMA = """
drop table if exists jobs, snapshots, ops, documents, members, projects, accounts cascade;
create table accounts (id bigserial primary key, email text unique not null, name text not null);
create table projects (id bigserial primary key, owner bigint not null references accounts, name text not null);
-- role: 0 view, 1 comment, 2 edit, 3 owner
create table members (project bigint references projects, account bigint references accounts,
                      role smallint not null, primary key (project, account));
create table documents (id bigserial primary key, project bigint not null references projects,
                        path text not null, head_seq bigint not null default 0, unique (project, path));
create table ops (doc bigint not null references documents, seq bigint not null, author bigint not null,
                  client text not null, op_id bigint not null, base bigint not null, op jsonb not null,
                  inv jsonb, text_hash bigint not null, at timestamptz not null default now(),
                  primary key (doc, seq));
create unique index ops_dedup on ops (doc, client, op_id);
create table snapshots (doc bigint not null references documents, seq bigint not null, text text not null,
                        text_hash bigint not null, primary key (doc, seq));
create table jobs (id bigserial primary key, kind text not null, input bytea not null, state smallint not null default 0,
                   worker int, lease_until timestamptz, attempts int not null default 0, result bytea,
                   unique (kind, input));
create index jobs_ready on jobs (id) where state = 0;
"""

APPEND = """
with fence as (
  update documents set head_seq = %(seq)s
   where id = %(doc)s and head_seq = %(seq)s - 1
     and exists (select 1 from members m join documents d on d.project = m.project
                  where d.id = %(doc)s and m.account = %(author)s and m.role >= 2)
  returning id)
insert into ops (doc, seq, author, client, op_id, base, op, text_hash)
select %(doc)s, %(seq)s, %(author)s, %(client)s, %(op_id)s, %(base)s, %(op)s::jsonb, %(th)s from fence
"""
BARE = """insert into ops (doc, seq, author, client, op_id, base, op, text_hash)
          values (%(doc)s, %(seq)s, %(author)s, %(client)s, %(op_id)s, %(base)s, %(op)s::jsonb, %(th)s)"""


def setup():
    c = psycopg2.connect(host=HOST, port=PORT, user="postgres", dbname="postgres")
    c.autocommit = True
    cur = c.cursor()
    cur.execute("select 1 from pg_database where datname = 'phase2'")
    if not cur.fetchone():
        cur.execute("create database phase2")
    c.close()
    c = conn()
    cur = c.cursor()
    cur.execute(SCHEMA)
    cur.execute("insert into accounts (email, name) select 'a' || i || '@example.invalid', 'a' || i from generate_series(1, 64) i")
    cur.execute("insert into projects (owner, name) select i, 'p' || i from generate_series(1, 64) i")
    cur.execute("insert into members select i, i, 3 from generate_series(1, 64) i")
    cur.execute("insert into documents (project, path) select i, 'part.fg' from generate_series(1, 64) i")
    c.commit()
    cur.execute("show synchronous_commit"); sc = cur.fetchone()[0]
    cur.execute("show fsync"); fs = cur.fetchone()[0]
    cur.execute("select version()"); v = cur.fetchone()[0]
    c.close()
    return {"synchronous_commit": sc, "fsync": fs, "version": v}


def appender(doc, n, sql, lat, client="c1"):
    c = conn()
    cur = c.cursor()
    cur.execute("select head_seq from documents where id = %s", (doc,))
    seq = cur.fetchone()[0]
    c.commit()
    for i in range(n):
        seq += 1
        a = {"doc": doc, "seq": seq, "author": doc, "client": f"a{doc}/{client}", "op_id": seq, "base": seq - 1,
             "op": random.choice(OPS), "th": random.getrandbits(63)}
        t = time.perf_counter()
        cur.execute(sql, a)
        ok = cur.rowcount
        c.commit()
        lat.append(time.perf_counter() - t)
        assert ok == 1, (doc, seq)
        if sql is BARE:
            cur.execute("update documents set head_seq = %s where id = %s", (seq, doc)); c.commit()
    c.close()


def run_conc(sql, first_doc):
    lat, ths = [], []
    t = time.perf_counter()
    for w in range(W):
        th = threading.Thread(target=appender, args=(first_doc + w, N_CONC, sql, lat))
        th.start(); ths.append(th)
    for th in ths:
        th.join()
    el = time.perf_counter() - t
    return dict(summary(lat), writers=W, ops_per_s=round(W * N_CONC / el))


def retry_and_fence():
    c = conn(); cur = c.cursor()
    lat = []
    cur.execute("select head_seq from documents where id = 1"); head = cur.fetchone()[0]; c.commit()
    for i in range(500):
        oid = random.randint(1, head)
        t = time.perf_counter()
        cur.execute("select seq from ops where doc = 1 and client = 'a1/c1' and op_id = %s", (oid,))
        r = cur.fetchone()
        c.commit()
        lat.append(time.perf_counter() - t)
        assert r is not None
    # a stale sequencer: appends at a seq the head has passed, or without edit rights
    accepted = 0
    for seq in [head, head - 5, 1]:
        cur.execute(APPEND, {"doc": 1, "seq": seq, "author": 1, "client": "a1/stale", "op_id": 10**9 + seq,
                             "base": seq - 1, "op": OPS[0], "th": 1})
        accepted += cur.rowcount
        c.commit()
    cur.execute("update members set role = 0 where project = 2 and account = 2"); c.commit()
    cur.execute("select head_seq from documents where id = 2"); h2 = cur.fetchone()[0]; c.commit()
    cur.execute(APPEND, {"doc": 2, "seq": h2 + 1, "author": 2, "client": "a2/c1", "op_id": 10**9, "base": h2,
                         "op": OPS[0], "th": 1})
    viewer_accepted = cur.rowcount
    c.commit()
    cur.execute("update members set role = 3 where project = 2 and account = 2"); c.commit()
    c.close()
    return {"retry_lookup": summary(lat), "stale_sequencer_accepted": accepted, "viewer_accepted": viewer_accepted}


def open_doc():
    c = conn(); cur = c.cursor()
    # document 1 holds the N ops of append_1: a snapshot K before its head
    cur.execute("select head_seq from documents where id = 1"); head = cur.fetchone()[0]
    k = min(K, head)
    cur.execute("insert into snapshots values (1, %s, %s, 1) on conflict do nothing", (head - k, SNAP_TEXT)); c.commit()
    lat = []
    for i in range(300):
        t = time.perf_counter()
        cur.execute("select seq, text from snapshots where doc = 1 order by seq desc limit 1")
        s, text = cur.fetchone()
        cur.execute("select seq, op from ops where doc = 1 and seq > %s order by seq", (s,))
        rows = cur.fetchall()
        c.commit()
        lat.append(time.perf_counter() - t)
        assert len(rows) == k, len(rows)
    c.close()
    return dict(summary(lat), ops_after_snapshot=k, snapshot_bytes=len(SNAP_TEXT))


CLAIM = """
update jobs set state = 1, worker = %(w)s, lease_until = now() + interval '60 s', attempts = attempts + 1
 where id = (select id from jobs where state = 0 order by id for update skip locked limit 1)
returning id"""


def queue(workers):
    c = conn(); cur = c.cursor()
    cur.execute("truncate jobs"); c.commit()
    for i in range(JOBS):
        cur.execute("insert into jobs (kind, input) values ('mesh', %s) on conflict (kind, input) do nothing",
                    (random.randbytes(16),))
    # the same input twice: one job
    cur.execute("insert into jobs (kind, input) values ('mesh', '\\x00') on conflict do nothing")
    cur.execute("insert into jobs (kind, input) values ('mesh', '\\x00') on conflict do nothing")
    c.commit()
    cur.execute("select count(*) from jobs"); total = cur.fetchone()[0]
    c.close()
    claims, done, lock = [], [], threading.Lock()

    def worker(w):
        c = conn(); cur = c.cursor()
        mine = []
        while True:
            t = time.perf_counter()
            cur.execute(CLAIM, {"w": w}); r = cur.fetchone(); c.commit()
            if r is None:
                break
            claims.append(time.perf_counter() - t)
            cur.execute("update jobs set state = 2, result = '\\x01' where id = %s and worker = %s", (r[0], w)); c.commit()
            mine.append(r[0])
        with lock:
            done.extend(mine)
        c.close()

    t = time.perf_counter()
    ths = [threading.Thread(target=worker, args=(w,)) for w in range(workers)]
    [th.start() for th in ths]; [th.join() for th in ths]
    el = time.perf_counter() - t
    return dict(summary(claims), workers=workers, jobs=total, completed=len(done),
                double_claims=len(done) - len(set(done)), jobs_per_s=round(len(done) / el))


def wakeup():
    lc = psycopg2.connect(host=HOST, port=PORT, user="postgres", dbname="phase2"); lc.autocommit = True
    lc.cursor().execute("listen jobs")
    c = conn(); cur = c.cursor()
    lat = []
    for i in range(300):
        t = time.perf_counter()
        cur.execute("insert into jobs (kind, input) values ('wake', %s) returning id", (random.randbytes(16),))
        jid = cur.fetchone()[0]
        cur.execute("select pg_notify('jobs', %s)", (str(jid),))
        c.commit()
        while True:
            if select.select([lc], [], [], 5) == ([], [], []):
                raise SystemExit("no wakeup in 5 s")
            lc.poll()
            got = [n for n in lc.notifies if n.payload == str(jid)]
            lc.notifies.clear()
            if got:
                break
        lat.append(time.perf_counter() - t)
    c.close(); lc.close()
    return summary(lat)


def main():
    res = {"server": setup(), "params": {"N": N, "W": W, "N_CONC": N_CONC, "K": K, "JOBS": JOBS}}
    lat = []
    appender(1, N, APPEND, lat)
    res["append_1"] = summary(lat)
    lat = []
    appender(4, N, BARE, lat)
    res["append_bare_1"] = summary(lat)
    res["append_W"] = run_conc(APPEND, 10)
    res["append_bare_W"] = run_conc(BARE, 40)
    res.update(retry_and_fence())
    res["open"] = open_doc()
    res["queue"] = [queue(q) for q in (1, 4, 16)]
    res["wakeup"] = wakeup()
    json.dump(res, open(OUT, "w"), indent=1)
    print(json.dumps(res, indent=1))


main()
