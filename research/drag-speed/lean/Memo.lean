/-!
# A memo changes no answer

The model of `ssi::searched` and `swept::tool_of`. `compute` is the work, on inputs `I`; `key` is what the memo looks up by.
The one assumption about the Rust: the work's answer is a function of the key (`Covers`). See README for what backs it.
-/
namespace Memo

variable {I K V : Type}

/-- The key holds every input the work reads: equal keys, equal answers. -/
def Covers (compute : I → V) (key : I → K) : Prop :=
  ∀ x y, key x = key y → compute x = compute y

/-- Every entry was computed for its key. -/
def Sound (compute : I → V) (key : I → K) (c : List (K × V)) : Prop :=
  ∀ e ∈ c, ∃ x, key x = e.1 ∧ compute x = e.2

def find [DecidableEq K] (c : List (K × V)) (k : K) : Option V :=
  (c.find? (fun e => e.1 == k)).map (·.2)

theorem find_mem [DecidableEq K] {c : List (K × V)} {k : K} {v : V} (h : find c k = some v) : (k, v) ∈ c := by
  simp only [find, Option.map_eq_some_iff] at h
  obtain ⟨e, he, rfl⟩ := h
  have hm := List.mem_of_find?_eq_some he
  have hk := List.find?_some he
  simp at hk
  cases e
  simp_all

/-- A hit is the answer: to a key equal in full, whatever the hash (the map compares keys, a hash only picks where to look). -/
theorem hit_is_answer [DecidableEq K] {compute : I → V} {key : I → K} {c : List (K × V)} (hc : Covers compute key)
    (hs : Sound compute key c) (x : I) {v : V} (h : find c (key x) = some v) : v = compute x := by
  obtain ⟨y, hy, hv⟩ := hs _ (find_mem h)
  simp at hy hv
  rw [← hv]
  exact hc y x hy

/-- One asking: the entry if there is one, else the work, remembered. -/
def ask [DecidableEq K] (compute : I → V) (key : I → K) (c : List (K × V)) (x : I) : V × List (K × V) :=
  match find c (key x) with
  | some v => (v, c)
  | none => (compute x, (key x, compute x) :: c)

theorem ask_answer [DecidableEq K] {compute : I → V} {key : I → K} {c : List (K × V)} (hc : Covers compute key)
    (hs : Sound compute key c) (x : I) : (ask compute key c x).1 = compute x := by
  unfold ask
  split
  · rename_i v h
    exact hit_is_answer hc hs x h
  · rfl

theorem ask_sound [DecidableEq K] {compute : I → V} {key : I → K} {c : List (K × V)}
    (hs : Sound compute key c) (x : I) : Sound compute key (ask compute key c x).2 := by
  unfold ask
  split
  · exact hs
  · intro e he
    simp only [List.mem_cons] at he
    cases he with
    | inl h => exact ⟨x, by simp [h], by simp [h]⟩
    | inr h => exact hs e h

/-- Forgetting any entries, eviction or a clear, keeps what is left sound. -/
theorem forget_sound {compute : I → V} {key : I → K} {c d : List (K × V)}
    (hs : Sound compute key c) (h : ∀ e ∈ d, e ∈ c) : Sound compute key d :=
  fun e he => hs e (h e he)

/-- Work done on a form of its input (the box on the grid): a key of that form covers it. -/
theorem covers_of_form {F : Type} (form : I → F) (work : F → V) (key : F → K)
    (inj : ∀ a b, key a = key b → a = b) :
    Covers (fun x => work (form x)) (fun x => key (form x)) :=
  fun x y h => by simp only [inj _ _ h]

/-- The assumption cannot go: with an input left out of the key, a hit is wrong. -/
theorem covers_is_needed :
    ∃ (compute : Nat → Nat) (key : Nat → Nat) (c : List (Nat × Nat)) (x : Nat) (v : Nat),
      Sound compute key c ∧ find c (key x) = some v ∧ v ≠ compute x := by
  refine ⟨fun n => n, fun _ => 0, [(0, 1)], 2, 1, ?_, ?_, ?_⟩
  · intro e he
    simp at he
    subst he
    exact ⟨1, rfl, rfl⟩
  · decide
  · decide

end Memo
