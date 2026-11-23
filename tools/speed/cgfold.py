#!/usr/bin/env python3
"""Reads a callgrind out file (valgrind --tool=callgrind, any --compress-* setting) and prints:

  cgfold.py OUT incl NAME..     the inclusive Ir of each function whose name contains NAME (summed
                                over the matches that are not called from another match, so a
                                function and its own helpers are not counted twice), one per line
  cgfold.py OUT sum NAME..      the inclusive Ir of the union of every function whose name contains
                                any NAME, a match called from another match counted once
  cgfold.py OUT fold [ROOT]     folded stacks (`a;b;c IR`, flamegraph.pl's input) of the call
                                tree under ROOT (default: the whole program), the cost of each
                                call edge shared down the callee's edges in proportion
  cgfold.py OUT tree [ROOT] [DEPTH] [MIN%]
                                the same tree as indented text, inclusive Ir and share of ROOT

Callgrind records call edges, not stacks, so a deeper frame's share is the callee's average over
its callers: exact for a function with one caller (the phases of a boolean), an estimate below a
function called from many places. Cycles are cut where they close. Only the standard library.
"""
import re
import sys
from collections import defaultdict


def demangle(n):
    # legacy Rust mangling, as valgrind prints it when it does not demangle: strip the hash
    return re.sub(r"::h[0-9a-f]{16}$", "", n)


def parse(path):
    names = {}  # compressed id -> name
    self_cost = defaultdict(int)
    calls = defaultdict(lambda: defaultdict(int))  # caller -> callee -> inclusive Ir
    fn = None
    cfn = None
    pending_call = False
    events_col = 0
    num = re.compile(r"^[+\-*]?\d|^\*|^[+-]")

    def name_of(spec):
        m = re.match(r"\((\d+)\)\s*(.*)", spec)
        if m:
            i, rest = m.group(1), m.group(2)
            if rest:
                names[i] = demangle(rest.strip())
            return names.get(i, "?" + i)
        return demangle(spec.strip())

    with open(path, "r", errors="replace") as f:
        for line in f:
            line = line.rstrip("\n")
            if not line:
                continue
            if line.startswith("fn="):
                fn = name_of(line[3:])
                continue
            if line.startswith("cfn="):
                cfn = name_of(line[4:])
                continue
            if line.startswith("cfi=") or line.startswith("cfl="):
                continue
            if line.startswith("calls="):
                pending_call = True
                continue
            if line.startswith("events:"):
                continue
            if num.match(line):
                parts = line.split()
                cost = int(parts[1 + events_col]) if len(parts) > 1 + events_col else 0
                if pending_call:
                    calls[fn][cfn] += cost
                    pending_call = False
                else:
                    self_cost[fn] += cost
                continue
            # other headers (fl=, fi=, fe=, ob=, positions:, summary: ...) ignored
            if line.startswith("fl=") or line.startswith("fi=") or line.startswith("fe="):
                m = re.match(r"f[lie]=\((\d+)\)\s*(.*)", line)
                continue
    incl = defaultdict(int)
    for f_, c in self_cost.items():
        incl[f_] += c
    for f_, cs in calls.items():
        for g, c in cs.items():
            if g != f_:
                incl[f_] += c
    return self_cost, calls, incl


def reached(calls, ms):
    """The members of `ms` reachable through calls from another member (counted in its cost)."""
    inner = set()
    for f_ in ms:
        seen = {f_}
        todo = [f_]
        while todo:
            x = todo.pop()
            for g in calls.get(x, {}):
                if g in seen:
                    continue
                seen.add(g)
                if g in ms and g != f_:
                    inner.add(g)
                todo.append(g)
    return inner


def roots(calls, incl, root):
    if root:
        return [f for f in incl if root in f]
    called = set()
    for f_, cs in calls.items():
        for g in cs:
            if g != f_:
                called.add(g)
    return [f for f in incl if f not in called]


def walk(calls, incl, selfc, node, cost, stack, out, depth, maxdepth, floor):
    """Shares `cost` (the edge's Ir into `node`) down node's call edges."""
    total = incl.get(node, 0)
    if total <= 0:
        return
    scale = cost / total
    kids = 0
    if depth < maxdepth:
        for g, c in sorted(calls.get(node, {}).items(), key=lambda x: -x[1]):
            if g == node or g in stack:
                continue
            share = c * scale
            if share < floor:
                continue
            kids += share
            walk(calls, incl, selfc, g, share, stack + [g], out, depth + 1, maxdepth, floor)
    rest = cost - kids
    if rest >= 1:
        out.append((stack, rest))


def main():
    if len(sys.argv) < 3:
        print(__doc__)
        sys.exit(64)
    path, mode = sys.argv[1], sys.argv[2]
    selfc, calls, incl = parse(path)
    if mode == "incl":
        for want in sys.argv[3:]:
            ms = [f for f in incl if want in f]
            ms_set = set(ms)
            # drop a match reached from another match (its cost is already in the caller's)
            inner = reached(calls, ms_set)
            print(want, sum(incl[f] for f in ms if f not in inner))
        return
    if mode == "sum":
        ms = [f for f in incl if any(w in f for w in sys.argv[3:])]
        ms_set = set(ms)
        inner = reached(calls, ms_set)
        print(sum(incl[f] for f in ms if f not in inner))
        return
    root = sys.argv[3] if len(sys.argv) > 3 else None
    rs = roots(calls, incl, root)
    if mode == "fold":
        out = []
        for r in rs:
            walk(calls, incl, selfc, r, incl[r], [r], out, 0, 40, max(1, incl[r] * 1e-4))
        for stack, c in out:
            print(";".join(s.replace(";", ":") for s in stack), int(round(c)))
        return
    if mode == "tree":
        maxdepth = int(sys.argv[4]) if len(sys.argv) > 4 else 6
        minpct = float(sys.argv[5]) if len(sys.argv) > 5 else 1.0
        for r in rs:
            top = incl[r]

            def rec(node, cost, depth, stack):
                print("%s%5.1f%% %14d  %s" % ("  " * depth, 100.0 * cost / top, cost, node))
                if depth >= maxdepth:
                    return
                total = incl.get(node, 0) or 1
                for g, c in sorted(calls.get(node, {}).items(), key=lambda x: -x[1]):
                    if g == node or g in stack:
                        continue
                    share = c * cost / total
                    if 100.0 * share / top < minpct:
                        continue
                    rec(g, share, depth + 1, stack | {g})

            rec(r, top, 0, {r})
        return
    print(__doc__)
    sys.exit(64)


if __name__ == "__main__":
    main()
