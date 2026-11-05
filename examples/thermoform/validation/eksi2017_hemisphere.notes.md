# eksi2017_hemisphere: assumptions

- Mould: hemisphere opening 100, depth 50 (paper text). Rim sharp.
- index_map: 1 = pole (s 0), 20 = rim (s = pi*50/2 = 78.54).
- Sheet ABS Terluran HI-10, 5.0 mm, blank 150 x 150: flange 20 + margin 5 gives a 150 window.
- Temperature: measured 120-155 C NON-UNIFORM across the sheet (map not published); modelled uniform 140 C with the
  library ABS Ogden fit (150 C). The non-uniformity is the authors' own explanation of the misfit.
- Vacuum 680-690 mmHg = 0.091 MPa. No plug. Mould aluminium, temperature not given.
- Data x = point INDEX, not arc length: mapped to arc length by the anchors in cases.toml (index_map), evenly spaced
  between anchors (ASSUMED; spacing not printed). Flange points (beyond the rim index) are not compared.
- 5 mm on a 50 mm deep part: membrane theory (no bending) is at its limit; t/R = 0.1.
