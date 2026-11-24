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

end forge_topo
