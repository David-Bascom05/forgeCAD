#!/usr/bin/env python3
"""FreeCAD .FCStd beyond the library: every body forge-import translates, built, against the shape
FreeCAD stored for that body (its Shape's .brp member, read by OCCT under research/import/truth.md's
rule, via brep_judge.ref), by a rule fixed before any result was looked at: the part's volume and
area (`forgecad query F stat`) within 1e-4 of the truth, the tolerance coverage.md judges the
library by. FreeCAD is never run; nothing in a file executes.

  $VENV/bin/python -I fcstd_judge.py PROBE FORGECAD OUT.jsonl WORK_DIR FILE.. [@LIST]
"""
import json, os, subprocess, sys, tempfile, zipfile
import xml.etree.ElementTree as ET

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import brep_judge  # noqa: E402


def shapes(path):
    """Body name -> its Shape property's member, from Document.xml."""
    z = zipfile.ZipFile(path)
    root = ET.fromstring(z.read("Document.xml"))
    out = {}
    for o in root.iter("Object"):
        name = o.get("name")
        props = o.find("Properties")
        if name is None or props is None:
            continue
        for p in props.findall("Property"):
            if p.get("name") == "Shape":
                part = p.find("Part")
                if part is not None and part.get("file"):
                    out[name] = part.get("file")
    return z, out


def main(argv):
    probe, cli, out, work = argv[:4]
    fs = []
    for a in argv[4:]:
        fs += [l.strip() for l in open(a[1:]) if l.strip()] if a.startswith("@") else [a]
    me = os.path.join(os.path.dirname(os.path.abspath(__file__)), "brep_judge.py")
    with open(out, "w") as o:
        for k, f in enumerate(fs):
            d = os.path.join(work, str(k))
            os.makedirs(d, exist_ok=True)
            t = brep_judge.child([probe, "fcstd", f, d], 120)
            if "crash" in t or "err" in t:
                o.write(json.dumps({"file": f, "verdict": "crash" if "crash" in t else "refused", "why": [t.get("err") or t.get("panic") or t["crash"]]}) + "\n")
                continue
            if not t["bodies"]:
                o.write(json.dumps({"file": f, "verdict": "no-body", "why": []}) + "\n")
                continue
            z, sh = shapes(f)
            for b in t["bodies"]:
                r = {"file": f, "body": b["name"], "status": b["status"]}
                if b["status"] != "translated":
                    r["verdict"], r["why"] = b["status"], [b["why"]]
                    o.write(json.dumps(r) + "\n")
                    continue
                if b["name"] not in sh:
                    r["verdict"], r["why"] = "no-truth", ["no stored shape"]
                    o.write(json.dumps(r) + "\n")
                    continue
                with tempfile.TemporaryDirectory() as tmp:
                    brp = os.path.join(tmp, "s.brep")
                    open(brp, "wb").write(z.read(sh[b["name"]]))
                    truth = brep_judge.child([sys.executable, "-I", me, "ref", brp], 600)
                st = brep_judge.child([cli, "query", b["fg"], "stat", "--json"], 300)
                part = st.get("part") or {}
                r["ref"], r["ours"] = truth, part
                if not part.get("volume"):
                    r["verdict"], r["why"] = "not-built", [str(st.get("errors") or st.get("error") or st)[:160]]
                elif "err" in truth or "crash" in truth or not truth.get("solids"):
                    r["verdict"], r["why"] = "no-truth", [str(truth.get("err") or truth.get("crash") or "no solid")]
                else:
                    j = brep_judge.judged(truth, 1e-4)
                    if j is None:
                        r["verdict"], r["why"] = "uncertain", []
                    else:
                        # `stat` prints six figures: 5e-6 of rounding at most
                        dv, da = (part["volume"] - j[0]) / j[0], (part["area"] - j[1]) / j[1]
                        r["verdict"] = "match" if abs(dv) <= 1e-4 and abs(da) <= 1e-4 else "mismatch"
                        r["why"] = [f"dvol {dv:.2e} darea {da:.2e}"]
                o.write(json.dumps(r) + "\n")
                o.flush()


if __name__ == "__main__":
    main(sys.argv[1:])
