#!/usr/bin/env python3
"""Which files of the repository each package's tests read: the map tools/preverify/affected.py
selects with. Built on main (once per main commit, off the critical path), never on a branch.

    tools/preverify/readsmap.py OUT.json [WORKSPACE_DIR ...]   (default: . interface/forge-render
                                                                interface/forge-speech)

For each workspace: one `cargo test --workspace --no-run` build, then every test binary run as
cargo runs it (from its package's directory, CARGO_MANIFEST_DIR set) under `strace -f -e
trace=%file`, JOBS at a time. Recorded, per package, repository-relative:
  files  every file a test opened, stat'ed or executed at run time, and every file its build read
         (the binary's dep-info .d: its sources, include_str!/include_bytes! inputs)
  dirs   every directory a test opened (read_dir: a file added to or removed from one changes
         what the test sees)
Whether a test passes does not matter here: what it reads does. A binary that runs over TIMEOUT
seconds is stopped; what it read until then is kept, and the package is marked `partial` (the
selector then treats any change under the repository as reaching it: never a missed package).
JOBS=4 TIMEOUT=1800 by default. Needs strace.
"""
import concurrent.futures, json, os, re, subprocess, sys, tempfile

root = subprocess.run(["git", "rev-parse", "--show-toplevel"], capture_output=True, text=True,
                      check=True).stdout.strip()
JOBS = int(os.environ.get("JOBS", "4"))
TIMEOUT = int(os.environ.get("TIMEOUT", "1800"))
out = sys.argv[1]
workspaces = sys.argv[2:] or [".", "interface/forge-render", "interface/forge-speech"]
# a quoted path argument of a file syscall, and the cwd changes strace shows
PATH_RE = re.compile(r'^(\d+)\s+(\w+)\((?:AT_FDCWD,\s*)?"((?:[^"\\]|\\.)*)"')
CHDIR_RE = re.compile(r'^(\d+)\s+chdir\("((?:[^"\\]|\\.)*)"\)\s*=\s*0')


def rel(path):
    """The path relative to the repository, or None outside it (or under a target directory)."""
    p = os.path.normpath(path)
    if not p.startswith(root + "/"):
        return None
    r = p[len(root) + 1:]
    if r.startswith(".git/") or "/target/" in "/" + r:
        return None
    return r


def depinfo(exe):
    """The repository files the binary's build read (its .d file beside it)."""
    d = os.path.splitext(exe)[0] + ".d"
    files = set()
    if not os.path.exists(d):
        return files, False
    for line in open(d):
        if ":" not in line or line.startswith(" "):
            continue
        for f in line.split(":", 1)[1].replace("\\ ", "\0").split():
            r = rel(f.replace("\0", " "))
            if r:
                files.add(r)
    return files, True


def trace(exe, cwd, pkg, target):
    files, dirs = set(), set()
    with tempfile.NamedTemporaryFile(suffix=".strace", delete=False) as t:
        tf = t.name
    env = dict(os.environ, CARGO_MANIFEST_DIR=cwd, CARGO_PKG_NAME=pkg, CARGO_CRATE_NAME=target)
    partial = False
    try:
        subprocess.run(["strace", "-f", "-qq", "-e", "trace=%file,chdir", "-e", "signal=none",
                        "-o", tf, exe], cwd=cwd, env=env,
                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=TIMEOUT)
    except subprocess.TimeoutExpired:
        partial = True
    cwds = {}
    for line in open(tf, errors="replace"):
        m = CHDIR_RE.match(line)
        if m:
            pid, d = m.groups()
            cwds[pid] = os.path.join(cwds.get(pid, cwd), d)
            continue
        m = PATH_RE.match(line)
        # (whatever the call's result: a missing file a test looked for counts too, since adding
        # it changes what the test sees)
        if not m:
            continue
        pid, call, path = m.groups()
        full = path if path.startswith("/") else os.path.join(cwds.get(pid, cwd), path)
        r = rel(full)
        if not r:
            continue
        if "O_DIRECTORY" in line or call in ("getdents", "getdents64"):
            dirs.add(r)
        else:
            files.add(r)
    os.unlink(tf)
    return files, dirs, partial


def build(ws):
    """Test binaries of the workspace: (exe, package dir, package name, target name)."""
    env = dict(os.environ, CARGO_PROFILE_TEST_DEBUG="0", CARGO_PROFILE_DEV_DEBUG="0")
    b = subprocess.run(["cargo", "test", "--workspace", "--locked", "--no-run",
                        "--message-format=json"], cwd=os.path.join(root, ws), env=env,
                       stdout=subprocess.PIPE, text=True)
    if b.returncode:
        sys.exit(f"readsmap: the {ws} test build failed")
    bins, seen = [], set()
    for line in b.stdout.splitlines():
        m = json.loads(line)
        if m.get("reason") == "compiler-artifact" and m.get("executable") and m["profile"]["test"]:
            if m["executable"] in seen:
                continue
            seen.add(m["executable"])
            pkg = m["package_id"]
            name = pkg.split(" ")[0] if " " in pkg else pkg.split("#")[-1].split("@")[0]
            bins.append((m["executable"], os.path.dirname(m["manifest_path"]), name,
                         m["target"]["name"]))
    return bins


result = {"root": root,
          "commit": subprocess.run(["git", "rev-parse", "HEAD"], capture_output=True, text=True,
                                   cwd=root).stdout.strip(),
          "workspaces": {}}
for ws in workspaces:
    bins = build(ws)
    print(f"readsmap: {ws}: {len(bins)} test binaries", flush=True)
    pk = {}

    def one(b):
        exe, cwd, name, target = b
        f, d, partial = trace(exe, cwd, name, target)
        df, ok = depinfo(exe)
        return name, f | df, d, partial or not ok

    with concurrent.futures.ThreadPoolExecutor(JOBS) as ex:
        for name, f, d, partial in ex.map(one, bins):
            e = pk.setdefault(name, {"files": set(), "dirs": set(), "partial": False})
            e["files"] |= f
            e["dirs"] |= d
            e["partial"] |= partial
    result["workspaces"][ws] = {n: {"files": sorted(e["files"]), "dirs": sorted(e["dirs"]),
                                    "partial": e["partial"]} for n, e in sorted(pk.items())}
with open(out + ".tmp", "w") as fh:
    json.dump(result, fh, indent=0)
os.replace(out + ".tmp", out)
print(f"readsmap: wrote {out}")
