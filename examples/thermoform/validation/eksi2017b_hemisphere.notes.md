# eksi2017b_hemisphere: assumptions

- Mould: hemisphere opening 100, depth 50 (eksi2017 Ruse paper; same mould, s2_eksi2017b_hips1p5_thickness.csv header).
- index_map: 1 -> 0 (pole), 14.5 -> 78.54 (rim) ASSUMED: the rim index is placed where the measured thickness
  jumps back to ~t0 (1.30 mm at 14, 1.61 at 15); points evenly spaced in between. Flange points not compared.
- Sheet SABIC PS 825E (HIPS), t0 1.5 mm, blank 150 x 150 (p.199): flange 20 + margin 5 gives a 150 window.
- Temperature: 120-150 C NON-UNIFORM (thermal camera, Fig. 7, 'approximately a 30 C variation'); modelled uniform
  135 C (the middle) with the library HIPS Ogden fit (145 C). No heat map is published.
- Vacuum 730 mmHg (display) = 0.097 MPa. Mould aluminium; the same moulds as eksi2017 (Fig. 4 is the Ruse paper's
  mould photo, s2_eksi2017b_hips1p5_thickness.csv header).
- Data: s2_eksi2017b_hips1p5_thickness.csv, digitised from Fig. 8 (hemisphere) / Fig. 9 (cone), means of 3 parts, +-0.01 mm reading.
  x = point INDEX (20 points, centre of the base to the flange edge; spacing NOT printed).
- Plug-assisted columns not scored: the plug (the aluminium male mould) has no printed size, stroke or temperature.
- Text check (p.202): vacuum minimum 0.35 mm at point 1 (the pole), as digitised.
