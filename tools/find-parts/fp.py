"""Shared by every finder: judge .fg files in child processes, sign, dedupe, shrink, keep a ledger.

Judging goes through verify/forge-proof/examples/judge (one process per part), so a hang, abort or
stack overflow costs one case. Verdicts: ok, refused (a clear error), fail (kernel bug by
forge_lang::fuzz::classify), wrong (built, disagrees with a known truth), unread (the language did
not read the program: a generator's bug), timeout, crash, oom.
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
# forge_lang::fuzz's UNREAD: error kinds that say the program is not one the language reads
UNREAD = ("Syntax", "ReservedWord", "UnknownName", "UnknownMember", "UnknownArg", "MissingArg", "TooManyArgs", "NotCallable",
          "WrongType", "WrongDim", "DimMismatch", "NotWhole", "Rebind", "ShadowsBuiltin", "ShadowsTopLevel", "ParamTwice",
          "DuplicateId", "IndexOut", "WrongPlace", "OtherSketch", "WrongEntity", "NotWith", "UnnamedFeature", "PositionalId",
          "PositionalSelection", "LooseLiteral", "WorldAfterFirst", "ParamOutOfScope", "Forms", "Conflict", "NotApplicable",
          "UnknownWord", "UnknownSize", "NotARef")


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
    if r["v"] != "ok" and tail.startswith(UNREAD):
        r["v"] = "unread"
    if r["v"] == "fail" and r.get("loc"):
        r["sig"] += " @" + r["loc"]
    return r


def judge_text(src, timeout=20.0):
    with tempfile.NamedTemporaryFile("w", suffix=".fg", dir=os.environ.get("FP_TMP"), delete=True) as f:
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
    """A part's droppable units: its statements (a line, or a sketch or call running over lines)."""
    out, cur, depth = [], [], 0
    for l in src.splitlines():
        if not cur and (not l.strip() or l.lstrip().startswith("#")):
            continue
        cur.append(l)
        body = re.sub(r'"[^"]*"', "", l.split("#")[0])
        depth += sum(body.count(c) for c in "{([") - sum(body.count(c) for c in "})]")
        if depth <= 0:
            out.append("\n".join(cur))
            cur, depth = [], 0
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
    # arguments and summed selections a statement can lose, then each number made plainer
    for i in range(len(bs)):
        k = 0
        while not spent():
            sp = spans(bs[i])
            if k >= len(sp):
                break
            a, b = sp[k]
            t = list(bs)
            t[i] = bs[i][:a].rstrip() + bs[i][b:]
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


def spans(stmt):
    """What a statement can lose and still read: an optional `, key: value` argument of a call, or one
    term of a summed selection (`+ edges(..)`)."""
    out = []
    depth = 0
    start = {}  # depth -> where the droppable piece open at that depth began
    for i, c in enumerate(stmt):
        if c in "([{":
            depth += 1
        elif c in ")]}":
            if depth in start:
                out.append((start.pop(depth), i))
            depth -= 1
        elif c == "," and depth >= 1:
            if depth in start:
                out.append((start.pop(depth), i))
            # a keyword argument after the first: `, name: value`
            if re.match(r",\s*\w+:\s", stmt[i:]):
                start[depth] = i
        elif c == "+" and depth >= 1 and re.match(r"\+\s*(edges|faces)\(", stmt[i:]):
            if depth in start:
                out.append((start.pop(depth), i))
            start[depth] = i
    return sorted(out)


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
                with open(os.path.join(self.out, "sigs", sig_id(s) + ".src.fg"), "w") as f:
                    f.write(f"# {s}\n# first: {self.method} {name}\n{src}")

    def shrink_all(self, want=is_failure):
        """Each failure signature's first repro, shrunk to a .fg beside it."""
        for s in sorted(self.sigs):
            i = sig_id(s)
            a = os.path.join(self.out, "sigs", i + ".src.fg")
            b = os.path.join(self.out, "sigs", i + ".fg")
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
    """A random program (the baseline, forge_lang::fuzz): its failures [(sig, src)] and CPU time; a hang is a failure."""
    with tempfile.NamedTemporaryFile("r", suffix=".fg", dir=os.environ.get("FP_TMP")) as last:
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
