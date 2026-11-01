#!/usr/bin/env python3
"""Joins occt_second.py and selection_check.py into one label per refused or failed part.

  fillet_split.py SECOND.jsonl SELECT.jsonl --out SPLIT.tsv      prints the tally

kernel-gap         the edges fit the source's result, or OCCT built them and matched its STEP
importer-suspect   a picked edge is still sharp in the source's result, or OCCT's result on these edges is off the STEP
undecided          an unpicked edge is gone (2 of the 3 such parts OCCT could judge had the right edges), or not checkable
other-stage        the part stopped at another feature, before this check applies
occt column: A built and matches, B built and off, C built with features after it, D refused or invalid, X not run.
"""
import collections
import json
import sys


def label(sec, sel):
    if sec.get("kind") is None and sel.get("kind") is None:
        return "other-stage"
    if sec["cls"] == "A":
        return "kernel-gap"
    if sec["cls"] == "B":
        return "importer-suspect"
    if sel["cls"] != "checked":
        return "undecided"
    if sel["picked_kept"] > 0:
        return "importer-suspect"
    if sel["last"] and sel["unpicked_gone"] > 0:
        return "undecided"
    return "kernel-gap"


def main():
    a = sys.argv[1:]
    sec = {json.loads(l)["id"]: json.loads(l) for l in open(a[0])}
    sel = {json.loads(l)["id"]: json.loads(l) for l in open(a[1])}
    t = collections.Counter()
    with open(a[a.index("--out") + 1], "w") as f:
        f.write("id\tkind\tlabel\tocct\tsignature\n")
        for i in sorted(sec):
            k = sec[i].get("kind") or sel[i].get("kind") or "-"
            l = label(sec[i], sel[i])
            t[(k, l, sec[i]["cls"] if l == "kernel-gap" else "")] += 1
            f.write(f"{i}\t{k}\t{l}\t{sec[i]['cls']}\t{sec[i]['sig'][:70]}\n")
    for k, v in sorted(t.items()):
        print(v, *k)


if __name__ == "__main__":
    main()
