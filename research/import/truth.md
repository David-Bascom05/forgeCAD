# The truth rule: how a source shape is measured

A translated part is judged by its volume and area against its source's: FreeCAD's stored shape, or the STEP a CADFS
or WHUCAD part was exported as. Those are measured with OCCT. This note says how, and why. The rule was adopted by
the owner on 2026-11-23. It is in tools/find-parts/recreate/truth_rule.py, which both truth tools (fcstd_truth.py and
step_truth.py) and research/import/tally.py use.

## The rule

For each solid, three measures are taken and summed over the solids:

| measure | call |
|---|---|
| default | `BRepGProp::VolumeProperties` and `SurfaceProperties`, no precision given |
| precise | the same, with a precision of 1e-9 |
| Gauss–Kronrod | `BRepGProp::VolumePropertiesGK`, 1e-9, spans on (a volume only) |

Then:

- the **volume** is the Gauss–Kronrod one;
- the **area** is the default's or the precise call's, whichever goes with a volume that agrees with the
  Gauss–Kronrod one to a tenth of the tolerance the part is judged by (so 1e-5 for the 1e-4 sets);
- the truth is **uncertain**, and the part is counted neither as a match nor as a mismatch, where neither volume
  agrees, or where both agree and their areas differ by more than that.

A truth measured before the rule (no Gauss–Kronrod volume) is read as it was measured.

## Why

Until this rule the truth tools took the default measure alone. It is not exact on every face.

**The check.** `truth_check.py` makes a part whose filleted volume has a closed form, rounds it with OCCT, and measures
it each way. A 100 by 60 by 10 block has an elliptical hole (semi-axes 20 and 10) through it, and both of the hole's
edges are rounded at r = 2. The rolling ball between a plane and a wall square to it takes away a section of
(1 − π/4)r², whose centroid lies x̄ = r(10 − 3π)/(3(4 − π)) from the edge. That section's plane is square to the edge
all the way round, and the material lies outside the hole. So by Pappus each round takes (1 − π/4)r²(L + 2πx̄),
L being the ellipse's perimeter, since the curvature turns once round it. The part's volume is then exactly
53545.663021384.

| measure | volume | off |
|---|---|---|
| Forge's kernel (the same part as .fg) | 53545.663021377 | 1.3e-13 |
| OCCT's fillet, default | 53545.683153952 | 3.8e-7 |
| OCCT's fillet, precise | 53545.663563069 | 1.0e-8 |
| OCCT's fillet, Gauss–Kronrod | 53545.662984143 | 7.0e-10 |

**Neither of the old two is right everywhere.**

- On FreeCAD 82d7764f.PolarPattern004, the default is wrong. The stored shape matches ours feature by feature until a
  radius-2 fillet along elliptical edges. From there the default comes out 2.0 below the precise measure
  (27743.68 against 27745.69), and ours is the precise one to 1e-6. Gauss–Kronrod: 26028.8318 at the tip, ours 5e-8.
- On FreeCAD 5ab390cb.Body001, the precise call is wrong. That body is an ellipse 20 by 15 padded 0.4, exactly
  π·20·15·0.4 = 376.991118. The default gives that, while the precise call gives 376.990968 and moves the area by
  6e-5. Gauss–Kronrod: 376.991118.

Gauss–Kronrod agreed with the closed form in every case checked, which is why it gives the volume. There is no
Gauss–Kronrod area, so the area is taken from whichever of the other two is right about the volume. Where that can't
be told, the truth is too unsure to judge by.

**How much it moved.** Before the rule, the two old measures of FreeCAD's stored shapes disagreed by over 1e-5 on 68
of the 79 bodies then counted as mismatches. coverage.md has the counts under the rule.

## Copies

A source file can hold exact copies of a solid. Some files hold a part N times that the program made once, and
step_truth used to count every exact copy once. But a program can make the copies itself: a pattern of new bodies
that spins each copy onto the last makes them coincide. Counting those once mis-judges a right translation. So the
rule, adopted by the owner, keeps the multiplicity the program produces, read from the program and never from our
result:

- step_truth counts each distinct solid once, as before, and keeps the copies' measures aside;
- the CADFS reader records the bodies the program makes as it reads it (`program_bodies`: one per new body, one per
  instance of a pattern of new bodies, less the bodies an add or a boolean joins). Which bodies an add meets the
  reader works out on the part so far, so this one step leans on the kernel. The count is never taken from the
  built part;
- where the source's solids, copies and all, are as many as the program's bodies, the copies are counted;
- where its distinct solids are, they are not;
- where neither, the truth is uncertain.

One limit: where the program's bodies match neither count, the truth is uncertain. One way that can happen is an
Onshape body of two disjoint lumps, which the STEP writes as two solids. That may be 00647071's case (below); it is
not checked. FreeCAD's truths
are not affected: fcstd_truth measures the stored shape as it is and drops no copies.

**What it moved (CADFS).** Of the 49 parts whose STEP holds exact copies:

- 3 mismatches become matches (00293826, 00724634, 00846949: patterns of new bodies spun onto each other);
- 1 match becomes uncertain (00647071: the program makes 1 body, and the STEP holds 2 distinct solids and 6
  copies);
- 45 stay as they were, among them 00879304. Its STEP holds 199 solids, the pattern's 100 and 99 copies of them,
  and its program makes 100 bodies, so the copies are counted once, as before.

## Re-measuring

FreeCAD's truths are taken as each body is translated (`run_freecad.py run`), and CADFS's and WHUCAD's by
`step_truth.py`. Either run, made with these tools, carries the three measures, and `tally.py` applies the rule.
