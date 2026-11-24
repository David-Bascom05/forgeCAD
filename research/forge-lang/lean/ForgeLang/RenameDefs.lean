import ForgeLang.RenamePart

/-!
# Rename in a part document, definitions included

Rust's Rename is token-level across the whole text: a function's, a procedure's or a
component's body that reads `a` reads `b` after it (they see the names bound where they are
called, and a component its document's visible names, `vis`). `RenamePart.rename_body` assumed no
definition reads either name; here the definitions are renamed with the document
(`rnDefs`: bodies, defaults and `vis`), as the op does, and the run is the same:

* `renD_nested`, `renD_top`: as in `RenamePart`, with `rnDefs a b D` on the renamed side; a call,
  a procedure's use and a component's occurrence go through the same induction instead of the
  free-name lemma.
* `rename_defs`: the document. The same fuel, the same traces and log, bindings renamed, ids
  renamed in their first segment only.

Hypotheses (`DefsRn`): no definition's parameter is either name, `b` is read by no definition
and is in no component's `vis`, and no procedure or component body binds either name.
-/

namespace ForgeLang

/-- A component renamed: its defaults, its body, the names it sees. -/
def rnComp (a b : Name) (C : Comp) : Comp :=
  { params := rnKV a b C.params, body := rnB a b C.body, vis := C.vis.map (rn a b) }

/-- The definitions renamed with the document. -/
def rnDefs (a b : Name) (D : Defs) : Defs :=
  { fns := D.fns.map fun f => (f.1, (f.2.1, rnE a b f.2.2)),
    procs := D.procs.map fun p => (p.1, (p.2.1, rnB a b p.2.2)),
    comps := D.comps.map fun c => (c.1, rnComp a b c.2),
    track := D.track }

/-- What renaming the definitions needs. -/
def DefsRn (a b : Name) (D : Defs) : Prop :=
  (∀ f ∈ D.fns, f.2.1 ≠ a ∧ f.2.1 ≠ b ∧ b ∉ fvE f.2.2) ∧
  (∀ p ∈ D.procs, p.2.1 ≠ a ∧ p.2.1 ≠ b ∧ NoBindB a b p.2.2 ∧ b ∉ fvB p.2.2) ∧
  (∀ c ∈ D.comps, (∀ q ∈ c.2.params, q.1 ≠ a ∧ q.1 ≠ b ∧ b ∉ fvE q.2) ∧ NoBindB a b c.2.body ∧
    b ∉ fvB c.2.body ∧ b ∉ c.2.vis)

theorem lookup_mapv {β γ : Type} (g : β → γ) :
    ∀ (l : List (Name × β)) (k : Name), (l.map fun p => (p.1, g p.2)).lookup k = (l.lookup k).map g
  | [], _ => rfl
  | (k', v) :: l, k => by
    simp only [List.map_cons, List.lookup]
    split <;> simp_all [lookup_mapv g l k]

@[simp] theorem obs_rnDefs (a b : Name) (D : Defs) (t : Tr) : obs (rnDefs a b D) t = obs D t := rfl

theorem fns_rnDefs (a b : Name) (D : Defs) (f : Name) :
    (rnDefs a b D).fns.lookup f = (D.fns.lookup f).map fun q => (q.1, rnE a b q.2) :=
  lookup_mapv (fun q : Name × Expr => (q.1, rnE a b q.2)) D.fns f

theorem procs_rnDefs (a b : Name) (D : Defs) (p : Name) :
    (rnDefs a b D).procs.lookup p = (D.procs.lookup p).map fun q => (q.1, rnB a b q.2) :=
  lookup_mapv (fun q : Name × List Stmt => (q.1, rnB a b q.2)) D.procs p

theorem comps_rnDefs (a b : Name) (D : Defs) (c : Name) :
    (rnDefs a b D).comps.lookup c = (D.comps.lookup c).map (rnComp a b) :=
  lookup_mapv (rnComp a b) D.comps c

theorem argsOK_rnKV2 {a b : Name} (ps : List (Name × Expr)) :
    ∀ (args : List (Name × Expr)), argsOK (rnKV a b ps) (rnKV a b args) = argsOK ps args
  | [] => rfl
  | (k, e) :: args => by
    have := argsOK_rnKV2 (a := a) (b := b) ps args
    simp only [argsOK, rnKV, List.all_cons, rnKV_keys] at this ⊢
    rw [this]; cases ps.lookup k <;> rfl

theorem filter_rnKV {a b : Name} (P : Name → Bool) :
    ∀ (ps : List (Name × Expr)), (rnKV a b ps).filter (fun p => P p.1) = rnKV a b (ps.filter fun p => P p.1)
  | [] => rfl
  | (k, e) :: ps => by
    simp only [rnKV, List.filter_cons]
    cases P k <;> simp [rnKV, filter_rnKV P ps]

theorem dflts_rnKV2 {a b : Name} (args : List (Name × Expr)) (ps : List (Name × Expr)) :
    dflts (rnKV a b ps) (rnKV a b args) = rnKV a b (dflts ps args) := by
  have hp : (fun p : Name × Expr => ((rnKV a b args).lookup p.1).isNone) =
      (fun p => (args.lookup p.1).isNone) := by
    funext p; rw [rnKV_keys]; cases args.lookup p.1 <;> rfl
  unfold dflts; rw [hp]; exact filter_rnKV (fun k => (args.lookup k).isNone) ps

theorem rrel_append_keep {a b : Name} {ρ ρ' : Env} (h : RRel a b ρ ρ') (Δ : Env)
    (hk : ∀ k ∈ keys Δ, k ≠ a ∧ k ≠ b) : RRel a b (Δ ++ ρ) (Δ ++ ρ') := by
  have := rrel_append_ren h Δ fun k h' => (hk k h').2
  rwa [rnEnv_keep Δ fun k h' => (hk k h').1] at this

theorem rrel_only {a b : Name} {ρ ρ' : Env} (h : RRel a b ρ ρ') {V : List Name} (hv : b ∉ V) :
    RRel a b (only V ρ) (only (V.map (rn a b)) ρ') := by
  intro x hx
  rw [lookup_only, lookup_only]
  have hm : rn a b x ∈ V.map (rn a b) ↔ x ∈ V := by
    constructor
    · intro hm
      obtain ⟨y, hy, e⟩ := List.mem_map.1 hm
      rw [← rn_inj (fun h' => hv (by rw [← h']; exact hy)) hx e]; exact hy
    · exact fun h' => List.mem_map_of_mem h'
  by_cases hxv : x ∈ V
  · rw [if_pos (hm.2 hxv), if_pos hxv]; exact h x hx
  · rw [if_neg (fun h' => hxv (hm.1 h')), if_neg hxv]

theorem not_mem_fvKV {b : Name} {a : Name} :
    ∀ {ps : List (Name × Expr)}, (∀ q ∈ ps, q.1 ≠ a ∧ q.1 ≠ b ∧ b ∉ fvE q.2) → b ∉ fvKV ps
  | [], _ => by simp [fvKV]
  | (k, e) :: ps, h => by
    simp only [fvKV, List.mem_append, not_or]
    exact ⟨(h (k, e) List.mem_cons_self).2.2, not_mem_fvKV fun q hq => h q (List.mem_cons_of_mem _ hq)⟩

theorem not_mem_fvKV_dflts {a b : Name} {ps args : List (Name × Expr)}
    (h : ∀ q ∈ ps, q.1 ≠ a ∧ q.1 ≠ b ∧ b ∉ fvE q.2) : b ∉ fvKV (dflts ps args) :=
  fun hm => not_mem_fvKV h (fvKV_filter hm)

theorem dflts_keys {a b : Name} {ps args : List (Name × Expr)} (h : ∀ q ∈ ps, q.1 ≠ a ∧ q.1 ≠ b ∧ b ∉ fvE q.2)
    {n D σ ρ Δ l} (hd : evalA n D σ ρ (dflts ps args) = .ok (Δ, l)) : ∀ k ∈ keys Δ, k ≠ a ∧ k ≠ b := by
  intro k hk
  rw [evalA_keys _ hd] at hk
  obtain ⟨q, hq, rfl⟩ := List.mem_map.1 hk
  have := h q ((List.mem_filter.1 hq).1)
  exact ⟨this.1, this.2.1⟩

theorem args_keys {a b : Name} {ps args : List (Name × Expr)} (h : ∀ q ∈ ps, q.1 ≠ a ∧ q.1 ≠ b ∧ b ∉ fvE q.2)
    (hok : argsOK ps args = true) {n D σ ρ Δ l} (hg : evalA n D σ ρ args = .ok (Δ, l)) :
    ∀ k ∈ keys Δ, k ≠ a ∧ k ≠ b := by
  intro k hk
  rw [evalA_keys _ hg] at hk
  obtain ⟨q, hq, rfl⟩ := List.mem_map.1 hk
  have hs : (ps.lookup q.1).isSome = true := List.all_eq_true.1 hok q hq
  obtain ⟨v, hv⟩ := Option.isSome_iff_exists.1 hs
  have := h (q.1, v) (lookup_mem hv)
  exact ⟨this.1, this.2.1⟩

structure RenD (a b : Name) (n : Nat) : Prop where
  e : ∀ {D σ ρ ρ' x r}, DefsRn a b D → evalE n D σ ρ x = .ok r → RRel a b ρ ρ' → b ∉ fvE x →
    evalE n (rnDefs a b D) σ ρ' (rnE a b x) = .ok r
  one : ∀ {D σ ρ ρ' x r}, DefsRn a b D → execOne n D σ ρ x = .ok r → RRel a b ρ ρ' → NoBindS a b x →
    b ∉ fvS x → execOne n (rnDefs a b D) σ ρ' (rnS a b x) = .ok r
  s : ∀ {D σ ρ ρ' x r}, DefsRn a b D → execS n D σ ρ x = .ok r → RRel a b ρ ρ' → NoBindB a b x →
    b ∉ fvB x → execS n (rnDefs a b D) σ ρ' (rnB a b x) = .ok r
  ln : ∀ {D σ ρ ρ' i body ks r}, DefsRn a b D → loopN n D σ ρ i body ks = .ok r → RRel a b ρ ρ' →
    i ≠ a → i ≠ b → NoBindB a b body → b ∉ fvB body → loopN n (rnDefs a b D) σ ρ' i (rnB a b body) ks = .ok r
  lk : ∀ {D σ ρ ρ' i body kvs r}, DefsRn a b D → loopK n D σ ρ i body kvs = .ok r → RRel a b ρ ρ' →
    i ≠ a → i ≠ b → NoBindB a b body → b ∉ fvKV kvs → b ∉ fvB body →
    loopK n (rnDefs a b D) σ ρ' i (rnB a b body) (rnKV a b kvs) = .ok r
  as : ∀ {D σ ρ ρ' as r}, DefsRn a b D → evalA n D σ ρ as = .ok r → RRel a b ρ ρ' → b ∉ fvKV as →
    evalA n (rnDefs a b D) σ ρ' (rnKV a b as) = .ok r

theorem renD_nested (a b : Name) : ∀ n, RenD a b n := by
  intro n
  induction n with
  | zero => constructor <;> intros <;> simp_all [evalE, execOne, execS, loopN, loopK, evalA]
  | succ n ih =>
    constructor
    · intro D σ ρ ρ' x r hD h hr hb
      cases x with
      | lit s => simpa [evalE, rnE] using h
      | cst c => simpa [evalE, rnE] using h
      | var x =>
        have hx : x ≠ b := fun e => hb (by simp [fvE, e])
        simp only [evalE, rnE, bind_ok] at h ⊢
        obtain ⟨t, ht, h⟩ := h
        exact ⟨t, lookupNum_rn hr hx ht, h⟩
      | ref x rel =>
        have hx : x ≠ b := fun e => hb (by simp [fvE, e])
        simp only [evalE, rnE, bind_ok] at h ⊢
        obtain ⟨t, ht, h⟩ := h
        exact ⟨t, lookupRef_rn hr hx ht, h⟩
      | add u v | mul u v | eq u v | lt u v | bin o u v =>
        simp only [fvE, List.mem_append, not_or] at hb
        simp only [evalE, bind_ok] at h
        obtain ⟨⟨ta, la⟩, ha, ⟨tb, lb⟩, hb', h⟩ := h
        cases h
        simp [evalE, rnE, ih.e hD ha hr hb.1, ih.e hD hb' hr hb.2]
      | ite c u v =>
        simp only [fvE, List.mem_append, not_or] at hb
        simp only [evalE, bind_ok] at h
        obtain ⟨⟨tc, lc⟩, hc, ⟨t, l⟩, hu, h⟩ := h
        cases h
        have hc' := ih.e hD hc hr hb.1.1
        have hu' : evalE n (rnDefs a b D) σ ρ' (if tc.den σ ≠ 0 then rnE a b u else rnE a b v) = .ok (t, l) := by
          split
          · rename_i hne; rw [if_pos hne] at hu; exact ih.e hD hu hr hb.1.2
          · rename_i hne; rw [if_neg hne] at hu; exact ih.e hD hu hr hb.2
        simp only [evalE, rnE, hc', ok_bind, hu', obs_rnDefs]
      | sel es k =>
        simp only [evalE, bind_ok] at h
        obtain ⟨⟨tk, lk⟩, hk, i, hi, h⟩ := h
        simp only [fvE, List.mem_append, not_or] at hb
        split at h
        · rename_i e' he
          rw [bind_ok] at h
          obtain ⟨⟨t, l⟩, he', h⟩ := h
          cases h
          have hsub := fvE_sub_fvEs (List.mem_of_getElem? he)
          have he'' := ih.e hD he' hr (fun hm => hb.1 (hsub b hm))
          simp [evalE, rnE, ih.e hD hk hr hb.2, hi, rnEs_getElem?, he, he'']
        · cases h
      | call f u =>
        simp only [fvE] at hb
        simp only [evalE, bind_ok] at h
        obtain ⟨⟨ta, la⟩, ha, h⟩ := h
        split at h
        · rename_i x body hf
          rw [bind_ok] at h
          obtain ⟨⟨t, l⟩, hbd, h⟩ := h
          cases h
          obtain ⟨hx1, hx2, hxb⟩ := hD.1 (f, x, body) (lookup_mem hf)
          have hbd' := ih.e hD hbd (rrel_cons_keep hr x _ hx1 hx2) hxb
          simp [evalE, rnE, ih.e hD ha hr hb, fns_rnDefs, hf, hbd']
        · cases h
      | refIx x k rel =>
        simp only [fvE, List.mem_cons, not_or] at hb
        simp only [evalE, bind_ok] at h
        obtain ⟨⟨tk, lk⟩, hk, i, hi, t, ht, h⟩ := h
        cases h
        have ht' := lookupRef_rn hr (Ne.symm hb.1) ht
        simp [evalE, rnE, ih.e hD hk hr hb.2, hi, ht']
    · intro D σ ρ ρ' x r hD h hr hn hb
      cases x with
      | let_ x e | feat x e =>
        simp only [NoBindS] at hn
        simp only [fvS] at hb
        simp only [execOne, bind_ok] at h
        obtain ⟨⟨t, l⟩, he, h⟩ := h
        cases h
        simp [execOne, rnS, ih.e hD he hr hb, rn_ne hn.1]
      | set x e =>
        simp only [NoBindS] at hn
        simp only [fvS, List.mem_cons, not_or] at hb
        simp only [execOne, bind_ok] at h
        obtain ⟨u, hu, ⟨t, l⟩, he, h⟩ := h
        cases h
        have hu' := lookupNum_rn hr (Ne.symm hb.1) hu
        rw [rn_ne hn.1] at hu'
        simp [execOne, rnS, rn_ne hn.1, hu', ih.e hD he hr hb.2]
      | forN x i c body =>
        simp only [NoBindS] at hn
        simp only [fvS, List.mem_append, not_or] at hb
        simp only [execOne, bind_ok] at h
        obtain ⟨⟨tc, lc⟩, hc, k, hk, ⟨mm, l⟩, hl, h⟩ := h
        cases h
        simp [execOne, rnS, rn_ne hn.1, ih.e hD hc hr hb.1, hk,
          ih.ln hD hl hr hn.2.2.1 hn.2.2.2.1 hn.2.2.2.2 hb.2]
      | forK x i kvs body =>
        simp only [NoBindS] at hn
        simp only [fvS, List.mem_append, not_or] at hb
        simp only [execOne, bind_ok] at h
        obtain ⟨⟨mm, l⟩, hl, h⟩ := h
        cases h
        simp [execOne, rnS, rn_ne hn.1, ih.lk hD hl hr hn.2.2.1 hn.2.2.2.1 hn.2.2.2.2 hb.1 hb.2]
      | ifs c u v =>
        simp only [NoBindS] at hn
        simp only [fvS, List.mem_append, not_or] at hb
        simp only [execOne, bind_ok] at h
        obtain ⟨⟨tc, lc⟩, hc, ⟨ρ1, mm, l⟩, hbr, h⟩ := h
        cases h
        have hc' := ih.e hD hc hr hb.1.1
        have hb' : execS n (rnDefs a b D) σ ρ' (if tc.den σ ≠ 0 then rnB a b u else rnB a b v) = .ok (ρ1, mm, l) := by
          split
          · rename_i hne; rw [if_pos hne] at hbr; exact ih.s hD hbr hr hn.1 hb.1.2
          · rename_i hne; rw [if_neg hne] at hbr; exact ih.s hD hbr hr hn.2 hb.2
        simp only [execOne, rnS, hc', ok_bind, hb', obs_rnDefs]
      | use x p e =>
        simp only [NoBindS] at hn
        simp only [fvS] at hb
        simp only [execOne, bind_ok] at h
        obtain ⟨⟨ta, la⟩, ha, h⟩ := h
        split at h
        · rename_i y body hp
          rw [bind_ok] at h
          obtain ⟨⟨ρ1, mm, l⟩, hbd, h⟩ := h
          cases h
          obtain ⟨hy1, hy2, hyn, hyb⟩ := hD.2.1 (p, y, body) (lookup_mem hp)
          have hbd' := ih.s hD hbd (rrel_cons_keep hr y _ hy1 hy2) hyn hyb
          simp [execOne, rnS, rn_ne hn.1, ih.e hD ha hr hb, procs_rnDefs, hp, hbd']
        · cases h
      | occ x c args =>
        simp only [NoBindS] at hn
        simp only [fvS] at hb
        simp only [execOne] at h
        split at h
        · rename_i C hC
          split at h
          · rename_i hok
            simp only [bind_ok] at h
            obtain ⟨⟨Δg, lg⟩, hg, ⟨Δd, ld⟩, hd, ⟨ρ1, mm, l⟩, hbd, h⟩ := h
            cases h
            simp only [List.append_assoc] at hbd
            obtain ⟨hps, hbn, hbb, hvis⟩ := hD.2.2 (c, C) (lookup_mem hC)
            have hro := rrel_only hr hvis
            have hd' := ih.as hD hd hro (not_mem_fvKV_dflts hps)
            have hrb : RRel a b (Δg ++ (Δd ++ only C.vis ρ)) (Δg ++ (Δd ++ only (C.vis.map (rn a b)) ρ')) :=
              rrel_append_keep (rrel_append_keep hro Δd (dflts_keys hps hd)) Δg (args_keys hps hok hg)
            have hbd' := ih.s hD hbd hrb hbn hbb
            simp [execOne, rnS, rn_ne hn.1, comps_rnDefs, hC, rnComp, argsOK_rnKV2, hok, dflts_rnKV2,
              ih.as hD hg hr hb, hd', hbd']
          · cases h
        · cases h
    · intro D σ ρ ρ' x r hD h hr hn hb
      cases x with
      | nil => simpa [execS, rnB] using h
      | cons s ss =>
        simp only [NoBindB] at hn
        simp only [fvB, List.mem_append, not_or] at hb
        simp only [execS, bind_ok] at h
        obtain ⟨⟨Δ1, m1, l1⟩, h1, ⟨Δ2, m2, l2⟩, h2, h⟩ := h
        cases h
        have hk : ∀ k ∈ keys Δ1, k ≠ a ∧ k ≠ b := fun k hk' =>
          binder_noBind hn.1 (execOne_keys h1 k hk')
        have hr' : RRel a b (Δ1 ++ ρ) (Δ1 ++ ρ') := by
          have := rrel_append_ren hr Δ1 fun k hk' => (hk k hk').2
          rwa [rnEnv_keep Δ1 fun k hk' => (hk k hk').1] at this
        simp [execS, rnB, ih.one hD h1 hr hn.1 hb.1, ih.s hD h2 hr' hn.2 hb.2]
    · intro D σ ρ ρ' i body ks r hD h hr hia hib hn hb
      cases ks with
      | nil => simpa [loopN] using h
      | cons k ks =>
        simp only [loopN, bind_ok] at h
        obtain ⟨⟨ρ1, m1, l1⟩, h1, ⟨m2, l2⟩, h2, h⟩ := h
        cases h
        simp [loopN, ih.s hD h1 (rrel_cons_keep hr _ _ hia hib) hn hb,
          ih.ln hD h2 hr hia hib hn hb]
    · intro D σ ρ ρ' i body kvs r hD h hr hia hib hn hkv hb
      cases kvs with
      | nil => simpa [loopK, rnKV] using h
      | cons kv kvs =>
        obtain ⟨k, e⟩ := kv
        simp only [fvKV, List.mem_append, not_or] at hkv
        simp only [loopK, bind_ok] at h
        obtain ⟨⟨t, l0⟩, h0, ⟨ρ1, m1, l1⟩, h1, ⟨m2, l2⟩, h2, h⟩ := h
        cases h
        simp [loopK, rnKV, ih.e hD h0 hr hkv.1, ih.s hD h1 (rrel_cons_keep hr _ _ hia hib) hn hb,
          ih.lk hD h2 hr hia hib hn hkv.2 hb]
    · intro D σ ρ ρ' as r hD h hr hb
      cases as with
      | nil => simpa [evalA, rnKV] using h
      | cons a' as =>
        obtain ⟨y, e⟩ := a'
        simp only [fvKV, List.mem_append, not_or] at hb
        simp only [evalA, bind_ok] at h
        obtain ⟨⟨t, l⟩, h0, ⟨Δ, l'⟩, h1, h⟩ := h
        cases h
        simp [evalA, rnKV, ih.e hD h0 hr hb.1, ih.as hD h1 hr hb.2]


structure RenDTop (a b : Name) (n : Nat) : Prop where
  one : ∀ {D σ ρ ρ' x Δ m l}, DefsRn a b D → execOne n D σ ρ x = .ok (Δ, m, l) → RRel a b ρ ρ' →
    TopS a b x → b ∉ fvS x → execOne n (rnDefs a b D) σ ρ' (rnS a b x) = .ok (rnEnv a b Δ, rnModel a b m, l)
  s : ∀ {D σ ρ ρ' x Δ m l}, DefsRn a b D → execS n D σ ρ x = .ok (Δ, m, l) → RRel a b ρ ρ' →
    TopB a b x → b ∉ fvB x → execS n (rnDefs a b D) σ ρ' (rnB a b x) = .ok (rnEnv a b Δ, rnModel a b m, l)

theorem renD_top (a b : Name) : ∀ n, RenDTop a b n := by
  intro n
  induction n with
  | zero => constructor <;> intros <;> simp_all [execOne, execS]
  | succ n ih =>
    have N := renD_nested a b n
    constructor
    · intro D σ ρ ρ' x Δ m l hD h hr ht hb
      cases x with
      | let_ x e =>
        simp only [fvS] at hb
        simp only [execOne, bind_ok] at h
        obtain ⟨⟨t, l'⟩, he, h⟩ := h
        cases h
        simp [execOne, rnS, N.e hD he hr hb, rnEnv, rnModel]
      | feat x e =>
        simp only [fvS] at hb
        simp only [execOne, bind_ok] at h
        obtain ⟨⟨t, l'⟩, he, h⟩ := h
        cases h
        simp [execOne, rnS, N.e hD he hr hb, rnEnv, rnModel, renFirst]
      | set x e =>
        simp only [fvS, List.mem_cons, not_or] at hb
        simp only [execOne, bind_ok] at h
        obtain ⟨u, hu, ⟨t, l'⟩, he, h⟩ := h
        cases h
        simp [execOne, rnS, lookupNum_rn hr (Ne.symm hb.1) hu, N.e hD he hr hb.2, rnEnv, rnModel]
      | forN x i c body =>
        simp only [TopS] at ht
        simp only [fvS, List.mem_append, not_or] at hb
        simp only [execOne, bind_ok] at h
        obtain ⟨⟨tc, lc⟩, hc, k, hk, ⟨mm, l'⟩, hl, h⟩ := h
        cases h
        simp [execOne, rnS, N.e hD hc hr hb.1, hk, N.ln hD hl hr ht.2.1 ht.2.2.1 ht.2.2.2 hb.2,
          rnEnv, rnModel_pre]
      | forK x i kvs body =>
        simp only [TopS] at ht
        simp only [fvS, List.mem_append, not_or] at hb
        simp only [execOne, bind_ok] at h
        obtain ⟨⟨mm, l'⟩, hl, h⟩ := h
        cases h
        simp [execOne, rnS, N.lk hD hl hr ht.2.1 ht.2.2.1 ht.2.2.2 hb.1 hb.2, rnEnv, rnModel_pre]
      | ifs c u v =>
        simp only [TopS] at ht
        simp only [fvS, List.mem_append, not_or] at hb
        simp only [execOne, bind_ok] at h
        obtain ⟨⟨tc, lc⟩, hc, ⟨ρ1, mm, l'⟩, hbr, h⟩ := h
        cases h
        have hc' := N.e hD hc hr hb.1.1
        have hb' : execS n (rnDefs a b D) σ ρ' (if tc.den σ ≠ 0 then rnB a b u else rnB a b v) =
            .ok (rnEnv a b ρ1, rnModel a b mm, l') := by
          split
          · rename_i hne; rw [if_pos hne] at hbr; exact ih.s hD hbr hr ht.1 hb.1.2
          · rename_i hne; rw [if_neg hne] at hbr; exact ih.s hD hbr hr ht.2 hb.2
        simp only [execOne, rnS, hc', ok_bind, hb', obs_rnDefs]
        simp [rnEnv]
      | use x p e =>
        simp only [fvS] at hb
        simp only [execOne, bind_ok] at h
        obtain ⟨⟨ta, la⟩, ha, h⟩ := h
        split at h
        · rename_i y body hp
          rw [bind_ok] at h
          obtain ⟨⟨ρ1, mm, l'⟩, hbd, h⟩ := h
          cases h
          obtain ⟨hy1, hy2, hyn, hyb⟩ := hD.2.1 (p, y, body) (lookup_mem hp)
          have hbd' := N.s hD hbd (rrel_cons_keep hr y _ hy1 hy2) hyn hyb
          simp [execOne, rnS, N.e hD ha hr hb, procs_rnDefs, hp, hbd', rnEnv, rnModel_pre]
        · cases h
      | occ x c args =>
        simp only [fvS] at hb
        simp only [execOne] at h
        split at h
        · rename_i C hC
          split at h
          · rename_i hok
            simp only [bind_ok] at h
            obtain ⟨⟨Δg, lg⟩, hg, ⟨Δd, ld⟩, hd, ⟨ρ1, mm, l'⟩, hbd, h⟩ := h
            cases h
            simp only [List.append_assoc] at hbd
            obtain ⟨hps, hbn, hbb, hvis⟩ := hD.2.2 (c, C) (lookup_mem hC)
            have hro := rrel_only hr hvis
            have hd' := N.as hD hd hro (not_mem_fvKV_dflts hps)
            have hrb : RRel a b (Δg ++ (Δd ++ only C.vis ρ)) (Δg ++ (Δd ++ only (C.vis.map (rn a b)) ρ')) :=
              rrel_append_keep (rrel_append_keep hro Δd (dflts_keys hps hd)) Δg (args_keys hps hok hg)
            have hbd' := N.s hD hbd hrb hbn hbb
            simp [execOne, rnS, comps_rnDefs, hC, rnComp, argsOK_rnKV2, hok, dflts_rnKV2,
              N.as hD hg hr hb, hd', hbd', rnEnv, rnModel_pre]
          · cases h
        · cases h
    · intro D σ ρ ρ' x Δ m l hD h hr ht hb
      cases x with
      | nil =>
        simp only [execS, Except.ok.injEq, Prod.mk.injEq] at h
        obtain ⟨rfl, rfl, rfl⟩ := h
        simp [execS, rnB, rnEnv, rnModel]
      | cons s ss =>
        simp only [TopB] at ht
        simp only [fvB, List.mem_append, not_or] at hb
        simp only [execS, bind_ok] at h
        obtain ⟨⟨Δ1, m1, l1⟩, h1, ⟨Δ2, m2, l2⟩, h2, h⟩ := h
        cases h
        have hr' := rrel_append_ren hr Δ1 fun k hk => binder_top ht.1 (execOne_keys h1 k hk)
        simp only [execS, rnB, ih.one hD h1 hr ht.1 hb.1, ok_bind, ih.s hD h2 hr' ht.2 hb.2]
        simp [rnEnv, rnModel]


/-- Rename in a part document, its definitions renamed too: the same fuel, traces and log,
bindings renamed, every feature's id renamed in its first segment only. -/
theorem rename_defs {n : Nat} {D : Defs} {σ : Site → Int} {body : List Stmt} {Δ : Env} {m : Model}
    {l : Log} (a b : Name) (hD : DefsRn a b D) (ht : TopB a b body) (hb : b ∉ fvB body)
    (h : execS n D σ [] body = .ok (Δ, m, l)) :
    execS n (rnDefs a b D) σ [] (rnB a b body) = .ok (rnEnv a b Δ, rnModel a b m, l) :=
  (renD_top a b n).s hD h (fun _ _ => rfl) ht hb

end ForgeLang
