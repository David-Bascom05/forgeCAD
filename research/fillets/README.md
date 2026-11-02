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

## fillet-2 (rebased on 272014c9 and after), judged by the real judge against the run of main 33d794df

`census.py` / `tally_census.py`: every part holding a fillet. A `stopped` part always counts as failed.
WHUCAD is quantised, so its small errors can be the dataset's.

```
           parts   match        wrong       failed
WHUCAD      1783   1044 > 1045   245 > 246   494 > 492
FreeCAD      327    206 > 207      4 > 3     117 > 117
CADFS        211    103 > 103     10 > 10     51 > 49
```

Three WHUCAD match > failed (100003203, 100004130, 100004231) fail the same on main: an extrude's boolean,
since 33d794df. Six WHUCAD failed > wrong were invalid or far off on main and now build valid, nearer truth.
Refused on purpose: 100001816, whose STEP truth has its round inside the cavity; 100000470 and FreeCAD
4a66f49a, whose ball crosses several short faces, which no rule does yet.

Rounds meeting at a ridge where no ball rests on both outer faces: exact on CADFS 00476278 and 00438053
(OCCT's STEP), WHUCAD 100003117 at 2.7e-6; no part shows the ridge itself wrong. Kept.

## Rolling on a strip no edge runs along (night 2026-11-02)

`closing.rs` is an independent reference for `fillet_strips.rs`: the added volume of a ball's closing of a block with a
rounded rim and a boss standing over it, on a grid (`rustc -O closing.rs; closing 3 40 12 5 0.08`, args rim radius, boss
y, boss radius, ball radius, step). It gave 1100.8 mm3 at step 0.08 against 1101.83 from the kernel's exact mass; the
result moves with how the radius falls on the grid at coarser steps. No kernel code is shared.

`closing_rib.rs` is the same for a rib on a cone (`closing_rib 2 0.08`, args ball radius, step). The closing's noise at these
steps is about 2%, from how the radius falls on the grid, and it stands about 1.5% high: over 8 steps it gave 1118.7 against
1101.83 exact on the first scene, and over 7 steps 840.1 against 829.65 on the narrower rim. These are what the numbers in
`fillet_strips.rs` were held against.

## Balls resting on a support carried on past its end (2026-11-02)

`blends::audit` counts these as `carried`, not `unsupported`. Why that is sound, measured on 19 fillets in 17 parts with 1 to 18 such points each (largest distance off the support as it stands 0.12 at r 2):
- OCCT's own fillet of the same body (`occt_fillet.py`) builds on 10 of the 17, 8 of them valid. On those 8, away from corners our points lie on its surface to within 0.01, which is the fit of the STEP file. Within 1.5 r of a corner they differ by up to 0.27 at r 2, and added volume by 0.1 to 0.4%, on lofted walls: past a wall's end, what the surface is carried on as is a choice, and the two kernels choose differently. Which is better there is not settled by this.
- A control with plane walls and skew corners agrees with OCCT everywhere, added volume to 5e-8. It also lies up to 0.335 outside the ball's closing at r 2 (`closure_body.rs`, functions to paste into a bridge): that is what a mitre is. So being outside the closing by that much is not itself a fault.
- OCCT refuses the other 7. They are judged by the closure only: nothing out by more than the control's mitre on the 17 parts' flagged points (0.206 at r 2).
The bound of four radii on a line's or circle's carry was set to admit one case, a 0.2 long flat end with r 1, off by 3.5e-4; OCCT's surface agrees there on the sibling case it builds. It is a choice, not a derived limit: `worst_carried` reports what passed.
