import ForgeExact.DecideOnce.Refines

/-!
# A8 L2: the simulation theorem (`core_refines`)

Stage by stage, for `Refines A B σ τ ν κ`:
1. the split checks agree;
2. memberships and keeps agree facet-side by facet-side, so the classify checks agree and the
   results correspond (`σ φ ∈ B.result ↔ φ ∈ A.result`);
3. the touch rule: bad edges and bad verts correspond;
4. unify: faces, edges and verts correspond.
-/

set_option linter.unusedSectionVars false

namespace ForgeExact.DecideOnce

universe u v

namespace IncLedger

variable {α : Type u} [DecidableEq α] [BEq α] [LawfulBEq α] (L : IncLedger α)

theorem plus_mem_rooms {φ : α} (h : φ ∈ L.facets) : L.plus φ ∈ L.rooms :=
  (mem_dedup _ _).mpr (List.mem_flatMap.mpr ⟨φ, h, by simp⟩)

theorem minus_mem_rooms {φ : α} (h : φ ∈ L.facets) : L.minus φ ∈ L.rooms :=
  (mem_dedup _ _).mpr (List.mem_flatMap.mpr ⟨φ, h, by simp⟩)

theorem find?_pair' {γ : Type u} {δ : Type v} [BEq γ] [LawfulBEq γ] (f : γ → δ) (k : γ) :
    ∀ (xs : List γ), k ∈ xs → ((xs.map fun x => (x, f x)).find? fun e => e.1 == k) = some (k, f k)
  | [], h => nomatch h
  | x :: xs, h => by
    simp only [List.map_cons, List.find?_cons]
    by_cases e : x = k
    · subst e; simp
    · have : (x == k) = false := by simp [e]
      simp only [this]
      exact find?_pair' f k xs ((List.mem_cons.mp h).resolve_left (Ne.symm e))

theorem keepIn_tab (rule : Rule) {T : α} (hT : T ∈ L.rooms) : keepInI (L.keepTab rule) T = L.keep rule T := by
  simp only [keepInI, keepTab]
  rw [find?_pair' (L.keep rule) T _ hT]; rfl

theorem isPieceOf_iff (i : Nat) (φ : α) : L.isPieceOf i φ = true ↔ ∃ f, (φ, f) ∈ L.pieces ∧ L.bodyOf f = i := by
  simp only [isPieceOf, List.any_eq_true, Bool.and_eq_true, beq_iff_eq]
  constructor
  · rintro ⟨⟨a, f⟩, hp, rfl, hb⟩; exact ⟨f, hp, hb⟩
  · rintro ⟨f, hp, hb⟩; exact ⟨(φ, f), hp, rfl, hb⟩

theorem mem_result {kt : KeepTabI α} {φ : α} :
    φ ∈ L.result kt ↔ φ ∈ L.facets ∧ keepInI kt (L.plus φ) ≠ keepInI kt (L.minus φ) := by
  simp [result, List.mem_filter]

end IncLedger

/-! ## Chains under an onto map whose fibres are connected (face-arrangement.md §2.2a, `cls_map`) -/

section ConnMap

variable {γ : Type u} {δ : Type v} [DecidableEq γ] [DecidableEq δ]

theorem conn_map {f : γ → δ} {r : γ → γ → Bool} {r' : δ → δ → Bool} {xs : List γ} {ys : List δ}
    (hmem : ∀ a ∈ xs, f a ∈ ys)
    (h1 : ∀ a ∈ xs, ∀ b ∈ xs, r a b = true → f a = f b ∨ r' (f a) (f b) = true)
    (h2 : ∀ c ∈ ys, ∀ d ∈ ys, r' c d = true → ∃ a ∈ xs, ∃ b ∈ xs, f a = c ∧ f b = d ∧ r a b = true)
    (h3 : ∀ a ∈ xs, ∀ b ∈ xs, f a = f b → Conn r xs a b)
    {a b : γ} (ha : a ∈ xs) (hb : b ∈ xs) : Conn r xs a b ↔ Conn r' ys (f a) (f b) := by
  constructor
  · intro hc
    induction hc with
    | refl => exact Conn.refl
    | @step b c hab hc hbc ih =>
      have hb' := Conn.mem ha hab
      rcases h1 b hb' c hc hbc with e | e
      · rw [← e]; exact ih hb'
      · exact Conn.step (ih hb') (hmem c hc) e
  · intro hc
    suffices H : ∀ d, Conn r' ys (f a) d → ∀ b ∈ xs, f b = d → Conn r xs a b from H _ hc b hb rfl
    intro d hd
    induction hd with
    | refl => intro b hb e; exact h3 a ha b hb e.symm
    | @step d d' hdd hd' hr ih =>
      intro b hb e
      have hdm := Conn.mem (hmem a ha) hdd
      obtain ⟨a₁, ha₁, a₂, ha₂, e₁, e₂, hr'⟩ := h2 d hdm d' hd' hr
      exact ((ih a₁ ha₁ e₁).step ha₂ hr').trans (h3 a₂ ha₂ b hb (e₂.trans e.symm))

/-- A chain whose steps keep a property stays inside any list holding every member with it. -/
theorem conn_restrict {r : γ → γ → Bool} {xs ys : List γ} {P : γ → Prop} {a b : γ}
    (hab : Conn r xs a b) (ha : P a) (hstep : ∀ x y, P x → y ∈ xs → r x y = true → P y)
    (hys : ∀ y ∈ xs, P y → y ∈ ys) : Conn r ys a b := by
  have : P b ∧ Conn r ys a b := by
    induction hab with
    | refl => exact ⟨ha, Conn.refl⟩
    | step _ hc hbc ih =>
      have hP := hstep _ _ ih.1 hc hbc
      exact ⟨hP, Conn.step ih.2 (hys _ hc hP) hbc⟩
  exact this.2

theorem filterMap_inj {γ : Type u} {δ : Type v} {f : γ → Option δ} :
    ∀ {l : List γ}, (l.filterMap f).Nodup → ∀ {a b x}, a ∈ l → b ∈ l → f a = some x → f b = some x → a = b
  | [], _, _, _, _, ha, _, _, _ => nomatch ha
  | c :: l, hnd, a, b, x, ha, hb, hfa, hfb => by
    by_cases hc : f c = none
    · simp only [List.filterMap_cons, hc] at hnd
      rcases List.mem_cons.mp ha with rfl | ha
      · rw [hc] at hfa; cases hfa
      rcases List.mem_cons.mp hb with rfl | hb
      · rw [hc] at hfb; cases hfb
      exact filterMap_inj hnd ha hb hfa hfb
    · obtain ⟨y, hy⟩ := Option.ne_none_iff_exists'.mp hc
      simp only [List.filterMap_cons, hy, List.nodup_cons, List.mem_filterMap] at hnd
      rcases List.mem_cons.mp ha with ea | ha' <;> rcases List.mem_cons.mp hb with eb | hb'
      · rw [ea, eb]
      · subst ea; rw [hy] at hfa; cases hfa; exact absurd ⟨b, hb', hfb⟩ hnd.1
      · subst eb; rw [hy] at hfb; cases hfb; exact absurd ⟨a, ha', hfa⟩ hnd.1
      · exact filterMap_inj hnd.2 ha' hb' hfa hfb

/-- A chain whose steps keep a property, re-read with another relation that agrees on such pairs. -/
theorem conn_restrict2 {r r' : γ → γ → Bool} {xs ys : List γ} {P : γ → Prop} {a b : γ}
    (hab : Conn r xs a b) (ha : P a) (hstep : ∀ x y, P x → y ∈ xs → r x y = true → P y)
    (hys : ∀ y ∈ xs, P y → y ∈ ys) (hrel : ∀ x y, P x → P y → r x y = true → r' x y = true) :
    Conn r' ys a b := by
  have : P b ∧ Conn r' ys a b := by
    induction hab with
    | refl => exact ⟨ha, Conn.refl⟩
    | step _ hc hbc ih =>
      have hP := hstep _ _ ih.1 hc hbc
      exact ⟨hP, Conn.step ih.2 (hys _ hc hP) (hrel _ _ ih.1 hP hbc)⟩
  exact this.2

theorem nodup_map_of_inj {G : γ → δ} : ∀ {l : List γ}, l.Nodup → (∀ x ∈ l, ∀ y ∈ l, G x = G y → x = y) →
    (l.map G).Nodup
  | [], _, _ => List.nodup_nil
  | x :: xs, hl, hinj => by
    rw [List.nodup_cons] at hl
    rw [List.map_cons, List.nodup_cons]
    refine ⟨fun hm => ?_, nodup_map_of_inj hl.2 fun a ha b hb => hinj a (List.mem_cons_of_mem _ ha) b (List.mem_cons_of_mem _ hb)⟩
    obtain ⟨y, hy, e⟩ := List.mem_map.mp hm
    have := hinj x List.mem_cons_self y (List.mem_cons_of_mem _ hy) e.symm
    exact hl.1 (this ▸ hy)

/-- Counting distinct values through a map injective on them. -/
theorem dedup_length_map {G : γ → δ} {l₁ : List γ} {l₂ : List δ}
    (hinj : ∀ x ∈ l₁, ∀ y ∈ l₁, G x = G y → x = y) (hmem : ∀ y, y ∈ l₂ ↔ ∃ x ∈ l₁, G x = y) :
    (dedup l₂).length = (dedup l₁).length := by
  have hnd : ((dedup l₁).map G).Nodup :=
    nodup_map_of_inj (nodup_dedup l₁) fun x hx y hy => hinj x ((mem_dedup _ _).mp hx) y ((mem_dedup _ _).mp hy)
  have hp : (dedup l₂).Perm ((dedup l₁).map G) := by
    apply (List.perm_ext_iff_of_nodup (nodup_dedup l₂) hnd).mpr
    intro y
    rw [mem_dedup, hmem, List.mem_map]
    simp only [mem_dedup]
  rw [hp.length_eq, List.length_map]

end ConnMap

namespace IncLedger

variable {α : Type u} [DecidableEq α] [BEq α] [LawfulBEq α]

theorem fanRel_symm : ∀ s t : Sector α, fanRel s t = fanRel t s := by
  intro s t
  rw [Bool.eq_iff_iff]
  simp only [fanRel, List.any_eq_true, List.contains_iff_mem, decide_eq_true_eq]
  exact ⟨fun ⟨e, h1, h2⟩ => ⟨e, h2, h1⟩, fun ⟨e, h1, h2⟩ => ⟨e, h2, h1⟩⟩

/-- A bad vert: two result sectors round it in different fans. -/
theorem badVert_iff (L : IncLedger α) (kt : KeepTabI α) (v : α) (hnd : (L.aroundV kt v).Nodup) :
    L.badVert kt v = true ↔
    ∃ c ∈ L.aroundV kt v, ∃ d ∈ L.aroundV kt v, ¬Conn fanRel (L.aroundV kt v) c d := by
  unfold badVert
  generalize hX : L.aroundV kt v = X at hnd ⊢
  cases X with
  | nil => simp
  | cons c₀ t =>
    have hc₀ : c₀ ∈ c₀ :: t := List.mem_cons_self
    have key : ∀ x, (!(cls fanRel (c₀ :: t) c₀).contains x) = true ↔ ¬Conn fanRel (c₀ :: t) c₀ x := by
      intro x
      rw [Bool.not_eq_true', Bool.eq_false_iff, ne_eq, List.contains_iff_mem, cls_spec fanRel hnd hc₀]
    simp only [List.any_eq_true, key]
    constructor
    · rintro ⟨d, hd, hn⟩
      exact ⟨c₀, hc₀, d, hd, hn⟩
    · rintro ⟨c, hc, d, hd, hn⟩
      by_cases Hc : Conn fanRel (c₀ :: t) c₀ c
      · by_cases Hd : Conn fanRel (c₀ :: t) c₀ d
        · exact absurd ((Conn.symm fanRel_symm hc₀ Hc).trans Hd) hn
        · exact ⟨d, hd, Hd⟩
      · exact ⟨c, hc, Hc⟩

/-! ### Faces of the result (IncLedger) -/

theorem joinRel_symm (L : IncLedger α) (kt : KeepTabI α) : ∀ x y, L.joinRel kt x y = L.joinRel kt y x := by
  intro x y
  rw [Bool.eq_iff_iff]
  simp only [joinRel, Bool.and_eq_true, beq_iff_eq, List.any_eq_true]
  exact ⟨fun ⟨⟨a, b⟩, e, he, c, d⟩ => ⟨⟨a.symm, b.symm⟩, e, he, d, c⟩,
    fun ⟨⟨a, b⟩, e, he, c, d⟩ => ⟨⟨a.symm, b.symm⟩, e, he, d, c⟩⟩

theorem result_nodup (L : IncLedger α) (kt : KeepTabI α) (h : L.facets.Nodup) : (L.result kt).Nodup :=
  h.filter _

theorem mem_heads (L : IncLedger α) {kt : KeepTabI α} {h : α} :
    h ∈ L.heads kt ↔ h ∈ L.result kt ∧ head (L.joinRel kt) (L.result kt) h = some h := by
  simp [heads]

theorem mem_faceOf (L : IncLedger α) {kt : KeepTabI α} (hnd : L.facets.Nodup) {h c : α} (hh : h ∈ L.result kt) :
    c ∈ L.faceOf kt h ↔ c ∈ L.result kt ∧ Conn (L.joinRel kt) (L.result kt) h c := by
  simp only [faceOf, List.mem_filter, List.contains_iff_mem]
  rw [cls_spec _ (L.result_nodup kt hnd) hh]

theorem in_some_face (L : IncLedger α) {kt : KeepTabI α} (hnd : L.facets.Nodup) {c : α} (hc : c ∈ L.result kt) :
    ∃ h ∈ L.heads kt, c ∈ L.faceOf kt h := by
  obtain ⟨h, hhd, hm, hconn⟩ := head_some (L.joinRel kt) (L.result_nodup kt hnd) hc
  have hh : head (L.joinRel kt) (L.result kt) h = some h := by
    rw [← head_eq (L.joinRel kt) (L.joinRel_symm kt) (L.result_nodup kt hnd) hc hconn]; exact hhd
  refine ⟨h, (L.mem_heads).mpr ⟨hm, hh⟩, (L.mem_faceOf hnd hm).mpr ⟨hc, ?_⟩⟩
  exact Conn.symm (L.joinRel_symm kt) hc hconn

/-- Carrier and sense are constant along `joinRel`. -/
theorem joinRel_carrier (L : IncLedger α) {kt : KeepTabI α} {x y : α} (h : L.joinRel kt x y = true) :
    L.carrier x = L.carrier y ∧ L.orient kt x = L.orient kt y := by
  simp only [joinRel, Bool.and_eq_true, beq_iff_eq] at h
  exact ⟨h.1.1, h.1.2⟩

theorem conn_carrier (L : IncLedger α) {kt : KeepTabI α} {xs : List α} {x y : α}
    (h : Conn (L.joinRel kt) xs x y) : L.carrier x = L.carrier y ∧ L.orient kt x = L.orient kt y := by
  induction h with
  | refl => exact ⟨rfl, rfl⟩
  | step _ _ hr ih => exact ⟨ih.1.trans (L.joinRel_carrier hr).1, ih.2.trans (L.joinRel_carrier hr).2⟩

/-- The face key unify uses: the head of a result facet's face. -/
def faceKey (L : IncLedger α) (kt : KeepTabI α) (c : α) : α :=
  let fk : List (α × α) := (L.heads kt).flatMap fun h => (L.faceOf kt h).map fun c => (c, h)
  ((fk.find? fun e => e.1 == c).map Prod.snd).getD c

theorem faceKey_spec (L : IncLedger α) {kt : KeepTabI α} (hnd : L.facets.Nodup) {c : α} (hc : c ∈ L.result kt) :
    L.faceKey kt c ∈ L.heads kt ∧ c ∈ L.faceOf kt (L.faceKey kt c) := by
  obtain ⟨h, hh, hcf⟩ := L.in_some_face hnd hc
  simp only [faceKey]
  have hin : (c, h) ∈ (L.heads kt).flatMap fun h => (L.faceOf kt h).map fun c => (c, h) :=
    List.mem_flatMap.mpr ⟨h, hh, List.mem_map.mpr ⟨c, hcf, rfl⟩⟩
  cases hf : ((L.heads kt).flatMap fun h => (L.faceOf kt h).map fun c => (c, h)).find? (fun e => e.1 == c) with
  | none => exact absurd hin (by intro hm; have := List.find?_eq_none.mp hf _ hm; simp at this)
  | some p =>
    have hp := List.mem_of_find?_eq_some hf
    have hq := List.find?_some hf
    simp only [beq_iff_eq] at hq
    obtain ⟨h', hh', hm⟩ := List.mem_flatMap.mp hp
    obtain ⟨c', hc', e⟩ := List.mem_map.mp hm
    subst e
    simp only [Option.map_some, Option.getD_some]
    simp only at hq
    exact ⟨hh', hq ▸ hc'⟩

/-- Two result facets have one key iff they are joined. -/
theorem faceKey_eq_iff (L : IncLedger α) {kt : KeepTabI α} (hnd : L.facets.Nodup) {c d : α}
    (hc : c ∈ L.result kt) (hd : d ∈ L.result kt) :
    L.faceKey kt c = L.faceKey kt d ↔ Conn (L.joinRel kt) (L.result kt) c d := by
  have rn := L.result_nodup kt hnd
  have sy := L.joinRel_symm kt
  obtain ⟨hk1, hf1⟩ := L.faceKey_spec hnd hc
  obtain ⟨hk2, hf2⟩ := L.faceKey_spec hnd hd
  have m1 := (L.mem_heads).mp hk1
  have m2 := (L.mem_heads).mp hk2
  have c1 := ((L.mem_faceOf hnd m1.1).mp hf1).2
  have c2 := ((L.mem_faceOf hnd m2.1).mp hf2).2
  constructor
  · intro e
    rw [e] at c1
    exact (Conn.symm sy m2.1 c1).trans c2
  · intro hcd
    have h12 : Conn (L.joinRel kt) (L.result kt) (L.faceKey kt c) (L.faceKey kt d) :=
      (c1.trans hcd).trans (Conn.symm sy m2.1 c2)
    have := head_eq (L.joinRel kt) sy rn m1.1 h12
    rw [m1.2, m2.2] at this
    exact Option.some.inj this

theorem faceKey_result (L : IncLedger α) {kt : KeepTabI α} (hnd : L.facets.Nodup) {c : α} (hc : c ∈ L.result kt) :
    L.faceKey kt c ∈ L.result kt :=
  ((L.mem_heads).mp (L.faceKey_spec hnd hc).1).1

theorem conn_faceKey (L : IncLedger α) {kt : KeepTabI α} (hnd : L.facets.Nodup) {c : α} (hc : c ∈ L.result kt) :
    Conn (L.joinRel kt) (L.result kt) c (L.faceKey kt c) := by
  obtain ⟨hk, hf⟩ := L.faceKey_spec hnd hc
  have m := (L.mem_heads).mp hk
  exact Conn.symm (L.joinRel_symm kt) m.1 ((L.mem_faceOf hnd m.1).mp hf).2

end IncLedger

namespace Refines

variable {α : Type u} {β : Type v} [DecidableEq α] [BEq α] [LawfulBEq α] [DecidableEq β] [BEq β] [LawfulBEq β]
  {A : IncLedger α} {B : IncLedger β} {σ : α → β} {τ ν : α → Option β} {κ : α → Sector α → Sector β}

theorem bodyOf_eq (h : Refines A B σ τ ν κ) (f : Nat) : B.bodyOf f = A.bodyOf f := by
  simp [IncLedger.bodyOf, h.faceBody]

theorem clsOf_eq (h : Refines A B σ τ ν κ) (f : Nat) : B.clsOf f = A.clsOf f := by
  simp [IncLedger.clsOf, h.faceCls]

theorem isPieceOf_eq (h : Refines A B σ τ ν κ) {φ : α} (hφ : φ ∈ A.facets) (i : Nat) :
    B.isPieceOf i (σ φ) = A.isPieceOf i φ := by
  rw [Bool.eq_iff_iff, IncLedger.isPieceOf_iff, IncLedger.isPieceOf_iff]
  simp only [bodyOf_eq h]
  exact ⟨fun ⟨f, hp, hb⟩ => ⟨f, (h.pieces φ hφ f).mpr hp, hb⟩, fun ⟨f, hp, hb⟩ => ⟨f, (h.pieces φ hφ f).mp hp, hb⟩⟩

/-- 1. The coarse split is sound when the fine one is. -/
theorem splitOk (h : Refines A B σ τ ν κ) (hs : A.splitOk = true) : B.splitOk = true := by
  simp only [IncLedger.splitOk, List.all_eq_true, Bool.and_eq_true] at hs ⊢
  intro p hp
  obtain ⟨φ, hφ, hσ⟩ := h.pieces_from p hp
  have hpA : (φ, p.2) ∈ A.pieces := (h.pieces φ hφ p.2).mpr (by rw [hσ]; exact hp)
  obtain ⟨⟨⟨⟨_, hc⟩, hb⟩, _⟩, hq⟩ := hs _ hpA
  have hψ : p.1 ∈ B.facets := hσ ▸ h.facet_mem φ hφ
  refine ⟨⟨⟨⟨by simpa using hψ, ?_⟩, ?_⟩, by simpa using h.sidesB _ hψ⟩, fun q hq' => ?_⟩
  · rw [← hσ, h.carrier φ hφ, clsOf_eq h]; exact hc
  · rw [bodyOf_eq h, h.nb]; exact hb
  · by_cases e1 : q.1 = p.1
    · by_cases e2 : B.bodyOf q.2 = B.bodyOf p.2
      · have hqA : (φ, q.2) ∈ A.pieces := (h.pieces φ hφ q.2).mpr (by rw [hσ, ← e1]; exact hq')
        have := hq _ hqA
        simp only [bodyOf_eq h] at e2
        simp only [e2, beq_self_eq_true, Bool.and_true, Bool.not_true, Bool.false_or] at this
        simp [this]
      · simp [e2]
    · simp [e1]

/-! ### 2. Membership, keep, result -/

theorem keep_plus (h : Refines A B σ τ ν κ) (rule : Rule) {φ : α} (hφ : φ ∈ A.facets) :
    B.keep rule (B.plus (σ φ)) = A.keep rule (A.plus φ) := by
  simp only [IncLedger.keep, h.nb]
  congr 1; funext i
  by_cases hi : i < A.nb
  · simp [hi, h.memb_plus φ hφ i hi]
  · simp [hi]

theorem keep_minus (h : Refines A B σ τ ν κ) (rule : Rule) {φ : α} (hφ : φ ∈ A.facets) :
    B.keep rule (B.minus (σ φ)) = A.keep rule (A.minus φ) := by
  simp only [IncLedger.keep, h.nb]
  congr 1; funext i
  by_cases hi : i < A.nb
  · simp [hi, h.memb_minus φ hφ i hi]
  · simp [hi]

theorem keepIn_plus (h : Refines A B σ τ ν κ) (rule : Rule) {φ : α} (hφ : φ ∈ A.facets) :
    keepInI (B.keepTab rule) (B.plus (σ φ)) = keepInI (A.keepTab rule) (A.plus φ) := by
  rw [B.keepIn_tab rule (B.plus_mem_rooms (h.facet_mem φ hφ)), A.keepIn_tab rule (A.plus_mem_rooms hφ),
    keep_plus h rule hφ]

theorem keepIn_minus (h : Refines A B σ τ ν κ) (rule : Rule) {φ : α} (hφ : φ ∈ A.facets) :
    keepInI (B.keepTab rule) (B.minus (σ φ)) = keepInI (A.keepTab rule) (A.minus φ) := by
  rw [B.keepIn_tab rule (B.minus_mem_rooms (h.facet_mem φ hφ)), A.keepIn_tab rule (A.minus_mem_rooms hφ),
    keep_minus h rule hφ]

/-- A fine facet is in the result iff its image is. -/
theorem result_iff (h : Refines A B σ τ ν κ) (rule : Rule) {φ : α} (hφ : φ ∈ A.facets) :
    σ φ ∈ B.result (B.keepTab rule) ↔ φ ∈ A.result (A.keepTab rule) := by
  rw [IncLedger.mem_result, IncLedger.mem_result, keepIn_plus h rule hφ, keepIn_minus h rule hφ]
  exact ⟨fun ⟨_, h2⟩ => ⟨hφ, h2⟩, fun ⟨_, h2⟩ => ⟨h.facet_mem φ hφ, h2⟩⟩

/-- Every coarse result facet is the image of a fine one. -/
theorem result_from (h : Refines A B σ τ ν κ) (rule : Rule) {ψ : β} (hψ : ψ ∈ B.result (B.keepTab rule)) :
    ∃ φ ∈ A.result (A.keepTab rule), σ φ = ψ := by
  obtain ⟨φ, hφ, rfl⟩ := h.facet_onto ψ (B.mem_result.mp hψ).1
  exact ⟨φ, (result_iff h rule hφ).mp hψ, rfl⟩

theorem orient_eq (h : Refines A B σ τ ν κ) (rule : Rule) {φ : α} (hφ : φ ∈ A.facets) :
    B.orient (B.keepTab rule) (σ φ) = A.orient (A.keepTab rule) φ := by
  simp only [IncLedger.orient, keepIn_minus h rule hφ]

/-- The classify checks agree. -/
theorem classifyOk_eq (h : Refines A B σ τ ν κ) : B.classifyOk = A.classifyOk := by
  have hφ : ∀ φ ∈ A.facets, ((List.range B.nb).all fun i =>
      !B.isPieceOf i (σ φ) || (B.memb i (B.plus (σ φ)) != B.memb i (B.minus (σ φ)))) =
      ((List.range A.nb).all fun i => !A.isPieceOf i φ || (A.memb i (A.plus φ) != A.memb i (A.minus φ))) := by
    intro φ hφ
    have hi : ∀ i, i < A.nb → (!B.isPieceOf i (σ φ) || (B.memb i (B.plus (σ φ)) != B.memb i (B.minus (σ φ)))) =
        (!A.isPieceOf i φ || (A.memb i (A.plus φ) != A.memb i (A.minus φ))) := fun i hi => by
      rw [isPieceOf_eq h hφ, h.memb_plus φ hφ i hi, h.memb_minus φ hφ i hi]
    rw [h.nb, Bool.eq_iff_iff, List.all_eq_true, List.all_eq_true]
    exact ⟨fun H i hr => by rw [← hi i (List.mem_range.mp hr)]; exact H i hr,
      fun H i hr => by rw [hi i (List.mem_range.mp hr)]; exact H i hr⟩
  rw [Bool.eq_iff_iff]
  unfold IncLedger.classifyOk
  rw [List.all_eq_true, List.all_eq_true]
  constructor
  · intro hB φ hφ'; rw [← hφ φ hφ']; exact hB _ (h.facet_mem φ hφ')
  · intro hA ψ hψ
    obtain ⟨φ, hφ', rfl⟩ := h.facet_onto ψ hψ
    rw [hφ φ hφ']; exact hA φ hφ'

/-! ### 3. The touch rule at edges -/

theorem aroundE_perm (h : Refines A B σ τ ν κ) (rule : Rule) {e : α} (he : e ∈ A.edges) {E : β}
    (hτ : τ e = some E) :
    ((A.aroundE (A.keepTab rule) e).map σ).Perm (B.aroundE (B.keepTab rule) E) := by
  have hw := (h.edge_wings e he E hτ).filter fun ψ => (B.result (B.keepTab rule)).contains ψ
  simp only [IncLedger.aroundE]
  refine List.Perm.trans ?_ hw
  rw [List.filter_map]
  apply List.Perm.of_eq
  congr 1
  apply List.filter_congr
  intro φ hφ
  have hf := h.edge_facets_A e he φ hφ
  simp only [Function.comp_def]
  rw [Bool.eq_iff_iff]
  simp only [List.contains_iff_mem]
  exact (result_iff h rule hf).symm

theorem badEdge_some (h : Refines A B σ τ ν κ) (rule : Rule) {e : α} (he : e ∈ A.edges) {E : β}
    (hτ : τ e = some E) : B.badEdge (B.keepTab rule) E = A.badEdge (A.keepTab rule) e := by
  simp only [IncLedger.badEdge, ← (aroundE_perm h rule he hτ).length_eq, List.length_map]

theorem badEdge_none (h : Refines A B σ τ ν κ) (rule : Rule) {e : α} (he : e ∈ A.edges)
    (hτ : τ e = none) : A.badEdge (A.keepTab rule) e = false := by
  simp only [IncLedger.badEdge, IncLedger.aroundE]
  apply decide_eq_false
  apply Nat.not_lt.mpr
  exact Nat.le_trans (List.length_filter_le _ _) (h.edge_inner e he hτ).1

/-- A coarse edge is bad iff some fine edge is. -/
theorem bad_edge_iff (h : Refines A B σ τ ν κ) (rule : Rule) :
    (∃ E ∈ B.edges, B.badEdge (B.keepTab rule) E = true) ↔
    (∃ e ∈ A.edges, A.badEdge (A.keepTab rule) e = true) := by
  constructor
  · rintro ⟨E, hE, hb⟩
    obtain ⟨e, he, hτ⟩ := h.edge_onto E hE
    exact ⟨e, he, by rw [← badEdge_some h rule he hτ]; exact hb⟩
  · rintro ⟨e, he, hb⟩
    cases hτ : τ e with
    | none => rw [badEdge_none h rule he hτ] at hb; cases hb
    | some E => exact ⟨E, h.edge_mem e he E hτ, by rw [badEdge_some h rule he hτ]; exact hb⟩

/-! ### 3'. The touch rule at verts -/

theorem shareNone_fan {s t : Sector α}
    (h : (s.bounds.any fun e => t.bounds.contains e && τ e == none) = true) : IncLedger.fanRel s t = true := by
  simp only [List.any_eq_true, Bool.and_eq_true, List.contains_iff_mem] at h
  obtain ⟨e, h1, h2, _⟩ := h
  simp only [IncLedger.fanRel, List.any_eq_true, List.contains_iff_mem]
  exact ⟨e, h1, by simpa using h2⟩

theorem mem_aroundV {L : IncLedger α} {kt : KeepTabI α} {v : α} {s : Sector α} :
    s ∈ L.aroundV kt v ↔ s ∈ L.vertSectors v ∧ s.facet ∈ L.result kt := by
  simp [IncLedger.aroundV, List.mem_filter]

theorem mem_aroundV' {L : IncLedger β} {kt : KeepTabI β} {v : β} {s : Sector β} :
    s ∈ L.aroundV kt v ↔ s ∈ L.vertSectors v ∧ s.facet ∈ L.result kt := by
  simp [IncLedger.aroundV, List.mem_filter]

theorem badVert_some (h : Refines A B σ τ ν κ) (rule : Rule) {w : α} (hw : w ∈ A.verts) {V : β}
    (hν : ν w = some V) : B.badVert (B.keepTab rule) V = A.badVert (A.keepTab rule) w := by
  have hV := h.vert_mem w hw V hν
  let XA := A.aroundV (A.keepTab rule) w
  let XB := B.aroundV (B.keepTab rule) V
  have hres : ∀ s ∈ A.vertSectors w, (σ s.facet ∈ B.result (B.keepTab rule) ↔ s.facet ∈ A.result (A.keepTab rule)) :=
    fun s hs => result_iff h rule (h.sector_facets_A w hw s hs)
  have hmem : ∀ s ∈ XA, κ w s ∈ XB := by
    intro s hs
    obtain ⟨hs1, hs2⟩ := mem_aroundV.mp hs
    exact mem_aroundV'.mpr ⟨h.sector_mem w hw V hν s hs1, by rw [h.sector_facet w hw s hs1]; exact (hres s hs1).mpr hs2⟩
  have hfrom : ∀ S ∈ XB, ∃ s ∈ XA, κ w s = S := by
    intro S hS
    obtain ⟨hS1, hS2⟩ := mem_aroundV'.mp hS
    obtain ⟨s, hs, rfl⟩ := h.sector_onto w hw V hν S hS1
    refine ⟨s, mem_aroundV.mpr ⟨hs, (hres s hs).mp ?_⟩, rfl⟩
    rw [← h.sector_facet w hw s hs]; exact hS2
  have hfib : ∀ s ∈ XA, ∀ t ∈ XA, κ w s = κ w t → Conn IncLedger.fanRel XA s t := by
    intro s hs t ht e
    have hs1 := (mem_aroundV.mp hs).1
    have hc := h.sector_fibre w hw s hs1 t (mem_aroundV.mp ht).1 e
    have hc' := conn_restrict (ys := XA) (P := fun x => x ∈ A.vertSectors w ∧ κ w x = κ w s) hc ⟨hs1, rfl⟩
      (fun x y ⟨hx, hxe⟩ hy hr => by
        refine ⟨hy, ?_⟩
        simp only [List.any_eq_true, Bool.and_eq_true, List.contains_iff_mem, beq_iff_eq] at hr
        obtain ⟨e, he1, he2, he3⟩ := hr
        rw [← hxe]; exact (h.sector_inner w hw x hx y hy e he1 he2 he3).symm)
      (fun y _ ⟨hy, hye⟩ => mem_aroundV.mpr ⟨hy, by
        rw [← hres y hy, ← h.sector_facet w hw y hy, hye, h.sector_facet w hw s hs1]
        exact (hres s hs1).mpr (mem_aroundV.mp hs).2⟩)
    exact Conn.congr_rel (fun x y _ hr => shareNone_fan hr) hc'
  have h1 : ∀ s ∈ XA, ∀ t ∈ XA, IncLedger.fanRel s t = true →
      κ w s = κ w t ∨ IncLedger.fanRel (κ w s) (κ w t) = true := by
    intro s hs t ht hr
    have hs1 := (mem_aroundV.mp hs).1
    have ht1 := (mem_aroundV.mp ht).1
    simp only [IncLedger.fanRel, List.any_eq_true, List.contains_iff_mem] at hr ⊢
    obtain ⟨e, he1, he2⟩ := hr
    cases hτ : τ e with
    | none => exact Or.inl (h.sector_inner w hw s hs1 t ht1 e he1 (by simpa using he2) hτ)
    | some E => exact Or.inr ⟨E, h.sector_bounds w hw s hs1 e he1 E hτ, by
        simpa using h.sector_bounds w hw t ht1 e (by simpa using he2) E hτ⟩
  have h2 : ∀ S ∈ XB, ∀ T ∈ XB, IncLedger.fanRel S T = true →
      ∃ s ∈ XA, ∃ t ∈ XA, κ w s = S ∧ κ w t = T ∧ IncLedger.fanRel s t = true := by
    intro S hS T hT hr
    simp only [IncLedger.fanRel, List.any_eq_true, List.contains_iff_mem] at hr
    obtain ⟨E, hE1, hE2⟩ := hr
    obtain ⟨s₀, hs₀, rfl⟩ := h.sector_onto w hw V hν S (mem_aroundV'.mp hS).1
    obtain ⟨t₀, ht₀, rfl⟩ := h.sector_onto w hw V hν T (mem_aroundV'.mp hT).1
    obtain ⟨s, hs, es, e, he, hτe⟩ := h.sector_bounds_from w hw s₀ hs₀ E hE1
    obtain ⟨t, ht, et, e', he', hτe'⟩ := h.sector_bounds_from w hw t₀ ht₀ E (by simpa using hE2)
    have hee : e = e' := filterMap_inj ((h.vert_edges w hw V hν).nodup_iff.mpr (h.vertEdgesB_nodup V hV))
      (h.sector_bounds_sub w hw s hs e he) (h.sector_bounds_sub w hw t ht e' he') hτe hτe'
    subst hee
    have hinA : ∀ x ∈ A.vertSectors w, κ w x ∈ XB → x ∈ XA := fun x hx hxB =>
      mem_aroundV.mpr ⟨hx, (hres x hx).mp (by rw [← h.sector_facet w hw x hx]; exact (mem_aroundV'.mp hxB).2)⟩
    refine ⟨s, hinA s hs (es ▸ hS), t, hinA t ht (et ▸ hT), es, et, ?_⟩
    simp only [IncLedger.fanRel, List.any_eq_true, List.contains_iff_mem]
    exact ⟨e, he, by simpa using he'⟩
  have hnA : XA.Nodup := (h.sectorsA_nodup w hw).filter _
  have hnB : XB.Nodup := (h.sectorsB_nodup V hV).filter _
  have hcm : ∀ s ∈ XA, ∀ t ∈ XA, Conn IncLedger.fanRel XA s t ↔ Conn IncLedger.fanRel XB (κ w s) (κ w t) :=
    fun s hs t ht => conn_map hmem h1 h2 hfib hs ht
  rw [Bool.eq_iff_iff, IncLedger.badVert_iff B _ V hnB, IncLedger.badVert_iff A _ w hnA]
  constructor
  · rintro ⟨C, hC, D, hD, hn⟩
    obtain ⟨c, hc, rfl⟩ := hfrom C hC
    obtain ⟨d, hd, rfl⟩ := hfrom D hD
    exact ⟨c, hc, d, hd, fun hcd => hn ((hcm c hc d hd).mp hcd)⟩
  · rintro ⟨c, hc, d, hd, hn⟩
    exact ⟨κ w c, hmem c hc, κ w d, hmem d hd, fun hcd => hn ((hcm c hc d hd).mpr hcd)⟩

theorem badVert_none (h : Refines A B σ τ ν κ) (rule : Rule) {w : α} (hw : w ∈ A.verts)
    (hν : ν w = none) : A.badVert (A.keepTab rule) w = false := by
  let XA := A.aroundV (A.keepTab rule) w
  have hres : ∀ s ∈ A.vertSectors w, (σ s.facet ∈ B.result (B.keepTab rule) ↔ s.facet ∈ A.result (A.keepTab rule)) :=
    fun s hs => result_iff h rule (h.sector_facets_A w hw s hs)
  -- sectors of one coarse facet are joined within the result sectors
  have hsame : ∀ s ∈ XA, ∀ t ∈ A.vertSectors w, σ t.facet = σ s.facet → Conn IncLedger.fanRel XA s t := by
    intro s hs t ht e
    have hs1 := (mem_aroundV.mp hs).1
    have hc := h.inner_same w hw hν s hs1 t ht e.symm
    have hc' := conn_restrict (ys := XA) (P := fun x => x ∈ A.vertSectors w ∧ σ x.facet = σ s.facet) hc ⟨hs1, rfl⟩
      (fun x y ⟨hx, hxe⟩ hy hr => by
        refine ⟨hy, ?_⟩
        simp only [List.any_eq_true, Bool.and_eq_true, List.contains_iff_mem, beq_iff_eq] at hr
        obtain ⟨e, he1, he2, he3⟩ := hr
        rw [← hxe]; exact (h.inner_edge_facet w hw x hx y hy e he1 he2 he3).symm)
      (fun y _ ⟨hy, hye⟩ => mem_aroundV.mpr ⟨hy, (hres y hy).mp (by
        rw [hye]; exact (hres s hs1).mpr (mem_aroundV.mp hs).2)⟩)
    exact Conn.congr_rel (fun x y _ hr => shareNone_fan hr) hc'
  have hnA : XA.Nodup := (h.sectorsA_nodup w hw).filter _
  rw [Bool.eq_false_iff, ne_eq, IncLedger.badVert_iff A _ w hnA]
  rintro ⟨c, hc, d, hd, hn⟩
  apply hn
  have hc1 := (mem_aroundV.mp hc).1
  have hd1 := (mem_aroundV.mp hd).1
  obtain ⟨s', hs', t', ht', es, et, e, he1, he2⟩ := h.inner_meet w hw hν c hc1 d hd1
  have hs'X : s' ∈ XA := mem_aroundV.mpr ⟨hs', (hres s' hs').mp (by rw [es]; exact (hres c hc1).mpr (mem_aroundV.mp hc).2)⟩
  have ht'X : t' ∈ XA := mem_aroundV.mpr ⟨ht', (hres t' ht').mp (by rw [et]; exact (hres d hd1).mpr (mem_aroundV.mp hd).2)⟩
  have h1 := hsame c hc s' hs' es
  have h2 : Conn IncLedger.fanRel XA s' t' := Conn.step Conn.refl ht'X (by
    simp only [IncLedger.fanRel, List.any_eq_true, List.contains_iff_mem]; exact ⟨e, he1, by simpa using he2⟩)
  have h3 := Conn.symm IncLedger.fanRel_symm hd (hsame d hd t' ht' et)
  exact (h1.trans h2).trans h3

/-- A coarse vert is bad iff some fine vert is. -/
theorem bad_vert_iff (h : Refines A B σ τ ν κ) (rule : Rule) :
    (∃ V ∈ B.verts, B.badVert (B.keepTab rule) V = true) ↔
    (∃ w ∈ A.verts, A.badVert (A.keepTab rule) w = true) := by
  constructor
  · rintro ⟨V, hV, hb⟩
    obtain ⟨w, hw, hν⟩ := h.vert_onto V hV
    exact ⟨w, hw, by rw [← badVert_some h rule hw hν]; exact hb⟩
  · rintro ⟨w, hw, hb⟩
    cases hν : ν w with
    | none => rw [badVert_none h rule hw hν] at hb; cases hb
    | some V => exact ⟨V, h.vert_mem w hw V hν, by rw [badVert_some h rule hw hν]; exact hb⟩

/-! ### 4. Unify: faces -/

theorem join_conn (h : Refines A B σ τ ν κ) (rule : Rule) {c d : α}
    (hc : c ∈ A.result (A.keepTab rule)) (hd : d ∈ A.result (A.keepTab rule)) :
    Conn (A.joinRel (A.keepTab rule)) (A.result (A.keepTab rule)) c d ↔
    Conn (B.joinRel (B.keepTab rule)) (B.result (B.keepTab rule)) (σ c) (σ d) := by
  have hfA : ∀ x ∈ A.result (A.keepTab rule), x ∈ A.facets := fun x hx => (A.mem_result.mp hx).1
  refine conn_map (fun x hx => (result_iff h rule (hfA x hx)).mpr hx) ?_ ?_ ?_ hc hd
  · intro x hx y hy hr
    have hxf := hfA x hx
    have hyf := hfA y hy
    obtain ⟨hcar, hor⟩ := A.joinRel_carrier hr
    simp only [IncLedger.joinRel, Bool.and_eq_true, List.any_eq_true, List.contains_iff_mem] at hr
    obtain ⟨_, e, he, hxe, hye⟩ := hr
    cases hτ : τ e with
    | none => exact Or.inl ((h.edge_inner e he hτ).2 x hxe y hye)
    | some E =>
      right
      have hw := h.edge_wings e he E hτ
      simp only [IncLedger.joinRel, Bool.and_eq_true, beq_iff_eq, List.any_eq_true, List.contains_iff_mem]
      refine ⟨⟨by rw [h.carrier x hxf, h.carrier y hyf, hcar], by rw [orient_eq h rule hxf, orient_eq h rule hyf, hor]⟩,
        E, h.edge_mem e he E hτ, hw.mem_iff.mp (List.mem_map_of_mem hxe), hw.mem_iff.mp (List.mem_map_of_mem hye)⟩
  · intro C hC D hD hr
    obtain ⟨hcar, hor⟩ := B.joinRel_carrier hr
    simp only [IncLedger.joinRel, Bool.and_eq_true, List.any_eq_true, List.contains_iff_mem] at hr
    obtain ⟨_, E, hE, hCE, hDE⟩ := hr
    obtain ⟨e, he, hτ⟩ := h.edge_onto E hE
    have hw := h.edge_wings e he E hτ
    obtain ⟨c, hce, rfl⟩ := List.mem_map.mp (hw.mem_iff.mpr hCE)
    obtain ⟨d, hde, rfl⟩ := List.mem_map.mp (hw.mem_iff.mpr hDE)
    have hcf := h.edge_facets_A e he c hce
    have hdf := h.edge_facets_A e he d hde
    refine ⟨c, (result_iff h rule hcf).mp hC, d, (result_iff h rule hdf).mp hD, rfl, rfl, ?_⟩
    simp only [IncLedger.joinRel, Bool.and_eq_true, beq_iff_eq, List.any_eq_true, List.contains_iff_mem]
    exact ⟨⟨by rw [← h.carrier c hcf, ← h.carrier d hdf, hcar], by rw [← orient_eq h rule hcf, ← orient_eq h rule hdf, hor]⟩,
      e, he, hce, hde⟩
  · intro x hx y hy e
    have hxf := hfA x hx
    have hc := h.facet_fibre x hxf y (hfA y hy) e
    refine conn_restrict2 (P := fun z => z ∈ A.facets ∧ σ z = σ x) hc ⟨hxf, rfl⟩ ?_ ?_ ?_
    · intro z w ⟨hz, hze⟩ hw hr
      refine ⟨hw, ?_⟩
      simp only [List.any_eq_true, Bool.and_eq_true, beq_iff_eq, List.contains_iff_mem] at hr
      obtain ⟨g, hg, ⟨hτg, hzg⟩, hwg⟩ := hr
      rw [← hze]; exact ((h.edge_inner g hg hτg).2 z hzg w hwg).symm
    · intro z _ ⟨hz, hze⟩
      exact (result_iff h rule hz).mp (by rw [hze]; exact (result_iff h rule hxf).mpr hx)
    · intro z w ⟨hz, hze⟩ ⟨hw, hwe⟩ hr
      simp only [List.any_eq_true, Bool.and_eq_true, beq_iff_eq, List.contains_iff_mem] at hr
      obtain ⟨g, hg, ⟨_, hzg⟩, hwg⟩ := hr
      simp only [IncLedger.joinRel, Bool.and_eq_true, beq_iff_eq, List.any_eq_true, List.contains_iff_mem]
      refine ⟨⟨?_, ?_⟩, g, hg, hzg, hwg⟩
      · rw [← h.carrier z hz, ← h.carrier w hw, hze, hwe]
      · rw [← orient_eq h rule hz, ← orient_eq h rule hw, hze, hwe]

/-- The coarse face of `σ c` corresponds to the fine face of `c`. -/
theorem face_corr (h : Refines A B σ τ ν κ) (rule : Rule) {c h₀ : α} {h₁ : β}
    (hc : c ∈ A.result (A.keepTab rule)) (h₀m : h₀ ∈ A.heads (A.keepTab rule)) (hc₀ : c ∈ A.faceOf (A.keepTab rule) h₀)
    (h₁m : h₁ ∈ B.heads (B.keepTab rule)) (hc₁ : σ c ∈ B.faceOf (B.keepTab rule) h₁) :
    FaceCorr σ ⟨B.faceOf (B.keepTab rule) h₁, B.carrier h₁, B.orient (B.keepTab rule) h₁⟩
      ⟨A.faceOf (A.keepTab rule) h₀, A.carrier h₀, A.orient (A.keepTab rule) h₀⟩ := by
  have hA0 := ((A.mem_heads).mp h₀m).1
  have hB1 := ((B.mem_heads).mp h₁m).1
  have cA := ((A.mem_faceOf h.nodupA hA0).mp hc₀).2
  have cB := ((B.mem_faceOf h.nodupB hB1).mp hc₁).2
  have hcf := (A.mem_result.mp hc).1
  refine ⟨?_, ?_, fun ψ => ⟨fun hψ => ?_, fun ⟨φ, hφ, e⟩ => ?_⟩⟩
  · simp only
    rw [(B.conn_carrier cB).1, h.carrier c hcf, ← (A.conn_carrier cA).1]
  · simp only
    rw [(B.conn_carrier cB).2, orient_eq h rule hcf, ← (A.conn_carrier cA).2]
  · obtain ⟨hψr, hψc⟩ := (B.mem_faceOf h.nodupB hB1).mp hψ
    obtain ⟨φ, hφ, rfl⟩ := result_from h rule hψr
    refine ⟨φ, (A.mem_faceOf h.nodupA hA0).mpr ⟨hφ, cA.trans ?_⟩, rfl⟩
    exact (join_conn h rule hc hφ).mpr ((Conn.symm (B.joinRel_symm _) hB1 cB).trans hψc)
  · subst e
    obtain ⟨hφr, hφc⟩ := (A.mem_faceOf h.nodupA hA0).mp hφ
    refine (B.mem_faceOf h.nodupB hB1).mpr ⟨(result_iff h rule (A.mem_result.mp hφr).1).mpr hφr, cB.trans ?_⟩
    exact (join_conn h rule hc hφr).mp ((Conn.symm (A.joinRel_symm _) hA0 cA).trans hφc)

/-! ### 4'. Unify: face keys -/

theorem key_iff (h : Refines A B σ τ ν κ) (rule : Rule) {c d : α}
    (hc : c ∈ A.result (A.keepTab rule)) (hd : d ∈ A.result (A.keepTab rule)) :
    B.faceKey (B.keepTab rule) (σ c) = B.faceKey (B.keepTab rule) (σ d) ↔
    A.faceKey (A.keepTab rule) c = A.faceKey (A.keepTab rule) d := by
  have hcf := (A.mem_result.mp hc).1
  have hdf := (A.mem_result.mp hd).1
  rw [B.faceKey_eq_iff h.nodupB ((result_iff h rule hcf).mpr hc) ((result_iff h rule hdf).mpr hd),
    A.faceKey_eq_iff h.nodupA hc hd, join_conn h rule hc hd]

theorem key_key (h : Refines A B σ τ ν κ) (rule : Rule) {c : α} (hc : c ∈ A.result (A.keepTab rule)) :
    B.faceKey (B.keepTab rule) (σ (A.faceKey (A.keepTab rule) c)) = B.faceKey (B.keepTab rule) (σ c) := by
  have hk := A.faceKey_result h.nodupA hc
  rw [key_iff h rule hk hc]
  exact (A.faceKey_eq_iff h.nodupA hk hc).mpr (Conn.symm (A.joinRel_symm _) hc (A.conn_faceKey h.nodupA hc))

/-- Distinct fine faces among result facets `xs`, and distinct coarse faces among their images. -/
theorem faces_count (h : Refines A B σ τ ν κ) (rule : Rule) {xs : List α} {ys : List β}
    (hxs : ∀ x ∈ xs, x ∈ A.result (A.keepTab rule)) (hys : ∀ y, y ∈ ys ↔ ∃ x ∈ xs, σ x = y) :
    (dedup (ys.map (B.faceKey (B.keepTab rule)))).length = (dedup (xs.map (A.faceKey (A.keepTab rule)))).length := by
  apply dedup_length_map (G := fun k => B.faceKey (B.keepTab rule) (σ k))
  · intro x hx y hy e
    obtain ⟨c, hc, rfl⟩ := List.mem_map.mp hx
    obtain ⟨d, hd, rfl⟩ := List.mem_map.mp hy
    simp only [key_key h rule (hxs c hc), key_key h rule (hxs d hd)] at e
    exact (key_iff h rule (hxs c hc) (hxs d hd)).mp e
  · intro k
    simp only [List.mem_map]
    constructor
    · rintro ⟨y, hy, rfl⟩
      obtain ⟨x, hx, rfl⟩ := (hys y).mp hy
      exact ⟨A.faceKey (A.keepTab rule) x, ⟨x, hx, rfl⟩, key_key h rule (hxs x hx)⟩
    · rintro ⟨_, ⟨x, hx, rfl⟩, rfl⟩
      exact ⟨σ x, (hys _).mpr ⟨x, hx, rfl⟩, (key_key h rule (hxs x hx)).symm⟩

theorem atE_some (h : Refines A B σ τ ν κ) (rule : Rule) {e : α} (he : e ∈ A.edges) {E : β} (hτ : τ e = some E) :
    (B.facesAtE (B.keepTab rule) (B.faceKey (B.keepTab rule)) E).length =
    (A.facesAtE (A.keepTab rule) (A.faceKey (A.keepTab rule)) e).length := by
  simp only [IncLedger.facesAtE]
  apply faces_count h rule
  · intro x hx; exact (List.mem_filter.mp hx).2 |> fun h' => by simpa using h'
  · intro y; rw [← (aroundE_perm h rule he hτ).mem_iff, List.mem_map]

theorem atE_none (h : Refines A B σ τ ν κ) (rule : Rule) {e : α} (he : e ∈ A.edges) (hτ : τ e = none) :
    (A.facesAtE (A.keepTab rule) (A.faceKey (A.keepTab rule)) e).length ≤ 1 := by
  simp only [IncLedger.facesAtE]
  match hl : A.aroundE (A.keepTab rule) e with
  | [] => simp [dedup, dedupAux]
  | c :: t =>
    have hres : ∀ x ∈ c :: t, x ∈ A.result (A.keepTab rule) := fun x hx => by
      rw [← hl] at hx; simpa using (List.mem_filter.mp hx).2
    have hfac : ∀ x ∈ c :: t, x ∈ A.edgeFacets e := fun x hx => by
      rw [← hl] at hx; exact (List.mem_filter.mp hx).1
    have hk : ∀ x ∈ c :: t, A.faceKey (A.keepTab rule) x = A.faceKey (A.keepTab rule) c := fun x hx =>
      (key_iff h rule (hres x hx) (hres c List.mem_cons_self)).mp
        (by rw [(h.edge_inner e he hτ).2 x (hfac x hx) c (hfac c List.mem_cons_self)])
    have hsub : dedup ((c :: t).map (A.faceKey (A.keepTab rule))) ⊆ [A.faceKey (A.keepTab rule) c] := by
      intro y hy
      obtain ⟨x, hx, rfl⟩ := List.mem_map.mp ((mem_dedup _ _).mp hy)
      rw [hk x hx]; exact List.mem_singleton_self _
    exact (nodup_dedup _).length_le_of_subset hsub

theorem atV_some (h : Refines A B σ τ ν κ) (rule : Rule) {w : α} (hw : w ∈ A.verts) {V : β} (hν : ν w = some V) :
    (B.facesAtV (B.keepTab rule) (B.faceKey (B.keepTab rule)) V).length =
    (A.facesAtV (A.keepTab rule) (A.faceKey (A.keepTab rule)) w).length := by
  simp only [IncLedger.facesAtV]
  have e1 : (B.aroundV (B.keepTab rule) V).map (fun s => B.faceKey (B.keepTab rule) s.facet) =
      ((B.aroundV (B.keepTab rule) V).map Sector.facet).map (B.faceKey (B.keepTab rule)) := by
    rw [List.map_map]; rfl
  have e2 : (A.aroundV (A.keepTab rule) w).map (fun s => A.faceKey (A.keepTab rule) s.facet) =
      ((A.aroundV (A.keepTab rule) w).map Sector.facet).map (A.faceKey (A.keepTab rule)) := by
    rw [List.map_map]; rfl
  rw [e1, e2]
  apply faces_count h rule
  · intro x hx
    obtain ⟨s, hs, rfl⟩ := List.mem_map.mp hx
    exact (mem_aroundV.mp hs).2
  · intro y
    simp only [List.mem_map]
    constructor
    · rintro ⟨S, hS, rfl⟩
      obtain ⟨hS1, hS2⟩ := mem_aroundV'.mp hS
      obtain ⟨s, hs, rfl⟩ := h.sector_onto w hw V hν S hS1
      have hsf := h.sector_facets_A w hw s hs
      refine ⟨s.facet, ⟨s, mem_aroundV.mpr ⟨hs, (result_iff h rule hsf).mp ?_⟩, rfl⟩, (h.sector_facet w hw s hs).symm⟩
      rw [← h.sector_facet w hw s hs]; exact hS2
    · rintro ⟨_, ⟨s, hs, rfl⟩, rfl⟩
      obtain ⟨hs1, hs2⟩ := mem_aroundV.mp hs
      refine ⟨κ w s, mem_aroundV'.mpr ⟨h.sector_mem w hw V hν s hs1, ?_⟩, h.sector_facet w hw s hs1⟩
      rw [h.sector_facet w hw s hs1]; exact (result_iff h rule (h.sector_facets_A w hw s hs1)).mpr hs2

theorem atV_none (h : Refines A B σ τ ν κ) (rule : Rule) {w : α} (hw : w ∈ A.verts) (hν : ν w = none)
    (hE : ∀ E ∈ B.edges, B.badEdge (B.keepTab rule) E = false) :
    (A.facesAtV (A.keepTab rule) (A.faceKey (A.keepTab rule)) w).length < 3 := by
  simp only [IncLedger.facesAtV]
  have e2 : (A.aroundV (A.keepTab rule) w).map (fun s => A.faceKey (A.keepTab rule) s.facet) =
      ((A.aroundV (A.keepTab rule) w).map Sector.facet).map (A.faceKey (A.keepTab rule)) := by
    rw [List.map_map]; rfl
  rw [e2]
  have hxs : ∀ x ∈ (A.aroundV (A.keepTab rule) w).map Sector.facet, x ∈ A.result (A.keepTab rule) := by
    intro x hx; obtain ⟨s, hs, rfl⟩ := List.mem_map.mp hx; exact (mem_aroundV.mp hs).2
  have hsec : ∀ x ∈ (A.aroundV (A.keepTab rule) w).map Sector.facet, ∃ s ∈ A.vertSectors w, s.facet = x := by
    intro x hx; obtain ⟨s, hs, rfl⟩ := List.mem_map.mp hx; exact ⟨s, (mem_aroundV.mp hs).1, rfl⟩
  rw [← faces_count h rule hxs (fun y => List.mem_map)]
  rcases h.vert_inner w hw hν with ⟨ψ, hψ⟩ | ⟨E, hEm, e, he, hτ, hEs⟩
  · -- inside one coarse facet: one face
    have hsub : dedup ((((A.aroundV (A.keepTab rule) w).map Sector.facet).map σ).map (B.faceKey (B.keepTab rule))) ⊆
        [B.faceKey (B.keepTab rule) ψ] := by
      intro y hy
      obtain ⟨z, hz, rfl⟩ := List.mem_map.mp ((mem_dedup _ _).mp hy)
      obtain ⟨x, hx, rfl⟩ := List.mem_map.mp hz
      obtain ⟨s, hs, rfl⟩ := hsec x hx
      rw [hψ s hs]; exact List.mem_singleton_self _
    have := (nodup_dedup _).length_le_of_subset hsub
    simp only [List.length_singleton] at this; omega
  · -- inside a coarse edge: its faces, at most two since it is not bad
    have hsub : dedup ((((A.aroundV (A.keepTab rule) w).map Sector.facet).map σ).map (B.faceKey (B.keepTab rule))) ⊆
        (B.aroundE (B.keepTab rule) E).map (B.faceKey (B.keepTab rule)) := by
      intro y hy
      obtain ⟨z, hz, rfl⟩ := List.mem_map.mp ((mem_dedup _ _).mp hy)
      obtain ⟨x, hx, rfl⟩ := List.mem_map.mp hz
      obtain ⟨s, hs, rfl⟩ := hsec x hx
      refine List.mem_map.mpr ⟨σ s.facet, ?_, rfl⟩
      simp only [IncLedger.aroundE, List.mem_filter, List.contains_iff_mem, decide_eq_true_eq]
      exact ⟨hEs s hs, (result_iff h rule (h.sector_facets_A w hw s hs)).mpr (hxs _ hx)⟩
    have h1 := (nodup_dedup _).length_le_of_subset hsub
    have h2 := hE E hEm
    simp only [IncLedger.badEdge, decide_eq_false_iff_not, Nat.not_lt] at h2
    rw [List.length_map] at h1
    omega

end Refines

end ForgeExact.DecideOnce
