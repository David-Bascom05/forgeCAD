# Loft x loft intersection: research behind branch loft-speed

- `BOARD.md`: the shared board (profiles, findings, recipes R1-R5), as it stood when the branch landed.
- `novel/`: what ships. Hull separation and Bernstein sign tests on the exact rational Bezier form of a
  circle-section span, and marching in the surfaces' own parameters (`crates/forge-geom/src/loftmeet.rs`
  cites `novel/README.md`).
- `shipped/`: Lean proofs of what loftmeet.rs certifies (exact patch form, hull, separating axis,
  Bernstein sign test), the assumptions each needs, and its f64 tolerances (`shipped/README.md`).

## Proven alternative, not built: lean-derive's exact formula and certificate (BOARD R1-R5)
`derive/` (README there): the exact section-pair polynomial for two arc-rail lofts, proved in Lean 4
with no sorry. General arcs: the sections meet iff F = E1^2 - 4 E0 E2 = 0 (Pluecker coordinates of the
5-column meet system) with E0 <= 0, E2 <= 0, in general position; bidegree (24, 24), tight. For the
volute and its core (one common section axis) F is the square of the bidegree (12, 12) volute formula.
The disjointness certificate built on it (g = Psi = 0 in (s1, s2, cos phi2), Bernstein boxes with an
f64 rounding bound proved under a standard rounding model) certifies all 72 volute x core face pairs in
616 boxes (`derive/cert.py`, about 1e7 instructions). On the branch the certification it would replace
costs about 0.25 G of volute's 3.1 G, so it was left out; it is the exact fallback if the hull test
ever fails to separate nested walls. Nothing in the kernel uses `derive/`.
