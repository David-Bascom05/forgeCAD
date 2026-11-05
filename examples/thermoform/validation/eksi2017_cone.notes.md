# eksi2017_cone: assumptions

- Mould: opening 100, depth 50 (paper). Base diameter NOT printed: 77.0 DERIVED from the point labels (1 = centre,
  10 = base edge, 22 = rim) assuming even point spacing: rb/9 = slant/12. Floor fillet 6 ASSUMED (the kernel needs a round over t = 5 to offset the sheet).
- index_map: 1 -> 0, 10 -> 38.5, 22 -> 89.805458.
- Sheet ABS Terluran HI-10, 5.0 mm, blank 150 x 150: flange 20 + margin 5 gives a 150 window.
- Temperature: measured 120-155 C NON-UNIFORM across the sheet (map not published); modelled uniform 140 C with the
  library ABS Ogden fit (150 C). The non-uniformity is the authors' own explanation of the misfit.
- Vacuum 680-690 mmHg = 0.091 MPa. No plug. Mould aluminium, temperature not given.
- Data x = point INDEX, not arc length: mapped to arc length by the anchors in cases.toml (index_map), evenly spaced
  between anchors (ASSUMED; spacing not printed). Flange points (beyond the rim index) are not compared.
- 5 mm on a 50 mm deep part: membrane theory (no bending) is at its limit; t/R = 0.1.
