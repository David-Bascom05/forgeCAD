# ref_inflation_mooney_rivlin: assumptions

- Exact reference (DERIVED, axisymmetric ODE): pole height H = h/a and pole thickness t/t0 against P.
- Set-up: a deep female cylinder, radius a = 50, the sheet stuck on the top round it = clamped at r = a (the grid's
  staircase approximates the circle: an O(h/a) error).
- Load: P_end = 2 (well below the table end); the 10 load steps
  P_end k/10 are compared, the pole taken at the window's centre node.
- The model's pressure is a dead load on the deformed surface's projected volume (p x signed volume): the
  follower load of the reference. Exact up to the mesh.
