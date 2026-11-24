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
    "sumitomo": "Sumitomo Electric Hardmetal", "korloy": "Korloy", "ingersoll": "Ingersoll",
    "tungaloy": "Tungaloy",
}
# Turning sources that print inches (their corner radii are rounded metric values).
INCH_RE = {"ingersoll"}
# Values a turning source states that are not what our field means: not mapped.
TURN_SKIP = {
    "ingersoll": {"l": "Ingersoll's 'l' falls as the corner radius grows (CNMG120404 .488 in, CNMG120408 .472 in): not the ISO cutting edge length"},
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
# ISO 513 group by the maker's material name; H only for a steel or cast iron whose stated
# hardness starts at 45 HRC or above (ISO 513 H: hardened steel and chilled cast iron)
GROUPS = [
    ("N", r"copper alloy"),  # copper-nickels are N, not S, whatever "nickel" says
    ("S", r"titan|\bti\b|cobalt alloy|inconel|nickel|monel|hastelloy|waspaloy|high[- ]temp|superalloy|heat[- ]resist"),
    ("M", r"stainless|duplex|austenit"),
    ("K", r"cast iron|grey iron|gray iron|ductile|nodular"),
    ("N", r"alumin|copper|brass|bronze|magnes|zinc|non[- ]?ferrous"),
    ("O", r"plastic|acrylic|acetal|delrin|nylon|polycarbonate|peek|ptfe|uhmw"),
    ("P", r"steel|carbon|alloy steel|tool steel|low carbon|medium carbon"),
]


def hrc_low(m):
    """The low end of a stated Rockwell C hardness, or None."""
    x = re.search(r"(\d+)\s*-\s*\d+\s*(?:hrc|rc)\b", m) or re.search(r">\s*(\d+)\s*(?:hrc|rc)\b", m) \
        or re.search(r"(\d+)\s*(?:hrc|rc)\s*\+", m) or re.search(r"(?<![<≤>])\s(\d+)\s*(?:hrc|rc)\b", m)
    return int(x.group(1)) if x else None


def group_of(material):
    m = material.lower()
    for g, rx in GROUPS:
        if re.search(rx, m):
            h = hrc_low(m)
            return "H" if g in ("P", "K") and h is not None and h >= 45 else g
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

    def rec(self, header, fields, cites, presets=()):
        self.lines.append(f"[[{header}]]")
        for k, v in fields.items():
            if v is not None and v != "":
                self.lines.append(f"{k} = {tv(v)}")
        for pr in presets:
            self.lines.append(f"[[{header}.preset]]")
            for k in ("name", "group", "vc", "fz", "ap", "ae"):
                v = pr.get(k)
                if v is not None:
                    self.lines.append(f"{k} = {tv(v) if isinstance(v, str) else repr(round(float(v), 6))}")
        cites = list(cites) + [pr["cite"] for pr in presets]
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


# FEEDS-SPEEDS' preset spec: one preset per (ISO group, class), class from the maker's column
# header; vc and fz as printed (the low end of a printed range, as an estimate); ap/ae from the
# stated depths; drills fz in mm/rev; taps feed their pitch per rev. "<G> <class>" is the
# group's representative material; every row is also "<G> <class> <maker's material>".
CLASS = {"slot": "slot", "slotting": "slot", "rough": "rough", "roughing": "rough",
         "finish": "finish", "finishing": "finish", "milling (general)": "general",
         "drilling": "general", "chamfer": "general", None: "general", "": "general"}
GROUP_ORDER = "NPMKSHO"
CLASS_ORDER = ["slot", "rough", "finish", "general"]


def low(x):
    v = (x or {}).get("v")
    if isinstance(v, list):
        v = [y for y in v if isinstance(y, (int, float))]
        return (min(v), True) if v else (None, False)
    return (v, False) if isinstance(v, (int, float)) else (None, False)


def exact(x, dc=None):
    """A value restated from its printed inch text exactly (SFM x 0.3048, IPT/IPR x 25.4; a
    depth as a multiple or percent of the diameter x dc), the low end of a printed range; else
    the raw value's low end. Returns (value, was_a_range)."""
    st = str((x or {}).get("stated", ""))
    n = r"(\d*\.?\d+)"
    rng = re.search(n + r"\s*-\s*" + n, st) is not None
    for unit, k in (("SFM", 0.3048), ("IPT", 25.4), ("IPR", 25.4)):
        m = re.search(n + r"(?:\s*-\s*" + n + r")?\s*" + unit, st) or (
            re.search(unit + r"\s+" + n + r"(?:\s*-\s*" + n + r")?", st))
        if m:
            return round(float(m.group(1)) * k, 9), m.group(2) is not None
    if dc:
        m = re.search(n + r"x?\s*(?:-\s*" + n + r"x?\s*)?(?:x\s*)?Dia", st) if "%" not in st else None
        if m:
            return round(float(m.group(1)) * dc, 9), m.group(2) is not None
        m = re.search(n + r"%\s*(?:-\s*" + n + r"%\s*)?Diam", st)
        if m:
            return round(float(m.group(1)) / 100 * dc, 9), m.group(2) is not None
    v, r_ = low(x)
    return v, r_ or (rng and v is not None and isinstance((x or {}).get("v"), list))


# FEEDS-SPEEDS' representative material per group, tiers in order (the first tier any row
# matches wins; within a tier the softest stated hardness, then the source's order). H: the
# lowest stated hardness at or above 45 HRC. A group with no tier match falls back to its
# lowest-vc row, and the cite says so.
REPRESENTATIVE = {
    "N": [r"6061|7075", r"wrought|^aluminums?$|aluminium and al-alloys", r"alumin"],
    "P": [r"1018|1010|low carbon|mild steel", r"1045|med(ium)? carbon|carbon steel",
          (r"4140|4130|alloy steel", 30)],
    "M": [r"304|316|austenitic|stainless.*soft"],
    "K": [r"gr[ae]y (cast )?iron|cast irons? - soft|cast irons? - medium"],
    "S": [r"6al-4v|titanium - soft|titanium alloy|ti and ti-alloys", r"nickel"],
    "O": [r"plastic"],
}


def mat_of(c):
    return " ".join(str(c.get("material") or c.get("material_stated") or "").split())


def representative(g, rows):
    """(row, how) for group g among rows (each a dict with 'c' and 'vc')."""
    def hard(x):
        return hrc_low(mat_of(x["c"]).lower())
    if g == "H":
        hs = [x for x in rows if (hard(x) or 0) >= 45]
        if hs:
            return min(hs, key=hard), "the row with the lowest stated hardness at or above 45 HRC"
    for tier in REPRESENTATIVE.get(g, []):
        rx, cap = tier if isinstance(tier, tuple) else (tier, None)
        hit = [x for x in rows if re.search(rx, mat_of(x["c"]).lower())
               and (cap is None or (hard(x) or 0) <= cap)]
        if hit:
            return min(hit, key=lambda x: hard(x) or 0), "the group's representative by FEEDS-SPEEDS' preference list"
    return min(rows, key=lambda x: (x["vc"], x["fz"])), \
        "no material of FEEDS-SPEEDS' preference list is in the source: the group's lowest-vc row"


def make_presets(vendor, ty, f, r, skips):
    rows = []
    for c in r.get("cutting") or []:
        op = c.get("operation")
        if op not in CLASS:  # Harvey's "max" is a ceiling chip load, not an operation
            skips[f"cutting row of operation {op!r}"] += 1
            continue
        printed = c.get("iso_group")
        g = printed or group_of(str(c.get("material", "")) + " " + str(c.get("material_stated", "")))
        vc, vr = exact(c.get("vc"))
        fzx = c.get("fz") or c.get("fn_or_fz")
        fz, fr = exact(fzx)
        derived_fz = False
        if fz is None and ty == "tap" and f.get("tp"):
            fz, derived_fz = f["tp"], True
        if not g or vc is None or vc <= 0 or fz is None or fz <= 0:
            skips["cutting row without group, vc or feed"] += 1
            continue
        rows.append(dict(g=g, cls=CLASS[op], vc=vc, fz=fz, vr=vr, fr=fr, dfz=derived_fz,
                         printed=printed, fzx=fzx, c=c))
    # a diameter on the boundary of two of the maker's diameter columns gets a row from each:
    # keep the column whose lower edge is the diameter (its range's low end, which a preset
    # takes, is the value for that diameter), else the lower feed; and say so
    def lower_edge(y):
        m = re.search(r"column\s+(\d+)(?:/(\d+))?\s*-", str(y["c"].get("condition") or ""))
        if not m:
            return None
        return float(m.group(1)) / float(m.group(2) or 1) * 25.4
    uniq = {}
    for x in rows:
        k = (x["g"], x["cls"], mat_of(x["c"]))
        if k in uniq:
            skips["boundary-column rows merged"] += 1
            at_edge = [y for y in (uniq[k], x) if lower_edge(y) is not None
                       and abs(lower_edge(y) - f["dc"]) < 1e-6]
            keep = at_edge[0] if len(at_edge) == 1 else min(uniq[k], x, key=lambda y: (y["fz"], y["vc"]))
            other = x if keep is uniq[k] else uniq[k]
            why = "the one whose lower edge is the diameter" if len(at_edge) == 1 else "the lower feed"
            keep = dict(keep, boundary=(keep["c"].get("condition"), other["c"].get("condition"), why))
            uniq[k] = keep
        else:
            uniq[k] = x
    rows = list(uniq.values())
    gkey = lambda x: (GROUP_ORDER.index(x["g"]) if x["g"] in GROUP_ORDER else 9, CLASS_ORDER.index(x["cls"]))
    out, names = [], set()
    # the representative per (group, class), named "<G> <class>"; then every row, named
    # "<G> <class> <the maker's material>"
    keys = sorted({(x["g"], x["cls"]) for x in rows}, key=lambda k: gkey(dict(g=k[0], cls=k[1])))
    for g, cls in keys:
        x, how = representative(g, [y for y in rows if y["g"] == g and y["cls"] == cls])
        out.append(preset(vendor, ty, f, r, x, f"{g} {cls}", how))
    for x in sorted(rows, key=gkey):
        name = f"{x['g']} {x['cls']} {mat_of(x['c'])}"
        assert name not in names, name
        names.add(name)
        out.append(preset(vendor, ty, f, r, x, name, None))
    return out


def preset(vendor, ty, f, r, x, name, how):
    g, cls, c, vc, fz, printed = x["g"], x["cls"], x["c"], x["vc"], x["fz"], x["printed"]
    ap, apr = exact(c.get("ap"), f.get("dc"))
    ae, aer = exact(c.get("ae"), f.get("dc"))
    mat = mat_of(c)
    basis, notes = [], []
    if not printed:
        basis.append(f"the maker's material '{mat[:70]}' mapped to ISO 513 group {g} by its name")
    if how:
        basis.append(how) if not how.startswith("the group's representative") or not printed else notes.append(how)
    for what, rng in (("vc", x["vr"]), ("fz", x["fr"]), ("ap", apr), ("ae", aer)):
        if rng:
            basis.append(f"{what} is the low end of the printed range")
    st = lambda k: (c.get(k) or {}).get("stated", "")
    for k in ("ap", "ae"):
        if re.match(r"\s*up to", st(k), re.I):
            notes.append(f"{k} is the source's upper limit (\"{st(k)}\")")
    if vendor == "helical" and cls == "finish" and ae is not None:
        notes.append("programmed at ae: the finishing column is a programmed feed at the stated RDOC")
    if x.get("boundary"):
        a, b, why = x["boundary"]
        notes.append(f"the tool's diameter lies in two of the source's columns ({a}; {b}): the first is kept, {why}")
    if x["dfz"]:
        notes.append("fz is the tap's pitch per revolution (a tap feeds its pitch); the source prints vc only")
    elif ty in ("drill", "spot") or (x["fzx"] or {}).get("unit") == "mm/rev":
        notes.append("fz is a feed per revolution (mm/rev), as printed for a drill")
    value = "; ".join(v for v in (
        mat, st("vc"), st("fz") or st("fn_or_fz"),
        f"ap {st('ap')}" if st("ap") else "", f"ae {st('ae')}" if st("ae") else "") if v)
    if re.search(r"\b(SFM|IPT|IPR|in)\b", value):
        notes.append("metric values are exact conversions (SFM x 0.3048 m/min, IPT/IPR x 25.4 mm, in x 25.4 mm)")
    cite = dict(path=f"preset.{name}", status="estimate" if basis else "published",
                source=c.get("url") or r["url"], taken=r.get("taken", TAKEN), value=value,
                basis="; ".join(basis), note="; ".join(notes))
    return dict(name=name, group=g, vc=vc, fz=fz, ap=ap, ae=ae, cite=cite)


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
    presets = make_presets(vendor, ty, f, r, skips)
    tid = f"{vendor}-{r['id']}"
    head = dict(id=tid, type=ty, **f, vendor=VENDORS[vendor], product_id=r["id"], real=True,
                source=r["url"], taken=r.get("taken", TAKEN), note=r.get("name", "")[:140])
    out.lines.append("[[tool]]")
    for k, v in head.items():
        if v is not None and v != "":
            out.lines.append(f"{k} = {tv(v)}")
    for pr in presets:
        out.lines.append("[[tool.preset]]")
        for k in ("name", "group", "vc", "fz", "ap", "ae"):
            v = pr.get(k)
            if v is not None:
                out.lines.append(f"{k} = {tv(v) if isinstance(v, str) else repr(round(float(v), 6))}")
        cites.append(pr["cite"])
    for c in cites:
        out.lines.append("[[tool.cite]]")
        for k in ("path", "status", "source", "taken", "value", "basis", "note"):
            if c.get(k):
                out.lines.append(f"{k} = {tv(c[k])}")
    out.lines.append("")


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


TURN_VC = os.path.join(RAW, "turn-vc")


def turn_vc_rows(vendor):
    p = os.path.join(TURN_VC, f"{vendor}.json")
    return json.load(open(p))["rows"] if os.path.exists(p) else []


def nominal_or_low(x):
    """(value, how): the maker's printed nominal, optimum or recommended value ("start here")
    when given, else a range's low
    end, else the single value; how is None for a single printed value."""
    for k in ("optimum", "recommended"):  # Sumitomo min-optimum-max, Korloy "300 (210~390)"
        if isinstance((x or {}).get(k), (int, float)):
            return x[k], None
    v = (x or {}).get("v")
    if isinstance(v, dict):
        if isinstance(v.get("nominal"), (int, float)):
            return v["nominal"], None
        v = v.get("range")
    if isinstance(v, list):
        v = [y for y in v if isinstance(y, (int, float))]
        return (min(v), "the low end of the printed range") if v else (None, None)
    return (v, None) if isinstance(v, (int, float)) else (None, None)


def insert_presets(vendor, r, f, skips):
    """FEEDS-SPEEDS' turning presets: per printed (ISO group, grade) vc for this insert, with
    the feed (mm/rev, in `fz`) and ap printed with it or on the insert's own row; a vc without a
    feed is held back. "<G> <chipbreaker> <grade>" is the first material the maker prints for
    the group (their table's lead row); every material is also "<... grade> <material>"."""
    out, names = [], set()
    own = (r.get("cutting") or [None])[0]  # Korloy/Ingersoll: f and ap per chipbreaker row
    cb = f.get("chipbreaker") or ""
    for row in turn_vc_rows(vendor):
        if r["id"] not in row.get("insert_ids", []):
            continue
        printed = row.get("iso_group")
        g = printed or group_of(str(row.get("material_stated") or ""))
        vc, vhow = nominal_or_low(row.get("vc"))
        fx, ax, src2 = row.get("f"), row.get("ap"), None
        if not fx and own and own.get("f"):
            fx, ax, src2 = own.get("f"), ax or own.get("ap"), r["url"]
        fz, fhow = nominal_or_low(fx)
        ap, ahow = nominal_or_low(ax)
        if not g or not vc or not fz:
            skips["turning vc row held back (no group, vc or feed)"] += 1
            continue
        head = " ".join(x for x in (g, cb or row.get("chipbreaker") or "", row.get("grade") or "") if x)
        mat = " ".join(str(row.get("material_stated") or "").split())
        for name, lead in ((head, True), (f"{head} {mat}" if mat else None, False)):
            if not name or name in names:
                continue
            names.add(name)
            basis, notes = [], []
            if not printed:
                basis.append(f"the maker's material '{mat}' mapped to ISO 513 group {g} by its name")
            if lead:
                notes.append("of the maker's materials for this group and grade, the first printed")
            if not row.get("chipbreaker"):
                basis.append("the maker prints this vc per grade, not per chipbreaker: matched to the insert "
                             "by the grades its catalogue row offers")
            for what, how in (("vc", vhow), ("f", fhow), ("ap", ahow)):
                if how:
                    basis.append(f"{what} is {how}")
            if src2:
                notes.append(f"f and ap from the insert's own row ({src2}), vc from the grade table")
            notes.append("fz holds the feed per revolution, mm/rev; a printed optimum, recommended or "
                         "nominal value is kept as the start value")
            st = lambda x: (x or {}).get("stated", "")
            value = "; ".join(v for v in (row.get("grade") or "", mat, st(row.get("vc")), st(fx), st(ax)) if v)
            if re.search(r"\b(SFM|IPR|ipr|inch|in)\b", value):
                notes.append("metric values are exact conversions (SFM x 0.3048 m/min, IPR x 25.4 mm, in x 25.4 mm)")
            if row.get("note"):
                notes.append(row["note"])
            cite = dict(path=f"preset.{name}", status="estimate" if basis else "published", source=row["url"],
                        taken=r.get("taken", TAKEN), value=value, basis="; ".join(basis), note="; ".join(notes))
            out.append(dict(name=name, group=g, vc=vc, fz=fz, ap=ap, cite=cite))
    order = lambda p: (GROUP_ORDER.index(p["group"]) if p["group"] in GROUP_ORDER else 9, p["name"])
    return sorted(out, key=order)


def turning(vendor, r, out, skips):
    vals = r["values"]
    code = (r.get("iso_code") or "").replace(" ", "")
    if not code:
        skips["no ISO code (inch/ANSI only)"] += 1
        return
    base = dict(maker=VENDORS[vendor], real=True, source=r["url"], taken=r.get("taken", TAKEN), note=r.get("name", "")[:140])
    if r["kind"] == "insert":
        f, cites = {"code": code}, [pub("code", r, r.get("id", code))]
        for k in ("ic", "s", "re"):
            v = num(vals, k)
            if v is None:
                skips[f"insert without {k}"] += 1
                return
            f[k] = v
            cites.append(pub(k, r, stated(r, k)))
            # an inch catalogue's corner radius is the metric one rounded to 3 decimals of an inch
            # (.016 in for 0.4 mm, .031 in for 0.8 mm): the ISO code's RE digits disagree with it
            if k == "re" and vendor in INCH_RE:
                code_re = int(code[8:10]) / 10 if len(code) >= 10 and code[8:10].isdigit() else None
                if code_re is not None and abs(code_re - v) > 1e-6:
                    cites.append(dict(path="re", status="conflict", source=r["url"], taken=r.get("taken", TAKEN),
                                      value=stated(r, "re"),
                                      note=(f"the catalogue prints inches; {stated(r, 're')} = {v:g} mm, while the ISO code's "
                                            f"corner-radius digits give {code_re:g} mm; the record holds the printed value")))
        for k in ("l", "clearance"):
            if k in TURN_SKIP.get(vendor, {}):
                continue
            v = num(vals, k)
            if v is not None:
                f[k] = v
                cites.append(pub(k, r, stated(r, k)))
        cb = (vals.get("chipbreaker") or {}).get("v")
        if isinstance(cb, str) and cb:
            f["chipbreaker"] = cb
            cites.append(pub("chipbreaker", r, stated(r, "chipbreaker")))
        rid = f"{vendor}-{r['id']}".replace(" ", "")
        for k, why in TURN_SKIP.get(vendor, {}).items():
            base["note"] = (base["note"] + "; " if base["note"] else "") + f"not mapped: {why}"
        out.rec("insert", {"id": rid, **f, **base}, cites, insert_presets(vendor, r, f, skips))
        return
    # holders and bars: a row listing both hands becomes one record per hand
    kind = "holder" if r["kind"] == "turn_holder" else "bar"
    need = ["l1", "f", "kr"] + (["h", "b"] if kind == "holder" else ["dmm"])
    f = {}
    for k in need:
        v = num(vals, k)
        if v is None:
            skips[f"{kind} without {k}"] += 1
            return
        f[k] = v
    hands_stated = str((vals.get("hand") or {}).get("v", ""))
    hands = [h for h in ("R", "L", "N") if h in hands_stated.replace("/", " ").split() or hands_stated == h]
    if not hands:
        hands = [h for h in ("R", "L") if h in hands_stated]
    if not hands:
        skips[f"{kind} without hand"] += 1
        return
    for hand in hands:
        hc = code
        if "R/L" in hc:
            hc = hc.replace("R/L", hand)
        g = dict(f)
        cites = [pub("code", r, r.get("id", code)), pub("hand", r, stated(r, "hand"))]
        cites += [pub(k, r, stated(r, k)) for k in need]
        mb = num(vals, "min_bore")
        if mb:
            g["min_bore"] = mb
            cites.append(pub("min_bore", r, stated(r, "min_bore")))
        ins = (vals.get("insert") or {}).get("v")
        if isinstance(ins, str) and ins:
            g["insert"] = ins
            cites.append(pub("insert", r, stated(r, "insert")))
        rid = f"{vendor}-{str(r['id']).replace('R/L', hand)}".replace(" ", "")
        out.rec("turn_holder", {"id": rid, "code": hc, "kind": kind, "hand": hand, **g, **base}, cites)


# Single tools found for the shop parts (raw/real-tools/<vendor>.json): parting blades and
# inserts, indexable drills, solid drills, thread mills, each value cited to the catalogue page.
REAL_TOOLS = os.path.join(RAW, "real-tools")


def shop_presets(r, kind):
    """One preset per printed ISO group: the first material the maker prints (its table's lead
    row; of bracketing diameter columns, the first, the smaller tool's); vc the printed
    optimum (Sumitomo min-optimum-max) else the range's low end; the feed as printed."""
    out, seen = [], set()
    for c in r.get("cutting") or []:
        g = c.get("iso_group")
        fx = c.get("f") or c.get("fz")
        if not g or g in seen or not fx:
            continue
        def start(x):
            v = x.get("v")
            if isinstance(v, list) and len(v) == 3 and "optimum" in str(c.get("material_stated", "")).lower() + str(r.get("notes", "")).lower():
                return v[1], None
            if isinstance(v, list) and len(v) == 3 and re.search(r"\d+-\d+-\d+", str(x.get("stated", ""))):
                return v[1], "the printed optimum (min-optimum-max)"
            if isinstance(v, list):
                return min(v), "the low end of the printed range"
            return v, None
        vc, vh = start(c["vc"])
        fz, fh = start(fx)
        seen.add(g)
        name = f"{g} general"
        basis = [f"{w} is {h}" for w, h in (("vc", vh), ("feed", fh)) if h and "low end" in h]
        notes = ["of the maker's materials for this group, the first printed"]
        notes += [f"{w} is {h}" for w, h in (("vc", vh), ("feed", fh)) if h and "optimum" in h]
        if kind == "drill":
            notes.append("fz is the feed per revolution, mm/rev")
        if re.search(r"column", str(c.get("material_stated", ""))):
            basis.append("the feed is tabulated per diameter column, not for this diameter: the first (smaller) column printed")
        if re.search(r"\b(SFM|IPR)\b", str(c["vc"].get("stated", "")) + str(fx.get("stated", ""))):
            notes.append("metric values are exact conversions (SFM x 0.3048, IPR x 25.4)")
        cite = dict(path=f"preset.{name}", status="estimate" if basis else "published", source=c.get("url") or r["url"],
                    taken=r.get("taken", TAKEN),
                    value=f"{c.get('material_stated', '')}; vc {c['vc'].get('stated', '')}; {fx.get('stated', '')}",
                    basis="; ".join(basis), note="; ".join(notes))
        out.append(dict(name=name, group=g, vc=vc, fz=fz, cite=cite))
    return out


def shop(vendor, r, outs, skips):
    vals, url, tk = r["values"], r["url"], r.get("taken", TAKEN)
    V = lambda k: (vals.get(k) or {}).get("v")
    S = lambda k: (vals.get(k) or {}).get("stated", "")
    P = lambda path, k: dict(path=path, status="published", source=url, taken=tk, value=S(k))
    E = lambda path, k, basis: dict(path=path, status="estimate", source=url, taken=tk, value=S(k), basis=basis)
    base = dict(maker=VENDORS[vendor], real=True, source=url, taken=tk, note=r.get("name", "")[:140])
    rid = f"{vendor}-{r['id']}".replace(" ", "")
    k = r["kind"]
    if k == "parting_blade":
        f = dict(code=r["id"], h=V("h"), t_blade=V("b"), l1=V("lf"), cdx=V("cutdia") / 2, max_bar=V("cutdia"),
                 seat=str(V("seat_size")), insert=", ".join(V("compatible_inserts") or []), hand="N")
        cites = [dict(path="code", status="published", source=url, taken=tk, value=r["id"]),
                 P("h", "h"), P("t_blade", "b"), P("l1", "lf"), P("max_bar", "cutdia"), P("seat", "seat_size"),
                 P("insert", "compatible_inserts"),
                 E("cdx", "cutdia", "no reach is printed: half the printed max cut-off diameter CUTDIA"),
                 dict(path="hand", status="estimate", source=url, taken=tk, value=S("fig"),
                      basis="the catalogue figure shows a double-ended neutral blade; no R/L is printed")]
        base["note"] = (base["note"] + f"; clamps in tool blocks {V('tool_block')}; {V('clamp')}").strip("; ")
        outs["turning"].rec("blade", {"id": rid, **f, **base}, cites)
    elif k == "grooving_insert":
        f = dict(code=r["id"], width=V("cw"), re=V("re"), head=V("insl"), seat=str(V("seat_size")))
        cites = [dict(path="code", status="published", source=url, taken=tk, value=r["id"]),
                 P("width", "cw"), P("re", "re"), P("head", "insl"), P("seat", "seat_size")]
        base["note"] = (base["note"] + "; grades " + ", ".join(V("grades_available") or [])
                        + "; no preset: the maker prints vc but its feed only as a bar chart (F236)").strip("; ")
        outs["turning"].rec("groove_insert", {"id": rid, **f, **base}, cites)
    elif k in ("indexable_drill", "drill", "thread_mill"):
        ty = "thread_mill" if k == "thread_mill" else "drill"
        dcon = V("dcon") if V("dcon") is not None else V("dconms")
        dk = "dcon" if V("dcon") is not None else "dconms"
        f = dict(type=ty, dc=V("dc"), dcon=dcon, oal=V("oal") or V("lf"))
        cites = [P("dc", "dc"), P("dcon", dk), P("oal", "oal" if V("oal") is not None else "lf")]
        if k == "indexable_drill":
            if V("lcf") is not None:
                f["lcf"] = V("lcf"); cites.append(P("lcf", "lcf"))
            else:
                f["lcf"] = V("lux"); cites.append(E("lcf", "lux", "no flute length or drilling depth is printed: the neck length LUX, the length clear of the body"))
            f["sig"] = 180
            cites.append(dict(path="sig", status="estimate", source=url, taken=tk, value=f"insert {V('insert')}",
                              basis="an indexable drill cuts a flat bottom: 180 (CAM-TURN's drill model: tip height r/tan(sig/2) = 0)"))
            base["note"] = (base["note"] + f"; insert {V('insert')}").strip("; ")
        elif k == "drill":
            f["lcf"], f["sig"] = V("lcf"), V("sig")
            cites += [P("lcf", "lcf"), P("sig", "sig")]
            if isinstance(V("flutes"), (int, float)):
                f["nof"] = V("flutes"); cites.append(P("nof", "flutes"))
        else:
            f["lcf"], f["tp"], f["nof"] = V("apmx"), V("pitch"), V("flutes")
            cites += [P("lcf", "apmx"), P("tp", "pitch"), P("nof", "flutes")]
        mat = V("material")
        if isinstance(mat, str):
            f["material"] = "carbide" if "carbide" in mat.lower() else ("hss" if "hs" in mat.lower() else mat)
            cites.append(P("material", "material"))
        presets = shop_presets(r, ty)
        head = dict(id=rid, **f, vendor=VENDORS[vendor], product_id=r["id"], real=True, source=url, taken=tk,
                    note=base["note"])
        out = outs["tools"]
        out.lines.append("[[tool]]")
        for kk, v in head.items():
            if v is not None and v != "":
                out.lines.append(f"{kk} = {tv(v)}")
        for pr in presets:
            out.lines.append("[[tool.preset]]")
            for kk in ("name", "group", "vc", "fz"):
                out.lines.append(f"{kk} = {tv(pr[kk]) if isinstance(pr[kk], str) else repr(round(float(pr[kk]), 6))}")
        for c in cites + [p["cite"] for p in presets]:
            out.lines.append("[[tool.cite]]")
            for kk in ("path", "status", "source", "taken", "value", "basis", "note"):
                if c.get(kk):
                    out.lines.append(f"{kk} = {tv(c[kk])}")
        out.lines.append("")
    elif k == "tool_block":
        f = dict(code=r["id"], h=V("h"), b=V("b"), l1=V("oal"), blade_h=32 if V("blade") == "CGP32..." else None)
        cites = [dict(path="code", status="published", source=url, taken=tk, value=r["id"]),
                 P("h", "h"), P("b", "b"), P("l1", "oal")]
        if f["blade_h"]:
            cites.append(dict(path="blade_h", status="estimate", source=url, taken=tk, value=S("blade"),
                              basis="the block takes CGP32 blades, whose height H is 32 (F229)"))
        base["note"] = (base["note"] + f"; HFASSY {V('hfassy')}, HBH {V('hbh')}, OAH {V('oah')}, OAW {V('oaw')} mm "
                        "(the block's other printed dimensions)").strip("; ")
        outs["turning"].rec("tool_block", {"id": rid, **f, **base}, cites)
    else:
        skips[f"shop kind {k}"] += 1


def main():
    report = {}
    for vendor in VENDORS:
        p = os.path.join(RAW, f"{vendor}.json")
        if not os.path.exists(p):
            continue
        d = json.load(open(p))
        outs = {"tools": Out(), "holders": Out(), "collets": Out(), "turning": Out()}
        skips = collections.Counter()
        counts = collections.Counter()
        seen = set()
        for r in d["records"]:
            key = (r["kind"], r["id"], r.get("iso_code"))
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
            elif r["kind"] in ("insert", "turn_holder", "boring_bar"):
                turning(vendor, r, outs["turning"], skips)
            else:
                skips[f"kind {r['kind']}"] += 1
            for k, o in outs.items():
                if len(o.lines) > n0[k]:
                    counts[k] += 1
        rp = os.path.join(REAL_TOOLS, f"{vendor}.json")
        for r in (json.load(open(rp))["records"] if os.path.exists(rp) else []):
            shop(vendor, r, outs, skips)
            counts["shop tools"] += 1
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
