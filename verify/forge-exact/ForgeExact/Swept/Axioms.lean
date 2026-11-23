import ForgeExact.Axioms
import ForgeExact.Swept.Prism
import ForgeExact.Swept.Taper
import ForgeExact.Swept.Rev
import ForgeExact.Swept.Screw
import ForgeExact.Swept.Loft

/-!
# The swept solids rest on the standard axioms only

Every main theorem of ForgeExact/Swept is listed (`#standard_axioms`, ForgeExact/Axioms.lean):
the build fails on `sorryAx` or any axiom past propext, Classical.choice and Quot.sound.
-/

#standard_axioms
  ForgeExact.Swept.coords_at
  ForgeExact.Swept.at_coords
  ForgeExact.Swept.prism_iff
  ForgeExact.Swept.prismAll_iff
  ForgeExact.Swept.through_finite
  ForgeExact.Swept.range_twoSided_union
  ForgeExact.Swept.Taper.corner_on_both
  ForgeExact.Swept.Taper.corner_unique
  ForgeExact.Swept.Taper.edge_length
  ForgeExact.Swept.Taper.contact_distance
  ForgeExact.Swept.Taper.hexagon_side
  ForgeExact.Swept.Rev.lt_iff_sq_lt
  ForgeExact.Swept.Rev.lt_mul_sqrt_iff
  ForgeExact.Swept.Rev.reach_arc
  ForgeExact.Swept.Screw.same_point_iff
  ForgeExact.Swept.Screw.coil_injective
  ForgeExact.Swept.ruled_translate
