import SplineSpec
open SplineSpec
-- `lake env lean Axioms.lean`: every line must print only propext, Classical.choice, Quot.sound.
-- The through-spline: exists, is unique, passes its points; the slope system's bound.
#print axioms interp_exists_unique
#print axioms interp_through
#print axioms slopes_exist
#print axioms slopes_unique
#print axioms slopes_bound
-- The basis: partition of unity, nonnegative, convex hull, exact clamped ends.
#print axioms nb_sum_one
#print axioms nb_nonneg
#print axioms eval_between
#print axioms clamped_start
#print axioms clamped_end
-- The language's through-spline at centripetal parameters.
#print axioms centripetal_strictMono
#print axioms spline_exists_unique
-- Derivatives are B-splines of one degree less; C^(p-k) at a knot of multiplicity k.
#print axioms hasDerivAt_nb
#print axioms hasDerivAt_curve
#print axioms hasDerivAt_rational
#print axioms nb_continuous
#print axioms iter_curveOn
#print axioms curve_smooth
-- T6: knot insertion leaves the curve unchanged.
#print axioms insert_basis
#print axioms insert_curve
-- The oracle's answers are proofs: a control spline's Bernstein coefficients are the span's
-- polynomial; pieces within e at their points are within e; the intervals enclose; the
-- through-spline's slopes and pieces are where the oracle says.
#print axioms span_bernstein
#print axioms bern_close
#print axioms Oracle.in_add
#print axioms Oracle.in_mul
#print axioms Oracle.in_div
#print axioms Oracle.in_sqrt
#print axioms Oracle.in_far
#print axioms Oracle.rows_in
#print axioms Oracle.residual_in
#print axioms Oracle.through_cert
-- A profile loop's simplicity: boxes apart do not touch; cones apart meet only at the joint.
#print axioms apart_of_boxes
#print axioms cone_sep
#print axioms convex_in_cone
#print axioms injective_of_dir_deriv_pos
#print axioms bern_pos_off_start
-- The kernel's tridiagonal solve, in exact arithmetic: nonzero pivots, every row satisfied.
#print axioms pivot_ne
#print axioms thomas_solves
