#!/usr/bin/env python3
"""Research notes (research/bendsim/env/data/*-tooling.md: `key = value  # status | source | statement`
blocks) -> forge-brake library TOML (kernel/forge-brake/data/tooling/<maker>.toml), every value
cited. Usage: md2toml.py NOTES.md OUTDIR. Rules (stated in each record's cites):
- working geometry (height, tip radius/angle; V, angle, shoulder radius, die height) only when the
  notes hold a number: a missing one is left out, so the library lists the tool as UNKNOWN;
- body width: twice the largest horizontal body dimension on the drawing (an envelope about the
  tip line that errs large), else unknown;
- a kind not named: straight (no throat: more tool, errs large);
- loads in t/m converted at 9.80665 kN/t; segment lists: one piece of each single length listed.
"""
import re, sys, os
from collections import defaultdict

TAKEN = "2026-11-24"

def parse(path):
    recs, cur = [], None
    for line in open(path, encoding="utf-8"):
        m = re.match(r"^\[\[(punch|die)\]\]", line)
        if m:
            cur = {"_kind": m.group(1), "_f": {}}
            recs.append(cur)
            continue
        if cur is None:
            continue
        m = re.match(r"^([A-Za-z0-9_]+) = (.*)$", line.rstrip("\n"))
        if not m:
            if line.strip() == "" or line.startswith("#"):
                continue
            cur = None if line.startswith("##") else cur
            continue
        key, rest = m.group(1), m.group(2)
        # value up to the first '#' outside quotes
        val, i, q = "", 0, False
        while i < len(rest):
            c = rest[i]
            if c == '"':
                q = not q
            if c == "#" and not q:
                break
            val += c
            i += 1
        comment = rest[i + 1:].strip() if i < len(rest) else ""
        parts = [p.strip() for p in comment.split("|")]
        status = parts[0] if parts else ""
        source = parts[1] if len(parts) > 1 else ""
        stmt = "|".join(parts[2:]).strip() if len(parts) > 2 else ""
        val = val.strip()
        if val.startswith('"'):
            v = val.strip('"')
        else:
            try:
                v = float(val)
            except ValueError:
                v = val
        cur["_f"][key] = (v, status, source, stmt)
    return recs

def num(f, k):
    x = f.get(k)
    if x is None or isinstance(x[0], str):
        if x and isinstance(x[0], str):
            m = re.match(r"^\s*([0-9]+(?:[.,][0-9]+)?)\s*kN/m", x[0])
            if m:
                return float(m.group(1).replace(",", "."))
        return None
    return x[0]

def q(s):
    return '"' + str(s).replace("\\", "\\\\").replace('"', '\\"') + '"'

def url(src):
    m = re.search(r"https?://\S+", src)
    return m.group(0) if m else src

def cite(kind, path, entry, status=None, basis=None, value=None):
    v, st, src, stmt = entry if entry else (None, "", "", "")
    word = (st.split()[0] if st else "").strip("(;,").lower()
    status = status or ({"published": "published", "estimate": "estimate", "conflict": "conflict"}.get(word, "published" if src.startswith("http") else "estimate"))
    out = [f"[[{kind}.cite]]", f"path = {q(path)}", f"status = {q(status)}"]
    if src:
        page = re.search(r"\((p\.[^)]*)\)", src)
        out.append(f"source = {q(url(src) + (' ' + page.group(1) if page else ''))}")
        out.append(f"taken = {q(TAKEN)}")
    stmt = value or stmt or (st if st else "")
    if stmt:
        out.append(f"value = {q(stmt)}")
    if status in ("estimate", "unknown"):
        out.append(f"basis = {q(basis or st)}")
    if status == "conflict":
        out.append(f"note = {q(st)}")
    return "\n".join(out)

def clamp(s, maker):
    s = s.lower()
    if "trumpf" in s:
        return "trumpf"
    if "bystronic" in s:
        return "bystronic-rf"
    if "amada" in s:
        return "amada-std" if "not afh" in s else "amada-afh"
    if "wila" in s or "new standard" in s:
        return "wila-ns"
    if "american" in s:
        return "american"
    if "promecam" in s or "european" in s:
        return "promecam"
    return maker.lower()

def kind_of(s):
    s = (s or "").lower()
    if "gooseneck" in s or "offset" in s:
        return "gooseneck"
    if "acute" in s:
        return "acute"
    if "straight" in s:
        return "straight"
    return None

def segs(f):
    for k in ("segments_mat_no", "segments_part_no"):
        if k in f and isinstance(f[k][0], str):
            ls = []
            for tok in f[k][0].split():
                m = re.match(r"^([0-9]+(?:\.[0-9]+)?):", tok)
                if m:
                    ls.append(float(m.group(1)))
            if ls:
                return sorted(set(ls), reverse=True), k
    return None, None

def emit(r):
    f, kind = r["_f"], r["_kind"]
    lines, cites, notes = [], [], []
    rid = f["id"][0]
    maker = f.get("maker", ("",))[0]
    lines += [f"[[{kind}]]", f"id = {q(rid)}", f"maker = {q(maker)}"]
    if "name" in f:
        lines.append(f"name = {q(f['name'][0])}")
    lines.append("real = true")
    cl = f.get("clamping", ("",))[0]
    lines.append(f"clamp = {q(clamp(cl, maker))}")
    cites.append(cite(kind, "record", f.get("clamping"), value="clamping: " + cl))
    def put(path, v, entry, **kw):
        lines.append(f"{path} = {round(v, 4):g}" if "." not in path else None)
        cites.append(cite(kind, path, entry, **kw))
    if kind == "punch":
        k = kind_of(f.get("kind", ("",))[0])
        if k is None:
            lines.append('kind = "straight"')
            cites.append(cite(kind, "record", f.get("kind"), status="estimate", basis="kind not published: modelled straight, no throat (more tool: errs large)", value="kind: UNKNOWN"))
        else:
            lines.append(f"kind = {q(k)}")
            cites.append(cite(kind, "record", f.get("kind"), value="kind: " + str(f["kind"][0])))
        for src, dst in (("height_mm", "height"), ("tip_radius_mm", "tip_radius"), ("tip_angle_deg", "tip_angle")):
            v = num(f, src)
            if v is not None:
                put(dst, v, f[src])
            else:
                cites.append(cite(kind, dst, f.get(src), status="unknown", basis="not in the published catalogue pages read"))
        dims = [(num(f, k), k) for k in ("body_width_at_clamp_mm", "body_width_mm", "dim_body_overall_mm", "neck_thickness_mm", "shoulder_width_mm", "top_width_mm", "blade_thickness_mm", "shoulder_thickness_mm")]
        dims = [d for d in dims if d[0] is not None]
        if dims:
            w, k = max(dims)
            put("width", 2 * w, f[k], status="estimate", basis=f"twice the largest horizontal body dimension on the drawing ({k} = {w:g} mm): a body symmetric about the tip line that holds it, errs large")
        else:
            cites.append(cite(kind, "width", None, status="unknown", basis="no body dimension published"))
        td = num(f, "throat_depth_mm")
        if td is not None and k is not None and kind_of(f.get("kind", ("",))[0]) == "gooseneck":
            lines.append(f"relief = {{ depth = {td:g} }}")
            cites.append(cite(kind, "relief.depth", f["throat_depth_mm"]))
        elif kind_of(f.get("kind", ("",))[0]) == "gooseneck":
            cites.append(cite(kind, "relief.depth", f.get("throat_depth_mm"), status="unknown", basis="throat not dimensioned: the gooseneck is refused (UNKNOWN geometry)"))
    else:
        for src, dst in (("v_opening_mm", "v"), ("v_angle_deg", "angle"), ("shoulder_radius_mm", "shoulder_radius"), ("die_height_mm", "height")):
            v = num(f, src)
            if v is not None:
                put(dst, v, f[src])
            else:
                cites.append(cite(kind, dst, f.get(src), status="unknown", basis="not in the published catalogue pages read"))
        vd = num(f, "v_depth_mm")
        if vd is not None:
            put("floor", vd, f["v_depth_mm"])
        w = num(f, "die_width_mm")
        fw = num(f, "foot_width_mm")
        if w is not None or fw is not None:
            if fw is not None and (w is None or fw > w):
                put("width", fw, f["foot_width_mm"], status="estimate", basis="the foot's width, wider than the body: the envelope errs large")
            else:
                put("width", w, f["die_width_mm"])
        else:
            cites.append(cite(kind, "width", f.get("die_width_mm"), status="unknown", basis="not in the published catalogue pages read"))
    ml = num(f, "max_load") or num(f, "max_load_kN_per_m")
    mlk = "max_load" if num(f, "max_load") else "max_load_kN_per_m"
    if ml is not None:
        put("max_load", ml, f[mlk])
    elif num(f, "max_load_t_per_m") is not None:
        t = num(f, "max_load_t_per_m")
        put("max_load", t * 9.80665, f["max_load_t_per_m"], status="published", value=f"{f['max_load_t_per_m'][3]} ({t:g} t/m x 9.80665 kN/t)")
    ls, sk = segs(f)
    if ls:
        lines.append("segments = [" + ", ".join(f"{{ length = {l:g}, count = 1 }}" for l in ls) + "]")
        cites.append(cite(kind, "segments", f[sk], status="estimate", basis="the published single lengths, one piece of each (what a shop holds varies)"))
    lines = [l for l in lines if l]
    return "\n".join(lines) + "\n\n" + "\n\n".join(cites) + "\n"

def main():
    recs = parse(sys.argv[1])
    by = defaultdict(list)
    for r in recs:
        if "id" in r["_f"]:
            by[r["_f"]["id"][0].split("/")[0]].append(r)
    for maker, rs in by.items():
        p = os.path.join(sys.argv[2], f"{maker}.toml")
        with open(p, "w", encoding="utf-8") as o:
            o.write(f"# {maker} press-brake tooling: forgecad's records restated from the maker's openly published\n"
                    f"# catalogue (research/bendsim/env/data/), every value cited. GENERATED by tools/bendsim/md2toml.py;\n"
                    f"# edit the notes and re-run. Verify against the maker's catalogue before use.\n\n")
            o.write("\n".join(emit(r) for r in rs))
        print(p, len(rs))

main()
