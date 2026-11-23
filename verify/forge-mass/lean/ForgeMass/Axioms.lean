import ForgeMass.Table
import ForgeMass.Gauss
import ForgeMass.Fourier
import ForgeMass.Errors
import ForgeMass.Rounding

/-! Every theorem the README cites uses only the standard axioms: the build fails otherwise. -/

/-- info: 'ForgeMass.div_phi' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms ForgeMass.div_phi

/-- info: 'ForgeMass.mom_flux' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms ForgeMass.mom_flux

/-- info: 'ForgeMass.plane_n' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms ForgeMass.plane_n

/-- info: 'ForgeMass.cylinder_n' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms ForgeMass.cylinder_n

/-- info: 'ForgeMass.cone_n' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms ForgeMass.cone_n

/-- info: 'ForgeMass.sphere_n' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms ForgeMass.sphere_n

/-- info: 'ForgeMass.torus_n' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms ForgeMass.torus_n

/-- info: 'ForgeMass.cylinder_area' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms ForgeMass.cylinder_area

/-- info: 'ForgeMass.cone_area' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms ForgeMass.cone_area

/-- info: 'ForgeMass.sphere_signed' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms ForgeMass.sphere_signed

/-- info: 'ForgeMass.sphere_area' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms ForgeMass.sphere_area

/-- info: 'ForgeMass.torus_area' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms ForgeMass.torus_area

/-- info: 'ForgeMass.torus_signed' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms ForgeMass.torus_signed

/-- info: 'ForgeMass.plane_u' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms ForgeMass.plane_u

/-- info: 'ForgeMass.plane_v' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms ForgeMass.plane_v

/-- info: 'ForgeMass.cylinder_u' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms ForgeMass.cylinder_u

/-- info: 'ForgeMass.cylinder_v' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms ForgeMass.cylinder_v

/-- info: 'ForgeMass.cone_u' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms ForgeMass.cone_u

/-- info: 'ForgeMass.cone_v' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms ForgeMass.cone_v

/-- info: 'ForgeMass.sphere_u' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms ForgeMass.sphere_u

/-- info: 'ForgeMass.sphere_v' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms ForgeMass.sphere_v

/-- info: 'ForgeMass.torus_u' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms ForgeMass.torus_u

/-- info: 'ForgeMass.torus_v' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms ForgeMass.torus_v

/-- info: 'ForgeMass.area_cone' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms ForgeMass.area_cone

/-- info: 'ForgeMass.area_sphere' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms ForgeMass.area_sphere

/-- info: 'ForgeMass.area_torus' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms ForgeMass.area_torus

/-- info: 'ForgeMass.gl2_exact' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms ForgeMass.gl2_exact

/-- info: 'ForgeMass.gl3_exact' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms ForgeMass.gl3_exact

/-- info: 'ForgeMass.gl2_G' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms ForgeMass.gl2_G

/-- info: 'ForgeMass.gl3_G' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms ForgeMass.gl3_G

/-- info: 'ForgeMass.sum_exp' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms ForgeMass.sum_exp

/-- info: 'ForgeMass.dft_exact' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms ForgeMass.dft_exact

/-- info: 'ForgeMass.trig_shift' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms ForgeMass.trig_shift

/-- info: 'ForgeMass.G_fourier' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms ForgeMass.G_fourier

/-- info: 'ForgeMass.fourier_G' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms ForgeMass.fourier_G

/-- info: 'ForgeMass.centroid_bound' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms ForgeMass.centroid_bound

/-- info: 'ForgeMass.centroid_err_understates' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms ForgeMass.centroid_err_understates

/-- info: 'ForgeMass.second_moment_bound' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms ForgeMass.second_moment_bound

/-- info: 'ForgeMass.sum_err' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms ForgeMass.sum_err

/-- info: 'ForgeMass.inertia_entry_bound' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms ForgeMass.inertia_entry_bound
