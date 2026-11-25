#!/usr/bin/env python3
"""Phase-2 replication, failover and backup measurements for research/hosted/DESIGN.md §17
(throwaway). Builds its own two Postgres 16 clusters on this machine (a primary and a streaming
standby, unix sockets under WORK, ports 5440/5441), so the network between them is loopback: a
real standby in another zone adds that zone's round trip to every synchronous commit (cited in
§17, not measured here). Runs as root and drives the clusters as the postgres user.

  commit       op append latency (one row, one transaction) with: no standby; async standby;
               synchronous standby with synchronous_commit = on (flushed there),
               remote_apply (applied there, readable on the standby).
  failover     a writer appends and records every acked seq; the primary is killed (immediate
               stop, no shutdown checkpoint); the standby is promoted; the writer reconnects to it.
               Measured: kill -> promote returns, kill -> first write accepted; acked seqs missing
               on the promoted standby (async: may be > 0; synchronous: must be 0).
  backup       pg_basebackup of the primary with D MB of ops (tar, WAL streamed): seconds, MB/s;
               then a point-in-time restore: the base backup + archived WAL replayed to a named
               restore point, seconds until it accepts reads, and rows present = rows at the point.
usage: phase2_ha.py WORK OUT.json   (WORK is created and removed)"""
import json, os, shutil, signal, statistics, subprocess, sys, threading, time
import psycopg2

WORK, OUT = sys.argv[1], sys.argv[2]
BIN = "/usr/lib/postgresql/16/bin"
P, S = 5440, 5441
MB = int(os.environ.get("MB", 512))
N = int(os.environ.get("N", 1000))


def sh(cmd, check=True):
    r = subprocess.run(["su", "postgres", "-c", cmd], capture_output=True, text=True)
    if check and r.returncode != 0:
        raise SystemExit(f"{cmd}\n{r.stdout}\n{r.stderr}")
    return r


def pg(port, db="postgres", timeout=None):
    c = psycopg2.connect(host=WORK, port=port, user="postgres", dbname=db, connect_timeout=timeout or 5)
    c.autocommit = True
    return c


def pct(xs, p):
    xs = sorted(xs)
    return round(xs[min(len(xs) - 1, int(p / 100 * len(xs)))] * 1e3, 3)


def summary(xs):
    return {"n": len(xs), "p50_ms": pct(xs, 50), "p99_ms": pct(xs, 99)}


def ctl(d, what, port, extra=""):
    # unix sockets only: a TCP listen can collide with a transient ephemeral port on this host
    return sh(f"{BIN}/pg_ctl -D {d} -o \"-p {port} -k {WORK} -c listen_addresses=''\" -l {d}.log -w {what} {extra}")


def setup():
    if os.path.exists(WORK):
        shutil.rmtree(WORK)
    os.makedirs(WORK + "/archive")
    shutil.chown(WORK, "postgres", "postgres")
    shutil.chown(WORK + "/archive", "postgres", "postgres")
    prim = WORK + "/primary"
    sh(f"{BIN}/initdb -D {prim} -A trust >/dev/null")
    with open(prim + "/postgresql.conf", "a") as f:
        f.write(f"""
wal_level = replica
max_wal_senders = 8
archive_mode = on
archive_command = 'test ! -f {WORK}/archive/%f && cp %p {WORK}/archive/%f'
synchronous_commit = on
fsync = on
""")
    with open(prim + "/pg_hba.conf", "a") as f:
        f.write("local replication all trust\n")
    ctl(prim, "start", P)
    c = pg(P)
    c.cursor().execute("create table ops (seq bigint primary key, op text not null)")
    c.close()
    return prim


def standby(prim):
    sb = WORK + "/standby"
    sh(f"{BIN}/pg_basebackup -h {WORK} -p {P} -D {sb} -R -X stream -c fast")
    with open(sb + "/postgresql.auto.conf", "a") as f:
        f.write("cluster_name = 'sb'\n")
    ctl(sb, "start", S)
    # wait until streaming
    c = pg(P)
    for _ in range(100):
        cur = c.cursor(); cur.execute("select state from pg_stat_replication")
        r = cur.fetchone()
        if r and r[0] == "streaming":
            break
        time.sleep(0.1)
    c.close()
    return sb


def commits(label, setting, start):
    c = pg(P); cur = c.cursor()
    if setting:
        cur.execute(f"set synchronous_commit = {setting}")
    lat = []
    for i in range(N):
        t = time.perf_counter()
        cur.execute("insert into ops values (%s, %s)", (start + i, '{"op":"SetParam","param":"t","value":3.15}'))
        lat.append(time.perf_counter() - t)
    c.close()
    return dict(summary(lat), mode=label)


def sync_names(v):
    c = pg(P); cur = c.cursor()
    cur.execute(f"alter system set synchronous_standby_names = '{v}'")
    cur.execute("select pg_reload_conf()")
    c.close()
    time.sleep(0.5)


def failover(prim, sb, sync):
    acked, stop, start = [], threading.Event(), 10_000_000 if sync else 20_000_000

    def writer():
        c = pg(P); cur = c.cursor()
        i = start
        while not stop.is_set():
            try:
                cur.execute("insert into ops values (%s, 'x')", (i,))
                acked.append(i)
                i += 1
            except Exception:
                return

    th = threading.Thread(target=writer); th.start()
    time.sleep(2.0)
    t0 = time.perf_counter()
    ctl(prim, "stop", P, "-m immediate")
    stop.set(); th.join()
    sh(f"{BIN}/pg_ctl -D {sb} -w promote")
    t_promoted = time.perf_counter() - t0
    first = None
    while first is None:
        try:
            c = pg(S, timeout=1); c.cursor().execute("insert into ops values (%s, 'after')", (start + 5_000_000,))
            first = time.perf_counter() - t0
            c.close()
        except Exception:
            time.sleep(0.01)
    c = pg(S); cur = c.cursor()
    cur.execute("select seq from ops where seq >= %s and seq < %s", (start, start + 5_000_000))
    have = {r[0] for r in cur.fetchall()}
    c.close()
    lost = [s for s in acked if s not in have]
    return {"sync": sync, "acked": len(acked), "acked_lost": len(lost),
            "kill_to_promoted_s": round(t_promoted, 3), "kill_to_first_write_s": round(first, 3)}


def backup(prim):
    c = pg(P); cur = c.cursor()
    # MB of ops rows (~250 B a row, as real ops are)
    rows = MB * 1024 * 1024 // 250
    cur.execute("create table if not exists bulk (seq bigint primary key, op text)")
    cur.execute("insert into bulk select i, repeat('x', 230) from generate_series(1, %s) i", (rows,))
    cur.execute("checkpoint")
    cur.execute("select pg_database_size('postgres')"); size = cur.fetchone()[0]
    c.close()
    bk = WORK + "/base"
    t = time.perf_counter()
    sh(f"{BIN}/pg_basebackup -h {WORK} -p {P} -D {bk} -Ft -z -X stream -c fast")
    t_base = time.perf_counter() - t
    # more ops after the backup, a named restore point, then more that must NOT be restored
    c = pg(P); cur = c.cursor()
    cur.execute("insert into bulk select i, 'after' from generate_series(%s, %s) i", (rows + 1, rows + 10000))
    cur.execute("select pg_create_restore_point('phase2')")
    cur.execute("insert into bulk select i, 'late' from generate_series(%s, %s) i", (rows + 10001, rows + 20000))
    cur.execute("select pg_walfile_name(pg_switch_wal())"); seg = cur.fetchone()[0]
    # every segment up to the restore point archived before the primary goes
    for _ in range(600):
        cur.execute("select last_archived_wal from pg_stat_archiver"); last = cur.fetchone()[0]
        if last and last >= seg:
            break
        time.sleep(0.1)
    c.close()
    ctl(prim, "stop", P, "-m fast")
    rs = WORK + "/restored"
    os.makedirs(rs); shutil.chown(rs, "postgres", "postgres"); os.chmod(rs, 0o700)
    t = time.perf_counter()
    sh(f"tar -xzf {bk}/base.tar.gz -C {rs} && mkdir -p {rs}/pg_wal && tar -xzf {bk}/pg_wal.tar.gz -C {rs}/pg_wal")
    with open(rs + "/postgresql.auto.conf", "a") as f:
        f.write(f"restore_command = 'cp {WORK}/archive/%f %p'\nrecovery_target_name = 'phase2'\n"
                "recovery_target_action = 'promote'\n")
    open(rs + "/recovery.signal", "w").close()
    shutil.chown(rs + "/recovery.signal", "postgres", "postgres")
    ctl(rs, "start", P)
    t_restore = time.perf_counter() - t
    c = pg(P); cur = c.cursor()
    cur.execute("select count(*), max(seq) from bulk"); n, mx = cur.fetchone()
    c.close()
    ctl(rs, "stop", P, "-m fast")
    return {"db_bytes": size, "base_backup_s": round(t_base, 2), "base_MB_per_s": round(size / 1e6 / t_base, 1),
            "restore_to_point_s": round(t_restore, 2), "rows_restored": n, "rows_wanted": rows + 10000,
            "late_rows_excluded": mx == rows + 10000}


def main():
    res = {"params": {"N": N, "MB": MB}}
    if os.environ.get("ONLY") == "backup":
        prim = setup(); res["backup"] = backup(prim)
        print(json.dumps(res, indent=1)); return
    prim = setup()
    res["commit"] = [commits("no standby", None, 0)]
    sb = standby(prim)
    res["commit"].append(commits("async standby", None, 100_000))
    sync_names("sb")
    res["commit"].append(commits("sync standby, synchronous_commit=on", "on", 200_000))
    res["commit"].append(commits("sync standby, remote_apply", "remote_apply", 300_000))
    res["failover_sync"] = failover(prim, sb, True)
    # again, asynchronous: a fresh pair
    shutil.rmtree(WORK); prim = setup(); sb = standby(prim)
    res["failover_async"] = failover(prim, sb, False)
    ctl(sb, "stop", S, "-m fast")
    shutil.rmtree(WORK); prim = setup()
    res["backup"] = backup(prim)
    shutil.rmtree(WORK)
    json.dump(res, open(OUT, "w"), indent=1)
    print(json.dumps(res, indent=1))


main()
