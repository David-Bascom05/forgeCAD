#!/usr/bin/env python3
# The wasm suite on merge: one build, then the test binaries at once. Fails on a binary that reports
# no result, as tools/test-wasm.sh does (under load Node has exited 0 having run nothing).
import concurrent.futures, json, os, subprocess, sys, time

JOBS = int(os.environ.get("JOBS", "4"))
root = os.getcwd()
build = subprocess.run(
    ["cargo", "test", "--workspace", "--target", "wasm32-wasip1", "--no-run", "--message-format=json"],
    stdout=subprocess.PIPE, text=True)
if build.returncode:
    sys.exit(build.returncode)
bins = []
for line in build.stdout.splitlines():
    m = json.loads(line)
    if m.get("reason") == "compiler-artifact" and m.get("executable") and m["profile"]["test"]:
        bins.append((m["executable"], os.path.dirname(m["manifest_path"])))

def run(b):
    exe, cwd = b
    t = time.time()
    env = dict(os.environ, CARGO_MANIFEST_DIR=cwd)
    p = subprocess.run(["node", "--no-turbo-fast-api-calls", f"{root}/tools/wasi-run.mjs", exe],
                       cwd=cwd, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    return exe, p.returncode, p.stdout, time.time() - t

bad = 0
with concurrent.futures.ThreadPoolExecutor(JOBS) as pool:
    for exe, code, out, secs in pool.map(run, bins):
        result = [l for l in out.splitlines() if l.startswith("test result")]
        print(f"{secs:6.1f}s {os.path.basename(exe)}: {result[-1] if result else 'NO RESULT'}")
        if code or not result:
            bad += 1
            print(out)
print(f"{len(bins)} binaries, {bad} failed")
sys.exit(1 if bad or not bins else 0)
