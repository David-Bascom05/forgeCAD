#!/usr/bin/env python3
"""Real CAD programs at scale: translate each to .fg, build it with our kernel, judge it against its own source STEP,
append one compact line, delete what was fetched. Streaming and resumable: ids are sorted and cut into fixed shards
(shard k of S is ids[k::S]), a shard's results.jsonl is appended and flushed per part, and a part already there is
skipped. Every part runs in its own processes (translate, judge, measure) at nice 19 with 3 GB of address space and
a time limit, 4 at a time at most. Data stays outside the repo; inputs are read as data, nothing in them run.

  scale.py ids CORPUS > IDS                       the corpus's sorted ids (those with a program and a STEP)
  scale.py run CORPUS --shard K --of S --out RUN [--jobs 4] [--limit N]
  scale.py tally RUN.. [--tol 1e-4]               classes, untranslated causes, kernel failure causes

CORPUS: cadfs (CADFS's FeatureScript, judged against its step.zip, as run_cadfs.py does), deepcad (DeepCAD's JSON,
judged against ABC's STEP from CADFS's step_abc.zip), fusion (Fusion 360 Gallery reconstruction r1.0.1, its own
STEP; test-only, owner-approved 2026-11-23: nothing from it is committed but verdicts, causes and synthetic
repros). Where each is fetched from, and its terms: research/import/scale.md.

A part's class: untranslated (with the importer's why), match or mismatch (built, both volume and area within --tol
of the source's by the truth rule, truth.md), uncertain (the source's OCCT measures disagree), no truth (the source
STEP held no solid or would not read), refused, fail, crash, oom, timeout (the kernel, from the judge's verdict).
"""
import collections
import concurrent.futures as cf
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import time
import zipfile

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import run_cadfs  # noqa: E402  the judge and its limits, shared with the CADFS sweep
import truth_rule  # noqa: E402

DATA = os.environ.get("FORGECAD_DATA", os.path.expanduser("~/data"))
REPO = os.path.abspath(os.path.join(HERE, "..", "..", ".."))
EXE = os.environ.get("FORGECAD_EXE", os.path.join(REPO, "target", "release", "examples"))
OCCT = os.environ.get("FORGECAD_OCCT", os.path.expanduser("~/.local/share/forgecad-occt/venv/bin/python"))
HF = "https://huggingface.co/datasets/VladPyatov/CADFS/resolve/dd76a9b891a6921815fd13c0452afb2d61e5f833/cadfs/"
FUSION = "https://fusion-360-gallery-dataset.s3.us-west-2.amazonaws.com/reconstruction/r1.0.1/r1.0.1.zip"
TRANSLATE = 300  # wall seconds to translate one part (CADFS builds its prefixes to resolve queries)
TRUTH = 180  # wall seconds to read and measure one STEP
TOL = 1e-4


def remote(url, name):
    import httpzip

    return httpzip.HttpZip(url, cache=os.path.join(DATA, "idx", name + ".json"))


def stem(n):
    return os.path.splitext(os.path.basename(n))[0]


class Corpus:
    """Where a corpus's programs and STEPs are, and how a program is translated."""

    def __init__(self, name):
        self.name = name
        self.z = {}

    def _zip(self, key):
        if key not in self.z:
            if key == "fp":
                self.z[key] = zipfile.ZipFile(os.path.join(DATA, "cadfs", "featurescript_fp.zip"))
            elif key == "sk":
                self.z[key] = zipfile.ZipFile(os.path.join(DATA, "cadfs", "sketch_raw.zip"))
            elif key == "dc":
                self.z[key] = zipfile.ZipFile(os.path.join(DATA, "deepcad", "cad_json.zip"))
            elif key == "step":
                self.z[key] = remote(HF + "dataset/step.zip", "dataset_step.zip")
            elif key == "abc":
                self.z[key] = remote(HF + "raw/step_abc.zip", "raw_step_abc.zip")
            elif key == "fusion":
                self.z[key] = remote(FUSION, "fusion")
        return self.z[key]

    def _names(self, key, ext):
        z = self._zip(key)
        ns = z.namelist() if isinstance(z, zipfile.ZipFile) else z.names()
        out = {}
        for n in ns:
            if n.endswith(ext):
                out.setdefault(stem(n)[:8] if key in ("fp", "sk", "step", "abc") else stem(n), n)
        return out

    def _names_once(self, key, ext):
        if not hasattr(self, "_once"):
            self._once = {}
        if key not in self._once:
            self._once[key] = self._names(key, ext)
        return self._once[key]

    def index(self):
        if not hasattr(self, "_ix"):
            if self.name == "cadfs":
                self._ix = {"prog": self._names("fp", ".txt"), "sk": self._names("sk", ".json"), "step": self._names("step", ".step")}
            elif self.name == "deepcad":
                self._ix = {"prog": self._names("dc", ".json"), "step": self._names("abc", ".step")}
            else:
                ns = self._names("fusion", ".json")
                self._ix = {"prog": ns, "step": self._names("fusion", ".step")}
        return self._ix

    def ids(self):
        ix = self.index()
        if self.name == "cadfs":
            keep = set(json.load(open(os.path.join(DATA, "cadfs", "unique_models.json"))))
            return sorted(i for i in keep if i in ix["prog"] and i in ix["sk"] and i in ix["step"])
        return sorted(i for i in ix["prog"] if i in ix["step"])

    def fetch(self, i, d):
        """The program's files in d, and the command that translates it to OUT."""
        ix = self.index()
        if self.name == "cadfs":
            a, b = os.path.join(d, i + ".txt"), os.path.join(d, i + ".json")
            open(a, "wb").write(self._zip("fp").read(ix["prog"][i]))
            open(b, "wb").write(self._zip("sk").read(ix["sk"][i]))
            return lambda out: [sys.executable, "-I", os.path.join(HERE, "cadfs.py"), a, b, "--topo", os.path.join(EXE, "topo"), "--out", out]
        a = os.path.join(d, i + ".json")
        open(a, "wb").write(self._zip("dc" if self.name == "deepcad" else "fusion").read(ix["prog"][i]))
        more = []
        if self.name == "deepcad":
            # CADFS's FeatureScript of the same ABC model: whether DeepCAD's program is all of it
            fp = self._names_once("fp", ".txt").get(i)
            if fp:
                w = os.path.join(d, i + ".fs.txt")
                open(w, "wb").write(self._zip("fp").read(fp))
                more = ["--whole", w]
        if os.environ.get("FORGECAD_ON"):  # an experiment: fusion.ON, how near an arc's joints keep the data's centre
            more += ["--on", os.environ["FORGECAD_ON"]]
        return lambda out: [sys.executable, "-I", os.path.join(HERE, "deepcad.py"), a, "--out", out, "--source", self.name] + more

    def step(self, i, path):
        ix = self.index()
        key = {"cadfs": "step", "deepcad": "abc", "fusion": "fusion"}[self.name]
        open(path, "wb").write(self._zip(key).read(ix["step"][i]))


def measure(path):
    """The source STEP's measures (step_truth.measure), in a child of the OCCT interpreter with its limits."""
    try:
        c = subprocess.run([OCCT, "-I", os.path.join(HERE, "step_truth.py"), "--child", path], capture_output=True, text=True, timeout=TRUTH, preexec_fn=lambda: os.nice(19))
        if c.returncode == 0 and c.stdout.strip():
            return json.loads(c.stdout.strip().splitlines()[-1])
        return {"err": f"exit {c.returncode}"}
    except subprocess.TimeoutExpired:
        return {"err": "timeout"}
    except ValueError:
        return {"err": "bad output"}


def klass(r, tol=TOL):
    if r["status"] == "untranslated":
        return "untranslated"
    v = r.get("v")
    if v != "ok":
        return v or r["status"]
    if r.get("whole") is False:
        return "partial source"
    t = r.get("truth") or {}
    if t.get("err") or not t.get("solids") or not t.get("vol") or not t.get("area"):
        return "no truth"
    j = truth_rule.judged(t, tol, r.get("program_bodies"))
    if j is None:
        return "uncertain"
    ev = abs(r["vol"] - j[0]) / j[0]
    ea = abs(r["area"] - j[1]) / j[1]
    r["evol"], r["earea"] = ev, ea
    return "match" if ev <= tol and ea <= tol else "mismatch"


def one(C, i, run, tmp):
    d = tempfile.mkdtemp(prefix=i + "-", dir=tmp)
    t0 = time.time()
    try:
        cmd = C.fetch(i, d)
        part = os.path.join(d, i + ".fg")
        try:
            p = subprocess.run(cmd(part), capture_output=True, text=True, timeout=TRANSLATE, preexec_fn=run_cadfs.limits)
            r = json.loads(p.stdout)
        except subprocess.TimeoutExpired:
            r = {"status": "untranslated", "cls": "importer", "why": "translation timed out"}
        except ValueError:
            m = re.search(r"(\w+Error[^\n]*)\s*$", p.stderr or "")
            r = {"status": "untranslated", "cls": "importer", "why": "importer died: " + (m.group(1) if m else (p.stderr or "")[-80:])[:80]}
        r = {"id": i, **r}
        if r["status"] != "untranslated" and os.path.exists(part):
            j = run_cadfs.judge(os.path.join(EXE, "judge"), part)
            # n-ary regeneration is single-body by design: the sequential result stands (run_cadfs.one)
            if j["v"] == "fail" and j["sig"].startswith("nary: the part is"):
                j = dict(j, v="ok", sig="", multi=True)
            r["v"], r["sig"] = j["v"], j.get("sig", "")
            for k in ("vol", "area", "bodies", "faces", "cpu", "line", "multi"):
                if k in j:
                    r[k] = j[k]
            if r["v"] == "ok":
                sp = os.path.join(d, i + ".step")
                try:
                    C.step(i, sp)
                    t = measure(sp)
                except Exception as x:  # the fetch: counted as no truth, never as the part's fault
                    t = {"err": "fetch: " + repr(x)[:60]}
                r["truth"] = {k: t[k] for k in ("err", "solids", "copies", "vol", "area", "vol_eps", "area_eps", "vol_gk", "copy_measures", "faces") if k in t}
            r["class"] = klass(r)
            if r["class"] not in ("match", "untranslated"):
                os.makedirs(os.path.join(run, "parts"), exist_ok=True)
                shutil.copy(part, os.path.join(run, "parts", i + ".fg"))
        else:
            r["class"] = "untranslated"
    except Exception as x:
        r = {"id": i, "status": "untranslated", "cls": "sweep", "why": "sweep: " + repr(x)[:70], "class": "untranslated"}
    finally:
        shutil.rmtree(d, ignore_errors=True)
    r["t"] = round(time.time() - t0, 1)
    return r


def ids_cmd(a):
    for i in Corpus(a[0]).ids():
        print(i)


def run_cmd(a):
    opt = lambda k, dflt=None: a[a.index(k) + 1] if k in a else dflt
    C = Corpus(a[0])
    k, S = int(opt("--shard")), int(opt("--of"))
    run = opt("--out")
    os.makedirs(run, exist_ok=True)
    path = os.path.join(run, "results.jsonl")
    done = set()
    if os.path.exists(path):
        for l in open(path):
            try:
                done.add(json.loads(l)["id"])
            except ValueError:
                pass  # a line cut off by a kill: its part runs again
    ids = C.ids()[k::S]
    if opt("--limit"):
        ids = ids[: int(opt("--limit"))]
    todo = [i for i in ids if i not in done]
    json.dump({"corpus": a[0], "shard": k, "of": S, "ids": len(ids), "tools": HERE, "exe": EXE}, open(os.path.join(run, "shard.json"), "w"))
    tmp = tempfile.mkdtemp(prefix="scale-", dir=opt("--tmp"))
    log = open(path, "a")
    jobs = min(int(opt("--jobs", "4")), 4)
    n, t0 = 0, time.time()
    with cf.ThreadPoolExecutor(jobs) as ex:
        futs = {ex.submit(one, C, i, run, tmp) for i in todo[: jobs * 2]}
        rest = iter(todo[jobs * 2 :])
        while futs:
            fin, futs = cf.wait(futs, return_when=cf.FIRST_COMPLETED)
            for f in fin:
                log.write(json.dumps(f.result()) + "\n")
                log.flush()
                n += 1
                if n % 100 == 0:
                    print(f"{a[0]} shard {k}/{S}: {n + len(done)} of {len(ids)}, {n / (time.time() - t0):.2f}/s", flush=True)
                nxt = next(rest, None)
                if nxt is not None:
                    futs.add(ex.submit(one, C, nxt, run, tmp))
    shutil.rmtree(tmp, ignore_errors=True)
    open(os.path.join(run, "DONE"), "w").write(f"{len(ids)}\n")


def cause(r):
    """A kernel failure's cause: its signature with numbers and names blanked (run_cadfs.sigkey's form)."""
    s = r.get("sig", "") or ""
    s = re.sub(r"`[^`]*`", "`X`", s)
    s = re.sub(r"\"[^\"]*\"", '"X"', s)
    s = re.sub(r"-?\d+\.?\d*(?:e-?\d+)?", "N", s)
    return f"{r['class']}: {s[:120]}"


def tally_cmd(a):
    opt = lambda k, dflt=None: a[a.index(k) + 1] if k in a else dflt
    tol = float(opt("--tol", str(TOL)))
    runs = [x for x in a if not x.startswith("--") and x != opt("--tol")]
    rows = {}
    for run in runs:
        for l in open(os.path.join(run, "results.jsonl")):
            try:
                r = json.loads(l)
            except ValueError:
                continue
            rows[r["id"]] = r
    C = collections.Counter
    cl = C()
    for r in rows.values():
        r["class"] = klass(r, tol)
        cl[r["class"]] += 1
    print(f"{len(rows)} parts, match at {tol:g}\n")
    print("| class | parts |\n|---|---:|")
    for c in ("match", "mismatch", "uncertain", "no truth", "partial source", "refused", "fail", "crash", "oom", "timeout", "untranslated"):
        if cl[c]:
            print(f"| {c} | {cl[c]} |")
    for c, v in cl.items():
        if c not in ("match", "mismatch", "uncertain", "no truth", "partial source", "refused", "fail", "crash", "oom", "timeout", "untranslated"):
            print(f"| {c} | {v} |")
    print("\n| untranslated cause | parts |\n|---|---:|")
    w = C((r.get("cls", ""), re.sub(r"-?\d+\.?\d*(?:e-?\d+)?", "N", r.get("why", ""))) for r in rows.values() if r["class"] == "untranslated")
    for (k, why), v in w.most_common(25):
        print(f"| {k}: {why} | {v} |")
    print("\n| kernel cause | parts | e.g. |\n|---|---:|---|")
    s = collections.defaultdict(list)
    for r in rows.values():
        if r["class"] in ("refused", "fail", "crash", "oom", "timeout"):
            s[cause(r)].append(r["id"])
    for k, ids in sorted(s.items(), key=lambda x: -len(x[1]))[:25]:
        print(f"| {k} | {len(ids)} | {', '.join(sorted(ids)[:3])} |")
    mm = [r for r in rows.values() if r["class"] == "mismatch"]
    if mm:
        print("\n| mismatch, volume off | parts |\n|---|---:|")
        b = C("<1e-3" if r["evol"] < 1e-3 else "<1e-2" if r["evol"] < 1e-2 else "<10%" if r["evol"] < 0.1 else "big" for r in mm)
        for k in ("<1e-3", "<1e-2", "<10%", "big"):
            print(f"| {k} | {b[k]} |")


if __name__ == "__main__":
    {"ids": ids_cmd, "run": run_cmd, "tally": tally_cmd}[sys.argv[1]](sys.argv[2:])
