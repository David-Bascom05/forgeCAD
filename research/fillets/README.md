# Fillets: what fails and why

Started 2026-10-31. Measured on real parts: CADFS Onshape 1-in-200 pilot; WHUCAD and FreeCAD to come.

## How to rerun

- `verify/forge-proof/examples/filletcheck.rs FILE.fc`: a part's fate as one JSON line, every fillet
  audited against the part before it (`forge_proof::blends`).
- `research/fillets/run.py EXE PARTS TRUTH SPLIT OUT`: every fillet part of a directory, against the STEP.
- `research/fillets/report.py RESULTS TRUTH SPLIT`: a translated run's rate and refusals.
- Match means volume and area within 1e-4 of the source's STEP. The pilot's 1e-3 hid a wrong corner
  (00307228, 0.09% off).

`blends::points` (points the fillet changed must lie beyond a ball surface) is a diagnostic on real
parts, not in the `blend` property yet: it still flags parts that match their source.

## Where it stands

Batch fillet-1, on main with volume and area within 1e-3 of the source (the OCCT replay's rule, without
its box test). Part files as the dataset agent translated them, so a part it stopped at a refused fillet
holds only the features up to there: it cannot match until it is translated again, and counts as off.

| dataset | parts with a fillet | match | off | refused | invalid |
|---|---|---|---|---|---|
| CADFS pilot, the OCCT replay's set | 237 | 129 (OCCT 111, main 93) | 47 | 60 | 1 (an extrude's, before the fillet) |
| CADFS 1-in-20, last feature a fillet, all kernel gaps on main | 149 | 91 | 4 | 54 | 0 |
| WHUCAD, the 1000-part replay's fillet parts | 456 | 270 | 76 | 103 | 7 (all failed on main too) |
| FreeCAD library, bodies with a part file | 243 | 171 | 26 | 44 | 2 |

WHUCAD's values are quantised to 256 levels, so its refusals may overstate real parts. No per-feature
FreeCAD truth yet: tip shape only.

## Ranked, by root cause, from the pilot's 52 refusals

| class | parts then | root cause | state |
|---|---|---|---|
| two blends and a sharp edge | ~6 + 5 wrong | a corner patch where the blends should cut each other | fixed: `mitre`; closed form, OCCT and the STEP agree, unequal radii and a re-entrant third edge too |
| radius = the arc's or rim's radius | 9 | offset collapses: a torus of no major radius | fixed: `Pivot`, a sphere |
| radius = face width or half | 8 + 5 | spring on the far edge, or two springs on one line | fixed: `merged`, `collapse` |
| past half the width | ~4 | blends overlap with no vertex between | fixed: `overlapping_rounds`, a boolean of each group's round |
| ball wider than its face | ~6 | no rule | fixed where the far edge spans the spring: `cliff`; else over a sharp edge: `switch_support` |
| both kinds meet at a vertex | ~4 + 00307228 | patched | fixed: `minority_first` |
| crowding: springs crossing or touching a neighbour's | most of what is left | a boolean of the rounds needs torus against torus; a touch needs the face split there | open |
| mitre not followed | 2 of 237, `00057004`, `00309568` | refused, not patched | open |

## Prior art, briefly

Parasolid's public overflow table: smooth, then cliff, then notch; internal sharp opposite convexity
gives cliff, then notch as the radius grows. Mitres: Parasolid and ACIS. 00508513 (a rim over a step) is cliff
to ten digits. Sources: Parasolid docs through third-party mirrors, ACIS R10, facts only.
