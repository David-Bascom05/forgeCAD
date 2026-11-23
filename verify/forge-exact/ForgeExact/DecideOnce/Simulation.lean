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

end Refines

end ForgeExact.DecideOnce
