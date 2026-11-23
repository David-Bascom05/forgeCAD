import ForgeTopo.Links
import TopoCheck.Flood
open Aeneas Aeneas.Std Result

/-!
# The shipped `topo::shell_faces` refines the model's `checkShellFaces`

`shell_faces` (kernel/forge-topo/src/topo.rs; check.rs's `check_shell_faces` calls it) as Aeneas
translates it, against verify/forge-topo-check's `Body.checkShellFaces`, for bodies whose links
are sound (`checkLinks = []`: `check_known` runs it only then). `ShellReads` adds to `Reads` what
an instance gives of the shells; ids fit in u32, as a body's are (body.rs's ids are u32).
-/

set_option maxRecDepth 8192

namespace forge_topo

open TopoCheck

/-- What a `ShellTopo` instance reads of model body `b`. -/
structure ShellReads {T : Type} (SI : topo_shell.ShellTopo T) (t : T) (b : Body) : Prop where
  base : Reads SI.topoTopoInst t b
  ns : SI.n_shells t ⦃ n => n.val = b.shells.size ⦄
  face_shell : ∀ f : Usize, f.val < b.faces.size → SI.face_shell t f ⦃ x => x.val = b.faceShell f.val ⦄
  shell_n_faces : ∀ s : Usize, s.val < b.shells.size →
    SI.shell_n_faces t s ⦃ n => n.val = (b.shellList s.val).length ⦄
  shell_face : ∀ s i : Usize, s.val < b.shells.size → i.val < (b.shellList s.val).length →
    SI.shell_face t s i ⦃ f => f.val = (b.shellList s.val)[i.val]! ⦄
  /-- Shell ids are u32 and `u32::MAX` is no shell's (the flood's mark for none). -/
  ns_lt : b.shells.size < U32.max
  /-- Face ids are u32. -/
  nf_le : b.faces.size ≤ U32.max

/-- Below `u32::MAX`, one more is still a usize (usize has 32 bits or more). -/
theorem succ_lt_usize (x : Nat) (h : x < U32.max) : x + 1 < 2 ^ UScalarTy.Usize.numBits := by
  have h32 : 32 ≤ UScalarTy.Usize.numBits := by
    rw [UScalarTy.Usize_numBits_eq]; rcases System.Platform.numBits_eq with h | h <;> rw [h] <;> omega
  have : 2 ^ 32 ≤ 2 ^ UScalarTy.Usize.numBits := Nat.pow_le_pow_right (by norm_num) h32
  have hm : U32.max = 2 ^ 32 - 1 := by rw [U32.max_eq]; rfl
  omega

/-! ## What sound links give -/

/-- The indices `shell_faces` follows, in range once `checkLinks = []`. -/
structure Sound (b : Body) : Prop where
  uses : ∀ e < b.edges.size, ∀ c ∈ (b.ed e).uses,
    c < b.coedges.size ∧ (b.co c).lp < b.loops.size ∧ b.faceOf c < b.faces.size
  loops : ∀ f < b.faces.size, ∀ l ∈ b.faceLoops f,
    l < b.loops.size ∧ (b.lo l).face = f ∧ b.listed l = 1
  coedges : ∀ l < b.loops.size, b.listed l = 1 → ∀ c ∈ b.loopCoedges l,
    c < b.coedges.size ∧ (b.co c).edge < b.edges.size ∧ c ∈ (b.ed (b.co c).edge).uses

theorem sound_of (b : Body) (h : b.checkLinks = []) : Sound b := by
  have W := b.links_ok h
  unfold Body.checkLinks at h
  simp only [List.append_eq_nil_iff, List.filterMap_eq_nil_iff, List.mem_range] at h
  obtain ⟨⟨_, he⟩, hl⟩ := h
  have he' : ∀ e < b.edges.size, b.edgeOk e = true := by
    intro e hel; have := he e hel; revert this; split <;> simp_all
  have hl' : ∀ l < b.loops.size, (b.loopSkip l || b.loopOk l) = true := by
    intro l hll; have := hl l hll; revert this; split <;> simp_all
  refine ⟨?_, ?_, ?_⟩
  · intro e hel c hc
    have := he' e hel
    simp only [Body.edgeOk, Bool.and_eq_true, decide_eq_true_eq, List.all_eq_true] at this
    have hc' := this.2 c hc
    simp only [Bool.and_eq_true, decide_eq_true_eq, beq_iff_eq] at hc'
    refine ⟨hc'.1.1.1.1, hc'.1.1.2, ?_⟩
    exact (W.listed_le _ hc'.1.1.2).1
  · intro f hf l hl
    have ⟨hln, hlf⟩ := W.face_loops f hf l hl
    refine ⟨hln, hlf, ?_⟩
    have hle := (W.listed_le l hln).2
    have : b.listed l ≠ 0 := by
      unfold Body.listed
      rw [if_pos ⟨hln, by rw [hlf]; exact hf⟩, hlf]
      exact Nat.pos_iff_ne_zero.1 (List.count_pos_iff.2 hl)
    omega
  · intro l hln h1 c hc
    have := hl' l hln
    have hskip : b.loopSkip l = false := by simp [Body.loopSkip, h1]
    rw [hskip, Bool.false_or] at this
    simp only [Body.loopOk, Bool.and_eq_true, decide_eq_true_eq, List.all_eq_true] at this
    have hc' := this.2 c hc
    simp only [Bool.and_eq_true, decide_eq_true_eq, beq_iff_eq] at hc'
    exact ⟨hc'.1.1.1, hc'.1.2, by simpa using hc'.2⟩

/-- Every neighbour of a face is a face. -/
theorem nbrs_lt (b : Body) (H : Sound b) : ∀ f, ∀ y ∈ b.nbrs f, y < b.faces.size := by
  intro f y hy
  unfold Body.nbrs at hy
  simp only [List.mem_flatMap, List.mem_map] at hy
  obtain ⟨l, hl, c, hc, u, hu, rfl⟩ := hy
  by_cases hf : f < b.faces.size
  · have ⟨hln, _, h1⟩ := H.loops f hf l hl
    have ⟨_, hce, _⟩ := H.coedges l hln h1 c hc
    exact (H.uses _ hce u hu).2.2
  · have : b.faceLoops f = [] := by
      unfold Body.faceLoops; simp [Array.getD_eq_getD_getElem?, Nat.not_lt.1 hf]
    simp [this] at hl

/-! ## The shells' own lists: the first pass -/

/-- An entry `f` shell `s` should not list. -/
def badEntry (b : Body) (s f : Nat) : Bool := !(decide (f < b.faces.size) && b.faceShell f == s)

/-- `bad`'s first clause: a shell that lists nothing, or a face it should not. -/
def bad1 (b : Body) (s : Nat) : Bool :=
  decide (s < b.shells.size) && ((b.shellList s).isEmpty || (b.shellList s).any (badEntry b s))

/-- Entries for face `f` its shell lists, among shells before `s` and the first `i` of `s`'s. -/
def preS (b : Body) (s i f : Nat) : Nat :=
  (if b.faceShell f < s then (b.shellList (b.faceShell f)).count f else 0) +
  (if b.faceShell f = s then ((b.shellList s).take i).count f else 0)

theorem preS_succ (b : Body) (s i f : Nat) (hi : i < (b.shellList s).length) :
    preS b s (i + 1) f = preS b s i f +
      (if b.faceShell f = s ∧ (b.shellList s)[i]! = f then 1 else 0) := by
  unfold preS
  rw [List.take_add_one, List.count_append]
  have : (b.shellList s)[i]? = some (b.shellList s)[i]! := by
    rw [getElem!_pos (b.shellList s) i hi, List.getElem?_eq_getElem hi]
  rw [this]
  by_cases h1 : b.faceShell f = s <;> by_cases h2 : (b.shellList s)[i]! = f <;>
    simp [h1, h2, List.count_cons, Option.toList] <;> omega

theorem preS_next (b : Body) (s f : Nat) :
    preS b s (b.shellList s).length f = preS b (s + 1) 0 f := by
  unfold preS
  by_cases h : b.faceShell f = s
  · simp [h]
  · have : b.faceShell f < s + 1 ↔ b.faceShell f < s := by omega
    simp [h, this]

theorem listedIn_eq (b : Body) (f : Nat) : b.listedIn f = preS b b.shells.size 0 f := by
  unfold Body.listedIn preS
  by_cases h : b.faceShell f < b.shells.size
  · simp [h]
  · simp [h]

/-- What the first pass keeps of `listed`: `preS`, saturated at 2. -/
def KeptS (b : Body) (s i : Nat) (listed : alloc.vec.Vec U32) : Prop :=
  listed.val.length = b.faces.size ∧ ∀ f < b.faces.size, (listed.val[f]!).val = min 2 (preS b s i f)

/-- What the first pass keeps of `bad`, shell `k` done to entry `i` (`cur`: its own verdict). -/
def BadS (b : Body) (k : Nat) (cur : Bool) (bad : alloc.vec.Vec Bool) : Prop :=
  bad.val.length = b.shells.size + 1 ∧
    ∀ s ≤ b.shells.size, bad.val[s]! = ((decide (s < k) && bad1 b s) || (decide (s = k) && cur))

theorem keptS_skip (b : Body) (k i : Nat) (u : alloc.vec.Vec U32) (hi : i < (b.shellList k).length)
    (hn : ¬ ((b.shellList k)[i]! < b.faces.size ∧ b.faceShell (b.shellList k)[i]! = k))
    (hk : KeptS b k i u) : KeptS b k (i + 1) u := by
  refine ⟨hk.1, fun f hf => ?_⟩
  rw [hk.2 f hf, preS_succ b k i f hi]
  have : ¬ (b.faceShell f = k ∧ (b.shellList k)[i]! = f) := by
    rintro ⟨h1, h2⟩; apply hn; rw [h2]; exact ⟨hf, h1⟩
  rw [if_neg this, Nat.add_zero]

theorem keptS_own (b : Body) (k i : Nat) (u : alloc.vec.Vec U32) (hi : i < (b.shellList k).length)
    (f : Usize) (hf : (b.shellList k)[i]! = f.val) (hfn : f.val < b.faces.size)
    (hfs : b.faceShell f.val = k) (x : U32) (hx : x.val = (u.val[f.val]!).val + 1)
    (h2 : (u.val[f.val]!).val < 2) (hk : KeptS b k i u) : KeptS b k (i + 1) (u.set f x) := by
  refine ⟨by simp [alloc.vec.Vec.set_val_eq, hk.1], fun g hg => ?_⟩
  rw [preS_succ b k i g hi]
  simp only [alloc.vec.Vec.set_val_eq]
  rw [getElem!_set _ _ _ _ (by rw [hk.1]; exact hg)]
  by_cases hgf : g = f.val
  · subst hgf
    have hc := hk.2 f.val hfn
    simp only [ite_true, hx, hfs, hf, and_self]
    rw [hc] at h2 ⊢; omega
  · have : ¬ (b.faceShell g = k ∧ (b.shellList k)[i]! = g) := by
      rintro ⟨_, h⟩; exact hgf (h ▸ hf)
    simp only [hgf, ite_false, this, Nat.add_zero]
    exact hk.2 g hg

theorem keptS_full (b : Body) (k i : Nat) (u : alloc.vec.Vec U32) (hi : i < (b.shellList k).length)
    (f : Nat) (hf : (b.shellList k)[i]! = f) (hfn : f < b.faces.size)
    (h2 : ¬ (u.val[f]!).val < 2) (hk : KeptS b k i u) : KeptS b k (i + 1) u := by
  refine ⟨hk.1, fun g hg => ?_⟩
  rw [hk.2 g hg, preS_succ b k i g hi]
  by_cases hgf : g = f
  · subst hgf
    rw [hk.2 g hfn] at h2
    split <;> omega
  · have : ¬ (b.faceShell g = k ∧ (b.shellList k)[i]! = g) := by
      rintro ⟨_, h⟩; exact hgf (h ▸ hf)
    rw [if_neg this, Nat.add_zero]

theorem badS_set (b : Body) (k : Nat) (hk : k < b.shells.size) (cur : Bool) (bad : alloc.vec.Vec Bool)
    (h : BadS b k cur bad) (ks : Usize) (hks : ks.val = k) : BadS b k true (bad.set ks true) := by
  refine ⟨by simp [alloc.vec.Vec.set_val_eq, h.1], fun s hs => ?_⟩
  simp only [alloc.vec.Vec.set_val_eq]
  rw [getElem!_set _ _ _ _ (by rw [h.1]; omega)]
  by_cases hsk : s = k
  · subst hsk; simp [hks]
  · rw [if_neg (by rw [hks]; exact hsk), h.2 s hs]; simp [hsk]

variable {T : Type} (SI : topo_shell.ShellTopo T) (t : T) (b : Body)

theorem shell_entries_spec (R : ShellReads SI t b) (nf s n : Usize) (hnf : nf.val = b.faces.size)
    (hs : s.val < b.shells.size) (hn : n.val = (b.shellList s.val).length)
    (bad : alloc.vec.Vec Bool) (listed : alloc.vec.Vec U32)
    (hb : BadS b s.val (b.shellList s.val).isEmpty bad) (hk : KeptS b s.val 0 listed) :
    topo_shell.shell_faces_loop0_loop0 SI t nf bad listed s n 0#usize ⦃ r =>
      BadS b (s.val + 1) false r.1 ∧ KeptS b (s.val + 1) 0 r.2 ⦄ := by
  unfold topo_shell.shell_faces_loop0_loop0
  apply loop.spec_decr_nat (fun (y : alloc.vec.Vec Bool × alloc.vec.Vec U32 × Usize) => n.val - y.2.2.val)
    (fun y => y.2.2.val ≤ n.val ∧
      BadS b s.val ((b.shellList s.val).isEmpty || ((b.shellList s.val).take y.2.2.val).any (badEntry b s.val)) y.1 ∧
      KeptS b s.val y.2.2.val y.2.1)
  · rintro ⟨bd, u, i⟩ ⟨hle, hbd, hu⟩
    simp only at hle hbd hu ⊢
    unfold topo_shell.shell_faces_loop0_loop0.body
    split
    · rename_i hlt
      have hlt' : i.val < n.val := by scalar_tac
      have hi : i.val < (b.shellList s.val).length := by omega
      have hw : ∀ i1 : Usize, i1 = i.wrapping_add 1#usize → i1.val = i.val + 1 := by
        intro i1 h; rw [h]; exact succ_val i n hlt'
      have htk := all_take_succ (b.shellList s.val) (badEntry b s.val) i.val hi
      have hany : ((b.shellList s.val).take (i.val + 1)).any (badEntry b s.val) =
          (((b.shellList s.val).take i.val).any (badEntry b s.val) || badEntry b s.val (b.shellList s.val)[i.val]!) := by
        rw [List.take_add_one, List.any_append]
        have : (b.shellList s.val)[i.val]? = some (b.shellList s.val)[i.val]! := by
          rw [getElem!_pos (b.shellList s.val) i.val hi, List.getElem?_eq_getElem]
        rw [this]; simp
      have hsb : s.val < bd.val.length := by rw [hbd.1]; omega
      step with R.shell_face s i hs hi as ⟨f, hf⟩
      split
      · rename_i hge
        have hge' : ¬ f.val < b.faces.size := by rw [← hnf]; scalar_tac
        step as ⟨p, hp1, hp2⟩
        obtain ⟨i0, back⟩ := p
        simp only at hp1 hp2 ⊢
        step as ⟨i1, hi1⟩
        refine ⟨by rw [hw i1 hi1]; omega, ?_, ?_, by rw [hw i1 hi1]; omega⟩
        · rw [hw i1 hi1, hany, hp2]
          have : badEntry b s.val (b.shellList s.val)[i.val]! = true := by
            rw [← hf]; simp [badEntry, hge']
          rw [this, Bool.or_true, Bool.or_true]
          exact badS_set b s.val hs _ bd hbd s rfl
        · rw [hw i1 hi1]
          exact keptS_skip b s.val i.val u hi (by rw [← hf]; intro h; exact hge' h.1) hu
      · rename_i hge
        have hfn : f.val < b.faces.size := by rw [← hnf]; scalar_tac
        step with R.base.face_alive f hfn as ⟨al, hal⟩
        rw [hal]; simp only [ite_true]
        step with R.face_shell f hfn as ⟨fs, hfs⟩
        split
        · rename_i hne
          have hne' : b.faceShell f.val ≠ s.val := by
            rw [← hfs]; intro h; have : fs = s := UScalar.eq_of_val_eq h
            rw [this] at hne; exact absurd hne (by simp)
          step as ⟨p, hp1, hp2⟩
          obtain ⟨i0, back⟩ := p
          simp only at hp1 hp2 ⊢
          step as ⟨i1, hi1⟩
          refine ⟨by rw [hw i1 hi1]; omega, ?_, ?_, by rw [hw i1 hi1]; omega⟩
          · rw [hw i1 hi1, hany, hp2]
            have : badEntry b s.val (b.shellList s.val)[i.val]! = true := by
              rw [← hf]; simp [badEntry, hne']
            rw [this, Bool.or_true, Bool.or_true]
            exact badS_set b s.val hs _ bd hbd s rfl
          · rw [hw i1 hi1]
            exact keptS_skip b s.val i.val u hi (by rw [← hf]; intro h; exact hne' h.2) hu
        · rename_i hne
          have hfs' : b.faceShell f.val = s.val := by
            rw [← hfs]; simp only [bne_iff_ne, ne_eq, Decidable.not_not] at hne; rw [hne]
          have hgood : badEntry b s.val (b.shellList s.val)[i.val]! = false := by
            rw [← hf]; simp [badEntry, hfn, hfs']
          have hbd' : BadS b s.val ((b.shellList s.val).isEmpty ||
              ((b.shellList s.val).take (i.val + 1)).any (badEntry b s.val)) bd := by
            rw [hany, hgood, Bool.or_false]; exact hbd
          have hfb : f.val < u.val.length := by rw [hu.1]; exact hfn
          step as ⟨x, hx⟩
          have hx' : x.val = (u.val[f.val]!).val := by rw [hx, getElem!_pos]
          split
          · rename_i h2
            step as ⟨p, hp1, hp2⟩
            obtain ⟨i3, back⟩ := p
            simp only at hp1 hp2 ⊢
            step as ⟨i4, hi4⟩
            step as ⟨i1, hi1⟩
            have h2' : (u.val[f.val]!).val < 2 := by rw [← hx']; scalar_tac
            have hv : i4.val = (u.val[f.val]!).val + 1 := by
              rw [hi4, hp1, ← getElem!_pos]; exact u32_succ _ h2'
            refine ⟨by rw [hw i1 hi1]; omega, by rw [hw i1 hi1]; exact hbd', ?_, by rw [hw i1 hi1]; omega⟩
            rw [hw i1 hi1, hp2]
            exact keptS_own b s.val i.val u hi f hf.symm hfn hfs' i4 hv h2' hu
          · rename_i h2
            step as ⟨i1, hi1⟩
            have h2' : ¬ (u.val[f.val]!).val < 2 := by rw [← hx']; scalar_tac
            refine ⟨by rw [hw i1 hi1]; omega, by rw [hw i1 hi1]; exact hbd', ?_, by rw [hw i1 hi1]; omega⟩
            rw [hw i1 hi1]
            exact keptS_full b s.val i.val u hi f.val hf.symm hfn h2' hu
    · rename_i hlt
      have hi : i.val = (b.shellList s.val).length := by scalar_tac
      simp only [WP.spec_ok]
      rw [hi, List.take_length] at hbd
      rw [hi] at hu
      refine ⟨⟨hbd.1, fun s' hs' => ?_⟩, ⟨hu.1, fun f hf => ?_⟩⟩
      · rw [hbd.2 s' hs']
        by_cases h : s' = s.val
        · subst h; simp [bad1, hs]
        · have : decide (s' ≤ s.val) = decide (s' < s.val) := by
            simp only [decide_eq_decide]; omega
          simp [h, this]
      · rw [hu.2 f hf, preS_next]
  · refine ⟨by simp, by simpa using hb, by simpa using hk⟩

theorem shell_pass_spec (R : ShellReads SI t b) (ns nf : Usize) (hns : ns.val = b.shells.size)
    (hnf : nf.val = b.faces.size) (bad : alloc.vec.Vec Bool) (listed : alloc.vec.Vec U32)
    (hb : BadS b 0 false bad) (hk : KeptS b 0 0 listed) :
    topo_shell.shell_faces_loop0 SI t ns nf bad listed 0#usize ⦃ r =>
      BadS b b.shells.size false r.1 ∧ KeptS b b.shells.size 0 r.2 ⦄ := by
  unfold topo_shell.shell_faces_loop0
  apply loop.spec_decr_nat (fun (y : alloc.vec.Vec Bool × alloc.vec.Vec U32 × Usize) => ns.val - y.2.2.val)
    (fun y => y.2.2.val ≤ ns.val ∧ BadS b y.2.2.val false y.1 ∧ KeptS b y.2.2.val 0 y.2.1)
  · rintro ⟨bd, u, k⟩ ⟨hle, hbd, hu⟩
    simp only at hle hbd hu ⊢
    unfold topo_shell.shell_faces_loop0.body
    split
    · rename_i hlt
      have hlt' : k.val < ns.val := by scalar_tac
      have hk' : k.val < b.shells.size := by omega
      have hw : ∀ k1 : Usize, k1 = k.wrapping_add 1#usize → k1.val = k.val + 1 := by
        intro k1 h; rw [h]; exact succ_val k ns hlt'
      have hkb : k.val < bd.val.length := by rw [hbd.1]; omega
      step with R.shell_n_faces k hk' as ⟨n, hn⟩
      split
      · rename_i h0
        have he : (b.shellList k.val).isEmpty = true := by
          rw [List.isEmpty_iff, ← List.length_eq_zero_iff, ← hn, h0]; rfl
        step as ⟨p, hp1, hp2⟩
        obtain ⟨i0, back⟩ := p
        simp only at hp1 hp2 ⊢
        have hbd' : BadS b k.val (b.shellList k.val).isEmpty (back true) := by
          rw [hp2, he]; exact badS_set b k.val hk' false bd hbd k rfl
        step with shell_entries_spec SI t b R nf k n hnf hk' hn (back true) u hbd' hu as ⟨bd2, u2, hb2, hu2⟩
        step as ⟨k1, hk1⟩
        rw [hw k1 hk1]; exact ⟨by omega, hb2, hu2, by omega⟩
      · rename_i h0
        have he : (b.shellList k.val).isEmpty = false := by
          rw [Bool.eq_false_iff]; intro h
          rw [List.isEmpty_iff, ← List.length_eq_zero_iff, ← hn] at h
          exact h0 (UScalar.eq_of_val_eq h)
        simp only [Std.bind_ok]
        have hbd' : BadS b k.val (b.shellList k.val).isEmpty bd := by rw [he]; exact hbd
        step with shell_entries_spec SI t b R nf k n hnf hk' hn bd u hbd' hu as ⟨bd2, u2, hb2, hu2⟩
        step as ⟨k1, hk1⟩
        rw [hw k1 hk1]; exact ⟨by omega, hb2, hu2, by omega⟩
    · rename_i hlt
      have hk : k.val = b.shells.size := by scalar_tac
      simp only [WP.spec_ok]
      rw [← hk]; exact ⟨hbd, hu⟩
  · exact ⟨by simp, hb, hk⟩

theorem beq_symm (x y : Nat) : (x == y) = (y == x) := by
  by_cases h : x = y <;> simp [h, eq_comm]

/-! ## The faces listed other than once, the edges between shells: the second and third pass -/

/-- `bad`'s second clause, over the first `k` faces. -/
def badF (b : Body) (k s : Nat) : Bool := (List.range k).any fun f => b.listedIn f != 1 && b.atS f == s

/-- An edge whose faces are in more than one shell, at the first's (clamped). -/
def splitAt (b : Body) (e s : Nat) : Bool :=
  match (b.ed e).uses with
  | [] => false
  | c0 :: _ => (b.ed e).uses.any (fun c => b.faceShell (b.faceOf c) != b.faceShell (b.faceOf c0)) &&
      b.atS (b.faceOf c0) == s

/-- `bad`'s third clause, over the first `k` edges. -/
def badE (b : Body) (k s : Nat) : Bool := (List.range k).any fun e => splitAt b e s

theorem bad_eq (b : Body) (s : Nat) : b.bad s = (bad1 b s || badF b b.faces.size s || badE b b.edges.size s) := by
  unfold Body.bad bad1 badF badE splitAt badEntry; rfl

/-- `bad` as the second and third pass keep it. -/
def BadT (b : Body) (kf ke : Nat) (bad : alloc.vec.Vec Bool) : Prop :=
  bad.val.length = b.shells.size + 1 ∧
    ∀ s ≤ b.shells.size, bad.val[s]! = (bad1 b s || badF b kf s || badE b ke s)

theorem any_range_succ (k : Nat) (p : Nat → Bool) :
    (List.range (k + 1)).any p = ((List.range k).any p || p k) := by
  rw [List.range_succ, List.any_append]; simp

/-- Setting `bad[a]` when the step's test holds at `a` and nowhere else. -/
theorem badT_step (b : Body) (kf ke : Nat) (bad : alloc.vec.Vec Bool) (h : BadT b kf ke bad)
    (kf' ke' : Nat) (q : Nat → Bool) (a : Usize) (ha : a.val ≤ b.shells.size)
    (hq : ∀ s, q s = (s == a.val))
    (heq : ∀ s, (badF b kf' s || badE b ke' s) = (badF b kf s || badE b ke s || q s)) :
    BadT b kf' ke' (bad.set a true) := by
  refine ⟨by simp [alloc.vec.Vec.set_val_eq, h.1], fun s hs => ?_⟩
  simp only [alloc.vec.Vec.set_val_eq]
  rw [getElem!_set _ _ _ _ (by rw [h.1]; omega), Bool.or_assoc, heq s, hq s]
  by_cases hsa : s = a.val
  · subst hsa; simp
  · have : (s == a.val) = false := by simp [hsa]
    rw [if_neg hsa, h.2 s hs, this]; simp [Bool.or_assoc]

theorem badT_same (b : Body) (kf ke : Nat) (bad : alloc.vec.Vec Bool) (h : BadT b kf ke bad)
    (kf' ke' : Nat) (heq : ∀ s, (badF b kf' s || badE b ke' s) = (badF b kf s || badE b ke s)) :
    BadT b kf' ke' bad :=
  ⟨h.1, fun s hs => by rw [h.2 s hs, Bool.or_assoc, Bool.or_assoc, heq s]⟩

theorem face_listed_spec (R : ShellReads SI t b) (ns nf : Usize) (hns : ns.val = b.shells.size)
    (hnf : nf.val = b.faces.size) (listed : alloc.vec.Vec U32) (hk : KeptS b b.shells.size 0 listed)
    (bad : alloc.vec.Vec Bool) (hb : BadT b 0 0 bad) :
    topo_shell.shell_faces_loop1 SI t ns nf bad listed 0#usize ⦃ r => BadT b b.faces.size 0 r ⦄ := by
  unfold topo_shell.shell_faces_loop1
  apply loop.spec_decr_nat (fun (y : alloc.vec.Vec Bool × Usize) => nf.val - y.2.val)
    (fun y => y.2.val ≤ nf.val ∧ BadT b y.2.val 0 y.1)
  · rintro ⟨bd, f⟩ ⟨hle, hbd⟩
    simp only at hle hbd ⊢
    unfold topo_shell.shell_faces_loop1.body
    split
    · rename_i hlt
      have hlt' : f.val < nf.val := by scalar_tac
      have hf : f.val < b.faces.size := by omega
      have hw : ∀ f1 : Usize, f1 = f.wrapping_add 1#usize → f1.val = f.val + 1 := by
        intro f1 h; rw [h]; exact succ_val f nf hlt'
      have hstep : ∀ s, badF b (f.val + 1) s = (badF b f.val s || (b.listedIn f.val != 1 && b.atS f.val == s)) := by
        intro s; unfold badF; rw [any_range_succ]
      step with R.base.face_alive f hf as ⟨al, hal⟩
      rw [hal]; simp only [ite_true]
      have hfl : f.val < listed.val.length := by rw [hk.1]; exact hf
      step as ⟨x, hx⟩
      have hx' : x.val = (listed.val[f.val]!).val := by rw [hx, getElem!_pos]
      have hli : (listed.val[f.val]!).val = min 2 (b.listedIn f.val) := by
        rw [hk.2 f.val hf, listedIn_eq]
      split
      · rename_i h1
        have h1' : b.listedIn f.val ≠ 1 := by
          intro h
          have : x = 1#u32 := UScalar.eq_of_val_eq (by rw [hx', hli, h]; rfl)
          rw [this] at h1; simp at h1
        step with R.face_shell f hf as ⟨fs, hfs⟩
        have hatS : b.atS f.val = min fs.val b.shells.size := by unfold Body.atS; rw [hfs]
        split
        · rename_i hlt2
          have hlt2' : fs.val < b.shells.size := by rw [← hns]; exact hlt2
          have hbl : fs.val < bd.val.length := by rw [hbd.1]; omega
          simp only [Std.bind_ok]
          step as ⟨p, hp1, hp2⟩
          obtain ⟨i0, back⟩ := p
          simp only at hp1 hp2 ⊢
          step as ⟨f1, hf1⟩
          refine ⟨by rw [hw f1 hf1]; omega, ?_, by rw [hw f1 hf1]; omega⟩
          rw [hw f1 hf1, hp2]
          apply badT_step b f.val 0 bd hbd (f.val + 1) 0 (fun s => b.listedIn f.val != 1 && b.atS f.val == s) fs (by omega)
          · intro s; simp only [h1', bne_iff_ne, ne_eq, not_false_eq_true, Bool.true_and, hatS,
              Nat.min_eq_left (Nat.le_of_lt hlt2')]
            rw [show (b.listedIn f.val != 1) = true from bne_iff_ne.2 h1', Bool.true_and, beq_symm]
          · intro s; rw [hstep]; simp [Bool.or_assoc, Bool.or_comm (badE b 0 s)]
        · rename_i hlt2
          have hlt2' : ¬ fs.val < b.shells.size := by rw [← hns]; exact hlt2
          have hbl : ns.val < bd.val.length := by rw [hbd.1, hns]; omega
          simp only [Std.bind_ok]
          step as ⟨p, hp1, hp2⟩
          obtain ⟨i0, back⟩ := p
          simp only at hp1 hp2 ⊢
          step as ⟨f1, hf1⟩
          refine ⟨by rw [hw f1 hf1]; omega, ?_, by rw [hw f1 hf1]; omega⟩
          rw [hw f1 hf1, hp2]
          apply badT_step b f.val 0 bd hbd (f.val + 1) 0 (fun s => b.listedIn f.val != 1 && b.atS f.val == s) ns (by omega)
          · intro s; simp only [h1', bne_iff_ne, ne_eq, not_false_eq_true, Bool.true_and, hatS,
              Nat.min_eq_right (Nat.le_of_not_lt hlt2'), hns]
            rw [show (b.listedIn f.val != 1) = true from bne_iff_ne.2 h1', Bool.true_and, beq_symm]
          · intro s; rw [hstep]; simp [Bool.or_assoc, Bool.or_comm (badE b 0 s)]
      · rename_i h1
        have h1' : b.listedIn f.val = 1 := by
          simp only [bne_iff_ne, ne_eq, Decidable.not_not] at h1
          have := congrArg UScalar.val h1
          rw [hx', hli] at this
          have h12 : (1#u32).val = 1 := rfl
          rw [h12] at this; omega
        step as ⟨f1, hf1⟩
        refine ⟨by rw [hw f1 hf1]; omega, ?_, by rw [hw f1 hf1]; omega⟩
        rw [hw f1 hf1]
        apply badT_same b f.val 0 bd hbd (f.val + 1) 0
        intro s; rw [hstep]; simp [h1']
    · rename_i hlt
      have hf : f.val = b.faces.size := by scalar_tac
      simp only [WP.spec_ok]
      rw [← hf]; exact hbd
  · exact ⟨by simp, hb⟩

theorem coedge_face_spec (TI : topo.Topo T) (R : Reads TI t b) (c : Usize) (hc : c.val < b.coedges.size)
    (hl : (b.co c.val).lp < b.loops.size) :
    topo_shell.coedge_face TI t c ⦃ g => g.val = b.faceOf c.val ⦄ := by
  unfold topo_shell.coedge_face
  step with R.coedge_loop c hc as ⟨l, hl'⟩
  have hl2 : l.val < b.loops.size := by rw [hl']; exact hl
  step with R.loop_face l hl2 as ⟨g, hg⟩
  rw [hg, hl']; rfl

theorem split_spec (R : ShellReads SI t b) (H : Sound b) (e n s0 : Usize) (he : e.val < b.edges.size)
    (hn : n.val = (b.ed e.val).uses.length) (hn0 : 0 < n.val)
    (hs0 : s0.val = b.faceShell (b.faceOf (b.ed e.val).uses[0]!)) :
    topo_shell.shell_faces_loop2_loop0 SI t e n s0 false 1#usize ⦃ r =>
      r = (b.ed e.val).uses.any (fun c => b.faceShell (b.faceOf c) != s0.val) ⦄ := by
  have hfirst : (b.faceShell (b.faceOf (b.ed e.val).uses[0]!) != s0.val) = false := by rw [hs0]; simp
  unfold topo_shell.shell_faces_loop2_loop0
  apply loop.spec_decr_nat (fun (y : Bool × Usize) => n.val - y.2.val)
    (fun y => 1 ≤ y.2.val ∧ y.2.val ≤ n.val ∧
      y.1 = ((b.ed e.val).uses.take y.2.val).any (fun c => b.faceShell (b.faceOf c) != s0.val))
  · rintro ⟨sp, i⟩ ⟨h1, hle, hsp⟩
    simp only at h1 hle hsp ⊢
    unfold topo_shell.shell_faces_loop2_loop0.body
    split
    · rename_i hsplit
      simp only [WP.spec_ok]
      rw [hsplit] at hsp
      rw [hsplit, ← List.take_append_drop i.val (b.ed e.val).uses, List.any_append, ← hsp]; rfl
    · rename_i hsplit
      split
      · rename_i hlt
        have hlt' : i.val < n.val := by scalar_tac
        have hi : i.val < (b.ed e.val).uses.length := by omega
        step with R.base.edge_use e i he hi as ⟨c, hc⟩
        have hmem : c.val ∈ (b.ed e.val).uses := by
          rw [hc, getElem!_pos _ i.val hi]; exact List.getElem_mem hi
        have ⟨hcn, hcl, hcf⟩ := H.uses e.val he c.val hmem
        step with coedge_face_spec t b SI.topoTopoInst R.base c hcn hcl as ⟨g, hg⟩
        have hgf : g.val < b.faces.size := by rw [hg]; exact hcf
        step with R.face_shell g hgf as ⟨gs, hgs⟩
        step as ⟨i1, hi1⟩
        have hw : i1.val = i.val + 1 := by rw [hi1]; exact succ_val i n hlt'
        refine ⟨by omega, by omega, ?_, by omega⟩
        rw [hw, List.take_add_one, List.any_append]
        have : (b.ed e.val).uses[i.val]? = some (b.ed e.val).uses[i.val]! := by
          rw [getElem!_pos _ i.val hi, List.getElem?_eq_getElem]
        rw [this, ← hsp]
        simp only [Bool.not_eq_true] at hsplit
        rw [hsplit, Bool.false_or, ← hc]
        simp only [Option.toList_some, List.any_cons, List.any_nil, Bool.or_false]
        rw [← hg, ← hgs]
        by_cases hq : gs = s0
        · subst hq; simp
        · have : gs.val ≠ s0.val := fun h => hq (UScalar.eq_of_val_eq h)
          have e1 : (gs != s0) = true := by simp [hq]
          have e2 : (gs.val != s0.val) = true := by simp [this]
          rw [e1, e2]
      · rename_i hlt
        have hi : i.val = (b.ed e.val).uses.length := by scalar_tac
        simp only [WP.spec_ok]
        rw [hsp, hi, List.take_length]
  · refine ⟨by simp, by simp; omega, ?_⟩
    have h0 : 0 < (b.ed e.val).uses.length := by omega
    rw [show (1#usize).val = 1 from rfl]
    cases hu : (b.ed e.val).uses with
    | nil => simp [hu] at h0
    | cons c0 rest =>
      rw [hu] at hfirst
      simp only [List.take_succ_cons, List.take_zero, List.any_cons, List.any_nil, Bool.or_false]
      simpa using hfirst

theorem splitAt_eq (e s : Nat) (h0 : 0 < (b.ed e).uses.length) :
    splitAt b e s = ((b.ed e).uses.any (fun c => b.faceShell (b.faceOf c) != b.faceShell (b.faceOf (b.ed e).uses[0]!)) &&
      b.atS (b.faceOf (b.ed e).uses[0]!) == s) := by
  unfold splitAt
  cases hu : (b.ed e).uses with
  | nil => simp [hu] at h0
  | cons c0 rest => simp

theorem edge_split_spec (R : ShellReads SI t b) (H : Sound b) (ns ne : Usize) (hns : ns.val = b.shells.size)
    (hne : ne.val = b.edges.size) (bad : alloc.vec.Vec Bool) (hb : BadT b b.faces.size 0 bad) :
    topo_shell.shell_faces_loop2 SI t ns ne bad 0#usize ⦃ r => BadT b b.faces.size b.edges.size r ⦄ := by
  unfold topo_shell.shell_faces_loop2
  apply loop.spec_decr_nat (fun (y : alloc.vec.Vec Bool × Usize) => ne.val - y.2.val)
    (fun y => y.2.val ≤ ne.val ∧ BadT b b.faces.size y.2.val y.1)
  · rintro ⟨bd, e⟩ ⟨hle, hbd⟩
    simp only at hle hbd ⊢
    unfold topo_shell.shell_faces_loop2.body
    split
    · rename_i hlt
      have hlt' : e.val < ne.val := by scalar_tac
      have he : e.val < b.edges.size := by omega
      have hw : ∀ e1 : Usize, e1 = e.wrapping_add 1#usize → e1.val = e.val + 1 := by
        intro e1 h; rw [h]; exact succ_val e ne hlt'
      have hstep : ∀ s, badE b (e.val + 1) s = (badE b e.val s || splitAt b e.val s) := by
        intro s; unfold badE; rw [any_range_succ]
      step with R.base.edge_alive e he as ⟨al, hal⟩
      rw [hal]; simp only [ite_true]
      step with R.base.edge_n_uses e he as ⟨n, hn⟩
      split
      · rename_i hn0
        have hn0' : 0 < n.val := by scalar_tac
        have h0 : 0 < (b.ed e.val).uses.length := by omega
        step with R.base.edge_use e 0#usize he (by simpa using h0) as ⟨c0, hc0⟩
        have hmem : c0.val ∈ (b.ed e.val).uses := by
          rw [hc0, getElem!_pos (b.ed e.val).uses 0 h0]
          exact List.getElem_mem h0
        have ⟨hcn, hcl, hcf⟩ := H.uses e.val he c0.val hmem
        step with coedge_face_spec t b SI.topoTopoInst R.base c0 hcn hcl as ⟨g0, hg0⟩
        have hgf : g0.val < b.faces.size := by rw [hg0]; exact hcf
        step with R.face_shell g0 hgf as ⟨s0, hs0⟩
        have hs0' : s0.val = b.faceShell (b.faceOf (b.ed e.val).uses[0]!) := by
          rw [hs0, hg0, hc0]
        step with split_spec SI t b R H e n s0 he hn hn0' hs0' as ⟨sp, hsp⟩
        have hsat : ∀ s, splitAt b e.val s = (sp && b.atS (b.faceOf (b.ed e.val).uses[0]!) == s) := by
          intro s; rw [splitAt_eq b e.val s h0, hsp, ← hs0']
        have hatS : b.atS (b.faceOf (b.ed e.val).uses[0]!) = min s0.val b.shells.size := by
          unfold Body.atS; rw [hs0']
        split
        · rename_i hsplit
          split
          · rename_i hlt2
            have hlt2' : s0.val < b.shells.size := by rw [← hns]; exact hlt2
            have hbl : s0.val < bd.val.length := by rw [hbd.1]; omega
            simp only [Std.bind_ok]
            step as ⟨p, hp1, hp2⟩
            obtain ⟨i0, back⟩ := p
            simp only at hp1 hp2 ⊢
            step as ⟨e1, he1⟩
            refine ⟨by rw [hw e1 he1]; omega, ?_, by rw [hw e1 he1]; omega⟩
            rw [hw e1 he1, hp2]
            apply badT_step b b.faces.size e.val bd hbd b.faces.size (e.val + 1) (splitAt b e.val) s0 (by omega)
            · intro s; rw [hsat, hsplit, Bool.true_and, hatS, Nat.min_eq_left (Nat.le_of_lt hlt2'), beq_symm]
            · intro s; rw [hstep, Bool.or_assoc]
          · rename_i hlt2
            have hlt2' : ¬ s0.val < b.shells.size := by rw [← hns]; exact hlt2
            have hbl : ns.val < bd.val.length := by rw [hbd.1, hns]; omega
            simp only [Std.bind_ok]
            step as ⟨p, hp1, hp2⟩
            obtain ⟨i0, back⟩ := p
            simp only at hp1 hp2 ⊢
            step as ⟨e1, he1⟩
            refine ⟨by rw [hw e1 he1]; omega, ?_, by rw [hw e1 he1]; omega⟩
            rw [hw e1 he1, hp2]
            apply badT_step b b.faces.size e.val bd hbd b.faces.size (e.val + 1) (splitAt b e.val) ns (by omega)
            · intro s; rw [hsat, hsplit, Bool.true_and, hatS, Nat.min_eq_right (Nat.le_of_not_lt hlt2'), hns, beq_symm]
            · intro s; rw [hstep, Bool.or_assoc]
        · rename_i hsplit
          simp only [Bool.not_eq_true] at hsplit
          step as ⟨e1, he1⟩
          refine ⟨by rw [hw e1 he1]; omega, ?_, by rw [hw e1 he1]; omega⟩
          rw [hw e1 he1]
          apply badT_same b b.faces.size e.val bd hbd b.faces.size (e.val + 1)
          intro s; rw [hstep, hsat, hsplit]; simp
      · rename_i hn0
        have h0 : (b.ed e.val).uses = [] := by
          rw [← List.length_eq_zero_iff, ← hn]; scalar_tac
        step as ⟨e1, he1⟩
        refine ⟨by rw [hw e1 he1]; omega, ?_, by rw [hw e1 he1]; omega⟩
        rw [hw e1 he1]
        apply badT_same b b.faces.size e.val bd hbd b.faces.size (e.val + 1)
        intro s; rw [hstep]; simp [splitAt, h0]
    · rename_i hlt
      have he : e.val = b.edges.size := by scalar_tac
      simp only [WP.spec_ok]
      rw [← he]; exact hbd
  · exact ⟨by simp, hb⟩

/-- A sink that appends each shell problem it is handed. -/
def AppendsS {S : Type} (SK : topo.Sink S) (P : S → List Problem) : Prop :=
  (∀ (o : S) (s : Usize), SK.shell_faces o s ⦃ o' => P o' = P o ++ [Problem.shellFaces s.val] ⦄) ∧
  (∀ (o : S) (s : Usize), SK.shell_disconnected o s ⦃ o' => P o' = P o ++ [Problem.shellDisconnected s.val] ⦄)

theorem emit_bad_spec {S : Type} (SK : topo.Sink S) (P : S → List Problem) (hS : AppendsS SK P)
    (ns : Usize) (hns : ns.val = b.shells.size) (hle : b.shells.size < U32.max)
    (bad : alloc.vec.Vec Bool) (hb : BadT b b.faces.size b.edges.size bad) (out : S) :
    topo_shell.shell_faces_loop3 SK out ns bad 0#usize ⦃ r =>
      P r = P out ++ ((List.range (b.shells.size + 1)).filterMap fun s =>
        if b.bad s then some (Problem.shellFaces s) else none) ⦄ := by
  unfold topo_shell.shell_faces_loop3
  apply loop.spec_decr_nat (fun (y : S × Usize) => ns.val + 1 - y.2.val)
    (fun y => y.2.val ≤ ns.val + 1 ∧ P y.1 = P out ++ ((List.range y.2.val).filterMap fun s =>
        if b.bad s then some (Problem.shellFaces s) else none))
  · rintro ⟨o, s⟩ ⟨hle', ho⟩
    simp only at hle' ho ⊢
    unfold topo_shell.shell_faces_loop3.body
    split
    · rename_i hlt
      have hlt' : s.val ≤ ns.val := by scalar_tac
      have hsb : s.val < bad.val.length := by rw [hb.1]; omega
      have hbs : bad.val[s.val]! = b.bad s.val := by rw [hb.2 s.val (by omega), bad_eq]
      have hw : ∀ s1 : Usize, s1 = s.wrapping_add 1#usize → s1.val = s.val + 1 := by
        intro s1 h; rw [h, Usize.wrapping_add_val_eq]; apply Nat.mod_eq_of_lt
        have := succ_lt_usize s.val (by omega)
        simp only [UScalar.size]; exact this
      step as ⟨x, hx⟩
      have hx' : x = b.bad s.val := by rw [hx, ← hbs, getElem!_pos]
      split
      · rename_i hxt
        step with hS.1 o s as ⟨o1, ho1⟩
        step as ⟨s1, hs1⟩
        refine ⟨by rw [hw s1 hs1]; omega, ?_, by rw [hw s1 hs1]; omega⟩
        rw [hw s1 hs1, ho1, filterMap_range_succ, ho, List.append_assoc]
        rw [← hx', hxt]; simp
      · rename_i hxt
        step as ⟨s1, hs1⟩
        refine ⟨by rw [hw s1 hs1]; omega, ?_, by rw [hw s1 hs1]; omega⟩
        rw [hw s1 hs1, filterMap_range_succ, ho]
        rw [← hx']; simp at hxt; simp [hxt]
    · rename_i hlt
      have : s.val = ns.val + 1 := by scalar_tac
      simp only [WP.spec_ok]
      rw [ho, this, hns]
  · exact ⟨by simp, by simp⟩

theorem fill_spec (nf : Usize) (seen : alloc.vec.Vec U32) (hl : seen.val.length = nf.val) :
    topo_shell.shell_faces_loop4 nf seen 0#usize ⦃ r =>
      r.val.length = nf.val ∧ ∀ f < nf.val, r.val[f]! = core.num.U32.MAX ⦄ := by
  unfold topo_shell.shell_faces_loop4
  apply loop.spec_decr_nat (fun (y : alloc.vec.Vec U32 × Usize) => nf.val - y.2.val)
    (fun y => y.2.val ≤ nf.val ∧ y.1.val.length = nf.val ∧ ∀ f < y.2.val, y.1.val[f]! = core.num.U32.MAX)
  · rintro ⟨u, k⟩ ⟨hle, hlen, hu⟩
    simp only at hle hlen hu ⊢
    unfold topo_shell.shell_faces_loop4.body
    split
    · rename_i hlt
      have hlt' : k.val < nf.val := by scalar_tac
      have hkb : k.val < u.val.length := by omega
      step as ⟨i0, back, hp1, hp2⟩
      step as ⟨k1, hk1⟩
      have hw : k1.val = k.val + 1 := by rw [hk1]; exact succ_val k nf hlt'
      refine ⟨by omega, ?_, ?_, by omega⟩
      · rw [hp2]; simp [alloc.vec.Vec.set_val_eq, hlen]
      · intro f hf
        rw [hp2]; simp only [alloc.vec.Vec.set_val_eq]
        rw [getElem!_set _ _ _ _ (by omega)]
        by_cases hfk : f = k.val
        · simp [hfk]
        · rw [if_neg hfk]; exact hu f (by omega)
    · rename_i hlt
      have : k.val = nf.val := by scalar_tac
      simp only [WP.spec_ok]
      exact ⟨hlen, fun f hf => hu f (by omega)⟩
  · exact ⟨by simp, hl, by simp⟩

/-! ## The flood: a depth-first search that stamps what it reaches -/

theorem u32max_eq : U32.max = 2 ^ 32 - 1 := by
  rw [U32.max_eq]; rfl

theorem cast32_val (x : Usize) (h : x.val ≤ U32.max) : (UScalar.cast .U32 x).val = x.val := by
  rw [UScalar.cast_val_eq]; apply Nat.mod_eq_of_lt
  have := u32max_eq
  simp only [UScalarTy.numBits]; omega

theorem getElem!_append_left' {α} [Inhabited α] (l m : List α) (j : Nat) (h : j < l.length) :
    (l ++ m)[j]! = l[j]! := by
  rw [getElem!_pos (l ++ m) j (by simp; omega), getElem!_pos l j h, List.getElem_append_left h]

theorem getElem!_append_len {α} [Inhabited α] (l : List α) (x : α) : (l ++ [x])[l.length]! = x := by
  rw [getElem!_pos (l ++ [x]) l.length (by simp)]; simp

theorem castUsize_val (x : U32) : (UScalar.cast .Usize x).val = x.val := by
  rw [UScalar.cast_val_eq]; apply Nat.mod_eq_of_lt
  have h := x.hBounds
  have h32 : 32 ≤ UScalarTy.Usize.numBits := by
    rw [UScalarTy.Usize_numBits_eq]; rcases System.Platform.numBits_eq with h | h <;> rw [h] <;> omega
  have : 2 ^ 32 ≤ 2 ^ UScalarTy.Usize.numBits := Nat.pow_le_pow_right (by norm_num) h32
  simp only [UScalarTy.numBits] at h; omega

theorem max_val : (core.num.U32.MAX).val = U32.max := by
  rw [U32.max_eq]; rfl

/-- The faces not stamped `σ`. -/
def unm (b : Body) (σ : U32) (seen : alloc.vec.Vec U32) : Nat :=
  (List.range b.faces.size).countP fun g => seen.val[g]! != σ

theorem unm_mark (b : Body) (σ : U32) (seen : alloc.vec.Vec U32) (g : Usize) (hg : g.val < b.faces.size)
    (hl : seen.val.length = b.faces.size) (hn : seen.val[g.val]! ≠ σ) :
    unm b σ (seen.set g σ) + 1 = unm b σ seen := by
  unfold unm
  rw [TopoCheck.Body.countP_drop_one (p := fun g => seen.val[g]! != σ) _ List.nodup_range g.val
    (List.mem_range.2 hg) (by simp only [bne_iff_ne, ne_eq]; exact hn)]
  congr 1
  apply List.countP_congr
  intro x hx
  have hx' := List.mem_range.1 hx
  simp only [alloc.vec.Vec.set_val_eq]
  rw [getElem!_set _ _ _ _ (by omega)]
  by_cases h : x = g.val <;> simp [h]

/-- The search for shell `s` from `f0`, stamping `σ`, with `cur` the face being expanded. -/
structure DFS (b : Body) (f0 : Nat) (σ : U32) (cur : Option Nat)
    (seen stack : alloc.vec.Vec U32) (sp : Nat) : Prop where
  len : seen.val.length = b.faces.size
  reach : ∀ g < b.faces.size, seen.val[g]! = σ → b.Reach f0 g
  sp_le : sp ≤ stack.val.length
  stack_le : stack.val.length ≤ b.faces.size
  on : ∀ j < sp, (stack.val[j]!).val < b.faces.size ∧ seen.val[(stack.val[j]!).val]! = σ
  done : ∀ g < b.faces.size, seen.val[g]! = σ →
    (∃ j < sp, (stack.val[j]!).val = g) ∨ cur = some g ∨ ∀ y ∈ b.nbrs g, seen.val[y]! = σ
  count : sp + unm b σ seen ≤ b.faces.size
  cur_ok : ∀ f, cur = some f → f < b.faces.size ∧ seen.val[f]! = σ ∧ b.Reach f0 f

/-- Stamping `g`, a neighbour of the face being expanded, and pushing it. -/
theorem dfs_push (b : Body) (f0 : Nat) (σ : U32) (f : Nat) (seen stack : alloc.vec.Vec U32) (sp : Usize)
    (D : DFS b f0 σ (some f) seen stack sp.val) (g : Usize) (hg : g.val < b.faces.size)
    (hgn : g.val ∈ b.nbrs f) (hun : seen.val[g.val]! ≠ σ) (stack' : alloc.vec.Vec U32)
    (hst : stack'.val = if sp.val < stack.val.length then stack.val.set sp.val (UScalar.cast .U32 g)
      else stack.val ++ [UScalar.cast .U32 g]) (hgle : g.val ≤ U32.max)
    (hN : ∀ f, ∀ y ∈ b.nbrs f, y < b.faces.size) :
    DFS b f0 σ (some f) (seen.set g σ) stack' (sp.val + 1) := by
  have hcnt := unm_mark b σ seen g hg D.len hun
  have hspn : sp.val + 1 ≤ b.faces.size := by have := D.count; omega
  have hcg : (UScalar.cast .U32 g).val = g.val := cast32_val g hgle
  have hsl : stack'.val.length = max stack.val.length (sp.val + 1) := by
    rw [hst]; split
    · simp; omega
    · simp; have := D.sp_le; omega
  have hsj : ∀ j < sp.val, stack'.val[j]! = stack.val[j]! := by
    intro j hj; rw [hst]; split
    · rw [getElem!_set _ _ _ _ (by omega), if_neg (by omega)]
    · rw [getElem!_append_left' _ _ _ (by have := D.sp_le; omega)]
  have hstop : stack'.val[sp.val]! = UScalar.cast .U32 g := by
    rw [hst]; split
    · rename_i h; rw [getElem!_set _ _ _ _ h, if_pos rfl]
    · rename_i h
      have : sp.val = stack.val.length := by have := D.sp_le; omega
      rw [this, getElem!_append_len]
  have hseen : ∀ y < b.faces.size, (seen.set g σ).val[y]! = if y = g.val then σ else seen.val[y]! := by
    intro y hy; simp only [alloc.vec.Vec.set_val_eq]; rw [getElem!_set _ _ _ _ (by rw [D.len]; exact hy)]
  have hmono : ∀ y < b.faces.size, seen.val[y]! = σ → (seen.set g σ).val[y]! = σ := by
    intro y hy h; rw [hseen y hy]; split <;> simp_all
  have hfr := D.cur_ok f rfl
  refine ⟨by simp [alloc.vec.Vec.set_val_eq, D.len], ?_, ?_, ?_, ?_, ?_, ?_, ?_⟩
  · intro y hy h
    rw [hseen y hy] at h
    split at h
    · rename_i hyg; subst hyg; exact TopoCheck.Body.Reach.step hfr.2.2 hgn
    · exact D.reach y hy h
  · rw [hsl]; omega
  · rw [hsl]; have := D.stack_le; omega
  · intro j hj
    by_cases hjs : j < sp.val
    · rw [hsj j hjs]; have := D.on j hjs; exact ⟨this.1, hmono _ this.1 this.2⟩
    · have : j = sp.val := by omega
      subst this; rw [hstop, hcg]; exact ⟨hg, by rw [hseen _ hg, if_pos rfl]⟩
  · intro y hy h
    rw [hseen y hy] at h
    split at h
    · rename_i hyg; left; exact ⟨sp.val, by omega, by rw [hstop, hcg, hyg]⟩
    · rcases D.done y hy h with ⟨j, hj, hjy⟩ | hc | hall
      · left; exact ⟨j, by omega, by rw [hsj j hj]; exact hjy⟩
      · right; left; exact hc
      · right; right; intro z hz
        exact hmono z (hN y z hz) (hall z hz)
  · have := D.count; omega
  · intro f' hf'
    have := D.cur_ok f' hf'
    exact ⟨this.1, hmono _ this.1 this.2.1, this.2.2⟩

/-- Stamps only grow, and only `σ` is written. -/
def Mono (b : Body) (σ : U32) (seen seen' : alloc.vec.Vec U32) : Prop :=
  (∀ y < b.faces.size, seen.val[y]! = σ → seen'.val[y]! = σ) ∧
  (∀ y < b.faces.size, seen'.val[y]! = seen.val[y]! ∨ seen'.val[y]! = σ)

theorem mono_trans (b : Body) (σ : U32) (a c d : alloc.vec.Vec U32) (h1 : Mono b σ a c) (h2 : Mono b σ c d) :
    Mono b σ a d := by
  refine ⟨fun y hy h => h2.1 y hy (h1.1 y hy h), fun y hy => ?_⟩
  rcases h2.2 y hy with h | h
  · rw [h]; exact h1.2 y hy
  · right; exact h

theorem mono_refl (b : Body) (σ : U32) (a : alloc.vec.Vec U32) : Mono b σ a a :=
  ⟨fun _ _ h => h, fun _ _ => Or.inl rfl⟩

theorem mono_set (b : Body) (σ : U32) (a : alloc.vec.Vec U32) (g : Usize) (hl : a.val.length = b.faces.size) :
    Mono b σ a (a.set g σ) := by
  refine ⟨fun y hy h => ?_, fun y hy => ?_⟩
  · simp only [alloc.vec.Vec.set_val_eq]
    rw [getElem!_set _ _ _ _ (by rw [hl]; exact hy)]
    split <;> simp_all
  · simp only [alloc.vec.Vec.set_val_eq]
    rw [getElem!_set _ _ _ _ (by rw [hl]; exact hy)]
    split
    · right; rfl
    · left; rfl

theorem uses_mark_spec (R : ShellReads SI t b) (H : Sound b) (f0 : Nat) (s : Usize) (σ : U32)
    (hσ : σ = UScalar.cast .U32 s) (f : Nat) (e nu : Usize) (he : e.val < b.edges.size)
    (hnu : nu.val = (b.ed e.val).uses.length) (hnb : ∀ u ∈ (b.ed e.val).uses, b.faceOf u ∈ b.nbrs f)
    (seen stack : alloc.vec.Vec U32) (sp : Usize) (D : DFS b f0 σ (some f) seen stack sp.val) :
    topo_shell.shell_faces_loop5_loop0_loop0_loop0_loop0 SI t seen stack s sp e nu 0#usize ⦃ r =>
      DFS b f0 σ (some f) r.1 r.2.1 r.2.2.val ∧ Mono b σ seen r.1 ∧
        (∀ u ∈ (b.ed e.val).uses, r.1.val[b.faceOf u]! = σ) ∧
        r.2.2.val + unm b σ r.1 = sp.val + unm b σ seen ⦄ := by
  have hN := nbrs_lt b H
  have hnf32 := R.nf_le
  unfold topo_shell.shell_faces_loop5_loop0_loop0_loop0_loop0
  apply loop.spec_decr_nat
    (fun (y : alloc.vec.Vec U32 × alloc.vec.Vec U32 × Usize × Usize) => nu.val - y.2.2.2.val)
    (fun y => y.2.2.2.val ≤ nu.val ∧ DFS b f0 σ (some f) y.1 y.2.1 y.2.2.1.val ∧ Mono b σ seen y.1 ∧
      (∀ j < y.2.2.2.val, y.1.val[b.faceOf (b.ed e.val).uses[j]!]! = σ) ∧
      y.2.2.1.val + unm b σ y.1 = sp.val + unm b σ seen)
  · rintro ⟨sn, st, sq, j⟩ ⟨hle, Dj, hm, hdone, hacc⟩
    simp only at hle Dj hm hdone hacc ⊢
    unfold topo_shell.shell_faces_loop5_loop0_loop0_loop0_loop0.body
    split
    · rename_i hlt
      have hlt' : j.val < nu.val := by scalar_tac
      have hj : j.val < (b.ed e.val).uses.length := by omega
      have hw : ∀ j1 : Usize, j1 = j.wrapping_add 1#usize → j1.val = j.val + 1 := by
        intro j1 h; rw [h]; exact succ_val j nu hlt'
      step with R.base.edge_use e j he hj as ⟨u, hu⟩
      have hmem : u.val ∈ (b.ed e.val).uses := by
        rw [hu, getElem!_pos (b.ed e.val).uses j.val hj]; exact List.getElem_mem hj
      have ⟨hun, hul, huf⟩ := H.uses e.val he u.val hmem
      step with coedge_face_spec t b SI.topoTopoInst R.base u hun hul as ⟨g, hg⟩
      have hgf : g.val < b.faces.size := by rw [hg]; exact huf
      have hgn : g.val ∈ b.nbrs f := by rw [hg]; exact hnb u.val hmem
      have hgb : g.val < sn.val.length := by rw [Dj.len]; exact hgf
      step as ⟨x, hx⟩
      have hx' : x = sn.val[g.val]! := by rw [hx, getElem!_pos]
      simp only [lift, Std.bind_ok, ← hσ]
      have hfaceu : b.faceOf (b.ed e.val).uses[j.val]! = g.val := by rw [← hu, hg]
      split
      · rename_i hneq
        have hun' : sn.val[g.val]! ≠ σ := by
          intro h; rw [← hx'] at h; rw [h] at hneq; simp at hneq
        step as ⟨i0, hb1, hb2⟩
        obtain ⟨i0a, back⟩ := i0
        simp only at hb1 hb2 ⊢
        have hsq : sq.val + 1 ≤ b.faces.size := by
          have := unm_mark b σ sn g hgf Dj.len hun'; have := Dj.count; omega
        have hw2 : (sq.wrapping_add 1#usize).val = sq.val + 1 := by
          rw [Usize.wrapping_add_val_eq]; apply Nat.mod_eq_of_lt
          have := succ_lt_usize sq.val (by omega); simp only [UScalar.size]; exact this
        have hgle : g.val ≤ U32.max := by omega
        have fin : ∀ st' : alloc.vec.Vec U32, st'.val = (if sq.val < st.val.length then
            st.val.set sq.val (UScalar.cast .U32 g) else st.val ++ [UScalar.cast .U32 g]) →
            (j.wrapping_add 1#usize).val ≤ nu.val ∧
            DFS b f0 σ (some f) (sn.set g σ) st' (sq.wrapping_add 1#usize).val ∧ Mono b σ seen (sn.set g σ) ∧
            (∀ j' < (j.wrapping_add 1#usize).val, (sn.set g σ).val[b.faceOf (b.ed e.val).uses[j']!]! = σ) ∧
            (sq.wrapping_add 1#usize).val + unm b σ (sn.set g σ) = sp.val + unm b σ seen ∧
            nu.val - (j.wrapping_add 1#usize).val < nu.val - j.val := by
          intro st' hst'
          have hw' := hw _ rfl
          have D' := dfs_push b f0 σ f sn st sq Dj g hgf hgn hun' st' hst' hgle hN
          rw [hw2]
          have hum := unm_mark b σ sn g hgf Dj.len hun'
          refine ⟨by omega, D', mono_trans b σ _ _ _ hm (mono_set b σ sn g Dj.len), ?_, by omega, by omega⟩
          intro j' hj'
          rw [hw'] at hj'
          by_cases hjj : j' = j.val
          · subst hjj; rw [hfaceu]; simp only [alloc.vec.Vec.set_val_eq]
            rw [getElem!_set _ _ _ _ hgb, if_pos rfl]
          · have hj'' : j' < (b.ed e.val).uses.length := by omega
            have hm' : (b.ed e.val).uses[j']! ∈ (b.ed e.val).uses := by
              rw [getElem!_pos (b.ed e.val).uses j' hj'']; exact List.getElem_mem hj''
            exact (mono_set b σ sn g Dj.len).1 _ (H.uses e.val he _ hm').2.2 (hdone j' (by omega))
        split
        · rename_i hlt2
          have hlt2' : sq.val < st.val.length := by simpa using hlt2
          step as ⟨i1, hc1, hc2⟩
          obtain ⟨i1a, back1⟩ := i1
          simp only at hc1 hc2
          step*
          rw [hb2, hc2]
          have := fin (st.set sq (UScalar.cast .U32 g)) (by simp [alloc.vec.Vec.set_val_eq, hlt2'])
          exact this
        · rename_i hlt2
          have hlt2' : ¬ sq.val < st.val.length := by simpa using hlt2
          have hpl : st.val.length < Usize.max := by
            have := Dj.sp_le; have := Dj.stack_le
            have : U32.max ≤ Usize.max := by
              have := Usize.max_def; have := succ_lt_usize 0 (by rw [U32.max_eq]; omega)
              rw [U32.max_eq]
              rcases Usize.bounds_eq with h | h <;> rw [h] <;> simp [U32.max_eq, U64.max_eq]
            omega
          step with alloc.vec.Vec.push_spec st (UScalar.cast .U32 g) hpl as ⟨st3, hst3⟩
          step*
          rw [hb2]
          exact fin st3 (by rw [hst3, if_neg hlt2'])
      · rename_i heq
        have hmk : sn.val[g.val]! = σ := by
          simp only [bne_iff_ne, ne_eq, Decidable.not_not] at heq; rw [← hx', heq]
        step*
        have hw' := hw _ rfl
        refine ⟨by rw [hw']; omega, Dj, hm, ?_, hacc, by rw [hw']; omega⟩
        intro j' hj'
        rw [hw'] at hj'
        by_cases hjj : j' = j.val
        · subst hjj; rw [hfaceu]; exact hmk
        · exact hdone j' (by omega)
    · rename_i hlt
      have hj : j.val = (b.ed e.val).uses.length := by scalar_tac
      simp only [WP.spec_ok]
      refine ⟨Dj, hm, fun u hu => ?_, hacc⟩
      obtain ⟨k, hk, rfl⟩ := List.getElem_of_mem hu
      have := hdone k (by omega)
      rwa [getElem!_pos (b.ed e.val).uses k hk] at this
  · exact ⟨by simp, D, mono_refl b σ seen, by simp, rfl⟩

theorem mem_loopCoedges (l c : Nat) :
    c ∈ b.loopCoedges l ↔ (b.lo l).first ≤ c ∧ c < (b.lo l).first + (b.lo l).len := by
  unfold Body.loopCoedges; rw [List.mem_map]; simp only [List.mem_range]
  constructor
  · rintro ⟨k, hk, rfl⟩; omega
  · rintro ⟨h1, h2⟩; exact ⟨c - (b.lo l).first, by omega, by omega⟩

theorem coedges_mark_spec (R : ShellReads SI t b) (H : Sound b) (f0 : Nat) (s : Usize) (σ : U32)
    (hσ : σ = UScalar.cast .U32 s) (f : Nat) (hf : f < b.faces.size) (l : Nat) (hl : l ∈ b.faceLoops f)
    (c0 en : Usize) (hc0 : c0.val = (b.lo l).first) (hen : en.val = (b.lo l).first + (b.lo l).len)
    (seen stack : alloc.vec.Vec U32) (sp : Usize) (D : DFS b f0 σ (some f) seen stack sp.val) :
    topo_shell.shell_faces_loop5_loop0_loop0_loop0 SI t seen stack s sp en c0 ⦃ r =>
      DFS b f0 σ (some f) r.1 r.2.1 r.2.2.val ∧ Mono b σ seen r.1 ∧
        (∀ c ∈ b.loopCoedges l, ∀ u ∈ (b.ed (b.co c).edge).uses, r.1.val[b.faceOf u]! = σ) ∧
        r.2.2.val + unm b σ r.1 = sp.val + unm b σ seen ⦄ := by
  have ⟨hln, hlf, hl1⟩ := H.loops f hf l hl
  unfold topo_shell.shell_faces_loop5_loop0_loop0_loop0
  apply loop.spec_decr_nat
    (fun (y : alloc.vec.Vec U32 × alloc.vec.Vec U32 × Usize × Usize) => en.val - y.2.2.2.val)
    (fun y => c0.val ≤ y.2.2.2.val ∧ y.2.2.2.val ≤ en.val ∧ DFS b f0 σ (some f) y.1 y.2.1 y.2.2.1.val ∧
      Mono b σ seen y.1 ∧ (∀ c, c0.val ≤ c → c < y.2.2.2.val → ∀ u ∈ (b.ed (b.co c).edge).uses, y.1.val[b.faceOf u]! = σ) ∧
      y.2.2.1.val + unm b σ y.1 = sp.val + unm b σ seen)
  · rintro ⟨sn, st, sq, c⟩ ⟨h0, hle, Dc, hm, hdone, hacc⟩
    simp only at h0 hle Dc hm hdone hacc ⊢
    unfold topo_shell.shell_faces_loop5_loop0_loop0_loop0.body
    split
    · rename_i hlt
      have hlt' : c.val < en.val := by scalar_tac
      have hcm : c.val ∈ b.loopCoedges l := by rw [mem_loopCoedges]; omega
      have ⟨hcn, hce, hcu⟩ := H.coedges l hln hl1 c.val hcm
      have hw : ∀ c1 : Usize, c1 = c.wrapping_add 1#usize → c1.val = c.val + 1 := by
        intro c1 h; rw [h, Usize.wrapping_add_val_eq]; apply Nat.mod_eq_of_lt
        obtain ⟨n, hn⟩ := WP.spec_exists R.base.nc
        have hb := n.hBounds
        have h1 : (1#usize).val = 1 := rfl
        simp only [UScalar.size]; omega
      step with R.base.coedge_edge c hcn as ⟨e, he⟩
      have he' : e.val < b.edges.size := by rw [he]; exact hce
      step with R.base.edge_n_uses e he' as ⟨nu, hnu⟩
      have hnb : ∀ u ∈ (b.ed e.val).uses, b.faceOf u ∈ b.nbrs f := by
        intro u hu
        unfold Body.nbrs
        simp only [List.mem_flatMap, List.mem_map]
        exact ⟨l, hl, c.val, hcm, u, by rw [← he]; exact hu, rfl⟩
      step with uses_mark_spec SI t b R H f0 s σ hσ f e nu he' hnu hnb sn st sq Dc as ⟨sn2, st2, sq2, D2, hm2, hall, hacc2⟩
      step as ⟨c1, hc1⟩
      have hw1 := hw c1 hc1
      refine ⟨by rw [hw1]; omega, by rw [hw1]; omega, D2, mono_trans b σ _ _ _ hm hm2, ?_, by omega, by rw [hw1]; omega⟩
      intro c' hc'0 hc'1 u hu
      rw [hw1] at hc'1
      by_cases hcc : c' = c.val
      · subst hcc; rw [he] at hall; exact hall u hu
      · have hc'm : c' ∈ b.loopCoedges l := by rw [mem_loopCoedges]; omega
        have ⟨_, hce', hcu'⟩ := H.coedges l hln hl1 c' hc'm
        exact hm2.1 _ (H.uses _ hce' u hu).2.2 (hdone c' hc'0 (by omega) u hu)
    · rename_i hlt
      have hc : c.val = en.val := by scalar_tac
      simp only [WP.spec_ok]
      refine ⟨Dc, hm, fun c' hc' u hu => ?_, hacc⟩
      rw [mem_loopCoedges] at hc'
      exact hdone c' (by omega) (by omega) u hu
  · exact ⟨le_refl _, by rw [hc0, hen]; omega, D, mono_refl b σ seen, fun c h1 h2 => absurd h1 (by simp at h2 ⊢; omega), rfl⟩

theorem loops_mark_spec (R : ShellReads SI t b) (H : Sound b) (f0 : Nat) (s : Usize) (σ : U32)
    (hσ : σ = UScalar.cast .U32 s) (f : Usize) (hf : f.val < b.faces.size) (nlf : Usize)
    (hnlf : nlf.val = (b.faceLoops f.val).length)
    (seen stack : alloc.vec.Vec U32) (sp : Usize) (D : DFS b f0 σ (some f.val) seen stack sp.val) :
    topo_shell.shell_faces_loop5_loop0_loop0 SI t seen stack s sp f nlf 0#usize ⦃ r =>
      DFS b f0 σ (some f.val) r.1 r.2.1 r.2.2.val ∧ Mono b σ seen r.1 ∧
        (∀ y ∈ b.nbrs f.val, r.1.val[y]! = σ) ∧ r.2.2.val + unm b σ r.1 = sp.val + unm b σ seen ⦄ := by
  have hN := nbrs_lt b H
  unfold topo_shell.shell_faces_loop5_loop0_loop0
  apply loop.spec_decr_nat
    (fun (y : alloc.vec.Vec U32 × alloc.vec.Vec U32 × Usize × Usize) => nlf.val - y.2.2.2.val)
    (fun y => y.2.2.2.val ≤ nlf.val ∧ DFS b f0 σ (some f.val) y.1 y.2.1 y.2.2.1.val ∧ Mono b σ seen y.1 ∧
      (∀ i < y.2.2.2.val, ∀ c ∈ b.loopCoedges (b.faceLoops f.val)[i]!, ∀ u ∈ (b.ed (b.co c).edge).uses,
        y.1.val[b.faceOf u]! = σ) ∧ y.2.2.1.val + unm b σ y.1 = sp.val + unm b σ seen)
  · rintro ⟨sn, st, sq, i⟩ ⟨hle, Di, hm, hdone, hacc⟩
    simp only at hle Di hm hdone hacc ⊢
    unfold topo_shell.shell_faces_loop5_loop0_loop0.body
    split
    · rename_i hlt
      have hlt' : i.val < nlf.val := by scalar_tac
      have hi : i.val < (b.faceLoops f.val).length := by omega
      step with R.base.face_loop f i hf hi as ⟨l, hl⟩
      have hlm : l.val ∈ b.faceLoops f.val := by
        rw [hl, getElem!_pos (b.faceLoops f.val) i.val hi]; exact List.getElem_mem hi
      have ⟨hln, hlf, hl1⟩ := H.loops f.val hf l.val hlm
      step with R.base.loop_first l hln as ⟨fs, hfs⟩
      step with R.base.loop_len l hln as ⟨len, hlen⟩
      have hend : (fs.wrapping_add len).val = (b.lo l.val).first + (b.lo l.val).len := by
        rw [Usize.wrapping_add_val_eq, hfs, hlen]; apply Nat.mod_eq_of_lt
        by_cases h0 : (b.lo l.val).len = 0
        · have := fs.hBounds; rw [h0, Nat.add_zero, ← hfs]; simp only [UScalar.size]; exact this
        · have hm' : (b.lo l.val).first + ((b.lo l.val).len - 1) ∈ b.loopCoedges l.val := by
            rw [mem_loopCoedges]; omega
          have := (H.coedges l.val hln hl1 _ hm').1
          obtain ⟨n, hn⟩ := WP.spec_exists R.base.nc
          have hb := n.hBounds
          simp only [UScalar.size]; omega
      simp only [lift, Std.bind_ok]
      have hcm := coedges_mark_spec SI t b R H f0 s σ hσ f.val hf l.val hlm fs (fs.wrapping_add len) hfs hend
        sn st sq Di
      step with hcm as ⟨sn2, st2, sq2, D2, hm2, hall, hacc2⟩
      have hw : (i.wrapping_add 1#usize).val = i.val + 1 := succ_val i nlf hlt'
      refine ⟨by omega, D2, mono_trans b σ _ _ _ hm hm2, ?_, by omega, by omega⟩
      intro i' hi' c hc u hu
      rw [hw] at hi'
      by_cases hii : i' = i.val
      · subst hii; rw [← hl] at hc; exact hall c hc u hu
      · have hi'' : i' < (b.faceLoops f.val).length := by omega
        have hlm' : (b.faceLoops f.val)[i']! ∈ b.faceLoops f.val := by
          rw [getElem!_pos (b.faceLoops f.val) i' hi'']; exact List.getElem_mem hi''
        have ⟨hln', _, hl1'⟩ := H.loops f.val hf _ hlm'
        have ⟨_, hce, _⟩ := H.coedges _ hln' hl1' c hc
        exact hm2.1 _ (H.uses _ hce u hu).2.2 (hdone i' (by omega) c hc u hu)
    · rename_i hlt
      have hi : i.val = (b.faceLoops f.val).length := by scalar_tac
      simp only [WP.spec_ok]
      refine ⟨Di, hm, fun y hy => ?_, hacc⟩
      unfold Body.nbrs at hy
      simp only [List.mem_flatMap, List.mem_map] at hy
      obtain ⟨l, hl, c, hc, u, hu, rfl⟩ := hy
      obtain ⟨k, hk, rfl⟩ := List.getElem_of_mem hl
      have := hdone k (by omega) c (by rw [getElem!_pos _ k hk]; exact hc) u hu
      exact this
  · exact ⟨by simp, D, mono_refl b σ seen, fun i h => absurd h (by simp), rfl⟩

theorem unm_mono (b : Body) (σ : U32) (a c : alloc.vec.Vec U32) (h : Mono b σ a c) : unm b σ c ≤ unm b σ a := by
  unfold unm
  apply List.countP_mono_left
  intro g hg h1
  simp only [bne_iff_ne, ne_eq] at h1 ⊢
  intro h2; exact h1 (h.1 g (List.mem_range.1 hg) h2)

theorem dfs_pop (b : Body) (f0 : Nat) (σ : U32) (seen stack : alloc.vec.Vec U32) (sp : Nat)
    (D : DFS b f0 σ none seen stack (sp + 1)) :
    DFS b f0 σ (some (stack.val[sp]!).val) seen stack sp := by
  have hon := D.on sp (by omega)
  refine ⟨D.len, D.reach, by have := D.sp_le; omega, D.stack_le, fun j hj => D.on j (by omega), ?_, by have := D.count; omega, ?_⟩
  · intro g hg h
    rcases D.done g hg h with ⟨j, hj, hjg⟩ | hc | hall
    · by_cases hjs : j = sp
      · subst hjs; right; left; rw [hjg]
      · left; exact ⟨j, by omega, hjg⟩
    · simp at hc
    · right; right; exact hall
  · intro f hf
    simp only [Option.some.injEq] at hf
    subst hf
    exact ⟨hon.1, hon.2, D.reach _ hon.1 hon.2⟩

theorem dfs_close (b : Body) (f0 : Nat) (σ : U32) (f : Nat) (seen stack : alloc.vec.Vec U32) (sp : Nat)
    (D : DFS b f0 σ (some f) seen stack sp) (hall : ∀ y ∈ b.nbrs f, seen.val[y]! = σ) :
    DFS b f0 σ none seen stack sp := by
  refine ⟨D.len, D.reach, D.sp_le, D.stack_le, D.on, ?_, D.count, by simp⟩
  intro g hg h
  rcases D.done g hg h with hj | hc | hn
  · left; exact hj
  · simp only [Option.some.injEq] at hc; subst hc; right; right; exact hall
  · right; right; exact hn

theorem pop_loop_spec (R : ShellReads SI t b) (H : Sound b) (f0 : Nat) (s : Usize) (σ : U32)
    (hσ : σ = UScalar.cast .U32 s) (seen stack : alloc.vec.Vec U32) (sp : Usize)
    (D : DFS b f0 σ none seen stack sp.val) :
    topo_shell.shell_faces_loop5_loop0 SI t seen stack s sp ⦃ r =>
      DFS b f0 σ none r.1 r.2 0 ∧ Mono b σ seen r.1 ⦄ := by
  unfold topo_shell.shell_faces_loop5_loop0
  apply loop.spec_decr_nat
    (fun (y : alloc.vec.Vec U32 × alloc.vec.Vec U32 × Usize) =>
      unm b σ y.1 * (b.faces.size + 1) + y.2.2.val)
    (fun y => DFS b f0 σ none y.1 y.2.1 y.2.2.val ∧ Mono b σ seen y.1)
  · rintro ⟨sn, st, sq⟩ ⟨Dq, hm⟩
    simp only at Dq hm ⊢
    unfold topo_shell.shell_faces_loop5_loop0.body
    split
    · rename_i hpos
      have hpos' : 0 < sq.val := by scalar_tac
      have hsub : (sq.wrapping_sub 1#usize).val = sq.val - 1 := sub_val sq 1#usize (by
        have : (1#usize).val = 1 := rfl; omega)
      simp only [lift, Std.bind_ok]
      have Dq' : DFS b f0 σ none sn st ((sq.val - 1) + 1) := by
        rw [Nat.sub_add_cancel hpos']; exact Dq
      have Dp := dfs_pop b f0 σ sn st (sq.val - 1) Dq'
      have hspb : (sq.wrapping_sub 1#usize).val < st.val.length := by
        rw [hsub]; have := Dq.sp_le; omega
      step as ⟨x, hx⟩
      have hx' : x = st.val[sq.val - 1]! := by
        have : x = st.val[(sq.wrapping_sub 1#usize).val]! := by rw [hx, getElem!_pos]
        rw [hsub] at this; exact this
      have hfv : (UScalar.cast .Usize x).val = (st.val[sq.val - 1]!).val := by rw [castUsize_val, hx']
      have hon := (Dq.on (sq.val - 1) (by omega))
      have hfn : (UScalar.cast .Usize x).val < b.faces.size := by rw [hfv]; exact hon.1
      step with R.base.face_n_loops (UScalar.cast .Usize x) hfn as ⟨nlf, hnlf⟩
      have Dp' : DFS b f0 σ (some (UScalar.cast .Usize x).val) sn st (sq.wrapping_sub 1#usize).val := by
        rw [hfv, hsub]; exact Dp
      step with loops_mark_spec SI t b R H f0 s σ hσ (UScalar.cast .Usize x) hfn nlf hnlf sn st
        (sq.wrapping_sub 1#usize) Dp' as ⟨sn2, st2, sq2, D2, hm2, hall, hacc⟩
      step*
      have hmono := unm_mono b σ _ _ hm2
      have hcnt := D2.count
      refine ⟨dfs_close b f0 σ _ sn2 st2 sq2.val D2 hall, mono_trans b σ _ _ _ hm hm2, ?_⟩
      rw [hsub] at hacc
      obtain ⟨d, hd⟩ : ∃ d, unm b σ sn = unm b σ sn2 + d := ⟨_, (Nat.add_sub_cancel' hmono).symm⟩
      rw [hd, Nat.add_mul]
      have : d ≤ d * (b.faces.size + 1) := Nat.le_mul_of_pos_right d (by omega)
      omega
    · rename_i hpos
      have h0 : sq.val = 0 := by scalar_tac
      simp only [WP.spec_ok]
      rw [← h0]; exact ⟨Dq, hm⟩
  · exact ⟨D, mono_refl b σ seen⟩

theorem joined_spec (R : ShellReads SI t b) (s : Usize) (hs : s.val < b.shells.size) (σ : U32)
    (hσ : σ = UScalar.cast .U32 s) (seen : alloc.vec.Vec U32) (hl : seen.val.length = b.faces.size)
    (hin : ∀ f ∈ b.shellList s.val, f < b.faces.size) (n : Usize) (hn : n.val = (b.shellList s.val).length) :
    topo_shell.shell_faces_loop5_loop1 SI t seen s n true 0#usize ⦃ r =>
      r = (b.shellList s.val).all (fun f => decide (seen.val[f]! = σ)) ⦄ := by
  unfold topo_shell.shell_faces_loop5_loop1
  apply loop.spec_decr_nat (fun (y : Bool × Usize) => n.val - y.2.val)
    (fun y => y.2.val ≤ n.val ∧ y.1 = ((b.shellList s.val).take y.2.val).all (fun f => decide (seen.val[f]! = σ)))
  · rintro ⟨jn, i⟩ ⟨hle, hj⟩
    simp only at hle hj ⊢
    unfold topo_shell.shell_faces_loop5_loop1.body
    split
    · rename_i hjt
      split
      · rename_i hlt
        have hlt' : i.val < n.val := by scalar_tac
        have hi : i.val < (b.shellList s.val).length := by omega
        step with R.shell_face s i hs hi as ⟨f, hf⟩
        have hfm : f.val ∈ b.shellList s.val := by
          rw [hf, getElem!_pos (b.shellList s.val) i.val hi]; exact List.getElem_mem hi
        have hfb : f.val < seen.val.length := by rw [hl]; exact hin _ hfm
        step as ⟨x, hx⟩
        have hx' : x = seen.val[f.val]! := by rw [hx, getElem!_pos]
        simp only [lift, Std.bind_ok, ← hσ, WP.spec_ok]
        have hw : (i.wrapping_add 1#usize).val = i.val + 1 := succ_val i n hlt'
        refine ⟨by omega, ?_, by omega⟩
        rw [hw, all_take_succ _ _ _ hi, ← hj, hjt, Bool.true_and, ← hf, hx']
      · rename_i hlt
        have hi : i.val = (b.shellList s.val).length := by scalar_tac
        simp only [WP.spec_ok]
        rw [hj, hi, List.take_length]
    · rename_i hjt
      simp only [WP.spec_ok]
      simp only [Bool.not_eq_true] at hjt
      rw [hjt] at hj
      rw [hjt, ← List.take_append_drop i.val (b.shellList s.val), List.all_append, ← hj]; rfl
  · exact ⟨by simp, by simp⟩

theorem reach_lt (H : Sound b) (f0 : Nat) (h0 : f0 < b.faces.size) :
    ∀ x, b.Reach f0 x → x < b.faces.size := by
  intro x hx
  induction hx with
  | refl => exact h0
  | step _ hy _ => exact nbrs_lt b H _ _ hy

theorem reach_marked (H : Sound b) (f0 : Nat) (h0 : f0 < b.faces.size) (σ : U32) (seen : alloc.vec.Vec U32)
    (hf0 : seen.val[f0]! = σ) (hc : ∀ g < b.faces.size, seen.val[g]! = σ → ∀ y ∈ b.nbrs g, seen.val[y]! = σ) :
    ∀ x, b.Reach f0 x → seen.val[x]! = σ := by
  intro x hx
  induction hx with
  | refl => exact hf0
  | step hr hy ih => exact hc _ (reach_lt b H f0 h0 _ hr) ih _ hy

/-- The model's second list: the sound shells not joined. -/
def discon (b : Body) (s : Nat) : Option Problem :=
  if !b.bad s && !((b.shellList s).all fun f => f ∈ b.reached s) then some (Problem.shellDisconnected s) else none

theorem flood_shells_spec {S : Type} (SK : topo.Sink S) (P : S → List Problem) (hS : AppendsS SK P)
    (R : ShellReads SI t b) (H : Sound b) (ns : Usize) (hns : ns.val = b.shells.size)
    (bad : alloc.vec.Vec Bool) (hbl : bad.val.length = b.shells.size + 1)
    (hb : ∀ s ≤ b.shells.size, bad.val[s]! = b.bad s)
    (seen : alloc.vec.Vec U32) (hsl : seen.val.length = b.faces.size)
    (hmax : ∀ g < b.faces.size, seen.val[g]! = core.num.U32.MAX) (out : S) :
    topo_shell.shell_faces_loop5 SI SK t out ns bad seen (alloc.vec.Vec.new U32) 0#usize ⦃ r =>
      P r = P out ++ (List.range b.shells.size).filterMap (discon b) ⦄ := by
  have hN := nbrs_lt b H
  have hns32 := R.ns_lt
  have hnf32 := R.nf_le
  unfold topo_shell.shell_faces_loop5
  apply loop.spec_decr_nat
    (fun (y : S × alloc.vec.Vec U32 × alloc.vec.Vec U32 × Usize) => ns.val - y.2.2.2.val)
    (fun y => y.2.2.2.val ≤ ns.val ∧ y.2.1.val.length = b.faces.size ∧
      (∀ g < b.faces.size, (y.2.1.val[g]!).val < y.2.2.2.val ∨ y.2.1.val[g]! = core.num.U32.MAX) ∧
      y.2.2.1.val.length ≤ b.faces.size ∧
      P y.1 = P out ++ (List.range y.2.2.2.val).filterMap (discon b))
  · rintro ⟨o, sn, st, s⟩ ⟨hle, hlen, hst, hstl, ho⟩
    simp only at hle hlen hst hstl ho ⊢
    unfold topo_shell.shell_faces_loop5.body
    split
    · rename_i hlt
      have hlt' : s.val < ns.val := by scalar_tac
      have hs : s.val < b.shells.size := by omega
      have hw : (s.wrapping_add 1#usize).val = s.val + 1 := succ_val s ns hlt'
      have hacc : ∀ o', P o' = P o ++ (discon b s.val).toList →
          P o' = P out ++ (List.range (s.val + 1)).filterMap (discon b) := by
        intro o' h; rw [h, filterMap_range_succ, ho, List.append_assoc]
      have hsb : s.val < bad.val.length := by omega
      step as ⟨x, hx⟩
      have hx' : x = b.bad s.val := by rw [hx, ← hb s.val (by omega), getElem!_pos]
      split
      · rename_i hbad
        step as ⟨s1, hs1⟩
        have hw : s1.val = s.val + 1 := by rw [hs1]; exact succ_val s ns hlt'
        refine ⟨by rw [hw]; omega, hlen, fun g hg => ?_, hstl, ?_, by rw [hw]; omega⟩
        · rw [hw]; rcases hst g hg with h | h
          · left; omega
          · right; exact h
        · rw [hw]; apply hacc
          have : discon b s.val = none := by
            have hbt : b.bad s.val = true := by rw [← hx']; exact hbad
            simp [discon, hbt]
          rw [this]; simp
      · rename_i hbad
        have hbad' : b.bad s.val = false := by rw [← hx']; simpa using hbad
        have hb1 : bad1 b s.val = false := by
          have := bad_eq b s.val; rw [hbad'] at this; simp at this; exact this.1
        have hb1' : ((b.shellList s.val).isEmpty || (b.shellList s.val).any (badEntry b s.val)) = false := by
          unfold bad1 at hb1; rw [decide_eq_true hs, Bool.true_and] at hb1; exact hb1
        rw [Bool.or_eq_false_iff] at hb1'
        have hne : b.shellList s.val ≠ [] := by
          intro h; rw [h] at hb1'; simp at hb1'
        have hin : ∀ f ∈ b.shellList s.val, f < b.faces.size ∧ b.faceShell f = s.val := by
          intro f hf
          have := List.any_eq_false.1 hb1'.2 f hf
          unfold badEntry at this
          simp only [Bool.not_eq_true', Bool.not_eq_false', Bool.and_eq_true, decide_eq_true_eq,
            beq_iff_eq] at this
          simpa using this
        have hl0 : 0 < (b.shellList s.val).length := List.length_pos_iff.2 hne
        step with R.shell_face s 0#usize hs (by simpa using hl0) as ⟨f0, hf0⟩
        have hf0m : f0.val ∈ b.shellList s.val := by
          rw [hf0]; rw [getElem!_pos (b.shellList s.val) _ hl0]; exact List.getElem_mem hl0
        have hf0n : f0.val < b.faces.size := (hin _ hf0m).1
        simp only [alloc.vec.Vec.is_empty, Std.bind_ok]
        apply WP.spec_bind (Pₘ := fun st1 : alloc.vec.Vec U32 => st1.val[0]! = UScalar.cast .U32 f0 ∧
          1 ≤ st1.val.length ∧ st1.val.length ≤ b.faces.size)
        · split
          · rename_i hemp
            have hemp' : st.val.length = 0 := by simpa using hemp
            simp only [lift, Std.bind_ok]
            have hpl : st.val.length < Usize.max := by
              rw [hemp']; rcases Usize.bounds_eq with h | h <;> rw [h] <;> simp [U32.max_eq, U64.max_eq]
            apply WP.spec_mono (alloc.vec.Vec.push_spec st (UScalar.cast .U32 f0) hpl)
            intro st1 hst1v
            rw [hst1v, List.length_eq_zero_iff.1 hemp']
            refine ⟨rfl, by simp, by simp; omega⟩
          · rename_i hemp
            have hemp' : 0 < st.val.length := by
              have : ¬ st.val.length = 0 := by simpa using hemp
              omega
            step*
            rw [index_mut_back_post, i_post]
            simp only [alloc.vec.Vec.set_val_eq, List.length_set]
            refine ⟨?_, hemp', hstl⟩
            rw [getElem!_set _ _ _ _ hemp']; rfl
        intro st1 hst1
        have hσv : (UScalar.cast .U32 s).val = s.val := cast32_val s (by omega)
        have hfresh : ∀ g < b.faces.size, sn.val[g]! ≠ UScalar.cast .U32 s := by
          intro g hg h
          have := congrArg UScalar.val h
          rw [hσv] at this
          rcases hst g hg with h1 | h1
          · omega
          · rw [h1, max_val] at this; omega
        have hf0b : f0.val < sn.val.length := by rw [hlen]; exact hf0n
        step as ⟨i0, back, hb1, hb2⟩
        try simp only [lift, Std.bind_ok]
        try rw [hb2]
        have hset : ∀ g < b.faces.size, (sn.set f0 (UScalar.cast .U32 s)).val[g]! =
            if g = f0.val then UScalar.cast .U32 s else sn.val[g]! := by
          intro g hg; simp only [alloc.vec.Vec.set_val_eq]; rw [getElem!_set _ _ _ _ (by rw [hlen]; exact hg)]
        have hcount := unm_mark b (UScalar.cast .U32 s) sn f0 hf0n hlen (hfresh _ hf0n)
        have hunm : unm b (UScalar.cast .U32 s) sn ≤ b.faces.size := by
          unfold unm; exact (List.countP_le_length).trans (by simp)
        have D0 : DFS b f0.val (UScalar.cast .U32 s) none (sn.set f0 (UScalar.cast .U32 s)) st1 (1#usize).val := by
          have h1 : (1#usize).val = 1 := rfl
          rw [h1]
          refine ⟨by simp [alloc.vec.Vec.set_val_eq, hlen], ?_, hst1.2.1, hst1.2.2, ?_, ?_, by omega, by simp⟩
          · intro g hg h
            rw [hset g hg] at h
            split at h
            · rename_i hgf; subst hgf; exact TopoCheck.Body.Reach.refl
            · exact absurd h (hfresh g hg)
          · intro j hj
            have : j = 0 := by omega
            subst this
            rw [hst1.1, cast32_val f0 (by omega)]
            exact ⟨hf0n, by rw [hset _ hf0n, if_pos rfl]⟩
          · intro g hg h
            rw [hset g hg] at h
            split at h
            · rename_i hgf; left; exact ⟨0, by omega, by rw [hst1.1, cast32_val f0 (by omega), hgf]⟩
            · exact absurd h (hfresh g hg)
        step with pop_loop_spec SI t b R H f0.val s (UScalar.cast .U32 s) rfl _ st1 1#usize D0 as ⟨sn2, st2, D2, hm2⟩
        step with R.shell_n_faces s hs as ⟨n, hn⟩
        have hin' : ∀ f ∈ b.shellList s.val, f < b.faces.size := fun f hf => (hin f hf).1
        step with joined_spec SI t b R s hs (UScalar.cast .U32 s) rfl sn2 D2.len hin' n hn as ⟨jn, hjn⟩
        have hclosed : ∀ g < b.faces.size, sn2.val[g]! = UScalar.cast .U32 s →
            ∀ y ∈ b.nbrs g, sn2.val[y]! = UScalar.cast .U32 s := by
          intro g hg h
          rcases D2.done g hg h with ⟨j, hj, _⟩ | hc | hall
          · omega
          · simp at hc
          · exact hall
        have hf0m2 : sn2.val[f0.val]! = UScalar.cast .U32 s :=
          hm2.1 _ hf0n (by rw [hset _ hf0n, if_pos rfl])
        have hhead : (b.shellList s.val).headD 0 = f0.val := by
          rw [hf0]; cases hl : b.shellList s.val with
          | nil => exact absurd hl hne
          | cons a t => simp
        have hjall : jn = (b.shellList s.val).all (fun f => decide (f ∈ b.reached s.val)) := by
          rw [hjn, Bool.eq_iff_iff, List.all_eq_true, List.all_eq_true]
          apply forall_congr'; intro f; apply imp_congr_right; intro hf
          have hfn := hin' f hf
          rw [decide_eq_true_iff, decide_eq_true_iff, TopoCheck.Body.reached_iff b hN s.val
            (by rw [hhead]; exact hf0n) f, hhead]
          constructor
          · intro h; exact D2.reach f hfn h
          · intro h; exact reach_marked b H f0.val hf0n _ sn2 hf0m2 hclosed f h
        have hdis : discon b s.val = if jn then none else some (Problem.shellDisconnected s.val) := by
          unfold discon; rw [hbad', hjall]; cases (b.shellList s.val).all (fun f => decide (f ∈ b.reached s.val)) <;> simp
        have hW3 : ∀ g < b.faces.size, (sn2.val[g]!).val < s.val + 1 ∨ sn2.val[g]! = core.num.U32.MAX := by
          intro g hg
          rcases hm2.2 g hg with h | h
          · rw [h, hset g hg]; split
            · left; rw [hσv]; omega
            · rcases hst g hg with h1 | h1
              · left; omega
              · right; exact h1
          · left; rw [h, hσv]; omega
        split
        · rename_i hj
          step*
          have hw1 : (s.wrapping_add 1#usize).val = s.val + 1 := succ_val s ns hlt'
          rw [hw1]
          refine ⟨by omega, D2.len, hW3, D2.stack_le, ?_, by omega⟩
          apply hacc; rw [hdis, hj]; simp
        · rename_i hj
          step with hS.2 o s as ⟨o1, ho1⟩
          step*
          have hw1 : (s.wrapping_add 1#usize).val = s.val + 1 := succ_val s ns hlt'
          rw [hw1]
          refine ⟨by omega, D2.len, hW3, D2.stack_le, ?_, by omega⟩
          apply hacc; rw [ho1, hdis]; simp only [Bool.not_eq_true] at hj; rw [hj]; simp
    · rename_i hlt
      have : s.val = ns.val := by scalar_tac
      simp only [WP.spec_ok]
      rw [ho, this, hns]
  · refine ⟨by simp, hsl, fun g hg => Or.inr (hmax g hg), by simp, by simp⟩

/-! ## The whole of `shell_faces` -/

/-- **The shipped `check_shell_faces` refines the model.** For a body whose links are sound (as
`check_known` runs it: only once `check_links` found nothing), any `ShellTopo` view that reads it
(`ShellReads`, as check.rs's `ShellView` does) and any sink that appends what it is handed, the
translated Rust `shell_faces` terminates without panic or overflow and appends exactly
`Model.checkShellFaces`. -/
theorem shell_faces_refines {S : Type} (SK : topo.Sink S) (P : S → List Problem) (hS : AppendsS SK P)
    (R : ShellReads SI t b) (hL : b.checkLinks = []) (out : S) :
    topo_shell.shell_faces SI SK t out ⦃ r => P r = P out ++ b.checkShellFaces ⦄ := by
  have H := sound_of b hL
  have hns32 := R.ns_lt
  unfold topo_shell.shell_faces
  have rns := R.ns
  have rnf := R.base.nf
  have rne := R.base.ne
  step with rns as ⟨ns, hns⟩
  step with rnf as ⟨nf, hnf⟩
  step with rne as ⟨ne, hne⟩
  have hw : (ns.wrapping_add 1#usize).val = b.shells.size + 1 := by
    rw [Usize.wrapping_add_val_eq, hns]; apply Nat.mod_eq_of_lt
    have := succ_lt_usize b.shells.size hns32
    have h1 : (1#usize).val = 1 := rfl
    simp only [UScalar.size, h1]; exact this
  simp only [lift, Std.bind_ok]
  step as ⟨bad, hbad⟩
  step as ⟨listed, hlisted⟩
  have hb0 : BadS b 0 false bad := by
    refine ⟨by rw [hbad]; simp [hw], fun s hs => ?_⟩
    rw [hbad]; simp [hw]
  have hk0 : KeptS b 0 0 listed := by
    refine ⟨by rw [hlisted]; simp [hnf], fun f hf => ?_⟩
    rw [hlisted]; simp [preS, hnf, hf]
  step with shell_pass_spec SI t b R ns nf hns hnf bad listed hb0 hk0 as ⟨bad1, listed1, hb1, hk1⟩
  have hbt0 : BadT b 0 0 bad1 := by
    refine ⟨hb1.1, fun s hs => ?_⟩
    rw [hb1.2 s hs]
    by_cases h : s < b.shells.size
    · simp [h, badF, badE]
    · have : s = b.shells.size := by omega
      subst this; simp [forge_topo.bad1, badF, badE]
  step with face_listed_spec SI t b R ns nf hns hnf listed1 hk1 bad1 hbt0 as ⟨bad2, hb2⟩
  step with edge_split_spec SI t b R H ns ne hns hne bad2 hb2 as ⟨bad3, hb3⟩
  step with emit_bad_spec b SK P hS ns hns hns32 bad3 hb3 out as ⟨out1, ho1⟩
  step with fill_spec nf listed1 (by rw [hk1.1, hnf]) as ⟨seen, hsl, hmax⟩
  have hbb : ∀ s ≤ b.shells.size, bad3.val[s]! = b.bad s := by
    intro s hs; rw [hb3.2 s hs, bad_eq]
  have hsl' : seen.val.length = b.faces.size := by rw [hsl, hnf]
  have hmax' : ∀ g < b.faces.size, seen.val[g]! = core.num.U32.MAX := by
    intro g hg; exact hmax g (by rw [hnf]; exact hg)
  apply WP.spec_mono (flood_shells_spec SI t b SK P hS R H ns hns bad3 hb3.1 hbb seen hsl' hmax' out1)
  intro r hr
  rw [hr, ho1, Body.checkShellFaces, List.append_assoc]
  rfl

end forge_topo
