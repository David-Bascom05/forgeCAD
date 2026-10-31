# Loft x loft intersection: research behind branch loft-speed

- `BOARD.md`: the shared board (profiles, findings, recipes R1-R5), as it stood when the branch landed.
- `novel/`: what ships. Hull separation and Bernstein sign tests on the exact rational Bezier form of a
  circle-section span, and marching in the surfaces' own parameters (`crates/forge-geom/src/loftmeet.rs`
  cites `novel/README.md`).
- `shipped/`: Lean proofs of what loftmeet.rs certifies (exact patch form, hull, separating axis,
  Bernstein sign test), the assumptions each needs, and its f64 tolerances (`shipped/README.md`).

## Proven alternative, not built: lean-derive's certificate (BOARD R2, R3)
For two circle-section walls whose section frames share one axis (the volute and its core), the walls meet
on a span pair iff g = Psi = 0 has a root in (s1, s2, cos phi2), with g of degree (6,3,1) and Psi of
degree (18,6,2) in Bernstein form; a box clears when either has one strict sign. The iff lemmas and the
Bernstein enclosure are proved in Lean 4 (no sorry). Measured in its reference implementation
(`~/projects/forgecad-gate/research/loft-ssi/derive/cert.py`, f64 with rounding bounds): all 72
volute x core face pairs certified disjoint in 616 boxes, about 1e7 instructions. On the branch, the
certification it would replace costs about 0.25 G of volute's 3.1 G (hull tests and quadric sign tests
together), so it was left out; it is the exact fallback if the hull test ever fails to separate nested
walls. Its sources stay outside the repo since nothing here uses them.
