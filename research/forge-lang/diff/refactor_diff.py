"""Make / Extract / Inline Component and file components against the Lean theorems.

The theorems are equations, so the oracle is the document before the refactor:
- `Refactor.extract_occ`: an occurrence of the inline block and the same occurrence of the file
  form (shared params read passed with `with: {W: W}`, functions copied) give the same model,
  ids and traces. Checked: every solid's volume, every top-level feature's value and provenance,
  which params are structural; and SetParam on each shared param moves both alike
  (`extract_copy_cuts_link`: a copied value would not follow). Inline brings back the original's
  run (`extract_occ` read the other way). Extracting onto an existing file: FileExists.
- `Refactor.makeComponent`: moving statements into `component N() {..}` placed where they were
  gives the old model with the moved fragment under the occurrence (`make_ids_move`), every other
  feature and the log unchanged; refused when a moved statement reads a name left behind
  (`make_reads_local`: ReadsOutside) or a statement left behind reads a moved one (StillReadBy).
- `Modules`: a used file's component placed (`part(t.cup(..))`) is the inline component's part;
  `part(FILE, with: {W: v})` is the file with `W = v`; `use_cycle` refused.

  python3 refactor_diff.py --fg PATH/fg [--seed S --count N]

Deterministic (seeded), quiet, temp dirs. Exit 1 on a difference that is not a filed finding.
"""
import argparse, json, os, random, shutil, subprocess, sys, tempfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import core_diff as cd  # noqa: E402


def run(fg, path):
    out = subprocess.run([fg, "run", path, "--json"], capture_output=True, text=True)
    return json.loads(out.stdout)


def edit(fg, path, op):
    out = subprocess.run([fg, "edit", path, json.dumps(op)], capture_output=True, text=True)
    return json.loads(out.stdout.splitlines()[0])


def view(j):
    """What the theorems keep: solids' volumes, top-level features' values and provenance, the
    shared params' structural flags, errors."""
    if j.get("errors"):
        return {"err": sorted(e["kind"] for e in j["errors"])}
    solids = {s["id"]: round(s["volume"], 3) for s in (j.get("physical") or {}).get("solids", [])}
    feats = {f["id"]: [(d["value"], json.dumps(d["from"], sort_keys=True)) for d in f["dims"]]
             for f in j["features"] if f["dims"]}
    st = {p["param"]: p["structural"] for p in j["params"] if p.get("shared") and "." not in p["param"]}
    return {"solids": solids, "feats": feats, "structural": st}


def per_feature_sketches(doc):
    """The document with its own sketch per top-level feature, so a feature can move alone."""
    text = cd.fg_doc(doc)
    out = []
    for line in text.splitlines():
        if line.startswith("e") and " = extrude(plan, " in line:
            name = line.split(" = ")[0]
            out += [f"sketch k_{name}(on: xy) {{", "  outline = rect(corner: (0, 0), size: (10, 10))", "}"]
            line = line.replace("extrude(plan, ", f"extrude(k_{name}, ", 1)
        out.append(line)
    return "\n".join(out) + "\n"


class Tally:
    def __init__(self):
        self.n, self.ex = {}, {}

    def note(self, cat, key, detail):
        self.n[cat] = self.n.get(cat, 0) + 1
        self.ex.setdefault(cat, (key, detail))


def diff_views(a, b):
    if a == b:
        return None
    out = []
    for k in ("err", "solids", "feats", "structural"):
        if a.get(k) != b.get(k):
            out.append(f"{k}: before {a.get(k)} after {b.get(k)}")
    return "; ".join(out)


def check_extract(fg, tmp, key, doc, t):
    for c in doc["comps"]:
        d = os.path.join(tmp, f"{key}_x_{c}")
        os.makedirs(d)
        src = os.path.join(d, "m.fg")
        with open(src, "w") as f:
            f.write(cd.fg_doc(doc))
        before = view(run(fg, src))
        if "err" in before:
            continue
        r = edit(fg, src, {"op": "ExtractComponent", "component": c, "file": f"{c}.fg"})
        if not r.get("ok"):
            t.note("extract-refused", key, str(r.get("error")))
            continue
        t.note("extract", key, "")
        after = view(run(fg, src))
        # the file form evaluates its part as a whole: compare solids and the top level
        dv = diff_views({k: before[k] for k in ("solids",)}, {k: after[k] for k in ("solids",)} if "err" not in after else after)
        if dv:
            t.note("extract-model", key, dv)
        top = {k: v for k, v in before["feats"].items() if not any(k == o[1] for o in doc["body"] if o[0] == "occ")}
        if "err" not in after:
            if {k: v for k, v in after["feats"].items() if k in top} != top:
                t.note("extract-top", key, f"{top} vs {after['feats']}")
            if after["structural"] != before["structural"]:
                t.note("extract-structural", key, f"{before['structural']} vs {after['structural']}")
        # SetParam on every shared length param: both forms move alike
        for s in doc["body"]:
            if s[0] == "shared" and s[3] == "len":
                op = {"op": "SetParam", "param": s[1], "value": {"value": s[2] + 1, "dim": "Length"}}
                p1, p2 = os.path.join(d, "s1.fg"), os.path.join(d, "s2.fg")
                with open(p1, "w") as f:
                    f.write(cd.fg_doc(doc))
                shutil.copy(src, p2)
                e1, e2 = edit(fg, p1, op), edit(fg, p2, op)
                if e1.get("ok") != e2.get("ok"):
                    t.note("extract-setparam-refusal", key, f"{s[1]}: inline {e1} file {e2}")
                    continue
                if not e1.get("ok"):
                    continue
                v1, v2 = view(run(fg, p1)), view(run(fg, p2))
                if v1.get("solids") != v2.get("solids"):
                    t.note("extract-link-cut", key, f"SetParam {s[1]}: inline {v1.get('solids')} file {v2.get('solids')}")
                if e1.get("drives_values") != e2.get("drives_values"):
                    t.note("extract-reach", key, f"SetParam {s[1]}: drives_values inline {e1.get('drives_values')} file {e2.get('drives_values')}")
        # the file exists now: a second extract is refused
        r2 = edit(fg, src, {"op": "ExtractComponent", "component": c, "file": f"{c}.fg"})
        if r2.get("ok") or "FileExists" not in json.dumps(r2.get("error")):
            # the component is gone from the document, so NoComponent is also a refusal
            if r2.get("ok"):
                t.note("extract-again-applied", key, str(r2))
        # inline back: the original's run
        r3 = edit(fg, src, {"op": "InlineComponent", "file": f"{c}.fg", "name": c})
        if not r3.get("ok"):
            placed = any(s[0] == "occ" and s[2] == c for s in cd.stmts_walk(doc["body"]))
            t.note("inline-refused" if placed else "inline-refused-unplaced", key, str(r3.get("error")))
            continue
        back = view(run(fg, src))
        dv = diff_views(before, back)
        if dv:
            t.note("inline-model", key, dv)
        if open(src).read() != cd.fg_doc(doc):
            t.note("inline-text-differs", key, "")


def check_make(fg, tmp, key, doc, rng, t):
    feats = [s[1] for s in doc["body"] if s[0] == "feat"]
    if not feats:
        return
    i = rng.randrange(len(feats))
    j = rng.randrange(i, len(feats))
    moved = feats[i:j + 1]
    d = os.path.join(tmp, f"{key}_m")
    os.makedirs(d)
    src = os.path.join(d, "m.fg")
    with open(src, "w") as f:
        f.write(per_feature_sketches(doc))
    before = view(run(fg, src))
    if "err" in before:
        return
    stmts = [x for e in moved for x in (f"k_{e}", e)]
    r = edit(fg, src, {"op": "MakeComponent", "statements": stmts, "name": "mk"})
    if not r.get("ok"):
        # the model refuses only ReadsOutside (a moved statement reads a non-shared document
        # name) or StillReadBy (a later statement reads a moved one); the generator writes neither
        t.note("make-refused", key, f"{stmts}: {r.get('error')}")
        return
    t.note("make", key, "")
    j2 = run(fg, src)
    after = view(j2)
    if "err" in after:
        t.note("make-error", key, str(after))
        return
    occ = [c for c in j2.get("components", []) if c.get("call", "").startswith("mk(")]
    inside = {}
    for c in occ:
        for o in c["occurrences"]:
            for f in c["features"]:
                if f["dims"]:
                    inside[f["id"].split(".", 1)[1]] = (o, [(x["value"], json.dumps(x["from"], sort_keys=True)) for x in f["dims"]])
    for e in moved:
        if e not in inside:
            t.note("make-missing", key, e)
        elif inside[e][1] != before["feats"].get(e):
            t.note("make-value", key, f"{e}: before {before['feats'].get(e)} after {inside[e][1]}")
    rest = {k: v for k, v in before["feats"].items() if k not in moved}
    if {k: v for k, v in after["feats"].items() if k in rest} != rest:
        t.note("make-others", key, f"{rest} vs {after['feats']}")
    if after["structural"] != before["structural"]:
        t.note("make-structural", key, f"{before['structural']} vs {after['structural']}")
    # (no total volume: the root body is the union of the top-level bodies, so moving an
    # overlapping one out changes the sum, not any feature)


FIXED = [
    # (name, files, op, want: "ok" or a refusal kind)
    ("make_reads_local", {"m.fg": "shared W = 4\nr = W * 2\nsketch p(on: xy) {\n  o = rect(corner: (0, 0), size: (10, 10))\n}\ne = extrude(p, r)\n"},
     {"op": "MakeComponent", "statements": ["p", "e"], "name": "mk"}, "ReadsOutside"),
    ("make_still_read", {"m.fg": "shared W = 4\nsketch p(on: xy) {\n  o = rect(corner: (0, 0), size: (10, 10))\n}\ne = extrude(p, W)\ng = extrude(e.end, W)\n"},
     {"op": "MakeComponent", "statements": ["p", "e"], "name": "mk"}, "StillReadBy"),
    ("make_name_taken", {"m.fg": "shared W = 4\nsketch p(on: xy) {\n  o = rect(corner: (0, 0), size: (10, 10))\n}\ne = extrude(p, W)\nmk = W + 1\n"},
     {"op": "MakeComponent", "statements": ["p", "e"], "name": "mk"}, "refused"),
    ("extract_file_exists", {"m.fg": "shared W = 4\ncomponent cup(d: 5) {\n  sketch s(on: xy) {\n    o = circle((0, 0), 5)\n  }\n  b = extrude(s, d + W)\n}\nc1 = part(cup)\n", "cup.fg": "shared d = 1\n"},
     {"op": "ExtractComponent", "component": "cup", "file": "cup.fg"}, "FileExists"),
    ("extract_read_otherwise", {"m.fg": "shared W = 4\ncomponent cup(d: 5) {\n  sketch s(on: xy) {\n    o = circle((0, 0), 5)\n  }\n  b = extrude(s, d + W)\n}\nc1 = part(cup)\nx = cup\n"},
     {"op": "ExtractComponent", "component": "cup", "file": "cup.fg"}, "refused"),
    # Counter/Nested.nested_delete_dangles (R2): a placer's block reads the deleted feature
    ("nested_still_read", {"m.fg": "component lid(d: 1) {\n  sketch s(on: xy) {\n    o = circle((0, 0), 5)\n  }\n  plate = extrude(s, d)\n}\ncomponent box(d: 1) {\n  lid1 = part(lid)\n  sketch t(on: lid1.plate.end) {\n    o = circle((0, 0), 2)\n  }\n  h = extrude(t, d)\n}\nbox1 = part(box)\n"},
     {"op": "DeleteFeature", "id": "lid.plate"}, "StillReadBy"),
]

# Files components: the used file's component and `with:` against the inline forms.
MODULES = [
    ("use_component",
     {"lib.fg": "shared W = 7\ncomponent cup(d: 9) {\n  sketch s(on: xy) {\n    o = circle((0, 0), 5)\n  }\n  b = extrude(s, d + W)\n}\n",
      "m.fg": "t = use(\"lib.fg\")\nc1 = part(t.cup(d: 3))\n"},
     {"m.fg": "shared W = 7\ncomponent cup(d: 9) {\n  sketch s(on: xy) {\n    o = circle((0, 0), 5)\n  }\n  b = extrude(s, d + W)\n}\nc1 = part(cup(d: 3))\n"}),
    ("with_shared",
     {"lib.fg": "shared W = 7\nsketch s(on: xy) {\n  o = circle((0, 0), 5)\n}\nb = extrude(s, W * 2)\n",
      "m.fg": "c1 = part(\"lib.fg\", with: {W: 12})\n"},
     {"m.fg": "shared W = 12\ncomponent lib() {\n  sketch s(on: xy) {\n    o = circle((0, 0), 5)\n  }\n  b = extrude(s, W * 2)\n}\nc1 = part(lib)\n"}),
]


def check_fixed(fg, tmp, t):
    for name, files, op, want in FIXED:
        d = os.path.join(tmp, "fixed_" + name)
        os.makedirs(d)
        for fn, text in files.items():
            with open(os.path.join(d, fn), "w") as f:
                f.write(text)
        r = edit(fg, os.path.join(d, "m.fg"), op)
        kind = (r.get("error") or {}).get("kind") if isinstance(r.get("error"), dict) else None
        ok = (want == "ok" and r.get("ok")) or (want == "refused" and not r.get("ok")) or (kind == want)
        t.note("fixed" if ok else f"fixed:{name}", name, f"want {want}, got {r}")
    for name, a, b in MODULES:
        views = []
        for k, files in (("a", a), ("b", b)):
            d = os.path.join(tmp, f"mod_{name}_{k}")
            os.makedirs(d)
            for fn, text in files.items():
                with open(os.path.join(d, fn), "w") as f:
                    f.write(text)
            views.append(view(run(fg, os.path.join(d, "m.fg"))))
        va, vb = views
        same = va.get("err") is None and vb.get("err") is None and \
            sorted(va["solids"].values()) == sorted(vb["solids"].values())
        t.note("module" if same else f"module:{name}", name, f"file form {va} inline form {vb}")
    # use_cycle
    d = os.path.join(tmp, "mod_use_cycle")
    os.makedirs(d)
    for fn, text in {"a.fg": "b = use(\"b.fg\")\nshared x = 1\n", "b.fg": "a = use(\"a.fg\")\nshared y = 2\n",
                     "m.fg": "q = use(\"a.fg\")\nshared z = 1\n"}.items():
        with open(os.path.join(d, fn), "w") as f:
            f.write(text)
    j = run(fg, os.path.join(d, "m.fg"))
    t.note("module" if any(e["kind"] == "UseFailed" for e in j["errors"]) else "module:use_cycle", "use_cycle", str(j["errors"]))


KNOWN: set = set()  # X1-X4 fixed on unified-doc-2 936d26d6


def main():
    p = argparse.ArgumentParser()
    p.add_argument("--fg", default="fg")
    p.add_argument("--seed", type=int, default=1)
    p.add_argument("--count", type=int, default=60)
    p.add_argument("--known", default=",".join(sorted(KNOWN)))
    a = p.parse_args()
    known = set(x for x in a.known.split(",") if x)
    tmp = tempfile.mkdtemp(prefix="refactor-diff-")
    probe = os.path.join(tmp, "probe.fg")
    with open(probe, "w") as f:
        f.write("shared W = 1\n")
    r = subprocess.run([a.fg, "edit", probe, json.dumps({"op": "InlineComponent", "file": "x.fg", "name": "x"})],
                       capture_output=True, text=True)
    if "not an op" in r.stderr:
        print("refactor_diff: skipped, this fg has no component refactors")
        return
    t = Tally()
    rng = random.Random(a.seed)
    g = cd.Gen(random.Random(a.seed))
    for n in range(a.count):
        doc = g.doc()
        key = f"s{a.seed}c{n}"
        check_extract(a.fg, tmp, key, doc, t)
        check_make(a.fg, tmp, key, doc, rng, t)
    check_fixed(a.fg, tmp, t)
    counts = {"extract", "make", "fixed", "module"}
    bad = sorted(k for k in t.n if k not in counts and k not in known)
    for k in sorted(t.n):
        print(f"{k:28} {t.n[k]}{'  NEW' if k in bad else ''}")
    for k in bad:
        key, det = t.ex[k]
        print(f"\n## {k}: {key}\n{det}")
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main()
