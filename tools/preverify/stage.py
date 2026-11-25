#!/usr/bin/env python3
"""One stage of a train's verification on an exact commit: what a verifier session runs.

    tools/preverify/stage.py STAGE SHA [--repo DIR] [--publish | --emit FILE]

STAGE: native | render | wasm | clippy | guards | gate | all-local. The commit is checked out
detached in DIR (default ~/verify), a worktree kept between trains: its target directories stay
warm, so a train's build is incremental on the last one's. Never a branch name: the sha the
conductor sent. The stage runs exactly as the train's own script runs it (TRAIN-CONDUCTOR's
t7gates.sh), whole, never a selection:
  native  tools/ci/native-tests.py --workspace (JOBS=3)
  render  forge-render: clippy (wasm32, -D warnings), cargo test --release (CI's gpu job) and
          cargo test (debug, the train's), lavapipe
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
env = dict(os.environ, CARGO_PROFILE_TEST_DEBUG="0", CARGO_PROFILE_DEV_DEBUG="0", CARGO_PROFILE_RELEASE_DEBUG="0",
           CARGO_INCREMENTAL="0", JOBS=os.environ.get("JOBS", "3"))
py = sys.executable
STAGES = {
    "native": [([py, "tools/ci/native-tests.py", "--workspace"], ".")],
    "render": [(["sh", "-c", 'CLIPPY_CONF_DIR="$PWD/web/clippy" cargo clippy --lib --locked '
                 "--target wasm32-unknown-unknown -- -D warnings"], "interface/forge-render"),
               # release first, as CI's gpu job (release-only checks: studio_model's first-frame
               # time, races the debug build is too slow to show), then debug, as the train's run
               (["cargo", "test", "--release", "--locked", "--no-fail-fast"], "interface/forge-render"),
               (["cargo", "test", "--locked", "--no-fail-fast"], "interface/forge-render")],
    "wasm": [([py, "tools/ci/wasm-tests.py"], ".")],
    "clippy": [(["cargo", "clippy", "--workspace", "--all-targets", "--locked", "--", "-D",
                 "warnings"], ".")],
    "guards": [([py, "tools/ci/model-licences.py", "."], "."),
               ([py, "tools/ci/removed_checks.py"], "."),
               ([py, "-I", "tools/ci/no-fusion-gallery.py"], "."),
               ([py, "-I", "tools/ci/no-vendor-tool-libraries.py"], ".")],
}
if stage == "render":
    # its clippy builds wasm32-unknown-unknown, which rust-toolchain.toml does not list
    subprocess.run(["rustup", "target", "add", "wasm32-unknown-unknown"],
                   cwd=os.path.join(repo, "interface/forge-render"), capture_output=True)
def make_room(need=25, cap=30):
    """Under NEED GB free: whole build profiles (target/{debug,release,wasm32-*}, the root's and
    forge-render's), until there is room (a profile goes whole: cargo rebuilds it next time; a
    profile half pruned could be trusted with outputs gone). Profiles of 1 GB or more go first,
    least recently used first; small ones (forge-render's wasm32 clippy, ~0.2 GB) only after,
    since dropping them frees little and costs a rebuild. NEED is 25 GB: a cold release build of
    forge-render takes ~16 GB, so 15 left too little for one to finish. And any profile over CAP GB
    goes before any other: it holds stale outputs of earlier shas and only grows (forge-render's
    debug reached 39 GB while release, 16 GB, was the one dropped and could not be rebuilt)."""
    import shutil
    free = lambda: shutil.disk_usage(repo).free / 2**30
    if free() >= need:
        return
    def size(d):
        n = 0
        for root, _, files in os.walk(d):
            for f in files:
                try:
                    n += os.lstat(os.path.join(root, f)).st_size
                except OSError:
                    pass
        return n / 2**30
    profs = []
    for t in (os.path.join(repo, "target"), os.path.join(repo, "interface/forge-render/target")):
        for d in ([os.path.join(t, x) for x in ("debug", "release")] +
                  [os.path.join(t, x) for x in (os.listdir(t) if os.path.isdir(t) else []) if x.startswith("wasm32")]):
            if os.path.isdir(d):
                gb = size(d)
                profs.append((gb <= cap, gb < 1, os.path.getmtime(d), gb, d))
    for *_, gb, d in sorted(profs):
        if free() >= need:
            break
        why = f"over the {cap} GB cap" if gb > cap else "the least recently used large build" if gb >= 1 else "a small build"
        print(f"(only {free():.0f} GB free: removing {d}, {gb:.1f} GB, {why})", flush=True)
        shutil.rmtree(d, ignore_errors=True)


def prune_stale(text):
    """After a run: in each profile's deps/, a test executable this run ran (cargo's "Running ...
    (target/.../deps/NAME-HASH)" lines in TEXT) keeps only the copies that ran; other hashed copies
    of the same NAME are from earlier commits. Each commit leaves one more copy of every forge-render
    test binary (~130 MB each, 28 GB after three commits), none of which the next commit runs. An
    older copy goes with its .d file and its .fingerprint/ directory, so if a later commit maps to
    that hash again, cargo finds no fingerprint and rebuilds it: nothing stale is ever reused.
    Names this run did not run, and libraries, are left alone."""
    import shutil
    # (profile, file): "debug", "release" or "wasm32-.../debug", as the Running line names it
    ran = set(re.findall(r"((?:wasm32[^/\s]*/)?(?:debug|release))/deps/([^/()\s]+-[0-9a-f]{16}(?:\.wasm)?)\)", text))
    ran_names = {(p, re.sub(r"-[0-9a-f]{16}(\.wasm)?$", "", f)) for p, f in ran}
    for t in (os.path.join(repo, "target"), os.path.join(repo, "interface/forge-render/target")):
        if not os.path.isdir(t):
            continue
        profs = [os.path.join(t, "debug"), os.path.join(t, "release")] + [
            os.path.join(t, x, p) for x in os.listdir(t) if x.startswith("wasm32") for p in ("debug", "release")]
        for prof in profs:
            key = os.path.relpath(prof, t)
            deps = os.path.join(prof, "deps")
            if not os.path.isdir(deps):
                continue
            newest, freed = {}, 0
            for f in os.listdir(deps):
                m = re.fullmatch(r"(.+)-([0-9a-f]{16})(\.wasm)?", f)
                path = os.path.join(deps, f)
                if not m or not os.path.isfile(path) or not os.access(path, os.X_OK) and not m.group(3):
                    continue
                newest.setdefault(m.group(1), []).append((os.path.getmtime(path), m.group(2), f))
            for name, copies in newest.items():
                if (key, name) not in ran_names:
                    continue
                for _, h, f in copies:
                    if (key, f) in ran:
                        continue
                    freed += os.path.getsize(os.path.join(deps, f))
                    for x in (f, f"{name}-{h}.d"):
                        try:
                            os.remove(os.path.join(deps, x))
                        except OSError:
                            pass
                    shutil.rmtree(os.path.join(prof, ".fingerprint", f"{name}-{h}"), ignore_errors=True)
            if freed > 2**30:
                print(f"(pruned {freed / 2**30:.1f} GB of older test binaries from {prof})", flush=True)


make_room()
log = os.path.join(logs, f"{stage}-{sha[:12]}.log")
t, code = time.time(), 0
with open(log, "w") as fh:
    if stage == "gate":
        # the train against the main it was built on, both measured here: its merge base with main
        # (a train head is often a merge commit, whose first parent is the train, not main)
        subprocess.run(["git", "-C", repo, "fetch", "-q", "origin", "main"], check=True)
        base = subprocess.run(["git", "-C", repo, "merge-base", "FETCH_HEAD", sha],
                              capture_output=True, text=True, check=True).stdout.strip()
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
prune_stale(text)
failed = sorted(set(re.findall(r"^test (\S+) \.\.\. FAILED", text, re.M)) |
                set(re.findall(r"^---- (\S+) stdout ----", text, re.M)) |
                set(re.findall(r"^(\S+): NO RESULT", text, re.M)))
# the toolchain the checkout pins (rust-toolchain.toml), as it ran here: a verdict is stamped with it
tc = " / ".join(subprocess.run(c, cwd=repo, capture_output=True, text=True).stdout.strip()
                for c in (["rustc", "-V"], ["cargo", "clippy", "-V"]))
# this stage's items for this commit, published (branch preverify/maps), so a branch's pre-verify
# on this commit as its main compares against them instead of running main again
# (only with --publish: a push to preverify/maps, which a verifier does only when allowed to)
# (or --emit FILE: the same results written to FILE only, nothing pushed, for someone who may push
# to publish: tools/preverify/publish.py FILE results-<sha>-<stage>.json)
emit = sys.argv[sys.argv.index("--emit") + 1] if "--emit" in sys.argv else None
if stage in ("native", "render", "wasm") and ("--publish" in sys.argv or emit):
    sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
    import json
    from pvitems import ALONE, items, publish
    its = items(stage, text)
    doc = {"sha": sha, "stage": stage, "exit": code, "toolchain": tc,
           "items": sorted(" ".join(x) for x in its),
           "alone": {" ".join(k): v for k, v in ALONE.items()}}
    if emit:
        with open(emit, "w") as fh:
            json.dump(doc, fh, separators=(",", ":"))
        print(f"(results written to {emit}: results-{sha}-{stage}.json)")
    elif not publish(repo, f"results-{sha}-{stage}.json", json.dumps(doc, indent=0)):
        print("(could not publish this stage's results)")
print(f"VERIFY {stage} {sha} exit {code} {secs}s [{tc}]" +
      (f" failed: {' '.join(failed)}" if failed else "") + f"\nlog: {log}")
sys.exit(code)
