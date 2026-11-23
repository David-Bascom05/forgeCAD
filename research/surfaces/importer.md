# Importer: surfaceEntities on solid features (2026-11-23)

Onshape's extrude and revolve read `surfaceEntities` only when `bodyType` is SURFACE. A solid
feature can still carry the list: the UI leaves the sketch's edges in it. The importer used to call
any feature with that list "a surface extrude" and stop. It now reads the list only for a SURFACE
body. This needs no kernel change.

The sample is CADFS at dd76a9b, `sorted(unique_models.json)[::200]` (1914 parts), the same as
research/import/coverage.md. "Before" is cadfs.py at the import branch's head (c5dea21) with the
base binaries; "after" is this branch. Only the 97 parts whose program holds `surfaceEntities` were
rerun, since nothing else reaches the changed lines. STEP truth comes from step_truth.py on the
1211 parts that built in either run. The tables come from `research/import/tally.py`. The before
run reproduces coverage.md's after column: 1106 matches, 627 untranslated.

| set | untranslated before | after | match (1e-4) before | after |
|---|---:|---:|---:|---:|
| CADFS, all parts | 627 | 598 | 1106 | 1123 |
| CADFS, parts with a fillet | 234 | 222 | 164 | 169 |

**No regressions.** Each of the 29 class changes is from untranslated. 48 parts left "a surface
extrude": 29 now translate and 19 meet another cause later in the program.

| after | parts |
|---|---:|
| match | 17 (vol and area to 8.2e-05, most to 1e-12) |
| mismatch | 6 (00262773, 00351396, 00581345, 00730748, 00842278, 00875968) |
| refused | 5 (Boolean ZeroThickness, Shell Rebuild, Fillet Crossed, Shell SelfIntersecting, Fillet NoBlend) |
| fail | 1 (00983911: Boolean InconsistentClassification, at a revolve) |

The 17 real surface bodies (`a thin or surface extrude` 17, `a thin or surface revolve` 5) stay
untranslated until the importer emits sheets.

## Sheets (SURFACE extrudes and revolves)

The importer now writes a SURFACE extrude or revolve as a sheet:
`extrude([edges..], d, surface: true)` or `revolve(.., surface: true)`, one per chain of its edges
(cadfs.py `sheet`, fgw.py `sheet`). The judge measures each body on its own. A sheet has no volume
or area of its own, so a part's volume and area are its solids'. That is also what step_truth
measures.

| set | untranslated (base) | after surfaceEntities | after sheets | match (base) | after surfaceEntities | after sheets |
|---|---:|---:|---:|---:|---:|---:|
| CADFS, all parts | 627 | 598 | 583 | 1106 | 1123 | 1128 |
| CADFS, parts with a fillet | 234 | 222 | 221 | 164 | 169 | 169 |

Of the 26 sample parts that make a real sheet:
- 15 now translate.
- 5 match their solids to 1e-4: 00226295, 00268238, 00271994, 00418772 and 00544311, at 2e-13 to
  1e-12.
- 1 is a mismatch: 00940483, volume 1.5e-5, area 1.2e-4. Its solid is off while its sheets are
  not the cause.
- 1 is refused: 00633983, a fillet `Mitre` on its solid.
- 8 are sheets alone, with no solid for the truth to measure. For these I measured the STEP's
  surface area with OpenCascade and compared it with our `sheet_area`. All 8 agree to 5e-12 or
  better:

| part | ours | STEP (OCCT) | relative |
|---|---:|---:|---:|
| 00179175 | 81896.20192143036 | 81896.20192142087 | 1.2e-13 |
| 00255212 | 1064.1376227605324 | 1064.1376227556436 | 4.6e-12 |
| 00312026 | 848230.0164692441 | 848230.0164692443 | 1.4e-16 |
| 00352666 | 1159.3513887153133 | 1159.3513887153135 | 2.0e-16 |
| 00422939 | 11124.394457268598 | 11124.394457269636 | 9.3e-14 |
| 00696668 | 1882.6230567692676 | 1882.6230567699515 | 3.6e-13 |
| 00859196 | 460.7307185447487 | 460.730718543842 | 2.0e-12 |
| 00982822 | 6017.153366107386 | 6017.153366105666 | 2.9e-13 |

The 11 still untranslated stop on:
- a surface along a spline (3);
- a region or entity the query does not settle (4);
- a surface sweep (1);
- a revolve about a solid's edge (1);
- a surface of a part's edges (1);
- an axis that is not a line (1).

Every class change from the base is from untranslated. No part that built before stopped building,
and no match was lost.
