import ForgeTopo.Euler
open Aeneas Aeneas.Std Result

/-!
# The shipped `topo_fans::fans` refines the model's `checkFans`

`fans` (kernel/forge-topo/src/topo_fans.rs; check.rs's `check_fans` calls it) as Aeneas translates
it, against verify/forge-topo-check's `Body.checkFans`, for bodies whose links are sound. The
corners of a face at a vertex are geometry: the view's `corners` is the model's `corners`
argument, taken as given, with one hypothesis (`CornersUse`): the coedge closing a corner is a use
of an edge (`Body::corners` takes them from the face's own loops).
-/

set_option maxRecDepth 8192

namespace forge_topo

open TopoCheck

section
variable {T : Type} (FI : topo_fans.FanTopo T) (t : T) (b : Body) {corners : Nat → Nat → List (Nat × Nat)}

/-- What a `FanTopo` instance reads of model body `b`, with the model's corner table. -/
structure FanReads (corners : Nat → Nat → List (Nat × Nat)) : Prop where
  base : Reads FI.topoTopoInst t b
  fwd : ∀ c : Usize, c.val < b.coedges.size → FI.coedge_forward t c ⦃ x => x = (b.co c.val).fwd ⦄
  corners : ∀ f v : Usize, FI.corners t f v ⦃ r =>
    r.val.map (fun p => (p.1.val, p.2.val)) = corners f.val v.val ⦄
  /-- Face ids are u32, below `u32::MAX`. -/
  nf_lt : b.faces.size < U32.max

/-- A use of an edge: what the walk visits. -/
def Use (c : Nat) : Prop := ∃ e < b.edges.size, c ∈ (b.ed e).uses

/-- The closing coedge of every corner is a use. -/
def CornersUse (corners : Nat → Nat → List (Nat × Nat)) : Prop :=
  ∀ f v, ∀ p ∈ corners f v, Use b p.2

variable {b}

theorem use_facts (H : Sound b) (W : b.WF) {c : Nat} (hc : Use b c) :
    c < b.coedges.size ∧ (b.co c).edge < b.edges.size ∧ c ∈ (b.ed (b.co c).edge).uses ∧
    (b.co c).lp < b.loops.size ∧ c ∈ b.loopCoedges (b.co c).lp ∧ b.listed (b.co c).lp = 1 := by
  obtain ⟨e, he, hm⟩ := hc
  have he' := W.use_edge e he c hm
  have ⟨hl, hlm, hl1⟩ := W.use_loop e he c hm
  exact ⟨(H.uses e he c hm).1, by rw [he']; exact he, by rw [he']; exact hm, hl, hlm, hl1⟩

theorem use_prev (H : Sound b) (W : b.WF) {c : Nat} (hc : Use b c) : Use b (b.prev c) := by
  obtain ⟨-, -, -, hl, hlm, hl1⟩ := use_facts H W hc
  have hlen : 0 < (b.lo (b.co c).lp).len := by
    rw [mem_loopCoedges] at hlm; omega
  have hpm : b.prev c ∈ b.loopCoedges (b.co c).lp := by
    rw [mem_loopCoedges] at hlm ⊢
    unfold Body.prev; dsimp only
    have := Nat.mod_lt (c - (b.lo (b.co c).lp).first + (b.lo (b.co c).lp).len - 1) hlen
    omega
  have ⟨he, hu⟩ := W.loop_use _ hl hl1 _ hpm
  exact ⟨_, he, hu⟩

theorem use_partner (H : Sound b) (W : b.WF) {c n : Nat} (hc : Use b c) (hp : b.partner c = some n) :
    Use b n := by
  obtain ⟨-, he, -⟩ := use_facts H W hc
  unfold Body.partner at hp
  exact ⟨_, he, List.mem_of_find?_eq_some hp⟩

/-! ## The leaves -/

theorem start_spec (R : FanReads FI t b corners) (c : Usize) (hc : c.val < b.coedges.size)
    (he : (b.co c.val).edge < b.edges.size) :
    topo_fans.start FI t c ⦃ r => r.val = b.start c.val ⦄ := by
  unfold topo_fans.start
  step with R.base.coedge_edge c hc as ⟨e, hev⟩
  step with R.base.edge_ends e (by rw [hev]; exact he) as ⟨x, y, hx, hy⟩
  step with R.fwd c hc as ⟨f, hf⟩
  unfold Body.start
  split <;> rename_i h <;> rw [hf] at h <;> simp [h, hx, hy, hev]

theorem stop_spec (R : FanReads FI t b corners) (c : Usize) (hc : c.val < b.coedges.size)
    (he : (b.co c.val).edge < b.edges.size) :
    topo_fans.stop FI t c ⦃ r => r.val = b.stop c.val ⦄ := by
  unfold topo_fans.stop
  step with R.base.coedge_edge c hc as ⟨e, hev⟩
  step with R.base.edge_ends e (by rw [hev]; exact he) as ⟨x, y, hx, hy⟩
  step with R.fwd c hc as ⟨f, hf⟩
  unfold Body.stop
  split <;> rename_i h <;> rw [hf] at h <;> simp [h, hx, hy, hev]

theorem face_of_spec (R : FanReads FI t b corners) (c : Usize) (hc : c.val < b.coedges.size)
    (hl : (b.co c.val).lp < b.loops.size) :
    topo_fans.face_of FI t c ⦃ r => r.val = b.face c.val ⦄ := by
  unfold topo_fans.face_of
  step with R.base.coedge_loop c hc as ⟨l, hlv⟩
  step with R.base.loop_face l (by rw [hlv]; exact hl) as ⟨f, hf⟩
  rw [hf, hlv]; rfl

theorem lt_size_of_le_max (x : Nat) (h : x ≤ Usize.max) : x < 2 ^ UScalarTy.Usize.numBits := by
  have : Usize.max = 2 ^ UScalarTy.Usize.numBits - 1 := by rw [Usize.max_def, Usize.numBits_def]
  have := Nat.one_le_two_pow (n := UScalarTy.Usize.numBits)
  omega

/-- The model's `prev` of a coedge of its loop: the one before, the last for the first. -/
theorem prev_eq (c first len : Nat) (h1 : first ≤ c) (h2 : c < first + len) :
    first + (c - first + len - 1) % len = if c = first then first + len - 1 else c - 1 := by
  split
  · subst c; rw [Nat.sub_self, Nat.zero_add, Nat.mod_eq_of_lt (by omega)]; omega
  · rw [show c - first + len - 1 = (c - first - 1) + len by omega, Nat.add_mod_right,
      Nat.mod_eq_of_lt (by omega)]; omega

theorem prev_of_spec (R : FanReads FI t b corners) (H : Sound b) (W : b.WF) (c : Usize) (hc : Use b c.val) :
    topo_fans.prev_of FI t c ⦃ r => r.val = b.prev c.val ⦄ := by
  obtain ⟨hcn, -, -, hl, hlm, hl1⟩ := use_facts H W hc
  have hlm' := hlm
  rw [mem_loopCoedges] at hlm
  have hlast : (b.lo (b.co c.val).lp).first + ((b.lo (b.co c.val).lp).len - 1) ∈ b.loopCoedges (b.co c.val).lp := by
    rw [mem_loopCoedges]; omega
  have hlastn := (H.coedges _ hl hl1 _ hlast).1
  have hprev : b.prev c.val = if c.val = (b.lo (b.co c.val).lp).first
      then (b.lo (b.co c.val).lp).first + (b.lo (b.co c.val).lp).len - 1 else c.val - 1 := by
    unfold Body.prev; dsimp only; exact prev_eq _ _ _ hlm.1 hlm.2
  obtain ⟨m, hm⟩ := WP.spec_exists R.base.nc
  have hmb := m.hBounds
  have h1 : (1#usize).val = 1 := rfl
  unfold topo_fans.prev_of
  step with R.base.coedge_loop c hcn as ⟨l, hlv⟩
  step with R.base.loop_first l (by rw [hlv]; exact hl) as ⟨first, hfirst⟩
  rw [hlv] at hfirst
  split
  · rename_i hcf
    have hcf' : c.val = first.val := by rw [hcf]
    step with R.base.loop_len l (by rw [hlv]; exact hl) as ⟨n, hn⟩
    rw [hlv] at hn
    have hmx := le_max m
    step with usize_add_wspec first n (by omega) as ⟨i1, hi1⟩
    have hi1v : i1.val = first.val + n.val := by
      rw [hi1]; exact add_val first n (lt_size_of_le_max _ (by omega))
    step with usize_sub_wspec i1 1#usize (by omega) as ⟨r, hr⟩
    rw [hr, sub_val i1 1#usize (by omega), hi1v, hprev, if_pos (by omega), hfirst, hn, h1]
  · rename_i hcf
    have hcf' : c.val ≠ first.val := fun h => hcf (UScalar.eq_of_val_eq h)
    step with usize_sub_wspec c 1#usize (by omega) as ⟨r, hr⟩
    rw [hr, sub_val c 1#usize (by omega), hprev, if_neg (by omega), h1]

theorem none_val : topo_fans.NONE.val = Usize.max := by
  unfold topo_fans.NONE; simp [core.num.Usize.MAX]

/-- `partner`'s result: the model's partner, or `NONE` for none. -/
def PartnerIs (o : Option Nat) (r : Usize) : Prop :=
  (∃ x, o = some x ∧ r.val = x ∧ r ≠ topo_fans.NONE) ∨ (o = none ∧ r = topo_fans.NONE)

theorem find_take_succ (L : List Nat) (P : Nat → Bool) (i : Nat) (hi : i < L.length)
    (h : (L.take i).find? P = none) :
    (L.take (i + 1)).find? P = if P L[i] then some L[i] else none := by
  rw [List.take_succ_eq_append_getElem hi, List.find?_append, h]
  simp only [Option.none_or]
  split <;> simp_all

theorem partner_spec (R : FanReads FI t b corners) (H : Sound b) (W : b.WF) (c : Usize) (hc : Use b c.val) :
    topo_fans.partner FI t c ⦃ r => PartnerIs (b.partner c.val) r ⦄ := by
  obtain ⟨hcn, hen, -, -, -, -⟩ := use_facts H W hc
  obtain ⟨m, hm⟩ := WP.spec_exists R.base.nc
  have hmx := le_max m
  have hnone := none_val
  unfold topo_fans.partner
  step with R.base.coedge_edge c hcn as ⟨e, hev⟩
  have hen' : e.val < b.edges.size := by rw [hev]; exact hen
  step with R.base.edge_n_uses e hen' as ⟨n, hn⟩
  set U := (b.ed e.val).uses with hU
  have hUlt : ∀ x ∈ U, x < b.coedges.size := fun x hx => (H.uses e.val hen' x hx).1
  have hpart : b.partner c.val = U.find? (· != c.val) := by unfold Body.partner; rw [← hev]
  rw [hpart]
  unfold topo_fans.partner_loop
  apply loop.spec_decr_nat
    (fun (y : Usize × Usize) => n.val - y.1.val + (if y.2 = topo_fans.NONE then 1 else 0))
    (fun y => y.1.val ≤ n.val ∧ PartnerIs ((U.take y.1.val).find? (· != c.val)) y.2)
  · rintro ⟨i, found⟩ ⟨hle, hP⟩
    simp only at hle hP ⊢
    unfold topo_fans.partner_loop.body
    split
    · rename_i hf
      rcases hP with ⟨x, -, -, hx⟩ | ⟨hfn, -⟩
      · exact absurd hf hx
      split
      · rename_i hlt
        have hlt' : i.val < n.val := by scalar_tac
        have hiU : i.val < U.length := by rw [← hn]; exact hlt'
        step with R.base.edge_use e i hen' hiU as ⟨o, ho⟩
        have hoU : o.val = U[i.val] := by rw [ho, getElem!_pos U i.val hiU]
        have hstep := find_take_succ U (· != c.val) i.val hiU hfn
        apply WP.spec_bind (Pₘ := fun f1 : Usize => PartnerIs ((U.take (i.val + 1)).find? (· != c.val)) f1)
        · split
          · rename_i hoc
            simp only [WP.spec_ok]
            have hoc' : (U[i.val] != c.val) = true := by
              have hoc2 : o ≠ c := by simpa using hoc
              rw [← hoU]; simp only [bne_iff_ne, ne_eq]; intro h; exact hoc2 (UScalar.eq_of_val_eq h)
            rw [hstep, if_pos hoc']
            refine Or.inl ⟨_, rfl, hoU, fun h => ?_⟩
            have := hUlt _ (List.getElem_mem hiU)
            rw [h, hnone] at hoU; omega
          · rename_i hoc
            simp only [WP.spec_ok]
            have hoc' : (U[i.val] != c.val) = false := by
              have hoc2 : o.val = c.val := by simpa using hoc
              rw [← hoU, hoc2]; simp
            rw [hstep, if_neg (by simp [hoc'])]
            exact Or.inr ⟨rfl, hf⟩
        · intro f1 hf1
          step with usize_add_wspec i 1#usize (succ_le_max i n hlt') as ⟨i1, hi1⟩
          have hi1v : i1.val = i.val + 1 := by rw [hi1]; exact succ_val i n hlt'
          rw [hi1v]
          refine ⟨by omega, hf1, ?_⟩
          split <;> (try split) <;> omega
      · rename_i hlt
        have hi : i.val = n.val := by scalar_tac
        simp only [WP.spec_ok]
        have : U.take i.val = U := by rw [hi, hn]; exact List.take_length
        rw [this] at hfn
        exact Or.inr ⟨hfn, hf⟩
    · rename_i hf
      simp only [WP.spec_ok]
      rcases hP with ⟨x, hx, hxv, hxn⟩ | ⟨-, hfn⟩
      · -- found in the first `i`: the whole list finds it first
        refine Or.inl ⟨x, ?_, hxv, hxn⟩
        rw [← List.take_append_drop i.val U, List.find?_append, hx]; rfl
      · exact absurd hfn hf
  · have h0 : (0#usize).val = 0 := rfl
    simp only [h0, List.take_zero, List.find?_nil]
    exact ⟨by simp, Or.inr ⟨rfl, rfl⟩⟩

end

theorem contains_spec (xs : Slice Usize) (n x : Usize) (hn : n.val ≤ xs.val.length) :
    topo_fans.contains xs n x ⦃ r => r = decide (x ∈ xs.val.take n.val) ⦄ := by
  unfold topo_fans.contains topo_fans.contains_loop
  apply loop.spec_decr_nat (fun (i : Usize) => n.val - i.val)
    (fun i => i.val ≤ n.val ∧ x ∉ xs.val.take i.val)
  · rintro i ⟨hle, hnot⟩
    unfold topo_fans.contains_loop.body
    split
    · rename_i hlt
      have hlt' : i.val < n.val := by scalar_tac
      have hix : i.val < xs.length := by have : xs.length = xs.val.length := rfl; omega
      step with Slice.index_usize_spec xs i hix as ⟨y, hy⟩
      split
      · rename_i hyx
        simp only [WP.spec_ok, decide_eq_true_eq]
        rw [← hyx, hy]
        have hmem : xs.val[i.val] ∈ List.take n.val xs.val := by
          rw [List.mem_take_iff_getElem]; exact ⟨i.val, by simp; omega, rfl⟩
        exact (decide_eq_true hmem).symm
      · rename_i hyx
        step with usize_add_wspec i 1#usize (succ_le_max i n hlt') as ⟨i1, hi1⟩
        have hi1v : i1.val = i.val + 1 := by rw [hi1]; exact succ_val i n hlt'
        refine ⟨by omega, ?_, by omega⟩
        rw [hi1v, List.take_succ_eq_append_getElem (by omega)]
        simp only [List.mem_append, List.mem_singleton, not_or]
        refine ⟨hnot, fun h => hyx ?_⟩
        rw [hy, h]
    · rename_i hlt
      have hi : i.val = n.val := by scalar_tac
      simp only [WP.spec_ok]
      rw [← hi]; simp [hnot]
  · exact ⟨by simp, by simp⟩

/-! ## The model's walk -/

section model
variable (b : Body)

/-- Every edge use, edge by edge. -/
def allUses : List Nat := b.edges.toList.flatMap (·.uses)

theorem allUses_length : (allUses b).length = (b.edges.toList.map (·.uses.length)).sum := by
  unfold allUses; rw [List.length_flatMap]

theorem use_mem_allUses {c : Nat} (h : Use b c) : c ∈ allUses b := by
  obtain ⟨e, he, hm⟩ := h
  unfold allUses; rw [List.mem_flatMap]
  refine ⟨b.edges[e], by simp, ?_⟩
  have : b.ed e = b.edges[e] := by unfold Body.ed; rw [Array.getD_eq_getD_getElem?, Array.getElem?_eq_getElem he]; rfl
  rw [← this]; exact hm

/-- What a walk's seen list stays: distinct uses. -/
def SeenOK (S : List Nat) : Prop := S.Nodup ∧ ∀ x ∈ S, Use b x

theorem seenOK_length {S : List Nat} (h : SeenOK b S) : S.length ≤ (allUses b).length :=
  (h.1.subperm fun x hx => use_mem_allUses b (h.2 x hx)).length_le

theorem seenOK_length_nc (H : Sound b) (W : b.WF) {S : List Nat} (h : SeenOK b S) :
    S.length ≤ b.coedges.size := by
  have := (h.1.subperm fun x hx => List.mem_range.2 (use_facts H W (h.2 x hx)).1).length_le
  simpa using this

theorem seenOK_snoc {S : List Nat} {c : Nat} (h : SeenOK b S) (hc : Use b c) (hn : c ∉ S) :
    SeenOK b (S ++ [c]) := by
  refine ⟨List.nodup_append.2 ⟨h.1, by simp, by simp; intro a ha hac; exact hn (hac ▸ ha)⟩, fun x hx => ?_⟩
  rcases List.mem_append.1 hx with hx | hx
  · exact h.2 x hx
  · simp at hx; rw [hx]; exact hc

variable {b}

theorem walk_seen (bef : Nat → Nat) : ∀ (F c : Nat) (S : List Nat),
    (b.walk bef F c S).2 = S ++ (b.walk bef F c S).1
  | 0, c, S => by simp [Body.walk]
  | F + 1, c, S => by
    unfold Body.walk
    split
    · simp
    · split
      · rename_i n hn
        simp only
        rw [walk_seen bef F n (S ++ [c])]; simp
      · simp

/-- A corner rule that keeps the walk on uses. -/
def BefUse (bef : Nat → Nat) : Prop := ∀ c, Use b c → Use b (bef c)

theorem walk_ok (H : Sound b) (W : b.WF) {bef : Nat → Nat} (hbef : BefUse (b := b) bef) :
    ∀ (F c : Nat) (S : List Nat), Use b c → SeenOK b S → SeenOK b (b.walk bef F c S).2
  | 0, c, S, _, hS => by simpa [Body.walk] using hS
  | F + 1, c, S, hc, hS => by
    unfold Body.walk
    split
    · exact hS
    · rename_i hcs
      have hcs' : c ∉ S := by simpa using hcs
      split
      · rename_i n hn
        exact walk_ok H W hbef F n _ (use_partner H W (hbef c hc) hn) (seenOK_snoc b hS hc hcs')
      · exact seenOK_snoc b hS hc hcs'

/-- **Fuel past the unseen uses does not matter.** -/
theorem walk_fuel (H : Sound b) (W : b.WF) {bef : Nat → Nat} (hbef : BefUse (b := b) bef) :
    ∀ (F G c : Nat) (S : List Nat), Use b c → SeenOK b S →
      (allUses b).length < F + S.length → (allUses b).length < G + S.length →
      b.walk bef F c S = b.walk bef G c S
  | 0, G, c, S, _, hS, hF, _ => by have := seenOK_length b hS; omega
  | F + 1, 0, c, S, _, hS, _, hG => by have := seenOK_length b hS; omega
  | F + 1, G + 1, c, S, hc, hS, hF, hG => by
    unfold Body.walk
    split
    · rfl
    · rename_i hcs
      have hcs' : c ∉ S := by simpa using hcs
      split
      · rename_i n hn
        have hS' := seenOK_snoc b hS hc hcs'
        rw [walk_fuel H W hbef F G n (S ++ [c]) (use_partner H W (hbef c hc) hn) hS'
          (by simp; omega) (by simp; omega)]
      · rfl

/-- The seen list after the fans of `L`, from `S`. -/
def fansSeen (bef : Nat → Nat) (F : Nat) : List Nat → List Nat → List Nat
  | [], S => S
  | c0 :: rest, S => if S.contains c0 then fansSeen bef F rest S else fansSeen bef F rest (b.walk bef F c0 S).2

theorem fansGo_snoc (bef : Nat → Nat) (F : Nat) : ∀ (L : List Nat) (c : Nat) (S : List Nat),
    b.fansGo bef F (L ++ [c]) S = b.fansGo bef F L S ++
      (if (fansSeen (b := b) bef F L S).contains c then [] else [(b.walk bef F c (fansSeen (b := b) bef F L S)).1])
  | [], c, S => by simp [Body.fansGo, fansSeen]
  | c0 :: rest, c, S => by
    simp only [List.cons_append, Body.fansGo, fansSeen]
    split
    · exact fansGo_snoc bef F rest c S
    · rw [fansGo_snoc bef F rest c]; simp

theorem fansSeen_snoc (bef : Nat → Nat) (F : Nat) : ∀ (L : List Nat) (c : Nat) (S : List Nat),
    fansSeen (b := b) bef F (L ++ [c]) S =
      (if (fansSeen (b := b) bef F L S).contains c then fansSeen (b := b) bef F L S
       else (b.walk bef F c (fansSeen (b := b) bef F L S)).2)
  | [], c, S => by simp [fansSeen]
  | c0 :: rest, c, S => by
    simp only [List.cons_append, fansSeen]
    split <;> exact fansSeen_snoc bef F rest c _

theorem fansSeen_flatten (bef : Nat → Nat) (F : Nat) : ∀ (L S : List Nat),
    fansSeen (b := b) bef F L S = S ++ (b.fansGo bef F L S).flatten
  | [], S => by simp [fansSeen, Body.fansGo]
  | c0 :: rest, S => by
    simp only [fansSeen, Body.fansGo]
    split
    · exact fansSeen_flatten bef F rest S
    · rw [fansSeen_flatten bef F rest, walk_seen]; simp

end model

/-! ## The walk, as shipped -/

/-- The logical seen list: the first `sl` of the buffer. -/
def seenOf (seen : alloc.vec.Vec Usize) (sl : Nat) : List Nat := (seen.val.take sl).map (·.val)

theorem mem_seenOf (seen : alloc.vec.Vec Usize) (sl : Nat) (c : Usize) :
    c ∈ seen.val.take sl ↔ c.val ∈ seenOf seen sl := by
  unfold seenOf; rw [List.mem_map]
  constructor
  · intro h; exact ⟨c, h, rfl⟩
  · rintro ⟨d, hd, hdc⟩; rwa [← UScalar.eq_of_val_eq hdc]

theorem seenOf_write (seen : alloc.vec.Vec Usize) (sl : Nat) (c : Usize) (hsl : sl ≤ seen.val.length) :
    List.map (·.val) ((if sl < seen.val.length then seen.val.set sl c else seen.val ++ [c]).take (sl + 1)) =
      seenOf seen sl ++ [c.val] := by
  unfold seenOf
  split
  · rename_i h
    rw [List.take_succ_eq_append_getElem (by simp; omega)]
    simp [List.take_set_of_le (le_refl sl), List.getElem_set_self]
  · rename_i h
    have : sl = seen.val.length := by omega
    subst this
    simp

section walk
variable {T : Type} (FI : topo_fans.FanTopo T) (t : T) {b : Body} {corners : Nat → Nat → List (Nat × Nat)}

/-- `before` computes the corner rule `bef` on uses. -/
def BefSpec (closes : topo_fans.Closes) (byc : Bool) (bef : Nat → Nat) : Prop :=
  ∀ c : Usize, Use b c.val → topo_fans.before FI t closes byc c ⦃ r => r.val = bef c.val ⦄

theorem walk_spec (R : FanReads FI t b corners) (H : Sound b) (W : b.WF) (closes : topo_fans.Closes)
    (byc : Bool) (bef : Nat → Nat) (hB : BefSpec FI t (b := b) closes byc bef) (hbef : BefUse (b := b) bef)
    (seen : alloc.vec.Vec Usize) (sl c : Usize) (hsl : sl.val ≤ seen.val.length)
    (hc : Use b c.val) (hS : SeenOK b (seenOf seen sl.val)) :
    topo_fans.count_fans_loop0_loop0 FI t closes byc seen sl c true ⦃ r =>
      r.2.val ≤ r.1.val.length ∧ SeenOK b (seenOf r.1 r.2.val) ∧
      seenOf r.1 r.2.val = (b.walk bef ((allUses b).length + 1) c.val (seenOf seen sl.val)).2 ⦄ := by
  set U := (allUses b).length
  set Tgt := (b.walk bef (U + 1) c.val (seenOf seen sl.val)).2 with hTgt
  unfold topo_fans.count_fans_loop0_loop0
  apply loop.spec_decr_nat
    (fun (y : alloc.vec.Vec Usize × Usize × Usize × Bool) => 2 * (U + 1 - y.2.1.val) + (if y.2.2.2 then 1 else 0))
    (fun y => y.2.1.val ≤ y.1.val.length ∧ SeenOK b (seenOf y.1 y.2.1.val) ∧
      (y.2.2.2 = true → Use b y.2.2.1.val ∧ Tgt = (b.walk bef (U + 1) y.2.2.1.val (seenOf y.1 y.2.1.val)).2) ∧
      (y.2.2.2 = false → Tgt = seenOf y.1 y.2.1.val))
  · rintro ⟨sn, k, d, go⟩ ⟨hle, hSk, hgo, hngo⟩
    simp only at hle hSk hgo hngo ⊢
    unfold topo_fans.count_fans_loop0_loop0.body
    split
    · rename_i hg
      obtain ⟨hd, hT⟩ := hgo hg
      have hkl : (seenOf sn k.val).length = k.val := by unfold seenOf; simp; omega
      have hkU : k.val ≤ U := by have := seenOK_length b hSk; omega
      have hderef : (alloc.vec.Vec.deref sn).val = sn.val := by simp [alloc.vec.Vec.deref]
      step with contains_spec (alloc.vec.Vec.deref sn) k d (by rw [hderef]; exact hle) as ⟨isin, hisin⟩
      rw [hderef] at hisin
      split
      · rename_i hin
        simp only [WP.spec_ok]
        refine ⟨hle, hSk, ?_⟩
        have hin' : d.val ∈ seenOf sn k.val := by
          rw [← mem_seenOf]; simpa [hisin] using hin
        rw [hT]; unfold Body.walk; simp [hin']
      · rename_i hin
        have hin' : d.val ∉ seenOf sn k.val := by
          rw [← mem_seenOf]; simpa [hisin] using hin
        have hS' := seenOK_snoc b hSk hd hin'
        have hkU' : k.val + 1 ≤ U := by
          have := seenOK_length b hS'; simp [hkl] at this; omega
        obtain ⟨m, hm⟩ := WP.spec_exists R.base.nc
        have hmx := le_max m
        have hkn : k.val + 1 ≤ b.coedges.size := by
          have := seenOK_length_nc b H W hS'; simp [hkl] at this; omega
        apply WP.spec_bind (Pₘ := fun s1 : alloc.vec.Vec Usize =>
          k.val + 1 ≤ s1.val.length ∧ seenOf s1 (k.val + 1) = seenOf sn k.val ++ [d.val])
        · split
          · rename_i hlt
            step as ⟨x0, back, hx0, hback⟩
            rw [hback]
            simp only [WP.spec_ok, alloc.vec.Vec.set_val_eq, List.length_set]
            refine ⟨by scalar_tac, ?_⟩
            have := seenOf_write sn k.val d hle
            rw [if_pos (by scalar_tac)] at this
            unfold seenOf at this ⊢; simpa using this
          · rename_i hlt
            have hpl : sn.val.length < Usize.max := by
              have : k.val = sn.val.length := by scalar_tac
              omega
            apply WP.spec_mono (alloc.vec.Vec.push_spec sn d hpl)
            intro s1 hs1
            refine ⟨by rw [hs1]; simp; scalar_tac, ?_⟩
            have := seenOf_write sn k.val d hle
            rw [if_neg (by scalar_tac)] at this
            unfold seenOf at this ⊢; rw [hs1]; simpa using this
        · intro s1 ⟨hs1l, hs1⟩
          have h1 : (1#usize).val = 1 := rfl
          step with usize_add_wspec k 1#usize (by rw [h1]; omega) as ⟨k1, hk1⟩
          have hk1v : k1.val = k.val + 1 := by rw [hk1]; exact add_val k 1#usize (lt_size_of_le_max _ (by omega))
          step with hB d hd as ⟨bd, hbd⟩
          have hbu : Use b bd.val := by rw [hbd]; exact hbef _ hd
          step with partner_spec FI t R H W bd hbu as ⟨p, hp⟩
          rw [hbd] at hp
          have hwalk : (b.walk bef (U + 1) d.val (seenOf sn k.val)).2 =
              (match b.partner (bef d.val) with
                | some n => (b.walk bef U n (seenOf sn k.val ++ [d.val])).2
                | none => seenOf sn k.val ++ [d.val]) := by
            have hc : (seenOf sn k.val).contains d.val = false := by simpa using hin'
            conv_lhs => unfold Body.walk
            simp only [hc, Bool.false_eq_true, ite_false]
            cases b.partner (bef d.val) <;> rfl
          split
          · rename_i hpn
            rcases hp with ⟨x, -, -, hx⟩ | ⟨hno, -⟩
            · exact absurd hpn hx
            simp only [WP.spec_ok]
            rw [hk1v]
            refine ⟨hs1l, by rw [hs1]; exact hS', fun h => absurd h (by simp), fun _ => ?_, by simp; omega⟩
            rw [hs1, hT, hwalk, hno]
          · rename_i hpn
            rcases hp with ⟨x, hx, hxv, -⟩ | ⟨-, hno⟩
            · simp only [WP.spec_ok]
              rw [hk1v]
              have hxu : Use b x := use_partner H W (hbef _ hd) hx
              refine ⟨hs1l, by rw [hs1]; exact hS', fun _ => ⟨by rw [hxv]; exact hxu, ?_⟩,
                fun h => absurd h (by simp_all), by simp_all; omega⟩
              rw [hs1, hT, hwalk, hx, hxv]
              simp only
              rw [walk_fuel H W hbef U (U + 1) x _ hxu hS' (by simp [hkl]; omega) (by simp [hkl]; omega)]
            · exact absurd hno hpn
    · rename_i hg
      simp only [WP.spec_ok]
      have hg' : go = false := by simpa using hg
      exact ⟨hle, hSk, (hngo hg').symm⟩
  · refine ⟨hsl, hS, fun _ => ⟨hc, rfl⟩, fun h => absurd h (by simp)⟩

end walk

section count
variable {T : Type} (FI : topo_fans.FanTopo T) (t : T) {b : Body} {corners : Nat → Nat → List (Nat × Nat)}

theorem fansGo_length_le (bef : Nat → Nat) (F : Nat) : ∀ (L S : List Nat),
    (b.fansGo bef F L S).length ≤ L.length
  | [], S => by simp [Body.fansGo]
  | c0 :: rest, S => by
    simp only [Body.fansGo]
    split
    · have := fansGo_length_le bef F rest S; simp; omega
    · have := fansGo_length_le bef F rest (b.walk bef F c0 S).2; simp; omega

theorem fansSeen_ok (H : Sound b) (W : b.WF) {bef : Nat → Nat} (hbef : BefUse (b := b) bef) (F : Nat) :
    ∀ (L S : List Nat), (∀ x ∈ L, Use b x) → SeenOK b S → SeenOK b (fansSeen (b := b) bef F L S)
  | [], S, _, hS => hS
  | c0 :: rest, S, hL, hS => by
    simp only [fansSeen]
    split
    · exact fansSeen_ok H W hbef F rest S (fun x hx => hL x (by simp [hx])) hS
    · exact fansSeen_ok H W hbef F rest _ (fun x hx => hL x (by simp [hx]))
        (walk_ok H W hbef F c0 S (hL c0 (by simp)) hS)

/-- The coedges `items[lo..hi]`, as numbers. -/
def frmOf (items : Slice Usize) (lo hi : Nat) : List Nat := ((items.val.drop lo).take (hi - lo)).map (·.val)

theorem frmOf_succ (items : Slice Usize) (lo i hi : Nat) (h1 : lo ≤ i) (h2 : i < hi) (h3 : hi ≤ items.val.length) :
    (frmOf items lo hi).take (i + 1 - lo) = (frmOf items lo hi).take (i - lo) ++ [(items.val[i]!).val] := by
  unfold frmOf
  rw [← List.map_take, ← List.map_take, List.take_take, List.take_take,
    show min (i + 1 - lo) (hi - lo) = (i - lo) + 1 by omega, show min (i - lo) (hi - lo) = i - lo by omega,
    List.take_succ_eq_append_getElem (by simp; omega)]
  simp only [List.map_append, List.map_cons, List.map_nil, List.getElem_drop]
  rw [getElem!_pos items.val i (by omega)]
  simp [show lo + (i - lo) = i by omega]

theorem count_spec (R : FanReads FI t b corners) (H : Sound b) (W : b.WF) (closes : topo_fans.Closes)
    (byc : Bool) (bef : Nat → Nat) (hB : BefSpec FI t (b := b) closes byc bef) (hbef : BefUse (b := b) bef)
    (F : Nat) (hF : (allUses b).length < F)
    (items : Slice Usize) (lo hi : Usize) (hlo : lo.val ≤ hi.val) (hhi : hi.val ≤ items.val.length)
    (hfrm : ∀ x ∈ frmOf items lo.val hi.val, Use b x) (seen : alloc.vec.Vec Usize) :
    topo_fans.count_fans FI t items lo hi closes byc seen ⦃ r =>
      r.1.1.val = (b.fansGo bef F (frmOf items lo.val hi.val) []).length ∧
      r.1.2.val ≤ r.2.val.length ∧
      seenOf r.2 r.1.2.val = fansSeen (b := b) bef F (frmOf items lo.val hi.val) [] ⦄ := by
  set frm := frmOf items lo.val hi.val with hfrmdef
  have hfl : frm.length = hi.val - lo.val := by simp [hfrmdef, frmOf]; omega
  unfold topo_fans.count_fans topo_fans.count_fans_loop0
  apply WP.spec_bind (Pₘ := fun r : alloc.vec.Vec Usize × Usize × Usize =>
      r.2.1.val = (b.fansGo bef F frm []).length ∧ r.2.2.val ≤ r.1.val.length ∧
      seenOf r.1 r.2.2.val = fansSeen (b := b) bef F frm [])
  · apply loop.spec_decr_nat
      (fun (y : alloc.vec.Vec Usize × Usize × Usize × Usize) => hi.val - y.2.2.2.val)
      (fun y => lo.val ≤ y.2.2.2.val ∧ y.2.2.2.val ≤ hi.val ∧
        y.2.1.val = (b.fansGo bef F (frm.take (y.2.2.2.val - lo.val)) []).length ∧
        y.2.2.1.val ≤ y.1.val.length ∧
        seenOf y.1 y.2.2.1.val = fansSeen (b := b) bef F (frm.take (y.2.2.2.val - lo.val)) [])
    · rintro ⟨sn, cnt, k, i⟩ ⟨hlo', hle, hcnt, hkl, hsk⟩
      simp only at hlo' hle hcnt hkl hsk ⊢
      have hSok : SeenOK b (seenOf sn k.val) := by
        rw [hsk]; exact fansSeen_ok H W hbef F _ [] (fun x hx => hfrm x (List.mem_of_mem_take hx)) ⟨by simp, by simp⟩
      unfold topo_fans.count_fans_loop0.body
      split
      · rename_i hlt
        have hlt' : i.val < hi.val := by scalar_tac
        have hix : i.val < items.length := by have : items.length = items.val.length := rfl; omega
        step with Slice.index_usize_spec items i hix as ⟨c0, hc0⟩
        have hc0' : c0 = items.val[i.val]! := by rw [hc0, getElem!_pos]
        have hpre := frmOf_succ items lo.val i.val hi.val hlo' hlt' hhi
        rw [← hfrmdef, ← hc0'] at hpre
        have hcu : Use b c0.val := hfrm _ (by
          have : c0.val ∈ frm.take (i.val + 1 - lo.val) := by rw [hpre]; simp
          exact List.mem_of_mem_take this)
        have hderef : (alloc.vec.Vec.deref sn).val = sn.val := by simp [alloc.vec.Vec.deref]
        step with contains_spec (alloc.vec.Vec.deref sn) k c0 (by rw [hderef]; exact hkl) as ⟨isin, hisin⟩
        rw [hderef] at hisin
        have hcnt_le : cnt.val ≤ i.val - lo.val := by
          rw [hcnt]; have := fansGo_length_le (b := b) bef F (frm.take (i.val - lo.val)) []; simp at this; omega
        have hhx := le_max hi
        have hU := hF
        split
        · rename_i hin
          have hin' : c0.val ∈ seenOf sn k.val := by rw [← mem_seenOf]; simpa [hisin] using hin
          step with usize_add_wspec i 1#usize (succ_le_max i hi hlt') as ⟨i1, hi1⟩
          have hi1v : i1.val = i.val + 1 := by rw [hi1]; exact succ_val i hi hlt'
          rw [hi1v]
          refine ⟨by omega, by omega, ?_, hkl, ?_, by omega⟩
          · rw [hpre, fansGo_snoc, ← hsk, hcnt]; simp [hin']
          · rw [hpre, fansSeen_snoc, ← hsk]; simp [hin']
        · rename_i hin
          have hin' : c0.val ∉ seenOf sn k.val := by rw [← mem_seenOf]; simpa [hisin] using hin
          have h1 : (1#usize).val = 1 := rfl
          step with usize_add_wspec cnt 1#usize (by rw [h1]; omega) as ⟨cnt1, hcnt1⟩
          have hcnt1v : cnt1.val = cnt.val + 1 := by
            rw [hcnt1]; exact add_val cnt 1#usize (lt_size_of_le_max _ (by rw [h1]; omega))
          step with walk_spec FI t R H W closes byc bef hB hbef sn k c0 hkl hcu hSok as ⟨s1, k1, hk1l, hS1, hs1⟩
          step with usize_add_wspec i 1#usize (succ_le_max i hi hlt') as ⟨i1, hi1⟩
          have hi1v : i1.val = i.val + 1 := by rw [hi1]; exact succ_val i hi hlt'
          rw [hi1v]
          have hwf := walk_fuel H W hbef ((allUses b).length + 1) F c0.val (seenOf sn k.val) hcu hSok
            (by omega) (by omega)
          refine ⟨by omega, by omega, ?_, hk1l, ?_, by omega⟩
          · rw [hcnt1v, hpre, fansGo_snoc, ← hsk, hcnt]; simp [hin']
          · rw [hs1, hpre, fansSeen_snoc, ← hsk, hwf]; simp [hin']
      · rename_i hlt
        have hi : i.val = hi.val := by scalar_tac
        simp only [WP.spec_ok]
        have htk : frm.take (i.val - lo.val) = frm := by rw [hi]; exact List.take_of_length_le (by omega)
        rw [htk] at hcnt hsk
        exact ⟨hcnt, hkl, hsk⟩
    · have h0 : (0#usize).val = 0 := rfl
      simp only [h0, Nat.sub_self, List.take_zero]
      exact ⟨le_refl _, hlo, by simp [Body.fansGo], by simp, by simp [seenOf, fansSeen]⟩
  · rintro ⟨sn, cnt, k⟩ ⟨h1, h2, h3⟩
    show (ok ((cnt, k), sn) : Result _) ⦃ _ ⦄
    simp only [WP.spec_ok]
    exact ⟨h1, h2, h3⟩

end count

/-! ## The corner table -/

theorem eraseDups_snoc (xs : List Nat) (x : Nat) :
    (xs ++ [x]).eraseDups = xs.eraseDups ++ (if x ∈ xs then [] else [x]) := by
  rw [List.eraseDups_append]
  by_cases h : x ∈ xs
  · simp [h, List.removeAll]
  · simp [h, List.removeAll, List.eraseDups_cons]

theorem eraseDups_nodup (xs : List Nat) : xs.eraseDups.Nodup := by
  induction xs using List.reverseRecOn with
  | nil => simp
  | append_singleton xs x ih =>
    rw [eraseDups_snoc]; split
    · simpa using ih
    · rename_i h
      exact List.nodup_append.2 ⟨ih, by simp, by simp; intro a ha hax; exact h (hax ▸ ha)⟩

/-- The three vectors of `Closes` hold the table `L` (faces, each once, with their corners). -/
structure CRep (cf : alloc.vec.Vec Usize) (cl : alloc.vec.Vec (Usize × Usize)) (cs : alloc.vec.Vec Usize)
    (L : List (Nat × List (Nat × Nat))) : Prop where
  cf : cf.val.map (·.val) = L.map (·.1)
  cl : cl.val.map (fun p => (p.1.val, p.2.val)) = L.flatMap (·.2)
  cs_len : cs.val.length = L.length + 1
  cs : ∀ k ≤ L.length, (cs.val[k]!).val = ((L.take k).flatMap (·.2)).length
  nodup : (L.map (·.1)).Nodup

section before
variable {T : Type} (FI : topo_fans.FanTopo T) (t : T) {b : Body} {corners : Nat → Nat → List (Nat × Nat)}

theorem filter_face_nodup (L : List (Nat × List (Nat × Nat))) (hn : (L.map (·.1)).Nodup) (k : Nat)
    (hk : k < L.length) : L.filter (fun x => x.1 == L[k].1) = [L[k]] := by
  induction L generalizing k with
  | nil => simp at hk
  | cons y ys ih =>
    simp only [List.map_cons, List.nodup_cons, List.mem_map] at hn
    cases k with
    | zero =>
      simp only [List.getElem_cons_zero, List.filter_cons, beq_self_eq_true, ite_true, List.cons.injEq, true_and]
      rw [List.filter_eq_nil_iff]
      intro a ha h; simp at h; exact hn.1 ⟨a, ha, h⟩
    | succ k =>
      simp only [List.getElem_cons_succ, List.filter_cons]
      have hk' : k < ys.length := by simpa using hk
      have hne : (y.1 == ys[k].1) = false := by
        simp; intro h; exact hn.1 ⟨ys[k], List.getElem_mem hk', h.symm⟩
      rw [hne]; simp only [Bool.false_eq_true, ite_false]; exact ih hn.2 k hk'

theorem filter_face_none (L : List (Nat × List (Nat × Nat))) (f : Nat) (h : f ∉ L.map (·.1)) :
    L.filter (fun x => x.1 == f) = [] := by
  rw [List.filter_eq_nil_iff]; intro a ha hf; simp at hf; exact h (List.mem_map.2 ⟨a, ha, hf⟩)

theorem seg_flatMap (L : List (Nat × List (Nat × Nat))) (k : Nat) (hk : k < L.length) :
    ((L.flatMap (·.2)).drop ((L.take k).flatMap (·.2)).length).take (L[k].2.length) = L[k].2 := by
  have h : L.flatMap (·.2) = (L.take k).flatMap (·.2) ++ (L[k].2 ++ (L.drop (k + 1)).flatMap (·.2)) := by
    conv_lhs => rw [← List.take_append_drop k L]
    rw [List.flatMap_append, List.drop_eq_getElem_cons hk, List.flatMap_cons]
  rw [h, List.drop_left, List.take_left]

theorem inner_spec (cl : alloc.vec.Vec (Usize × Usize)) (c : Usize) (P : List (Nat × Nat))
    (j0 e : Usize) (hseg : ((cl.val.map (fun p => (p.1.val, p.2.val))).drop j0.val).take (e.val - j0.val) = P)
    (hje : j0.val ≤ e.val) (he : e.val ≤ cl.val.length) (into : Usize) :
    topo_fans.before_loop0_loop0 cl c into false j0 e ⦃ r =>
      (r.2 = true ∧ P.find? (·.1 == c.val) = some (c.val, r.1.val)) ∨
      (r.2 = false ∧ P.find? (·.1 == c.val) = none) ⦄ := by
  have hPl : P.length = e.val - j0.val := by rw [← hseg]; simp; omega
  unfold topo_fans.before_loop0_loop0
  apply loop.spec_decr_nat
    (fun (y : Usize × Bool × Usize) => e.val - y.2.2.val + (if y.2.1 then 0 else 1))
    (fun y => j0.val ≤ y.2.2.val ∧ y.2.2.val ≤ e.val ∧
      ((y.2.1 = true ∧ P.find? (·.1 == c.val) = some (c.val, y.1.val)) ∨
       (y.2.1 = false ∧ (P.take (y.2.2.val - j0.val)).find? (·.1 == c.val) = none)))
  · rintro ⟨into0, found, j⟩ ⟨hj0, hje', hinv⟩
    simp only at hj0 hje' hinv ⊢
    unfold topo_fans.before_loop0_loop0.body
    split
    · rename_i hf
      simp only [WP.spec_ok]
      rcases hinv with ⟨-, hq⟩ | ⟨hf', -⟩
      · exact Or.inl ⟨hf, hq⟩
      · rw [hf] at hf'; simp at hf'
    · rename_i hf
      have hf' : found = false := by simpa using hf
      rcases hinv with ⟨h1, -⟩ | ⟨-, hq⟩
      · rw [hf'] at h1; simp at h1
      split
      · rename_i hlt
        have hlt' : j.val < e.val := by scalar_tac
        have hjl : j.val < cl.val.length := by omega
        step as ⟨x, y, hxy⟩
        have hPj : P[j.val - j0.val]'(by omega) = (x.val, y.val) := by
          subst hseg
          simp only [List.getElem_take, List.getElem_drop, List.getElem_map]
          simp only [show j0.val + (j.val - j0.val) = j.val by omega, ← hxy]
        have htk : P.take (j.val + 1 - j0.val) = P.take (j.val - j0.val) ++ [(x.val, y.val)] := by
          rw [show j.val + 1 - j0.val = (j.val - j0.val) + 1 by omega,
            List.take_succ_eq_append_getElem (by omega), hPj]
        have h1 : (1#usize).val = 1 := rfl
        apply WP.spec_bind (Pₘ := fun (r : Usize × Bool) =>
          (r.2 = true ∧ r.1.val = y.val ∧ x.val = c.val) ∨ (r.2 = false ∧ x.val ≠ c.val))
        · split
          · rename_i hx; simp only [WP.spec_ok]; exact Or.inl ⟨trivial, by simp, by rw [hx]⟩
          · rename_i hx; simp only [WP.spec_ok]
            exact Or.inr ⟨hf', fun h => hx (UScalar.eq_of_val_eq h)⟩
        · rintro ⟨into1, found1⟩ hr
          step with usize_add_wspec j 1#usize (by have := le_max e; rw [h1]; omega) as ⟨j1, hj1⟩
          have hj1v : j1.val = j.val + 1 := by rw [hj1]; exact succ_val j e hlt'
          rw [hj1v]
          rcases hr with ⟨hf1, hi1, hx⟩ | ⟨hf1, hx⟩
          · have hf1' : found1 = true := hf1
            refine ⟨by omega, by omega, Or.inl ⟨hf1, ?_⟩, by rw [if_pos hf1']; omega⟩
            rw [← List.take_append_drop (j.val + 1 - j0.val) P, List.find?_append, htk, List.find?_append, hq]
            simp [hx, hi1]
          · have hf1' : found1 = false := hf1
            refine ⟨by omega, by omega, Or.inr ⟨hf1, ?_⟩, by rw [if_neg (by simp [hf1'])]; omega⟩
            rw [htk, List.find?_append, hq]; simp; exact fun h => hx h
      · rename_i hlt
        have hje2 : j.val = e.val := by scalar_tac
        simp only [WP.spec_ok]
        refine Or.inr ⟨hf', ?_⟩
        rw [hje2, ← hPl, List.take_length] at hq; exact hq
  · have h0 : j0.val - j0.val = 0 := by omega
    refine ⟨le_refl _, hje, Or.inr ⟨rfl, by rw [h0]; simp⟩⟩

theorem find_seg_at (L : List (Nat × List (Nat × Nat))) (hn : (L.map (·.1)).Nodup) (c k : Nat)
    (hk : k < L.length) :
    ((L.filter (fun x => x.1 == L[k].1)).flatMap (·.2)).find? (·.1 == c) = L[k].2.find? (·.1 == c) := by
  rw [filter_face_nodup L hn k hk]; simp

theorem find_seg_none (L : List (Nat × List (Nat × Nat))) (f c : Nat) (hf : f ∉ L.map (·.1)) :
    ((L.filter (fun x => x.1 == f)).flatMap (·.2)).find? (·.1 == c) = none := by
  rw [filter_face_none L f hf]; rfl

theorem outer_spec (cf : alloc.vec.Vec Usize) (cl : alloc.vec.Vec (Usize × Usize)) (cs : alloc.vec.Vec Usize)
    (L : List (Nat × List (Nat × Nat))) (hR : CRep cf cl cs L) (c f nf : Usize) (hnf : nf.val = cf.val.length) :
    topo_fans.before_loop0 cf cl cs c 0#usize false f nf 0#usize ⦃ r =>
      (r.2 = true ∧ ((L.filter (fun x => x.1 == f.val)).flatMap (·.2)).find? (·.1 == c.val) = some (c.val, r.1.val)) ∨
      (r.2 = false ∧ ((L.filter (fun x => x.1 == f.val)).flatMap (·.2)).find? (·.1 == c.val) = none) ⦄ := by
  have hcfl : cf.val.length = L.length := by have := congrArg List.length hR.cf; simpa using this
  have hcll : cl.val.length = (L.flatMap (·.2)).length := by have := congrArg List.length hR.cl; simpa using this
  have hcfk : ∀ k (hk : k < L.length), (cf.val[k]'(by omega)).val = L[k].1 := by
    intro k hk; have := congrArg (fun l => l[k]?) hR.cf
    simp [List.getElem?_eq_getElem hk, List.getElem?_eq_getElem (show k < cf.val.length by omega)] at this
    exact this
  have hlen : nf.val = L.length := by omega
  have hnfm := le_max nf
  unfold topo_fans.before_loop0
  apply loop.spec_decr_nat
    (fun (y : Usize × Bool × Usize) => L.length - y.2.2.val)
    (fun y => y.2.2.val ≤ L.length ∧
      ((y.2.2.val = L.length ∧ ((y.2.1 = true ∧ ((L.filter (fun x => x.1 == f.val)).flatMap (·.2)).find? (·.1 == c.val) = some (c.val, y.1.val)) ∨
        (y.2.1 = false ∧ ((L.filter (fun x => x.1 == f.val)).flatMap (·.2)).find? (·.1 == c.val) = none))) ∨
       (y.2.1 = false ∧ f.val ∉ (L.take y.2.2.val).map (·.1))))
  · rintro ⟨into0, found, k⟩ ⟨hle, hinv⟩
    simp only at hle hinv ⊢
    unfold topo_fans.before_loop0.body
    split
    · rename_i hlt
      have hlt' : k.val < L.length := by rw [← hlen]; scalar_tac
      rcases hinv with ⟨hkl, -⟩ | ⟨hf0, hnot⟩
      · omega
      step as ⟨fk, hfk⟩
      have hfkv : fk.val = L[k.val].1 := by rw [hfk]; exact hcfk k.val hlt'
      have htake : (L.take (k.val + 1)).map (·.1) = (L.take k.val).map (·.1) ++ [L[k.val].1] := by
        rw [List.take_succ_eq_append_getElem hlt']; simp only [List.map_append, List.map_cons, List.map_nil]
      split
      · rename_i hff
        have hffv : L[k.val].1 = f.val := by rw [← hfkv, hff]
        have hcsl := hR.cs_len
        step as ⟨j0, hj0⟩
        have h1 : (1#usize).val = 1 := rfl
        step with usize_add_wspec k 1#usize (by rw [h1]; omega) as ⟨k1, hk1⟩
        have hk1v : k1.val = k.val + 1 := by rw [hk1]; exact add_val k 1#usize (lt_size_of_le_max _ (by rw [h1]; omega))
        step as ⟨e, he⟩
        have hj0v : j0.val = ((L.take k.val).flatMap (·.2)).length := by
          rw [hj0, ← getElem!_pos cs.val k.val (by omega)]; exact hR.cs k.val (by omega)
        have hev : e.val = ((L.take (k.val + 1)).flatMap (·.2)).length := by
          rw [he, ← getElem!_pos cs.val k1.val (by omega), hR.cs k1.val (by omega), hk1v]
        have hlen2 : e.val = j0.val + L[k.val].2.length := by
          rw [hev, hj0v, List.take_succ_eq_append_getElem hlt', List.flatMap_append, List.length_append]; simp
        have hseg : ((cl.val.map (fun p => (p.1.val, p.2.val))).drop j0.val).take (e.val - j0.val) = L[k.val].2 := by
          rw [hR.cl, hj0v, show e.val - _ = L[k.val].2.length by omega]; exact seg_flatMap L k.val hlt'
        have hcl2 : e.val ≤ cl.val.length := by
          rw [hcll, hev]
          conv_rhs => rw [← List.take_append_drop (k.val + 1) L]
          rw [List.flatMap_append, List.length_append]; omega
        rw [hf0]
        step with inner_spec cl c L[k.val].2 j0 e hseg (by omega) hcl2 into0 as ⟨into1, found1, hr⟩
        refine ⟨by omega, Or.inl ⟨by omega, ?_⟩, by omega⟩
        rw [← hffv, find_seg_at L hR.nodup c.val k.val hlt']
        exact hr
      · rename_i hff
        have h1 : (1#usize).val = 1 := rfl
        step with usize_add_wspec k 1#usize (by rw [h1]; omega) as ⟨k1, hk1⟩
        have hk1v : k1.val = k.val + 1 := by rw [hk1]; exact add_val k 1#usize (lt_size_of_le_max _ (by rw [h1]; omega))
        rw [hk1v]
        refine ⟨by omega, Or.inr ⟨hf0, ?_⟩, by omega⟩
        rw [htake]; simp only [List.mem_append, List.mem_singleton, not_or]
        exact ⟨hnot, fun h => hff (UScalar.eq_of_val_eq (by rw [hfkv, h]))⟩
    · rename_i hlt
      have hk : k.val = L.length := by rw [← hlen]; scalar_tac
      simp only [WP.spec_ok]
      rcases hinv with ⟨-, h⟩ | ⟨hf0, hnot⟩
      · exact h
      · rw [hk, List.take_length] at hnot
        exact Or.inr ⟨hf0, find_seg_none L f.val c.val hnot⟩
  · simp

end before

section beforeS
variable {T : Type} (FI : topo_fans.FanTopo T) (t : T) {b : Body} {corners : Nat → Nat → List (Nat × Nat)}

theorem before_none_spec (R : FanReads FI t b corners) (H : Sound b) (W : b.WF) (closes : topo_fans.Closes) :
    BefSpec FI t (b := b) closes false (b.before none) := by
  intro c hc
  unfold topo_fans.before
  simp only [Bool.false_eq_true, ite_false]
  exact prev_of_spec FI t R H W c hc

theorem before_some_spec (R : FanReads FI t b corners) (H : Sound b) (W : b.WF) (closes : topo_fans.Closes)
    (L : List (Nat × List (Nat × Nat))) (hR : CRep closes.cf closes.cl closes.cs L) :
    BefSpec FI t (b := b) closes true (b.before (some L)) := by
  intro c hc
  obtain ⟨hcn, -, -, hl, -, -⟩ := use_facts H W hc
  unfold topo_fans.before
  simp only [ite_true]
  step with face_of_spec FI t R c hcn hl as ⟨f, hf⟩
  step with outer_spec closes.cf closes.cl closes.cs L hR c f (alloc.vec.Vec.len closes.cf) (by simp) as ⟨into, found, hr⟩
  unfold Body.before
  simp only
  rw [← hf]
  rcases hr with ⟨hft, hq⟩ | ⟨hff, hq⟩
  · rw [hq, hft]; simp
  · rw [hq, hff]; simp only [Bool.false_eq_true, ite_false]
    exact prev_of_spec FI t R H W c hc

end beforeS

/-! ## Building the corner table -/

section table
variable {T : Type} (FI : topo_fans.FanTopo T) (t : T) {b : Body} {corners : Nat → Nat → List (Nat × Nat)}

/-- The contract of the view's `corners` (body.rs `corners`): each entry pairs a distinct coedge of
face `f` (its leaving uses) with a use closing its corner. -/
def CornersOK (b : Body) (corners : Nat → Nat → List (Nat × Nat)) : Prop :=
  ∀ f v, ((corners f v).map (·.1)).Nodup ∧
    ∀ p ∈ corners f v, p.1 < b.coedges.size ∧ b.face p.1 = f ∧ Use b p.2

/-- The table for faces `fs` at `v`. -/
def tableOf (corners : Nat → Nat → List (Nat × Nat)) (v : Nat) (fs : List Nat) : List (Nat × List (Nat × Nat)) :=
  fs.map fun f => (f, corners f v)

theorem table_firsts_nodup (hC : CornersOK b corners) (v : Nat) (fs : List Nat) (hfs : fs.Nodup) :
    (((tableOf corners v fs).flatMap (·.2)).map (·.1)).Nodup ∧
    ∀ x ∈ ((tableOf corners v fs).flatMap (·.2)).map (·.1), x < b.coedges.size ∧ b.face x ∈ fs := by
  induction fs using List.reverseRecOn with
  | nil => simp [tableOf]
  | append_singleton fs f ih =>
    have hfs' : fs.Nodup := (List.nodup_append.1 hfs).1
    have hf : f ∉ fs := by
      intro h; have := (List.nodup_append.1 hfs).2.2; simp at this; exact this f h rfl
    obtain ⟨ih1, ih2⟩ := ih hfs'
    simp only [tableOf, List.map_append, List.map_cons, List.map_nil, List.flatMap_append,
      List.flatMap_cons, List.flatMap_nil, List.append_nil] at ih1 ih2 ⊢
    obtain ⟨hn, hp⟩ := hC f v
    refine ⟨?_, ?_⟩
    · refine List.nodup_append.2 ⟨ih1, hn, ?_⟩
      intro x hx y hy hxy
      subst hxy
      obtain ⟨-, hxf⟩ := ih2 x hx
      obtain ⟨p, hp', rfl⟩ := List.mem_map.1 hy
      rw [(hp p hp').2.1] at hxf; exact hf hxf
    · intro x hx
      rw [List.mem_append] at hx
      rcases hx with hx | hx
      · obtain ⟨h1, h2⟩ := ih2 x hx; exact ⟨h1, List.mem_append_left _ h2⟩
      · obtain ⟨p, hp', rfl⟩ := List.mem_map.1 hx
        exact ⟨(hp p hp').1, by rw [(hp p hp').2.1]; simp⟩

theorem table_len (hC : CornersOK b corners) (v : Nat) (fs : List Nat) (hfs : fs.Nodup) :
    ((tableOf corners v fs).flatMap (·.2)).length ≤ b.coedges.size := by
  obtain ⟨h1, h2⟩ := table_firsts_nodup hC v fs hfs
  have := (h1.subperm fun x hx => List.mem_range.2 (h2 x hx).1).length_le
  simpa using this

theorem append_spec (v cr : alloc.vec.Vec (Usize × Usize)) (m : Usize) (hm : m.val = cr.val.length)
    (hb : v.val.length + cr.val.length ≤ Usize.max) :
    topo_fans.n_fans_loop0_loop0 v cr m 0#usize ⦃ r => r.val = v.val ++ cr.val ⦄ := by
  unfold topo_fans.n_fans_loop0_loop0
  apply loop.spec_decr_nat (fun (y : alloc.vec.Vec (Usize × Usize) × Usize) => m.val - y.2.val)
    (fun y => y.2.val ≤ m.val ∧ y.1.val = v.val ++ cr.val.take y.2.val)
  · rintro ⟨w, j⟩ ⟨hle, hw⟩
    simp only at hle hw ⊢
    unfold topo_fans.n_fans_loop0_loop0.body
    split
    · rename_i hlt
      have hlt' : j.val < m.val := by scalar_tac
      step as ⟨p, hp⟩
      have hwl : w.val.length < Usize.max := by rw [hw]; simp; omega
      step with alloc.vec.Vec.push_spec w p hwl as ⟨w1, hw1⟩
      step with usize_add_wspec j 1#usize (succ_le_max j m hlt') as ⟨j1, hj1⟩
      have hj1v : j1.val = j.val + 1 := by rw [hj1]; exact succ_val j m hlt'
      rw [hj1v]
      refine ⟨by omega, ?_, by omega⟩
      rw [hw1, hw, List.take_succ_eq_append_getElem (by omega), hp]; simp
    · rename_i hlt
      have hj : j.val = m.val := by scalar_tac
      simp only [WP.spec_ok]
      rw [hw, hj, hm, List.take_length]
  · have h0 : (0#usize).val = 0 := rfl
    simp [h0]

/-- The faces of the coedges `S`, each once. -/
def facesOf (b : Body) (S : List Nat) : List Nat := (S.map b.face).eraseDups

theorem face_lt (W : b.WF) (H : Sound b) {c : Nat} (hc : Use b c) : b.face c < b.faces.size := by
  obtain ⟨-, -, -, hl, -, -⟩ := use_facts H W hc
  exact (W.listed_le _ hl).1

theorem facesOf_len (W : b.WF) (H : Sound b) (S : List Nat) (hS : ∀ x ∈ S, Use b x) :
    (facesOf b S).length ≤ b.faces.size := by
  have := ((eraseDups_nodup (S.map b.face)).subperm fun x hx => by
    rw [List.mem_eraseDups, List.mem_map] at hx
    obtain ⟨c, hc, rfl⟩ := hx
    exact List.mem_range.2 (face_lt W H (hS c hc))).length_le
  simpa [facesOf] using this

theorem crep_snoc {cf : alloc.vec.Vec Usize} {cl : alloc.vec.Vec (Usize × Usize)} {cs : alloc.vec.Vec Usize}
    {L : List (Nat × List (Nat × Nat))} (hR : CRep cf cl cs L) (f : Usize) (cr : alloc.vec.Vec (Usize × Usize))
    (cf1 : alloc.vec.Vec Usize) (cl1 : alloc.vec.Vec (Usize × Usize)) (cs1 : alloc.vec.Vec Usize)
    (hcf1 : cf1.val = cf.val ++ [f]) (hcl1 : cl1.val = cl.val ++ cr.val) (n1 : Usize) (hn1 : n1.val = cl1.val.length)
    (hcs1 : cs1.val = cs.val ++ [n1]) (hfn : f.val ∉ L.map (·.1)) :
    CRep cf1 cl1 cs1 (L ++ [(f.val, cr.val.map (fun p => (p.1.val, p.2.val)))]) := by
  have hcll : cl.val.length = (L.flatMap (·.2)).length := by have := congrArg List.length hR.cl; simpa using this
  refine ⟨by rw [hcf1]; simp [hR.cf], by rw [hcl1]; simp [hR.cl], by rw [hcs1]; simp [hR.cs_len], ?_, ?_⟩
  · intro k hk
    simp only [List.length_append, List.length_singleton] at hk
    rw [hcs1]
    by_cases hkL : k ≤ L.length
    · rw [getElem!_append_left' _ _ _ (by rw [hR.cs_len]; omega), hR.cs k hkL, List.take_append_of_le_length hkL]
    · have hk' : k = L.length + 1 := by omega
      subst hk'
      rw [show L.length + 1 = cs.val.length by rw [hR.cs_len], getElem!_append_len, hn1, hcl1]
      simp [hcll, hR.cs_len]
  · simp only [List.map_append, List.map_cons, List.map_nil]
    refine List.nodup_append.2 ⟨hR.nodup, by simp, ?_⟩
    intro x hx y hy hxy
    simp at hy; subst hy; subst hxy; exact hfn hx

theorem table_spec (R : FanReads FI t b corners) (H : Sound b) (W : b.WF) (hC : CornersOK b corners)
    (seen : alloc.vec.Vec Usize) (ns vtx : Usize) (hns : ns.val ≤ seen.val.length)
    (hS : SeenOK b (seenOf seen ns.val)) (cs0 : alloc.vec.Vec Usize) (hcs0 : cs0.val = [0#usize]) :
    topo_fans.n_fans_loop0 FI t seen (alloc.vec.Vec.new Usize) (alloc.vec.Vec.new (Usize × Usize))
      cs0 vtx false ns 0#usize ⦃ r =>
      CRep r.1 r.2.1 r.2.2.1 (tableOf corners vtx.val (facesOf b (seenOf seen ns.val))) ∧
      r.2.2.2 = !((tableOf corners vtx.val (facesOf b (seenOf seen ns.val))).all (·.2.isEmpty)) ⦄ := by
  set S := seenOf seen ns.val with hSdef
  have hSl : S.length = ns.val := by simp [hSdef, seenOf]; omega
  have hnf := R.nf_lt
  have hu32 : U32.max ≤ Usize.max := by
    have := succ_lt_usize (U32.max - 1) (by rw [u32max_eq]; omega)
    have := usize_le_max_of_lt _ this; rw [u32max_eq] at *; omega
  obtain ⟨mc, hmc⟩ := WP.spec_exists R.base.nc
  have hmcx := le_max mc
  unfold topo_fans.n_fans_loop0
  apply loop.spec_decr_nat
    (fun (y : alloc.vec.Vec Usize × alloc.vec.Vec (Usize × Usize) × alloc.vec.Vec Usize × Bool × Usize) => ns.val - y.2.2.2.2.val)
    (fun y => y.2.2.2.2.val ≤ ns.val ∧
      CRep y.1 y.2.1 y.2.2.1 (tableOf corners vtx.val (facesOf b (S.take y.2.2.2.2.val))) ∧
      y.2.2.2.1 = !((tableOf corners vtx.val (facesOf b (S.take y.2.2.2.2.val))).all (·.2.isEmpty)))
  · rintro ⟨cf, cl, cs, any, k⟩ ⟨hle, hR, hany⟩
    simp only at hle hR hany ⊢
    unfold topo_fans.n_fans_loop0.body
    split
    · rename_i hlt
      have hlt' : k.val < ns.val := by scalar_tac
      have hkS : k.val < S.length := by omega
      step as ⟨c, hc⟩
      have hcv : c.val = S[k.val] := by simp [hSdef, seenOf, hc]
      have hcu : Use b c.val := by rw [hcv]; exact hS.2 _ (List.getElem_mem hkS)
      obtain ⟨hcn, -, -, hl, -, -⟩ := use_facts H W hcu
      step with face_of_spec FI t R c hcn hl as ⟨f, hf⟩
      have hderef : (alloc.vec.Vec.deref cf).val = cf.val := by simp [alloc.vec.Vec.deref]
      step with contains_spec (alloc.vec.Vec.deref cf) (alloc.vec.Vec.len cf) f (by rw [hderef]; simp) as ⟨isin, hisin⟩
      rw [hderef] at hisin
      have htake : S.take (k.val + 1) = S.take k.val ++ [c.val] := by
        rw [List.take_succ_eq_append_getElem hkS, hcv]
      have hfac : facesOf b (S.take (k.val + 1)) = facesOf b (S.take k.val) ++
          (if b.face c.val ∈ (S.take k.val).map b.face then [] else [b.face c.val]) := by
        rw [htake]; unfold facesOf; rw [List.map_append, List.map_singleton, eraseDups_snoc]
      have hcfm : (cf.val.map (·.val)) = facesOf b (S.take k.val) := by
        rw [hR.cf]; simp [tableOf, Function.comp_def]
      have hmem : (f ∈ List.take (alloc.vec.Vec.len cf).val cf.val) ↔ b.face c.val ∈ (S.take k.val).map b.face := by
        rw [show (alloc.vec.Vec.len cf).val = cf.val.length by simp, List.take_length, ← hf]
        constructor
        · intro h; have := List.mem_map_of_mem (f := (·.val)) h; rw [hcfm] at this
          unfold facesOf at this; exact List.mem_eraseDups.1 this
        · intro h
          have : f.val ∈ cf.val.map (·.val) := by rw [hcfm]; unfold facesOf; rw [List.mem_eraseDups]; exact h
          obtain ⟨g, hg, hgf⟩ := List.mem_map.1 this
          rwa [← UScalar.eq_of_val_eq hgf]
      have hfacS : ∀ x ∈ S.take k.val, Use b x := fun x hx => hS.2 x (List.mem_of_mem_take hx)
      have hflen := facesOf_len W H (S.take (k.val + 1)) (fun x hx => hS.2 x (List.mem_of_mem_take hx))
      split
      · rename_i hin
        have hin' : b.face c.val ∈ (S.take k.val).map b.face := hmem.1 (by simpa [hisin] using hin)
        step with usize_add_wspec k 1#usize (succ_le_max k ns hlt') as ⟨k1, hk1⟩
        have hk1v : k1.val = k.val + 1 := by rw [hk1]; exact succ_val k ns hlt'
        rw [hk1v, hfac, if_pos hin', List.append_nil]
        exact ⟨by omega, hR, hany, by omega⟩
      · rename_i hin
        have hin' : b.face c.val ∉ (S.take k.val).map b.face := fun h => hin (by simpa [hisin] using hmem.2 h)
        rw [hfac, if_neg hin'] at hflen
        simp only [List.length_append, List.length_singleton] at hflen
        have hcfl : cf.val.length = (facesOf b (S.take k.val)).length := by
          have := congrArg List.length hcfm; simpa using this
        step with alloc.vec.Vec.push_spec cf f (by omega) as ⟨cf1, hcf1⟩
        step with R.corners f vtx as ⟨cr, hcr⟩
        have hnd := eraseDups_nodup ((S.take (k.val + 1)).map b.face)
        have htl := table_len hC vtx.val (facesOf b (S.take (k.val + 1))) hnd
        rw [hfac, if_neg hin'] at htl
        simp only [tableOf, List.map_append, List.map_cons, List.map_nil, List.flatMap_append,
          List.flatMap_cons, List.flatMap_nil, List.append_nil, List.length_append] at htl
        have hcll : cl.val.length = ((tableOf corners vtx.val (facesOf b (S.take k.val))).flatMap (·.2)).length := by
          have := congrArg List.length hR.cl; simpa using this
        have hcrl : cr.val.length = (corners f.val vtx.val).length := by
          have := congrArg List.length hcr; simpa using this
        apply WP.spec_bind (Pₘ := fun any1 : Bool => any1 = (any || decide (0 < cr.val.length)))
        · split
          · rename_i hm; simp only [WP.spec_ok]; simp; right; scalar_tac
          · rename_i hm; simp only [WP.spec_ok]
            have : ¬ 0 < cr.val.length := by scalar_tac
            simp [this]
        intro any1 hany1
        step with append_spec cl cr (alloc.vec.Vec.len cr) (by simp)
          (by rw [hcll, hcrl]; simp only [tableOf] at htl ⊢; rw [← hf] at htl; omega) as ⟨cl1, hcl1⟩
        step with alloc.vec.Vec.push_spec cs (alloc.vec.Vec.len cl1)
          (by rw [hR.cs_len]; simp [tableOf]; omega) as ⟨cs1, hcs1⟩
        step with usize_add_wspec k 1#usize (succ_le_max k ns hlt') as ⟨k1, hk1⟩
        have hk1v : k1.val = k.val + 1 := by rw [hk1]; exact succ_val k ns hlt'
        rw [hk1v, hfac, if_neg hin']
        have hfnot : f.val ∉ (tableOf corners vtx.val (facesOf b (S.take k.val))).map (·.1) := by
          simp only [tableOf, List.map_map]; simp only [Function.comp_def, List.map_id']
          unfold facesOf; rw [List.mem_eraseDups, hf]; exact hin'
        have hrep := crep_snoc hR f cr cf1 cl1 cs1 hcf1 hcl1 (alloc.vec.Vec.len cl1) (by simp) hcs1 hfnot
        rw [hcr, hf] at hrep
        refine ⟨by omega, ?_, ?_, by omega⟩
        · simpa [tableOf] using hrep
        · rw [hany1, hany]; simp [tableOf, hcrl, List.all_append]
          rw [hf]; cases (corners (b.face c.val) vtx.val) <;> simp
    · rename_i hlt
      have hk : k.val = ns.val := by scalar_tac
      simp only [WP.spec_ok]
      rw [hk, ← hSl, List.take_length] at hR hany
      exact ⟨hR, hany⟩
  · have h0 : (0#usize).val = 0 := rfl
    simp only [h0, List.take_zero]
    refine ⟨by omega, ⟨by simp [tableOf, facesOf], by simp [tableOf, facesOf], by simp [hcs0, tableOf, facesOf], ?_, by simp [tableOf, facesOf]⟩, by simp [tableOf, facesOf]⟩
    intro k hk; simp [tableOf, facesOf] at hk; subst hk; simp [hcs0]

end table

/-! ## The guard -/

section guard
variable {T : Type} (FI : topo_fans.FanTopo T) (t : T) {b : Body} {corners : Nat → Nat → List (Nat × Nat)}

theorem used_spec (R : FanReads FI t b corners) (a e nu : Usize) (he : e.val < b.edges.size)
    (hnu : nu.val = (b.ed e.val).uses.length) :
    topo_fans.n_fans_loop1_loop0 FI t a e false nu 0#usize ⦃ r => r = decide (a.val ∈ (b.ed e.val).uses) ⦄ := by
  set U := (b.ed e.val).uses
  unfold topo_fans.n_fans_loop1_loop0
  apply loop.spec_decr_nat (fun (y : Bool × Usize) => nu.val - y.2.val)
    (fun y => y.2.val ≤ nu.val ∧ y.1 = decide (a.val ∈ U.take y.2.val))
  · rintro ⟨u, j⟩ ⟨hle, hu⟩
    simp only at hle hu ⊢
    unfold topo_fans.n_fans_loop1_loop0.body
    split
    · rename_i hlt
      have hlt' : j.val < nu.val := by scalar_tac
      have hjU : j.val < U.length := by omega
      step with R.base.edge_use e j he hjU as ⟨o, ho⟩
      have hov : o.val = U[j.val] := by rw [ho, getElem!_pos U j.val hjU]
      have htk : U.take (j.val + 1) = U.take j.val ++ [U[j.val]] := List.take_succ_eq_append_getElem hjU
      apply WP.spec_bind (Pₘ := fun u1 : Bool => u1 = decide (a.val ∈ U.take (j.val + 1)))
      · split
        · rename_i hoa; simp only [WP.spec_ok]; rw [htk, ← hov, hoa]; simp
        · rename_i hoa; simp only [WP.spec_ok]; rw [hu, htk, ← hov]
          have : a.val ≠ o.val := fun h => hoa (UScalar.eq_of_val_eq h).symm
          simp [this]
      · intro u1 hu1
        step with usize_add_wspec j 1#usize (succ_le_max j nu hlt') as ⟨j1, hj1⟩
        have hj1v : j1.val = j.val + 1 := by rw [hj1]; exact succ_val j nu hlt'
        rw [hj1v]; exact ⟨by omega, hu1, by omega⟩
    · rename_i hlt
      have hj : j.val = nu.val := by scalar_tac
      simp only [WP.spec_ok]
      rw [hu, hj, hnu, List.take_length]
  · have h0 : (0#usize).val = 0 := rfl
    simp [h0]

/-- The guard's test of one coedge. -/
def guardP (b : Body) (bef : Nat → Nat) (v c : Nat) : Bool :=
  bef c < b.coedges.size && b.stop (bef c) == v && (b.co (bef c)).edge < b.edges.size &&
    (b.ed (b.co (bef c)).edge).uses.contains (bef c)

theorem closesOk_eq (b : Body) (bef : Nat → Nat) (frm : List Nat) :
    b.closesOk bef frm = (frm.all (guardP b bef (b.start (frm.headD 0))) && decide (frm.map bef).Nodup) := by
  unfold Body.closesOk guardP; rfl

set_option hygiene false in
/-- The guard loop's step to the next coedge, from what this one gave (`hT`, `hF`). -/
macro "guard_next" : tactic => `(tactic| (
  step with usize_add_wspec i 1#usize (succ_le_max i hi hlt') as ⟨i2, hi2⟩
  have hi2v : i2.val = i.val + 1 := by rw [hi2]; exact succ_val i hi hlt'
  rw [hi2v]
  refine ⟨by omega, by omega, fun h => ?_, fun h => ?_, ?_⟩
  rotate_left 2
  · have h1 := hi2v; have h2 := hlt'; (try rw [hok]); split <;> simp <;> omega
  · obtain ⟨hg, hnm, hi1⟩ := hT (by simpa using h)
    refine ⟨by rw [hpre, List.all_append, hall]; simp [hg], ?_, by rw [hpre, List.map_append, hi1]; simp⟩
    rw [hpre, List.map_append]
    refine List.nodup_append.2 ⟨hnd, by simp, ?_⟩
    intro x hx y hy hxy; simp at hy; subst hy; subst hxy; exact hnm hx
  · rcases hF (by simpa using h) with hg | hm
    · simp only [Bool.and_eq_false_iff]; left
      apply hwhole _ _ (i.val + 1 - lo.val); rw [hpre, List.all_append]; simp [hg]
    · simp only [Bool.and_eq_false_iff]; right; simp only [decide_eq_false_iff_not]
      apply hwnd frm (i.val + 1 - lo.val); rw [hpre, List.map_append]
      intro hn; exact (List.nodup_append.1 hn).2.2 _ hm _ (by simp) rfl))

theorem guard_spec (R : FanReads FI t b corners) (H : Sound b) (W : b.WF)
    (closes : topo_fans.Closes) (bef : Nat → Nat) (hB : BefSpec FI t (b := b) closes true bef)
    (items : Slice Usize) (lo hi : Usize) (hlo : lo.val ≤ hi.val) (hhi : hi.val ≤ items.val.length)
    (hfrm : ∀ x ∈ frmOf items lo.val hi.val, Use b x) (vtx nc ne : Usize)
    (hnc : nc.val = b.coedges.size) (hne : ne.val = b.edges.size) :
    topo_fans.n_fans_loop1 FI t items hi closes.cf closes.cl closes.cs vtx nc ne
      (alloc.vec.Vec.new Usize) true lo ⦃ r =>
      r = ((frmOf items lo.val hi.val).all (guardP b bef vtx.val) && decide ((frmOf items lo.val hi.val).map bef).Nodup) ⦄ := by
  set frm := frmOf items lo.val hi.val with hfrmdef
  have hfl : frm.length = hi.val - lo.val := by simp [hfrmdef, frmOf]; omega
  have hncx := le_max nc
  unfold topo_fans.n_fans_loop1
  apply loop.spec_decr_nat
    (fun (y : alloc.vec.Vec Usize × Bool × Usize) => hi.val - y.2.2.val + (if y.2.1 then 1 else 0))
    (fun y => lo.val ≤ y.2.2.val ∧ y.2.2.val ≤ hi.val ∧
      (y.2.1 = true → (frm.take (y.2.2.val - lo.val)).all (guardP b bef vtx.val) ∧
          ((frm.take (y.2.2.val - lo.val)).map bef).Nodup ∧
          y.1.val.map (·.val) = (frm.take (y.2.2.val - lo.val)).map bef) ∧
      (y.2.1 = false → ((frm.all (guardP b bef vtx.val) && decide (frm.map bef).Nodup) = false)))
  · rintro ⟨into, ok1, i⟩ ⟨hlo', hle, htrue, hfalse⟩
    simp only at hlo' hle htrue hfalse ⊢
    unfold topo_fans.n_fans_loop1.body
    have hwhole : ∀ (P : Nat → Bool) (L : List Nat) (k : Nat), (L.take k).all P = false → L.all P = false := by
      intro P L k h; rw [← List.take_append_drop k L, List.all_append, h]; rfl
    have hwnd : ∀ (L : List Nat) (k : Nat), ¬ ((L.take k).map bef).Nodup → ¬ (L.map bef).Nodup := by
      intro L k h hn; apply h; rw [List.map_take]; exact hn.sublist (List.take_sublist _ _)
    split
    · rename_i hok
      obtain ⟨hall, hnd, hinto⟩ := htrue hok
      split
      · rename_i hlt
        have hlt' : i.val < hi.val := by scalar_tac
        have hix : i.val < items.length := by have : items.length = items.val.length := rfl; omega
        step with Slice.index_usize_spec items i hix as ⟨c, hc⟩
        have hc' : c = items.val[i.val]! := by rw [hc, getElem!_pos]
        have hpre := frmOf_succ items lo.val i.val hi.val hlo' hlt' hhi
        rw [← hfrmdef, ← hc'] at hpre
        have hcu : Use b c.val := hfrm _ (by
          have : c.val ∈ frm.take (i.val + 1 - lo.val) := by rw [hpre]; simp
          exact List.mem_of_mem_take this)
        step with hB c hcu as ⟨a, ha⟩
        have h1 : (1#usize).val = 1 := rfl
        split
        · rename_i hge
          have hT : false = true → guardP b bef vtx.val c.val = true ∧ bef c.val ∉ (frm.take (i.val - lo.val)).map bef ∧
              into.val.map (·.val) = (frm.take (i.val - lo.val)).map bef ++ [bef c.val] := fun h => absurd h (by simp)
          have hF : false = false → (guardP b bef vtx.val c.val = false ∨ bef c.val ∈ (frm.take (i.val - lo.val)).map bef) := by
            intro _; left; unfold guardP; rw [← ha]; simp; intro h; scalar_tac
          guard_next
        · rename_i hge
          have hav : a.val < b.coedges.size := by rw [← hnc]; scalar_tac
          step with R.base.coedge_edge a hav as ⟨e, hev⟩
          split
          · rename_i hge2
            have hT : false = true → guardP b bef vtx.val c.val = true ∧ bef c.val ∉ (frm.take (i.val - lo.val)).map bef ∧
                into.val.map (·.val) = (frm.take (i.val - lo.val)).map bef ++ [bef c.val] := fun h => absurd h (by simp)
            have hF : false = false → (guardP b bef vtx.val c.val = false ∨ bef c.val ∈ (frm.take (i.val - lo.val)).map bef) := by
              intro _; left; unfold guardP; rw [← ha, ← hev]; simp; intro _ _ h; scalar_tac
            guard_next
          · rename_i hge2
            have hen : e.val < b.edges.size := by rw [← hne]; scalar_tac
            step with stop_spec FI t R a hav (by rw [← hev]; exact hen) as ⟨st, hst⟩
            split
            · rename_i hsv
              have hT : false = true → guardP b bef vtx.val c.val = true ∧ bef c.val ∉ (frm.take (i.val - lo.val)).map bef ∧
                  into.val.map (·.val) = (frm.take (i.val - lo.val)).map bef ++ [bef c.val] := fun h => absurd h (by simp)
              have hF : false = false → (guardP b bef vtx.val c.val = false ∨ bef c.val ∈ (frm.take (i.val - lo.val)).map bef) := by
                intro _; left; unfold guardP; rw [← ha, ← hst]
                have hsv2 : st ≠ vtx := by simpa using hsv
                have : st.val ≠ vtx.val := fun h => hsv2 (UScalar.eq_of_val_eq h)
                simp [this]
              guard_next
            · rename_i hsv
              have hsv' : st.val = vtx.val := by simpa using hsv
              step with R.base.edge_n_uses e hen as ⟨nu, hnu⟩
              step with used_spec FI t R a e nu hen hnu as ⟨used, hused⟩
              have hgP : guardP b bef vtx.val c.val = decide (a.val ∈ (b.ed e.val).uses) := by
                unfold guardP; rw [← ha, ← hst, hsv', ← hev]; simp [hav, hen]
              apply WP.spec_bind (Pₘ := fun (r : alloc.vec.Vec Usize × Bool) =>
                (r.2 = true → guardP b bef vtx.val c.val = true ∧ bef c.val ∉ (frm.take (i.val - lo.val)).map bef ∧
                  r.1.val.map (·.val) = (frm.take (i.val - lo.val)).map bef ++ [bef c.val]) ∧
                (r.2 = false → (guardP b bef vtx.val c.val = false ∨ bef c.val ∈ (frm.take (i.val - lo.val)).map bef)))
              · split
                · rename_i hu
                  have hderef : (alloc.vec.Vec.deref into).val = into.val := by simp [alloc.vec.Vec.deref]
                  step with contains_spec (alloc.vec.Vec.deref into) (alloc.vec.Vec.len into) a (by rw [hderef]; simp) as ⟨isin, hisin⟩
                  rw [hderef] at hisin
                  have hmemI : a ∈ into.val.take (alloc.vec.Vec.len into).val ↔ bef c.val ∈ (frm.take (i.val - lo.val)).map bef := by
                    rw [show (alloc.vec.Vec.len into).val = into.val.length by simp, List.take_length, ← ha, ← hinto]
                    constructor
                    · intro h; exact List.mem_map_of_mem h
                    · intro h; obtain ⟨g, hg, hgv⟩ := List.mem_map.1 h; rwa [← UScalar.eq_of_val_eq hgv]
                  split
                  · rename_i hin
                    simp only [WP.spec_ok]
                    exact ⟨fun h => absurd h (by simp), fun _ => Or.inr (hmemI.1 (by simpa [hisin] using hin))⟩
                  · rename_i hin
                    have hil : into.val.length < Usize.max := by
                      have := congrArg List.length hinto; simp at this
                      have hin2 : bef c.val ∉ (frm.take (i.val - lo.val)).map bef :=
                        fun h => hin (by simpa [hisin] using hmemI.2 h)
                      have hnd2 : ((frm.take (i.val - lo.val)).map bef ++ [bef c.val]).Nodup := by
                        refine List.nodup_append.2 ⟨hnd, by simp, ?_⟩
                        intro x hx y hy hxy; simp at hy; subst hy; subst hxy; exact hin2 hx
                      have hlt2 : ∀ x ∈ (frm.take (i.val - lo.val)).map bef ++ [bef c.val], x < b.coedges.size := by
                        intro x hx
                        rcases List.mem_append.1 hx with hx | hx
                        · obtain ⟨y, hy, rfl⟩ := List.mem_map.1 hx
                          have := List.all_eq_true.1 hall y hy
                          unfold guardP at this; simp only [Bool.and_eq_true, decide_eq_true_eq] at this; exact this.1.1.1
                        · simp at hx; rw [hx, ← ha]; exact hav
                      have := (hnd2.subperm fun x hx => List.mem_range.2 (hlt2 x hx)).length_le
                      simp at this
                      have hncb := le_max nc
                      omega
                    step with alloc.vec.Vec.push_spec into a hil as ⟨into2, hinto2⟩
                    refine ⟨fun _ => ⟨by rw [hgP]; simpa [hused] using hu, fun h => hin (by simpa [hisin] using hmemI.2 h),
                      by rw [hinto2, List.map_append, hinto]; simp [ha]⟩, fun h => absurd h (by simp [hok])⟩
                · rename_i hu
                  simp only [WP.spec_ok]
                  exact ⟨fun h => absurd h (by simp), fun _ => Or.inl (by rw [hgP]; simpa [hused] using hu)⟩
              · rintro ⟨into, ok2⟩ ⟨hT, hF⟩
                guard_next
      · rename_i hlt
        have hi : i.val = hi.val := by scalar_tac
        simp only [WP.spec_ok]
        have htk : frm.take (i.val - lo.val) = frm := by rw [hi]; exact List.take_of_length_le (by omega)
        rw [htk] at hall hnd
        rw [hok, hall, decide_eq_true hnd]; rfl
    · rename_i hok
      simp only [WP.spec_ok]
      have hok' : ok1 = false := by simpa using hok
      rw [hok', hfalse hok']
  · have h0 : lo.val - lo.val = 0 := by omega
    simp only [h0, List.take_zero]
    exact ⟨le_refl _, hlo, fun _ => ⟨by simp, by simp, by simp⟩, fun h => absurd h (by simp)⟩

end guard

/-! ## One vertex: `n_fans` -/

section nfans
variable {T : Type} (FI : topo_fans.FanTopo T) (t : T) {b : Body} {corners : Nat → Nat → List (Nat × Nat)}

theorem befUse_none (H : Sound b) (W : b.WF) : BefUse (b := b) (b.before none) :=
  fun _ hc => use_prev H W hc

theorem befUse_some (H : Sound b) (W : b.WF) (hC : CornersOK b corners) (v : Nat) (fs : List Nat) :
    BefUse (b := b) (b.before (some (tableOf corners v fs))) := by
  intro c hc
  unfold Body.before
  simp only
  split
  · rename_i x hx
    have hm := List.mem_of_find?_eq_some hx
    rw [List.mem_flatMap] at hm
    obtain ⟨p, hp, hxp⟩ := hm
    have hpL := (List.mem_filter.1 hp).1
    unfold tableOf at hpL
    obtain ⟨f, -, rfl⟩ := List.mem_map.1 hpL
    exact ((hC f v).2 x hxp).2.2
  · exact use_prev H W hc

theorem fuel_gt (frm : List Nat) : (allUses b).length < b.fuel frm := by
  unfold Body.fuel; rw [allUses_length]; omega

theorem frmOf_head (items : Slice Usize) (lo hi : Nat) (h1 : lo < hi) (h2 : hi ≤ items.val.length) :
    (frmOf items lo hi).headD 0 = (items.val[lo]!).val := by
  unfold frmOf
  rw [List.drop_eq_getElem_cons (by omega), show hi - lo = (hi - lo - 1) + 1 by omega, List.take_succ_cons]
  simp [getElem!_pos items.val lo (by omega)]

theorem frmOf_len (items : Slice Usize) (lo hi : Nat) (h1 : lo ≤ hi) (h2 : hi ≤ items.val.length) :
    (frmOf items lo hi).length = hi - lo := by
  simp [frmOf]; omega

theorem n_fans_spec (R : FanReads FI t b corners) (H : Sound b) (W : b.WF) (hC : CornersOK b corners)
    (items : Slice Usize) (lo hi : Usize) (hlo : lo.val ≤ hi.val) (hhi : hi.val ≤ items.val.length)
    (hfrm : ∀ x ∈ frmOf items lo.val hi.val, Use b x) (seen : alloc.vec.Vec Usize) :
    topo_fans.n_fans FI t items lo hi seen ⦃ r => r.1.val = (b.fans corners (frmOf items lo.val hi.val)).length ⦄ := by
  set frm := frmOf items lo.val hi.val with hfrmdef
  have hF := fuel_gt (b := b) frm
  have h0 : SeenOK b [] := ⟨by simp, by simp⟩
  unfold topo_fans.n_fans
  step with count_spec FI t R H W ⟨alloc.vec.Vec.new Usize, alloc.vec.Vec.new (Usize × Usize), alloc.vec.Vec.new Usize⟩
    false (b.before none) (before_none_spec FI t R H W _) (befUse_none H W) (b.fuel frm) hF items lo hi hlo hhi hfrm seen
    as ⟨cnt, sl, seen1, hcnt, hsl, hseen1⟩
  rw [← hfrmdef] at hcnt hseen1
  have hfs : b.fansBy frm none = b.fansGo (b.before none) (b.fuel frm) frm [] := rfl
  unfold Body.fans
  simp only
  rw [hfs]
  split
  · rename_i hlt
    have hlt' : cnt.val < 2 := by scalar_tac
    simp only [WP.spec_ok]
    rw [if_pos (by omega)]; exact hcnt
  · rename_i hlt
    have hge : 2 ≤ cnt.val := by scalar_tac
    rw [if_neg (by omega)]
    have hlen := fansGo_length_le (b := b) (b.before none) (b.fuel frm) frm []
    have hfl := frmOf_len items lo.val hi.val hlo hhi
    rw [← hfrmdef] at hfl
    have hlohi : lo.val < hi.val := by omega
    have hix : lo.val < items.length := by have : items.length = items.val.length := rfl; omega
    step with Slice.index_usize_spec items lo hix as ⟨c0, hc0⟩
    have hhead := frmOf_head items lo.val hi.val hlohi hhi
    rw [← hfrmdef] at hhead
    have hc0u : Use b c0.val := by
      apply hfrm
      have : c0.val = frm.headD 0 := by rw [hhead, hc0, getElem!_pos]
      have hne : frm ≠ [] := by intro h; rw [h] at hfl; simp at hfl; omega
      obtain ⟨x, xs, hx⟩ := List.exists_cons_of_ne_nil hne
      rw [this, hx]; simp
    obtain ⟨hcn, hen, -, -, -, -⟩ := use_facts H W hc0u
    step with start_spec FI t R c0 hcn hen as ⟨vtx, hvtx⟩
    have hvtx' : vtx.val = b.start (frm.headD 0) := by rw [hvtx, hhead, hc0, getElem!_pos]
    step as ⟨cs0, hcs0⟩
    have hSok : SeenOK b (seenOf seen1 sl.val) := by
      rw [hseen1]; exact fansSeen_ok H W (befUse_none H W) _ frm [] hfrm h0
    step with table_spec FI t R H W hC seen1 sl vtx hsl hSok cs0 (by rw [hcs0]; rfl)
      as ⟨cf, cl, cs, any, hrep, hany⟩
    have hfaces : facesOf b (seenOf seen1 sl.val) =
        ((b.fansGo (b.before none) (b.fuel frm) frm []).flatten.map b.face).eraseDups := by
      rw [hseen1, fansSeen_flatten]; simp [facesOf]
    rw [hfaces, hvtx'] at hrep hany
    set L := tableOf corners (b.start (frm.headD 0)) ((b.fansGo (b.before none) (b.fuel frm) frm []).flatten.map b.face).eraseDups with hL
    have hLeq : ((((b.fansGo (b.before none) (b.fuel frm) frm []).flatten.map b.face).eraseDups).map
        fun f => (f, corners f (b.start (frm.headD 0)))) = L := rfl
    rw [hLeq]
    split
    · rename_i hanyT
      rw [hany] at hanyT
      have hnall : L.all (·.2.isEmpty) = false := by simpa using hanyT
      rw [if_neg (by simp [hnall])]
      have rnc := R.base.nc
      have rne := R.base.ne
      step with rnc as ⟨nc', hnc'⟩
      step with rne as ⟨ne, hne⟩
      have hBS := before_some_spec FI t R H W ⟨cf, cl, cs⟩ L hrep
      step with guard_spec FI t R H W ⟨cf, cl, cs⟩ (b.before (some L)) hBS items lo hi hlo hhi hfrm vtx nc' ne hnc' hne
        as ⟨okg, hokg⟩
      rw [closesOk_eq, ← hvtx']
      split
      · rename_i hok
        rw [hokg] at hok
        rw [if_pos hok]
        step with count_spec FI t R H W ⟨cf, cl, cs⟩ true (b.before (some L)) hBS (befUse_some H W hC _ _)
          (b.fuel frm) hF items lo hi hlo hhi hfrm seen1 as ⟨cnt2, sl2, seen2, hcnt2, hsl2, hseen2⟩
        rw [← hfrmdef] at hcnt2
        exact hcnt2
      · rename_i hok
        have hok' : okg = false := by simpa using hok
        rw [hokg] at hok'
        rw [if_neg (by rw [hok']; simp)]
        simp only [WP.spec_ok]; exact hcnt
    · rename_i hanyF
      rw [hany] at hanyF
      have hall : L.all (·.2.isEmpty) = true := by simpa using hanyF
      rw [if_pos hall]
      simp only [WP.spec_ok]; exact hcnt

end nfans

/-! ## The leaving coedges, by counting sort -/

section sort
variable {T : Type} (FI : topo_fans.FanTopo T) (t : T) {b : Body} {corners : Nat → Nat → List (Nat × Nat)}

/-- The uses of the edges below `k`, edge by edge. -/
def usesUpTo (b : Body) (k : Nat) : List Nat := (List.range k).flatMap fun e => (b.ed e).uses

/-- How many of `Q` leave `w`. -/
def cntQ (b : Body) (Q : List Nat) (w : Nat) : Nat := (Q.filter fun c => b.start c == w).length

theorem usesUpTo_succ (b : Body) (k : Nat) : usesUpTo b (k + 1) = usesUpTo b k ++ (b.ed k).uses := by
  simp [usesUpTo, List.range_succ, List.flatMap_append]

theorem usesUpTo_all (b : Body) : usesUpTo b b.edges.size = allUses b := by
  unfold usesUpTo allUses
  have h : (List.range b.edges.size).map (fun e => b.ed e) = b.edges.toList := by
    apply List.ext_getElem (by simp)
    intro i h1 h2
    simp only [List.getElem_map, List.getElem_range, Array.getElem_toList]
    unfold Body.ed; rw [Array.getD_eq_getD_getElem?, Array.getElem?_eq_getElem (by simpa using h2)]; rfl
  rw [← h, List.flatMap_map]

theorem cntQ_snoc (b : Body) (Q : List Nat) (c w : Nat) :
    cntQ b (Q ++ [c]) w = cntQ b Q w + (if b.start c = w then 1 else 0) := by
  unfold cntQ; rw [List.filter_append]; simp; split <;> simp_all

theorem use_start_lt (H : Sound b) (W : b.WF) {c : Nat} (hc : Use b c) : b.start c < b.nv := by
  obtain ⟨-, he, -, -, -, -⟩ := use_facts H W hc
  have := W.ends _ he
  unfold Body.start; split <;> omega

/-- The counts so far, at `off[w + 1]`. -/
def CountInv (b : Body) (Q : List Nat) (off : alloc.vec.Vec Usize) : Prop :=
  off.val.length = b.nv + 1 ∧ (off.val[0]!).val = 0 ∧ ∀ w < b.nv, (off.val[w + 1]!).val = cntQ b Q w

theorem count_edge_spec (R : FanReads FI t b corners) (H : Sound b) (W : b.WF)
    (hsmall : (allUses b).length < Usize.max) (Q : List Nat) (hQ : List.Sublist Q (allUses b))
    (e n : Usize) (he : e.val < b.edges.size) (hn : n.val = (b.ed e.val).uses.length)
    (hQe : List.Sublist (Q ++ (b.ed e.val).uses) (allUses b))
    (off : alloc.vec.Vec Usize) (hI : CountInv b Q off) :
    topo_fans.fans_loop0_loop0 FI t off e n 0#usize ⦃ r => CountInv b (Q ++ (b.ed e.val).uses) r ⦄ := by
  set U := (b.ed e.val).uses with hU
  have hcntle : ∀ (P : List Nat) w, cntQ b P w ≤ P.length := fun P w => by unfold cntQ; exact List.length_filter_le _ _
  unfold topo_fans.fans_loop0_loop0
  apply loop.spec_decr_nat (fun (y : alloc.vec.Vec Usize × Usize) => n.val - y.2.val)
    (fun y => y.2.val ≤ n.val ∧ CountInv b (Q ++ U.take y.2.val) y.1)
  · rintro ⟨o, i⟩ ⟨hle, hI0⟩
    simp only at hle hI0 ⊢
    unfold topo_fans.fans_loop0_loop0.body
    split
    · rename_i hlt
      have hlt' : i.val < n.val := by scalar_tac
      have hiU : i.val < U.length := by omega
      step with R.base.edge_use e i he hiU as ⟨c, hc⟩
      have hcv : c.val = U[i.val] := by rw [hc, getElem!_pos U i.val hiU]
      have hcu : Use b c.val := ⟨e.val, he, by rw [hcv]; exact List.getElem_mem hiU⟩
      obtain ⟨hcn, hen, -, -, -, -⟩ := use_facts H W hcu
      step with start_spec FI t R c hcn hen as ⟨st, hst⟩
      have hstn : st.val < b.nv := by rw [hst]; exact use_start_lt H W hcu
      obtain ⟨hol, ho0, hos⟩ := hI0
      have hop : o.val.length ≤ Usize.max := o.property
      step with usize_add_wspec st 1#usize (by have h1 : (1#usize).val = 1 := rfl; rw [h1]; omega) as ⟨s1, hs1⟩
      have hs1v : s1.val = st.val + 1 := by rw [hs1]; exact add_val st 1#usize (lt_size_of_le_max _ (by have h1 : (1#usize).val = 1 := rfl; rw [h1]; omega))
      have hs1l : s1.val < o.val.length := by omega
      step as ⟨x, back, hx, hback⟩
      have hxv : x.val = cntQ b (Q ++ U.take i.val) st.val := by
        rw [hx, ← getElem!_pos o.val s1.val hs1l, hs1v]; exact hos st.val hstn
      have htk : Q ++ U.take (i.val + 1) = (Q ++ U.take i.val) ++ [c.val] := by
        rw [List.take_succ_eq_append_getElem hiU, hcv]; simp
      have hlen1 : (Q ++ U.take (i.val + 1)).length ≤ (allUses b).length := by
        have := hQe.length_le; simp at this ⊢; omega
      have hx1 : x.val + 1 ≤ Usize.max := by
        have := hcntle (Q ++ U.take i.val) st.val
        rw [htk, List.length_append, List.length_singleton] at hlen1; omega
      step with usize_add_wspec x 1#usize (by have h1 : (1#usize).val = 1 := rfl; rw [h1]; exact hx1) as ⟨x1, hx1'⟩
      have hx1v : x1.val = x.val + 1 := by rw [hx1']; exact add_val x 1#usize (lt_size_of_le_max _ (by have h1 : (1#usize).val = 1 := rfl; rw [h1]; exact hx1))
      step with usize_add_wspec i 1#usize (succ_le_max i n hlt') as ⟨i1, hi1⟩
      have hi1v : i1.val = i.val + 1 := by rw [hi1]; exact succ_val i n hlt'
      rw [hi1v, hback]
      have hsv : (o.set s1 x1).val = o.val.set s1.val x1 := by simp
      refine ⟨by omega, ⟨by rw [hsv]; simp [hol], ?_, fun w hw => ?_⟩, by omega⟩
      · rw [hsv, getElem!_set _ _ _ _ (by omega), if_neg (by omega)]; exact ho0
      · rw [hsv, getElem!_set _ _ _ _ (by omega), htk, cntQ_snoc]
        by_cases hws : w = st.val
        · subst hws; rw [if_pos (by omega), hx1v, hxv, if_pos hst.symm]
        · rw [if_neg (by omega), hos w hw, if_neg (by rw [← hst]; exact fun h => hws h.symm)]; rfl
    · rename_i hlt
      have hi : i.val = n.val := by scalar_tac
      simp only [WP.spec_ok]
      rw [hi, hn, List.take_length] at hI0; exact hI0
  · have h0 : (0#usize).val = 0 := rfl
    simp only [h0, List.take_zero, List.append_nil]; exact ⟨by simp, hI⟩

theorem usesUpTo_sublist (b : Body) (k : Nat) (hk : k ≤ b.edges.size) :
    List.Sublist (usesUpTo b k) (allUses b) := by
  rw [← usesUpTo_all]
  unfold usesUpTo
  have : List.range b.edges.size = List.range k ++ List.range' k (b.edges.size - k) := by
    rw [List.range_eq_range', List.range_eq_range']
    conv_lhs => rw [show b.edges.size = k + (b.edges.size - k) by omega]
    rw [← List.range'_append_1]; simp
  rw [this, List.flatMap_append]; exact List.sublist_append_left _ _

theorem count_all_spec (R : FanReads FI t b corners) (H : Sound b) (W : b.WF)
    (hsmall : (allUses b).length < Usize.max) (ne : Usize) (hne : ne.val = b.edges.size)
    (off : alloc.vec.Vec Usize) (hI : CountInv b [] off) :
    topo_fans.fans_loop0 FI t ne off 0#usize ⦃ r => CountInv b (allUses b) r ⦄ := by
  unfold topo_fans.fans_loop0
  apply loop.spec_decr_nat (fun (y : alloc.vec.Vec Usize × Usize) => ne.val - y.2.val)
    (fun y => y.2.val ≤ ne.val ∧ CountInv b (usesUpTo b y.2.val) y.1)
  · rintro ⟨o, e⟩ ⟨hle, hI0⟩
    simp only at hle hI0 ⊢
    unfold topo_fans.fans_loop0.body
    split
    · rename_i hlt
      have hlt' : e.val < ne.val := by scalar_tac
      have he : e.val < b.edges.size := by omega
      step with R.base.edge_n_uses e he as ⟨n, hn⟩
      have hsub : List.Sublist (usesUpTo b e.val ++ (b.ed e.val).uses) (allUses b) := by
        rw [← usesUpTo_succ]; exact usesUpTo_sublist b _ (by omega)
      step with count_edge_spec FI t R H W hsmall _ (usesUpTo_sublist b _ (by omega)) e n he hn hsub o hI0 as ⟨o1, hI1⟩
      step with usize_add_wspec e 1#usize (succ_le_max e ne hlt') as ⟨e1, he1⟩
      have he1v : e1.val = e.val + 1 := by rw [he1]; exact succ_val e ne hlt'
      rw [he1v, usesUpTo_succ]
      exact ⟨by omega, hI1, by omega⟩
    · rename_i hlt
      have hk : e.val = ne.val := by scalar_tac
      simp only [WP.spec_ok]
      rw [hk, hne, usesUpTo_all] at hI0; exact hI0
  · have h0 : (0#usize).val = 0 := rfl
    simp only [h0]; exact ⟨by simp, by simpa [usesUpTo] using hI⟩

/-- The uses of `A` starting below `k`. -/
def cntLt (b : Body) (A : List Nat) (k : Nat) : Nat := (A.filter fun c => decide (b.start c < k)).length

theorem cntLt_succ (b : Body) (A : List Nat) (k : Nat) : cntLt b A (k + 1) = cntLt b A k + cntQ b A k := by
  induction A with
  | nil => simp [cntLt, cntQ]
  | cons c A ih =>
    unfold cntLt cntQ at ih ⊢
    rw [List.filter_cons, List.filter_cons, List.filter_cons]
    by_cases h1 : b.start c < k
    · have h2 : b.start c < k + 1 := by omega
      have h3 : (b.start c == k) = false := by simp; omega
      rw [if_pos (by simpa using h2), if_pos (by simpa using h1), if_neg (by simp [h3])]
      simp only [List.length_cons]; omega
    · by_cases h4 : b.start c = k
      · rw [if_pos (by simp; omega), if_neg (by simpa using h1), if_pos (by simp [h4])]
        simp only [List.length_cons]; omega
      · rw [if_neg (by simp; omega), if_neg (by simpa using h1), if_neg (by simp [h4])]
        omega

/-- Where vertex `w`'s leaving coedges start in `items`. -/
def vpre (b : Body) (A : List Nat) (w : Nat) : Nat := ((List.range w).map (cntQ b A)).sum

theorem vpre_eq (b : Body) (A : List Nat) (w : Nat) : vpre b A w = cntLt b A w := by
  induction w with
  | zero => simp [vpre, cntLt]
  | succ w ih => rw [cntLt_succ, ← ih]; simp [vpre, List.range_succ]

theorem vpre_succ (b : Body) (A : List Nat) (w : Nat) : vpre b A (w + 1) = vpre b A w + cntQ b A w := by
  simp [vpre, List.range_succ]

theorem vpre_le (b : Body) (A : List Nat) (w : Nat) : vpre b A w ≤ A.length := by
  rw [vpre_eq]; exact List.length_filter_le _ _

theorem prefix_spec (hsmall : (allUses b).length < Usize.max) (nv : Usize) (hnv : nv.val = b.nv)
    (off atv : alloc.vec.Vec Usize) (hI : CountInv b (allUses b) off) (hat : atv.val.length = b.nv) :
    topo_fans.fans_loop1 nv off atv 0#usize ⦃ r =>
      r.1.val.length = b.nv + 1 ∧ r.2.val.length = b.nv ∧
      (∀ w ≤ b.nv, (r.1.val[w]!).val = vpre b (allUses b) w) ∧
      (∀ w < b.nv, (r.2.val[w]!).val = vpre b (allUses b) w) ⦄ := by
  set A := allUses b
  obtain ⟨hol, ho0, hos⟩ := hI
  unfold topo_fans.fans_loop1
  apply loop.spec_decr_nat (fun (y : alloc.vec.Vec Usize × alloc.vec.Vec Usize × Usize) => nv.val - y.2.2.val)
    (fun y => y.2.2.val ≤ nv.val ∧ y.1.val.length = b.nv + 1 ∧ y.2.1.val.length = b.nv ∧
      (∀ w ≤ y.2.2.val, (y.1.val[w]!).val = vpre b A w) ∧
      (∀ w, y.2.2.val ≤ w → w < b.nv → (y.1.val[w + 1]!).val = cntQ b A w) ∧
      (∀ w < y.2.2.val, (y.2.1.val[w]!).val = vpre b A w))
  · rintro ⟨o, a, v⟩ ⟨hle, hol', hal, hpre, hcnt, hat'⟩
    simp only at hle hol' hal hpre hcnt hat' ⊢
    unfold topo_fans.fans_loop1.body
    split
    · rename_i hlt
      have hlt' : v.val < nv.val := by scalar_tac
      step as ⟨x, hx⟩
      have hxv : x.val = vpre b A v.val := by rw [hx, ← getElem!_pos o.val v.val (by omega)]; exact hpre v.val (le_refl _)
      step as ⟨y0, backa, hy0, hbacka⟩
      step with usize_add_wspec v 1#usize (succ_le_max v nv hlt') as ⟨v1, hv1⟩
      have hv1v : v1.val = v.val + 1 := by rw [hv1]; exact succ_val v nv hlt'
      step as ⟨z, backo, hz, hbacko⟩
      have hzv : z.val = cntQ b A v.val := by
        rw [hz, ← getElem!_pos o.val v1.val (by omega), hv1v]; exact hcnt v.val (le_refl _) (by omega)
      have hsum : z.val + x.val ≤ Usize.max := by
        have := vpre_le b A (v.val + 1); rw [vpre_succ] at this; omega
      step with usize_add_wspec z x hsum as ⟨z1, hz1⟩
      have hz1v : z1.val = vpre b A (v.val + 1) := by
        rw [hz1, add_val z x (lt_size_of_le_max _ hsum), hzv, hxv, vpre_succ]; omega
      rw [hv1v, hbacko, hbacka]
      have hs1 : (o.set v1 z1).val = o.val.set v1.val z1 := by simp
      have hs2 : (a.set v x).val = a.val.set v.val x := by simp
      refine ⟨by omega, by rw [hs1]; simp [hol'], by rw [hs2]; simp [hal], fun w hw => ?_, fun w hw1 hw2 => ?_,
        fun w hw => ?_, by omega⟩
      · rw [hs1, getElem!_set _ _ _ _ (by omega), hv1v]
        split
        · rename_i h; subst h; exact hz1v
        · exact hpre w (by omega)
      · rw [hs1, getElem!_set _ _ _ _ (by omega), hv1v, if_neg (by omega)]; exact hcnt w (by omega) hw2
      · rw [hs2, getElem!_set _ _ _ _ (by omega)]
        split
        · rename_i h; subst h; exact hxv
        · exact hat' w (by omega)
    · rename_i hlt
      have hv : v.val = nv.val := by scalar_tac
      simp only [WP.spec_ok]
      rw [hv, hnv] at hpre hat'
      exact ⟨hol', hal, hpre, hat'⟩
  · have h0 : (0#usize).val = 0 := rfl
    simp only [h0]
    refine ⟨by omega, hol, hat, fun w hw => ?_, fun w _ hw => hos w hw, fun w hw => by omega⟩
    have : w = 0 := by omega
    subst this; rw [ho0]; simp [vpre]

theorem vpre_mono (b : Body) (A : List Nat) {w w' : Nat} (h : w ≤ w') : vpre b A w ≤ vpre b A w' := by
  induction w' with
  | zero => have : w = 0 := by omega
            subst this; exact le_refl _
  | succ w' ih =>
    rcases Nat.lt_or_ge w (w' + 1) with h1 | h1
    · have := ih (by omega); rw [vpre_succ]; omega
    · have : w = w' + 1 := by omega
      subst this; exact le_refl _

theorem cntQ_sub (b : Body) {Q A : List Nat} (h : List.Sublist Q A) (w : Nat) : cntQ b Q w ≤ cntQ b A w :=
  (h.filter _).length_le

/-- What the placing loop has done after the coedges `Q`. -/
def PlaceInv (b : Body) (A Q : List Nat) (atv items : List Usize) : Prop :=
  atv.length = b.nv ∧ items.length = A.length ∧
  (∀ w < b.nv, (atv[w]!).val = vpre b A w + cntQ b Q w) ∧
  (∀ w < b.nv, ∀ j < cntQ b Q w, (items[vpre b A w + j]!).val = (Q.filter fun c => b.start c == w)[j]!)

theorem place_bound (b : Body) (A Q : List Nat) (c : Nat) (hQA : List.Sublist (Q ++ [c]) A)
    (hnvA : vpre b A b.nv = A.length) (atv items : List Usize) (hP : PlaceInv b A Q atv items)
    (s : Nat) (hs : s < b.nv) (hcs : b.start c = s) : (atv[s]!).val < items.length := by
  obtain ⟨hal, hil, hat, hit⟩ := hP
  have hlt : cntQ b Q s < cntQ b A s := by
    have := cntQ_sub b hQA s; rw [cntQ_snoc, if_pos hcs] at this; omega
  have hend : vpre b A (s + 1) ≤ A.length := by rw [← hnvA]; exact vpre_mono b A (by omega)
  rw [hat s hs, hil]; rw [vpre_succ] at hend; omega

theorem place_step (b : Body) (A Q : List Nat) (c : Nat) (hQA : List.Sublist (Q ++ [c]) A) (hnvA : vpre b A b.nv = A.length)
    (atv items : List Usize) (hP : PlaceInv b A Q atv items) (s : Nat) (hs : s < b.nv) (hcs : b.start c = s)
    (cu : Usize) (hcu : cu.val = c) (a1 : Usize) (ha1 : a1.val = (atv[s]!).val + 1) :
    PlaceInv b A (Q ++ [c]) (atv.set s a1) (items.set (atv[s]!).val cu) ∧
    (atv[s]!).val < items.length := by
  obtain ⟨hal, hil, hat, hit⟩ := hP
  have hlt : cntQ b Q s < cntQ b A s := by
    have := cntQ_sub b hQA s; rw [cntQ_snoc, if_pos hcs] at this; omega
  have hpos : (atv[s]!).val < vpre b A (s + 1) := by rw [hat s hs, vpre_succ]; omega
  have hend : vpre b A (s + 1) ≤ A.length := by rw [← hnvA]; exact vpre_mono b A (by omega)
  refine ⟨⟨by simp [hal], by simp [hil], fun w hw => ?_, fun w hw j hj => ?_⟩, by omega⟩
  · rw [getElem!_set _ _ _ _ (by omega), cntQ_snoc]
    split
    · rename_i h; subst h; rw [ha1, hat w hw, if_pos hcs]; omega
    · rename_i h; rw [hat w hw, if_neg (by omega)]; rfl
  · rw [getElem!_set _ _ _ _ (by
      have := cntQ_sub b hQA w; have := vpre_mono b A (show w + 1 ≤ b.nv by omega); rw [vpre_succ] at this; omega)]
    rw [List.filter_append]
    by_cases hw' : w = s
    · subst hw'
      rw [cntQ_snoc, if_pos hcs] at hj
      by_cases hj' : j = cntQ b Q w
      · rw [if_pos (by rw [hat w hw, hj']), hcu]
        have hl : (Q.filter fun c => b.start c == w).length = j := by rw [hj']; rfl
        rw [getElem!_pos _ j (by simp [hcs]; omega)]
        simp [hcs, List.getElem_append_right (by omega : (Q.filter fun c => b.start c == w).length ≤ j), hl]
      · rw [if_neg (by rw [hat w hw]; omega)]
        rw [hit w hw j (by omega), getElem!_pos _ j (by unfold cntQ at hj hj'; omega),
          getElem!_pos _ j (by simp [hcs]; unfold cntQ at hj hj'; omega), List.getElem_append_left]
    · have hj2 : j < cntQ b Q w := by rw [cntQ_snoc, if_neg (by omega)] at hj; omega
      rw [if_neg ?_]
      · have hcw : (b.start c == w) = false := by simp; omega
        rw [hit w hw j hj2, getElem!_pos _ j (by unfold cntQ at hj2; omega),
          getElem!_pos _ j (by simp [hcw]; unfold cntQ at hj2; omega)]
        simp [hcw]; try rfl
      · rw [hat s hs]
        have h1 := cntQ_sub b hQA w; rw [cntQ_snoc, if_neg (by omega)] at h1
        rcases Nat.lt_or_ge w s with hws | hws
        · have := vpre_mono b A (show w + 1 ≤ s by omega); rw [vpre_succ] at this; omega
        · have := vpre_mono b A (show s + 1 ≤ w by omega); rw [vpre_succ] at this; omega

theorem place_edge_spec (R : FanReads FI t b corners) (H : Sound b) (W : b.WF)
    (hsmall : (allUses b).length < Usize.max) (Q : List Nat)
    (e n : Usize) (he : e.val < b.edges.size) (hn : n.val = (b.ed e.val).uses.length)
    (hQe : List.Sublist (Q ++ (b.ed e.val).uses) (allUses b)) (hnvA : vpre b (allUses b) b.nv = (allUses b).length)
    (atv items : alloc.vec.Vec Usize) (hP : PlaceInv b (allUses b) Q atv.val items.val) :
    topo_fans.fans_loop2_loop0 FI t atv items e n 0#usize ⦃ r =>
      PlaceInv b (allUses b) (Q ++ (b.ed e.val).uses) r.1.val r.2.val ⦄ := by
  set U := (b.ed e.val).uses with hU
  set A := allUses b
  unfold topo_fans.fans_loop2_loop0
  apply loop.spec_decr_nat (fun (y : alloc.vec.Vec Usize × alloc.vec.Vec Usize × Usize) => n.val - y.2.2.val)
    (fun y => y.2.2.val ≤ n.val ∧ PlaceInv b A (Q ++ U.take y.2.2.val) y.1.val y.2.1.val)
  · rintro ⟨a, it, i⟩ ⟨hle, hP0⟩
    simp only at hle hP0 ⊢
    unfold topo_fans.fans_loop2_loop0.body
    split
    · rename_i hlt
      have hlt' : i.val < n.val := by scalar_tac
      have hiU : i.val < U.length := by omega
      step with R.base.edge_use e i he hiU as ⟨c, hc⟩
      have hcv : c.val = U[i.val] := by rw [hc, getElem!_pos U i.val hiU]
      have hcu : Use b c.val := ⟨e.val, he, by rw [hcv]; exact List.getElem_mem hiU⟩
      obtain ⟨hcn, hen, -, -, -, -⟩ := use_facts H W hcu
      step with start_spec FI t R c hcn hen as ⟨st, hst⟩
      have hstn : st.val < b.nv := by rw [hst]; exact use_start_lt H W hcu
      have htk : Q ++ U.take (i.val + 1) = (Q ++ U.take i.val) ++ [c.val] := by
        rw [List.take_succ_eq_append_getElem hiU, hcv]; simp
      have hsub : List.Sublist ((Q ++ U.take i.val) ++ [c.val]) A := by
        rw [← htk]; exact ((List.take_sublist (i.val + 1) U).append_left Q).trans hQe
      have hal := hP0.1
      have hil := hP0.2.1
      step as ⟨p, hp⟩
      have hpv : p = a.val[st.val]! := by rw [hp, getElem!_pos]
      have hpb := place_bound b A (Q ++ U.take i.val) c.val hsub hnvA a.val it.val hP0 st.val hstn hst.symm
      rw [← hpv] at hpb
      have hAl : A.length < Usize.max := hsmall
      step as ⟨x0, back, hx0, hback⟩
      step as ⟨y0, back1, hy0, hback1⟩
      have hy0v : y0 = a.val[st.val]! := by rw [hy0, getElem!_pos]
      have h1 : (1#usize).val = 1 := rfl
      step with usize_add_wspec y0 1#usize (by rw [h1, hy0v, ← hpv]; omega) as ⟨y1, hy1⟩
      have hy1v : y1.val = (a.val[st.val]!).val + 1 := by
        rw [hy1, add_val y0 1#usize (lt_size_of_le_max _ (by rw [h1, hy0v, ← hpv]; omega)), hy0v, h1]
      step with usize_add_wspec i 1#usize (succ_le_max i n hlt') as ⟨i1, hi1⟩
      have hi1v : i1.val = i.val + 1 := by rw [hi1]; exact succ_val i n hlt'
      rw [hi1v, hback, hback1]
      have hsa : (a.set st y1).val = a.val.set st.val y1 := by simp
      have hsi : (it.set p c).val = it.val.set p.val c := by simp
      rw [hsa, hsi, htk, hpv]
      refine ⟨by omega, (place_step b A (Q ++ U.take i.val) c.val hsub hnvA a.val it.val hP0 st.val hstn hst.symm c rfl y1 hy1v).1, by omega⟩
    · rename_i hlt
      have hi : i.val = n.val := by scalar_tac
      simp only [WP.spec_ok]
      rw [hi, hn, List.take_length] at hP0; exact hP0
  · have h0 : (0#usize).val = 0 := rfl
    simp only [h0, List.take_zero, List.append_nil]; exact ⟨by simp, hP⟩

theorem place_all_spec (R : FanReads FI t b corners) (H : Sound b) (W : b.WF)
    (hsmall : (allUses b).length < Usize.max) (hnvA : vpre b (allUses b) b.nv = (allUses b).length)
    (ne : Usize) (hne : ne.val = b.edges.size)
    (atv items : alloc.vec.Vec Usize) (hP : PlaceInv b (allUses b) [] atv.val items.val) :
    topo_fans.fans_loop2 FI t ne atv items 0#usize ⦃ r =>
      r.val.length = (allUses b).length ∧
      ∀ w < b.nv, ∀ j < cntQ b (allUses b) w, (r.val[vpre b (allUses b) w + j]!).val =
        ((allUses b).filter fun c => b.start c == w)[j]! ⦄ := by
  unfold topo_fans.fans_loop2
  apply loop.spec_decr_nat (fun (y : alloc.vec.Vec Usize × alloc.vec.Vec Usize × Usize) => ne.val - y.2.2.val)
    (fun y => y.2.2.val ≤ ne.val ∧ PlaceInv b (allUses b) (usesUpTo b y.2.2.val) y.1.val y.2.1.val)
  · rintro ⟨a, it, e⟩ ⟨hle, hP0⟩
    simp only at hle hP0 ⊢
    unfold topo_fans.fans_loop2.body
    split
    · rename_i hlt
      have hlt' : e.val < ne.val := by scalar_tac
      have he : e.val < b.edges.size := by omega
      step with R.base.edge_n_uses e he as ⟨n, hn⟩
      have hsub : List.Sublist (usesUpTo b e.val ++ (b.ed e.val).uses) (allUses b) := by
        rw [← usesUpTo_succ]; exact usesUpTo_sublist b _ (by omega)
      step with place_edge_spec FI t R H W hsmall _ e n he hn hsub hnvA a it hP0 as ⟨a1, it1, hP1⟩
      step with usize_add_wspec e 1#usize (succ_le_max e ne hlt') as ⟨e1, he1⟩
      have he1v : e1.val = e.val + 1 := by rw [he1]; exact succ_val e ne hlt'
      rw [he1v, usesUpTo_succ]
      exact ⟨by omega, hP1, by omega⟩
    · rename_i hlt
      have hk : e.val = ne.val := by scalar_tac
      simp only [WP.spec_ok]
      rw [hk, hne, usesUpTo_all] at hP0
      exact ⟨hP0.2.1, hP0.2.2.2⟩
  · have h0 : (0#usize).val = 0 := rfl
    simp only [h0]; exact ⟨by simp, by simpa [usesUpTo] using hP⟩

theorem leaving_eq (b : Body) (v : Nat) : b.leaving v = (allUses b).filter fun c => b.start c == v := by
  unfold Body.leaving; rw [← usesUpTo_all]; unfold usesUpTo; rw [List.filter_flatMap]

theorem allUses_use (b : Body) {c : Nat} (h : c ∈ allUses b) : Use b c := by
  rw [← usesUpTo_all] at h; unfold usesUpTo at h
  obtain ⟨e, he, hc⟩ := List.mem_flatMap.1 h
  exact ⟨e, List.mem_range.1 he, hc⟩

theorem vpre_all (H : Sound b) (W : b.WF) : vpre b (allUses b) b.nv = (allUses b).length := by
  rw [vpre_eq]; unfold cntLt
  rw [List.filter_eq_self.2]
  intro c hc; simpa using use_start_lt H W (allUses_use b hc)

end sort

end forge_topo
