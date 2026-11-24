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
import argparse, json, os, shutil, subprocess, sys, tempfile, time

ap = argparse.ArgumentParser()
ap.add_argument("--base", default="origin/main")
ap.add_argument("--only")
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


head, base = git("rev-parse", "HEAD"), git("rev-parse", a.base)
if subprocess.run(["git", "merge-base", "--is-ancestor", base, head]).returncode:
    sys.exit(f"pre-verify: {a.base} ({base[:8]}) is not in this branch: merge it first")
if git("status", "--porcelain", "--untracked-files=no"):
    sys.exit("pre-verify: commit first (the record names a commit)")
reads = os.path.join(cache, f"reads-{base}.json")
sel_cmd = [sys.executable, "tools/preverify/affected.py", "--base", base, "--json"]
if os.path.exists(reads):
    sel_cmd += ["--reads", reads]
sel = json.loads(subprocess.run(sel_cmd, capture_output=True, text=True, check=True).stdout)
pk = sel["workspaces"]["."]["packages"]
render = bool(sel["workspaces"]["interface/forge-render"]["packages"])
only = set(a.only.split(",")) if a.only else None
record = {"branch": head, "base": base, "reads_map": os.path.exists(reads),
          "selected": {ws: w["packages"] for ws, w in sel["workspaces"].items()},
          "gate_reached": sel["gate"], "steps": {}}
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


def ps(names):
    return [x for n in names for x in ("-p", n)]


step("guards", [[sys.executable, "tools/ci/removed_checks.py"],
                [sys.executable, "tools/ci/model-licences.py", "."],
                [sys.executable, "-I", "tools/ci/no-fusion-gallery.py"],
                [sys.executable, "-I", "tools/ci/no-vendor-tool-libraries.py"]])
test_env = {"CARGO_PROFILE_TEST_DEBUG": "0", "CARGO_PROFILE_DEV_DEBUG": "0",
            "CARGO_INCREMENTAL": "0", "JOBS": os.environ.get("JOBS", "3")}
clippy = []
if pk:
    clippy.append(["cargo", "clippy", "--locked", "--all-targets", *ps(pk), "--", "-D", "warnings"])
if render:
    clippy.append(["sh", "-c", "cd interface/forge-render && CLIPPY_CONF_DIR=\"$PWD/web/clippy\" "
                   "cargo clippy --lib --locked --target wasm32-unknown-unknown -- -D warnings"])
if clippy:
    step("clippy", clippy, env=test_env)
if pk:
    step("native", [[sys.executable, "tools/ci/native-tests.py", *ps(pk)]], env=test_env)
if render:
    step("render", [["cargo", "test", "--release", "--locked", "--no-fail-fast"]],
         cwd=os.path.join(root, "interface/forge-render"))
wasm = [n for n in pk if n != "forge-certify"]
if wasm:
    step("wasm", [["sh", "tools/test-wasm.sh", *ps(wasm)]], env=test_env)


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


if sel["gate"] and (not only or "gate" in only):
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
ok = all(s["ok"] for s in record["steps"].values())
record["ok"] = ok
rec = os.path.join(cache, f"record-{head}.json")
json.dump(record, open(rec, "w"), indent=1)
steps = " ".join(f"{k}:{'ok' if v['ok'] else 'FAIL'}" for k, v in record["steps"].items())
print(f"\nPRE-VERIFY {'GREEN' if ok else 'RED'} {head[:12]} on main {base[:12]}: {steps}; "
      f"{len(pk)} packages{' + forge-render' if render else ''}; gate "
      f"{'judged' if sel['gate'] else 'not reached'}; map {'yes' if record['reads_map'] else 'no'}"
      f"\nrecord: {rec}")
sys.exit(0 if ok else 1)
