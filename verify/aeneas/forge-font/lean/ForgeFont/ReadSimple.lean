import ForgeFont.Contour

open Aeneas Aeneas.Std Result ControlFlow
open forge_font

namespace ForgeFont

/-- Through a translated body: simplify, take a step, or split, until none applies. -/
macro "walk" : tactic => `(tactic| iterate 6 (all_goals (repeat (first
  | simp only [↓reduceIte, Bool.false_eq_true, bind_ok, Aeneas.Std.bind_tc_ok,
      uncurry_apply_pair, Aeneas.Std.bind_assoc_eq, Aeneas.Std.bind_assoc]
  | step
  | split))))

@[step]
theorem byte_or0_spec (d : Slice U8) (o : Usize) : glyf_core.byte_or0 d o ⦃ _ => True ⦄ := by
  unfold glyf_core.byte_or0
  step*

@[step]
theorem word_or0_spec (d : Slice U8) (o : Usize) (h : o.val + 1 ≤ Usize.max) :
    glyf_core.word_or0 d o ⦃ _ => True ⦄ := by
  unfold glyf_core.word_or0
  step*

@[step]
theorem word_spec (d : Slice U8) (o : Usize) (h : o.val + 1 ≤ Usize.max) :
    glyf_core.word d o ⦃ _ => True ⦄ := by
  unfold glyf_core.word
  step*

/-- The contours' ends: `n` of them, each a word. -/
@[step]
theorem ends_spec (d : Slice U8) (a n : Usize) (h : a.val + 10 + 2 * n.val ≤ Usize.max) :
    glyf_core.read_simple_loop0 d a n (alloc.vec.Vec.with_capacity Usize n) 0#usize ⦃ ends =>
      ends.val.length = n.val ∧ ∀ e ∈ ends.val, e.val < 2^16 ⦄ := by
  unfold glyf_core.read_simple_loop0
  apply loop.spec_decr_nat (fun (s : alloc.vec.Vec Usize × Usize) => n.val - s.2.val)
    (fun s => s.2.val ≤ n.val ∧ s.1.val.length = s.2.val ∧ ∀ e ∈ s.1.val, e.val < 2^16)
  · rintro ⟨ends, k⟩ ⟨hk, hl, he⟩
    simp only at hk hl he
    unfold glyf_core.read_simple_loop0.body
    dsimp only
    split
    · rename_i hkn
      have hkn' : k.val < n.val := hkn
      step*
      have hb := i3.hBounds
      simp only [UScalarTy.numBits] at hb
      have hc : i4.val = i3.val := by
        rw [i4_post]; simp only [UScalar.cast_val_eq]
        apply Nat.mod_eq_of_lt
        rcases System.Platform.numBits_eq with h | h <;> simp [h] <;> omega
      refine ⟨by omega, by simp [ends1_post, hl, k1_post], ?_, by omega⟩
      intro e he'
      rw [ends1_post] at he'
      rcases List.mem_append.1 he' with h1 | h1
      · exact he e h1
      · simp at h1; subst h1; omega
    · simp_all; omega
  · simp [alloc.vec.Vec.with_capacity]

/-- The flags: `points` of them unless the data ends first; the offset never passes the end
by more than where it started. -/
@[step]
theorem flags_spec (d : Slice U8) (points o0 : Usize) (hp : points.val ≤ 2^17)
    (ho : o0.val + 2 ≤ Usize.max) (hd : d.val.length + 2 ≤ Usize.max) :
    glyf_core.read_simple_loop1 d points o0 (alloc.vec.Vec.with_capacity U8 points) 0#u8 0#u8 false
      ⦃ o flags bad => (bad = false → flags.val.length = points.val) ∧
        o.val ≤ max o0.val d.val.length ⦄ := by
  unfold glyf_core.read_simple_loop1
  apply loop.spec_decr_nat
    (fun (s : Usize × alloc.vec.Vec U8 × U8 × U8 × Bool) =>
      2 * (points.val - s.2.1.val.length) + if s.2.2.2.2 then 0 else 1)
    (fun s => s.2.1.val.length ≤ points.val ∧ s.1.val ≤ max o0.val d.val.length)
  · rintro ⟨o, flags, last, repeats, bad⟩ ⟨hl, hob⟩
    simp only at hl hob
    unfold glyf_core.read_simple_loop1.body
    have hpm : points.val < Usize.max := by scalar_tac
    cases bad
    · simp only [Bool.false_eq_true, ↓reduceIte]
      split
      · rename_i hlt
        have hlt' : flags.val.length < points.val := by simpa using hlt
        split
        · step*
          simp only [flags1_post, List.length_append, List.length_singleton, Bool.false_eq_true,
            ↓reduceIte]
          omega
        · step*
          all_goals (simp only [flags1_post, List.length_append, List.length_singleton,
            Bool.false_eq_true, ↓reduceIte]; scalar_tac)
      · rename_i hlt
        have hlt' : ¬ flags.val.length < points.val := by simpa using hlt
        simp only [WP.spec_ok]
        exact ⟨fun _ => by omega, hob⟩
    · simp; omega
  · simp [alloc.vec.Vec.with_capacity]

@[step]
theorem hcast_u8_i64 (x : U8) : lift (UScalar.hcast IScalarTy.I64 x) ⦃ y => y.val = x.val ⦄ :=
  UScalar.hcast_inBounds_spec _ x (by have := x.hBounds; simp [UScalarTy.numBits] at this; scalar_tac)

@[step]
theorem hcast_u16_i16 (x : U16) :
    lift (UScalar.hcast IScalarTy.I16 x) ⦃ y => -2^15 ≤ y.val ∧ y.val < 2^15 ⦄ := by
  simp only [lift, WP.spec_ok]
  have := (UScalar.hcast IScalarTy.I16 x).hBounds
  simpa [IScalarTy.numBits] using this

@[step]
theorem cast_i16_i64 (x : I16) : lift (IScalar.cast IScalarTy.I64 x) ⦃ y => y.val = x.val ⦄ :=
  IScalar.cast_inBounds_spec _ x (by have := x.hBounds; simp [IScalarTy.numBits] at this; scalar_tac)

@[step]
theorem cast_i64_i32 (x : I64) : lift (IScalar.cast IScalarTy.I32 x) ⦃ _ => True ⦄ := by
  simp [lift]

/-- One axis's coordinates: one per flag unless the data ends first, the running sum never
overflowing (each delta is at most 2¹⁵ either way, and there are at most 2¹⁷ of them). -/
theorem coords_loop_spec (d flags : Slice U8) (short same : U8) (o0 : Usize)
    (hf : flags.val.length ≤ 2^17) (ho : o0.val + 3 ≤ Usize.max)
    (hd : d.val.length + 3 ≤ Usize.max) :
    glyf_core.coords_loop d flags short same (alloc.vec.Vec.with_capacity I32 flags.len) 0#i64 o0
      0#usize false ⦃ out o bad =>
        (bad = false → out.val.length = flags.val.length) ∧ o.val ≤ max o0.val d.val.length ⦄ := by
  unfold glyf_core.coords_loop
  apply loop.spec_decr_nat
    (fun (s : alloc.vec.Vec I32 × I64 × Usize × Usize × Bool) =>
      flags.val.length - s.2.2.2.1.val + if s.2.2.2.2 then 0 else 1)
    (fun s => s.2.2.2.1.val ≤ flags.val.length ∧ (s.2.2.2.2 = false → s.1.val.length = s.2.2.2.1.val) ∧
      s.1.val.length ≤ s.2.2.2.1.val ∧
      -(s.2.2.2.1.val * 2^15 : Int) ≤ s.2.1.val ∧ s.2.1.val ≤ s.2.2.2.1.val * 2^15 ∧
      s.2.2.1.val ≤ max o0.val d.val.length)
  · rintro ⟨out, v, o, k, bad⟩ ⟨hk, hl, hl', hv0, hv1, hob⟩
    simp only at hk hl hl' hv0 hv1 hob
    unfold glyf_core.coords_loop.body
    cases bad
    · simp only [Bool.false_eq_true, ↓reduceIte]
      split
      · rename_i hlt
        have hlt' : k.val < flags.val.length := by simpa using hlt
        have hB0 : o0.val ≤ max o0.val d.val.length := Nat.le_max_left _ _
        have hBl : d.val.length ≤ max o0.val d.val.length := Nat.le_max_right _ _
        have hB : max o0.val d.val.length + 3 ≤ Usize.max := by
          rcases Nat.le_total o0.val d.val.length with h | h
          · rw [Nat.max_eq_right h]; omega
          · rw [Nat.max_eq_left h]; omega
        have hpush : out.val.length < Usize.max := by scalar_tac
        have hk1 : k.val + 1 ≤ Usize.max := by scalar_tac
        generalize max o0.val d.val.length = B at hB0 hBl hB hob ⊢
        walk
        all_goals (try (have hlen1 := congrArg List.length out1_post
                        simp only [List.length_append, List.length_singleton] at hlen1))
        all_goals (split_conjs <;> first | scalar_tac | (intro _; scalar_tac) | (simp; scalar_tac))
      · rename_i hlt
        have hlt' : ¬ k.val < flags.val.length := by simpa using hlt
        simp only [WP.spec_ok]
        exact ⟨fun _ => by simp at hl; omega, hob⟩
    · simp; omega
  · simp [alloc.vec.Vec.with_capacity]

@[step]
theorem coords_spec (d flags : Slice U8) (short same : U8) (o0 : Usize)
    (hf : flags.val.length ≤ 2^17) (ho : o0.val + 3 ≤ Usize.max)
    (hd : d.val.length + 3 ≤ Usize.max) :
    glyf_core.coords d o0 flags short same ⦃ r => ∀ xs o, r = .Ok (xs, o) →
      xs.val.length = flags.val.length ∧ o.val ≤ max o0.val d.val.length ⦄ := by
  unfold glyf_core.coords
  step with coords_loop_spec d flags short same o0 hf ho hd as ⟨out, o, bad, h1, h2⟩
  split
  · simp
  · rename_i hb
    simp only [WP.spec_ok, core.result.Result.Ok.injEq, Prod.mk.injEq]
    rintro xs o' ⟨rfl, rfl⟩
    exact ⟨h1 (by simpa using hb), h2⟩

/-- The on-curve flags, one per point. -/
@[step]
theorem on_spec (points : Usize) (flags : alloc.vec.Vec U8) (hp : points.val ≤ flags.val.length)
    (hm : points.val < Usize.max) :
    glyf_core.read_simple_loop2 points flags (alloc.vec.Vec.with_capacity Bool points) 0#usize
      ⦃ on => on.val.length = points.val ⦄ := by
  unfold glyf_core.read_simple_loop2
  apply loop.spec_decr_nat (fun (s : alloc.vec.Vec Bool × Usize) => points.val - s.2.val)
    (fun s => s.2.val ≤ points.val ∧ s.1.val.length = s.2.val)
  · rintro ⟨on, i⟩ ⟨hi, hl⟩
    simp only at hi hl
    unfold glyf_core.read_simple_loop2.body
    dsimp only
    walk
    all_goals (try (have hlen1 := congrArg List.length on1_post
                    simp only [List.length_append, List.length_singleton] at hlen1))
    all_goals (split_conjs <;> first | scalar_tac | (simp; scalar_tac))
  · simp [alloc.vec.Vec.with_capacity]

/-- **`read_simple` never fails** (no overflow, no index out of range, whatever the bytes), and
what it reads has the shape `contour` needs: `n` contour ends, each below 2¹⁶, and as many on
flags, x and y coordinates as the last end says there are points. -/
theorem read_simple_spec (d : Slice U8) (a n budget : Usize) (hn : n.val < 2^16)
    (ha : a.val + 2^18 ≤ Usize.max) (hd : d.val.length + 2^18 ≤ Usize.max) :
    glyf_core.read_simple d a n budget ⦃ r => ∀ s, r = .Ok s →
      s.ends.val.length = n.val ∧ (∀ e ∈ s.ends.val, e.val < 2^16) ∧
      s.on.val.length = (if n.val = 0 then 0 else (s.ends.val.getD (n.val - 1) 0#usize).val + 1) ∧
      s.xs.val.length = s.on.val.length ∧ s.ys.val.length = s.on.val.length ⦄ := by
  have hmax : ∀ x y : Nat, max x y ≤ x + y := fun x y => Nat.max_le.2 ⟨by omega, by omega⟩
  unfold glyf_core.read_simple
  step as ⟨i, hi⟩
  step as ⟨i1, hi1⟩
  step as ⟨i2, hi2⟩
  split
  · simp
  step with ends_spec d a n (by scalar_tac) as ⟨ends1, he1, he2⟩
  apply WP.spec_bind (Pₘ := fun (p : Usize) =>
    p.val = (if n.val = 0 then 0 else (ends1.val.getD (n.val - 1) 0#usize).val + 1))
  · split
    · simp_all
    · rename_i hn0
      have hn0' : n.val ≠ 0 := by intro h; apply hn0; scalar_tac
      step as ⟨m, hm⟩
      step as ⟨e, he⟩
      have hel : e.val < 2^16 := he2 e (by rw [he]; exact List.getElem_mem _)
      step as ⟨p, hp⟩
      simp only [hn0', ↓reduceIte, hp, he, hm, List.getD_eq_getElem?_getD]
      rw [List.getElem?_eq_getElem (by omega)]; rfl
  intro points hpts
  have hpb : points.val ≤ 2^16 := by
    rw [hpts]; split
    · omega
    · have := he2 (ends1.val.getD (n.val - 1) 0#usize) (by
        rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by omega)]; exact List.getElem_mem _)
      omega
  step as ⟨i4, hi4⟩
  split
  · simp
  step as ⟨i5, hi5⟩
  step as ⟨i6, hi6⟩
  step as ⟨i7, hi7⟩
  step with word_spec d i7 (by scalar_tac) as ⟨r, _⟩
  cases r with
  | Err e => simp [core.result.Result.Insts.CoreOpsTry.branch,
      core.result.Result.Insts.CoreOpsTry_traitFromResidualResult.from_residual]
  | Ok ins0 =>
    simp only [core.result.Result.Insts.CoreOpsTry.branch, bind_ok]
    have hins := ins0.hBounds
    simp only [UScalarTy.numBits] at hins
    step as ⟨ins, hins'⟩
    have hinsv : ins.val = ins0.val := by
      rw [hins']; simp only [UScalar.cast_val_eq]
      apply Nat.mod_eq_of_lt
      rcases System.Platform.numBits_eq with h | h <;> simp [h] <;> omega
    step as ⟨i8, hi8⟩
    step as ⟨i9, hi9⟩
    step as ⟨o, ho⟩
    step with flags_spec d points o (by omega) (by scalar_tac) (by scalar_tac)
      as ⟨o1, flags1, bad, hf1, hf2⟩
    split
    · simp
    rename_i hbad
    have hbad' : bad = false := by simpa using hbad
    have hfl : flags1.deref.val.length = points.val := by
      simp [alloc.vec.Vec.deref, hf1 hbad']
    have ho1 := hmax o.val d.val.length
    step with coords_spec d flags1.deref 2#u8 16#u8 o1 (by omega) (by scalar_tac) (by scalar_tac)
      as ⟨r1, hr1⟩
    cases r1 with
    | Err e => simp [core.result.Result.Insts.CoreOpsTry_traitFromResidualResult.from_residual]
    | Ok v1 =>
      obtain ⟨xs, o2⟩ := v1
      obtain ⟨hxs, ho2⟩ := hr1 xs o2 rfl
      have ho2' := hmax o1.val d.val.length
      simp only [bind_ok]
      step with coords_spec d flags1.deref 4#u8 32#u8 o2 (by omega) (by scalar_tac) (by scalar_tac)
        as ⟨r2, hr2⟩
      cases r2 with
      | Err e => simp [core.result.Result.Insts.CoreOpsTry_traitFromResidualResult.from_residual]
      | Ok v2 =>
        obtain ⟨ys, o3⟩ := v2
        obtain ⟨hys, _⟩ := hr2 ys o3 rfl
        simp only [bind_ok]
        step with on_spec points flags1 (by simp [alloc.vec.Vec.deref] at hfl; omega) (by scalar_tac)
          as ⟨on1, hon⟩
        simp only [WP.spec_ok, core.result.Result.Ok.injEq]
        rintro s rfl
        refine ⟨he1, he2, by rw [hon, hpts], by rw [hxs, hfl, hon], by rw [hys, hfl, hon]⟩

/-- **Reader and walk together**, as `Font::outline_into` uses them: for a glyph `read_simple`
returned, every contour the shell asks for (it checks `first ≤ e < on.len` before each, refusing
otherwise) is `contourOf` of its points, and so closed. -/
theorem contour_of_read (d : Slice U8) (a n budget : Usize) (hn : n.val < 2^16)
    (ha : a.val + 2^18 ≤ Usize.max) (hd : d.val.length + 2^18 ≤ Usize.max) :
    glyf_core.read_simple d a n budget ⦃ r => ∀ s, r = .Ok s →
      ∀ first last : Usize, first.val ≤ last.val → last.val < s.on.val.length →
        glyf_core.contour s first last ⦃ v =>
          v.val.map toSeg = ForgeText.contourOf (pts s first.val (last.val + 1 - first.val)) ⦄ ⦄ := by
  apply WP.spec_mono (read_simple_spec d a n budget hn ha hd)
  intro r h s hs first last h1 h2
  obtain ⟨_, he2, hon, hx, hy⟩ := h s hs
  have hsmall : s.on.val.length ≤ 2^16 := by
    rw [hon]; split
    · omega
    · rename_i hn0
      have := he2 (s.ends.val.getD (n.val - 1) 0#usize) (by
        rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by omega)]
        exact List.getElem_mem _)
      omega
  exact contour_spec s first last h1 h2 hx hy (by scalar_tac)

end ForgeFont
