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

end forge_topo
