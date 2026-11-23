# Conic fillets: what is left

2026-11-23, at the end of the conic-fillet session (branch `claude/repo-review-mbdrlj-conic-fillet`, merged
with round 6). What is built and how it is checked is in `README.md`.

## Not built

- **Variable radius with a conic section.** Refused (`ConicEdge`). Onshape's variable conic exists. The
  census's 3 variable parts are circular (`isVariable`).
- **Corners only a ball closes.** Three conic blends at a vertex, a pivot, a roll-over, setbacks: refused
  (`ConicVertex`). The fillet spec has no conic sphere corner. It leaves this row to refusal or its one named
  G1 corner rule, which is not built.
- **Rims, cliffs, rest beyond a smooth edge, `across` full rounds with a conic:** refused (`ConicEdge`). These
  are ball constructions with no conic counterpart defined yet.
- **Width-measured conics.** Onshape's "width" mode, where the setback is given instead of the radius, has no
  `.fg` form.
- **Curvature fillets.** Onshape's `CURVATURE` cross-section is refused in the importer.

## Built but held

- **Phase 2, `ConicPipe`, for edges of no closed form** (`a6dcc3e`, `49ce541`). Its gates pass on 3dcdfb4
  and on round 6. The master holds it for the round after phase 1.
- **The conic's bbox bound in `ConicPipe::new`.** It is taken from 256 samples of theta plus each one's slope,
  and is refused within a degree of folding. It is not a proven bound. A proven bound on theta along a traced
  spine would make it one.

## Seen, not ours

- `repro/spring_touches_hole.fg`: the circular fillet builds an INVALID body when a spring runs tangent to a
  hole's rim. This is the priority for the fillet-kernel session: an invalid result with no refusal.
- `repro/counterbore_wall.fg`: the circular fillet refuses a round whose spring a counterbore's head reaches
  past (CADFS 00961869).
- `fillet` refusals read as "the kernel refused it: fillet" in the language. The typed variant (`ConicEdge`,
  `Overflow`, ...) is in `--json` only as Debug. That is shared with every fillet error.

## Census

CADFS conic parts still blocked by other gaps:

| part | blocked by |
|---|---|
| 00368441 | sketch text |
| 00703705 | an unequal chamfer |
| 00909757 | a split piece |
| 00961869 | `BLEND_EDGE` of an unknown feature |
| 00330038 | `SWEPT_FACE` reference |
| 00838464 | region query |

00092585 translates, but is refused by the circular overflow in `spring_touches_hole.fg`.
