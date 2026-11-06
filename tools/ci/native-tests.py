#!/usr/bin/env python3
# The native suite on merge, timed: one build, then the test binaries JOBS at a time (cargo runs
# them one after another), each test's time from libtest's --report-time. Every test over BUDGET
# seconds is reported. One over LIMIT in the run is then timed again alone (--exact, one thread):
# that is a measurement under less load, not a retry, and it fails the run if still over BUDGET.
# A failing test fails at once, as does a binary that reports no result. Prints what `cargo test`
# prints for fmerge's greps: `test NAME ... FAILED`, `test result:` lines, panics.
#   tools/ci/native-tests.py [cargo test args..]     e.g. --workspace, or -p forge-model
# BUDGET=2 LIMIT=5 JOBS=4 by default (JOBS=2 on a machine shared with other builds; FORGECAD_EACH
# sets how many parts a suite sweep holds at once); SLOWEST=N lists the N slowest; SKIP="a b":
# --skip a --skip b.
import concurrent.futures, json, os, subprocess, sys, time

JOBS = int(os.environ.get("JOBS", "4"))
BUDGET = float(os.environ.get("BUDGET", "2"))
LIMIT = float(os.environ.get("LIMIT", "5"))
SLOWEST = int(os.environ.get("SLOWEST", "10"))
args = sys.argv[1:] or ["--workspace"]
build = subprocess.run(["cargo", "test", *args, "--no-run", "--message-format=json"],
                       stdout=subprocess.PIPE, text=True)
if build.returncode:
    sys.exit(build.returncode)
bins = []
for line in build.stdout.splitlines():
    m = json.loads(line)
    if m.get("reason") == "compiler-artifact" and m.get("executable") and m["profile"]["test"]:
        bins.append((m["executable"], os.path.dirname(m["manifest_path"]), m["target"]["name"]))


def libtest(exe, cwd, more):
    # the timing flags are unstable libtest ones: allowed for the run only, never the build
    env = dict(os.environ, RUSTC_BOOTSTRAP="1", CARGO_MANIFEST_DIR=cwd)
    p = subprocess.run([exe, "-Z", "unstable-options", "--format", "json", "--report-time", *more],
                       cwd=cwd, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    return p.returncode, p.stdout


def run(b):
    exe, cwd, name = b
    t = time.time()
    skip = [a for s in os.environ.get("SKIP", "").split() for a in ("--skip", s)]
    code, out = libtest(exe, cwd, skip)
    return name, exe, cwd, code, out, time.time() - t


def alone(exe, cwd, test):
    """The test's time run by itself, or None if it reported none."""
    _, out = libtest(exe, cwd, ["--exact", test, "--test-threads=1"])
    for line in out.splitlines():
        try:
            m = json.loads(line)
        except ValueError:
            continue
        if m.get("type") == "test" and m.get("name") == test and "exec_time" in m:
            return m["exec_time"]
    return None


tests, over, bad, total = [], [], 0, time.time()
counts = {"passed": 0, "failed": 0, "ignored": 0}
with concurrent.futures.ThreadPoolExecutor(JOBS) as pool:
    for name, exe, cwd, code, out, secs in pool.map(run, bins):
        suite, failed = None, False
        for line in out.splitlines():
            try:
                m = json.loads(line)
            except ValueError:
                continue
            if m.get("type") == "test" and m.get("event") in ("ok", "failed"):
                t = m.get("exec_time", 0.0)
                tests.append((t, name, m["name"]))
                if t > BUDGET and m["event"] == "ok":
                    over.append((t, name, m["name"], exe, cwd))
                if m["event"] == "failed":
                    failed = True
                    print(f"test {m['name']} ... FAILED")
                    print(m.get("stdout", ""))
            elif m.get("type") == "suite" and m.get("event") in ("ok", "failed"):
                suite = m
                for k in counts:
                    counts[k] += m.get(k, 0)
        if suite is None:
            bad += 1
            print(f"error: {os.path.basename(exe)} reported no result:\n{out[-2000:]}")
        else:
            print(f"test result: {suite['event']}. {suite.get('passed', 0)} passed; {suite.get('failed', 0)} failed; "
                  f"{suite.get('ignored', 0)} ignored; 0 measured; 0 filtered out; finished in {secs:.2f}s ({name})")
        if failed or code:
            bad += 1
tests.sort(reverse=True)
print(f"{len(bins)} binaries, {len(tests)} tests in {time.time() - total:.0f} s; the slowest:")
for t, name, test in tests[:SLOWEST]:
    print(f"  {t:7.2f} s  {name} {test}")
slow = 0
for t, name, test, exe, cwd in sorted(over, reverse=True):
    if t <= LIMIT:
        print(f"over the {BUDGET:g} s budget: {name} {test} took {t:.2f} s")
        continue
    a = alone(exe, cwd, test)
    if a is None or a > BUDGET:
        slow += 1
        took = "no time" if a is None else f"{a:.2f} s"
        print(f"error: over the {BUDGET:g} s budget: {name} {test} took {t:.2f} s in the run, {took} alone")
    else:
        print(f"over the {BUDGET:g} s budget: {name} {test} took {t:.2f} s in the run, {a:.2f} s alone")
sys.exit(1 if bad or slow or not bins else 0)
