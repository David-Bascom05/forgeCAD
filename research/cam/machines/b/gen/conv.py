"""Spec JSON (research agents' restatement of the makers' pages) -> template records.

Published numbers go in as they are; an unpublished number the geometry needs becomes an
estimate named in the note; an unpublished number a check needs takes its vacuous value (common.py)
named UNKNOWN beside the field. A machine whose defining numbers are unpublished (travels, spindle
rpm, a tilt range) is skipped and listed, never filled in."""
import json, os, re
from common import *
import arch

SPECS = "/tmp/claude-0/-home-user-forgecad/f84f38ce-577c-5af8-87d9-367113b8c34b/scratchpad/specs"

def load(slug):
    return json.load(open(os.path.join(SPECS, slug + ".json")))

def num(x):
    return float(x) if isinstance(x, (int, float)) else None

def clean_url(u):
    return u.split(" ")[0]

def sources(d):
    s = [clean_url(u) for u in d.get("source") or []]
    sec = [clean_url(u) for u in d.get("source_secondary") or []]
    out = s + sec
    extra = []
    if sec and not s:
        extra.append("numbers from a page hosted by a dealer/importer/marketplace (" + ", ".join(sec) + "), not the maker's own site")
    elif sec:
        extra.append("some numbers from " + ", ".join(sec) + " (not the maker's own site): see the research file")
    return " | ".join(out), extra

def dialect(d, lathe):
    c = ((d.get("control") or {}).get("standard") or "").lower()
    v = d["vendor"].lower()
    if "heidenhain" in c: return "heidenhain"
    if "siemens" in c or "sinumerik" in c: return "siemens"
    if "osp" in c or "okuma" in v: return "okuma-osp-lathe" if lathe else "okuma-osp"
    if "mazatrol" in c or "mazak" in v: return "mazak-eia-lathe" if lathe else "mazak-eia"
    if "brother" in c: return "brother"
    if any(k in c for k in ["fanuc", "g-tech", "professional", "pro6", "arumatik", "mapps"]):
        return "fanuc-lathe" if lathe else "fanuc"
    return None

def rapid_mm(d, k):
    v = num((d.get("rapid_m_min") or {}).get(k))
    return mm_min(v) if v else None

def rot_rapid(d, key):
    v = num((d.get("rapid_m_min") or {}).get(key))
    return rpm_to_deg_min(v) if v else None

def feed_mm(d):
    v = d.get("max_cutting_feed_m_min")
    v = num(v)
    return mm_min(v) if v else None

def changer(d):
    c = d.get("changer") or {}
    if not c.get("pockets"):
        return None, []
    n = []
    if c.get("chip_to_chip_s"):
        cs, cc = c["chip_to_chip_s"], "chip to chip, published" + (f" (tool to tool {c['tool_to_tool_s']})" if c.get("tool_to_tool_s") else "")
    elif c.get("tool_to_tool_s"):
        cs, cc = c["tool_to_tool_s"], "tool to tool, published (chip to chip UNKNOWN): a lower bound"
    else:
        cs, cc = 0.001, UNK_CHANGE
    kg, ck = (c["max_tool_kg"], None) if c.get("max_tool_kg") else (99.0, UNK_KG)
    ch = dict(pockets=int(c["pockets"]), change_s=cs, kg=kg, c_change_s=cc)
    if ck:
        ch["c_kg"] = ck
    if c.get("options"):
        n.append("magazine options: " + ", ".join(str(o) for o in c["options"]))
    return ch, n

def maxtool(d):
    c = d.get("changer") or {}
    L, D = num(c.get("max_tool_len_mm")), num(c.get("max_tool_dia_mm"))
    com = []
    if L is None:
        L = 1000.0; com.append("max tool length UNKNOWN: 1000 (no check)")
    if D is None:
        D = 500.0; com.append("max tool diameter UNKNOWN: 500 (no check)")
    elif c.get("max_tool_dia_empty_adjacent_mm"):
        com.append(f"max tool diameter {D:g} with the neighbouring pockets full, {c['max_tool_dia_empty_adjacent_mm']:g} with them empty")
    return (L, D), com

def spindle(d):
    sp = d.get("spindle") or {}
    s = dict(rpm_max=int(sp["rpm_max"]), iface=(sp.get("taper_or_interface") or "UNKNOWN")[:60])
    if sp.get("rpm_min"):
        s["rpm_min"] = int(sp["rpm_min"])
    pw = sp.get("power_kw") or {}
    tq = sp.get("torque_nm") or {}
    bits = []
    for k, v in pw.items():
        if v: bits.append(f"{k} {v} kW" if isinstance(v, (int, float)) else f"{k}: {v}")
    for k, v in tq.items():
        if v: bits.append(f"torque {k} {v} N.m")
    txt = "spindle power " + ("as published: " + "; ".join(bits) if bits else "UNKNOWN") + " (no curve: no power check)"
    return s, txt

def common_notes(d):
    n = []
    if d.get("notes"):
        pass
    if d.get("overall_mm") and any((d["overall_mm"] or {}).values()):
        o = d["overall_mm"]
        n.append("overall " + " x ".join(f"{o.get(k)}" for k in "WDH"))
    if d.get("weight_kg"):
        n.append(f"machine {d['weight_kg']} kg")
    return n

def mark_unknowns(m, rap_known, feed_known, rot_known=()):
    for j in m["joints"]:
        if j["type"] == "linear":
            if not rap_known.get(j["name"], True):
                j["c_rapid"] = UNK_RAPID
            if not feed_known:
                j["c_max_feed"] = UNK_FEED
        else:
            if j["name"] not in rot_known:
                j["c_rapid"] = UNK_RAPID
            j.setdefault("c_max_feed", "rotary max feed UNKNOWN: = rapid")

def linear_rates(d, letters="XYZ"):
    rap, known = [], {}
    for k in letters:
        r = rapid_mm(d, k)
        known[k] = r is not None
        rap.append(r if r else VAC_RAPID)
    f = feed_mm(d)
    return rap, known, f

# ---------------------------------------------------------------------------------------------
def vmc(slug, name, header, extra=(), **ov):
    d = load(slug)
    t = d["travel_mm"]
    X, Y, Z = (num(t.get(k)) for k in "XYZ")
    est = []
    tb = d.get("table_mm")
    if not tb:
        tb = [X + 200, Y]; est.append(f"table size UNKNOWN: {tb[0]:g} x {tb[1]:g} (estimate)")
    nose = d.get("spindle_nose_to_table_mm")
    if not nose or nose[0] is None or nose[1] is None:
        nose = [150.0, 150.0 + Z]; est.append("spindle nose to table UNKNOWN: 150 to 150 + Z (estimate)")
    rap, known, f = linear_rates(d)
    fr = f if f else max(rap)
    sp, ptxt = spindle(d)
    mt, mtc = maxtool(d)
    ch, chn = changer(d)
    col = num(d.get("spindle_center_to_column_mm"))
    colc = None
    if not col or col < tb[1] / 2 + Y / 2 + 1:
        colc = "spindle centre to column " + ("UNKNOWN" if not col else f"published {col:g} is less than the table's reach at full Y (the column's lower part sits behind the saddle)") + ": put at the least that clears the table at full Y (estimate)"
        col = tb[1] / 2 + Y / 2 + 60
    ov = d.get("overall_mm") or {}
    overall = tuple(num(ov.get(k)) for k in "WDH")
    p = dict(name=name, travel=(X, Y, Z), table=tuple(tb), nose=tuple(nose), col=col, rapid=tuple(rap), feed=fr,
             spindle=sp, maxtool=mt, changer=ch, overall=overall if all(overall) else None)
    p.update(ov)
    m = arch.vmc_cframe(p)
    mark_unknowns(m, known, f is not None)
    if any(e.startswith("spindle nose") for e in est):
        nose_estimate(m, "spindle nose to table UNKNOWN: 150 at the bottom of Z (estimate)")
    if mtc and "UNKNOWN" in " ".join(mtc):
        over(m, "spindle.S1.max", "unknown", "; ".join(mtc))
    if ch is None:
        m["changer"] = None
    src, sn = sources(d)
    ctl = ov.get("control") or dialect(d, False)
    m.update(name=name, control=ctl or "unknown", coolants=["flood"], source=src, taken=d.get("taken", TAKEN), header=header,
             note=note(*extra, "architecture assumed: X table on a Y saddle, Z head on a fixed column (the maker's page does not say which unit carries which: UNKNOWN unless stated above)",
                       colc, *est, ptxt, *mtc, *chn, *sn, *common_notes(d),
                       None if ch else "tool changer UNKNOWN (not published): none declared",
                       VACUOUS, CONV_VMC, ENV,
                       None if ctl in ("fanuc", "heidenhain", "siemens", None) else f"the dialect ({ctl}) is not in forge-post yet",
                       "control UNKNOWN (not published)" if not ctl else None))
    return m

def trunnion(slug, name, header, tilt_letter, tilt_range, extra=(), **ov):
    d = load(slug)
    t = d["travel_mm"]
    X, Y, Z = (num(t.get(k)) for k in "XYZ")
    est = []
    td = num(d.get("rotary_table_dia_mm"))
    if not td:
        tb = d.get("table_mm")
        td = num(tb[0]) if tb else 500.0
        est.append(f"rotary table diameter UNKNOWN: {td:g} (estimate)")
    nose = d.get("spindle_nose_to_table_mm")
    if not nose or nose[1] is None:
        nose = [100.0, 100.0 + Z]; est.append("spindle nose to table at tilt 0 UNKNOWN: 100 to 100 + Z (estimate)")
    elif nose[0] is None:
        nose = [nose[1] - Z, nose[1]]; est.append("spindle nose to table: the published maximum less the Z stroke")
    rap, known, f = linear_rates(d)
    fr = f if f else max(rap)
    rt, rc = rot_rapid(d, "A_or_B_rpm"), rot_rapid(d, "C_rpm")
    rk = tuple(k for k, v in ((tilt_letter, rt), ("C", rc)) if v)
    sp, ptxt = spindle(d)
    mt, mtc = maxtool(d)
    ch, chn = changer(d)
    p = dict(name=name, travel=(X, Y, Z), tilt=(tilt_letter, tilt_range), pivot_below_top=ov.pop("pivot", 150.0),
             table_d=td, nose=tuple(nose), rapid=tuple(rap), feed=fr, rapid_tilt=rt or VAC_ROT, rapid_c=rc or VAC_ROT,
             spindle=sp, maxtool=mt, changer=ch, nose_r=110.0, cradle_side=80.0)
    p.update(ov)
    m = arch.trunnion(p)
    mark_unknowns(m, known, f is not None, rk)
    if any(e.startswith("spindle nose") for e in est):
        nose_estimate(m, [e for e in est if e.startswith("spindle nose")][0])
    if mtc and "UNKNOWN" in " ".join(mtc):
        over(m, "spindle.S1.max", "unknown", "; ".join(mtc))
    src, sn = sources(d)
    ctl = ov.get("control") or dialect(d, False)
    axis = "X" if tilt_letter == "A" else "Y"
    m.update(name=name, control=ctl or "unknown", coolants=["flood"], source=src, taken=d.get("taken", TAKEN), header=header,
             note=note(*extra,
                       f"{tilt_letter} tilts the cradle about {axis} ({tilt_range[0]:g}..{tilt_range[1]:g} deg, published) and carries C; the sense of +{tilt_letter} (which way the table tips) is UNKNOWN: modelled right-handed about +{axis.lower()}",
                       f"{tilt_letter}'s pivot below the table top UNKNOWN: {p['pivot_below_top']:g} mm (estimate)",
                       "which unit carries X/Y/Z: see the header (where not published, all on the head here); the tool-to-part motion is the same either way",
                       *est, ptxt, *mtc, *chn, *sn, *common_notes(d), VACUOUS, CONV_VMC, ENV,
                       None if ctl in ("fanuc", "heidenhain", "siemens", None) else f"the dialect ({ctl}) is not in forge-post yet",
                       "control UNKNOWN (not published)" if not ctl else None))
    return m

def hmc(slug, name, header, extra=(), **ov):
    d = load(slug)
    t = d["travel_mm"]
    X, Y, Z = (num(t.get(k)) for k in "XYZ")
    est = []
    P = num((d.get("pallet") or {}).get("size_mm", [None])[0])
    cy = d.get("spindle_center_to_pallet_top_mm")
    if not cy or None in cy:
        cy = [100.0, 100.0 + Y]; est.append("spindle centre above the pallet UNKNOWN: 100 to 100 + Y (estimate)")
    rap, known, f = linear_rates(d)
    fr = f if f else max(rap)
    rb = rot_rapid(d, "A_or_B_rpm")
    sp, ptxt = spindle(d)
    mt, mtc = maxtool(d)
    ch, chn = changer(d)
    p = dict(name=name, travel=(X, Y, Z), pallet=P, spindle_above_pallet=tuple(cy), nose_to_center=(100.0, 100.0 + Z),
             rapid=tuple(rap), feed=fr, rapid_b=rb or VAC_ROT, spindle=sp, maxtool=mt, changer=ch)
    p.update(ov)
    m = arch.hmc(p)
    mark_unknowns(m, known, f is not None, ("B",) if rb else ())
    nose_estimate(m, "spindle nose to pallet centre not published: 100 at the end of Z (estimate)")
    if mtc and "UNKNOWN" in " ".join(mtc):
        over(m, "spindle.S1.max", "unknown", "; ".join(mtc))
    src, sn = sources(d)
    ctl = ov.get("control") or dialect(d, False)
    pal = d.get("pallet") or {}
    m.update(name=name, control=ctl or "unknown", coolants=["flood"], source=src, taken=d.get("taken", TAKEN), header=header,
             note=note(*extra,
                       f"pallets: {pal.get('count') or 'count UNKNOWN'}, {P:g} square, change time {pal.get('change_s') or 'UNKNOWN'}, load {pal.get('load_kg') or 'UNKNOWN'} kg; the APC is not in the format (FORMAT-GAPS B1): the pallet at the spindle is the part mount, kind vmc4",
                       "spindle nose to pallet centre UNKNOWN: 100 to 100 + Z (estimate)",
                       "Z modelled on the head (X column, Y head, Z ram) because the format needs the length joint on the head side (FORMAT-GAPS B10); the tool-to-part motion is the same either way",
                       *est, ptxt, *mtc, *chn, *sn, *common_notes(d), VACUOUS,
                       "joint 0 = mid X, Y 0 = spindle lowest (home at the top), Z 0 = spindle farthest from the pallet; machine zero UNKNOWN", ENV,
                       None if ctl in ("fanuc", "heidenhain", "siemens", None) else f"the dialect ({ctl}) is not in forge-post yet",
                       "control UNKNOWN (not published)" if not ctl else None))
    return m

def lathe(slug, name, header, extra=(), **ov):
    d = load(slug)
    t = d["travel_mm"]
    Xs, Zs = num(t.get("X")), num(t.get("Z"))
    est = []
    D = num(d.get("max_turning_dia_mm"))
    L = num(d.get("max_turning_length_mm"))
    if not L:
        L = Zs - 50; est.append(f"max turning length UNKNOWN: Z stroke - 50 = {L:g} (estimate)")
    ch = num(d.get("chuck_size_in"))
    if ch:
        cd = ch * 25.4
    else:
        cd = min(D, 254.0); est.append(f"chuck size UNKNOWN: {cd:g} mm (estimate)")
    sw = num(d.get("swing_over_bed_mm")) or D
    if not d.get("swing_over_bed_mm"):
        est.append("swing over bed UNKNOWN: the max turning diameter used")
    st = d.get("turret_stations")
    rap, known, f = linear_rates(d, "XZ")
    fr = f if f else max(rap)
    sp, ptxt = spindle(d)
    if d.get("bar_capacity_mm"):
        sp["iface"] = (d.get("spindle_nose_type") or "") + f" (bar {d['bar_capacity_mm']:g})"
    elif d.get("spindle_nose_type"):
        sp["iface"] = d["spindle_nose_type"]
    p = dict(name=name, stroke=(Xs, Zs), max_dia=D, max_len=L, chuck_d=cd, swing=sw, stations=int(st) if st else 24,
             rapid=tuple(rap), feed=fr, spindle=sp, index_s=num(d.get("turret_index_s")) or 0.001)
    p.update(ov)
    m = arch.lathe(p)
    mark_unknowns(m, dict(zip("XZ", known.values())), f is not None)
    if any(e.startswith("swing") or e.startswith("max turning length") for e in est):
        over(m, "spindle.main.max", "estimate", "; ".join(e for e in est if e.startswith(("swing", "max turning"))))
    if not st:
        m["turrets"][0]["c_stations"] = "UNKNOWN (unpublished): 24, no T-number check"
    if not d.get("turret_index_s"):
        m["turrets"][0]["c_index_s"] = "UNKNOWN (unpublished): 0.001 s (must be > 0)"
    src, sn = sources(d)
    ctl = ov.get("control") or dialect(d, True)
    m.update(name=name, control=ctl or "unknown", coolants=["flood"], source=src, taken=d.get("taken", TAKEN), header=header,
             note=note(*extra, *est, ptxt, *sn, *common_notes(d), CONV_LATHE, VACUOUS, ENV,
                       "the base frame is turned so x is the slide's X (a slant bed's angle is not modelled); the tailstock is not modelled",
                       None if ctl in ("fanuc-lathe", None) else f"the dialect ({ctl}) is not in forge-post yet",
                       "control UNKNOWN (not published)" if not ctl else None))
    return m

def head(slug, title, more=""):
    d = load(slug)
    t = {k: v for k, v in (d.get("travel_mm") or {}).items() if isinstance(v, (int, float))}
    sp = d.get("spindle") or {}
    ch = (d.get("changer") or {}).get("pockets")
    ctl = (d.get("control") or {}).get("standard")
    line1 = f"{title}: " + (d.get("family") or d.get("kind") or "") + (f", {ctl}" if ctl else "") + "."
    tr = " ".join(f"{k} {v:g}" for k, v in t.items())
    line2 = f"{tr}; {sp.get('rpm_max')} rpm {sp.get('taper_or_interface') or ''}".strip() + (f"; {ch} tools" if ch else "") + "."
    out = [line1[:100], line2[:100]]
    if more:
        out.append(more)
    arch_ = d.get("architecture")
    return out

def every(kind, slugs):
    pass
