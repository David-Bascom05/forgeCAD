"""The comparable items of a stage's log (what failed, what ran over the time budget), shared by
run.py (a branch against main) and stage.py (which publishes them per commit)."""
import re

ALONE = {}  # ("slow", binary, test) -> seconds alone, from native-tests.py's budget lines


def items(name, text, alone=None):
    """What failed in a step's log, as comparable items (no times, no line numbers)."""
    out, pending, binary = set(), [], ""
    for l in text.splitlines():
        m = re.match(r"^\s*Running (\S+)", l)
        if m:
            binary = m.group(1)
        m = re.match(r"^test (\S+) \.\.\. FAILED", l)
        if m:
            pending.append(m.group(1))
            if name == "render":
                out.add(("fail", binary, m.group(1)))
            continue
        m = re.match(r"^test result: .*\((\S+)\)$", l)
        if m:
            out |= {("fail", m.group(1), x) for x in pending}
            pending = []
        m = re.match(r"^error: (\S+?)(-[0-9a-f]{16})? reported no result", l)
        if m:
            out.add(("noresult", m.group(1), ""))
        m = re.match(r"^error: over the \S+ s budget: (\S+) (\S+) took .*?(?:([0-9.]+) s|no time) alone", l)
        if m:
            k = ("slow", m.group(1), m.group(2))
            out.add(k)
            (ALONE if alone is None else alone).setdefault(k, float(m.group(3)) if m.group(3) else 1e9)
            continue
        # over the budget but not an error (under the limit, or quick alone): on main's side it
        # makes a branch's slow verdict for the same test main-identical; never a failure itself
        m = re.match(r"^over the \S+ s budget: (\S+) (\S+) took", l)
        if m:
            out.add(("slowish", m.group(1), m.group(2)))
        m = re.match(r"^(FAILED|NO RESULT): (\S+) (\S+) \(", l)
        if m:
            out.add(("wasm", m.group(2), m.group(3)))
    if name != "render":
        out |= {("fail", "?", x) for x in pending}
    return out




def publish(repo, name, content, tries=5):
    """Add or replace one file on the branch preverify/maps (a branch of data files only: reads
    maps, and stage results per commit), retrying when another publisher pushed first."""
    import subprocess, time

    def git(*a, inp=None):
        return subprocess.run(["git", "-C", repo, *a], input=inp, capture_output=True, text=True)

    for i in range(tries):
        git("fetch", "-q", "origin", "preverify/maps")
        parent = git("rev-parse", "-q", "--verify", "FETCH_HEAD").stdout.strip()
        rows = git("ls-tree", parent).stdout.splitlines() if parent else []
        rows = [r for r in rows if r.split("\t", 1)[1] != name]
        blob = git("hash-object", "-w", "--stdin", inp=content).stdout.strip()
        rows.append(f"100644 blob {blob}\t{name}")
        tree = git("mktree", inp="\n".join(rows) + "\n").stdout.strip()
        args = ["commit-tree", tree, "-m", f"preverify maps: {name}"] + (["-p", parent] if parent else [])
        c = git(*args).stdout.strip()
        if git("push", "-q", "origin", f"{c}:refs/heads/preverify/maps").returncode == 0:
            return True
        time.sleep(2 + 3 * i)
    return False


def published(repo, name):
    """A file from preverify/maps, or None."""
    import subprocess
    subprocess.run(["git", "-C", repo, "fetch", "-q", "origin", "preverify/maps"], capture_output=True)
    r = subprocess.run(["git", "-C", repo, "show", f"FETCH_HEAD:{name}"], capture_output=True, text=True)
    return r.stdout if r.returncode == 0 else None


BUILD_ERROR = re.compile(r"error\[E\d+\]|could not compile|error: linking")


def stays_red(name, text):
    """Why a failed step's log must stay RED whatever the time-budget or main comparison would say
    (a reason), or None when its failure is wholly made of tests and may be judged: a build that
    did not compile has no test items, and nothing left over is not a pass."""
    if BUILD_ERROR.search(text):
        return "a build or compile error"
    if not {x for x in items(name, text, {}) if x[0] != "slowish"}:
        return "a failure no test accounts for"
    return None
