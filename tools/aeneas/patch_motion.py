#!/usr/bin/env python3
"""Check that a translation-only patch (translate.conf SRC_PATCH) is code motion only.

    tools/aeneas/patch_motion.py PATCH FN...

FN are the functions the patch moves code into. The check, over the patch's hunks:

1. every run of removed lines (shipped code taken out) appears verbatim, modulo leading
   whitespace and in order, as a contiguous run of added lines inside one of the FN functions;
2. every other added line is a wrapper line of an FN function -- its doc comment, its signature
   (`fn FN(`, a `name: Type,` parameter line, `) -> T {`), a return of names (`name` or
   `(a, b)`), its closing `}` -- or a blank line, or a call site: `let PAT = FN(ARGS);` with PAT
   and ARGS names only.

So the patched source is the shipped one with blocks moved into functions that take and give back
the names the blocks use: nothing else changes. Exit 1 (with the offending line) otherwise.
"""
import re
import sys

NAME = r"[A-Za-z_][A-Za-z0-9_]*"


def hunks(path):
    """Each hunk's lines as (kind, text), kind in ' ', '-', '+'."""
    out, cur = [], None
    for line in open(path):
        line = line.rstrip("\n")
        if line.startswith("@@"):
            cur = []
            out.append(cur)
        elif cur is not None and line[:1] in (" ", "-", "+") and not line.startswith(("---", "+++")):
            cur.append((line[0], line[1:]))
    return out


def main():
    patch, fns = sys.argv[1], sys.argv[2:]
    fn_alt = "|".join(map(re.escape, fns))
    removed_runs, added = [], []  # added: (text, fn it is inside or None)
    for h in hunks(patch):
        run, in_fn, depth = [], None, 0
        for kind, text in h:
            if kind == "-":
                run.append(text.strip())
                continue
            if run:
                removed_runs.append(run)
                run = []
            if kind == "+":
                m = re.match(r"\s*fn (%s)\(" % fn_alt, text)
                if m and in_fn is None:
                    in_fn, depth = m.group(1), 0
                added.append((text, in_fn))
                if in_fn is not None:
                    depth += text.count("{") - text.count("}")
                    if depth == 0 and "}" in text:
                        in_fn = None
        if run:
            removed_runs.append(run)
    # 1. each removed run inside one FN, verbatim and contiguous
    used = set()
    for run in removed_runs:
        found = False
        for i in range(len(added) - len(run) + 1):
            block = added[i:i + len(run)]
            if all(b[1] is not None and b[0].strip() == r for b, r in zip(block, run)):
                used.update(range(i, i + len(run)))
                found = True
                break
        if not found:
            print(f"patch_motion: removed block not moved verbatim into {fns}: {run[0]!r}...")
            return 1
    # 2. every other added line a wrapper line or a call site
    wrapper = [
        r"\s*$",
        r"\s*///.*$",
        r"\s*//.*$",
        r"\s*fn (%s)\(.*$" % fn_alt,
        r"\s*%s: [^;={}]*,?$" % NAME,
        r"\s*\)( -> [^;={}]*)? \{$",
        r"\s*\(?(%s)(, (%s))*\)?$" % (NAME, NAME),
        r"\s*\}$",
    ]
    call = r"\s*let (mut )?(\(?((mut )?%s)(, (mut )?%s)*\)?) = (%s)\((&?(mut )?%s)?(, &?(mut )?%s)*\);$" % (
        NAME, NAME, fn_alt, NAME, NAME)
    for i, (text, in_fn) in enumerate(added):
        if i in used:
            continue
        if in_fn is not None and any(re.match(p, text) for p in wrapper):
            continue
        if in_fn is None and (re.match(r"\s*$", text) or re.match(r"\s*///.*$", text) or re.match(call, text)):
            continue
        print(f"patch_motion: added line is neither moved code, a wrapper of {fns}, nor a call site: {text!r}")
        return 1
    print(f"patch_motion: {patch}: code motion only ({len(removed_runs)} blocks into {', '.join(fns)})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
