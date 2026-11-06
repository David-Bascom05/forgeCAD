# eksi2017_cone: assumptions

- Mould: opening 100, depth 50 (paper). Base diameter 75 (virtual corner) DERIVED from the authors' renders
  (s2_geom_case_dims_from_renders.csv: eksi2017_cone base_diameter_virtual_mould 75 [73-77], base_over_opening_ratio 0.735 from two
  renders); was 77.0 derived from the point labels. Floor fillet 6 (s2_geom_case_dims_from_renders.csv: floor_fillet_radius 6 [4-9]).
- index_map: 1 -> 0, 10 -> 37.5 (base edge), 22 -> 89.04 (37.5 + the 51.54 slant).
- Sheet ABS Terluran HI-10, 5.0 mm, blank 150 x 150: flange 20 + margin 5 gives a 150 window.
- Temperature: measured 120-155 C NON-UNIFORM across the sheet (map not published); modelled uniform 140 C with the
  library ABS Ogden fit (150 C). The non-uniformity is the authors' own explanation of the misfit.
- Vacuum 680-690 mmHg = 0.091 MPa. No plug. Mould aluminium, temperature not given.
- Data x = point INDEX, not arc length: mapped to arc length by the anchors in cases.toml (index_map), evenly spaced
  between anchors (ASSUMED; spacing not printed). Flange points (beyond the rim index) are not compared.
- 5 mm on a 50 mm deep part: membrane theory (no bending) is at its limit; t/R = 0.1.
