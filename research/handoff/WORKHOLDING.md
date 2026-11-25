# WORKHOLDING: handoff

Session: https://claude.ai/code/session_01WJtawz4GamJmXtW7gNNCqN (wound down 2026-11-25).
What I owned: workholding as a first-class part of CAM — the library (`kernel/forge-workholding`),
its builder (`interface/forge-workholding-builder`), and how CAM holds a part.

## Branches

| branch | sha | state |
|---|---|---|
| `cam/workholding` | 9edad4f6 | merged in train 7 (the library, kinds, lathe, generators, vendor data) |
| `cam/workholding-vise` | d9040ec4 | PRE-VERIFY GREEN-EXCEPT-RENDER, sent to TRAIN-CONDUCTOR (render: VERIFY-RENDER) |
| `cam/workholding-haas` | 783e2d57 | contains `-vise`; real Haas models (05-0404, 05-0405) in the certified path |
| `cam/workholding-any2` | 11c7a3b2 | contains `-haas`; any geometry, user imports, the speed fix |
| `cam/workholding-default2` | (this) | contains `-any2`; `generic-vise-150`/`-100` mean the real Haas vises |

They land in that order; each line goes to TRAIN-CONDUCTOR as its pre-verify comes in.
`cam/workholding-any` and `cam/workholding-default` are superseded (by `-any2` and `-default2`):
`-any` sat on the old default switch, so it could not land without it.

## What `cam/workholding-vise` does

`vise(NAME)` in a machining document resolves in the workholding library (the built-in set, with
the project's `cam/workholding/*.toml` on top), is placed by `forge_workholding::mill::vise`, and
becomes the checker's fixture through `forge_cam::fixture::Fixture::from_workholding`. `Fixture`
gained `table_z` (the table plane in the part frame) and `placed` (the library's placement: each
body's role, part and pose). `SetupView.fixture` exposes it read-only.

`generic-vise-150` resolves to `standin-vise-150` with a warning: that is the fix for David's
floating VF-2 vise, whose old forge-tooling record had no body below the bed and so no table
plane. A name only forge-tooling knows keeps the old boxes, with a warning saying what is missing.

bracket_cam.fg and manifold_cam.fg certify with byte-identical NC and unchanged verdicts; the band
is +0.0000% and the bits identical.

## What `cam/workholding-haas` does

A maker's model of a device becomes the certified checker's faces, so no box stands in the safety
path once a real model exists (David's question, through the coordinator).

- `forge_cam::enclose` gives a face `lower` cannot take exactly its own surface over a parameter
  box proven to enclose it (from the face's box: a plane's and a ruled face's boundary hull; the
  kernel's own extent for spheres and tori). A superset of the face, so a proven clearance holds.
- Procedural and NURBS faces become forge-mesh's triangles, each certified within the chord, as
  planar faces whose mu is the chord. A face the mesh cannot settle is refused: nothing falls
  back to a box, and `Fixture.enveloped` names any body that did (empty on both models).
- `forge_cam::fixture_model` is the text format (every number f64's shortest round-trip form, so
  reading returns the very faces written) and `moved` poses a face by a turn that permutes the
  axes (exact in f64) and a shift, whose rounding is added to mu.
- `forge-workholding-builder::compile` does this once per model; `wh_compile` writes
  `data/check/NAME.txt` and the per-body STLs. CAM reads the text (≈1 ms), never a STEP (0.3 s,
  as long as a whole document's CAM).
- `forge_workholding::model` is the built-in models; `Body.exact` names a body's model group and
  pose; `mill::vise` builds a record's bodies from its model and takes the table plane from the
  model's own underside. A body whose motion the model does not state (a vise's screw and nut) is
  placed both ways, fixed and with the moving jaw, and both are checked.

Models: `haas-05-0404` (6 in CrossOver, = 05-0213) and `haas-05-0405` (4 in, = 05-0209). Their
records are `haas-vise-05-0404` and `haas-vise-05-0405`, opt-in by name today.

Files: the STEP and the per-body STLs are in Git LFS (`.gitattributes`, `.lfsconfig`
fetchexclude, an AGENTS.md exception quoting David). The compiled faces are plain text committed
normally (≈1 MB each, 172 kB packed), so a clone without LFS still checks against the real
device. `kernel/forge-workholding/tests/models.rs` re-derives the text from the STEP byte for
byte when the objects are there, and otherwise checks the LFS pointer's own oid against the
sha256 the record cites, so a model swapped under a citation is caught either way.

## What `cam/workholding-any` does (David: "do not over fit to one Haas vise")

- No code names a model: forge-workholding's build.rs embeds every `data/check/*.txt` by file
  name, and a generic name's successor is the record's `replaces` word. A test
  (forge-cam-lang tests/workholding_any.rs `no_code_path_names_a_particular_model`) fails if
  the sources that place, compile or check workholding name one.
- `mill::model_bodies(model, pose, travel)` places any model; `mill::model_fixture` places a rigid
  device by its model.
- `fixture: workholding("NAME", at: (X, Y, Z), turn: DEG, clearance: MM)`: a record whose bodies are
  a maker's model (`model`), the user's own file (`model_file`: STEP, IGES, .brep or .fg beside the
  document), or a custom record's boxes. A user's file is compiled while running, once per
  sha256, cached in `cam/workholding/.compiled/SHA.txt` beside the document.
- Soft jaws on a model vise: the vise under them is still its sizes' boxes; each is named in
  `Fixture.enveloped` and the document warned.

## Unfinished, in the order I would do it

0. **Open shells (sheets)**: KERNEL-STRESS's `import-sheets` (@05d50e2d, on kernel-stress, not
   yet landed) adds `forge_step::Imported::sheets` (`Sheet { entity, name, body, open_edges }`) for
   the 51 open shells of 05-0223, 05-0224, 05-0994, 05-0995, 08-1717, 08-1865 and 08-1866. Once it
   lands, `compile::compile` should take `im.sheets` too (as further indices after the solids, or a
   group of their own), so those models check against their real faces. A sheet has no inside:
   the checker must use its faces plus mu only, never an inside test, and anything that needs a
   volume must keep refusing it. Two sliver faces (05-0994 #109088, 05-0995 #43971) mesh to nothing
   and will hit compile's "not settled by the mesh" refusal: decide them there (a face that small
   can be enclosed by its own box, reported). Add a test with one of those models, and put the
   branch after import-sheets in the train order.
   **Parasolid**: PARASOLID's .x_t/.x_b importer is heading for a train; once it lands,
   `model_file` should accept it with a one-line dispatch in `Cam::import_model` (beside the
   `.fg` branch: read it to bodies, then `compile_bodies`).
   **Soft jaws on a model vise's own bodies.** `forge_cam::fixture::soft_jaws` builds the vise
   under the jaws from `ViseSize` boxes; with a model it should place `mill::model_bodies` there
   instead (the hard jaws' faces against the soft jaws' backs). Until then each box is reported.
   Also: a non-Haas maker's model (none openly fetchable without sign-up was found), Parasolid
   (forge-step does not read it), and writing an STL beside a runtime import's compiled faces
   if CAM-SIM wants one to draw.

1. **Make a real vise the default.** Done on `cam/workholding-default2`, once the speed fix on
   `-any2` made it cost +2.6% / +2.7% instructions on bracket / manifold (it was +18% / +43%:
   the memo key printed the whole fixture each op, the faces were cloned each op, the text parsed
   each setup, and the fixture grid returned the vise's body under every move; `Grid::near_z`
   skips what `apart` drops anyway, in the same order). NC and verdicts identical to main. The coordinator approved repointing `generic-vise-150` to
   `haas-vise-05-0404`, with the full before/after (NC, verdicts, cam_time, band) and every
   verdict change explained — a new contact against the real vise is a real finding, not a
   regression. I did not get to it. It is a one-line change in `VISE_SUCCESSOR`
   (interface/forge-cam-lang/src/lib.rs) plus the warning's wording, and then the measurements.
   Expect more faces checked (3522 against 5 boxes) and new findings where a path runs below the
   jaw top outside the stock, since the real vise has a body there and the stand-in did not.
2. **`plate(NAME, ..)` and `zero_point(NAME, ..)` in forge-cam-lang** (CAM-CORE asked; it is
   their crate). The library side is done and kind-agnostic: `mill::fixture_plate` and
   `mill::zero_point` return a `Placed`, and `from_workholding` takes any `Placed`, model or not.
   What is missing is one builtin accepted by `fixture:` beside `vise(..)`: parse the name,
   resolve it as `Cam::vise_record` does, call the `mill` function, then `from_workholding`.
   Needed for the mould-core examples, which want a real Haas subplate rather than a made-up one.
3. **The leftover reason "held by the fixture"** (interface/forge-cam-lang/src/lib.rs ~2848)
   tests the fixture's *boxes*. For a model body that box is an enclosure of its faces and
   reaches past the body (6.5 mm under the Haas vise's base), so it excuses more leftover than
   the body would. It wants `f.faces()`. This is the only place left in CAM where a box is in a
   check rather than a display. CAM-CORE's.
4. **The machine simulation's collision bodies** are still `f.boxes()` (CAM-SIM's; in their
   handoff too). `Body.exact` plus `fixture_model::read`, posed by `Exact.pose`, is the same
   data G2 sees.
5. **More models.** 05-0406/05-0212 (8 in CrossOver) is next in the family but its two jaw plates
   share one part number (D80-7), so which is fixed cannot be read off the model: measure or ask
   before grouping it. 05-0182/05-0211 (double-station CarvLock) needs KERNEL-STRESS's
   BREP_WITH_VOIDS reads first. A self-centring vise (05-0004, 05-0013) needs `model_bodies` to
   move both jaws symmetrically; today `Moves::WithJaw` moves one group by one travel.
   The whole catalogue is in research/cam/workholding-haas.md: 299 files, 285 distinct models,
   each with its name, category, size and import status.

## Risks and things to know

- **The enclosure is loose in places.** A face the checker cannot take exactly is enclosed, and
  for a torus over its whole tube or a cone over a full turn the enclosure can reach a few mm
  past the face (worst on the Haas vise: 5.8 mm). That direction is safe (it can only fail to
  prove a clearance, never claim a false hit) but it can refuse a path that would really pass.
  If that bites, the fix is to enclose over the face's own angular span instead of the box's,
  which needs the face's uv loops rather than its box.
- **`Fixture::boxes()` for a model body is the faces' enclosure, not the body.** Draw from
  `placed` (prims and parts, tight to the chord), check from the faces. Mixing them looks like a
  bug and is not one.
- **A model's spec is read twice**: `model::builtin()` wants the compiled text to exist and to
  match, and `model::specs()` reads models.toml alone (what the compiler needs). A new model
  needs its `include_str!` arm in `model::check_text` and a placeholder
  `data/check/NAME.txt` (one header line) before the first compile, or forge-workholding will
  not build.
- **Mesh faces carry the deviation as mu, not as an outward offset.** The coordinator asked for
  them to match CAM-MACHINE's `CollisionMesh` (one-sided, inflated by the proven deviation). I
  asked CAM-MACHINE which they want and wound down without an answer, so the geometry is
  unchanged: mu widens the band both ways, which cannot produce a false hit and only withholds
  an unproven clearance. If CAM-MACHINE wants one type for both, that is the open choice; the
  numbers are in research/cam/workholding.md §5.
- **The vendor data rule held**: every value is cited with its source and the date, sizes no
  maker publishes are measured on the maker's own model (with the file's sha256) or left
  `unknown`, and a value two sources disagree on stays in `conflict` until something settles it.
  haas-vise-05-0404's height was the one I settled, and the model also corrected my own first
  reading of it (117.20284 → 117.1): a face box is padded for spheres and tori, so measure a
  body on its mesh (`wh_measure`), never on its face boxes.
