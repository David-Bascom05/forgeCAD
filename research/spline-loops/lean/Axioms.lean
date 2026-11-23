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
#print axioms nb_continuous
#print axioms iter_curveOn
#print axioms curve_smooth
