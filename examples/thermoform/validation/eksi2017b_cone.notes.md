# eksi2017b_cone: assumptions

- Mould: cone opening 100, depth 50, base 75 (virtual corner), floor fillet 6, DERIVED from the renders
  (s2_geom_case_dims_from_renders.csv: eksi2017_cone base_diameter_virtual_mould 75 [73-77], floor_fillet_radius 6 [4-9]).
- index_map: 1 -> 0, 6 -> 37.5 (base edge), 14.5 -> 89.04 (rim) ASSUMED: point 6 holds the vacuum minimum
  (text p.203: 0.13 mm at point 6, the base radius); the rim is placed where the thickness jumps back to ~t0
  (0.98 mm at 14, 1.51 at 15). Evenly spaced between anchors. Flange points not compared.
- Sheet SABIC PS 825E (HIPS), t0 1.5 mm, blank 150 x 150 (p.199): flange 20 + margin 5 gives a 150 window.
- Temperature: 120-150 C NON-UNIFORM (thermal camera, Fig. 7, 'approximately a 30 C variation'); modelled uniform
  135 C (the middle) with the library HIPS Ogden fit (145 C). No heat map is published.
- Vacuum 730 mmHg (display) = 0.097 MPa. Mould aluminium; the same moulds as eksi2017 (Fig. 4 is the Ruse paper's
  mould photo, s2_eksi2017b_hips1p5_thickness.csv header).
- Data: s2_eksi2017b_hips1p5_thickness.csv, digitised from Fig. 8 (hemisphere) / Fig. 9 (cone), means of 3 parts, +-0.01 mm reading.
  x = point INDEX (20 points, centre of the base to the flange edge; spacing NOT printed).
- Plug-assisted columns not scored: the plug (the aluminium male mould) has no printed size, stroke or temperature.
- Cylinder not modelled: Fig. 10 replots the cone (every marker matches Fig. 9); only text minima exist
  (vacuum 0.09 mm at point 7, plug 0.24 mm at point 8).
