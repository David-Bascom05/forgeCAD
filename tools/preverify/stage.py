#!/usr/bin/env python3
"""One stage of a train's verification on an exact commit: what a verifier session runs.

    tools/preverify/stage.py STAGE SHA [--repo DIR]

STAGE: native | render | wasm | clippy | guards | gate | all-local. The commit is checked out
detached in DIR (default ~/verify), a worktree kept between trains: its target directories stay
warm, so a train's build is incremental on the last one's. Never a branch name: the sha the
conductor sent. The stage runs exactly as the train's own script runs it (TRAIN-CONDUCTOR's
t7gates.sh), whole, never a selection:
  native  tools/ci/native-tests.py --workspace (JOBS=3)
  render  forge-render: clippy (wasm32, -D warnings) and cargo test --locked (lavapipe)
  wasm    tools/ci/wasm-tests.py
  clippy  cargo clippy --workspace --all-targets --locked -- -D warnings
  guards  model-licences, removed_checks, no-fusion-gallery, no-vendor-tool-libraries
  gate    gate.sh on the commit and on its first parent's main, this machine, band.py; bits
Prints one line to send back: STAGE sha exit code, seconds, and every failing test's name, so the
conductor can bisect without running anything again. The log stays in DIR/../verify-logs/.
"""
import os, re, subprocess, sys, time

stage, sha = sys.argv[1], sys.argv[2]
repo = os.path.expanduser(sys.argv[sys.argv.index("--repo") + 1] if "--repo" in sys.argv else "~/verify")
logs = os.path.join(os.path.dirname(repo.rstrip("/")), "verify-logs")
os.makedirs(logs, exist_ok=True)
if not re.fullmatch(r"[0-9a-f]{40}", sha):
    sys.exit("stage: give the full 40-hex sha, never a branch name")
subprocess.run(["git", "-C", repo, "fetch", "-q", "origin", sha], check=True)
subprocess.run(["git", "-C", repo, "checkout", "-q", "--detach", sha], check=True)
subprocess.run(["git", "-C", repo, "clean", "-qfdx", "-e", "target", "-e", "/interface/forge-render/target"], check=True)
if subprocess.run(["git", "-C", repo, "rev-parse", "HEAD"], capture_output=True, text=True).stdout.strip() != sha:
    sys.exit("stage: checkout is not the sha")
env = dict(os.environ, CARGO_PROFILE_TEST_DEBUG="0", CARGO_PROFILE_DEV_DEBUG="0",
           CARGO_INCREMENTAL="0", JOBS=os.environ.get("JOBS", "3"))
py = sys.executable
STAGES = {
    "native": [([py, "tools/ci/native-tests.py", "--workspace"], ".")],
    "render": [(["sh", "-c", 'CLIPPY_CONF_DIR="$PWD/web/clippy" cargo clippy --lib --locked '
                 "--target wasm32-unknown-unknown -- -D warnings"], "interface/forge-render"),
               (["cargo", "test", "--locked", "--no-fail-fast"], "interface/forge-render")],
    "wasm": [([py, "tools/ci/wasm-tests.py"], ".")],
    "clippy": [(["cargo", "clippy", "--workspace", "--all-targets", "--locked", "--", "-D",
                 "warnings"], ".")],
    "guards": [([py, "tools/ci/model-licences.py", "."], "."),
               ([py, "tools/ci/removed_checks.py"], "."),
               ([py, "-I", "tools/ci/no-fusion-gallery.py"], "."),
               ([py, "-I", "tools/ci/no-vendor-tool-libraries.py"], ".")],
}
log = os.path.join(logs, f"{stage}-{sha[:12]}.log")
t, code = time.time(), 0
with open(log, "w") as fh:
    if stage == "gate":
        # the train against main measured here: main is the commit's first parent
        base = subprocess.run(["git", "-C", repo, "rev-parse", f"{sha}^1"], capture_output=True,
                              text=True, check=True).stdout.strip()
        r = subprocess.run([py, os.path.join(os.path.dirname(os.path.abspath(__file__)), "run.py"), "--base", base, "--only", "gate", "--always-gate"],
                           cwd=repo, env=env, stdout=fh, stderr=subprocess.STDOUT)
        code = r.returncode
    else:
        for cmd, cwd in STAGES[stage]:
            r = subprocess.run(cmd, cwd=os.path.join(repo, cwd), env=env, stdout=fh,
                               stderr=subprocess.STDOUT)
            code = code or r.returncode
secs = round(time.time() - t)
text = open(log, errors="replace").read()
failed = sorted(set(re.findall(r"^test (\S+) \.\.\. FAILED", text, re.M)) |
                set(re.findall(r"^---- (\S+) stdout ----", text, re.M)) |
                set(re.findall(r"^(\S+): NO RESULT", text, re.M)))
print(f"VERIFY {stage} {sha} exit {code} {secs}s" +
      (f" failed: {' '.join(failed)}" if failed else "") + f"\nlog: {log}")
sys.exit(code)
