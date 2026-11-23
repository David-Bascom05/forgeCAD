import ForgeFont.Funs
import ForgeText

open Aeneas Aeneas.Std Result ControlFlow
open forge_font

namespace ForgeFont

/-- A point of the translation as the spec's. -/
def toPt (p : glyf_core.Pi) : ForgeText.Pt := ⟨p.x.val, p.y.val⟩

def toSeg : glyf_core.SegI → ForgeText.Seg
  | .Line a b => .line (toPt a) (toPt b)
  | .Quad a c b => .quad (toPt a) (toPt c) (toPt b)

def toWalk (w : glyf_core.Walk) : ForgeText.Walk :=
  { at_ := toPt w.at_, ctrl := w.ctrl.map toPt, out := w.out.val.map toSeg }

/-- A point a glyph's coordinates make on the doubled grid: even, within ±2³². -/
def Grid (p : glyf_core.Pi) : Prop :=
  2 ∣ p.x.val ∧ 2 ∣ p.y.val ∧ -2^32 ≤ p.x.val ∧ p.x.val ≤ 2^32 ∧ -2^32 ≤ p.y.val ∧ p.y.val ≤ 2^32

theorem toPt_eq {p q : glyf_core.Pi} : toPt p = toPt q ↔ p = q := by
  obtain ⟨px, py⟩ := p
  obtain ⟨qx, qy⟩ := q
  simp [toPt, IScalar.eq_equiv]

theorem eq_spec (p q : glyf_core.Pi) :
    glyf_core.Pi.Insts.CoreCmpPartialEqPi.eq p q ⦃ b => b ↔ p = q ⦄ := by
  obtain ⟨px, py⟩ := p
  obtain ⟨qx, qy⟩ := q
  unfold glyf_core.Pi.Insts.CoreCmpPartialEqPi.eq
  split <;> simp_all

@[step]
theorem ne_spec (p q : glyf_core.Pi) :
    core.cmp.PartialEq.ne.trait_default glyf_core.Pi.Insts.CoreCmpPartialEqPi p q
      ⦃ b => b ↔ p ≠ q ⦄ :=
  core.cmp.PartialEq.ne.trait_default.spec _ p q (eq_spec p q)

/-- Even sums halve the same way truncated (Rust) and floored (the spec). -/
theorem tdiv_two {a : Int} (h : 2 ∣ a) : Int.tdiv a 2 = a / 2 :=
  Int.tdiv_eq_ediv_of_dvd h

@[step]
theorem mid_spec (a b : glyf_core.Pi) (ha : Grid a) (hb : Grid b) :
    glyf_core.mid a b ⦃ m => toPt m = ForgeText.mid (toPt a) (toPt b) ⦄ := by
  obtain ⟨ax2, ay2, ax0, ax1, ay0, ay1⟩ := ha
  obtain ⟨bx2, by2, bx0, bx1, by0, by1⟩ := hb
  unfold glyf_core.mid
  step*
  simp only [toPt, ForgeText.mid, ForgeText.Pt.mk.injEq]
  rw [i1_post, i_post, tdiv_two (dvd_add ax2 bx2), i3_post, i2_post, tdiv_two (dvd_add ay2 by2)]
  simp

/-- What the walk keeps of its control point: it is a glyph's point. -/
def CtrlOk (w : glyf_core.Walk) : Prop := ∀ c, w.ctrl = some c → Grid c

/-- **One step of the translated walk is the spec's `step`**, for a control point and an off
point on the grid (an on point may be anything: the closing start is a midpoint). -/
theorem step_spec (w : glyf_core.Walk) (p : glyf_core.Pi) (on : Bool) (hc : CtrlOk w)
    (hp : on = false → Grid p) (hl : w.out.val.length < Usize.max) :
    glyf_core.step w p on ⦃ w' => toWalk w' = ForgeText.step (toWalk w) (toPt p, on) ∧
      CtrlOk w' ∧ w'.out.val.length ≤ w.out.val.length + 1 ⦄ := by
  obtain ⟨at_, ctrl, out⟩ := w
  unfold glyf_core.step
  cases on <;> cases ctrl
  · simp [toWalk, ForgeText.step, CtrlOk, hp rfl]
  · rename_i c
    have hc' : Grid c := hc c rfl
    step*
    all_goals
      have hm : ForgeText.mid (toPt c) (toPt p) = toPt at_ ↔ m = at_ := by rw [← m_post, toPt_eq]
      simp only [toWalk, ForgeText.step, Option.map, hm]
      simp_all [toSeg, CtrlOk]
  all_goals (step* <;> simp only [toWalk, ForgeText.step, Option.map] <;> simp_all [toPt_eq, toSeg, CtrlOk])


end ForgeFont
