# erdogan2014_cylinder: assumptions

- Mould: opening 200, depth 100 (paper). Draft 4.9 deg, floor fillet 14 DERIVED from the authors' CAD renders
  (s2_geom_case_dims_from_renders.csv: erdogan2014_cylinder wall_draft_angle 4.9 [4.0-5.0], floor_fillet_radius 14 [9-20],
  base_diameter_virtual 183; was draft 0, fillet 10 ASSUMED). Rim edge sharp (rim_fillet_radius_outer 0 [0-3]).
- Law: library HIPS Ogden (fit at 145 C). 180 C is outside the fit (+35 C); the law is used as is (no T shift exists).
- Vacuum 0.09 MPa (lab Yeniyurt machine; level not printed in 2014). Flange 40 (blank 300 x 300 less the clamp).
