/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import Lean
import IEEE754.Binary64
import IEEE754.Compare
import IEEE754.Computed
import IEEE754.Dmath.Reduction
import IEEE754.Dmath.CosPoly
import IEEE754.Dmath.SinCos
import IEEE754.Dmath.AtanPoly
import IEEE754.Dmath.AtanEval
import IEEE754.Dmath.AtanFacts
import IEEE754.Dmath.AsinAcos

/-!
# Only the standard axioms

`#standard_axioms T₁ T₂ …` fails the build if a named theorem rests on any axiom other than
`propext`, `Classical.choice` and `Quot.sound` (a `sorry` is the axiom `sorryAx`, and fails too).
Every main theorem of the package is listed, and this module is in the default build.
-/

open Lean Elab Command

elab "#standard_axioms " ids:ident* : command => do
  for id in ids do
    let n ← liftCoreM (realizeGlobalConstNoOverloadWithInfo id)
    let axs ← liftCoreM (collectAxioms n)
    for a in axs do
      unless a == ``propext || a == ``Classical.choice || a == ``Quot.sound do
        throwError "{n} rests on the axiom {a}"

#standard_axioms
  -- rounding
  IEEE754.round_rel_err IEEE754.round_abs_err IEEE754.round_err IEEE754.round_mono
  IEEE754.round_val IEEE754.round_exact
  -- each operation is the rounding of its exact result
  IEEE754.add_eq_round IEEE754.sub_eq_round IEEE754.mul_eq_round IEEE754.div_eq_round
  IEEE754.fma_eq_round IEEE754.sqrt_eq_round IEEE754.sqrtPos_eq
  -- error bounds
  IEEE754.add_rel_err IEEE754.sub_rel_err IEEE754.mul_rel_err IEEE754.div_rel_err
  IEEE754.fma_rel_err IEEE754.sqrt_rel_err
  IEEE754.add_abs_err IEEE754.sub_abs_err IEEE754.mul_abs_err IEEE754.div_abs_err
  IEEE754.fma_abs_err IEEE754.add_exact_of_small
  IEEE754.F64.add_rel_err IEEE754.F64.sub_rel_err IEEE754.F64.mul_rel_err IEEE754.F64.div_rel_err
  IEEE754.F64.fma_rel_err IEEE754.F64.mul_abs_err
  -- Sterbenz, monotonicity, integers
  IEEE754.sterbenz IEEE754.sqrt_mono
  IEEE754.FP.ofInt_exact IEEE754.FP.toInt_ofInt IEEE754.FP.add_int_exact IEEE754.FP.mul_int_exact
  IEEE754.F64.ofInt_exact
  -- identities
  IEEE754.FP.mul_one IEEE754.FP.one_mul IEEE754.FP.div_one IEEE754.FP.add_neg_zero
  IEEE754.FP.add_zero IEEE754.FP.neg_zero_add IEEE754.FP.sub_zero IEEE754.FP.add_comm
  IEEE754.FP.mul_comm IEEE754.FP.sub_self IEEE754.FP.fma_neg_zero IEEE754.FP.neg_neg
  IEEE754.FP.val_neg IEEE754.FP.abs_val
  -- comparisons
  IEEE754.FP.lt_iff IEEE754.FP.le_iff IEEE754.FP.eq_iff IEEE754.FP.zero_eq_zero
  -- encoding
  IEEE754.FP.ofBits_toBits IEEE754.FP.toBits_ofBits IEEE754.FP.toBits_lt IEEE754.F64.toBits_lt
  -- error relative to the result
  IEEE754.round_rel_err_res IEEE754.add_err_res IEEE754.sub_err_res IEEE754.mul_err_res
  IEEE754.round_err_abs IEEE754.mul_err_abs
  -- nearest
  IEEE754.rne_nearest IEEE754.round_nearest
  IEEE754.add_err_representable IEEE754.fast2sum IEEE754.fast_two_sum_exact IEEE754.two_sum_exact_of_le
  IEEE754.two_sum_core IEEE754.two_sum_exact
  IEEE754.veltkamp_core IEEE754.veltkamp IEEE754.dekker_core IEEE754.two_product_exact
  IEEE754.F64.two_product_exact
  -- dmath: rem_pio2
  IEEE754.Dmath.quadrant IEEE754.Dmath.rem1 IEEE754.Dmath.tail IEEE754.Dmath.refine
  IEEE754.Dmath.remPio2_rat IEEE754.Dmath.remPio2_spec IEEE754.Dmath.ksin_poly IEEE754.Dmath.kcos_poly
  IEEE754.Dmath.ksin_eval IEEE754.Dmath.half_mul IEEE754.Dmath.kcos_eval IEEE754.Dmath.pi_gt_d30 IEEE754.Dmath.pi_lt_d30
  IEEE754.Dmath.ksin_err IEEE754.Dmath.kcos_err IEEE754.Dmath.sinCos_err IEEE754.Dmath.sin_err IEEE754.Dmath.cos_err
  IEEE754.Dmath.atan_taylor IEEE754.Dmath.katan_poly IEEE754.Dmath.round_ulp IEEE754.Dmath.addU IEEE754.Dmath.subU
  IEEE754.Dmath.atanPoly_eval IEEE754.Dmath.arctan_lip IEEE754.Dmath.arctan_lip_m IEEE754.Dmath.red0 IEEE754.Dmath.red1
  IEEE754.Dmath.red2 IEEE754.Dmath.red3 IEEE754.Dmath.c_half IEEE754.Dmath.c_quarter IEEE754.Dmath.c_threehalf
  IEEE754.Dmath.c_pio2 IEEE754.Dmath.c_pi IEEE754.Dmath.c_PI IEEE754.Dmath.c_FRAC_PI_2
  IEEE754.Dmath.add_ulp IEEE754.Dmath.sub_ulp IEEE754.Dmath.mul_ulp IEEE754.Dmath.div_ulp
  IEEE754.Dmath.reduced_err IEEE754.Dmath.red_t0 IEEE754.Dmath.red_t1 IEEE754.Dmath.red_t2 IEEE754.Dmath.red_t3
  IEEE754.Dmath.big_err IEEE754.Dmath.small_err IEEE754.Dmath.tiny_err IEEE754.Dmath.atan_err
  IEEE754.Dmath.quot_le IEEE754.Dmath.arctan_rel IEEE754.Dmath.zA IEEE754.Dmath.zB IEEE754.Dmath.zC IEEE754.Dmath.quad
  IEEE754.Dmath.atan2_err IEEE754.Dmath.atan2_err50 IEEE754.Dmath.arctan_rel_gen
  IEEE754.Dmath.prod_err IEEE754.Dmath.sqrt_pert IEEE754.Dmath.sqrtArg_err IEEE754.Dmath.pert_y IEEE754.Dmath.pert_x
  IEEE754.Dmath.arccos_eq_atan2R IEEE754.Dmath.arcsin_eq_atan2R IEEE754.Dmath.asin_err IEEE754.Dmath.acos_err
