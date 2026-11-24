"""research/kernel-stress/haas-report.md from tools/kernel-stress/run.sh's JSON lines.

    python3 -I tools/kernel-stress/report.py OUTDIR > research/kernel-stress/haas-report.md

Each OUTDIR/*.jsonl is one zip: `parasolid`, `file`, `import`, `solid`, `op`, `hang`, `crash`,
`done` and `file_done` events (verify/forge-stress/src/main.rs says what each holds).
"""

import json
import os
import re
import sys
from collections import Counter, defaultdict


def norm(s, n=90):
    """A message with its numbers and ids out, so messages of one kind count together."""
    s = re.sub(r"#\d+", "#N", s or "")
    s = re.sub(r"\b(EdgeId|FaceId|VertexId|LoopId|CoedgeId|ShellId)\(\d+\)", r"\1(N)", s)
    s = re.sub(r"#[0-9a-f]{16}", "#NAME", s)
    s = re.sub(r"-?\d+(\.\d+)?(e-?\d+)?", "N", s)
    return s[:n]


def variant(d):
    """A kernel error by its kind: `Internal(SplitCrossed {..})` -> `Internal(SplitCrossed)`."""
    m = re.match(r"\s*(\w+)\((\w+)", d or "")
    if m:
        return f"{m.group(1)}({m.group(2)})"
    m = re.match(r"\s*(\w+) \{", d or "")
    if m:
        return m.group(1)
    m = re.match(r"\s*(check|mass|mesh): (\w+)", d or "")
    if m:
        return f"{m.group(1)}: {m.group(2)}"
    return norm(d, 60)


def family(op):
    if op.startswith("sec_"):
        return "section"
    if re.match(r"(union|int)_pair\d+", op):
        return op.split("_")[0] + "_pair"
    if re.match(r"pair\d+_sum", op):
        return "pair_sum"
    return op


def load(out):
    zips = {}
    for fn in sorted(os.listdir(out)):
        if not fn.endswith(".jsonl"):
            continue
        z = {"name": fn[:-6], "files": [], "parasolid": []}
        cur = None
        for line in open(os.path.join(out, fn), errors="replace"):
            line = line.strip()
            if not line.startswith("{"):
                continue
            try:
                e = json.loads(line)
            except json.JSONDecodeError:
                continue
            ev = e.get("ev")
            if ev == "parasolid":
                z["parasolid"].append(e)
            elif ev == "file":
                cur = {"file": e["file"], "bytes": e["bytes"], "import": None, "solids": {}, "ops": {},
                       "hangs": [], "crashes": [], "done": None, "wall": None}
                z["files"].append(cur)
            elif cur is None:
                continue
            elif ev == "import":
                cur["import"] = cur["import"] or e
            elif ev == "solid":
                cur["solids"].setdefault(e["s"], e)
            elif ev == "op":
                cur["ops"][(e["s"], e["op"])] = e
            elif ev == "hang":
                cur["hangs"].append(e)
            elif ev == "crash":
                cur["crashes"].append(e)
            elif ev == "done":
                cur["done"] = e
            elif ev == "file_done":
                cur["wall"] = e.get("wall_s")
        zips[z["name"]] = z
    return zips


def main():
    out = sys.argv[1]
    zips = load(out)
    files = [(z, f) for z in zips.values() for f in z["files"]]
    real = [(z, f) for z, f in files if f["bytes"] > 1000]

    # ---- per-file figures ----
    rows = []
    cats = Counter()  # (class, detail) -> count
    examples = {}  # (class, detail) -> first (zip, file, solid, op)
    slow = []  # (ms, what)
    surf = Counter()
    ents = Counter()
    curv = Counter()
    worst = []  # (score, text)
    totals = Counter()
    opcount = defaultdict(Counter)  # (group, status, variant) -> op family -> count

    def cat(cls, detail, where):
        k = (cls, norm(detail))
        cats[k] += 1
        examples.setdefault(k, where)

    for z, f in real:
        imp = f["import"] or {}
        where0 = f"{z['name']} / {os.path.basename(f['file'])}"
        r = Counter()
        r["mb"] = f["bytes"] / 1e6
        st = imp.get("st", "none")
        if st != "ok":
            if f["crashes"] and any(c.get("key") == "import" for c in f["crashes"]):
                cat("import crash", f["crashes"][0].get("d", ""), where0)
            elif any(h.get("key") == "import" for h in f["hangs"]):
                cat("import hang", "over the op timeout", where0)
            else:
                cat(f"import {st}", imp.get("d", ""), where0)
        for k, n in imp.get("surf", {}).items():
            surf[k] += n
        for k, n in imp.get("ents", {}).items():
            ents[k] += n
        for k, n in imp.get("curv", {}).items():
            curv[k] += n
        for ref in imp.get("refusals", []):
            cat("refusal: " + ref["kind"], ref["why"], f"{where0} #{ref['ent']}")
            r["refusals"] += 1
        if imp.get("ms") is not None:
            slow.append((imp["ms"], f"import {where0}"))
        for s, sol in f["solids"].items():
            w = f"{where0} solid {s} (#{sol['ent']} {sol['name']})"
            r["solids"] += 1
            r["faces"] += sol["faces"]
            r["edges"] += sol["edges"]
            if sol["problems"] != 0:
                r["invalid"] += 1
                kinds = sol["why"].split(" | ")[0]
                for kn in kinds.split(","):
                    if kn:
                        cat("solid fails check", kn.split(":")[0], w)
            if sol["merr"]:
                r["massfail"] += 1
                cat("mass fails", sol["merr"], w)
            if not sol["watertight"]:
                r["leaky"] += 1
                cat("mesh not watertight" if not sol["mesh_err"] else "mesh fails",
                    sol["mesh_err"] or "open edges", w)
            elif not sol["manifold"]:
                cat("mesh pinched", "watertight but not manifold", w)
            if sol["unverified"]:
                cat("unverified on import", sol["unverified"], w)
            for k in ("check_ms", "mass_ms", "mesh_ms"):
                slow.append((sol.get(k, 0), f"{k[:-3]} {w} ({sol['faces']} faces)"))
        for (s, op), e in f["ops"].items():
            sol = f["solids"].get(s, {})
            w = f"{where0} solid {s} (#{e['ent']}, {sol.get('faces', '?')} faces) {op}"
            r["ops"] += 1
            if e["ms"]:
                slow.append((e["ms"], w))
            stt = e["st"]
            if stt == "ok" or stt == "empty":
                continue
            if stt == "harness":
                continue
            r[stt] += 1
            fam = family(op)
            group = "section" if fam == "section" else ("oracle" if fam.endswith("_sum") else "boolean")
            cat(f"{group} {stt}", variant(e.get("d", "")), w)
            opcount[(group, stt, variant(e.get("d", "")))][fam] += 1
            if stt in ("wrong", "panic"):
                worst.append((3 if stt == "wrong" else 2, w, e.get("d", "")))
        for h in f["hangs"]:
            r["hang"] += 1
            cat(f"{family(h['key'].split(':')[-1])}: hang", "over the op timeout", f"{where0} {h['key']}")
            worst.append((2.5, f"{where0} {h['key']}", "hang"))
        for c in f["crashes"]:
            if c.get("key") != "import":
                r["crash"] += 1
                cat(f"{family((c.get('key') or '?').split(':')[-1])}: crash rc {c['rc']}", c.get("d", ""),
                    f"{where0} {c.get('key')}")
                worst.append((2.8, f"{where0} {c.get('key')}", f"crash rc {c['rc']}: {c.get('d', '')}"))
        if st != "ok":
            worst.append((1.5, where0, f"import {st}: {imp.get('d', '')}"))
        r["import_s"] = (imp.get("ms") or 0) / 1e3
        r["hwm"] = max(imp.get("hwm_mb", 0), (f["done"] or {}).get("hwm_mb", 0))
        r["wall"] = f["wall"] or 0
        r["st"] = st
        r["assembly"] = imp.get("assembly", 0)
        rows.append((z["name"], os.path.basename(f["file"]), r))
        for k in ("solids", "faces", "invalid", "leaky", "massfail", "refusals", "wrong", "refused",
                  "panic", "hang", "crash", "ops"):
            totals[k] += r[k]
        totals["import_" + st] += 1

    p = print
    p("# The kernel under the Haas machine models")
    p()
    p("Every STEP in the Haas solid-model zips (branch `haas-3d-models`, `research/haas-models/`)")
    p("through the kernel: import, `check`, mass properties, a mesh at the default tolerance, four")
    p("sections, and booleans, each op watched for hangs (60 s) and panics. Made by")
    p("`tools/kernel-stress/run.sh` (rerun it on the zips to regenerate this page); the harness is")
    p("`verify/forge-stress`.")
    p()
    p("A boolean's answer is judged three ways: `check` on the result, its mesh watertight, and its")
    p("volume against an oracle. Subtract and intersect with the same tool must add back to the")
    p("solid; union and intersect of two solids must add to their sum. An `Ok` that fails any of")
    p("these is **wrong**: worse than a refusal, as nothing downstream can tell.")
    p()
    p("## Totals")
    p()
    nzip = len(zips)
    p(f"- zips: {nzip}; STEP files: {len(real)} ({sum(f['bytes'] for _, f in real) / 1e9:.2f} GB)")
    p(f"- imports: {totals['import_ok']} read, " + ", ".join(
        f"{totals[k]} {k[7:]}" for k in sorted(totals) if k.startswith("import_") and k != "import_ok" and totals[k]))
    p(f"- solids read: {totals['solids']} ({totals['faces']} faces); failing `check`: {totals['invalid']};"
      f" mesh not watertight: {totals['leaky']}; mass fails: {totals['massfail']}; entities refused: {totals['refusals']}")
    p(f"- ops on the solids that passed: {totals['ops']}; wrong: {totals['wrong']}; refused: {totals['refused']};"
      f" panics: {totals['panic']}; hangs: {totals['hang']}; crashes: {totals['crash']}")
    para = [(z["name"], e) for z in zips.values() for e in z["parasolid"]]
    p(f"- Parasolid files beside them (not read: no x_t reader): {len(para)}"
      f" ({sum(e['bytes'] for _, e in para) / 1e9:.2f} GB)")
    p()

    notes = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "research", "kernel-stress",
                         "haas-notes.md")
    if os.path.exists(notes):
        # the hand-written part: what was fixed, who owns the rest, Parasolid
        p(open(notes).read().rstrip())
        p()

    p("## Failure classes")
    p()
    p("| class | detail | count | ops | first seen |")
    p("|---|---|---:|---|---|")
    order = sorted(cats.items(), key=lambda kv: (-kv[1], kv[0]))
    for (cls, det), n in order:
        g = cls.split(" ")
        ops = opcount.get((g[0], " ".join(g[1:]), det))
        ops = ", ".join(f"{k} {v}" for k, v in ops.most_common()) if ops else ""
        p(f"| {cls} | {det.replace('|', '/')} | {n} | {ops} | {examples[(cls, det)].replace('|', '/')} |")
    p()

    p("## Worst 10")
    p()
    worst.sort(key=lambda w: -w[0])
    seen = set()
    k = 0
    for sc, w, d in worst:
        key = (norm(d, 50), w.split(" solid ")[0])
        if key in seen:
            continue
        seen.add(key)
        k += 1
        p(f"{k}. {w}: {d[:200]}")
        if k == 10:
            break
    p()

    p("## Slowest 20 ops")
    p()
    slow.sort(key=lambda s: -s[0])
    p("| ms | op |")
    p("|---:|---|")
    for ms, w in slow[:20]:
        p(f"| {ms:.0f} | {w} |")
    p()

    p("## Memory: peak resident, largest 10 files")
    p()
    p("| peak MB | STEP MB | file |")
    p("|---:|---:|---|")
    for z, fn, r in sorted(rows, key=lambda x: -x[2]["hwm"])[:10]:
        p(f"| {r['hwm']:.0f} | {r['mb']:.1f} | {z} / {fn} |")
    p()

    p("## What the files hold")
    p()
    p("Faces read, by surface kind: " + ", ".join(f"{k} {n}" for k, n in surf.most_common()) + ".")
    p()
    p("Edges read, by curve kind: " + ", ".join(f"{k} {n}" for k, n in curv.most_common()) + ".")
    p()
    p("Geometric entities in the files' text: " + ", ".join(f"`{k}` {n}" for k, n in ents.most_common()) + ".")
    p()

    p("## Per machine")
    p()
    p("st: import status. asm: placements not applied (assembly parts each at home). Columns after")
    p("`edges` count solids (invalid, leaky mesh, mass fails), refused entities, then ops (wrong,")
    p("refused, panic, hang, crash) over all ops run.")
    p()
    p("| zip | STEP | MB | st | read s | peak MB | solids | faces | edges | invalid | leaky | mass | refused ents | wrong | refused | panic | hang | crash | ops | wall s | asm |")
    p("|---|---|---:|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|")
    for z, fn, r in rows:
        p(f"| {z} | {fn} | {r['mb']:.1f} | {r['st']} | {r['import_s']:.1f} | {r['hwm']:.0f} | {r['solids']} | {r['faces']} |"
          f" {r['edges']} | {r['invalid']} | {r['leaky']} | {r['massfail']} | {r['refusals']} | {r['wrong']} |"
          f" {r['refused']} | {r['panic']} | {r['hang']} | {r['crash']} | {r['ops']} | {r['wall']:.0f} | {r['assembly']} |")
    p()


if __name__ == "__main__":
    main()
