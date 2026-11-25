# BENDSIM-TOOLS hand-off (2026-11-25)

The bend simulator's tooling side: the press brake's tools, the certified checks, the planner.
Wound down at the coordinator's request; this is what is where and what is left.

## Branches

| Branch | Head | State |
|---|---|---|
| `bendsim/tools` | `d4e24e06` | Ready to merge (into the bendsim train through BENDSIM-ENV's head). Holds everything below. |
| `bendsim/hems` | `d4e24e06` | The same commits (hems and offsets were built here, then fast-forwarded into `bendsim/tools`). Can be deleted. |
| `sheet/near-limit` | `dd48635e` | forge-sheet case 64 (`hunt_by`'s Lipschitz leaf, 322 s to under 1 ms, verdicts identical over 7551 builds, EXACT-CORE approved), with main `c135b274` merged in. Pre-verify re-run at wind-down; its line goes to TRAIN-CONDUCTOR. |

## What `bendsim/tools` adds (since main)

- Operator handling certified (rank 3b), 12-15 ms a reply; the tessellation and context caches.
- `Scene::contact_bodies`: a step's collision regions posed at an instant, drawn as the cells'
  own footprint (BENDSIM-ENV's contact tint).
- `examples/sheetmetal/narrow_channel.fg`: a part that collides (its header names the words).
- The slow tests split or run side by side (each under the 4 s budget alone); the reply's
  contacts extracted on all cores.
- **Hems** (David: "step and hem in sim"): a hem is two steps on its bend, `Stroke::PreBend`
  (an air bend to 30 degrees included, WILA) then `Stroke::Flatten` (the base level on a flat die,
  a flat face down on the part's top, certified checks through the stroke). WILA's flattening pair
  BIU-041 over OZU-384 in the catalogue, cited. Rules `no_flattening`, `tool_rating`.
- **Offsets (Z)**: `Stroke::Offset { k }` makes both bends of a Z in one stroke, the outer walls
  level, both turns enclosed; rule `offset` (the part's offset against the inserts' Z, a
  hundredth); the planner prefers a mounted pair that passes. WILA HU-001 over HU-005 / HU-006
  with INZU-002 Z3-Z8, cited.
- Tools carry a `role` (bend, flatten, hem, offset z) and `max_thickness` (tooling files and the
  catalogue); `Step::mark` marks every bend a step makes (an offset makes two).
- `examples/sheetmetal/hem_joggle.fg`: the showcase, a hem and a joggle on three stations.
- `prepare()` keeps a tool shared by stations apart once (was refused as a duplicate).

Design and data: `research/bendsim/tools/hems.md`, `research/bendsim/tools/hems-data.md`.

## Not done, and what was found

- **Teardrop and rolled hems**: refused at the flattening (`set_flattening`, theta past half a
  turn). A flat face coming down meets only the fold's apex once the leg drops below it; no
  maker publishes the tool that closes a teardrop (WILA's tonnage table has a "partly closed"
  stage with heights D, the only trace). Next: a cited teardrop tool, then a stroke that stops
  at the teardrop's height with the face on the apex.
- **Combination spring-action dies** (WILA DDU-001/1, -002/1, -004/1; Mate 20.264, .303, .450,
  .231, .305): `Role::Hem` pre-bends but does not flatten (`Role::flattens` is false for it).
  Published: V and angle, open / closed heights, the 13-15 mm flange gap, widths, loads. Not
  published: how the height divides between the sprung part and the holder under it; the
  drawings read (not quoted) as the punch pushing the sprung part down onto the hem in the gap.
  Next: that split from a drawing with a dimension, then the flattening's punch side is the
  sprung part (a block riding the ram) and the die the holder.
- **Flattening force**: "not checked". WILA's flattening tonnage chart (catalogue p.111) and
  Mate's hemming force table (p.47) are transcribed in hems-data.md, but their column order was
  read from header icons; confirm against the page images, then a tonnage rule per metre.
- **Offsets with Mate's per-Z ratings**: Mate's Z inserts (40.510-40.615, T* per Z) are in
  hems-data.md, but Mate publishes no holder, so no working height: not in the catalogue. WILA's
  INZU-002 carries no rating per Z (UNKNOWN), so its records have no `max_thickness`. Next: a
  holder for Mate's inserts, or WILA's ratings.
- **Offsets, not yet checked**: the part on itself through the two bends (said so in the reply),
  the force (no maker publishes it), the backgauge (no stop in an offset or a flattening).
- **Pre-bend on WILA's 28 degree punches**: BIU-001's body is a catalogue envelope (twice the
  rear extent, errs large), so a base wider than about 10 mm meets it rising at 75 degrees. With
  a drawn section (Trumpf OW222-K) a 40 mm strip passes. Drawn sections for WILA's hemming
  punches (BIU-051, BIU-061: blade 10 / 6, R1, 120 high) would lift this.

## Risks

- `Rule` grew three variants: forge-render's `rule_label` now falls back on `r.word()`
  (BENDSIM-ENV, `bendsim/contact-tint` 22d30cc2); a branch of forge-render without that arm
  will not compile against `bendsim/tools`.
- Places that count a step's bends with `done[st.bend] = true` miss an offset's second bend:
  use `Step::mark`. BENDSIM-ENV has two in forge-render (bendsim.rs:530, gui_bendsim.rs).
- A hem plans as two steps with the same bend: code that assumes one step per bend (indexing
  steps by bend) will see the flattening as a second step.
- The level strokes' checks exempt the stroke's own pieces from the working tools (its forming
  contact) and let the base's side rest on the die (a micron of enclosure slack, `FLAT_REST`):
  a piece of the base's side going down into the die would still be caught (below the plane).
- Pre-verify: run it with nothing else on the machine; parallel builds push tests over the
  2 s / 4 s budgets workspace-wide.
