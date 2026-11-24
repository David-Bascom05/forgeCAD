#!/usr/bin/env python3
"""Generates forge-tooling's vendor files from the restated facts in research/cam/tooling/raw/:
kernel/forge-tooling/data/tools/<vendor>.toml, data/holders/<vendor>.toml,
data/collets/<vendor>.toml (collets, nuts, pull studs). Every record is `real = true` and cites
every value it carries (forge-tooling src/cite.rs): published values with the page and the
page's own wording; anything derived is `estimate` with its basis; stand-ins are `unknown`.
A record that lacks a value the format requires (dc, lcf, dcon, oal; a drill's point angle; a
tap's pitch; a holder's gauge length and nose) is left out, never filled; the run prints how
many and why.

    python3 tools/tooling/gen.py
"""
import collections
import json
import os
import re

ROOT = os.path.normpath(os.path.join(os.path.dirname(__file__), "..", ".."))
RAW = os.path.join(ROOT, "research/cam/tooling/raw")
DATA = os.path.join(ROOT, "kernel/forge-tooling/data")
TAKEN = "2026-11-23"

VENDORS = {
    "tormach": "Tormach", "harvey": "Harvey Tool", "helical": "Helical Solutions",
    "lakeshore": "Lakeshore Carbide", "yg1": "YG-1", "osg": "OSG", "guhring": "Guhring",
    "techniks": "Techniks", "rego-fix": "Rego-Fix", "schunk": "Schunk", "maritool": "Maritool",
    "shars": "Shars",
}
TOOL_TYPES = {"flat": "flat", "ball": "ball", "bull": "bull", "chamfer": "chamfer", "spot": "spot",
              "drill": "drill", "tap": "tap", "thread_mill": "thread_mill", "reamer": "reamer"}

# Flange diameters published on makers' own drawings: the conservative body stand-in behind the
# nose for a holder whose step profile is not published.
FLANGE = {
    "CAT40": (63.5, "https://www.maritool.com/Mill-Tool-Holders-CAT40-Shrink-Fit-Holders/c2",
              "Maritool CAT40 drawing: Ø2.50 flange, ordinates .125-.75", "published"),
    "BT40": (62.992, "https://www.maritool.com/", "Maritool BT40-ER20-80 drawing: Ø2.48 between ordinates 0.079 and 1.06",
             "published"),
    "HSK63A": (63.0, "", "", "estimate"),
    "BT30": (62.992, "", "", "unknown"),
    "TTS": (63.5, "", "", "unknown"),
}
FLANGE_BASIS = {
    "HSK63A": "the HSK size 63 names the flange's nominal diameter; the largest diameter published for an HSK63A holder in these sources is 63 mm (Schunk D3, Techniks D)",
    "BT30": "no BT30 flange diameter is published in these sources; the BT40 flange (62.99 mm, Maritool drawing) stands in (a BT30 flange is believed smaller: errs large, unverified)",
    "TTS": "the TTS collar diameter is not stated on Tormach's pages; the CAT40 flange (63.5 mm, Maritool drawing) stands in (believed larger than a 3/4 in TTS collar: errs large, unverified)",
}

# ISO 513 group by the vendor's material wording (an estimate: the vendors name materials, not
# groups). First match wins.
GROUPS = [
    ("H", r"harden|\b(4[5-9]|[56]\d)\s*-?\s*\d*\s*(hrc|rc)\b|> ?45 ?rc"),
    ("S", r"titan|inconel|nickel|monel|hastelloy|waspaloy|high[- ]temp|superalloy|heat[- ]resist"),
    ("M", r"stainless|duplex|austenit"),
    ("K", r"cast iron|grey iron|gray iron|ductile|nodular"),
    ("N", r"alumin|copper|brass|bronze|magnes|zinc|non[- ]?ferrous"),
    ("P", r"steel|carbon|alloy steel|tool steel|low carbon|medium carbon"),
]


def group_of(material):
    m = material.lower()
    for g, rx in GROUPS:
        if re.search(rx, m):
            return g
    return None


def tv(v):
    if isinstance(v, bool):
        return "true" if v else "false"
    if isinstance(v, (int, float)):
        r = round(float(v), 4)
        return str(int(r)) if r == int(r) else repr(r)
    if isinstance(v, (list, tuple)):
        return "[" + ", ".join(tv(x) for x in v) + "]"
    return json.dumps(str(v), ensure_ascii=False)


def num(vals, k):
    x = vals.get(k)
    if not x:
        return None
    v = x.get("v")
    if isinstance(v, list):
        return None
    if isinstance(v, (int, float)) and x.get("unit") not in ("unstated",):
        return float(v)
    return None


class Out:
    def __init__(self):
        self.lines = []

    def rec(self, header, fields, cites):
        self.lines.append(f"[[{header}]]")
        for k, v in fields.items():
            if v is not None and v != "":
                self.lines.append(f"{k} = {tv(v)}")
        for c in cites:
            self.lines.append(f"[[{header}.cite]]")
            for k in ("path", "status", "source", "taken", "value", "basis", "note"):
                if c.get(k):
                    self.lines.append(f"{k} = {tv(c[k])}")
        self.lines.append("")


def pub(path, r, key_or_stated, src=None):
    stated = key_or_stated if isinstance(key_or_stated, str) else key_or_stated
    return dict(path=path, status="published", source=src or r["url"], taken=r.get("taken", TAKEN), value=stated)


def stated(r, k):
    return r["values"][k].get("stated", "")


def tool(vendor, r, out, skips):
    vals = r["values"]
    ty = TOOL_TYPES.get(r["kind"])
    if not ty:
        skips[f"kind {r['kind']}"] += 1
        return
    f = {}
    cites = []
    for k in ("dc", "lcf", "dcon", "oal"):
        v = num(vals, k)
        if v is None or v <= 0:
            skips[f"no {k}"] += 1
            return
        f[k] = v
        cites.append(pub(k, r, stated(r, k)))
    if f["oal"] <= f["lcf"]:
        skips["oal <= lcf"] += 1
        return
    nof = num(vals, "nof")
    if nof:
        f["nof"] = int(nof)
        cites.append(pub("nof", r, stated(r, "nof")))
    if ty == "bull":
        re_ = num(vals, "re")
        if not re_:
            skips["bull without re"] += 1
            return
        f["re"] = re_
        cites.append(pub("re", r, stated(r, "re")))
    if ty in ("chamfer", "spot"):
        a = num(vals, "angle")
        if a:
            f["angle"] = a
            cites.append(pub("angle", r, stated(r, "angle")))
        elif ty == "spot" and num(vals, "sig"):
            f["angle"] = num(vals, "sig")
            cites.append(pub("angle", r, stated(r, "sig"), ))
        elif num(vals, "angle_per_side"):
            f["angle"] = 2 * num(vals, "angle_per_side")
            cites.append(dict(path="angle", status="estimate", source=r["url"], taken=r.get("taken", TAKEN),
                              value=stated(r, "angle_per_side"),
                              basis="the page states the angle per side; the included angle is twice it"))
        else:
            skips[f"{ty} without angle"] += 1
            return
        tip = num(vals, "tip_d")
        if tip:
            f["tip_d"] = tip
            cites.append(pub("tip_d", r, stated(r, "tip_d")))
    if ty == "drill":
        a = num(vals, "sig")
        if not a:
            skips["drill without point angle"] += 1
            return
        f["sig"] = a
        cites.append(pub("sig", r, stated(r, "sig")))
    if ty in ("tap", "thread_mill"):
        p = num(vals, "tp")
        if not p:
            skips[f"{ty} without pitch"] += 1
            return
        f["tp"] = p
        st = stated(r, "tp")
        if "tpi" in st.lower() or re.search(r"-\d+\b", st) or "25.4" in str(vals["tp"].get("note", "")):
            cites.append(dict(path="tp", status="estimate", source=r["url"], taken=r.get("taken", TAKEN), value=st,
                              basis="pitch = 25.4 / threads per inch, from the stated thread designation"))
        else:
            cites.append(pub("tp", r, st))
    for k in ("material", "coating"):
        x = vals.get(k)
        if x and isinstance(x.get("v"), str):
            f[k] = x["v"]
            cites.append(pub(k, r, x.get("stated", "")))
    helix = num(vals, "helix")
    if helix:
        f["helix"] = helix
        cites.append(pub("helix", r, stated(r, "helix")))
    # presets: one per ISO group the vendor's material wording maps to (estimate), slotting first
    presets = []
    seen = set()
    rows = sorted(r.get("cutting") or [], key=lambda c: (str(c.get("operation", "")) not in ("slot", "slotting"),))
    for c in rows:
        g = c.get("iso_group") or group_of(str(c.get("material", "")) + " " + str(c.get("material_stated", "")))
        vc = c.get("vc") or {}
        fz = c.get("fz") or c.get("fn_or_fz") or {}
        if not g or g in seen or not isinstance(vc.get("v"), (int, float)):
            continue
        fzv = fz.get("v")
        if isinstance(fzv, list):
            fzv = min(fzv)
        if not isinstance(fzv, (int, float)) or fzv <= 0:
            continue
        seen.add(g)
        name = f"{g} {c.get('operation', '') or 'general'}".strip()
        presets.append((name, g, vc["v"], fzv, c))
        if len(presets) >= 6:
            break
    tid = f"{vendor}-{r['id']}"
    head = dict(id=tid, type=ty, **f, vendor=VENDORS[vendor], product_id=r["id"], real=True,
                source=r["url"], taken=r.get("taken", TAKEN), note=r.get("name", "")[:140])
    out.lines.append("[[tool]]")
    for k, v in head.items():
        if v is not None and v != "":
            out.lines.append(f"{k} = {tv(v)}")
    for name, g, vc, fz, c in presets:
        out.lines += ["[[tool.preset]]", f"name = {tv(name)}", f"group = {tv(g)}", f"vc = {tv(vc)}", f"fz = {tv(fz)}"]
        cites.append(dict(path=f"preset.{name}", status="estimate" if not c.get("iso_group") else "published",
                          source=c.get("url") or r["url"], taken=r.get("taken", TAKEN),
                          value=f"{c.get('material', c.get('material_stated', ''))}: {vc_stated(c)}",
                          basis="" if c.get("iso_group") else
                          f"the vendor's material '{str(c.get('material', ''))[:60]}' mapped to ISO 513 group {g} by its name; fz is the smallest of a stated range"))
    for c in cites:
        out.lines.append("[[tool.cite]]")
        for k in ("path", "status", "source", "taken", "value", "basis", "note"):
            if c.get(k):
                out.lines.append(f"{k} = {tv(c[k])}")
    out.lines.append("")


def vc_stated(c):
    a = (c.get("vc") or {}).get("stated", "")
    b = (c.get("fz") or c.get("fn_or_fz") or {}).get("stated", "")
    return f"{a}; {b}"


def holder(vendor, r, out, skips):
    vals = r["values"]
    itf = (vals.get("interface") or {}).get("v")
    clamp = (vals.get("clamp") or {}).get("v")
    G = num(vals, "gauge_length")
    if not itf or not clamp or not G:
        skips["holder without interface/clamp/gauge length"] += 1
        return
    cites = [pub("interface", r, stated(r, "interface")), pub("clamp", r, stated(r, "clamp")),
             pub("gauge_length", r, stated(r, "gauge_length"))]
    prof = [p for p in (r.get("profile") or []) if isinstance(p.get("d"), (int, float)) and isinstance(p.get("z"), (int, float))]
    nose = num(vals, "nose_d")
    fl = FLANGE.get(itf)
    if len(prof) >= 2:
        zn = max(p["z"] for p in prof)
        # order rings at the same z so the profile steps in the drawn direction (nose side first)
        # nose first: z falling in the drawing's datum; rings at one z (a step) in reverse drawing order
        ordered = [[round(zn - p["z"], 3), p["d"]] for i, p in sorted(enumerate(prof), key=lambda ip: (-ip[1]["z"], -ip[0]))]
        # merge to z non-decreasing; clip at the gauge line
        profile = [x for x in ordered if x[0] <= G + 1e-9]
        if profile[0][0] != 0.0:
            profile.insert(0, [0.0, profile[0][1]])
        top = profile[-1][0]
        basis_extra = ""
        if top < G - 1e-6 and fl:
            profile.append([top, fl[0]])
            profile.append([G, fl[0]])
            basis_extra = f"; from z {top:g} to the gauge line ({G:g}) the drawing's undimensioned part is taken at the {itf} flange diameter {fl[0]:g} mm"
        elif top < G - 1e-6:
            profile.append([G, max(d for _, d in profile)])
            basis_extra = f"; from z {top:g} to the gauge line the largest drawn diameter stands in"
        cites.append(dict(path="profile", status="published" if not basis_extra else "estimate", source=r["url"],
                          taken=r.get("taken", TAKEN), value="; ".join(p.get("stated", "") for p in prof)[:400],
                          basis=("rings read from the dimensioned drawing, z measured from the nose" + basis_extra) if basis_extra else "",
                          note="z from the nose: the drawing's datum moved to the nose (the profile's farthest ring)"))
    elif nose and fl:
        profile = [[0.0, nose], [0.0, max(nose, fl[0])], [G, max(nose, fl[0])]]
        st = fl[3] if fl[3] != "published" else "estimate"
        cites.append(dict(path="profile", status=st, source=r["url"], taken=r.get("taken", TAKEN),
                          value=stated(r, "nose_d"),
                          basis=(f"only the nose diameter is published; behind the nose the whole body up to the gauge line is taken "
                                 f"at the {itf} flange diameter {fl[0]:g} mm" +
                                 (f" ({fl[2]}, {fl[1]})" if fl[1] else f" ({FLANGE_BASIS.get(itf, '')})") +
                                 ": errs large, never small, where the real body steps down")))
    else:
        skips["holder without a profile or nose diameter"] += 1
        return
    f = dict(id=f"{vendor}-{r['id']}", interface=itf, clamp=clamp, gauge_length=G, max_rpm=num(vals, "max_rpm"),
             vendor=VENDORS[vendor], product_id=r["id"], real=True, source=r["url"], taken=r.get("taken", TAKEN),
             note=r.get("name", "")[:140])
    if f["max_rpm"]:
        cites.append(pub("max_rpm", r, stated(r, "max_rpm")))
    for k in ("balance", "coolant", "nose_d"):
        if vals.get(k):
            cites.append(pub(k, r, stated(r, k)))
    out.lines.append("[[holder]]")
    for k, v in f.items():
        if v is not None and v != "":
            out.lines.append(f"{k} = {tv(v)}")
    out.lines.append("profile = [" + ", ".join(f"[{tv(z)}, {tv(d)}]" for z, d in profile) + "]")
    for c in cites:
        out.lines.append("[[holder.cite]]")
        for k in ("path", "status", "source", "taken", "value", "basis", "note"):
            if c.get(k):
                out.lines.append(f"{k} = {tv(c[k])}")
    out.lines.append("")


def collet(vendor, r, out, skips):
    vals = r["values"]
    kind = r["kind"]
    base = dict(id=f"{vendor}-{r['id']}", vendor=VENDORS[vendor], product_id=r["id"], real=True, source=r["url"],
                taken=r.get("taken", TAKEN), note=r.get("name", "")[:140])
    cites = []
    if kind == "collet":
        system = (vals.get("system") or {}).get("v")
        bore = num(vals, "bore")
        if not system or not bore:
            skips["collet without system/bore"] += 1
            return
        f = dict(system=system, bore=bore)
        cites += [pub("system", r, stated(r, "system")), pub("bore", r, stated(r, "bore"))]
        lo, hi = num(vals, "clamp_min"), num(vals, "clamp_max")
        if lo and hi and lo <= hi:
            f["clamp"] = [lo, hi]
            cites.append(pub("clamp", r, f"{stated(r, 'clamp_min')}; {stated(r, 'clamp_max')}"))
        for k in ("length", "od"):
            if num(vals, k):
                f[k] = num(vals, k)
                cites.append(pub(k, r, stated(r, k)))
        out.rec("collet", {**{"id": base["id"]}, **f, **{k: v for k, v in base.items() if k != "id"}}, cites)
    elif kind == "collet_nut":
        system = (vals.get("system") or {}).get("v")
        od, ln = num(vals, "od"), num(vals, "length")
        if not system or not od or not ln:
            skips["nut without system/od/length"] += 1
            return
        f = dict(system=system, od=od, length=ln)
        cites += [pub("system", r, stated(r, "system")), pub("od", r, stated(r, "od")), pub("length", r, stated(r, "length"))]
        th = (vals.get("thread") or {}).get("v")
        if th:
            f["thread"] = th
            cites.append(pub("thread", r, stated(r, "thread")))
        out.rec("collet_nut", {**{"id": base["id"]}, **f, **{k: v for k, v in base.items() if k != "id"}}, cites)
    elif kind == "pull_stud":
        itf = (vals.get("interface") or {}).get("v")
        ln = num(vals, "length")
        if not itf or not ln:
            skips["pull stud without interface/length"] += 1
            return
        f = dict(interface=itf, length=ln)
        cites += [pub("interface", r, stated(r, "interface")), pub("length", r, stated(r, "length"))]
        th = (vals.get("thread") or {}).get("v")
        if th:
            f["thread"] = th
            cites.append(pub("thread", r, stated(r, "thread")))
        a = num(vals, "angle")
        if a:
            f["angle"] = a
            cites.append(pub("angle", r, stated(r, "angle"), ))
        ch = (vals.get("coolant_hole") or {}).get("v")
        if isinstance(ch, str):
            f["coolant"] = ch.strip().lower().startswith(("y", "through", "with"))
            cites.append(pub("coolant", r, stated(r, "coolant_hole")))
        out.rec("pull_stud", {**{"id": base["id"]}, **f, **{k: v for k, v in base.items() if k != "id"}}, cites)
    else:
        skips[f"kind {kind}"] += 1


def main():
    report = {}
    for vendor in VENDORS:
        p = os.path.join(RAW, f"{vendor}.json")
        if not os.path.exists(p):
            continue
        d = json.load(open(p))
        outs = {"tools": Out(), "holders": Out(), "collets": Out()}
        skips = collections.Counter()
        counts = collections.Counter()
        seen = set()
        for r in d["records"]:
            key = (r["kind"], r["id"])
            if key in seen:
                skips["duplicate id"] += 1
                continue
            seen.add(key)
            n0 = {k: len(o.lines) for k, o in outs.items()}
            if r["kind"] in TOOL_TYPES or r["kind"] == "face_mill":
                tool(vendor, r, outs["tools"], skips)
            elif r["kind"] in ("holder", "tapping_head"):
                holder(vendor, r, outs["holders"], skips)
            elif r["kind"] in ("collet", "collet_nut", "pull_stud"):
                collet(vendor, r, outs["collets"], skips)
            else:
                skips[f"kind {r['kind']}"] += 1
            for k, o in outs.items():
                if len(o.lines) > n0[k]:
                    counts[k] += 1
        for k, o in outs.items():
            path = os.path.join(DATA, k, f"{vendor}.toml")
            if o.lines:
                os.makedirs(os.path.dirname(path), exist_ok=True)
                with open(path, "w") as fh:
                    fh.write(f"# {VENDORS[vendor]}: forgecad's records restated from the maker's own public pages/catalogues\n"
                             f"# (never copied). GENERATED by tools/tooling/gen.py from research/cam/tooling/raw/{vendor}.json;\n"
                             f"# every value carries a cite (forge-tooling src/cite.rs). Verify with the maker before use.\n\n")
                    fh.write("\n".join(o.lines))
            elif os.path.exists(path):
                os.remove(path)
        report[vendor] = (dict(counts), dict(skips))
    for v, (c, s) in report.items():
        print(f"{v:10} {c}  skipped: {s}")


if __name__ == "__main__":
    main()
