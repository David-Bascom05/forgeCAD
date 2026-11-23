# Surfaces and thin walls: what is left (2026-11-23)

The feature as it stands on claude/repo-review-mbdrlj-surfaces: sheets from extruded and revolved
chains, open-shell checks, area, STEP OPEN_SHELL export, a sheet flag in run results, thin walls as
the thicken of a sheet, the CADFS importer's surfaces, OCCT agreement on 30 cases. The owner's
ruling ended work here; these are open, in rough order of value.

**Not started (the ruling stopped them):**
- forge-sheetops: the sheet code and thicken's rim code in a crate of their own, so forge-ops'
  sources are base. Approved, never begun. The speed gate's user-facing lines were the reason (see
  the last report for where they stand).
- `upto` a sheet's face, as CADFS uses it (112 refs): `upto(FACES)` takes a sheet's face already,
  but the importer does not yet write it.
- `split(by: SHEET)`: in the design, skipped by the master (no dataset use).
- The STEP reader for OPEN_SHELL (export writes them; import of a sheet is unread).
- Two-sided drawing of sheets in the renderer (the UI branch draws them as surfaces).
- The selector words `boundary` and `sheet`, and STL's list of the sheets it left out.

**Done here but outside the ruling's last scope** (finished before it was read; drop by reverting
if not wanted): the sheet and thin fuzz kinds in a swarm of their own (e03a8fd), sheets.py in the
Forgejo OCCT job and the L1 thin spec research/surfaces/lean/Thin.lean (258cae3).

**Proofs:**
- research/forge-lang/lean SheetArgs.lean built by the IEEE session at 3d11b00: lake build, axioms
  (propext, Classical.choice, Quot.sound only), FG_LEAN parse_diff 4/4, digest unchanged.
- research/surfaces/lean/Sheet.lean (ValidSheet, check_sound_sheet and two more, stated) and
  Thin.lean (L1, stated) are not built: no Lean toolchain here, and installing one was refused. Sheet.lean is
  written against TopoCheck.Model at 3d17b13 and must take 25cb295's passes (fshell, checkLinks,
  checkShellFaces, checkStrays). For fundamentals, who said they would prove it after their L4.

**Kernel limits found, refused by a type, never a wrong part:**
- A thin wall or thicken where a spline meets its neighbour in a cusp (a path's `spline(..)` whose
  `tangent: true` came out anti-parallel to the line before it): `Rebuild(Vertex)` there. A cusp
  has no mitre; a clearer error kind (a cusp at the vertex) would help.
- Glyph outlines thickened out where an edge's offset shrinks to nothing (a "B" at size 10, t 0.1):
  `Rebuild(Collapsed)`. Taking the edge out (the mitre's topology changing) is not done.
- The fuzz swarm meets four kernel failures with no sheet in them (fuzz_known_swarm.txt): booleans'
  `SplitCrossed` and `InconsistentClassification`, a fillet left `DuplicateName`, a long cylinder
  over the mesh's point budget. Each reproduces on main.
