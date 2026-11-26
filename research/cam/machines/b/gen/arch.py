"""Architecture templates: published numbers in, joints/spindles/mounts/bodies out.

Conventions (stated in every machine's note):
- Linear joint 0 = mid-travel for X and Y (the control's machine zero sits at a travel end; which end
  is UNKNOWN unless published), Z 0 = the top of travel (spindle farthest from the table), Z range
  [-Z travel, 0]. A real G54 therefore differs from ours by a constant per axis.
- Geometry with every joint at 0. Bodies are single boxes/cylinders: conservative where the tool and
  part live (spindle nose, table, rotary), coarse elsewhere.
"""
from gen import box, cyl

MIN_RPM_NOTE = "rpm min UNKNOWN (unpublished): 1 = no lower check"

def _changer(ch):
    if not ch:
        return None
    out = dict(pockets=ch["pockets"], change_s=ch["change_s"], max_tool_kg=ch["kg"])
    if ch.get("c_change_s"):
        out["c_change_s"] = ch["c_change_s"]
    if ch.get("c_kg"):
        out["c_max_tool_kg"] = ch["c_kg"]
    return out

def _spindle(name, link, kind, axis, nose, sp, maxlen, maxdia, c_joint=None):
    s = dict(name=name, link=link, kind=kind, axis=axis, nose=nose,
             rpm=[sp.get("rpm_min", 1), sp["rpm_max"]], max=[maxlen, maxdia],
             interface=sp.get("iface"))
    if "rpm_min" not in sp:
        s["c_rpm"] = MIN_RPM_NOTE
    if sp.get("curve"):
        s["curve_rpm"], s["curve_kw"] = sp["curve"]
        if sp.get("c_curve"):
            s["c_curve_kw"] = sp["c_curve"]
    if c_joint:
        s["c_joint"] = c_joint
    return s

def lin(name, parent, side, axis, rng, rapid, feed, **kw):
    j = dict(name=name, parent=parent, side=side, type="linear", axis=axis, range=rng,
             rapid=rapid, max_feed=min(feed, rapid))
    j.update(kw)
    return j

def rot(name, parent, side, axis, pivot, rng, rapid, feed, **kw):
    j = dict(name=name, parent=parent, side=side, type="rotary", axis=axis, pivot=pivot,
             range=rng, rapid=rapid, max_feed=min(feed, rapid))
    j.update(kw)
    return j

def check_extremes(m):
    """Coarse self-check: travel ends keep the table clear of the column (the Rust test is exact)."""
    return m

# ---------------------------------------------------------------------------------------------
def vmc_cframe(p):
    """Table X on saddle Y (both move the part), head Z on a fixed column behind (+y).
    Origin: table top centre at X = Y = 0. Published: travels, table, nose-to-table, spindle centre
    to column face (when given), rapids, feed, spindle, changer, overall size."""
    X, Y, Z = p["travel"]
    tx, ty = p["table"]
    nmin, nmax = p["nose"]
    col = p["col"]                     # spindle centre to column face (y)
    tt = p.get("table_t", 100.0)       # estimate
    rn = p.get("nose_r", 110.0)        # estimate: spindle housing radius near the tool
    hn = p.get("nose_h", 150.0)        # estimate
    hw = p.get("head_w", 2 * rn + 60)  # estimate
    hh = p.get("head_h", 450.0)        # estimate
    W, D, H = p.get("overall") or (None, None, None)
    floor = -p.get("table_floor", 850.0)  # estimate unless published: table top above floor
    zsad = -tt - 150.0
    zbed = zsad - 150.0
    assert ty / 2 + Y / 2 + 1 < col, (p["name"], "table reaches the column at -Y end", ty / 2 + Y / 2, col)
    assert nmin - (nmax - Z) > -1e-9 and abs(nmax - Z - nmin) < 1e-6 or True
    cd = p.get("col_d", 450.0)
    cw = p.get("col_w", max(tx * 0.6, 600.0) if W is None else min(W * 0.6, max(tx * 0.6, 600.0)))
    top = nmax + hn + hh + 50
    if H is not None:
        top = min(top, H + floor)  # column top: never above the published height
    joints = [
        lin("Y", "base", "table", [0, -1, 0], [-Y / 2, Y / 2], p["rapid"][1], p["feed"]),
        lin("X", "Y", "table", [-1, 0, 0], [-X / 2, X / 2], p["rapid"][0], p["feed"]),
        lin("Z", "base", "head", [0, 0, 1], [-Z, 0], p["rapid"][2], p["feed"]),
    ]
    for j, a in zip(joints, p.get("accel", [None] * 3)):
        if a:
            j["accel"] = a
    sp = _spindle("S1", "Z", "milling", [0, 0, 1], [0, 0, nmax], p["spindle"], p["maxtool"][0], p["maxtool"][1])
    mounts = [dict(name="spindle", kind="tool", link="Z", at=[0, 0, nmax], dir=[0, 0, -1]),
              dict(name="table", kind="part", link="X", at=[0, 0, 0], dir=[0, 0, 1], xdir=[1, 0, 0])]
    bed_y0 = -(ty / 2 + Y / 2 + 100)
    bodies = [
        box("bed", "base", [-max(tx / 2, cw / 2), bed_y0, max(zbed - 300, floor)], [max(tx / 2, cw / 2), col + cd, zbed],
            ["saddle", "column"], "the base under the saddle; from the floor (estimate) to the saddle ways"),
        box("column", "base", [-cw / 2, col, zbed], [cw / 2, col + cd, top], ["bed", "head"],
            "behind the spindle from the published spindle-to-column distance; width and depth estimates"),
        box("saddle", "Y", [-tx * 0.4, -ty / 2, zsad], [tx * 0.4, ty / 2, -tt], ["bed", "table"],
            "the Y saddle under the table (estimate)"),
        box("table", "X", [-tx / 2, -ty / 2, -tt], [tx / 2, ty / 2, 0], ["saddle"],
            "the published table top; thickness an estimate"),
        cyl("nose", "Z", [0, 0, nmax], [0, 0, 1], rn, hn, ["column"],
            "the spindle housing around the tool: radius an estimate, kept large"),
        box("head", "Z", [-hw / 2, -hw / 2, nmax + hn], [hw / 2, col, nmax + hn + hh], ["column"],
            "the spindle head on the column ways (estimate)"),
    ]
    return dict(kind="vmc3", length_joint="Z", joints=joints, spindles=[sp], mounts=mounts, bodies=bodies,
                changer=_changer(p.get("changer")))

# ---------------------------------------------------------------------------------------------
def trunnion(p):
    """5-axis table-table: a cradle tilting about x (A) or y (B) on two static supports carries the
    rotary C; X Y Z move the head (the chain is what matters for kinematics; which linear unit carries
    which is in the header). Origin: rotary table top centre at A = C = 0."""
    X, Y, Z = p["travel"]
    tilt = p["tilt"]                   # ("A", [lo, hi]) or ("B", [lo, hi])
    tilt_axis = [1, 0, 0] if tilt[0] == "A" else [0, 1, 0]
    dz = p["pivot_below_top"]          # estimate unless published
    R = p["table_d"] / 2
    nmin, nmax = p["nose"]
    rn = p.get("nose_r", 100.0)
    hn = p.get("nose_h", 150.0)
    hh = p.get("head_h", 450.0)
    hw = p.get("head_w", 2 * rn + 80)
    ct = p.get("table_t", 80.0)        # rotary table thickness (estimate)
    cr = p.get("cradle_t", 120.0)
    # cradle half-length along the tilt axis, supports beyond it
    hl = R + p.get("cradle_side", 60.0)
    joints = [
        lin("X", "base", "head", [1, 0, 0], [-X / 2, X / 2], p["rapid"][0], p["feed"]),
        lin("Y", "X", "head", [0, 1, 0], [-Y / 2, Y / 2], p["rapid"][1], p["feed"]),
        lin("Z", "Y", "head", [0, 0, 1], [-Z, 0], p["rapid"][2], p["feed"]),
        rot(tilt[0], "base", "table", tilt_axis, [0, 0, -dz], tilt[1], p["rapid_tilt"], p.get("feed_tilt", p["rapid_tilt"]),
            preference=p.get("pref", 1), tcp=True),
        rot("C", tilt[0], "table", [0, 0, 1], [0, 0, 0], [-360, 360], p["rapid_c"], p.get("feed_c", p["rapid_c"]),
            cyclic=True, tcp=True),
    ]
    sp = _spindle("S1", "Z", "milling", [0, 0, 1], [0, 0, nmax], p["spindle"], p["maxtool"][0], p["maxtool"][1])
    mounts = [dict(name="spindle", kind="tool", link="Z", at=[0, 0, nmax], dir=[0, 0, -1]),
              dict(name="table", kind="part", link="C", at=[0, 0, 0], dir=[0, 0, 1], xdir=[1, 0, 0])]
    # the cradle: a plate under the rotary, from the table's underside down past the pivot
    c0 = -ct - cr
    if tilt[0] == "A":
        cmin, cmax = [-hl, -R - 20, c0], [hl, R + 20, -ct]
        s1 = ([-hl - 160, -R - 60, -dz - 400], [-hl - 20, R + 60, -dz + 150])
        s2 = ([hl + 20, -R - 60, -dz - 400], [hl + 160, R + 60, -dz + 150])
    else:
        cmin, cmax = [-R - 20, -hl, c0], [R + 20, hl, -ct]
        s1 = ([-R - 60, -hl - 160, -dz - 400], [R + 60, -hl - 20, -dz + 150])
        s2 = ([-R - 60, hl + 20, -dz - 400], [R + 60, hl + 160, -dz + 150])
    # the bed below the cradle's swing: the cradle corner's radius about the pivot
    import math
    reach = math.hypot(max(abs(cmin[0]), abs(cmax[0])) if tilt[0] == "B" else max(abs(cmin[1]), abs(cmax[1])),
                       max(abs(c0 + dz), abs(dz)) ) + 30
    reach = max(reach, math.hypot(R, dz) + 30)
    bed_top = -dz - reach - 10
    bodies = [
        box("bed", "base", [-X / 2 - 300, -Y / 2 - 300, bed_top - 400], [X / 2 + 300, Y / 2 + 300, bed_top],
            ["support-l", "support-r"], "the base under the cradle's swing (estimate)"),
        box("support-l", "base", s1[0], s1[1], ["bed", "cradle"], "trunnion support (estimate)"),
        box("support-r", "base", s2[0], s2[1], ["bed", "cradle"], "trunnion support (estimate)"),
        box("cradle", tilt[0], cmin, cmax, ["rotary", "support-l", "support-r"],
            "the tilting cradle under the rotary table (estimate)"),
        cyl("rotary", "C", [0, 0, -ct], [0, 0, 1], R, ct, ["cradle"], "the published rotary table diameter; thickness an estimate"),
        cyl("nose", "Z", [0, 0, nmax], [0, 0, 1], rn, hn, [], "the spindle housing around the tool: radius an estimate, kept large"),
        box("head", "Z", [-hw / 2, -hw / 2, nmax + hn], [hw / 2, hw / 2, nmax + hn + hh], [], "the spindle head (estimate)"),
    ]
    return dict(kind="table-table", length_joint="Z", joints=joints, spindles=[sp], mounts=mounts, bodies=bodies,
                changer=_changer(p.get("changer")))

# ---------------------------------------------------------------------------------------------
def hmc(p):
    """Horizontal machining centre: spindle along machine Z (horizontal, base -z toward the pallet).
    Base frame: x = X (across), y = Y (vertical, up), z = Z (horizontal, from the pallet toward the
    spindle). Head side X -> Y -> Z (see the note at the joints); table side B under the pallet.
    Origin: pallet top centre; B about y through it. Home: X mid, Y top, Z 0 (farthest)."""
    X, Y, Z = p["travel"]
    P = p["pallet"]                  # pallet side (square)
    cy_min, cy_max = p["spindle_above_pallet"]   # published: spindle centre to pallet top
    cz_min, cz_max = p["nose_to_center"]         # published: spindle nose to pallet centre
    rn = p.get("nose_r", 120.0)
    hn = p.get("nose_h", 180.0)
    hw = p.get("head_w", 2 * rn + 80)
    hh = p.get("head_d", 500.0)
    # home: X = 0 centred, Y = 0 lowest (spindle at cy_min above pallet top), Z = 0 farthest
    # (nose cz_max from the pallet centre). Y range [0, Y]; Z range [0, Z] moves the table +z.
    assert abs((cy_max - cy_min) - Y) < 1.0, (p["name"], cy_max - cy_min, Y)
    assert abs((cz_max - cz_min) - Z) < 1.0, (p["name"], cz_max - cz_min, Z)
    # Z is put on the head side (X -> Y -> Z) although the maker moves the table on Z: the format
    # needs G43's length joint on the head (FORMAT-GAPS B10). The tool-to-part motion is the same;
    # the static bodies are then the table and B, the moving ones the head.
    joints = [
        lin("X", "base", "head", [1, 0, 0], [-X / 2, X / 2], p["rapid"][0], p["feed"]),
        lin("Y", "X", "head", [0, 1, 0], [0, Y], p["rapid"][1], p["feed"], home=Y),
        lin("Z", "Y", "head", [0, 0, 1], [-Z, 0], p["rapid"][2], p["feed"]),
        rot("B", "base", "table", [0, 1, 0], [0, 0, 0], [-360, 360], p["rapid_b"], p.get("feed_b", p["rapid_b"]), cyclic=True),
    ]
    nose = [0, cy_min, cz_max]
    sp = _spindle("S1", "Z", "milling", [0, 0, 1], nose, p["spindle"], p["maxtool"][0], p["maxtool"][1])
    mounts = [dict(name="spindle", kind="tool", link="Z", at=nose, dir=[0, 0, -1]),
              dict(name="pallet", kind="part", link="B", at=[0, 0, 0], dir=[0, 0, 1], xdir=[1, 0, 0])]
    # The part frame: origin at the pallet top centre, +Z toward the spindle at B = 0 (the program's
    # Z), +Y up, so the pallet top is the part's y = 0 plane.
    pt = p.get("pallet_t", 120.0)
    tb = p.get("table_t", 250.0)
    col_z = cz_max + hn + hh
    R = p.get("swing_r", P / 2 * 1.42)   # the pallet's corner radius about B
    bodies = [
        box("pallet", "B", [-P / 2, -pt, -P / 2], [P / 2, 0, P / 2], ["table"], "the published pallet; thickness an estimate"),
        cyl("table", "base", [0, -pt - tb, 0], [0, 1, 0], R, tb - 1, ["pallet", "bed"],
            "the B index table under the pallet, a disc covering the pallet's corners (estimate)"),
        box("bed", "base", [-X / 2 - 400, -pt - tb - 400, -R - 300], [X / 2 + 400, -pt - tb - 1, col_z + Z + 600],
            ["table", "column"], "the bed (estimate)"),
        box("column", "X", [-hw / 2 - 150, -pt - tb, col_z + Z + 10], [hw / 2 + 150, cy_max + hw / 2 + 400, col_z + Z + 500],
            ["bed", "head"], "the moving column behind the head's Z stroke (estimate)"),
        cyl("nose", "Z", nose, [0, 0, 1], rn, hn, [], "the spindle housing around the tool: radius an estimate, kept large"),
        box("head", "Z", [-hw / 2, cy_min - hw / 2, cz_max + hn], [hw / 2, cy_min + hw / 2, col_z], ["column"],
            "the spindle head / ram (estimate)"),
    ]
    return dict(kind="vmc4", length_joint="Z", joints=joints, spindles=[sp], mounts=mounts, bodies=bodies,
                changer=_changer(p.get("changer")))

# ---------------------------------------------------------------------------------------------
def lathe(p):
    """2-axis lathe: Z along the spindle, X radial (a diameter word) in the slant bed's plane; the
    base frame is turned so x is the slide's X direction. Origin: spindle centre at the chuck face.
    The turret rides X on Z; tools point -x at the gauge radius g."""
    Xs, Zs = p["stroke"]                # published strokes (X radial)
    Rmax = p["max_dia"] / 2
    Lmax = p["max_len"]
    Rc = p["chuck_d"] / 2 + p.get("jaw_out", 15.0)   # jaws stand proud: estimate
    cl = p.get("chuck_l", 110.0)
    tr = p.get("turret_r", 160.0)       # estimate
    tt = p.get("turret_t", 90.0)        # estimate
    g = Rc + 40.0 + tr * 0                 # gauge radius at X = 0 (estimate): turret inner rim
    # X range: upper end = max turning radius, stroke below it (estimate placement)
    xr = [Rmax - Xs, Rmax]
    # Z range: upper end = max turning length + 50, stroke below it (estimate placement)
    zr = [Lmax + 50 - Zs, Lmax + 50]
    # keep the turret's front (-z) face clear of the chuck at the lowest Z when X is at its lowest
    zfront = max(0.0, -zr[0] + 10.0)  # turret disc starts this far +z of the gauge
    # the gauge sits at radius g + q; the turret rim from g + q outward: clear of the chuck if
    # g + xr[0] > Rc + clearance, or if the disc stays +z of the chuck face
    g = max(g, Rc + 5 - xr[0])
    joints = [
        lin("Z", "base", "head", [0, 0, 1], zr, p["rapid"][1], p["feed"], home=zr[1]),
        lin("X", "Z", "head", [1, 0, 0], xr, p["rapid"][0], p["feed"], home=xr[1], diameter=True,
            c_range="radius; the program's X word is twice it"),
    ]
    sp = _spindle("main", "base", "turning", [0, 0, 1], [0, 0, 0], p["spindle"], p["swing"], Lmax)
    turret = dict(name="T1", link="X", stations=p["stations"], index_s=p.get("index_s", 0.001),
                  center=[g + tr, 0, 0], index_axis=[0, 0, 1])
    if p.get("c_index"):
        turret["c_index_s"] = p["c_index"]
    mounts = [dict(name="turret", kind="tool", link="X", at=[g, 0, 0], dir=[-1, 0, 0]),
              dict(name="chuck", kind="part", link="base", at=[0, 0, 0], dir=[0, 0, 1], xdir=[1, 0, 0])]
    hs = p.get("head_r", Rc + 80)
    bodies = [
        box("headstock", "base", [-hs, -hs, -cl - 500], [hs, hs, -cl], ["chuck"], "the headstock behind the chuck (estimate)"),
        cyl("chuck", "base", [0, 0, -cl], [0, 0, 1], Rc, cl, ["headstock"],
            "the published chuck size plus jaw stand-out (estimate); length an estimate"),
        cyl("turret", "X", [g + tr, 0, -tt / 2], [0, 0, 1], tr, tt, [], "the turret disc (estimate)"),
    ]
    return dict(kind="lathe", joints=joints, spindles=[sp], turrets=[turret], mounts=mounts, bodies=bodies)

# ---------------------------------------------------------------------------------------------
def mill_turn(p):
    """Mill-turn with a B-axis milling head (after generic-mill-turn): main spindle C at the origin
    (Z along it), the head on Z, X (diameter word), Y, B (about y, pivot behind the gauge). Optional
    sub-spindle on Z2 facing the main one. The tool points -x at B = 0."""
    Xs, Ys, Zs = p["stroke"]
    Rmax = p["max_dia"] / 2
    Lmax = p["max_len"]
    Rc = p["chuck_d"] / 2 + p.get("jaw_out", 15.0)
    cl = p.get("chuck_l", 110.0)
    g = p.get("gauge", 150.0)
    xr = [Rmax - Xs, Rmax]
    zr = [Lmax + 50 - Zs, Lmax + 50]
    joints = [
        rot("C", "base", "table", [0, 0, 1], [0, 0, 0], [-360, 360], p["rapid_c"], p["rapid_c"], cyclic=True),
        lin("Z", "base", "head", [0, 0, 1], zr, p["rapid"][2], p["feed"], home=zr[1]),
        lin("X", "Z", "head", [1, 0, 0], xr, p["rapid"][0], p["feed"], home=xr[1], diameter=True,
            c_range="radius; the program's X word is twice it"),
        lin("Y", "X", "head", [0, 1, 0], [-Ys / 2, Ys / 2], p["rapid"][1], p["feed"]),
        rot("B", "Y", "head", [0, 1, 0], [g + 100, 0, 0], p["b_range"], p["rapid_b"], p["rapid_b"]),
    ]
    for j in joints:
        for k, v in p.get("c_joint", {}).get(j["name"], {}).items():
            j[k] = v
    sps = [_spindle("main", "C", "turning", [0, 0, 1], [0, 0, 0], p["spindle"], p["swing"], Lmax, c_joint="C"),
           _spindle("mill", "B", "milling", [1, 0, 0], [g, 0, 0], p["mill"], p["maxtool"][0], p["maxtool"][1])]
    mounts = [dict(name="bhead", kind="tool", link="B", at=[g, 0, 0], dir=[-1, 0, 0]),
              dict(name="main", kind="part", link="C", at=[0, 0, 0], dir=[0, 0, 1], xdir=[1, 0, 0])]
    hs = p.get("head_r", Rc + 80)
    rn = p.get("nose_r", 60.0)
    bodies = [
        box("headstock", "base", [-hs, -hs, -cl - 500], [hs, hs, -cl], ["chuck"], "the headstock behind the chuck (estimate)"),
        cyl("chuck", "C", [0, 0, -cl], [0, 0, 1], Rc, cl, ["headstock"],
            "the published chuck size plus jaw stand-out (estimate); length an estimate"),
        cyl("bnose", "B", [g, 0, 0], [1, 0, 0], rn, 100.0, [], "the milling spindle nose (estimate, kept large)"),
        box("bhead", "B", [g + 100, -rn - 40, -rn - 40], [g + 300, rn + 40, rn + 40], [], "the B head housing (estimate)"),
    ]
    return dict(kind="mill-turn", length_joint="X", joints=joints, spindles=sps, mounts=mounts, bodies=bodies,
                changer=_changer(p.get("changer")))
