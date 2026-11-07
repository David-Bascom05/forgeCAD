# eksi2017_cylinder: assumptions

- Mould: dimensions NOT printed (photo only). Opening 100 x depth 50 ASSUMED-SUPPORTED (s2_geom_case_dims_from_renders.csv: eksi2017_cylinder
  opening_diameter 100 [100-110]: render h:d 0.48-0.50 like the 100 x 50 cone). Draft 4.9 deg DERIVED (same CAD shape as
  erdogan2014_cylinder: wall_draft_angle 4.9 [4-5.5], base_over_opening_ratio 0.915; was 0 ASSUMED), floor fillet 6
  (floor_fillet_radius 6 [4-8]). Geometry still LOW CONFIDENCE (opening not printed).
- index_map: 1 -> 0, 10 -> 50 (base edge, where the minimum sits), 20 -> 100 (rim); 21-25 flange.
- Sheet ABS Terluran HI-10, 5.0 mm, blank 150 x 150: flange 20 + margin 5 gives a 150 window.
- Temperature: measured 120-155 C NON-UNIFORM across the sheet (map not published); modelled uniform 140 C with the
  library ABS Ogden fit (150 C). The non-uniformity is the authors' own explanation of the misfit.
- Vacuum 680-690 mmHg = 0.091 MPa. No plug. Mould aluminium, temperature not given.
- Data x = point INDEX, not arc length: mapped to arc length by the anchors in cases.toml (index_map), evenly spaced
  between anchors (ASSUMED; spacing not printed). Flange points (beyond the rim index) are not compared.
- 5 mm on a 50 mm deep part: membrane theory (no bending) is at its limit; t/R = 0.1.
