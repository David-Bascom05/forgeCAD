/-!
# The history as a Boolean formula, its two- and three-valued values

The curved reference decides the history on sign patterns (`eval2`), and over a box whose signs
are only partly certified, three-valued (`eval3`, Kleene's strong connectives). `eval3_sound`:
a value it gives is the formula's value under every completion of the certified signs.

`allFalse` enumerates the completions of the undecided signs that no exclusion forbids.
`allFalse_sound`: when it answers `true`, the formula is false on every pattern that agrees
with the certified signs and avoids the exclusions.

These are the functions the executable runs (ForgeExact/QRef/Decide.lean).
-/

namespace ForgeExact.QRef

/-- A formula over sign literals: quadric `i` positive (`pos`) or not. -/
inductive BF
  | tt
  | ff
  | lit (i : Nat) (pos : Bool)
  | not (a : BF)
  | and (a b : BF)
  | or (a b : BF)
  deriving Inhabited, Repr

/-- An exclusion: no point has quadric `i` of sign `si` and `j` of sign `sj`. -/
structure Excl where
  i : Nat
  si : Bool
  j : Nat
  sj : Bool
  deriving Inhabited, BEq

namespace BF

def eval2 (σ : Nat → Bool) : BF → Bool
  | tt => true
  | ff => false
  | lit i pos => σ i == pos
  | not a => !a.eval2 σ
  | and a b => a.eval2 σ && b.eval2 σ
  | or a b => a.eval2 σ || b.eval2 σ

def and3 : Option Bool → Option Bool → Option Bool
  | some false, _ => some false
  | _, some false => some false
  | some true, some true => some true
  | _, _ => none

def or3 : Option Bool → Option Bool → Option Bool
  | some true, _ => some true
  | _, some true => some true
  | some false, some false => some false
  | _, _ => none

/-- Kleene's value with signs `s` (`0`: undecided). -/
def eval3 (s : Nat → Int) : BF → Option Bool
  | tt => some true
  | ff => some false
  | lit i pos => if s i = 0 then none else some (decide (s i > 0) == pos)
  | not a => (a.eval3 s).map (!·)
  | and a b => and3 (a.eval3 s) (b.eval3 s)
  | or a b => or3 (a.eval3 s) (b.eval3 s)

/-- `σ` completes the certified signs `s`. -/
def Agree (s : Nat → Int) (σ : Nat → Bool) : Prop := ∀ k, s k ≠ 0 → σ k = decide (s k > 0)

theorem and3_sound {x y : Option Bool} {a b c : Bool} (hx : ∀ v, x = some v → a = v)
    (hy : ∀ v, y = some v → b = v) (h : and3 x y = some c) : (a && b) = c := by
  unfold and3 at h
  split at h <;> simp at h <;> subst h
  · rw [hx false rfl]; simp
  · rw [hy false rfl]; simp
  · rw [hx true rfl, hy true rfl]; simp

theorem or3_sound {x y : Option Bool} {a b c : Bool} (hx : ∀ v, x = some v → a = v)
    (hy : ∀ v, y = some v → b = v) (h : or3 x y = some c) : (a || b) = c := by
  unfold or3 at h
  split at h <;> simp at h <;> subst h
  · rw [hx true rfl]; simp
  · rw [hy true rfl]; simp
  · rw [hx false rfl, hy false rfl]; simp

/-- **Kleene soundness.** -/
theorem eval3_sound {s : Nat → Int} {σ : Nat → Bool} (hs : Agree s σ) :
    ∀ (f : BF) (b : Bool), f.eval3 s = some b → f.eval2 σ = b
  | tt, b, h => by simp [eval3] at h; simp [eval2, h]
  | ff, b, h => by simp [eval3] at h; simp [eval2, h]
  | lit i pos, b, h => by
    simp only [eval3] at h
    split at h
    · simp at h
    · next h0 => simp at h; rw [← h]; simp only [eval2]; rw [hs i h0]
  | not a, b, h => by
    simp only [eval3] at h
    obtain ⟨v, hv, rfl⟩ := Option.map_eq_some_iff.mp h
    simp only [eval2]; rw [eval3_sound hs a v hv]
  | and a c, b, h => by
    simp only [eval2]
    exact and3_sound (fun v hv => eval3_sound hs a v hv) (fun v hv => eval3_sound hs c v hv) h
  | or a c, b, h => by
    simp only [eval2]
    exact or3_sound (fun v hv => eval3_sound hs a v hv) (fun v hv => eval3_sound hs c v hv) h

/-- `σ` with `k` set to `b`. -/
def upd (σ : Nat → Bool) (k : Nat) (b : Bool) : Nat → Bool := fun m => if m = k then b else σ m

/-- Some exclusion forbids `σ`. -/
def excluded (ex : List Excl) (σ : Nat → Bool) : Bool :=
  ex.any fun e => σ e.i == e.si && σ e.j == e.sj

/-- Every completion of `σ` over the indices `und`: excluded, or the formula false there. -/
def allFalse (f : BF) (ex : List Excl) : List Nat → (Nat → Bool) → Bool
  | [], σ => excluded ex σ || !f.eval2 σ
  | k :: rest, σ => allFalse f ex rest (upd σ k true) && allFalse f ex rest (upd σ k false)

/-- **The enumeration is sound.** -/
theorem allFalse_sound (f : BF) (ex : List Excl) :
    ∀ (und : List Nat) (σ0 σ : Nat → Bool), allFalse f ex und σ0 = true →
      (∀ m, m ∉ und → σ m = σ0 m) → excluded ex σ = false → f.eval2 σ = false
  | [], σ0, σ, h, hag, hex => by
    have e : σ = σ0 := funext fun m => hag m (by simp)
    subst e
    simp only [allFalse, Bool.or_eq_true, Bool.not_eq_true'] at h
    rcases h with h | h
    · rw [h] at hex; simp at hex
    · exact h
  | k :: rest, σ0, σ, h, hag, hex => by
    simp only [allFalse, Bool.and_eq_true] at h
    have hk : ∀ m, m ∉ rest → σ m = upd σ0 k (σ k) m := fun m hm => by
      simp only [upd]
      split
      · next e => rw [e]
      · next e => exact hag m (by simp [e, hm])
    cases hb : σ k
    · exact allFalse_sound f ex rest _ σ h.2 (by rw [hb] at hk; exact hk) hex
    · exact allFalse_sound f ex rest _ σ h.1 (by rw [hb] at hk; exact hk) hex

end BF

end ForgeExact.QRef
