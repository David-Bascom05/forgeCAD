"""census_vs_step.py TALLY_IDS RESULTS.jsonl [TRUTH.jsonl]: a part's oracle label (worst fillet's,
from `census_tally.py ... --ids`) against its match with the source's STEP (volume and area, at 1e-3
and 1e-4). RESULTS is run_cadfs.py's or run_whucad.py's results.jsonl; TRUTH is step_truth.py's
output where the truth is not in the results (CADFS). Prints the cross table and the parts where
the two disagree."""
import collections
import json
import sys

ORDER = ["invalid", "wrong", "unsure", "refused", "unjudged", "right"]


def step(r, t, lim):
    if r["status"] == "untranslated":
        return "untranslated"
    if r.get("v") != "ok":
        return "not built"
    if not t or t.get("err") or not t.get("solids"):
        return "no truth"
    ev = abs(r["vol"] - t["vol"]) / max(t["vol"], 1e-12)
    ea = abs(r["area"] - t["area"]) / max(t["area"], 1e-12)
    return "match" if max(ev, ea) <= lim else "off"


def main():
    parts = collections.defaultdict(list)
    for l in open(sys.argv[1]):
        w = l.split()
        if len(w) >= 4 and not l.startswith("{"):
            parts[w[0].removesuffix(".fg")].append(w[3])
    rows = {json.loads(l)["id"]: json.loads(l) for l in open(sys.argv[2])}
    truth = {}
    if len(sys.argv) > 3:
        for l in open(sys.argv[3]):
            d = json.loads(l)
            truth[d["id"]] = d
    for lim in (1e-3, 1e-4):
        tab = collections.Counter()
        odd = []
        for i, labs in sorted(parts.items()):
            o = min(labs, key=ORDER.index)
            r = rows.get(i)
            if r is None:
                continue
            s = step(r, truth.get(i) or r.get("truth"), lim)
            tab[(o, s)] += 1
            if (o == "right") != (s == "match") and s in ("match", "off"):
                odd.append((i, o, s))
        print(f"STEP at {lim:g}: oracle label x STEP")
        for k, v in sorted(tab.items()):
            print(f"  {v:5d}  {k[0]:9s} {k[1]}")
        print("  disagree:", " ".join(f"{i}({o}/{s})" for i, o, s in odd))


main()
