# eksi2017_cylinder: assumptions

- Mould: dimensions NOT printed (photo only). ASSUMED 100 diameter x 50 deep like the hemisphere and cone, floor
  fillet 6 (over t = 5, for the sheet's offset). LOW CONFIDENCE geometry: treat this case's error as partly geometric.
- index_map: 1 -> 0, 10 -> 50 (base edge, where the minimum sits), 20 -> 100 (rim); 21-25 flange.
- Sheet ABS Terluran HI-10, 5.0 mm, blank 150 x 150: flange 20 + margin 5 gives a 150 window.
- Temperature: measured 120-155 C NON-UNIFORM across the sheet (map not published); modelled uniform 140 C with the
  library ABS Ogden fit (150 C). The non-uniformity is the authors' own explanation of the misfit.
- Vacuum 680-690 mmHg = 0.091 MPa. No plug. Mould aluminium, temperature not given.
- Data x = point INDEX, not arc length: mapped to arc length by the anchors in cases.toml (index_map), evenly spaced
  between anchors (ASSUMED; spacing not printed). Flange points (beyond the rim index) are not compared.
- 5 mm on a 50 mm deep part: membrane theory (no bending) is at its limit; t/R = 0.1.
