"""Generates MACHINE-LIB C's vendor machine files (kernel/forge-machine/data/machines/<vendor>.toml)
and their envelope parts (parts/<vendor>/<machine>/*.fg) from data.py, where every number carries
its citation. Run from the repository root: python3 -I tools/machines/vendor_c/gen.py .
The generated files are committed; edit data.py (or the builders here) and re-run."""
import os, sys

REPO = sys.argv[1]
MDIR = os.path.join(REPO, "kernel/forge-machine")
T = "2026-11-23"


def fmt(v):
    if isinstance(v, bool):
        return "true" if v else "false"
    if isinstance(v, (int, float)):
        f = float(v)
        return str(int(f)) if f == int(f) else repr(round(f, 4))
    if isinstance(v, str):
        return '"' + v.replace('\\', '\\\\').replace('"', '\\"') + '"'
    if isinstance(v, (list, tuple)):
        return "[" + ", ".join(fmt(x) for x in v) + "]"
    raise TypeError(v)


class M:
    def __init__(self, vendor, name, kind, control, source, note, coolants, length_joint="Z", clearance=1.0):
        self.vendor, self.name = vendor, name
        self.head = dict(name=name, kind=kind, control=control, length_joint=length_joint,
                         coolants=coolants, clearance=clearance, source=source, taken=T, note=note, real=True)
        self.joints, self.spindles, self.changer, self.mounts, self.bodies, self.cites = [], [], None, [], [], []
        self.parts = {}

    def cite(self, path, status, source=None, value=None, basis=None, note=None):
        c = dict(path=path, status=status)
        if source:
            c["source"] = source
            c["taken"] = T
        if value:
            c["value"] = value
        if basis:
            c["basis"] = basis
        if note:
            c["note"] = note
        self.cites.append(c)

    def body(self, name, link, boxes=(), cyls=(), ignore=(), comment=""):
        part = f"parts/{self.vendor}/{self.name}/{name}.fg"
        b = dict(name=name, link=link, part=part)
        if ignore:
            b["ignore"] = list(ignore)
        self.bodies.append((b, list(boxes), list(cyls)))
        self.parts[part] = fg_part(self.name, name, boxes, cyls, comment)

    def toml(self):
        o = []
        o.append("[[machine]]")
        for k, v in self.head.items():
            if v is None:
                continue
            o.append(f"{k} = {fmt(v)}")
        for j in self.joints:
            o.append("\n[[machine.joint]]")
            o += [f"{k} = {fmt(v)}" for k, v in j.items()]
        for s in self.spindles:
            o.append("\n[[machine.spindle]]")
            o += [f"{k} = {fmt(v)}" for k, v in s.items()]
        if self.changer:
            o.append("\n[[machine.changer]]")
            o += [f"{k} = {fmt(v)}" for k, v in self.changer.items()]
        for m in self.mounts:
            o.append("\n[[machine.mount]]")
            o += [f"{k} = {fmt(v)}" for k, v in m.items()]
        for b, boxes, cyls in self.bodies:
            o.append("\n[[machine.body]]")
            o += [f"{k} = {fmt(v)}" for k, v in b.items()]
            for mn, mx in boxes:
                o.append("[[machine.body.box]]")
                o.append(f"min = {fmt(mn)}")
                o.append(f"max = {fmt(mx)}")
            for base, r, h in cyls:
                o.append("[[machine.body.cyl]]")
                o.append(f"base = {fmt(base)}")
                o.append("axis = [0, 0, 1]")
                o.append(f"r = {fmt(r)}")
                o.append(f"h = {fmt(h)}")
        for c in self.cites:
            o.append("\n[[machine.cite]]")
            o += [f"{k} = {fmt(v)}" for k, v in c.items()]
        return "\n".join(o) + "\n"


INSET = 0.5  # the part lies this far inside its envelope


def fg_part(machine, body, boxes, cyls, comment):
    o = [f"# {machine} {body}: a conservative envelope solid from published dimensions (see the",
         "# machine's [[machine.cite]] for body." + body + "); not the maker's CAD.",
         "# frame: the machine's base frame at the home pose."]
    if comment:
        o.append("# " + comment)
    feats = []
    for i, (mn, mx) in enumerate(boxes):
        x0, y0, z0 = (a + INSET for a in mn)
        x1, y1, z1 = (a - INSET for a in mx)
        p = f"b{i}"
        o += [f"shared {p}x0 = {fmt(x0)}", f"shared {p}y0 = {fmt(y0)}", f"shared {p}z0 = {fmt(z0)}",
              f"shared {p}sx = {fmt(x1 - x0)}", f"shared {p}sy = {fmt(y1 - y0)}", f"shared {p}h = {fmt(z1 - z0)}"]
        feats.append((p, "box"))
    for i, (base, r, h) in enumerate(cyls):
        p = f"c{i}"
        o += [f"shared {p}x = {fmt(base[0])}", f"shared {p}y = {fmt(base[1])}",
              f"shared {p}z0 = {fmt(base[2] + INSET)}", f"shared {p}r = {fmt(r - INSET)}",
              f"shared {p}h = {fmt(h - 2 * INSET)}"]
        feats.append((p, "cyl"))
    o.append("")
    for p, k in feats:
        if k == "box":
            o += [f"sketch s{p}(on: offset(xy, {p}z0)) {{",
                  f"  outline = rect(corner: ({p}x0, {p}y0), size: ({p}sx, {p}sy))", "}",
                  f"{p} = extrude(s{p}, {p}h)"]
        else:
            o += [f"sketch s{p}(on: offset(xy, {p}z0)) {{",
                  f"  c = circle(({p}x, {p}y), {p}r)", "}",
                  f"{p} = extrude(s{p}.c, {p}h)"]
    return "\n".join(o) + "\n"


# ------------------------------------------------------------------------------------------------
ZERO_BASIS = ("travel length published; where the control's machine zero sits in it is not; we put "
              "zero at the + end (range [-travel, 0]); a setup's WCS absorbs the choice")


def vmc3(vendor, name, control, src, note, *, tx, ty, tz, hmin, hmax, D, table, rapid, feed, rpm, iface,
         tmax, changer, cites, coolants=("flood",), nose_r=75, head=(300, 420), tt=60, rapid_src=None,
         hmin_status="published"):
    """Knee-less VMC: table X on saddle Y (part side), head Z on the column. Origin: the spindle axis at
    the table-top plane, home pose (all joints at 0 = their + ends)."""
    m = M(vendor, name, "vmc3", control, src, note, list(coolants))
    L, W = table
    for n, ax, rng in (("Y", [0, -1, 0], ty), ("X", [-1, 0, 0], tx)):
        m.joints.append(dict(name=n, parent="base" if n == "Y" else "Y", side="table", type="linear",
                             axis=ax, range=[-rng, 0], rapid=rapid, max_feed=feed))
    m.joints.append(dict(name="Z", parent="base", side="head", type="linear", axis=[0, 0, 1],
                         range=[-tz, 0], rapid=rapid, max_feed=feed))
    m.spindles.append(dict(name="S1", link="Z", kind="milling", axis=[0, 0, 1], nose=[0, 0, hmax],
                           rpm=rpm, max=list(tmax), interface=iface))
    if changer:
        m.changer = dict(pockets=changer[0], change_s=changer[1], max_tool_kg=changer[2])
    m.mounts.append(dict(name="spindle", kind="tool", link="Z", at=[0, 0, hmax], dir=[0, 0, -1]))
    m.mounts.append(dict(name="table", kind="part", link="X", at=[-tx / 2, -ty / 2, 0], dir=[0, 0, 1],
                         xdir=[1, 0, 0]))
    ts, hb = 120, 200
    ymax = ty / 2 + W / 2
    assert ymax < D, (name, ymax, D)
    hw, hh = head
    cd = 300
    m.body("table", "X", boxes=[([-tx / 2 - L / 2, -ty / 2 - W / 2, -tt], [-tx / 2 + L / 2, -ty / 2 + W / 2, 0])],
           ignore=["saddle"], comment="the table top plate; T-slots are inside the box")
    m.body("saddle", "Y", boxes=[([-0.3 * L, -ty / 2 - W / 2, -tt - ts], [0.3 * L, -ty / 2 + W / 2, -tt])],
           ignore=["bed", "table"])
    m.body("bed", "base", boxes=[([-0.35 * L, -ty - W / 2, -tt - ts - hb], [0.35 * L, D + cd, -tt - ts])],
           ignore=["saddle", "column"])
    top = hmax + 100 + hh
    m.body("column", "base", boxes=[([-0.3 * L, D, -tt - ts], [0.3 * L, D + cd, top + 100])],
           ignore=["bed", "head"])
    m.body("head", "Z", boxes=[([-hw / 2, -hw / 2, hmax + 100], [hw / 2, D, hmax + 100 + hh])],
           cyls=[([0, 0, hmax], nose_r, 100)], ignore=["column"])
    m.cite("machine.unmodelled", "unknown", basis="tool changer, enclosure and guarding, way covers and "
           "chip conveyor are not bodies: their geometry is unpublished; a crash into them is not checked")
    # cites
    for n, rng in (("X", tx), ("Y", ty), ("Z", tz)):
        m.cite(f"joint.{n}.range", "published", src, value=f"{n} travel {rng} mm")
        m.cite(f"joint.{n}.range", "estimate", basis=ZERO_BASIS)
        rs, rv = rapid_src or (src, f"rapid {rapid} mm/min")
        m.cite(f"joint.{n}.rapid", "published", rs, value=rv)
    for path, c in cites.items():
        for x in (c if isinstance(c, list) else [c]):
            m.cite(path, **x)
    m.cite("spindle.S1.nose", hmin_status, src if hmin_status == "published" else None,
           value=f"spindle nose to table {hmin}-{hmax} mm" if hmin_status == "published" else None,
           basis=None if hmin_status == "published" else
           f"true value unpublished; nose put {hmin} mm above the table top at Z's bottom: the least value "
           "consistent with the self-collision test, not data; the head errs low (more Hits), the tool's reach "
           "errs long")
    m.cite("mount.spindle.at", "estimate", basis="gauge point taken at the spindle nose face (see spindle.S1.nose)")
    m.cite("mount.table.at", "estimate", basis="table top centre at the travel's centre; table thickness ours")
    m.cite("body.table", "estimate", src, value=f"table {L} x {W} mm",
           basis=f"published table size; thickness {tt} mm ours; T-slots inside the box (larger than the real plate)")
    m.cite("body.saddle", "estimate", basis="unpublished; a block under the table inside its y extent; size ours")
    m.cite("body.bed", "estimate", basis="unpublished; a slab under the saddle's travel; size ours, inside the published footprint")
    m.cite("body.column", "estimate", src if D else None, value=f"spindle centre to column {D} mm" if D else None,
           basis=f"column face placed at the published spindle-to-column distance; width, depth ({cd} mm) and height ours")
    m.cite("body.head", "estimate",
           basis=f"unpublished; nose cylinder r {nose_r} mm and a {hw} mm head box are ours, sized for the "
           f"{iface} taper class; the real housing may be larger: check before trusting a head-vs-fixture Clear")
    return m


def gantry(vendor, name, control, src, note, *, tx, ty, tz, passage, rapid, feed, rpm, iface, tmax,
           changer, cites, coolants=("air",), spindle_r=45, bed_xy=None):
    """Moving-gantry router: Y gantry on the static bed, X carriage on the beam, Z on the carriage; the
    part on the bed. Origin: the spindle axis at the table-top plane, home pose (joints at + ends)."""
    m = M(vendor, name, "vmc3", control, src, note, list(coolants))
    m.joints.append(dict(name="Y", parent="base", side="head", type="linear", axis=[0, 1, 0], range=[-ty, 0],
                         rapid=rapid, max_feed=feed))
    m.joints.append(dict(name="X", parent="Y", side="head", type="linear", axis=[1, 0, 0], range=[-tx, 0],
                         rapid=rapid, max_feed=feed))
    m.joints.append(dict(name="Z", parent="X", side="head", type="linear", axis=[0, 0, 1], range=[-tz, 0],
                         rapid=rapid, max_feed=feed))
    m.spindles.append(dict(name="S1", link="Z", kind="milling", axis=[0, 0, 1], nose=[0, 0, tz + 5], rpm=rpm,
                           max=list(tmax), interface=iface))
    if changer:
        m.changer = dict(pockets=changer[0], change_s=changer[1], max_tool_kg=changer[2])
    m.mounts.append(dict(name="spindle", kind="tool", link="Z", at=[0, 0, tz + 5], dir=[0, 0, -1]))
    m.mounts.append(dict(name="bed", kind="part", link="base", at=[-tx / 2, -ty / 2, 0], dir=[0, 0, 1],
                         xdir=[1, 0, 0]))
    bx, by = bed_xy or (tx, ty)
    m.body("bed", "base", boxes=[([-tx / 2 - bx / 2, -ty / 2 - by / 2, -100], [-tx / 2 + bx / 2, -ty / 2 + by / 2, 0])],
           ignore=["gantry"])
    cw, bh, bd, uw = 160, 150, 150, 80
    beam_y0 = spindle_r + 30
    # beam spans the carriage's whole x travel plus the uprights outside it
    xl, xr = -tx - cw / 2 - uw, cw / 2 + uw
    m.body("gantry", "Y", boxes=[([xl, beam_y0, passage], [xr, beam_y0 + bd, passage + bh]),
                                 ([xl, beam_y0, -100], [xl + uw, beam_y0 + bd, passage]),
                                 ([xr - uw, beam_y0, -100], [xr, beam_y0 + bd, passage])],
           ignore=["bed", "carriage", "spindle"])
    m.body("carriage", "X", boxes=[([-cw / 2, spindle_r + 5, passage], [cw / 2, beam_y0, passage + bh + 100])],
           ignore=["gantry", "spindle"])
    m.body("spindle", "Z", cyls=[([0, 0, tz + 5], spindle_r, 300)], ignore=["carriage", "gantry"])
    m.cite("machine.unmodelled", "unknown", basis="tool magazine, enclosure, terminal and cable chains are "
           "not bodies: their geometry is unpublished; a crash into them is not checked")
    for n, rng in (("X", tx), ("Y", ty), ("Z", tz)):
        m.cite(f"joint.{n}.range", "published", src, value=f"{n} traverse {rng} mm")
        m.cite(f"joint.{n}.range", "estimate", basis=ZERO_BASIS)
    for path, c in cites.items():
        for x in (c if isinstance(c, list) else [c]):
            m.cite(path, **x)
    m.cite("spindle.S1.nose", "estimate",
           basis=f"true value unpublished; nose put 5 mm above the bed top at Z's bottom (Z travel {tz} mm): the "
           "least value consistent with the self-collision test, not data; the spindle errs low (more Hits), the "
           "tool's reach errs long")
    m.cite("mount.spindle.at", "estimate", basis="gauge point at the spindle nose (see spindle.S1.nose)")
    m.cite("mount.bed.at", "estimate", basis="bed top at the travel's centre")
    m.cite("body.bed", "estimate", basis=f"a {bx} x {by} mm slab, top = the work surface; thickness ours")
    m.cite("body.gantry", "published", src, value=f"portal passage {passage} mm",
           basis=f"beam underside at the published passage; beam {bd} x {bh} mm and uprights {uw} mm ours")
    m.cite("body.carriage", "estimate", basis=f"unpublished; a {cw} mm block between spindle and beam, ours")
    m.cite("body.spindle", "estimate",
           basis=f"unpublished; spindle cylinder r {spindle_r} mm ours; the real motor housing may differ")
    return m


def pub(src, value):
    return dict(status="published", source=src, value=value)


def est(basis, **kw):
    return dict(status="estimate", basis=basis, **kw)


def unk(basis):
    return dict(status="unknown", basis=basis)


MACHINES = []
exec(open(os.path.join(os.path.dirname(os.path.abspath(__file__)), "data.py")).read())

by_vendor = {}
for m in MACHINES:
    by_vendor.setdefault(m.vendor, []).append(m)
for v, ms in by_vendor.items():
    head = open(os.path.join(os.path.dirname(os.path.abspath(__file__)), f"head-{v}.txt")).read()
    txt = head + "\n" + "\n# " + "-" * 93 + "\n".join("\n" + m.toml() for m in ms)
    with open(os.path.join(MDIR, "data/machines", f"{v}.toml"), "w") as f:
        f.write(txt)
    for m in ms:
        for p, s in m.parts.items():
            fp = os.path.join(MDIR, p)
            os.makedirs(os.path.dirname(fp), exist_ok=True)
            with open(fp, "w") as f:
                f.write(s)
    print(v, [m.name for m in ms])
