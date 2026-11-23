/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import IEEE754.Dmath.Consts

/-!
# `dmath::atan`, `atan2`'s constants

Each is the exact datum of the Rust literal, with its bit pattern checked (`decide`) against
`f64::to_bits` of that literal. `PI`, `FRAC_PI_2` are `core::f64::consts`; `TINY27 = 2^-27` and
`BIG66 = 2^66` are `atan`'s thresholds.
-/

namespace IEEE754.Dmath

open FP

def AT0 : F64 := .fin false 6004799503160589 (-54) (by decide)
theorem AT0_bits : AT0.toBits = 0x3fd555555555550d := by decide
theorem AT0_val : AT0.val = (6004799503160589 / 2 ^ 54 : ℚ) := by
  simp only [AT0, val, sgn]; norm_num

def AT1 : F64 := .fin true 7205759403748292 (-55) (by decide)
theorem AT1_bits : AT1.toBits = 0xbfc999999998ebc4 := by decide
theorem AT1_val : AT1.val = (-1801439850937073 / 2 ^ 53 : ℚ) := by
  simp only [AT1, val, sgn]; norm_num

def AT2 : F64 := .fin false 5146970997949439 (-55) (by decide)
theorem AT2_bits : AT2.toBits = 0x3fc24924920083ff := by decide
theorem AT2_val : AT2.val = (5146970997949439 / 2 ^ 55 : ℚ) := by
  simp only [AT2, val, sgn]; norm_num

def AT3 : F64 := .fin true 8006398829074033 (-56) (by decide)
theorem AT3_bits : AT3.toBits = 0xbfbc71c6fe231671 := by decide
theorem AT3_val : AT3.val = (-8006398829074033 / 2 ^ 56 : ℚ) := by
  simp only [AT3, val, sgn]; norm_num

def AT4 : F64 := .fin false 6550674545057902 (-56) (by decide)
theorem AT4_bits : AT4.toBits = 0x3fb745cdc54c206e := by decide
theorem AT4_val : AT4.val = (3275337272528951 / 2 ^ 55 : ℚ) := by
  simp only [AT4, val, sgn]; norm_num

def AT5 : F64 := .fin true 5542580929731181 (-56) (by decide)
theorem AT5_bits : AT5.toBits = 0xbfb3b0f2af749a6d := by decide
theorem AT5_val : AT5.val = (-5542580929731181 / 2 ^ 56 : ℚ) := by
  simp only [AT5, val, sgn]; norm_num

def AT6 : F64 := .fin false 4799809039908177 (-56) (by decide)
theorem AT6_bits : AT6.toBits = 0x3fb10d66a0d03d51 := by decide
theorem AT6_val : AT6.val = (4799809039908177 / 2 ^ 56 : ℚ) := by
  simp only [AT6, val, sgn]; norm_num

def AT7 : F64 := .fin true 8407060569849242 (-57) (by decide)
theorem AT7_bits : AT7.toBits = 0xbfadde2d52defd9a := by decide
theorem AT7_val : AT7.val = (-4203530284924621 / 2 ^ 56 : ℚ) := by
  simp only [AT7, val, sgn]; norm_num

def AT8 : F64 := .fin false 7172437082246635 (-57) (by decide)
theorem AT8_bits : AT8.toBits = 0x3fa97b4b24760deb := by decide
theorem AT8_val : AT8.val = (7172437082246635 / 2 ^ 57 : ℚ) := by
  simp only [AT8, val, sgn]; norm_num

def AT9 : F64 := .fin true 5264754476739631 (-57) (by decide)
theorem AT9_bits : AT9.toBits = 0xbfa2b4442c6a6c2f := by decide
theorem AT9_val : AT9.val = (-5264754476739631 / 2 ^ 57 : ℚ) := by
  simp only [AT9, val, sgn]; norm_num

def AT10 : F64 := .fin false 4694068057790993 (-58) (by decide)
theorem AT10_bits : AT10.toBits = 0x3f90ad3ae322da11 := by decide
theorem AT10_val : AT10.val = (4694068057790993 / 2 ^ 58 : ℚ) := by
  simp only [AT10, val, sgn]; norm_num

def ATANHI0 : F64 := .fin false 8352332796509007 (-54) (by decide)
theorem ATANHI0_bits : ATANHI0.toBits = 0x3fddac670561bb4f := by decide
theorem ATANHI0_val : ATANHI0.val = (8352332796509007 / 2 ^ 54 : ℚ) := by
  simp only [ATANHI0, val, sgn]; norm_num

def ATANHI1 : F64 := .fin false 7074237752028440 (-53) (by decide)
theorem ATANHI1_bits : ATANHI1.toBits = 0x3fe921fb54442d18 := by decide
theorem ATANHI1_val : ATANHI1.val = (884279719003555 / 2 ^ 50 : ℚ) := by
  simp only [ATANHI1, val, sgn]; norm_num

def ATANHI2 : F64 := .fin false 8852218891597467 (-53) (by decide)
theorem ATANHI2_bits : ATANHI2.toBits = 0x3fef730bd281f69b := by decide
theorem ATANHI2_val : ATANHI2.val = (8852218891597467 / 2 ^ 53 : ℚ) := by
  simp only [ATANHI2, val, sgn]; norm_num

def ATANHI3 : F64 := .fin false 7074237752028440 (-52) (by decide)
theorem ATANHI3_bits : ATANHI3.toBits = 0x3ff921fb54442d18 := by decide
theorem ATANHI3_val : ATANHI3.val = (884279719003555 / 2 ^ 49 : ℚ) := by
  simp only [ATANHI3, val, sgn]; norm_num

def ATANLO0 : F64 := .fin false 7366174428849634 (-108) (by decide)
theorem ATANLO0_bits : ATANLO0.toBits = 0x3c7a2b7f222f65e2 := by decide
theorem ATANLO0_val : ATANLO0.val = (3683087214424817 / 2 ^ 107 : ℚ) := by
  simp only [ATANLO0, val, sgn]; norm_num

def ATANLO1 : F64 := .fin false 4967757600021511 (-107) (by decide)
theorem ATANLO1_bits : ATANLO1.toBits = 0x3c81a62633145c07 := by decide
theorem ATANLO1_val : ATANLO1.val = (4967757600021511 / 2 ^ 107 : ℚ) := by
  simp only [ATANLO1, val, sgn]; norm_num

def ATANLO2 : F64 := .fin false 4511882386918333 (-108) (by decide)
theorem ATANLO2_bits : ATANLO2.toBits = 0x3c7007887af0cbbd := by decide
theorem ATANLO2_val : ATANLO2.val = (4511882386918333 / 2 ^ 108 : ℚ) := by
  simp only [ATANLO2, val, sgn]; norm_num

def ATANLO3 : F64 := .fin false 4967757600021511 (-106) (by decide)
theorem ATANLO3_bits : ATANLO3.toBits = 0x3c91a62633145c07 := by decide
theorem ATANLO3_val : ATANLO3.val = (4967757600021511 / 2 ^ 106 : ℚ) := by
  simp only [ATANLO3, val, sgn]; norm_num

def PI : F64 := .fin false 7074237752028440 (-51) (by decide)
theorem PI_bits : PI.toBits = 0x400921fb54442d18 := by decide
theorem PI_val : PI.val = (884279719003555 / 2 ^ 48 : ℚ) := by
  simp only [PI, val, sgn]; norm_num

def PI_LO : F64 := .fin false 4967757600021511 (-105) (by decide)
theorem PI_LO_bits : PI_LO.toBits = 0x3ca1a62633145c07 := by decide
theorem PI_LO_val : PI_LO.val = (4967757600021511 / 2 ^ 105 : ℚ) := by
  simp only [PI_LO, val, sgn]; norm_num

def FRAC_PI_2 : F64 := .fin false 7074237752028440 (-52) (by decide)
theorem FRAC_PI_2_bits : FRAC_PI_2.toBits = 0x3ff921fb54442d18 := by decide
theorem FRAC_PI_2_val : FRAC_PI_2.val = (884279719003555 / 2 ^ 49 : ℚ) := by
  simp only [FRAC_PI_2, val, sgn]; norm_num

def TWO : F64 := .fin false 4503599627370496 (-51) (by decide)
theorem TWO_bits : TWO.toBits = 0x4000000000000000 := by decide
theorem TWO_val : TWO.val = (2 : ℚ) := by
  simp only [TWO, val, sgn]; norm_num

def THREEHALF : F64 := .fin false 6755399441055744 (-52) (by decide)
theorem THREEHALF_bits : THREEHALF.toBits = 0x3ff8000000000000 := by decide
theorem THREEHALF_val : THREEHALF.val = (3 / 2 ^ 1 : ℚ) := by
  simp only [THREEHALF, val, sgn]; norm_num

def MONE : F64 := .fin true 4503599627370496 (-52) (by decide)
theorem MONE_bits : MONE.toBits = 0xbff0000000000000 := by decide
theorem MONE_val : MONE.val = (-1 : ℚ) := by
  simp only [MONE, val, sgn]; norm_num

def TINY27 : F64 := .fin false 4503599627370496 (-79) (by decide)
theorem TINY27_bits : TINY27.toBits = 0x3e40000000000000 := by decide
theorem TINY27_val : TINY27.val = (1 / 2 ^ 27 : ℚ) := by
  simp only [TINY27, val, sgn]; norm_num

def BIG66 : F64 := .fin false 4503599627370496 (14) (by decide)
theorem BIG66_bits : BIG66.toBits = 0x4410000000000000 := by decide
theorem BIG66_val : BIG66.val = (73786976294838206464 : ℚ) := by
  simp only [BIG66, val, sgn]; norm_num

end IEEE754.Dmath
