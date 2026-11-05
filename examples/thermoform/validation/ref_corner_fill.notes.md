# ref_corner_fill: assumptions

- Walls x = 0, y = 0 of the reference become the two faces of a 90 deg V groove (opening 28.284271, depth 14.142136),
  100 long; the strip's clamps (d, 0), (0, d) are the groove's rims (the sheet stuck on the top beside them).
- Plane strain is approximated by the groove's middle section (length / opening = 3.5); end effects remain.
- Compared at the 10 load steps Pi_k = 3 k/10: the achieved fillet R/d (the nearest stuck node to the vertex
  along the wall) and, at the last step, the wall t/t0 against (x/d)^k / lambda_c for x >= 2R.
- The reference itself warns that sticking contact is path dependent: 5 load jumps were 4-13 % off; the model takes
  10 linear steps then the creep raise (pressure x1.25 while any of the part is free).
