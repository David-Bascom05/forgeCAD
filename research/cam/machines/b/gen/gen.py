#!/usr/bin/env python3
"""Emit forge-machine TOML + one .fg part per collision body from compact machine records.

Each record is a dict built in machines_*.py. Every number in it is either published (the record's
`source`) or listed in the record's `est` with why. The emitted TOML carries `source`, `taken` and a
`note` naming the estimates and UNKNOWNs. One primitive per body: the part is the envelope itself
(a conservative stand-in built from published overall dimensions, never the maker's CAD).
"""
import os, sys, math

def fmt(x):
    if isinstance(x, bool):
        return "true" if x else "false"
    if isinstance(x, (int,)):
        return str(x)
    if isinstance(x, float):
        if x == int(x) and abs(x) < 1e12:
            return str(int(x))
        return repr(round(x, 6))
    if isinstance(x, str):
        return '"' + x.replace('\\', '\\\\').replace('"', '\\"') + '"'
    if isinstance(x, (list, tuple)):
        return "[" + ", ".join(fmt(v) for v in x) + "]"
    raise TypeError(x)

# ---------------------------------------------------------------------------------------------
# bodies

def box(name, link, mn, mx, ignore=(), why=""):
    return dict(name=name, link=link, prim=("box", list(mn), list(mx)), ignore=list(ignore), why=why)

def cyl(name, link, base, axis, r, h, ignore=(), why=""):
    return dict(name=name, link=link, prim=("cyl", list(base), list(axis), r, h), ignore=list(ignore), why=why)

def part_text(mname, b):
    p = b["prim"]
    head = f"# {mname} {b['name']}: {b['why'] or 'conservative envelope'}.\n# A stand-in built from published overall dimensions and estimates (not the maker's CAD).\n# frame: the machine's base frame at the home pose.\n"
    if p[0] == "box":
        mn, mx = p[1], p[2]
        return head + (
            f"shared x0 = {fmt(float(mn[0]))}\nshared y0 = {fmt(float(mn[1]))}\nshared z0 = {fmt(float(mn[2]))}\n"
            f"shared L = {fmt(float(mx[0]-mn[0]))}\nshared W = {fmt(float(mx[1]-mn[1]))}\nshared H = {fmt(float(mx[2]-mn[2]))}\n\n"
            "sketch plan(on: offset(xy, z0)) {\n  outline = rect(corner: (x0, y0), size: (L, W))\n}\n"
            "body = extrude(plan, H)\n")
    _, base, axis, r, h = p
    ax = [i for i in range(3) if axis[i] != 0]
    assert len(ax) == 1, axis
    i, s = ax[0], axis[ax[0]]
    lo = base[i] if s > 0 else base[i] - h
    if i == 2:
        plane, d, uv = "xy", lo, (base[0], base[1])
    elif i == 0:
        plane, d, uv = "yz", lo, (base[1], base[2])
    else:  # xz: normal -y, offset d puts the plane at y = -d; extrude runs -y
        plane, d, uv = "xz", -(lo + h), (base[0], base[2])
    return head + (
        f"shared d = {fmt(float(d))}\nshared u = {fmt(float(uv[0]))}\nshared v = {fmt(float(uv[1]))}\n"
        f"shared r = {fmt(float(r))}\nshared h = {fmt(float(h))}\n\n"
        f"sketch plan(on: offset({plane}, d)) {{\n  outline = circle((u, v), r)\n}}\n"
        "body = extrude(plan.outline, h)\n")

# ---------------------------------------------------------------------------------------------
# emit

def emit_machine(m, partdir_rel):
    o = []
    o.append("# " + "-" * 93)
    for line in m["header"]:
        o.append("# " + line)
    o.append("[[machine]]")
    o.append(f"name = {fmt(m['name'])}")
    o.append(f"kind = {fmt(m['kind'])}")
    o.append(f"control = {fmt(m['control'])}")
    if m.get("length_joint"):
        o.append(f"length_joint = {fmt(m['length_joint'])}")
    o.append(f"coolants = {fmt(m['coolants'])}")
    o.append(f"clearance = {fmt(m.get('clearance', 1.0))}")
    o.append(f"source = {fmt(m['source'])}")
    o.append(f"taken = {fmt(m['taken'])}")
    o.append(f"note = {fmt(m['note'])}")
    for j in m["joints"]:
        o.append("")
        o.append("[[machine.joint]]")
        for k in ["name", "letter", "parent", "side", "type", "axis", "pivot", "range", "home", "rapid", "max_feed", "accel", "cyclic", "tcp", "preference", "diameter"]:
            if k in j and j[k] is not None:
                c = j.get("c_" + k)
                o.append(f"{k} = {fmt(j[k])}" + (f"   # {c}" if c else ""))
    for s in m["spindles"]:
        o.append("")
        o.append("[[machine.spindle]]")
        for k in ["name", "link", "kind", "axis", "nose", "rpm", "curve_rpm", "curve_kw", "max", "interface", "c_joint"]:
            if k in s and s[k] is not None:
                c = s.get("c_" + k)
                o.append(f"{k} = {fmt(s[k])}" + (f"   # {c}" if c else ""))
    for t in m.get("turrets", []):
        o.append("")
        o.append("[[machine.turret]]")
        for k in ["name", "link", "stations", "index_s", "center", "index_axis", "live"]:
            if k in t and t[k] is not None:
                c = t.get("c_" + k)
                o.append(f"{k} = {fmt(t[k])}" + (f"   # {c}" if c else ""))
    if m.get("changer"):
        c = m["changer"]
        o.append("")
        o.append("[[machine.changer]]")
        for k in ["pockets", "change_s", "max_tool_kg"]:
            cc = c.get("c_" + k)
            o.append(f"{k} = {fmt(c[k])}" + (f"   # {cc}" if cc else ""))
    for mt in m["mounts"]:
        o.append("")
        o.append("[[machine.mount]]")
        for k in ["name", "kind", "link", "at", "dir", "xdir"]:
            if k in mt:
                o.append(f"{k} = {fmt(mt[k])}")
    for b in m["bodies"]:
        o.append("")
        o.append("[[machine.body]]")
        o.append(f"name = {fmt(b['name'])}")
        o.append(f"link = {fmt(b['link'])}")
        o.append(f"part = {fmt(partdir_rel + '/' + b['name'] + '.fg')}")
        if b["ignore"]:
            o.append(f"ignore = {fmt(b['ignore'])}")
        p = b["prim"]
        if p[0] == "box":
            o.append("[[machine.body.box]]")
            o.append(f"min = {fmt(p[1])}")
            o.append(f"max = {fmt(p[2])}")
        else:
            o.append("[[machine.body.cyl]]")
            o.append(f"base = {fmt(p[1])}")
            o.append(f"axis = {fmt(p[2])}")
            o.append(f"r = {fmt(p[3])}")
            o.append(f"h = {fmt(p[4])}")
    return "\n".join(o) + "\n"

def write_vendor(root, vendor, title, machines):
    tdir = os.path.join(root, "kernel/forge-machine/data/machines")
    os.makedirs(tdir, exist_ok=True)
    out = [f"# {title}: the maker's published specifications, restated (never copied); each record's",
           "# `source` is the page or catalogue read and `taken` the date. Estimates and UNKNOWNs are named in",
           "# each record's `note` and beside the field. Bodies are conservative envelopes from published",
           "# overall dimensions, not the maker's CAD. Format: docs/cam-machine.md.", ""]
    for m in machines:
        model = m["name"].split("-", 1)[1] if m["name"].startswith(vendor + "-") else m["name"]
        prel = f"parts/{vendor}/{model}"
        pdir = os.path.join(root, "kernel/forge-machine", prel)
        os.makedirs(pdir, exist_ok=True)
        for b in m["bodies"]:
            with open(os.path.join(pdir, b["name"] + ".fg"), "w") as f:
                f.write(part_text(m["name"], b))
        out.append(emit_machine(m, prel))
    with open(os.path.join(tdir, vendor + ".toml"), "w") as f:
        f.write("\n".join(out))
