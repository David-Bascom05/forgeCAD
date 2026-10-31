"""Shared by every finder: judge .fc files in child processes, sign, dedupe, shrink, keep a ledger.

Judging goes through verify/forge-proof/examples/judge (one process per part), so a hang, abort or
stack overflow costs one case. Verdicts: ok, refused (a clear error), fail (kernel bug by
history::classify), wrong (built, disagrees with a known truth), unread (the file did not parse:
a generator's bug), timeout, crash, oom.
"""
import concurrent.futures as cf
import hashlib
import json
import os
import re
import resource
import signal
import subprocess
import tempfile
import time

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
JUDGE = os.environ.get("JUDGE", os.path.join(ROOT, "target/release/examples/judge"))
MEM = int(os.environ.get("FP_MEM_MB", "700")) * 1 << 20
UNREAD = ("expected", "unexpected", "unknown", "no feature", "too soon", "a word the file uses")


def limits(cpu=None):
    os.nice(15)
    resource.setrlimit(resource.RLIMIT_AS, (MEM, MEM))
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
    if cpu:
        resource.setrlimit(resource.RLIMIT_CPU, (int(cpu), int(cpu) + 5))


def judge_file(path, timeout=20.0, exe=None, env=None):
    """One part file's result as a dict with v, sig, cpu, and when it built vol, bodies, faces."""
    t = time.monotonic()
    # the limit is CPU time: at nice 15 on a busy machine wall time says nothing about the part
    p = subprocess.Popen([exe or JUDGE, path], stdout=subprocess.PIPE, stderr=subprocess.PIPE, preexec_fn=lambda: limits(timeout), text=True, env=env)
    try:
        out, err = p.communicate(timeout=timeout * 20)
    except subprocess.TimeoutExpired:
        p.kill()
        p.communicate()
        return {"v": "timeout", "sig": "timeout", "cpu": timeout}
    wall = time.monotonic() - t
    if p.returncode in (-signal.SIGXCPU, -signal.SIGKILL):
        return {"v": "timeout", "sig": "timeout", "cpu": timeout}
    if p.returncode != 0 or not out.strip():
        why = "oom" if "memory allocation" in err or "alloc" in err else "crash"
        s = f"signal {-p.returncode}" if p.returncode < 0 else f"exit {p.returncode}"
        m = re.search(r"overflowed its stack|memory allocation|panicked at [^\n]*", err)
        return {"v": why, "sig": f"{why}: {s} {m.group(0) if m else ''}".strip(), "cpu": wall}
    r = json.loads(out)
    tail = r["sig"].split(": ", 1)[-1]
    if r["v"] != "ok" and (tail.startswith(UNREAD) or "unknown parameter" in tail):
        r["v"] = "unread"
    if r["v"] == "fail" and r.get("loc"):
        r["sig"] += " @" + r["loc"]
    return r


def judge_text(src, timeout=20.0):
    with tempfile.NamedTemporaryFile("w", suffix=".fc", dir=os.environ.get("FP_TMP"), delete=True) as f:
        f.write(src)
        f.flush()
        return judge_file(f.name, timeout)


def is_failure(r):
    """A failure whatever the part's source: a kernel bug, a hang, a crash."""
    return r["v"] in ("fail", "wrong", "timeout", "crash", "oom")


def sig_of(r):
    return f'{r["v"]}: {r["sig"]}' if not r["sig"].startswith(r["v"]) else r["sig"]


def sig_id(sig):
    return hashlib.sha1(sig.encode()).hexdigest()[:10]


def pool_map(fn, items, jobs=4):
    """fn over items on threads (each runs a child), yielding results as they finish."""
    jobs = min(jobs, 4)
    with cf.ThreadPoolExecutor(jobs) as ex:
        futs = [ex.submit(fn, it) for it in items]
        for f in cf.as_completed(futs):
            yield f.result()


def blocks(src):
    """A part's droppable units: a line, or a sketch with its rows."""
    out, cur = [], None
    for l in src.splitlines():
        if not l.strip() or l.lstrip().startswith("#"):
            continue
        if cur is not None:
            cur.append(l)
            if l.strip() == "end":
                out.append("\n".join(cur))
                cur = None
        elif re.match(r"(\w+:\s*)?sketch\b", l):
            cur = [l]
        else:
            out.append(l)
    if cur:
        out.append("\n".join(cur))
    return out


NUM = re.compile(r"(?<![\w.@])-?\d+\.\d+(?:e-?\d+)?(?![\w.])")


def shorter(x):
    """Plainer numbers near x, plainest first."""
    v = float(x)
    out = []
    for c in (f"{v:.0f}", f"{v:.1f}", f"{v:.2f}", f"{v:.4f}", f"{v:.6f}"):
        if len(c) < len(x) and c not in out:
            out.append(c)
    return out


def shrink(src, want, timeout=20.0, budget=250, seconds=120.0, keep=None):
    """The fewest lines and plainest numbers that still fail with signature `want` (or for which
    keep(source) holds)."""
    t0 = time.monotonic()
    n = [0]

    def same(bs):
        n[0] += 1
        text = "\n".join(bs) + "\n"
        return keep(text) if keep else sig_of(judge_text(text, timeout)) == want

    def spent():
        return n[0] >= budget or time.monotonic() - t0 > seconds

    bs = blocks(src)
    chunk = max(len(bs) // 2, 1)
    while chunk >= 1:
        end = len(bs)
        while end > 0 and not spent():
            start = max(end - chunk, 0)
            t = bs[:start] + bs[end:]
            if t and same(t):
                bs = t
                end = min(start, len(bs))
            else:
                end = start
        chunk //= 2
    # selectors and whole loop terms a line can lose, then each number made plainer
    for i in range(len(bs)):
        k = 0
        while not spent():
            sp = spans(bs[i])
            if k >= len(sp) or len(sp) < 2:
                break
            a, b = sp[k]
            t = list(bs)
            t[i] = (bs[i][:a].rstrip() + bs[i][b:]).strip()
            if same(t):
                bs = t
            else:
                k += 1
    for i in range(len(bs)):
        pos = 0
        while not spent():
            m = NUM.search(bs[i], pos)
            if not m:
                break
            hit = None
            for c in shorter(m.group(0)):
                t = list(bs)
                t[i] = bs[i][: m.start()] + c + bs[i][m.end() :]
                if not spent() and same(t):
                    hit = t
                    pos = m.start() + len(c)
                    break
            if hit:
                bs = hit
            else:
                pos = m.end()
    return "\n".join(bs) + "\n", n[0]


def spans(line):
    out, start = [], None
    for i, c in enumerate(line):
        if c == "(":
            start = i
        elif c == ")" and start is not None:
            out.append((start, i + 1))
            start = None
    return out


class Book:
    """A run's ledger: every case's result, the CPU spent, and one shrunk repro per signature."""

    def __init__(self, out, method):
        self.out, self.method = out, method
        os.makedirs(os.path.join(out, "sigs"), exist_ok=True)
        self.path = os.path.join(out, "ledger.jsonl")
        self.log = open(self.path, "a")
        self.cpu, self.n = 0.0, 0
        self.sigs = {}
        self.counts = {}

    def add(self, name, r, src=None):
        self.n += 1
        self.cpu += r.get("cpu", 0.0)
        self.counts[r["v"]] = self.counts.get(r["v"], 0) + 1
        s = sig_of(r) if r["v"] != "ok" else ""
        self.log.write(json.dumps({"m": self.method, "name": name, "v": r["v"], "sig": s, "cpu": r.get("cpu", 0), "vol": r.get("vol")}) + "\n")
        self.log.flush()
        if s and s not in self.sigs:
            self.sigs[s] = name
            if src is not None:
                with open(os.path.join(self.out, "sigs", sig_id(s) + ".src.fc"), "w") as f:
                    f.write(f"# {s}\n# first: {self.method} {name}\n{src}")

    def shrink_all(self, want=is_failure):
        """Each failure signature's first repro, shrunk to a .fc beside it."""
        for s in sorted(self.sigs):
            i = sig_id(s)
            a = os.path.join(self.out, "sigs", i + ".src.fc")
            b = os.path.join(self.out, "sigs", i + ".fc")
            if os.path.exists(b) or not os.path.exists(a):
                continue
            if not want({"v": s.split(":")[0]}):
                continue
            src = open(a).read()
            mini, probes = shrink(src, s)
            with open(b, "w") as f:
                f.write(f"# {s}\n# first: {self.method} {self.sigs[s]}, shrunk in {probes} probes\n{mini}")

    def summary(self):
        return {"method": self.method, "cases": self.n, "cpu_s": round(self.cpu, 1), "verdicts": self.counts, "signatures": len(self.sigs)}


def judge_gen(seed, length=12, timeout=120.0, kinds=None):
    """A random history (the baseline): its failures [(sig, src)] and CPU time; a hang is a failure."""
    with tempfile.NamedTemporaryFile("r", suffix=".fc", dir=os.environ.get("FP_TMP")) as last:
        env = dict(os.environ, HISTORY_LAST=last.name)
        p = subprocess.Popen([JUDGE, "gen", str(seed), str(length)] + ([str(kinds)] if kinds else []), stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, preexec_fn=limits, text=True, env=env)
        t = time.monotonic()
        try:
            out, _ = p.communicate(timeout=timeout)
        except subprocess.TimeoutExpired:
            p.kill()
            p.communicate()
            return [("timeout: timeout", open(last.name).read())], timeout
        if p.returncode != 0:
            return [(f"crash: signal {-p.returncode}", open(last.name).read())], time.monotonic() - t
    rows = [json.loads(l) for l in out.splitlines()]
    fails = [(f'fail: {r["sig"]}', r["src"]) for r in rows if r["v"] == "fail"]
    # parser complaints mean the generator wrote a bad line, not that the kernel failed
    fails = [(s, x) for s, x in fails if not s.split(": ", 2)[-1].startswith(UNREAD)]
    return fails, rows[-1].get("cpu", 0.0)
