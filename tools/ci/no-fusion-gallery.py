#!/usr/bin/env python3
"""Fail if any file of the Fusion 360 Gallery Dataset is tracked in this repository.

The dataset's licence allows non-commercial research use and forbids redistributing it; David's
rule (2026-11-24) is that none of it is ever committed (tools/fetch-fusion-gallery.sh,
research/cam/benchmarks/datasets.md). Checks every tracked file:
  1. the cache directory (.cache/fusion-gallery) or the dataset's archive names;
  2. the dataset's file names: <digits>_<8 hex>_<digits>[_<digits>].{json,obj,png,smt,step,stp,seg,fidx,xyz}
     (segmentation and reconstruction members, e.g. 136843_616abd92_0.stp, 100106_7f144e5b_0000_0001.smt);
  3. Autodesk Shape Manager text solids (.smt) and the dataset's label files (.seg, .fidx), which
     forgeCAD never uses;
  4. STEP/text content carrying the dataset's export markers (FILE_NAME under ...\\FusionGallery\\...,
     originating systems ColorSegmentExtractor / ReconstructionExtractor).
Exit 1 with the offending paths, else 0. Run: python3 -I tools/ci/no-fusion-gallery.py
"""
import re
import subprocess
import sys

NAME = re.compile(r"(^|/)\d+_[0-9a-f]{8}_\d+(_\d+)?\.(json|obj|png|smt|step|stp|seg|fidx|xyz)$", re.I)
EXT = re.compile(r"\.(smt|seg|fidx)$", re.I)
PATH = re.compile(r"(^|/)(\.cache/fusion-gallery/|s2\.0\.\d_extended_step|r1\.0\.\d(\.zip|/)|j1\.0\.0)", re.I)
MARK = re.compile(rb"FusionGallery\\\\|ColorSegmentExtractor|ReconstructionExtractor")
TEXT = re.compile(r"\.(step|stp|json|obj|txt)$", re.I)
SELF = {"tools/ci/no-fusion-gallery.py"}


def main():
    files = subprocess.run(["git", "ls-files", "-z"], check=True, capture_output=True).stdout.split(b"\0")
    bad = []
    for raw in files:
        if not raw:
            continue
        p = raw.decode("utf-8", "replace")
        if p in SELF:
            continue
        why = None
        if PATH.search(p):
            why = "dataset path"
        elif NAME.search(p):
            why = "dataset file name"
        elif EXT.search(p):
            why = "dataset file type"
        elif TEXT.search(p):
            try:
                with open(raw, "rb") as f:
                    if MARK.search(f.read(4096)):
                        why = "dataset export marker"
            except OSError:
                pass
        if why:
            bad.append(f"{p}: {why}")
    if bad:
        print("Fusion 360 Gallery data must never be committed (licence: non-commercial research,", file=sys.stderr)
        print("no redistribution; see research/cam/benchmarks/datasets.md). Remove:", file=sys.stderr)
        for b in bad:
            print("  " + b, file=sys.stderr)
        return 1
    print(f"no Fusion 360 Gallery files among {len([f for f in files if f])} tracked files")
    return 0


if __name__ == "__main__":
    sys.exit(main())
