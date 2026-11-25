#!/usr/bin/env python3
"""A branch's pre-verify, run by its owner before the branch joins a train (the train still runs
every check, whole, on the merged tree: this only finds a failure while it is one branch's).

    tools/preverify/run.py [--base origin/main] [--only STEP,..]   from the branch's checkout

The branch must contain BASE (merge main into it first: main only, never a train). Steps, each
on what tools/preverify/affected.py selects (the reads map of BASE's commit when there is one in
$PREVERIFY_CACHE, else the conservative selection):
  guards   removed_checks, model-licences, no-fusion-gallery, no-vendor-tool-libraries (whole)
  clippy   cargo clippy --all-targets -D warnings on the selected packages; forge-render's as CI
  native   tools/ci/native-tests.py on the selected packages
  render   forge-render's tests (release), when it is reached
  wasm     tools/test-wasm.sh on the selected packages (forge-certify excluded, as CI)
  gate     when its binaries can have changed: gate.sh on BASE and on the branch, both on this
           machine (GATE_JOBS, GATE_MEMO: main's side is a memo hit after its first count here),
           judged with tools/speed/band.py; bits must match the branch's bits.txt
Writes $PREVERIFY_CACHE/record-<branch sha>.json and prints the line to send the train conductor.
PREVERIFY_CACHE defaults to ~/.cache/forge-preverify. JOBS (tests) defaults to 3.
"""
import argparse, json, os, re, shutil, subprocess, sys, tempfile, time

ap = argparse.ArgumentParser()
ap.add_argument("--base", default="origin/main")
ap.add_argument("--only")
# (a train's gate: run whatever the selector says; a memo hit stands for an identical build)
ap.add_argument("--always-gate", action="store_true")
a = ap.parse_args()
root = subprocess.run(["git", "rev-parse", "--show-toplevel"], capture_output=True, text=True,
                      check=True).stdout.strip()
os.chdir(root)
cache = os.environ.get("PREVERIFY_CACHE", os.path.expanduser("~/.cache/forge-preverify"))
os.makedirs(cache, exist_ok=True)
gjobs = os.environ.get("GATE_JOBS", str(os.cpu_count() or 1))
memo = os.environ.get("GATE_MEMO", os.path.join(cache, "gate-memo"))


def git(*args):
    return subprocess.run(["git", *args], capture_output=True, text=True, check=True).stdout.strip()


# the wasm targets any step builds, for the pinned toolchain, before anything else (rust-toolchain.toml
# lists wasm32-wasip1 only; forge-render's clippy builds wasm32-unknown-unknown, its dependencies
# forge-par, forge-alloc, forge-num included): without it, E0463 "can't find crate for std"
WASM_UNKNOWN = subprocess.run(["rustup", "target", "add", "wasm32-unknown-unknown", "wasm32-wasip1"],
                              cwd=root, capture_output=True).returncode == 0
head, base = git("rev-parse", "HEAD"), git("rev-parse", a.base)
if subprocess.run(["git", "merge-base", "--is-ancestor", base, head]).returncode:
    sys.exit(f"pre-verify: {a.base} ({base[:8]}) is not in this branch: merge it first")
if git("status", "--porcelain", "--untracked-files=no"):
    sys.exit("pre-verify: commit first (the record names a commit)")
reads = os.path.join(cache, f"reads-{base}.json")
if not os.path.exists(reads):
    # the newest published map (branch preverify/maps, one reads-<main sha>.json per main) whose
    # main is in BASE; affected.py treats what changed since that main conservatively
    if subprocess.run(["git", "fetch", "-q", "origin", "preverify/maps"], capture_output=True).returncode == 0:
        for f in git("ls-tree", "--name-only", "FETCH_HEAD").split():
            c = f[len("reads-"):-len(".json")] if f.startswith("reads-") else ""
            if c and subprocess.run(["git", "merge-base", "--is-ancestor", c, base],
                                    capture_output=True).returncode == 0:
                n = int(git("rev-list", "--count", f"{c}..{base}"))
                if not os.path.exists(reads) or n < best:
                    best = n
                    with open(reads, "w") as fh:
                        fh.write(git("show", f"FETCH_HEAD:{f}"))
sel_cmd = [sys.executable, os.path.join(os.path.dirname(os.path.abspath(__file__)), "affected.py"), "--base", base, "--json"]
if os.path.exists(reads):
    sel_cmd += ["--reads", reads]
sel = json.loads(subprocess.run(sel_cmd, capture_output=True, text=True, check=True).stdout)
pk = sel["workspaces"]["."]["packages"]
render = bool(sel["workspaces"]["interface/forge-render"]["packages"])
only = set(a.only.split(",")) if a.only else None
record = {"branch": head, "base": base, "reads_map": os.path.exists(reads),
          "selected": {ws: w["packages"] for ws, w in sel["workspaces"].items()},
          "gate_reached": sel["gate"], "steps": {},
          # (the train squashes a branch that carries e3148c8b, else merges it --no-ff)
          "e3148c8b": subprocess.run(["git", "merge-base", "--is-ancestor", "e3148c8b", head],
                                     capture_output=True).returncode == 0}
log_dir = tempfile.mkdtemp(prefix="preverify.")


def step(name, cmds, cwd=root, env=None):
    if only and name not in only:
        return
    t, ok = time.time(), True
    log = os.path.join(log_dir, name + ".log")
    with open(log, "w") as fh:
        for c in cmds:
            r = subprocess.run(c, cwd=cwd, env=dict(os.environ, **(env or {})), stdout=fh,
                               stderr=subprocess.STDOUT)
            if r.returncode:
                ok = False
                break
    record["steps"][name] = {"ok": ok, "seconds": round(time.time() - t), "log": log}
    print(f"pre-verify {name:7s} {'ok  ' if ok else 'FAIL'} {round(time.time() - t):5d}s  {log}",
          flush=True)


BASE_WT = os.path.join(cache, "base-src")          # main's checkout for comparisons: one fixed path
BASE_TARGET = os.path.join(cache, "base-target")   # and its own build folder, never the branch's


def base_checkout():
    """BASE checked out at BASE_WT (a fixed path, so its builds stay incremental from one run to
    the next). Never the branch's target directory: cargo names a workspace crate's build by its
    path in the workspace, not the checkout's, so main built into the branch's folder replaced the
    branch's builds, with main's paths (env!/CARGO_MANIFEST_DIR) compiled in: false NotFounds."""
    subprocess.run(["git", "worktree", "remove", "--force", BASE_WT], capture_output=True)
    shutil.rmtree(BASE_WT, ignore_errors=True)
    subprocess.run(["git", "worktree", "prune"], capture_output=True)
    subprocess.run(["git", "worktree", "add", "-q", "--detach", BASE_WT, base], check=True)
    return BASE_WT


def base_env():
    env = dict(os.environ, **test_env)
    env["CARGO_TARGET_DIR"] = BASE_TARGET
    return env


def free_gb():
    return shutil.disk_usage(root).free / 2**30


def make_room(need=12):
    """Free space before a heavy step: this run's own build folders, largest-and-least-reused first
    (never anything outside this checkout's targets)."""
    for d in ("interface/forge-render/target", "target/wasm32-wasip1", "target/debug/incremental",
              "target/debug"):
        if free_gb() >= need:
            return
        p = os.path.join(root, d)
        if os.path.isdir(p):
            print(f"pre-verify: {free_gb():.0f} GB free; removing {d}", flush=True)
            shutil.rmtree(p, ignore_errors=True)
    if free_gb() < need:
        print(f"pre-verify: only {free_gb():.0f} GB free (want {need}); builds may fail", flush=True)


TARGETS = {}  # test target name -> package names (root workspace)
_TOUCHED = []


def touched():
    """The root workspace's packages with a file the branch changes (not their dependents)."""
    if not _TOUCHED:
        md = json.loads(subprocess.run(["cargo", "metadata", "--format-version", "1", "--no-deps"],
                                       cwd=root, capture_output=True, text=True).stdout)
        dirs = sorted(((os.path.relpath(os.path.dirname(p["manifest_path"]), root), p["name"])
                       for p in md["packages"]), key=lambda x: -len(x[0]))
        changed = git("diff", "--name-only", f"{base}...HEAD").split()
        t = set()
        for f in changed:
            for d, n in dirs:
                if f.startswith(d + "/"):
                    t.add(n)
                    break
        _TOUCHED.append(t)
    return _TOUCHED[0]


def target_pkgs(t):
    if not TARGETS:
        md = json.loads(subprocess.run(["cargo", "metadata", "--format-version", "1", "--no-deps"],
                                       cwd=root, capture_output=True, text=True).stdout)
        for p in md["packages"]:
            for tg in p["targets"]:
                TARGETS.setdefault(tg["name"].replace("-", "_"), set()).add(p["name"])
    return TARGETS.get(t.replace("-", "_"), set())


sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from pvitems import ALONE, items, published, stays_red  # noqa: E402
SLOW_RED = 2 * float(os.environ.get("BUDGET", "2"))
# corpus guards (one whole real document per test, named corpus_guard_<doc>): their budget is 15 s
# alone, not the 2 s of a unit test (the coordinator's ruling); under it, not an overrun at all
CORPUS_PREFIX, CORPUS_BUDGET = "corpus_guard_", float(os.environ.get("CORPUS_BUDGET", "15"))


def corpus(x):
    return x[2].rsplit("::", 1)[-1].startswith(CORPUS_PREFIX)


def relative(name, cmd_for, pkgs_of):
    """A failed step judged against BASE: the same step on BASE, in a scratch worktree, for the
    packages whose tests failed; RED only for what BASE does not fail too (a test main fails, or a
    test of main's over the time budget on this machine, is not the branch's)."""
    e = record["steps"].get(name)
    if not e or e["ok"]:
        return
    text = open(e["log"], errors="replace").read()
    mine = {x for x in items(name, text) if x[0] != "slowish"}
    # only a step whose failure is wholly made of tests (failed, no result, over the budget) is
    # judged again; a build that did not compile, or a failure no test line accounts for, stays RED
    why = stays_red(name, text)
    e["build_error"] = why == "a build or compile error"
    if why:
        e["why_red"] = why
        print(f"pre-verify {name:7s} FAIL stays: {e['why_red']} (see {e['log']})", flush=True)
        return
    # a test over the time budget on this machine is judged by the budget only well past it: under
    # SLOW_RED seconds alone it is a warning (machine load moves it, and main has ~70 such tests on
    # a 4-core VM); the train's run applies the budget as ever
    # and one in a package the branch does not change (only its dependencies, or nothing) is the
    # machine's load or main's own slowness, not the branch's: a warning too. A failure is never
    mine = {x for x in mine if not (x[0] == "slow" and corpus(x) and ALONE.get(x, 1e9) <= CORPUS_BUDGET)}
    mild = {x for x in mine if x[0] == "slow" and not corpus(x) and
            (ALONE.get(x, 1e9) <= SLOW_RED or not (target_pkgs(x[1]) & touched()))}
    mild |= {x for x in mine if x[0] == "slow" and corpus(x) and not (target_pkgs(x[1]) & touched())}
    e["slow_warnings"] = sorted(" ".join(x) for x in mild)
    mine -= mild
    if not mine:
        e["ok"] = True
        print(f"pre-verify {name:7s} ok   (only {len(mild)} tests over the time budget, each under "
              f"{SLOW_RED:g} s alone or in a package the branch does not change: warnings)", flush=True)
        return
    pk_fail = set()
    for it in mine:
        pk_fail |= pkgs_of(it)
    if not mine or (name != "render" and not pk_fail):
        return  # nothing comparable (a build error, say): stays RED
    # main's own results for this commit, if a verifier published them: no rerun
    pub = published(root, f"results-{base}-{name}.json")
    if pub:
        doc = json.loads(pub)
        theirs = {tuple(x.split(" ", 2)) if x.count(" ") >= 2 else tuple(x.split(" ")) + ("",)
                  for x in doc["items"]}
        log = f"published results-{base}-{name}.json ({doc.get('toolchain', '')})"
        judge(name, e, mine, theirs, log,
              {tuple(k.split(" ", 2)): v for k, v in doc.get("alone", {}).items()})
        return
    wt = base_checkout()
    log = e["log"] + ".base"
    try:
        env = base_env()
        with open(log, "w") as fh:
            for c, d in cmd_for(sorted(pk_fail)):
                subprocess.run(c, cwd=os.path.join(wt, d), env=env, stdout=fh, stderr=subprocess.STDOUT)
    finally:
        subprocess.run(["git", "worktree", "remove", "--force", wt])
    base_alone = {}
    theirs = items(name, open(log, errors="replace").read(), base_alone)
    judge(name, e, mine, theirs, log, base_alone)


def retime(target, test):
    """The test alone (--exact, one thread), on the branch's build and on main's, back to back,
    twice each: the least seconds of each, or None where it would not run."""
    def once(cwd, env):
        pk = sorted(target_pkgs(target))
        if not pk:
            return None
        lib = target.replace("-", "_") == pk[0].replace("-", "_")
        cmd = ["cargo", "test", "-q", "--locked", "-p", pk[0]] + (["--lib"] if lib else ["--test", target]) + \
              ["--", "--exact", test, "--test-threads=1"]
        t = time.time()
        r = subprocess.run(cmd, cwd=cwd, env=env, capture_output=True)
        return time.time() - t if r.returncode == 0 else None
    benv = dict(os.environ, **test_env)
    if not os.path.isdir(BASE_WT):
        base_checkout()
    tb, tm = [], []
    for _ in range(2):
        tb.append(once(root, benv))
        tm.append(once(BASE_WT, base_env()))
    tb = [x for x in tb if x is not None]
    tm = [x for x in tm if x is not None]
    return (min(tb) if tb else None), (min(tm) if tm else None)


WORSE = 1.2  # a test of main's already over the budget: RED only if over 20% slower alone than main's


def judge(name, e, mine, theirs, log, base_alone=None):
    base_alone = base_alone or {}
    theirs = set(theirs) | {("slow", b, t) for k, b, t in theirs if k == "slowish"}
    new = sorted(mine - theirs)
    # main's own overruns (in the packages the branch changes): a warning, unless the branch made
    # one materially worse: over WORSE x main's time alone, on this machine
    worse = []
    for x in sorted(mine & theirs):
        if x[0] == "slow" and x in ALONE:
            b = base_alone.get(x, SLOW_RED / 2 if ("slowish",) + x[1:] in theirs else None)
            if b is not None and ALONE[x] > WORSE * b:
                # the two times came from runs under different load: timed again back to back,
                # branch then main, the least of two each, before it counts
                tb, tm = retime(x[1], x[2])
                if tb is not None and tm is not None and tb > WORSE * tm:
                    worse.append(f"{' '.join(x)} ({tb:.1f} s alone vs main {tm:.1f} s, back to back)")
    e["worse"] = worse
    new += worse
    e["main_identical"] = sorted(" ".join(x) for x in mine & theirs)
    e["new"] = [x if isinstance(x, str) else " ".join(x) for x in new]
    e["base_log"] = log
    if not new:
        e["ok"] = True
        print(f"pre-verify {name:7s} ok   (main-identical: {len(mine & theirs)} on main too: "
              f"{e['main_identical'][:3]})", flush=True)
    else:
        print(f"pre-verify {name:7s} FAIL ({len(new)} new, not on main: {e['new'][:3]})", flush=True)


def ps(names):
    return [x for n in names for x in ("-p", n)]


# the patent grep the train's assembly runs on each branch's added lines (TRAIN-CONDUCTOR's)
PATENT = (r"git diff {base}...HEAD -- . ':(exclude)LICENSE' ':(exclude)**/LICENSE*' | grep '^+' | "
          r"grep -iE 'patent|\bFTO\b|freedom.to.operate|design.around|counsel|\bUS [0-9]{{1,2}},[0-9]{{3}},[0-9]{{3}}' | "
          r"grep -viE 'leftover|pdftotext|technical-information--patents|Patent and trademark rights' "
          r"&& {{ echo 'patent grep: lines above need the coordinator'; exit 1; }} || exit 0")
step("guards", [["sh", "-c", PATENT.format(base=base)],
                [sys.executable, "tools/ci/removed_checks.py"],
                [sys.executable, "tools/ci/model-licences.py", "."],
                [sys.executable, "-I", "tools/ci/no-fusion-gallery.py"],
                [sys.executable, "-I", "tools/ci/no-vendor-tool-libraries.py"]])
test_env0 = {"CARGO_PROFILE_TEST_DEBUG": "0", "CARGO_PROFILE_DEV_DEBUG": "0", "CARGO_INCREMENTAL": "0"}
# every hidden input reviewed (forge-model's test scans every kernel crate): whole, every branch
step("hidden", [["cargo", "test", "--locked", "-p", "forge-model", "--test", "hidden_inputs", "--",
                 "every_hidden_input_is_reviewed"]], env=test_env0)
# a private temp folder for this run's tests and builds: two runs on one machine (owners' runs, a
# gate) never share a file under /tmp (a font one writes while the other reads, say)
private_tmp = tempfile.mkdtemp(prefix="pv-tmp.")
test_env = {"TMPDIR": private_tmp, "CARGO_PROFILE_TEST_DEBUG": "0", "CARGO_PROFILE_DEV_DEBUG": "0",
            "CARGO_INCREMENTAL": "0", "JOBS": os.environ.get("JOBS", "3")}
clippy = []
if pk:
    clippy.append(["cargo", "clippy", "--locked", "--all-targets", *ps(pk), "--", "-D", "warnings"])
# forge-render's clippy is for wasm32-unknown-unknown, which rust-toolchain.toml does not list (CI
# adds it by hand): added here for the pinned toolchain; where it cannot be, render is NOT RUN,
# not a RED (E0463 "can't find crate for std" is the machine, not the branch)
if render and not WASM_UNKNOWN:
    record["steps"]["render"] = {"ok": None, "not_run": "no wasm32-unknown-unknown target (rustup target add failed)"}
    print("pre-verify render  NOT RUN here: `rustup target add wasm32-unknown-unknown` failed; "
          "ask VERIFY-RENDER for this sha (via TRAIN-CONDUCTOR)", flush=True)
    render = False
if render:
    clippy.append(["sh", "-c", "cd interface/forge-render && CLIPPY_CONF_DIR=\"$PWD/web/clippy\" "
                   "cargo clippy --lib --locked --target wasm32-unknown-unknown -- -D warnings"])


def lints(cmd, cwd):
    """The set of clippy/rustc diagnostics a command prints: (level, code, file, message), with no
    line numbers (they move between base and branch)."""
    # without -D warnings: denied, a warning main has (in a crate the branch's crates depend on)
    # stops the build there, and the crates after it are never linted at all. Warned, cargo lints
    # every crate, and the two sides' sets compare whole
    c = [x for x in cmd]
    if c[0] == "cargo":
        c = c[:2] + ["--message-format=short"] + c[2:]
        if c[-2:] == ["-D", "warnings"]:
            c = c[:-2]
    else:
        c = ["sh", "-c", c[2].replace("cargo clippy", "cargo clippy --message-format=short")
             .replace(" -- -D warnings", "")]
    env = base_env() if cwd.startswith(BASE_WT) else dict(os.environ, **test_env)
    r = subprocess.run(c, cwd=cwd, env=env, capture_output=True, text=True)
    out = set()
    for l in (r.stdout + r.stderr).splitlines():
        m = re.match(r"^(\S+?):\d+:\d+: (warning|error)(\[\S+\])?: (.*)$", l)
        if m:
            out.add((m.group(2), m.group(3) or "", m.group(1), m.group(4)))
    return out


if clippy:
    step("clippy", clippy, env=test_env)
    clippy_log = record["steps"].get("clippy", {}).get("log")
    # a real compile error (an error code, not a lint denied by -D warnings) is never re-judged
    if clippy_log and re.search(r"error\[E\d+\]", open(clippy_log, errors="replace").read()):
        record["steps"]["clippy"]["why_red"] = "a compile error"
        print("pre-verify clippy FAIL stays: a compile error (error[E...])", flush=True)
    elif record["steps"].get("clippy", {}).get("ok") is False:
        # judged on what the branch adds: clippy on BASE with the same packages, in a scratch
        # worktree; RED only for a diagnostic BASE does not have. (A lint main has, fixed in a train
        # not yet landed, is not this branch's. The train runs clippy whole, -D warnings, as ever.)
        wt = base_checkout()
        try:
            mine, theirs = set(), set()
            for cmd in clippy:
                mine |= lints(cmd, root)
                theirs |= lints(cmd, wt)
        finally:
            subprocess.run(["git", "worktree", "remove", "--force", wt])
        new = sorted(mine - theirs)
        e = record["steps"]["clippy"]
        e["main_has"] = len(mine & theirs)
        e["new"] = [" ".join(x) for x in new]
        # (mine is never empty here: the denied run failed, so the warned one printed something,
        # unless the failure was not a lint, e.g. a build error, which stays RED)
        if mine & theirs:
            print("pre-verify clippy: main has these lints itself. They are not your branch's: do NOT "
                  "change them on your branch (some are deliberate, e.g. forge-topo body.rs's closure, "
                  "which SPINE's Aeneas translation needs; train 7 allows it):", flush=True)
            for x in sorted(mine & theirs)[:10]:
                print(f"    main's own: {x[2]}: {x[1]} {x[3]}", flush=True)
        if mine and not new:
            e["ok"] = True
            print(f"pre-verify clippy ok   (only diagnostics main has too: {len(mine & theirs)})", flush=True)
        else:
            print(f"pre-verify clippy FAIL ({len(new)} the branch adds; first: {e['new'][:1]})", flush=True)
if free_gb() < 12:
    make_room()
if pk:
    step("native", [[sys.executable, "tools/ci/native-tests.py", *ps(pk)]], env=test_env)
    relative("native", lambda p: [([sys.executable, "tools/ci/native-tests.py", *ps(p)], ".")],
             lambda it: target_pkgs(it[1]) & set(pk))
def lavapipe():
    """A Vulkan driver forge-render's tests can run on (lavapipe, mesa-vulkan-drivers)."""
    import glob
    return any(glob.glob(d + "/*.json") for d in ("/usr/share/vulkan/icd.d", "/etc/vulkan/icd.d"))


if render and not lavapipe() and (not only or "render" in only):
    # no driver here (and no permission to install one, say): render is not judged here at all.
    # Neither GREEN nor RED: the train needs its verdict from a machine that has one
    record["steps"]["render"] = {"ok": None, "not_run": "no Vulkan driver (mesa-vulkan-drivers)"}
    print("pre-verify render  NOT RUN here: no Vulkan driver. Ask VERIFY-RENDER for this sha "
          "(via TRAIN-CONDUCTOR): stage.py render <sha>", flush=True)
    render = False
if render:
    make_room()
    step("render", [["cargo", "test", "--release", "--locked", "--no-fail-fast"]],
         cwd=os.path.join(root, "interface/forge-render"))
    relative("render", lambda p: [(["cargo", "test", "--release", "--locked", "--no-fail-fast"],
                                   "interface/forge-render")], lambda it: set())
wasm = [n for n in pk if n != "forge-certify"]
if wasm:
    make_room()
    step("wasm", [["sh", "tools/test-wasm.sh", *ps(wasm)]], env=test_env)
    relative("wasm", lambda p: [(["sh", "tools/test-wasm.sh", *ps(p)], ".")],
             lambda it: {it[1]} if it[0] == "wasm" else target_pkgs(it[1]) & set(wasm))


def gate_counts(rev, out):
    """gate.sh at REV in a scratch worktree; its count table to OUT; True if its bits hold."""
    wt = tempfile.mkdtemp(prefix="pv-gate.")
    subprocess.run(["git", "worktree", "add", "-q", "--detach", wt, rev], check=True)
    try:
        r = subprocess.run(["sh", "tools/speed/gate.sh"], cwd=wt, capture_output=True, text=True,
                           env=dict(os.environ, GATE_COUNTER="callgrind", GATE_JOBS=gjobs,
                                    GATE_MEMO=memo, CARGO_TARGET_DIR=os.path.join(cache, "gate-target")))
        rows = [l.split()[:3] for l in r.stdout.splitlines()
                if len(l.split()) >= 4 and l.split()[3] == "base"]
        with open(out, "w") as fh:
            fh.writelines(f"{k} {m} {n}\n" for k, m, n in rows)
        return "bits: identical" in r.stdout and len(rows) > 0, r.stdout
    finally:
        subprocess.run(["git", "worktree", "remove", "--force", wt])


if (sel["gate"] or a.always_gate) and (not only or "gate" in only):
    t = time.time()
    mo, bo = os.path.join(log_dir, "gate-main.txt"), os.path.join(log_dir, "gate-branch.txt")
    _, main_out = gate_counts(base, mo)
    bits_ok, br_out = gate_counts(head, bo)
    band = subprocess.run([sys.executable, "tools/speed/band.py", mo, bo], capture_output=True,
                          text=True)
    log = os.path.join(log_dir, "gate.log")
    with open(log, "w") as fh:
        fh.write(main_out + "\n----\n" + br_out + "\n----\n" + band.stdout + band.stderr)
    ok = bits_ok and band.returncode == 0
    record["steps"]["gate"] = {"ok": ok, "seconds": round(time.time() - t), "log": log}
    print(f"pre-verify gate    {'ok  ' if ok else 'FAIL'} {round(time.time() - t):5d}s  {log}")
ok = all(s["ok"] is not False for s in record["steps"].values())
pending = [k for k, s in record["steps"].items() if s["ok"] is None]
record["ok"] = ok
rec = os.path.join(cache, f"record-{head}" + (f".only-{a.only.replace(',', '-')}" if a.only else "") + ".json")
json.dump(record, open(rec, "w"), indent=1)
steps = " ".join(f"{k}:{'ok' if v['ok'] else ('NOT-RUN' if v['ok'] is None else 'FAIL')}"
                 for k, v in record["steps"].items())
verdict = "RED" if not ok else ("GREEN-EXCEPT-" + "+".join(pending).upper() if pending else "GREEN")
print(f"\nPRE-VERIFY {verdict} {head[:12]} on main {base[:12]}: {steps}; "
      f"{len(pk)} packages{' + forge-render' if render else ''}; gate "
      f"{'judged' if sel['gate'] else 'not reached'}; map {'yes' if record['reads_map'] else 'no'}; "
      f"e3148c8b {'yes' if record['e3148c8b'] else 'no'}"
      f"\nrecord: {rec}")
sys.exit(0 if ok else 1)
