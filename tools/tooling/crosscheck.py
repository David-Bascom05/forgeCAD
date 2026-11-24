#!/usr/bin/env python3
"""Cross-check our catalogue-cited tool records against the makers' own Fusion tool libraries.

The libraries come from the git-ignored cache (tools/fetch-tool-libraries.sh) and are never
committed, nor is any value read from them (David, 2026-11-24). This writes only:
  - research/cam/tooling/crosscheck.toml: which of OUR fields disagree with the maker's library,
    in our own words (vendor, product id, field; no library value), read by gen.py to add a
    `conflict` cite that cites the catalogue page; and which records agree throughout (gen.py
    notes them "cross-checked");
  - aggregate counts on stdout (for research/cam/tooling/libraries.md).
Matches by the maker's product id (Fusion `product-id`); compares dc, lcf, dcon (SFDM), oal, nof,
re (bull) after converting the library's inches exactly; a difference over 0.01 mm (or any nof
difference) disagrees.

Usage: python3 -I tools/tooling/crosscheck.py [CACHE_DIR]
  CACHE_DIR: $FORGECAD_TOOL_LIBRARIES, else .cache/tool-libraries
"""
import collections
import glob
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))
RAW = os.path.join(ROOT, "research", "cam", "tooling", "raw")
OUT = os.path.join(ROOT, "research", "cam", "tooling", "crosscheck.toml")
TOL = 0.01
# our vendor key -> the libraries' file-name prefixes
LIBS = {"harvey": ["Harvey_Tool_"], "helical": ["Helical_Solutions_"], "osg": ["OSG_"],
        "guhring": ["Guhring_"], "tormach": ["Tormach_"]}
FIELDS = [("dc", "DC"), ("lcf", "LCF"), ("dcon", "SFDM"), ("oal", "OAL"), ("nof", "NOF"), ("re", "RE")]


def norm(pid):
    return str(pid).replace(" ", "").replace("-", "").upper()


def ours(vendor):
    """(product id, {field: value mm}) of our records, read from the generated TOML."""
    path = os.path.join(ROOT, "kernel", "forge-tooling", "data", "tools", f"{vendor}.toml")
    recs, cur = [], None
    for line in open(path):
        line = line.strip()
        if line == "[[tool]]":
            cur = {}
            recs.append(cur)
        elif line.startswith("[[") or cur is None:
            if line.startswith("[["):
                cur = None if line != "[[tool]]" else cur
            continue
        elif " = " in line:
            k, v = line.split(" = ", 1)
            cur[k] = json.loads(v) if v.startswith('"') else (v if v in ("true", "false") else float(v))
    return [(r["product_id"], r) for r in recs if "product_id" in r]


def main():
    cache = sys.argv[1] if len(sys.argv) > 1 else os.environ.get(
        "FORGECAD_TOOL_LIBRARIES", os.path.join(ROOT, ".cache", "tool-libraries"))
    stats = collections.Counter()
    conflicts, agreed = [], []
    for vendor, prefixes in LIBS.items():
        lib = {}
        for pre in prefixes:
            for f in sorted(glob.glob(os.path.join(cache, pre + "*.json"))):
                for t in json.load(open(f)).get("data", []):
                    if t.get("product-id"):
                        lib.setdefault(norm(t["product-id"]), t)
        if not lib:
            stats[f"{vendor}: no library in the cache"] += 1
            continue
        for pid, r in ours(vendor):
            t = lib.get(norm(pid))
            if not t:
                stats[f"{vendor}: not in the library"] += 1
                continue
            stats[f"{vendor}: matched"] += 1
            n0 = len(conflicts)
            k = 25.4 if t.get("unit", "").startswith("inch") else 1.0
            g = t.get("geometry", {})
            for ours_k, lib_k in FIELDS:
                if ours_k not in r or lib_k not in g:
                    continue
                if ours_k == "re" and r.get("type") != "bull":
                    continue
                # Fusion's LCF on a drill runs to the point (Harvey's own descriptions give the
                # flute length we hold): a convention, not a disagreement
                if ours_k == "lcf" and r.get("type") in ("drill", "spot"):
                    stats["drill lcf not compared (the library measures to the point)"] += 1
                    continue
                lv = g[lib_k] * (1 if ours_k == "nof" else k)
                d = abs(float(r[ours_k]) - lv)
                if (ours_k == "nof" and d > 0) or (ours_k != "nof" and d > TOL):
                    stats[f"{vendor}: {ours_k} disagrees"] += 1
                    conflicts.append((vendor, pid, ours_k))
                else:
                    stats["values agree"] += 1
            if len(conflicts) == n0:
                agreed.append((vendor, pid))
    with open(OUT, "w") as fh:
        fh.write("# Our catalogue-cited values that the maker's own Fusion tool library disagrees with\n"
                 "# (tools/tooling/crosscheck.py, 2026-11-24). Our own words only: no library value is\n"
                 "# recorded here (the libraries are never committed). gen.py adds a `conflict` cite\n"
                 "# citing the catalogue page for each.\n\n")
        for v, pid, f in conflicts:
            fh.write(f'[[conflict]]\nvendor = "{v}"\nproduct_id = {json.dumps(pid)}\nfield = "{f}"\n\n')
        fh.write("# Records whose compared values all agree with the maker's library.\n\n")
        for v, pid in agreed:
            fh.write(f'[[agree]]\nvendor = "{v}"\nproduct_id = {json.dumps(pid)}\n\n')
    for k in sorted(stats):
        print(f"{stats[k]:6}  {k}")


if __name__ == "__main__":
    main()
