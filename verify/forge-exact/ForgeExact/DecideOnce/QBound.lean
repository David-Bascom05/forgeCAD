import ForgeExact.DecideOnce.QLedger

/-!
# Q1 L2: the bounding sphere, certified

LemmaD's sweep lemmas need compactness, and a plane, cylinder or cone carrier gives none. The cure:
a rational bounding sphere `B : |p|² − R² = 0` is appended to the carriers, and every body is
certified to lie strictly inside it. Every cell the ledger reads then lies in the closed ball.

**The certificate** (`BCert`, checked by `certOk`): terms `τ_t`, each negative on the body, built
from the body's top-level conjuncts. A term is a literal (`q_j` where `q_j < 0` is a conjunct,
`−q_j` where `q_j > 0` is one), or `−τ_j τ_k` for two plane literals (a slab: `x > a` and `x < b`).
There are weights `w_t ≥ 0`, not all zero, and a sum of squares `Σ dᵢ ℓᵢ²` (`dᵢ ≥ 0`, `ℓᵢ` affine) with

    Σ w_t τ_t − (|p|² − R²) = Σ dᵢ ℓᵢ²        (coefficient by coefficient)

Then on the body `|p|² − R² ≤ Σ w_t τ_t < 0`.

* **`certOk_inside`**: a checked certificate puts every point of the body strictly inside `B`, in
  every model of the reals;
* `proposeCert`: an untrusted proposer (uniform weights, an `LDLᵀ` of the remainder). It covers
  boxes, balls, capped cylinders and their intersections.
-/

namespace ForgeExact.DecideOnce.Curved

set_option linter.unusedSectionVars false

open ForgeExact ForgeExact.Ref ForgeExact.Alg ForgeExact.Quad ForgeExact.QRef ForgeExact.DecideOnce
  Lean.Grind

deriving instance DecidableEq for SForm

def qadd (p q : Quadric) : Quadric :=
  ⟨p.xx + q.xx, p.yy + q.yy, p.zz + q.zz, p.xy + q.xy, p.xz + q.xz, p.yz + q.yz,
   p.lx + q.lx, p.ly + q.ly, p.lz + q.lz, p.c + q.c⟩
def qscale (k : Rat) (q : Quadric) : Quadric :=
  ⟨k * q.xx, k * q.yy, k * q.zz, k * q.xy, k * q.xz, k * q.yz, k * q.lx, k * q.ly, k * q.lz, k * q.c⟩
def qzero : Quadric := ⟨0, 0, 0, 0, 0, 0, 0, 0, 0, 0⟩
def qsumL (l : List Quadric) : Quadric := l.foldr qadd qzero
def qeqB (p q : Quadric) : Bool :=
  p.xx == q.xx && p.yy == q.yy && p.zz == q.zz && p.xy == q.xy && p.xz == q.xz && p.yz == q.yz &&
    p.lx == q.lx && p.ly == q.ly && p.lz == q.lz && p.c == q.c

/-- The bounding sphere `|p|² − R²` (`R2 = R²`). -/
def sphereR (R2 : Rat) : Quadric := ⟨1, 1, 1, 0, 0, 0, 0, 0, 0, -R2⟩

/-- The top-level conjuncts of a formula. -/
def conjs : SForm → List SForm
  | .and a b => conjs a ++ conjs b
  | F => [F]

inductive BTerm
  | one (j : Nat) (s : Sgn)
  | two (j k : Nat) (sj sk : Sgn)
  deriving DecidableEq, Repr

def BTerm.lits : BTerm → List SForm
  | .one j s => [.lit j s]
  | .two j k sj sk => [.lit j sj, .lit k sk]

/-- A literal's value, oriented to be negative where the literal holds. -/
def litQ (qs : List Quadric) (j : Nat) (s : Sgn) : Option Quadric :=
  match qs[j]?, s with
  | some q, .neg => some q
  | some q, .pos => some (qscale (-1) q)
  | _, _ => none

/-- A plane literal's affine form, oriented the same way. -/
def litAff (qs : List Quadric) (j : Nat) (s : Sgn) : Option (Rat × Rat × Rat × Rat) :=
  match qs[j]?, s with
  | some q, .neg => if isPlaneQ q then some (q.lx, q.ly, q.lz, q.c) else none
  | some q, .pos => if isPlaneQ q then some (-q.lx, -q.ly, -q.lz, -q.c) else none
  | _, _ => none

def BTerm.quad (qs : List Quadric) : BTerm → Option Quadric
  | .one j s => litQ qs j s
  | .two j k sj sk => do
    let f ← litAff qs j sj
    let g ← litAff qs k sk
    pure (qscale (-1) (affMul f g))

structure BCert where
  terms : List (Rat × BTerm)
  sos : List (Rat × (Rat × Rat × Rat × Rat))
  deriving Repr

/-- **The check.** -/
def certOk (qs : List Quadric) (F : SForm) (R2 : Rat) (c : BCert) : Bool :=
  c.terms.all (fun t => decide (0 ≤ t.1)) && c.terms.any (fun t => decide (0 < t.1)) &&
    c.sos.all (fun t => decide (0 ≤ t.1)) &&
    c.terms.all (fun t => t.2.lits.all fun G => decide (G ∈ conjs F)) &&
    match c.terms.mapM (fun t => (t.2.quad qs).map (qscale t.1)) with
    | none => false
    | some qts => qeqB (qsub (qsumL qts) (sphereR R2))
        (qsumL (c.sos.map fun t => qscale t.1 (affMul t.2 t.2)))

section Model

variable {K : Type} [Field K] [LE K] [LT K] [Std.IsLinearOrder K] [Std.LawfulOrderLT K]
  [OrderedRing K] (M : Model K) [DecidableRel (α := K) (· < ·)]

theorem qadd_ev (p q : Quadric) (x : PK K) :
    Quadric.evK M (qadd p q) x = Quadric.evK M p x + Quadric.evK M q x := by
  simp only [qadd, Quadric.evK, Model.q_add]; grind

theorem qscale_ev (k : Rat) (q : Quadric) (x : PK K) :
    Quadric.evK M (qscale k q) x = M.q k * Quadric.evK M q x := by
  simp only [qscale, Quadric.evK, Model.q_mul]; grind

theorem qsumL_ev (x : PK K) : ∀ l : List Quadric,
    Quadric.evK M (qsumL l) x = (l.map fun q => Quadric.evK M q x).foldr (· + ·) 0
  | [] => by simp [qsumL, qzero, Quadric.evK]; grind
  | q :: l => by
    simp only [qsumL, List.foldr_cons, List.map_cons] at *
    rw [qadd_ev, ← qsumL_ev x l]; rfl

theorem qeqB_eq {p q : Quadric} (h : qeqB p q = true) (x : PK K) :
    Quadric.evK M p x = Quadric.evK M q x := by
  simp only [qeqB, Bool.and_eq_true, beq_iff_eq] at h
  obtain ⟨⟨⟨⟨⟨⟨⟨⟨⟨h1, h2⟩, h3⟩, h4⟩, h5⟩, h6⟩, h7⟩, h8⟩, h9⟩, h10⟩ := h
  simp only [Quadric.evK, h1, h2, h3, h4, h5, h6, h7, h8, h9, h10]

theorem den_conjs {cs : List (Carrier (PK K) K)} {p : PK K} :
    ∀ F : SForm, F.den cs p → ∀ G ∈ conjs F, G.den cs p
  | .and a b, h, G, hG => by
    simp only [conjs, List.mem_append] at hG
    rcases hG with hG | hG
    · exact den_conjs a h.1 G hG
    · exact den_conjs b h.2 G hG
  | .lit _ _, h, G, hG => by simp only [conjs, List.mem_singleton] at hG; subst hG; exact h
  | .tt, h, G, hG => by simp only [conjs, List.mem_singleton] at hG; subst hG; exact h
  | .ff, h, G, hG => by simp only [conjs, List.mem_singleton] at hG; subst hG; exact h
  | .or _ _, h, G, hG => by simp only [conjs, List.mem_singleton] at hG; subst hG; exact h
  | .not _, h, G, hG => by simp only [conjs, List.mem_singleton] at hG; subst hG; exact h

theorem sgnK_neg {y : K} (h : sgnK y = .neg) : y < 0 := by
  unfold sgnK at h
  split at h
  · cases h
  · split at h
    · assumption
    · cases h

theorem sgnK_pos {y : K} (h : sgnK y = .pos) : 0 < y := by
  unfold sgnK at h
  split at h
  · assumption
  · split at h <;> cases h

/-- A literal that holds makes its oriented value negative. -/
theorem litQ_neg {qs : List Quadric} {j : Nat} {s : Sgn} {q : Quadric} (h : litQ qs j s = some q)
    {p : PK K} (hl : (SForm.lit j s).den (carriersOf M qs) p) : Quadric.evK M q p < 0 := by
  obtain ⟨c, hc, hs⟩ := hl
  simp only [carriersOf, List.getElem?_map] at hc
  unfold litQ at h
  cases hq : qs[j]? with
  | none => rw [hq] at hc; cases hc
  | some q0 =>
    rw [hq] at hc h
    simp only [Option.map_some, Option.some.injEq] at hc
    subst hc
    cases s with
    | neg => simp only [Option.some.injEq] at h; subst h; exact sgnK_neg hs
    | pos =>
      simp only [Option.some.injEq] at h; subst h
      rw [qscale_ev, Model.q_neg, Model.q_one]
      have := sgnK_pos hs
      grind
    | zero => cases h

theorem litAff_neg {qs : List Quadric} {j : Nat} {s : Sgn} {f : Rat × Rat × Rat × Rat}
    (h : litAff qs j s = some f) {p : PK K} (hl : (SForm.lit j s).den (carriersOf M qs) p) :
    affEv M f p < 0 := by
  obtain ⟨c, hc, hs⟩ := hl
  simp only [carriersOf, List.getElem?_map] at hc
  unfold litAff at h
  cases hq : qs[j]? with
  | none => rw [hq] at hc; cases hc
  | some q0 =>
    rw [hq] at hc h
    simp only [Option.map_some, Option.some.injEq] at hc
    subst hc
    obtain ⟨x, y, z⟩ := p
    cases s with
    | neg =>
      simp only at h
      split at h
      · rename_i hp
        simp only [Option.some.injEq] at h; subst h
        have := sgnK_neg hs
        rw [plane_ev M hp] at this
        simpa [affEv] using this
      · cases h
    | pos =>
      simp only at h
      split at h
      · rename_i hp
        simp only [Option.some.injEq] at h; subst h
        have := sgnK_pos hs
        rw [plane_ev M hp] at this
        simp only [affEv, Model.q_neg]
        grind
      · cases h
    | zero => cases h

theorem BTerm.quad_neg {qs : List Quadric} {t : BTerm} {q : Quadric} (h : t.quad qs = some q)
    {p : PK K} (hl : ∀ G ∈ t.lits, G.den (carriersOf M qs) p) : Quadric.evK M q p < 0 := by
  cases t with
  | one j s => exact litQ_neg M h (hl _ (by simp [BTerm.lits]))
  | two j k sj sk =>
    simp only [BTerm.quad, Option.bind_eq_bind, Option.bind_eq_some_iff, Option.pure_def,
      Option.some.injEq] at h
    obtain ⟨f, hf, g, hg, rfl⟩ := h
    have h1 := litAff_neg M hf (hl _ (by simp [BTerm.lits]))
    have h2 := litAff_neg M hg (hl _ (by simp [BTerm.lits]))
    rw [qscale_ev, affMul_ev, Model.q_neg, Model.q_one]
    have : 0 < affEv M f p * affEv M g p := by
      have := OrderedRing.mul_pos (a := -affEv M f p) (b := -affEv M g p) (by grind) (by grind)
      grind
    grind

theorem sum_neg {qs : List Quadric} (p : PK K) : ∀ (ts : List (Rat × BTerm)) (qts : List Quadric),
    ts.mapM (fun t => (t.2.quad qs).map (qscale t.1)) = some qts →
    (∀ t ∈ ts, 0 ≤ t.1) → (∀ t ∈ ts, ∀ G ∈ t.2.lits, G.den (carriersOf M qs) p) →
    (qts.map fun q => Quadric.evK M q p).foldr (· + ·) 0 ≤ 0 ∧
      ((∃ t ∈ ts, 0 < t.1) → (qts.map fun q => Quadric.evK M q p).foldr (· + ·) 0 < 0)
  | [], qts, h, _, _ => by
    simp at h; subst h; simp
  | t :: ts, qts, h, hw, hl => by
    simp only [List.mapM_cons, Option.bind_eq_bind, Option.bind_eq_some_iff, Option.pure_def,
      Option.some.injEq, Option.map_eq_some_iff] at h
    obtain ⟨r, ⟨q, hq, rfl⟩, rs, hrs, rfl⟩ := h
    have ih := sum_neg p ts rs hrs (fun u hu => hw u (List.mem_cons_of_mem _ hu))
      (fun u hu => hl u (List.mem_cons_of_mem _ hu))
    have hqn := BTerm.quad_neg M hq (hl t (List.mem_cons_self ..))
    have hw0 : 0 ≤ M.q t.1 := (Model.q_nonneg M).mpr (hw t (List.mem_cons_self ..))
    simp only [List.map_cons, List.foldr_cons, qscale_ev]
    have hle : M.q t.1 * Quadric.evK M q p ≤ 0 := by
      have := OrderedRing.mul_nonneg (a := M.q t.1) (b := -Quadric.evK M q p) hw0 (by grind)
      grind
    refine ⟨by grind, ?_⟩
    rintro ⟨u, hu, hu0⟩
    rcases List.mem_cons.mp hu with rfl | hu
    · have h0 : 0 < M.q u.1 := (Model.q_pos M).mpr hu0
      have := OrderedRing.mul_pos (a := M.q u.1) (b := -Quadric.evK M q p) h0 (by grind)
      grind
    · have := ih.2 ⟨u, hu, hu0⟩; grind

theorem sos_nonneg (p : PK K) : ∀ l : List (Rat × (Rat × Rat × Rat × Rat)), (∀ t ∈ l, 0 ≤ t.1) →
    0 ≤ ((l.map fun t => qscale t.1 (affMul t.2 t.2)).map fun q => Quadric.evK M q p).foldr
      (· + ·) 0
  | [], _ => by simp
  | t :: l, h => by
    simp only [List.map_cons, List.foldr_cons, qscale_ev, affMul_ev]
    have ih := sos_nonneg p l (fun u hu => h u (List.mem_cons_of_mem _ hu))
    have hd : 0 ≤ M.q t.1 := (Model.q_nonneg M).mpr (h t (List.mem_cons_self ..))
    have hs : 0 ≤ affEv M t.2 p * affEv M t.2 p := by
      rcases Std.le_total (a := 0) (b := affEv M t.2 p) with h0 | h0
      · exact OrderedRing.mul_nonneg h0 h0
      · have := OrderedRing.mul_nonneg (a := -affEv M t.2 p) (b := -affEv M t.2 p) (by grind) (by grind)
        grind
    have := OrderedRing.mul_nonneg hd hs
    grind

/-- **A checked certificate puts the body strictly inside the bounding sphere.** -/
theorem certOk_inside {qs : List Quadric} {F : SForm} {R2 : Rat} {c : BCert}
    (h : certOk qs F R2 c = true) {p : PK K} (hp : F.den (carriersOf M qs) p) :
    Quadric.evK M (sphereR R2) p < 0 := by
  unfold certOk at h
  simp only [Bool.and_eq_true, List.all_eq_true, List.any_eq_true, decide_eq_true_eq] at h
  obtain ⟨⟨⟨⟨hw, hpos⟩, hd⟩, hlits⟩, hid⟩ := h
  split at hid
  · cases hid
  rename_i qts hqts
  have hl : ∀ t ∈ c.terms, ∀ G ∈ t.2.lits, G.den (carriersOf M qs) p :=
    fun t ht G hG => den_conjs F hp G (hlits t ht G hG)
  have hs := sum_neg M p c.terms qts hqts hw hl
  have e := qeqB_eq M hid p
  rw [qsub_ev, qsumL_ev, qsumL_ev] at e
  have hn := sos_nonneg M p c.sos hd
  have := hs.2 hpos
  grind

end Model

/-! ## The proposer (untrusted) -/

/-- `LDLᵀ` of a symmetric 4×4 matrix (`x, y, z, 1`): the pivots and the unit lower columns. -/
def ldl (A : Array (Array Rat)) : Option (List (Rat × (Rat × Rat × Rat × Rat))) := Id.run do
  let n := 4
  let mut L : Array (Array Rat) := Array.replicate n (Array.replicate n 0)
  let mut D : Array Rat := Array.replicate n 0
  for i in [0:n] do
    let mut d := A[i]![i]!
    for k in [0:i] do d := d - L[i]![k]! * L[i]![k]! * D[k]!
    D := D.set! i d
    L := L.set! i (L[i]!.set! i 1)
    for j in [i+1:n] do
      let mut v := A[j]![i]!
      for k in [0:i] do v := v - L[j]![k]! * L[i]![k]! * D[k]!
      if d == 0 then
        if v != 0 then return none
        L := L.set! j (L[j]!.set! i 0)
      else
        L := L.set! j (L[j]!.set! i (v / d))
  if D.any (· < 0) then return none
  return some ((List.range n).map fun i =>
    (D[i]!, (L[0]![i]!, L[1]![i]!, L[2]![i]!, L[3]![i]!)))

def qmat (q : Quadric) : Array (Array Rat) :=
  #[#[q.xx, q.xy / 2, q.xz / 2, q.lx / 2], #[q.xy / 2, q.yy, q.yz / 2, q.ly / 2],
    #[q.xz / 2, q.yz / 2, q.zz, q.lz / 2], #[q.lx / 2, q.ly / 2, q.lz / 2, q.c]]

/-- The terms of a body: its non-plane literals, and its pairs of parallel plane literals. -/
def bodyTerms (qs : List Quadric) (F : SForm) : List BTerm :=
  let lits := (conjs F).filterMap fun G => match G with
    | .lit j s => if s == .zero then none else some (j, s)
    | _ => none
  let ones := (lits.filter fun (j, _) => match qs[j]? with
    | some q => !isPlaneQ q
    | none => false).map fun (j, s) => BTerm.one j s
  let pl := lits.filter fun (j, _) => match qs[j]? with
    | some q => isPlaneQ q
    | none => false
  let twos := (pl.zipIdx.flatMap fun ((j, sj), i) => (pl.drop (i + 1)).filterMap fun (k, sk) =>
    match qs[j]?, qs[k]? with
    | some a, some b => if ((nrm a).cross (nrm b)).isZero then some (BTerm.two j k sj sk) else none
    | _, _ => none)
  ones ++ twos

def proposeCert (qs : List Quadric) (F : SForm) (R2 : Rat) : Option BCert :=
  let ts := bodyTerms qs F
  [1, 2, 4, 8, 16, 32].findSome? fun (w : Rat) => do
    let terms := ts.map fun t => (w, t)
    let qts ← terms.mapM fun t => (t.2.quad qs).map (qscale t.1)
    let N := qsub (qsumL qts) (sphereR R2)
    let sos ← ldl (qmat N)
    let c : BCert := ⟨terms, sos⟩
    if certOk qs F R2 c then some c else none

-- the unit box is inside |p|² < 4; the unit sphere's ball inside |p|² < 2
#guard (proposeCert unitBox unitBoxF 4).isSome
#guard (proposeCert [sphere1] (.lit 0 .neg) 2).isSome
-- and the unit box is not certified inside |p|² < 1 (its corner (1,1,1) is outside)
#guard (proposeCert unitBox unitBoxF 1).isNone

-- a capped cylinder x² + y² < 1/4, −1 < z < 1 is inside |p|² < 4
#guard (proposeCert [⟨1, 1, 0, 0, 0, 0, 0, 0, 0, -1/4⟩, planeQ 0 0 1 1, planeQ 0 0 1 (-1)]
  (.and (.lit 0 .neg) (.and (.lit 1 .pos) (.lit 2 .neg))) 4).isSome

/-! ## The bounded ledger -/

theorem mapM_length {α β : Type} {f : α → Option β} :
    ∀ {l : List α} {ls : List β}, l.mapM f = some ls → ls.length = l.length
  | [], ls, h => by simp at h; subst h; rfl
  | x :: xs, ls, h => by
    simp only [List.mapM_cons, Option.bind_eq_bind, Option.bind_eq_some_iff, Option.pure_def,
      Option.some.injEq] at h
    obtain ⟨r, -, rs, hrs, rfl⟩ := h
    simp [mapM_length hrs]

/-- The bodies, each conjoined with "inside `B`" (carrier `nc`, the last). -/
def boundForms (nc : Nat) (forms : List SForm) : List SForm :=
  forms.map fun F => .and F (.lit nc .neg)

/-- **The bounded curved ledger**: the carriers with `B` appended, each body certified inside it
(the certificates proposed, then checked), the bodies conjoined with "inside `B`". -/
def qledgerB (qs : List Quadric) (forms : List SForm) (R2 : Rat) : Option Ledger := do
  let certs ← forms.mapM fun F => proposeCert qs F R2
  if !((forms.zip certs).all fun (F, c) => certOk qs F R2 c) then none
  qledger (qs ++ [sphereR R2]) (boundForms qs.length forms)

section Model

variable {K : Type} [Field K] [LE K] [LT K] [Std.IsLinearOrder K] [Std.LawfulOrderLT K]
  [OrderedRing K] (M : Model K) [DecidableRel (α := K) (· < ·)]

/-- **The bounded bodies are the bodies.** With `B` appended, a certified body's set is unchanged
by "inside `B`": the ledger's bodies are the input's. -/
theorem qledgerB_bodies {qs : List Quadric} {forms : List SForm} {R2 : Rat} {L : Ledger}
    (h : qledgerB qs forms R2 = some L) (hb : ∀ F ∈ forms, F.Below qs.length) (i : Nat)
    (hi : i < forms.length) (p : PK K) :
    bodiesOf M (qs ++ [sphereR R2]) (boundForms qs.length forms) i p ↔
      (formOf forms i).den (carriersOf M qs) p := by
  unfold qledgerB at h
  simp only [Option.bind_eq_bind, Option.bind_eq_some_iff] at h
  obtain ⟨certs, hc, h⟩ := h
  split at h
  · cases h
  rename_i hok
  simp only [Bool.not_eq_true', Bool.not_eq_false, List.all_eq_true] at hok
  have hlen : certs.length = forms.length := mapM_length hc
  have hF : formOf forms i = forms[i] := by simp [formOf, List.getD_eq_getElem?_getD, hi]
  have hFb : formOf (boundForms qs.length forms) i = .and forms[i] (.lit qs.length .neg) := by
    simp [formOf, boundForms, List.getD_eq_getElem?_getD, hi]
  have hcert : certOk qs forms[i] R2 certs[i] = true :=
    hok (forms[i], certs[i]) (List.mem_iff_getElem.mpr ⟨i, by simp [hlen, hi], by simp⟩)
  have hbelow := hb _ (List.getElem_mem hi)
  unfold bodiesOf
  rw [hFb, hF]
  simp only [SForm.den]
  have hcs : carriersOf M (qs ++ [sphereR R2]) = carriersOf M qs ++ [Quadric.evK M (sphereR R2)] := by
    simp [carriersOf]
  have hbelow' : forms[i].Below (carriersOf M qs).length := by simpa [carriersOf] using hbelow
  rw [hcs, SForm.den_append _ _ p _ hbelow']
  constructor
  · exact fun h => h.1
  · intro h
    refine ⟨h, ?_⟩
    refine ⟨Quadric.evK M (sphereR R2), ?_, ?_⟩
    · simp [carriersOf]
    · have := certOk_inside M hcert h
      unfold sgnK
      have hn : ¬ (0 < Quadric.evK M (sphereR R2) p) := fun h0 => Std.lt_irrefl (Std.lt_trans h0 this)
      simp [hn, this]

end Model

-- the unit box, bounded: still 6 faces, 12 edges, 8 vertices (B adds none round them)
#guard (qledgerB unitBox [unitBoxF] 4).map (fun L => (L.facets.length, L.edges.length,
  L.verts.length)) = some (6, 12, 8)

end ForgeExact.DecideOnce.Curved
