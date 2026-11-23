"""The truth rule: how a source shape's volume and area are measured, and when the measure is too unsure to judge by
(research/import/truth.md has the reasons).

OCCT's `BRepGProp::VolumeProperties` with no precision given, the measure the truth tools used to take alone, can be
off by more than 1e-4 on blend and spline faces; with a precision of 1e-9 it is right there but can be wrong where
the default is exact. Its Gauss-Kronrod integrator (`VolumePropertiesGK`, 1e-9, spans on) agreed with the closed form
in every case checked. So:

- the volume is the Gauss-Kronrod one;
- the area is the default's or the precise call's, whichever goes with a volume that agrees with Gauss-Kronrod (to a
  tenth of the tolerance a part is judged by);
- a truth is uncertain, counted neither way, where neither volume agrees, or both do and their areas do not.

`measures(solid)` takes the five numbers in the OCCT interpreter; `judged(t, tol)` applies the rule to a truth's
sums, in any interpreter.
"""

EPS = 1e-9


def measures(solid):
    """(volume, area) by default, (volume, area) with a precision of EPS, and the Gauss-Kronrod volume."""
    from OCP.BRepGProp import BRepGProp
    from OCP.GProp import GProp_GProps

    def one(f, *a):
        g = GProp_GProps()
        f(solid, g, *a)
        return g.Mass()

    return {
        "vol": abs(one(BRepGProp.VolumeProperties_s)),
        "area": one(BRepGProp.SurfaceProperties_s),
        "vol_eps": abs(one(BRepGProp.VolumeProperties_s, EPS)),
        "area_eps": one(BRepGProp.SurfaceProperties_s, EPS),
        "vol_gk": abs(one(BRepGProp.VolumePropertiesGK_s, EPS, False, True)),
    }


def with_copies(t, program_bodies):
    """The truth with as many of its exact copies as the program makes (truth.md). step_truth counts each distinct
    solid once and keeps its copies' measures aside. Where the source's solids, copies and all, are as many as the
    bodies the program makes, the copies are added back; where its distinct solids are, they are left out; where
    neither, the truth is None (uncertain). A truth with no copies, or a part whose bodies were not counted, is as
    measured."""
    c = t.get("copies", 0)
    if not c or program_bodies is None or "copy_measures" not in t:
        return t
    n = t.get("solids", 0)
    if program_bodies == n + c:
        m = t["copy_measures"]
        return dict(t, **{k: t[k] + m[k] for k in m if k in t})
    if program_bodies == n:
        return t
    return None


def judged(t, tol, program_bodies=None):
    """(volume, area) a part is judged against, or None where the truth is uncertain. A truth measured before the rule
    (no Gauss-Kronrod volume) is taken as it was measured."""
    t = with_copies(t, program_bodies)
    if t is None:
        return None
    if "vol_gk" not in t:
        return t["vol"], t["area"]
    v, near = t["vol_gk"], tol / 10
    agree = lambda x: abs(x - v) <= near * max(v, 1e-12)
    cand = [a for a, w in ((t["area_eps"], t["vol_eps"]), (t["area"], t["vol"])) if agree(w)]
    if not cand:
        return None
    if len(cand) == 2 and abs(cand[0] - cand[1]) > near * max(abs(cand[0]), 1e-12):
        return None
    return v, cand[0]
