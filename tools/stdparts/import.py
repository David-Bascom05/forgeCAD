#!/usr/bin/env python3
"""Lines a source's tables up with ours, by the `# map X:` headers of interface/forge-lang/stdparts/*.csv.

    python3 -I tools/stdparts/import.py B=<bd_warehouse>/src/bd_warehouse/data F=<FastenersWB>/FsData ..

For a source the family lists as `src X` (bundleable) the mapped columns are written to
stdparts/third_party/<name>/<family>.csv; for a `chk X` source nothing is written. Either way every
difference from our table is printed, marked `resolved` when stdparts/resolutions/<family>.csv has
its row. Exit status 1 when any difference is unresolved. Re-runnable; reads no network.
"""
import csv
import sys
from fractions import Fraction
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2] / "interface/forge-lang/stdparts"
# the bundled copy's directory per source letter, and its notice
BUNDLE = {"B": ("bd_warehouse", "bd_warehouse 5a8a439, Copyright 2024 Gumyr, Apache-2.0: columns renamed and selected, sizes respelled"),
          "O": ("bosl2", "BOSL2, Copyright (c) 2017-2019 Revar Desmera, BSD-2-Clause: values transcribed from screws.scad")}


def table(path):
    head, rows = {}, []
    lines = path.read_text().splitlines()
    body = [l for l in lines if not l.startswith("#") and l.strip()]
    for l in lines:
        if l.startswith("#") and ":" in l:
            k, v = l[1:].split(":", 1)
            head.setdefault(k.strip(), []).append(v.strip())
    rd = list(csv.reader(body))
    for r in rd[1:]:
        rows.append(dict(zip(rd[0], r)))
    return head, rd[0], rows


def num(s):
    s = s.strip().strip('"')
    if s in ("", "-", "–", "—"):
        return None
    try:
        if " " in s:  # 1 1/4
            a, b = s.split()
            return float(int(a) + Fraction(b))
        return float(Fraction(s)) if "/" in s else float(s)
    except ValueError:
        return None


def theirs_rows(path, section=None):
    lines = path.read_text().splitlines()
    # FastenersWB files hold several tables, each after a line naming it: the first, or `file#name`
    if section is not None:
        lines = lines[[l.strip().strip('"') for l in lines].index(section) + 1:]
    elif lines and "," not in lines[0]:
        lines = lines[1:]
    end = next((i for i, l in enumerate(lines) if l.strip() and "," not in l), len(lines))
    rd = list(csv.reader(lines[:end]))
    hdr = [h.strip() for h in rd[0]]
    return [dict(zip(hdr, [c.strip() for c in r])) for r in rd[1:] if r]


def respell(size, ours):
    s = size.strip().strip('"')
    if s.startswith("M") and "-" in s:
        base, p = s.split("-", 1)
        row = ours.get(base)
        if row is not None and num(row.get("P", "")) == num(p):
            return base
        return f"{base}x{p}"
    return s


def resolutions(fam):
    p = ROOT / "resolutions" / f"{fam}.csv"
    if not p.exists():
        return {}
    with p.open() as f:
        return {(r["size"], r["column"], r["source"]): r["verdict"] for r in csv.DictReader(f)}


def main():
    roots = dict(a.split("=", 1) for a in sys.argv[1:])
    unresolved = 0
    for path in sorted(ROOT.glob("*.csv")):
        if path.parent.name != "stdparts":
            continue
        fam = path.stem
        head, cols, rows = table(path)
        ours = {r["size"]: r for r in rows}
        res = resolutions(fam)
        srcs = {k.split()[1] for k in head if k.startswith("src ")}
        for key, vals in head.items():
            if not key.startswith("map "):
                continue
            x = key.split()[1]
            if x not in roots:
                continue
            parts = [p.strip() for p in vals[0].split("|")]
            file, size_col, pairs = parts[0], "Size", []
            for p in parts[1:]:
                a, b = (t.strip() for t in p.split("=", 1))
                if a == "size":
                    size_col = b
                else:
                    pairs.append((a, b))
            file, _, section = file.partition("#")
            src = Path(roots[x]) / file
            if not src.exists():
                print(f"{fam}: {x}: no {src}")
                unresolved += 1
                continue
            got = {}
            for r in theirs_rows(src, section or None):
                if size_col not in r:
                    continue
                s = respell(r[size_col], ours)
                vals = {a: num(r.get(b, "")) for a, b in pairs}
                if any(v is not None for v in vals.values()):
                    got[s] = vals
            for s, vals in sorted(got.items()):
                if s not in ours:
                    continue
                for a, v in vals.items():
                    o = num(ours[s].get(a, ""))
                    if v is None or (o is not None and abs(o - v) <= 1e-9):
                        continue
                    k = (s, a, x)
                    tag = "resolved" if k in res else "UNRESOLVED"
                    unresolved += k not in res
                    print(f"{fam} {s} {a}: {x} {v:g} ours {o if o is None else f'{o:g}'}  {tag}")
            if x in srcs and x in BUNDLE:
                d = ROOT / "third_party" / BUNDLE[x][0]
                d.mkdir(parents=True, exist_ok=True)
                with (d / f"{fam}.csv").open("w", newline="") as f:
                    f.write(f"# {BUNDLE[x][1]}\n# from {file}\n")
                    w = csv.writer(f, lineterminator="\n")
                    names = [a for a, _ in pairs]
                    w.writerow(["size"] + names)
                    for s in [r["size"] for r in rows if r["size"] in got]:
                        w.writerow([s] + ["" if got[s][n] is None else f"{got[s][n]:g}" for n in names])
    print(f"{unresolved} unresolved")
    sys.exit(1 if unresolved else 0)


main()
