import ForgeTopo.Strays
open Aeneas Aeneas.Std Result

/-!
# The shipped `topo::links` refines the model's `checkLinks`

`links` (kernel/forge-topo/src/topo.rs, which check.rs's `check_links` calls) as Aeneas
translates it, against verify/forge-topo-check's `Body.checkLinks`. `Reads` says what a `Topo`
instance must give for a model body: its counts and, by index, every field `links` reads, all
entities alive (`check` looks only at compact bodies: check.rs:181–183). `links_refines`: for any
such instance and any sink that appends what it is handed, `links` returns ok (no index out of
bounds, no overflow, no divergence) having appended exactly `checkLinks`.
-/

set_option maxRecDepth 8192

namespace forge_topo

open TopoCheck

/-- What a `Topo` instance reads of model body `b`. -/
structure Reads {T : Type} (TI : topo.Topo T) (t : T) (b : Body) : Prop where
  nv : TI.n_vertices t ⦃ n => n.val = b.nv ⦄
  ne : TI.n_edges t ⦃ n => n.val = b.edges.size ⦄
  nc : TI.n_coedges t ⦃ n => n.val = b.coedges.size ⦄
  nl : TI.n_loops t ⦃ n => n.val = b.loops.size ⦄
  nf : TI.n_faces t ⦃ n => n.val = b.faces.size ⦄
  edge_alive : ∀ e : Usize, e.val < b.edges.size → TI.edge_alive t e ⦃ x => x = true ⦄
  edge_ends : ∀ e : Usize, e.val < b.edges.size →
    TI.edge_ends t e ⦃ p => p.1.val = (b.ed e.val).start ∧ p.2.val = (b.ed e.val).stop ⦄
  edge_n_uses : ∀ e : Usize, e.val < b.edges.size →
    TI.edge_n_uses t e ⦃ n => n.val = (b.ed e.val).uses.length ⦄
  edge_use : ∀ e i : Usize, e.val < b.edges.size → i.val < (b.ed e.val).uses.length →
    TI.edge_use t e i ⦃ c => c.val = (b.ed e.val).uses[i.val]! ⦄
  coedge_edge : ∀ c : Usize, c.val < b.coedges.size →
    TI.coedge_edge t c ⦃ e => e.val = (b.co c.val).edge ⦄
  coedge_loop : ∀ c : Usize, c.val < b.coedges.size →
    TI.coedge_loop t c ⦃ l => l.val = (b.co c.val).lp ⦄
  loop_first : ∀ l : Usize, l.val < b.loops.size → TI.loop_first t l ⦃ x => x.val = (b.lo l.val).first ⦄
  loop_len : ∀ l : Usize, l.val < b.loops.size → TI.loop_len t l ⦃ x => x.val = (b.lo l.val).len ⦄
  loop_face : ∀ l : Usize, l.val < b.loops.size → TI.loop_face t l ⦃ x => x.val = (b.lo l.val).face ⦄
  face_alive : ∀ f : Usize, f.val < b.faces.size → TI.face_alive t f ⦃ x => x = true ⦄
  face_n_loops : ∀ f : Usize, f.val < b.faces.size →
    TI.face_n_loops t f ⦃ n => n.val = (b.faceLoops f.val).length ⦄
  face_loop : ∀ f i : Usize, f.val < b.faces.size → i.val < (b.faceLoops f.val).length →
    TI.face_loop t f i ⦃ l => l.val = (b.faceLoops f.val)[i.val]! ⦄

/-! ## The face pass: `listed` and the faces at fault -/

/-- Entries for loop `l` its face lists, among faces before `f` and the first `i` of face `f`'s. -/
def pre (b : Body) (f i l : Nat) : Nat :=
  (if (b.lo l).face < f then (b.faceLoops (b.lo l).face).count l else 0) +
  (if (b.lo l).face = f then ((b.faceLoops f).take i).count l else 0)

theorem pre_succ (b : Body) (f i l : Nat) (hi : i < (b.faceLoops f).length) :
    pre b f (i + 1) l = pre b f i l +
      (if (b.lo l).face = f ∧ (b.faceLoops f)[i]! = l then 1 else 0) := by
  unfold pre
  rw [List.take_succ, List.count_append]
  have : (b.faceLoops f)[i]? = some (b.faceLoops f)[i]! := by
    rw [getElem!_pos (b.faceLoops f) i hi, List.getElem?_eq_getElem hi]
  rw [this]
  by_cases h1 : (b.lo l).face = f <;> by_cases h2 : (b.faceLoops f)[i]! = l <;>
    simp [h1, h2, List.count_cons, Option.toList] <;> omega

theorem pre_next (b : Body) (f l : Nat) :
    pre b f (b.faceLoops f).length l = pre b (f + 1) 0 l := by
  unfold pre
  by_cases h : (b.lo l).face = f
  · simp [h]
  · have : (b.lo l).face < f + 1 ↔ (b.lo l).face < f := by omega
    simp [h, this]

/-- The model's `listed`, as `pre` once every face is passed. -/
theorem listed_eq (b : Body) (l : Nat) (hl : l < b.loops.size) :
    b.listed l = pre b b.faces.size 0 l := by
  unfold Body.listed pre
  by_cases h : (b.lo l).face < b.faces.size
  · simp [h, hl]
  · simp [h]

theorem u32_succ (x : U32) (h : x.val < 2) : (U32.wrapping_add x 1#u32).val = x.val + 1 := by
  rw [U32.wrapping_add_val_eq]
  apply Nat.mod_eq_of_lt
  have h2 : (1#u32).val = 1 := rfl
  have : (2 : Nat) ^ 32 = 4294967296 := by norm_num
  simp only [UScalar.size, UScalarTy.U32_numBits_eq, h2, this]
  omega

/-- A face's entry `l` is its own loop. -/
def own (b : Body) (f l : Nat) : Bool := l < b.loops.size && (b.lo l).face == f

/-- What the face pass keeps of `listed`: `pre`, saturated at 2. -/
def Kept (b : Body) (f i : Nat) (listed : alloc.vec.Vec U32) : Prop :=
  listed.val.length = b.loops.size ∧ ∀ l < b.loops.size, (listed.val[l]!).val = min 2 (pre b f i l)

variable {T : Type} (TI : topo.Topo T) (t : T) (b : Body)

theorem all_take_succ {α} [Inhabited α] (L : List α) (P : α → Bool) (i : Nat) (hi : i < L.length) :
    (L.take (i + 1)).all P = ((L.take i).all P && P L[i]!) := by
  rw [List.take_succ, List.all_append]
  have : L[i]? = some L[i]! := by rw [getElem!_pos L i hi, List.getElem?_eq_getElem]
  rw [this]; simp

theorem kept_skip (f i : Nat) (u : alloc.vec.Vec U32) (hi : i < (b.faceLoops f).length)
    (hown : own b f (b.faceLoops f)[i]! = false) (hk : Kept b f i u) : Kept b f (i + 1) u := by
  refine ⟨hk.1, fun k hk' => ?_⟩
  rw [hk.2 k hk', pre_succ b f i k hi]
  have : ¬ ((b.lo k).face = f ∧ (b.faceLoops f)[i]! = k) := by
    rintro ⟨h1, h2⟩; rw [h2] at hown; simp [own, hk', h1] at hown
  rw [if_neg this, Nat.add_zero]

theorem kept_own (f i : Nat) (u : alloc.vec.Vec U32) (hi : i < (b.faceLoops f).length)
    (l : Usize) (hl : (b.faceLoops f)[i]! = l.val) (hln : l.val < b.loops.size)
    (hlf : (b.lo l.val).face = f) (x : U32) (hx : x.val = (u.val[l.val]!).val + 1)
    (h2 : (u.val[l.val]!).val < 2) (hk : Kept b f i u) : Kept b f (i + 1) (u.set l x) := by
  refine ⟨by simp [alloc.vec.Vec.set_val_eq, hk.1], fun k hk' => ?_⟩
  rw [pre_succ b f i k hi]
  simp only [alloc.vec.Vec.set_val_eq]
  rw [getElem!_set _ _ _ _ (by rw [hk.1]; exact hk')]
  by_cases hkl : k = l.val
  · subst hkl
    have hc := hk.2 l.val hln
    simp only [ite_true, hx, hlf, hl, and_self]
    rw [hc] at h2 ⊢; omega
  · have : ¬ ((b.lo k).face = f ∧ (b.faceLoops f)[i]! = k) := by
      rintro ⟨_, h2⟩; exact hkl (h2 ▸ hl)
    simp only [hkl, ite_false, this, Nat.add_zero]
    exact hk.2 k hk'

theorem kept_full (f i : Nat) (u : alloc.vec.Vec U32) (hi : i < (b.faceLoops f).length)
    (l : Nat) (hl : (b.faceLoops f)[i]! = l) (hln : l < b.loops.size)
    (h2 : ¬ (u.val[l]!).val < 2) (hk : Kept b f i u) : Kept b f (i + 1) u := by
  refine ⟨hk.1, fun k hk' => ?_⟩
  rw [hk.2 k hk', pre_succ b f i k hi]
  by_cases hkl : k = l
  · subst hkl
    rw [hk.2 k hln] at h2
    split <;> omega
  · have : ¬ ((b.lo k).face = f ∧ (b.faceLoops f)[i]! = k) := by
      rintro ⟨_, h⟩; exact hkl (h ▸ hl)
    rw [if_neg this, Nat.add_zero]


theorem face_entries_spec (R : Reads TI t b) (nl f n : Usize) (hnl : nl.val = b.loops.size)
    (hf : f.val < b.faces.size) (hn : n.val = (b.faceLoops f.val).length)
    (listed : alloc.vec.Vec U32) (ok0 : Bool) (hk : Kept b f.val 0 listed) :
    topo.links_loop0_loop0 TI t nl listed f ok0 n 0#usize ⦃ r =>
      Kept b (f.val + 1) 0 r.1 ∧ r.2 = (ok0 && (b.faceLoops f.val).all (own b f.val)) ⦄ := by
  unfold topo.links_loop0_loop0
  apply loop.spec_decr_nat (fun (y : alloc.vec.Vec U32 × Bool × Usize) => n.val - y.2.2.val)
    (fun y => y.2.2.val ≤ n.val ∧ Kept b f.val y.2.2.val y.1 ∧
      y.2.1 = (ok0 && ((b.faceLoops f.val).take y.2.2.val).all (own b f.val)))
  · rintro ⟨u, ok, i⟩ ⟨hle, ⟨hlen, hu⟩, hok⟩
    simp only at hle hlen hu hok ⊢
    unfold topo.links_loop0_loop0.body
    have hfl := R.face_loop f
    have hlf := R.loop_face
    split
    · rename_i hlt
      have hlt' : i.val < n.val := by scalar_tac
      have hi : i.val < (b.faceLoops f.val).length := by omega
      step with hfl i hf (by scalar_tac) as ⟨l, hl⟩
      split
      · rename_i hge
        step as ⟨i1, hi1⟩
        have hw : i1.val = i.val + 1 := by rw [hi1]; exact succ_val i n hlt'
        have hown : own b f.val (b.faceLoops f.val)[i.val]! = false := by
          rw [← hl]; simp [own]; scalar_tac
        refine ⟨by omega, ?_, ?_, by omega⟩
        · rw [hw]; exact kept_skip b f.val i.val u hi hown ⟨hlen, hu⟩
        · rw [hw, all_take_succ _ _ _ hi, hown]; simp
      · rename_i hge
        have hln : l.val < b.loops.size := by scalar_tac
        step with hlf l hln as ⟨g, hg⟩
        split
        · rename_i hne
          step as ⟨i1, hi1⟩
          have hw : i1.val = i.val + 1 := by rw [hi1]; exact succ_val i n hlt'
          have hne' : g.val ≠ f.val := by
            intro h
            have : g = f := UScalar.eq_of_val_eq h
            rw [this] at hne; exact absurd hne (by simp)
          have hown : own b f.val (b.faceLoops f.val)[i.val]! = false := by
            rw [← hl]; simp [own, ← hg, hne']
          refine ⟨by omega, ?_, ?_, by omega⟩
          · rw [hw]; exact kept_skip b f.val i.val u hi hown ⟨hlen, hu⟩
          · rw [hw, all_take_succ _ _ _ hi, hown]; simp
        · rename_i hne
          have hgf : (b.lo l.val).face = f.val := by
            rw [← hg]; simp only [bne_iff_ne, ne_eq, Decidable.not_not] at hne; rw [hne]
          have hown : own b f.val (b.faceLoops f.val)[i.val]! = true := by
            rw [← hl]; simp [own, hgf, hln]
          have hok' : (ok0 && ((b.faceLoops f.val).take (i.val + 1)).all (own b f.val)) = ok := by
            rw [all_take_succ _ _ _ hi, hown, hok]; simp
          step as ⟨x, hx⟩
          have hx' : x = u.val[l.val]! := by rw [hx, getElem!_pos]
          split
          · rename_i h2
            step as ⟨p, hp1, hp2⟩
            obtain ⟨i3, back⟩ := p
            simp only at hp1 hp2 ⊢
            step as ⟨i4, hi4⟩
            step as ⟨i1, hi1⟩
            have hw : i1.val = i.val + 1 := by rw [hi1]; exact succ_val i n hlt'
            have h2' : (u.val[l.val]!).val < 2 := by rw [← hx']; scalar_tac
            have hv : i4.val = (u.val[l.val]!).val + 1 := by
              rw [hi4, hp1, ← getElem!_pos]; exact u32_succ _ h2'
            refine ⟨by omega, ?_, ?_, by omega⟩
            · rw [hw, hp2]; exact kept_own b f.val i.val u hi l hl.symm hln hgf i4 hv h2' ⟨hlen, hu⟩
            · rw [hw, hok']
          · rename_i h2
            step as ⟨i1, hi1⟩
            have hw : i1.val = i.val + 1 := by rw [hi1]; exact succ_val i n hlt'
            have h2' : ¬ (u.val[l.val]!).val < 2 := by rw [← hx']; scalar_tac
            refine ⟨by omega, ?_, ?_, by omega⟩
            · rw [hw]; exact kept_full b f.val i.val u hi l.val hl.symm hln h2' ⟨hlen, hu⟩
            · rw [hw, hok']
    · rename_i hlt
      have hi : i.val = (b.faceLoops f.val).length := by scalar_tac
      simp only [WP.spec_ok]
      refine ⟨⟨hlen, fun k hk => ?_⟩, ?_⟩
      · rw [hu k hk, hi, pre_next]
      · rw [hok, hi, List.take_length]
  · exact ⟨by simp, hk, by simp⟩

/-- A sink that appends each problem it is handed, as `P` reads it. -/
def Appends {S : Type} (SI : topo.Sink S) (P : S → List Problem) : Prop :=
  ∀ (o : S) (w : U8) (id : Usize),
    SI.back_link o w id ⦃ o' => P o' = P o ++ [Problem.backLink w.val id.val] ⦄

theorem filterMap_range_succ {α} (n : Nat) (g : Nat → Option α) :
    (List.range (n + 1)).filterMap g = (List.range n).filterMap g ++ (g n).toList := by
  rw [List.range_succ, List.filterMap_append]; congr 1

theorem faceOk_eq (f : Nat) : b.faceOk f = (b.faceLoops f).all (own b f) := rfl

theorem face_pass_spec {S : Type} (SI : topo.Sink S) (P : S → List Problem) (hS : Appends SI P)
    (R : Reads TI t b) (nl nf : Usize) (hnl : nl.val = b.loops.size) (hnf : nf.val = b.faces.size)
    (out : S) (listed : alloc.vec.Vec U32) (hk : Kept b 0 0 listed) :
    topo.links_loop0 TI SI t out nl nf listed 0#usize ⦃ r =>
      Kept b b.faces.size 0 r.2 ∧ P r.1 = P out ++ ((List.range b.faces.size).filterMap fun f =>
        if b.faceOk f then none else some (Problem.backLink 0 f)) ⦄ := by
  unfold topo.links_loop0
  apply loop.spec_decr_nat (fun (y : S × alloc.vec.Vec U32 × Usize) => nf.val - y.2.2.val)
    (fun y => y.2.2.val ≤ nf.val ∧ Kept b y.2.2.val 0 y.2.1 ∧
      P y.1 = P out ++ ((List.range y.2.2.val).filterMap fun f =>
        if b.faceOk f then none else some (Problem.backLink 0 f)))
  · rintro ⟨o, u, f⟩ ⟨hle, hk, ho⟩
    simp only at hle hk ho ⊢
    unfold topo.links_loop0.body
    split
    · rename_i hlt
      have hlt' : f.val < nf.val := by scalar_tac
      have hf : f.val < b.faces.size := by omega
      step with R.face_alive f hf as ⟨al, hal⟩
      rw [hal]; simp only [ite_true]
      step with R.face_n_loops f hf as ⟨n, hn⟩
      step with face_entries_spec TI t b R nl f n hnl hf hn u true hk as ⟨r, okf, hr, hokf⟩
      simp only [Bool.true_and] at hokf
      have hw : ∀ f1 : Usize, f1 = f.wrapping_add 1#usize → f1.val = f.val + 1 := by
        intro f1 h; rw [h]; exact succ_val f nf hlt'
      have hacc : ∀ o', P o' = P o ++ (if b.faceOk f.val then none else some (Problem.backLink 0 f.val)).toList →
          P o' = P out ++ ((List.range (f.val + 1)).filterMap fun f =>
            if b.faceOk f then none else some (Problem.backLink 0 f)) := by
        intro o' h; rw [h, filterMap_range_succ, ho, List.append_assoc]
      split
      · rename_i hok
        step as ⟨f1, hf1⟩
        have := hw f1 hf1
        refine ⟨by omega, by rw [this]; exact hr, ?_, by omega⟩
        rw [this]; apply hacc
        rw [faceOk_eq, ← hokf, hok]; simp
      · rename_i hok
        step with hS o 0#u8 f as ⟨o1, ho1⟩
        step as ⟨f1, hf1⟩
        have := hw f1 hf1
        refine ⟨by omega, by rw [this]; exact hr, ?_, by omega⟩
        rw [this]; apply hacc
        rw [ho1, faceOk_eq, ← hokf]; simp at hok; simp [hok]
    · rename_i hlt
      have hf : f.val = b.faces.size := by scalar_tac
      simp only [WP.spec_ok]
      rw [← hf]; exact ⟨hk, ho⟩
  · exact ⟨by simp, hk, by simp⟩

/-! ## The edge pass -/

/-- `edgeOk`'s test of one use. -/
def useOk (e c : Nat) : Bool :=
  c < b.coedges.size && (b.co c).edge == e && (b.co c).lp < b.loops.size &&
    b.listed (b.co c).lp == 1 && (b.loopCoedges (b.co c).lp).contains c

theorem edgeOk_eq (e : Nat) :
    b.edgeOk e = ((b.ed e).start < b.nv && (b.ed e).stop < b.nv && (b.ed e).uses.all (useOk b e)) := by
  unfold Body.edgeOk useOk; rfl

theorem contains_loopCoedges (l c : Nat) :
    (b.loopCoedges l).contains c = (decide ((b.lo l).first ≤ c) && decide (c - (b.lo l).first < (b.lo l).len)) := by
  unfold Body.loopCoedges
  rw [Bool.eq_iff_iff, List.contains_iff_mem, List.mem_map]
  simp only [List.mem_range, Bool.and_eq_true, decide_eq_true_eq]
  constructor
  · rintro ⟨k, hk, rfl⟩; omega
  · rintro ⟨h1, h2⟩; exact ⟨c - (b.lo l).first, h2, by omega⟩

/-- Once every face is passed, `listed` says `== 1` as the model's `listed` does. -/
theorem kept_one (u : alloc.vec.Vec U32) (hk : Kept b b.faces.size 0 u) (l : Nat) (hl : l < b.loops.size) :
    ((u.val[l]!).val = 1) ↔ b.listed l = 1 := by
  rw [hk.2 l hl, listed_eq b l hl]; omega

theorem kept_zero (u : alloc.vec.Vec U32) (hk : Kept b b.faces.size 0 u) (l : Nat) (hl : l < b.loops.size) :
    ((u.val[l]!).val = 0) ↔ b.listed l = 0 := by
  rw [hk.2 l hl, listed_eq b l hl]; omega

theorem sub_val (x y : Usize) (h : y.val ≤ x.val) : (Std.Usize.wrapping_sub x y).val = x.val - y.val := by
  rw [Usize.wrapping_sub_val_eq]
  have hx := x.hBounds
  have hS : UScalar.size UScalarTy.Usize = 2 ^ UScalarTy.Usize.numBits := by simp only [UScalar.size]
  rw [hS]
  have : x.val + (2 ^ UScalarTy.Usize.numBits - y.val) = (x.val - y.val) + 2 ^ UScalarTy.Usize.numBits := by
    omega
  rw [this, Nat.add_mod_right, Nat.mod_eq_of_lt (by omega)]

theorem use_step (e i n c : Usize) (ok0 : Bool) (hlt : i.val < n.val)
    (hi : i.val < (b.ed e.val).uses.length) (hc : c.val = (b.ed e.val).uses[i.val]!)
    (hok : (ok0 && ((b.ed e.val).uses.take i.val).all (useOk b e.val)) = true) :
    ∀ v : Bool, v = useOk b e.val c.val → ∀ i1 : Usize, i1 = i.wrapping_add 1#usize →
      i1.val ≤ n.val ∧ v = (ok0 && ((b.ed e.val).uses.take i1.val).all (useOk b e.val)) ∧
        n.val - i1.val < n.val - i.val := by
  intro v hv i1 hi1
  have hw : i1.val = i.val + 1 := by rw [hi1]; exact succ_val i n hlt
  refine ⟨by omega, ?_, by omega⟩
  rw [hw, all_take_succ _ _ _ hi, ← Bool.and_assoc, hok, ← hc, hv]; rfl

theorem edge_uses_spec (R : Reads TI t b) (nc nl e n : Usize) (hnc : nc.val = b.coedges.size)
    (hnl : nl.val = b.loops.size) (he : e.val < b.edges.size) (hn : n.val = (b.ed e.val).uses.length)
    (listed : alloc.vec.Vec U32) (hk : Kept b b.faces.size 0 listed) (ok0 : Bool) :
    topo.links_loop1_loop0 TI t nc nl listed e ok0 n 0#usize ⦃ r =>
      r = (ok0 && (b.ed e.val).uses.all (useOk b e.val)) ⦄ := by
  unfold topo.links_loop1_loop0
  apply loop.spec_decr_nat (fun (y : Bool × Usize) => n.val - y.2.val)
    (fun y => y.2.val ≤ n.val ∧ y.1 = (ok0 && ((b.ed e.val).uses.take y.2.val).all (useOk b e.val)))
  · rintro ⟨ok, i⟩ ⟨hle, hok⟩
    simp only at hle hok ⊢
    unfold topo.links_loop1_loop0.body
    split
    · rename_i hok1
      split
      · rename_i hlt
        have hlt' : i.val < n.val := by scalar_tac
        have hi : i.val < (b.ed e.val).uses.length := by omega
        step with R.edge_use e i he hi as ⟨c, hc⟩
        have hok' : (ok0 && ((b.ed e.val).uses.take i.val).all (useOk b e.val)) = true := by
          rw [← hok]; exact hok1
        have fin := use_step b e i n c ok0 hlt' hi hc hok'
        clear hc hok hok1 hok'
        split
        · rename_i hcn
          have hcn' : c.val < b.coedges.size := by rw [← hnc]; exact hcn
          step with R.coedge_edge c hcn' as ⟨ce, hce⟩
          split
          · rename_i hee
            rw [hee] at hce; clear hee
            step with R.coedge_loop c hcn' as ⟨l, hl⟩
            split
            · rename_i hln
              have hln' : l.val < b.loops.size := by rw [← hnl]; exact hln
              have hlb : l.val < listed.val.length := by rw [hk.1]; exact hln'
              step as ⟨x, hx⟩
              have hx' : x.val = (listed.val[l.val]!).val := by rw [hx, getElem!_pos]
              split
              · rename_i h1
                have h1' : b.listed l.val = 1 := by
                  rw [← kept_one b listed hk l.val hln', ← hx', h1]; rfl
                step with R.loop_first l hln' as ⟨fs, hfs⟩
                split
                · rename_i hfc
                  have hfc' : fs.val ≤ c.val := hfc
                  step as ⟨d, hd⟩
                  step with R.loop_len l hln' as ⟨len, hlen⟩
                  step as ⟨i1, hi1⟩
                  apply fin _ _ i1 hi1
                  clear fin
                  have hd' : d.val = c.val - fs.val := by rw [hd]; exact sub_val c fs hfc'
                  unfold useOk
                  rw [contains_loopCoedges, ← hl, ← hce, ← hfs, ← hlen]
                  have e1 : (decide (d < len)) = decide (c.val - fs.val < len.val) := by
                    rw [← hd']; rfl
                  simp only [decide_eq_true hcn', decide_eq_true hln', h1', beq_self_eq_true,
                    decide_eq_true hfc', Bool.true_and, Bool.and_true]
                  exact e1
                · rename_i hfc
                  have hfc' : ¬ fs.val ≤ c.val := hfc
                  step as ⟨i1, hi1⟩
                  apply fin _ _ i1 hi1
                  clear fin
                  unfold useOk
                  rw [contains_loopCoedges, ← hl, ← hfs]
                  simp [hfc']
              · rename_i h1
                step as ⟨i1, hi1⟩
                apply fin _ _ i1 hi1
                clear fin
                have h1' : b.listed l.val ≠ 1 := by
                  intro h; apply h1; apply UScalar.eq_of_val_eq
                  rw [hx']; exact (kept_one b listed hk l.val hln').2 h
                unfold useOk; rw [← hl]; simp [h1']
            · rename_i hln
              have hln' : ¬ l.val < b.loops.size := by rw [← hnl]; exact hln
              step as ⟨i1, hi1⟩
              apply fin _ _ i1 hi1
              clear fin
              unfold useOk; rw [← hl]; simp [hln']
          · rename_i hee
            step as ⟨i1, hi1⟩
            apply fin _ _ i1 hi1
            clear fin
            unfold useOk
            have : (b.co c.val).edge ≠ e.val := by
              rw [← hce]; intro h; exact hee (UScalar.eq_of_val_eq h)
            simp [this]
        · rename_i hcn
          have hcn' : ¬ c.val < b.coedges.size := by rw [← hnc]; exact hcn
          step as ⟨i1, hi1⟩
          apply fin _ _ i1 hi1
          clear fin
          unfold useOk; simp [hcn']
      · rename_i hlt
        have hi : i.val = (b.ed e.val).uses.length := by scalar_tac
        simp only [WP.spec_ok]
        rw [hok, hi, List.take_length]
    · rename_i hok1
      simp only [WP.spec_ok]
      simp only [Bool.not_eq_true] at hok1
      rw [hok1] at hok
      rw [hok1, ← List.take_append_drop i.val (b.ed e.val).uses, List.all_append]
      rw [← Bool.and_assoc, ← hok]; rfl
  · exact ⟨by simp, by simp⟩

theorem edge_pass_spec {S : Type} (SI : topo.Sink S) (P : S → List Problem) (hS : Appends SI P)
    (R : Reads TI t b) (nv ne nc nl : Usize) (hnv : nv.val = b.nv) (hne : ne.val = b.edges.size)
    (hnc : nc.val = b.coedges.size) (hnl : nl.val = b.loops.size)
    (listed : alloc.vec.Vec U32) (hk : Kept b b.faces.size 0 listed) (out : S) :
    topo.links_loop1 TI SI t out nv ne nc nl listed 0#usize ⦃ r =>
      P r = P out ++ ((List.range b.edges.size).filterMap fun e =>
        if b.edgeOk e then none else some (Problem.backLink 1 e)) ⦄ := by
  unfold topo.links_loop1
  apply loop.spec_decr_nat (fun (y : S × Usize) => ne.val - y.2.val)
    (fun y => y.2.val ≤ ne.val ∧
      P y.1 = P out ++ ((List.range y.2.val).filterMap fun e =>
        if b.edgeOk e then none else some (Problem.backLink 1 e)))
  · rintro ⟨o, e⟩ ⟨hle, ho⟩
    simp only at hle ho ⊢
    unfold topo.links_loop1.body
    split
    · rename_i hlt
      have hlt' : e.val < ne.val := by scalar_tac
      have he : e.val < b.edges.size := by omega
      have hacc : ∀ o', P o' = P o ++ (if b.edgeOk e.val then none else some (Problem.backLink 1 e.val)).toList →
          P o' = P out ++ ((List.range (e.val + 1)).filterMap fun e =>
            if b.edgeOk e then none else some (Problem.backLink 1 e)) := by
        intro o' h; rw [h, filterMap_range_succ, ho, List.append_assoc]
      step with R.edge_alive e he as ⟨al, hal⟩
      rw [hal]; simp only [ite_true]
      step with R.edge_ends e he as ⟨a, a2, ha1, ha2⟩
      split
      · rename_i hav
        simp only [Std.bind_ok]
        have : a.val < b.nv := by rw [← hnv]; exact hav
        have hok1 : decide (a2 < nv) = (decide ((b.ed e.val).start < b.nv) && decide ((b.ed e.val).stop < b.nv)) := by
          rw [← ha1, ← ha2, decide_eq_true this, Bool.true_and, ← hnv]; rfl
        generalize decide (a2 < nv) = ok1 at hok1 ⊢
        step with R.edge_n_uses e he as ⟨n, hn⟩
        step with edge_uses_spec TI t b R nc nl e n hnc hnl he hn listed hk ok1 as ⟨ok2, hok2⟩
        have hok2' : ok2 = b.edgeOk e.val := by rw [hok2, hok1, edgeOk_eq]
        have hw : ∀ e1 : Usize, e1 = e.wrapping_add 1#usize → e1.val = e.val + 1 := by
          intro e1 h; rw [h]; exact succ_val e ne hlt'
        split
        · rename_i hok
          step as ⟨e1, he1⟩
          refine ⟨by rw [hw e1 he1]; omega, ?_, by rw [hw e1 he1]; omega⟩
          rw [hw e1 he1]; apply hacc; rw [← hok2', hok]; simp
        · rename_i hok
          step with hS o 1#u8 e as ⟨o1, ho1⟩
          step as ⟨e1, he1⟩
          refine ⟨by rw [hw e1 he1]; omega, ?_, by rw [hw e1 he1]; omega⟩
          rw [hw e1 he1]; apply hacc; rw [ho1, ← hok2']; simp at hok; simp [hok]
      · rename_i hav
        simp only [Std.bind_ok]
        have : ¬ a.val < b.nv := by rw [← hnv]; exact hav
        have hok1 : false = (decide ((b.ed e.val).start < b.nv) && decide ((b.ed e.val).stop < b.nv)) := by
          rw [← ha1, decide_eq_false this, Bool.false_and]
        generalize false = ok1 at hok1 ⊢
        step with R.edge_n_uses e he as ⟨n, hn⟩
        step with edge_uses_spec TI t b R nc nl e n hnc hnl he hn listed hk ok1 as ⟨ok2, hok2⟩
        have hok2' : ok2 = b.edgeOk e.val := by rw [hok2, hok1, edgeOk_eq]
        have hw : ∀ e1 : Usize, e1 = e.wrapping_add 1#usize → e1.val = e.val + 1 := by
          intro e1 h; rw [h]; exact succ_val e ne hlt'
        split
        · rename_i hok
          step as ⟨e1, he1⟩
          refine ⟨by rw [hw e1 he1]; omega, ?_, by rw [hw e1 he1]; omega⟩
          rw [hw e1 he1]; apply hacc; rw [← hok2', hok]; simp
        · rename_i hok
          step with hS o 1#u8 e as ⟨o1, ho1⟩
          step as ⟨e1, he1⟩
          refine ⟨by rw [hw e1 he1]; omega, ?_, by rw [hw e1 he1]; omega⟩
          rw [hw e1 he1]; apply hacc; rw [ho1, ← hok2']; simp at hok; simp [hok]
    · rename_i hlt
      have he : e.val = b.edges.size := by scalar_tac
      simp only [WP.spec_ok]
      rw [← he]; exact ho
  · exact ⟨by simp, by simp⟩

end forge_topo
