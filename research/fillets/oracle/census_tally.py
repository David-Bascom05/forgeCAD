"""census_tally.py LABELS.jsonl [OCCT_BODY.json] [REFILLET.jsonl] [--ids]: label_body's labels, a
`wrong` overruled where OCCT disagrees with the oracle.

A wrong fillet becomes right when
- occt_body.py finds none of its counted points stands (OCCT puts each on the kernel's side, so
  forge-bool's Index misclassified it), or
- occt_refillet.py's own OCCT fillet of the body before is valid, within 1e-6 of our volume, and
  puts none of the counted points on the oracle's side (the oracle asked for something no CAD
  fillet does: on every such case so far, a ball wider than the wall).
Prints counts per fillet and per part (a part's label is its worst fillet's: invalid, wrong,
unsure, refused, unjudged, right), and the overruled fillets by reason. With --ids, every fillet's
final label, one line each."""
import collections
import json
import sys

ORDER = ["invalid", "wrong", "unsure", "refused", "unjudged", "right"]


def main():
    a = [x for x in sys.argv[1:] if not x.startswith("--")]
    rows = [json.loads(l) for l in open(a[0])]
    occt = {}
    if len(a) > 1:
        for l in open(a[1]):
            j = json.loads(l)
            occt[j["name"]] = j
    refil = {}
    if len(a) > 2:
        for l in open(a[2]):
            j = json.loads(l)
            refil[j["name"]] = j
    why = collections.Counter()
    for r in rows:
        r["final"] = r["label"]
        if r["label"] != "wrong":
            continue
        name = f"{r['id']}.{r['feature']}"
        o, f = occt.get(name), refil.get(name)
        if o and not o.get("no_step") and o["stand"] == 0:
            r["final"], r["overruled"] = "right", "Index"
        elif f and f.get("valid") and abs(f.get("rel", 1)) < 1e-6 and f.get("occt_with_oracle") == 0:
            r["final"], r["overruled"] = "right", "OCCT's fillet is ours"
        if "overruled" in r:
            why[r["overruled"]] += 1
    first = collections.Counter(r["label"] for r in rows)
    final = collections.Counter(r["final"] for r in rows)
    parts = collections.defaultdict(list)
    for r in rows:
        parts[r["id"]].append(r["final"])
    worst = collections.Counter(min(v, key=ORDER.index) for v in parts.values())
    print(json.dumps({"fillets": len(rows), "first": dict(first), "final": dict(final), "overruled": dict(why),
                      "parts": len(parts), "parts_final": dict(worst)}))
    if "--ids" in sys.argv:
        for r in rows:
            print(r["id"], r["feature"], r["label"], r["final"], r.get("overruled", ""))


main()
