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
