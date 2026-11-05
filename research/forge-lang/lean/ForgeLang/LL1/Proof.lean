import ForgeLang.LL1.Core

/-!
# P6: a grammar the check accepts is unambiguous, and its LL(1) parser is exact

* `unambiguous`: two parse trees of the start symbol with the same leaves are the same tree.
* `parse_sound`: whatever the parser returns is a parse tree, and its leaves followed by the rest
  of the input are the input (lossless: nothing is dropped or invented).
* `parse_complete`: every parse tree is found, given its leaves (and a legal continuation).
* `roundtrip`: printing a tree (its leaves) and parsing gives the tree back.

All hold for any grammar and tables with `check G T = true`; nothing here mentions Forge.
-/

set_option linter.unusedSectionVars false

namespace ForgeLang.LL1

mutual
inductive Tree where
  | leaf : Tok → Tree
  | node : Nat → Forest → Tree
inductive Forest where
  | nil : Forest
  | cons : Tree → Forest → Forest
end

mutual
def Tree.yield : Tree → List Tok
  | .leaf a => [a]
  | .node _ f => f.yield
def Forest.yield : Forest → List Tok
  | .nil => []
  | .cons t f => t.yield ++ f.yield
end

mutual
def Tree.size : Tree → Nat
  | .leaf _ => 1
  | .node _ f => f.size + 1
def Forest.size : Forest → Nat
  | .nil => 1
  | .cons t f => max t.size f.size + 1
end

variable (G : Grammar) (T : Tables)

mutual
/-- `t` is a parse tree of `X`. -/
inductive VT : Tree → Sym → Prop
  | leaf (a : Tok) : VT (.leaf a) (.t a)
  | node {r A α f} : G.rules[r]? = some (A, α) → VF f α → VT (.node r f) (.n A)
inductive VF : Forest → List Sym → Prop
  | nil : VF .nil []
  | cons {t X f α} : VT t X → VF f α → VF (.cons t f) (X :: α)
end

variable {G T}

def la (s : List Tok) : Option Tok := s.head?

/-- What may follow `X`: anything for a terminal, FOLLOW for a nonterminal. -/
def FolOK (T : Tables) : Sym → List Tok → Prop
  | .t _, _ => True
  | .n A, s => la s ∈ T.fol A

theorem rule_mem {r A : Nat} {α : List Sym} (h : G.rules[r]? = some (A, α)) : (A, α) ∈ G.rules := List.mem_of_getElem? h

/-! ## Bool to Prop -/

theorem splits_mem : ∀ (pre : List Sym) (B : Nat) (post : List Sym), (pre, B, post) ∈ splits (pre ++ .n B :: post)
  | [], B, post => by simp [splits]
  | X :: pre, B, post => by
    have ih := splits_mem pre B post
    cases X with
    | t a => exact List.mem_map.2 ⟨_, ih, rfl⟩
    | n C => exact List.mem_cons_of_mem _ (List.mem_map.2 ⟨_, ih, rfl⟩)

theorem closed_of (h : closedB G T = true) : Closed G T := by
  unfold closedB at h
  simp only [Bool.and_eq_true, List.all_eq_true, decide_eq_true_eq] at h
  obtain ⟨hr, he⟩ := h
  refine ⟨?_, ?_, ?_, he⟩
  · intro A α hm hn
    have := (hr (A, α) hm).1.1
    simp only [Bool.or_eq_true, Bool.not_eq_true', decide_eq_true_eq] at this
    rcases this with h | h
    · rw [hn] at h; cases h
    · exact h
  · intro A α hm a ha
    exact (hr (A, α) hm).1.2 a ha
  · intro A α hm pre B post he
    have := (hr (A, α) hm).2 (pre, B, post) (he ▸ splits_mem pre B post)
    simp only [List.all_eq_true, decide_eq_true_eq, Bool.or_eq_true,
      Bool.not_eq_true'] at this
    refine ⟨this.1, fun hn o ho => ?_⟩
    rcases this.2 with h | h
    · rw [hn] at h; cases h
    · exact h o ho

theorem ll1_of (h : ll1B G T = true) : LL1 G T := by
  intro i j A α β hi hj hij o ho hb
  unfold ll1B at h
  simp only [List.all_eq_true, List.mem_range] at h
  have hi' : i < G.rules.length := by
    rcases Nat.lt_or_ge i G.rules.length with h | h
    · exact h
    · simp [List.getElem?_eq_none h] at hi
  have hj' : j < G.rules.length := by
    rcases Nat.lt_or_ge j G.rules.length with h | h
    · exact h
    · simp [List.getElem?_eq_none h] at hj
  have := h i hi' j hj'
  simp only [hi, hj, Bool.or_eq_true, beq_iff_eq, hij, false_or, bne_self_eq_false,
    disjoint, List.all_eq_true, Bool.not_eq_true', decide_eq_false_iff_not] at this
  exact (this.resolve_left (by simp)) o ho hb

theorem check_sound (h : check G T = true) : Closed G T ∧ LL1 G T := by
  unfold check at h
  simp only [Bool.and_eq_true] at h
  exact ⟨closed_of h.1, ll1_of h.2⟩

/-! ## Empty and first leaves -/

section
variable (hC : Closed G T)
include hC

mutual
theorem vt_null : ∀ {t X}, VT G t X → t.yield = [] → T.nullSym X = true
  | _, _, .leaf a, h => by simp [Tree.yield] at h
  | _, _, .node (A := A) (α := α) hr hf, h => by
    have := vf_null hf (by simpa [Tree.yield] using h)
    simp only [Tables.nullSym, decide_eq_true_eq]
    exact hC.nul A α (rule_mem hr) this
theorem vf_null : ∀ {f α}, VF G f α → f.yield = [] → T.nullSeq α = true
  | _, _, .nil, _ => rfl
  | _, _, .cons ht hf, h => by
    simp only [Forest.yield, List.append_eq_nil_iff] at h
    have h2 := vf_null hf h.2
    simp only [Tables.nullSeq] at h2 ⊢
    simp [vt_null ht h.1, h2]
end

mutual
theorem vt_first : ∀ {t X a w}, VT G t X → t.yield = a :: w → a ∈ T.firstSym X
  | _, _, _, _, .leaf b, h => by simp [Tree.yield] at h; simp [Tables.firstSym, h.1]
  | _, _, _, _, .node (A := A) (α := α) hr hf, h => by
    simp only [Tables.firstSym]
    exact hC.fst A α (rule_mem hr) _ (vf_first hf (by simpa [Tree.yield] using h))
theorem vf_first : ∀ {f α a w}, VF G f α → f.yield = a :: w → a ∈ T.firstSeq α
  | _, _, _, _, .nil, h => by simp [Forest.yield] at h
  | _, _, a, w, .cons (t := t) ht hf, h => by
    simp only [Forest.yield] at h
    simp only [Tables.firstSeq, List.mem_append]
    cases hy : t.yield with
    | nil =>
      rw [hy, List.nil_append] at h
      right; simp [vt_null hC ht hy, vf_first hf h]
    | cons b w' =>
      rw [hy, List.cons_append] at h
      simp only [List.cons.injEq] at h
      left; exact h.1 ▸ vt_first ht hy
end

theorem vf_la {f : Forest} {β : List Sym} (hf : VF G f β) (s : List Tok) :
    (∃ a, la (f.yield ++ s) = some a ∧ a ∈ T.firstSeq β) ∨ (T.nullSeq β = true ∧ la (f.yield ++ s) = la s) := by
  cases hy : f.yield with
  | nil => right; exact ⟨vf_null hC hf hy, by simp⟩
  | cons a w => left; exact ⟨a, by simp [la], vf_first hC hf hy⟩

/-- Inside `A := pre X post`, what follows `X` is legal for `X`. -/
theorem fol_ctx {A : Nat} {pre : List Sym} {X : Sym} {post : List Sym} {g : Forest} {s : List Tok} (hm : (A, pre ++ X :: post) ∈ G.rules) (hg : VF G g post)
    (hs : la s ∈ T.fol A) : FolOK T X (g.yield ++ s) := by
  cases X with
  | t a => trivial
  | n B =>
    have hc := hC.fol A _ hm pre B post rfl
    rcases vf_la hC hg s with ⟨a, ha, hf⟩ | ⟨hn, he⟩
    · show la _ ∈ _; rw [ha]; exact hc.1 a hf
    · show la _ ∈ _; rw [he]; exact hc.2 hn _ hs

theorem predict_mem {r A : Nat} {α : List Sym} {f : Forest} {s : List Tok} (_hr : G.rules[r]? = some (A, α)) (hf : VF G f α) (hs : la s ∈ T.fol A) :
    la (f.yield ++ s) ∈ T.predict A α := by
  simp only [Tables.predict, List.mem_append, List.mem_map]
  rcases vf_la hC hf s with ⟨a, ha, hfa⟩ | ⟨hn, he⟩
  · left; exact ⟨a, hfa, ha.symm⟩
  · right; simp [hn, he, hs]
end

/-! ## Unambiguity -/

section
variable (hC : Closed G T) (hL : LL1 G T)
include hC hL

theorem same_rule {r1 r2 A : Nat} {α1 α2 : List Sym} {o : Option Tok} (h1 : G.rules[r1]? = some (A, α1)) (h2 : G.rules[r2]? = some (A, α2))
    (o1 : o ∈ T.predict A α1) (o2 : o ∈ T.predict A α2) : r1 = r2 := by
  by_cases hne : r1 = r2
  · exact hne
  · exact absurd o2 (hL r1 r2 A α1 α2 h1 h2 hne o o1)

mutual
theorem det_t : ∀ {t1 X t2 s1 s2}, VT G t1 X → VT G t2 X → t1.yield ++ s1 = t2.yield ++ s2 →
    FolOK T X s1 → FolOK T X s2 → t1 = t2 ∧ s1 = s2
  | _, _, _, _, _, .leaf a, h2, he, _, _ => by
    cases h2; simp [Tree.yield] at he; exact ⟨rfl, he⟩
  | _, _, _, s1, s2, .node (r := r1) (A := A) (α := α1) (f := f1) hr1 hf1, h2, he, k1, k2 => by
    cases h2 with
    | node hr2 hf2 =>
      rename_i r2 α2 f2
      simp only [Tree.yield] at he
      have p1 := predict_mem hC hr1 hf1 k1
      have p2 := predict_mem hC hr2 hf2 k2
      rw [he] at p1
      have := same_rule hC hL hr1 hr2 p1 p2
      subst this
      rw [hr1] at hr2
      cases hr2
      obtain ⟨rfl, rfl⟩ := det_f hf1 hf2 (pre := []) (by simpa using rule_mem hr1) he k1 k2
      exact ⟨rfl, rfl⟩
theorem det_f : ∀ {f1 β f2 s1 s2 A pre}, VF G f1 β → VF G f2 β → (A, pre ++ β) ∈ G.rules →
    f1.yield ++ s1 = f2.yield ++ s2 → la s1 ∈ T.fol A → la s2 ∈ T.fol A → f1 = f2 ∧ s1 = s2
  | _, _, _, _, _, _, _, .nil, h2, _, he, _, _ => by
    cases h2; simp [Forest.yield] at he; exact ⟨rfl, he⟩
  | _, _, _, s1, s2, A, pre, .cons (t := t1) (X := X) (f := g1) (α := β) ht1 hg1, h2, hm, he, k1, k2 => by
    cases h2 with
    | cons ht2 hg2 =>
      rename_i t2 g2
      simp only [Forest.yield, List.append_assoc] at he
      obtain ⟨rfl, he'⟩ := det_t ht1 ht2 he (fol_ctx hC hm hg1 k1) (fol_ctx hC hm hg2 k2)
      obtain ⟨rfl, rfl⟩ := det_f hg1 hg2 (pre := pre ++ [X]) (by simpa using hm) he' k1 k2
      exact ⟨rfl, rfl⟩
end

end

/-- P6: a grammar the check accepts is unambiguous. -/
theorem unambiguous (hc : check G T = true) {t1 t2 : Tree} (h1 : VT G t1 (.n G.start))
    (h2 : VT G t2 (.n G.start)) (he : t1.yield = t2.yield) : t1 = t2 := by
  obtain ⟨hC, hL⟩ := check_sound hc
  have e : FolOK T (.n G.start) [] := hC.eof
  exact (det_t hC hL h1 h2 (by simpa using he) e e).1

/-! ## The parser -/

def choose (G : Grammar) (T : Tables) (A : Nat) (o : Option Tok) : Option Nat :=
  (List.range G.rules.length).find? fun i =>
    match G.rules[i]? with
    | some (B, β) => B == A && decide (o ∈ T.predict B β)
    | none => false

mutual
def parse (G : Grammar) (T : Tables) : Nat → Sym → List Tok → Option (Tree × List Tok)
  | 0, _, _ => none
  | _ + 1, .t a, w =>
    match w with
    | b :: w' => if a = b then some (.leaf a, w') else none
    | [] => none
  | n + 1, .n A, w =>
    match choose G T A (la w) with
    | some r =>
      match G.rules[r]? with
      | some (_, α) =>
        match parseF G T n α w with
        | some (f, w') => some (.node r f, w')
        | none => none
      | none => none
    | none => none
def parseF (G : Grammar) (T : Tables) : Nat → List Sym → List Tok → Option (Forest × List Tok)
  | 0, _, _ => none
  | _ + 1, [], w => some (.nil, w)
  | n + 1, X :: α, w =>
    match parse G T n X w with
    | some (t, w1) =>
      match parseF G T n α w1 with
      | some (f, w2) => some (.cons t f, w2)
      | none => none
    | none => none
end

theorem choose_rule {A o r} (h : choose G T A o = some r) : ∃ α, G.rules[r]? = some (A, α) := by
  unfold choose at h
  have := List.find?_some h
  split at this
  · rename_i B β hr
    simp only [Bool.and_eq_true, beq_iff_eq] at this
    exact ⟨β, this.1 ▸ hr⟩
  · cases this

theorem find_unique {p : Nat → Bool} : ∀ {l : List Nat} {r : Nat}, r ∈ l → p r = true →
    (∀ x ∈ l, p x = true → x = r) → l.find? p = some r
  | [], _, h, _, _ => by cases h
  | x :: l, r, hm, hp, hu => by
    by_cases hx : p x = true
    · simp only [List.find?, hx]; rw [hu x List.mem_cons_self hx]
    · simp only [List.find?, Bool.not_eq_true] at hx ⊢
      rw [hx]
      rcases List.mem_cons.1 hm with rfl | hm
      · rw [hp] at hx; cases hx
      · exact find_unique hm hp fun y hy hpy => hu y (List.mem_cons_of_mem _ hy) hpy

mutual
/-- Sound and lossless: a tree, and its leaves then the rest are the input. -/
theorem parse_sound : ∀ {n X w t w'}, parse G T n X w = some (t, w') → VT G t X ∧ t.yield ++ w' = w
  | 0, _, _, _, _, h => by simp [parse] at h
  | n + 1, .t a, w, t, w', h => by
    cases w with
    | nil => simp [parse] at h
    | cons b w0 =>
      simp only [parse] at h
      split at h
      · rename_i hab; cases h; subst hab; exact ⟨.leaf a, by simp [Tree.yield]⟩
      · cases h
  | n + 1, .n A, w, t, w', h => by
    simp only [parse] at h
    split at h
    · rename_i r hr
      split at h
      · rename_i B α hα
        split at h
        · rename_i f w1 hf
          cases h
          obtain ⟨β, hβ⟩ := choose_rule hr
          rw [hβ] at hα; cases hα
          obtain ⟨v, e⟩ := parseF_sound hf
          exact ⟨.node hβ v, by simpa [Tree.yield] using e⟩
        · cases h
      · cases h
    · cases h
theorem parseF_sound : ∀ {n α w f w'}, parseF G T n α w = some (f, w') → VF G f α ∧ f.yield ++ w' = w
  | 0, _, _, _, _, h => by simp [parseF] at h
  | n + 1, [], w, f, w', h => by simp [parseF] at h; obtain ⟨rfl, rfl⟩ := h; exact ⟨.nil, rfl⟩
  | n + 1, X :: α, w, f, w', h => by
    simp only [parseF] at h
    split at h
    · rename_i t w1 ht
      split at h
      · rename_i g w2 hg
        cases h
        obtain ⟨vt, e1⟩ := parse_sound ht
        obtain ⟨vg, e2⟩ := parseF_sound hg
        exact ⟨.cons vt vg, by simp [Forest.yield, e2, e1]⟩
      · cases h
    · cases h
end

section
variable (hC : Closed G T) (hL : LL1 G T)
include hC hL

theorem choose_eq {r A : Nat} {α : List Sym} {f : Forest} {s : List Tok} (hr : G.rules[r]? = some (A, α)) (hf : VF G f α) (hs : la s ∈ T.fol A) :
    choose G T A (la (f.yield ++ s)) = some r := by
  have hp := predict_mem hC hr hf hs
  have hlt : r < G.rules.length := by
    rcases Nat.lt_or_ge r G.rules.length with h | h
    · exact h
    · simp [List.getElem?_eq_none h] at hr
  unfold choose
  apply find_unique (by simp [hlt]) (by simp [hr, hp])
  intro x _ hx
  split at hx
  · rename_i B β hb
    simp only [Bool.and_eq_true, beq_iff_eq, decide_eq_true_eq] at hx
    obtain ⟨rfl, hx⟩ := hx
    exact same_rule hC hL hb hr hx hp
  · cases hx

mutual
/-- Complete: every tree is found from its leaves, given enough fuel and a legal continuation. -/
theorem parse_complete : ∀ {t X n s}, VT G t X → t.size ≤ n → FolOK T X s →
    parse G T n X (t.yield ++ s) = some (t, s)
  | _, _, n, s, .leaf a, hn, _ => by
    obtain ⟨k, rfl⟩ : ∃ k, n = k + 1 := ⟨n - 1, by simp [Tree.size] at hn; omega⟩
    simp [parse, Tree.yield]
  | _, _, n, s, .node (r := r) (A := A) (α := α) (f := f) hr hf, hn, hs => by
    obtain ⟨k, rfl⟩ : ∃ k, n = k + 1 := ⟨n - 1, by simp [Tree.size] at hn; omega⟩
    have hk : f.size ≤ k := by simp [Tree.size] at hn; omega
    simp only [parse, Tree.yield, choose_eq hC hL hr hf hs, hr,
      parseF_complete hf (pre := []) (by simpa using rule_mem hr) hk hs]
theorem parseF_complete : ∀ {f α n s A pre}, VF G f α → (A, pre ++ α) ∈ G.rules → f.size ≤ n →
    la s ∈ T.fol A → parseF G T n α (f.yield ++ s) = some (f, s)
  | _, _, n, s, _, _, .nil, _, hn, _ => by
    obtain ⟨k, rfl⟩ : ∃ k, n = k + 1 := ⟨n - 1, by simp [Forest.size] at hn; omega⟩
    simp [parseF, Forest.yield]
  | _, _, n, s, A, pre, .cons (t := t) (X := X) (f := g) (α := β) ht hg, hm, hn, hs => by
    obtain ⟨k, rfl⟩ : ∃ k, n = k + 1 := ⟨n - 1, by simp [Forest.size] at hn; omega⟩
    have h1 : t.size ≤ k := by simp [Forest.size] at hn; omega
    have h2 : g.size ≤ k := by simp [Forest.size] at hn; omega
    simp only [parseF, Forest.yield, List.append_assoc,
      parse_complete ht h1 (fol_ctx hC hm hg hs),
      parseF_complete hg (pre := pre ++ [X]) (by simpa using hm) h2 hs]
end

end

/-- Print (the leaves) then parse gives the tree back. -/
theorem roundtrip (hc : check G T = true) {t : Tree} (h : VT G t (.n G.start)) :
    parse G T t.size (.n G.start) t.yield = some (t, []) := by
  obtain ⟨hC, hL⟩ := check_sound hc
  simpa using parse_complete hC hL (s := []) h (Nat.le_refl _) hC.eof

/-- Lossless: a parse of the whole input has exactly the input as its leaves. -/
theorem lossless {n w t} (h : parse G T n (.n G.start) w = some (t, [])) : VT G t (.n G.start) ∧ t.yield = w := by
  simpa using parse_sound h

/-- The derived parser reads the whole input. -/
def parsesAll (G : Grammar) (T : Tables) (n : Nat) (w : List Tok) : Bool :=
  match parse G T n (.n G.start) w with
  | some (_, []) => true
  | _ => false

/-- An accepted input has a parse tree with exactly its tokens as leaves (and, under `check`, only
that one: `unambiguous`). -/
theorem parsesAll_tree {G : Grammar} {T : Tables} {n : Nat} {w : List Tok} (h : parsesAll G T n w = true) :
    ∃ t, VT G t (.n G.start) ∧ t.yield = w := by
  unfold parsesAll at h
  split at h
  · rename_i t hp; exact ⟨t, lossless hp⟩
  · cases h

end ForgeLang.LL1
