/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import IEEE754.TwoProduct

/-!
# `dmath`'s constants

Each constant of `kernel/forge-math/src/dmath.rs` as a canonical datum, with `toBits` the bit
pattern Rust gives the literal (checked by `decide`) and its exact value.
-/

namespace IEEE754.Dmath

open FP

def TOINT : F64 := .fin false 6755399441055744 (0) (by decide)
theorem TOINT_bits : TOINT.toBits = 0x4338000000000000 := by decide
theorem TOINT_val : TOINT.val = (6755399441055744 : ℚ) := by
  simp only [TOINT, val, sgn]; norm_num

def INVPIO2 : F64 := .fin false 5734161139222659 (-53) (by decide)
theorem INVPIO2_bits : INVPIO2.toBits = 0x3fe45f306dc9c883 := by decide
theorem INVPIO2_val : INVPIO2.val = (5734161139222659 / 2 ^ 53 : ℚ) := by
  simp only [INVPIO2, val, sgn]; norm_num

def PIO2_1 : F64 := .fin false 7074237751754752 (-52) (by decide)
theorem PIO2_1_bits : PIO2_1.toBits = 0x3ff921fb54400000 := by decide
theorem PIO2_1_val : PIO2_1.val = (1686629713 / 2 ^ 30 : ℚ) := by
  simp only [PIO2_1, val, sgn]; norm_num

def PIO2_1T : F64 := .fin false 4701928774853425 (-86) (by decide)
theorem PIO2_1T_bits : PIO2_1T.toBits = 0x3dd0b4611a626331 := by decide
theorem PIO2_1T_val : PIO2_1T.val = (4701928774853425 / 2 ^ 86 : ℚ) := by
  simp only [PIO2_1T, val, sgn]; norm_num

def PIO2_2 : F64 := .fin false 4701928774696960 (-86) (by decide)
theorem PIO2_2_bits : PIO2_2.toBits = 0x3dd0b4611a600000 := by decide
theorem PIO2_2_val : PIO2_2.val = (2242054355 / 2 ^ 65 : ℚ) := by
  simp only [PIO2_2, val, sgn]; norm_num

def PIO2_2T : F64 := .fin false 5376105825661043 (-121) (by decide)
theorem PIO2_2T_bits : PIO2_2T.toBits = 0x3ba3198a2e037073 := by decide
theorem PIO2_2T_val : PIO2_2T.val = (5376105825661043 / 2 ^ 121 : ℚ) := by
  simp only [PIO2_2T, val, sgn]; norm_num

def PIO2_3 : F64 := .fin false 5376105825435648 (-121) (by decide)
theorem PIO2_3_bits : PIO2_3.toBits = 0x3ba3198a2e000000 := by decide
theorem PIO2_3_val : PIO2_3.val = (160220439 / 2 ^ 96 : ℚ) := by
  simp only [PIO2_3, val, sgn]; norm_num

def PIO2_3T : F64 := .fin false 7744522442262977 (-156) (by decide)
theorem PIO2_3T_bits : PIO2_3T.toBits = 0x397b839a252049c1 := by decide
theorem PIO2_3T_val : PIO2_3T.val = (7744522442262977 / 2 ^ 156 : ℚ) := by
  simp only [PIO2_3T, val, sgn]; norm_num

def S1 : F64 := .fin true 6004799503160649 (-55) (by decide)
theorem S1_bits : S1.toBits = 0xbfc5555555555549 := by decide
theorem S1_val : S1.val = (-6004799503160649 / 2 ^ 55 : ℚ) := by
  simp only [S1, val, sgn]; norm_num

def S2 : F64 := .fin false 4803839602522278 (-59) (by decide)
theorem S2_bits : S2.toBits = 0x3f8111111110f8a6 := by decide
theorem S2_val : S2.val = (2401919801261139 / 2 ^ 58 : ℚ) := by
  simp only [S2, val, sgn]; norm_num

def S3 : F64 := .fin true 7320136532976085 (-65) (by decide)
theorem S3_bits : S3.toBits = 0xbf2a01a019c161d5 := by decide
theorem S3_val : S3.val = (-7320136532976085 / 2 ^ 65 : ℚ) := by
  simp only [S3, val, sgn]; norm_num

def S4 : F64 := .fin false 6506786730409597 (-71) (by decide)
theorem S4_bits : S4.toBits = 0x3ec71de357b1fe7d := by decide
theorem S4_val : S4.val = (6506786730409597 / 2 ^ 71 : ℚ) := by
  simp only [S4, val, sgn]; norm_num

def S5 : F64 := .fin true 7571127717829867 (-78) (by decide)
theorem S5_bits : S5.toBits = 0xbe5ae5e68a2b9ceb := by decide
theorem S5_val : S5.val = (-7571127717829867 / 2 ^ 78 : ℚ) := by
  simp only [S5, val, sgn]; norm_num

def S6 : F64 := .fin false 6149819165824380 (-85) (by decide)
theorem S6_bits : S6.toBits = 0x3de5d93a5acfd57c := by decide
theorem S6_val : S6.val = (1537454791456095 / 2 ^ 83 : ℚ) := by
  simp only [S6, val, sgn]; norm_num

def C1 : F64 := .fin false 6004799503160652 (-57) (by decide)
theorem C1_bits : C1.toBits = 0x3fa555555555554c := by decide
theorem C1_val : C1.val = (1501199875790163 / 2 ^ 55 : ℚ) := by
  simp only [C1, val, sgn]; norm_num

def C2 : F64 := .fin true 6405119470031223 (-62) (by decide)
theorem C2_bits : C2.toBits = 0xbf56c16c16c15177 := by decide
theorem C2_val : C2.val = (-6405119470031223 / 2 ^ 62 : ℚ) := by
  simp only [C2, val, sgn]; norm_num

def C3 : F64 := .fin false 7320136533611920 (-68) (by decide)
theorem C3_bits : C3.toBits = 0x3efa01a019cb1590 := by decide
theorem C3_val : C3.val = (457508533350745 / 2 ^ 64 : ℚ) := by
  simp only [C3, val, sgn]; norm_num

def C4 : F64 := .fin true 5205429506036397 (-74) (by decide)
theorem C4_bits : C4.toBits = 0xbe927e4f809c52ad := by decide
theorem C4_val : C4.val = (-5205429506036397 / 2 ^ 74 : ℚ) := by
  simp only [C4, val, sgn]; norm_num

def C5 : F64 := .fin false 5047440159060420 (-81) (by decide)
theorem C5_bits : C5.toBits = 0x3e21ee9ebdb4b1c4 := by decide
theorem C5_val : C5.val = (1261860039765105 / 2 ^ 79 : ℚ) := by
  simp only [C5, val, sgn]; norm_num

def C6 : F64 := .fin true 7031281271978196 (-89) (by decide)
theorem C6_bits : C6.toBits = 0xbda8fae9be8838d4 := by decide
theorem C6_val : C6.val = (-1757820317994549 / 2 ^ 87 : ℚ) := by
  simp only [C6, val, sgn]; norm_num

end IEEE754.Dmath
