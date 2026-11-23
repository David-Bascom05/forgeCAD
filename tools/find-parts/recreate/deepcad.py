#!/usr/bin/env python3
"""DeepCAD (Onshape via ABC) and Fusion 360 Gallery reconstruction programs to .fg (Forge), in cadfs.py's form: one
program in, the .fg out, the outcome printed as JSON for the sweep (scale.py, run_cadfs.py) to judge.

Both datasets share one JSON form, sketches with profiles and extrudes with extents, which ../fusion.py already
translates for the real-parts build rate. This reads it through that translator, so there is one translation of the
form, not two: a fix there is a fix here. What this adds is the outcome as the recreate sweeps read it (status, why,
the features, the notes, how many bodies the program makes, for truth_rule's copies) and the units: DeepCAD is in
metres, Fusion in centimetres. Data stays outside the repo; run with python3 -I.

  deepcad.py PROGRAM.json --out OUT.fg [--source deepcad|fusion] [--whole FEATURESCRIPT.txt]

DeepCAD kept a model's sketches and blind extrudes and dropped the rest: a fillet, a cut up to the next face, a
revolve. Its program is then not the model its STEP is of. --whole reads CADFS's FeatureScript of the same ABC model
and says whether DeepCAD's program is all of it ("whole": true), or why not.

status: "translated", or "untranslated" with why (cls "form": a form the translator does not take; "importer": a
bug in it).
"""
import json
import os
import re
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))
import fusion  # noqa: E402

SCALE = {"deepcad": 1000.0, "fusion": 10.0}
# DeepCAD keeps 8 decimals of a metre, so an arc's ends lie up to 1e-5 mm off its circle, and the sketch solver wants
# them on it to 1e-10: every such arc's centre is refit through its ends (on 400 parts: 286 match against 271, and 30
# SketchNotSolved refusals go). Fusion's keep the data's centre (fusion.ON).
ON = {"deepcad": 1e-10}
OPS = {"": "extrude", "new": "extrude", "add": "extrude", "cut": "cut", "keep": "intersect"}


def bodies(steps):
    """The bodies the program makes: each a label a step made, less those a union took in."""
    made = {st["label"] for st in steps if st.get("label")}
    gone = {t for st in steps if st["kind"] == "combine" for t in st["tools"]}
    return max(len(made - gone), 1) if made else 0


def with_bodies(d):
    """DeepCAD's extrudes carry no body records, which Fusion's do and the translator reads to know which body a
    feature makes or acts on. Read the way Onshape's defaults go: a new body is a body of its own, a join goes into the
    body made last, a cut or an intersect acts on every body."""
    alive, k = [], 0
    for s in d["sequence"]:
        e = d["entities"].get(s["entity"], {})
        if e.get("type") != "ExtrudeFeature" or "bodies" in e:
            continue
        op = e.get("operation")
        if op == "NewBodyFeatureOperation" or not alive:
            alive.append(f"B{k}")
            k += 1
            ext = [alive[-1]]
        elif op == "JoinFeatureOperation":
            ext = [alive[-1]]
        else:
            ext = list(alive)
        e["bodies"] = {b: {"faces": []} for b in alive}
        e["faces"] = {}
        e["extrude_bodies"] = ext
    return d


HARMLESS = {"newSketch", "cPlane", "cPoint", "assignVariable"}


def whole(fs_text, d):
    """(whether DeepCAD's program d is the whole model CADFS's FeatureScript fs_text writes, why not)."""
    ops = re.findall(r"\b(\w+)\(context, id \+ \"F\d+\"", fs_text)
    other = sorted({o for o in ops if o not in HARMLESS and o != "extrude"})
    if other:
        return False, "the model has " + ", ".join(other)
    bounds = set()
    for m in re.findall(r"extrude\(context, id \+ \"F\d+\", \{(.*?)\}\);", fs_text):
        bounds |= set(re.findall(r"BoundingType\.(\w+)", m)) - {"BLIND", "SYMMETRIC"}
    if bounds:
        return False, "an extrude to " + ", ".join(sorted(bounds)).lower()
    n = sum(1 for e in d.get("entities", {}).values() if e.get("type") == "ExtrudeFeature")
    m = ops.count("extrude")
    if n != m:
        return False, f"{m} extrudes in the model, {n} in the program"
    return True, ""


def translate(d, out):
    r = {"status": "translated", "notes": [], "why": ""}
    ents = d.get("entities", {})
    r["features"] = [{"ExtrudeFeature": "extrude"}.get(ents[s["entity"]]["type"], ents[s["entity"]]["type"]) for s in d.get("sequence", []) if s["entity"] in ents and ents[s["entity"]]["type"] != "Sketch"]
    try:
        steps, notes = fusion.translate(with_bodies(d))
        text = fusion.write(steps)
    except fusion.Unsupported as x:
        r.update(status="untranslated", cls="form", why=str(x)[:80])
        return r
    except (KeyError, IndexError, TypeError, ValueError, ZeroDivisionError, AttributeError, RecursionError) as x:
        r.update(status="untranslated", cls="importer", why="bug: " + repr(x)[:70])
        return r
    r["notes"] = [n for n in notes if not n.startswith("after:")]
    r["ops"] = [OPS.get(st.get("op", ""), st["kind"]) if st["kind"] == "extrude" else st["kind"] for st in steps]
    r["program_bodies"] = bodies(steps)
    with open(out, "w") as f:
        f.write(text)
    return r


def main():
    a = sys.argv[1:]
    opt = lambda k, dflt=None: a[a.index(k) + 1] if k in a else dflt
    fusion.MM = SCALE[opt("--source", "deepcad")]
    fusion.ON = ON.get(opt("--source", "deepcad"), fusion.ON)
    if opt("--on"):
        fusion.ON = float(opt("--on"))
    d = json.load(open(a[0]))
    w = whole(open(opt("--whole"), encoding="utf8", errors="replace").read(), d) if opt("--whole") else None
    r = translate(d, opt("--out"))
    if w is not None:
        r["whole"], r["partial"] = w[0], w[1]
    print(json.dumps(r))


if __name__ == "__main__":
    main()
