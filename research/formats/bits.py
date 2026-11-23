#!/usr/bin/env python3
"""The reader's bit gate on real files: two stepread builds (base and changed) over the same files,
each file's JSON compared field by field (timings dropped). Lists the files whose reading changed,
so a fix can show it changed only the files it was for.

  python3 -I bits.py BASE_STEPREAD NEW_STEPREAD OUT.jsonl FILE.. [@LIST] [--jobs 3] [--timeout 120]
"""
import concurrent.futures as cf, json, resource, subprocess, sys

MEM = 4 << 30
DROP = {"read_ms", "all_ms", "secs"}


def limit():
    resource.setrlimit(resource.RLIMIT_AS, (MEM, MEM))


def run(b, data, timeout):
    try:
        p = subprocess.run([b, "--no-box"], input=data, capture_output=True, timeout=timeout, preexec_fn=limit)
    except subprocess.TimeoutExpired:
        return {"crash": "timeout"}
    lines = p.stdout.decode("utf8", "replace").strip().splitlines()
    try:
        d = json.loads(lines[-1])
    except Exception:
        return {"crash": f"exit {p.returncode}"}
    return {k: v for k, v in d.items() if k not in DROP}


def main(a):
    jobs = int(a[a.index("--jobs") + 1]) if "--jobs" in a else 3
    timeout = int(a[a.index("--timeout") + 1]) if "--timeout" in a else 120
    a = [x for i, x in enumerate(a) if x not in ("--jobs", "--timeout") and (i == 0 or a[i - 1] not in ("--jobs", "--timeout"))]
    base, new, out = a[:3]
    fs = []
    for x in a[3:]:
        fs += [l.strip() for l in open(x[1:]) if l.strip()] if x.startswith("@") else [x]

    def one(f):
        data = open(f, "rb").read()
        x, y = run(base, data, timeout), run(new, data, timeout)
        if "crash" in x and x["crash"] == "timeout" and y == x:
            return {"file": f, "same": True}
        return {"file": f, "same": x == y, "base": None if x == y else x, "new": None if x == y else y}

    changed = 0
    with open(out, "w") as o, cf.ThreadPoolExecutor(jobs) as ex:
        for r in ex.map(one, fs):
            changed += not r["same"]
            o.write(json.dumps(r) + "\n")
    print(f"{len(fs)} files, {changed} read differently")


if __name__ == "__main__":
    main(sys.argv[1:])
