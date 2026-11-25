"""Two Haas sweeps side by side (run.sh's OUTDIRs): every op, solid and import that differs, worst
first, as a markdown report for a kernel change's owner to answer line by line.

    python3 -I tools/kernel-stress/diff.py A_OUT B_OUT [--md REPORT.md] [--all]

A is the base, B the change. A row is keyed by zip, STEP file, solid index and op. The sections:
  1. NEW WRONG: B answers Ok and wrong (check, mass, mesh or oracle) where A did not.
  2. OK, ANOTHER ANSWER: both Ok, but the volume differs past both runs' error bounds, or a
     section's area or loop count does: a changed answer nothing flags, to be shown right.
  3. NEW HANGS AND CRASHES: B hung, ran out of memory or crashed where A did not.
  4. NEWLY REFUSED: ok or empty in A, refused in B.
  5. FIXED: wrong, hung or crashed in A, ok in B; and refused in A, ok in B.
  6. SOLIDS AND IMPORTS: solids that read valid in A and not in B (and back), imports refused.
  7. TIME: ops whose time moved most (B against A), and the sums.
Only ops in both sweeps are compared; ops in one only are counted. Exit 1 when B has new wrong
answers, new hangs or crashes, or changed answers, so a script can gate on it.
"""

import json
import os
import sys
from collections import Counter, defaultdict

BAD = ("wrong", "panic")


def load(d):
    """{(zip, file, entity, op): event}, {(zip, file, entity): solid}, {(zip, file): import}.
    Keyed by the solid's entity, not its index: two sweeps that read different solids number them
    differently."""
    ops, sols, imps = {}, {}, {}
    for fn in sorted(os.listdir(d)):
        if not fn.endswith(".jsonl"):
            continue
        z = fn[: -len(".jsonl")]
        f = None
        ent_of = {}
        for line in open(os.path.join(d, fn), errors="replace"):
            try:
                e = json.loads(line)
            except ValueError:
                continue
            ev = e.get("ev")
            if ev == "file":
                f = e["file"]
                ent_of = {}
                continue
            if f is None:
                continue
            if ev == "op":
                ops[(z, f, e["ent"], e["op"])] = e
            elif ev == "solid":
                ent_of[e["s"]] = e["ent"]
                sols.setdefault((z, f, e["ent"]), e)
            elif ev == "import":
                imps.setdefault((z, f), e)
            elif ev in ("hang", "memory", "crash"):
                s, _, op = (e.get("key") or "").partition(":")
                if s[1:].isdigit() and int(s[1:]) in ent_of:
                    ent = ent_of[int(s[1:])]
                    st = "crash" if ev == "crash" else ev
                    ops[(z, f, ent, op)] = {"st": st, "ent": ent, "d": e.get("d", "")}
    return ops, sols, imps


def solid_ok(s):
    return s.get("problems") == 0 and not s.get("merr") and s.get("watertight")


def kind(st):
    if st in ("ok", "empty"):
        return "ok"
    if st in BAD:
        return "wrong"
    if st in ("hang", "memory", "crash"):
        return "hang"
    if st == "harness":
        return "harness"
    return "refused"


def name(k, a, b):
    z, f, s, op = k
    ent = (b or a or {}).get("ent", "?")
    return f"{z} / {os.path.basename(f)} #{ent} {op}"


def short(e):
    if not e:
        return "-"
    d = e.get("d", "")
    return f"{e['st']} {d[:110]}".strip()


def changed_answer(a, b, diag):
    """Both Ok: the volume past what 7 printed digits tell apart, or a section's area past what
    its mesh (chord 0.01) tells apart, or its loop count."""
    va, vb = a.get("vol"), b.get("vol")
    if isinstance(va, (int, float)) and isinstance(vb, (int, float)):
        scale = max(abs(va), abs(vb), 1e-30)
        # the volumes are written to 7 digits: a difference past 5e-7 of them is real
        if abs(va - vb) > 5e-7 * scale:
            return f"volume {va:.7g} -> {vb:.7g} ({(vb - va) / scale:+.2e})"
    aa, ab = a.get("area"), b.get("area")
    if isinstance(aa, (int, float)) and isinstance(ab, (int, float)):
        # a section is drawn through a mesh at a chord of 0.01: its area moves with the mesh by
        # about chord x length; a change past 1e-4 of it, and past 1e-6 of the part's box squared, is
        # the section's own
        scale = max(abs(aa), abs(ab), 1e-30)
        if abs(aa - ab) > max(1e-4 * scale, 1e-6 * diag * diag):
            return f"area {aa:.7g} -> {ab:.7g} ({(ab - aa) / scale:+.2e})"
    if "loops" in a and "loops" in b and a["loops"] != b["loops"]:
        return f"loops {a['loops']} -> {b['loops']}"
    return None


def main():
    args = sys.argv[1:]
    md = None
    if "--md" in args:
        i = args.index("--md")
        md = args[i + 1]
        del args[i : i + 2]
    show_all = "--all" in args
    args = [x for x in args if x != "--all"]
    if len(args) != 2:
        sys.exit(__doc__)
    A, B = args
    ao, as_, ai = load(A)
    bo, bs, bi = load(B)
    both = sorted(set(ao) & set(bo))
    sec = defaultdict(list)
    trans = Counter()
    for k in both:
        a, b = ao[k], bo[k]
        ka, kb = kind(a["st"]), kind(b["st"])
        trans[(ka, kb)] += 1
        if kb == "wrong" and ka != "wrong":
            sec["new wrong"].append((k, a, b, ""))
        elif ka == "ok" and kb == "ok":
            sol = bs.get(k[:3]) or as_.get(k[:3]) or {}
            why = changed_answer(a, b, sol.get("diag") or 0.0)
            if why:
                sec["changed"].append((k, a, b, why))
        elif kb == "hang" and ka != "hang":
            sec["new hang"].append((k, a, b, ""))
        elif ka == "ok" and kb == "refused":
            sec["newly refused"].append((k, a, b, ""))
        elif ka in ("wrong", "hang") and kb in ("ok", "refused"):
            sec["fixed"].append((k, a, b, ""))
        elif ka == "refused" and kb == "ok":
            sec["refused to ok"].append((k, a, b, ""))
    # solids
    sol_lost, sol_won = [], []
    for k in sorted(set(as_) & set(bs)):
        oa, ob = solid_ok(as_[k]), solid_ok(bs[k])
        if oa and not ob:
            sol_lost.append(k)
        elif ob and not oa:
            sol_won.append(k)
    imp_lost = [k for k in sorted(set(ai) & set(bi)) if ai[k].get("st") == "ok" and bi[k].get("st") != "ok"]
    imp_won = [k for k in sorted(set(ai) & set(bi)) if ai[k].get("st") != "ok" and bi[k].get("st") == "ok"]
    # time
    ta = sum(ao[k].get("ms", 0) or 0 for k in both)
    tb = sum(bo[k].get("ms", 0) or 0 for k in both)
    moved = sorted(
        ((bo[k].get("ms", 0) or 0) - (ao[k].get("ms", 0) or 0), k) for k in both
    )

    out = []
    p = out.append
    p(f"# Sweep diff: {os.path.basename(A.rstrip('/'))} -> {os.path.basename(B.rstrip('/'))}")
    p("")
    p(f"Ops in both: {len(both)} (only in A: {len(set(ao) - set(bo))}, only in B: {len(set(bo) - set(ao))}). "
      f"Solids in both: {len(set(as_) & set(bs))}. Op time in both: {ta / 1e3:.0f} s -> {tb / 1e3:.0f} s "
      f"({(tb - ta) / max(ta, 1) * 100:+.1f}%).")
    p("")
    p("| A \\ B | " + " | ".join(["ok", "refused", "wrong", "hang", "harness"]) + " |")
    p("|---|---:|---:|---:|---:|---:|")
    for ka in ["ok", "refused", "wrong", "hang", "harness"]:
        p(f"| {ka} | " + " | ".join(str(trans[(ka, kb)]) for kb in ["ok", "refused", "wrong", "hang", "harness"]) + " |")
    p("")
    titles = [
        ("new wrong", "1. NEW WRONG ANSWERS (B Ok and wrong)"),
        ("changed", "2. OK IN BOTH, ANOTHER ANSWER (show each right)"),
        ("new hang", "3. NEW HANGS, OUT OF MEMORY, CRASHES"),
        ("newly refused", "4. NEWLY REFUSED (ok in A)"),
        ("fixed", "5a. FIXED (wrong or hung in A)"),
        ("refused to ok", "5b. REFUSED IN A, OK IN B"),
    ]
    for key, title in titles:
        rows = sec[key]
        p(f"## {title}: {len(rows)}")
        p("")
        lim = None if show_all or key in ("new wrong", "changed", "new hang") else 40
        for k, a, b, why in rows[:lim]:
            p(f"- {name(k, a, b)}: {short(a)} -> {short(b)}" + (f" — {why}" if why else ""))
        if lim and len(rows) > lim:
            p(f"- ... and {len(rows) - lim} more (--all)")
        p("")
    p(f"## 6. SOLIDS AND IMPORTS")
    p("")
    p(f"Solids valid in A, not in B: {len(sol_lost)}; the other way: {len(sol_won)}. "
      f"Imports ok in A, not in B: {len(imp_lost)}; the other way: {len(imp_won)}.")
    for k in sol_lost[: None if show_all else 40]:
        s = bs[k]
        p(f"- lost: {k[0]} / {os.path.basename(k[1])} #{s.get('ent')}: "
          f"{(s.get('why') or s.get('merr') or s.get('mesh_err') or 'mesh open')[:110]}")
    for k in imp_lost:
        p(f"- import lost: {k[0]} / {os.path.basename(k[1])}: {bi[k].get('d', '')[:110]}")
    p("")
    p("## 7. TIME: the ops that moved most")
    p("")
    for dt, k in list(reversed(moved[-10:])) + moved[:10]:
        if abs(dt) < 100:
            continue
        p(f"- {dt / 1e3:+.1f} s: {name(k, ao[k], bo[k])} ({ao[k].get('ms', 0):.0f} -> {bo[k].get('ms', 0):.0f} ms)")
    p("")
    text = "\n".join(out)
    if md:
        open(md, "w").write(text)
    print(text)
    gate = len(sec["new wrong"]) + len(sec["new hang"]) + len(sec["changed"])
    sys.exit(1 if gate else 0)


if __name__ == "__main__":
    main()
