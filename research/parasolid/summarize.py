"""Totals of xtcompare's JSON lines (corpus.sh runs): solids, pairing with STEP, the checks, the
refusals by kind, and times.

    python3 research/parasolid/summarize.py RESULTS.jsonl...
"""
import collections
import json
import re
import sys


def main(files):
    t = collections.Counter()
    why = collections.Counter()
    secs = [0.0, 0.0]
    worst = []
    errors = []
    for f in files:
        for line in open(f):
            try:
                j = json.loads(line)
            except ValueError:
                continue
            t["files"] += 1
            if "timeout_or_crash" in j:
                errors.append((j["xt"], "timeout or crash"))
                continue
            if j.get("xt_error"):
                errors.append((j["xt"], j["xt_error"]))
                continue
            for k in ("xt_solids", "step_solids", "paired", "volume_ok", "volume_unknown", "area_ok",
                      "kind_area_ok", "box_ok", "xt_refusals", "xt_invalid", "xt_faces", "step_faces"):
                t[k] += j.get(k, 0) or 0
            for k, n in (j.get("xt_why") or {}).items():
                why[re.sub(r"\d+", "N", k)] += n
            secs[0] += j.get("xt_secs", 0) or 0
            secs[1] += j.get("step_secs", 0) or 0
            if j.get("paired"):
                worst.append((j.get("worst_volume", 0) or 0, j["xt"].split("/")[-1]))
    print(f"files {t['files']}, read errors {len(errors)}")
    print(f"XT solids {t['xt_solids']}, STEP solids {t['step_solids']}, paired {t['paired']}")
    for k in ("volume_ok", "volume_unknown", "area_ok", "kind_area_ok", "box_ok"):
        print(f"  {k}: {t[k]} of {t['paired']}")
    print(f"XT refusals {t['xt_refusals']}, solids unverified {t['xt_invalid']}")
    for k, n in why.most_common():
        print(f"  {n:5d}  {k}")
    print(f"time: XT {secs[0]:.0f} s, STEP {secs[1]:.0f} s ({secs[0] / max(secs[1], 1e-9):.2f}x)")
    worst.sort(reverse=True)
    print("worst volume:", ", ".join(f"{w:.1e} {n}" for w, n in worst[:5]))
    for x, e in errors[:10]:
        print("ERROR", x.split("/")[-1], e[:120])


if __name__ == "__main__":
    main(sys.argv[1:])
