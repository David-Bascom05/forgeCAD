import ForgeExact.DecideOnce.QVertPlane

/-!
# Q1 L2: the vertex proposer for three quadrics, complete

Mirrors the curved Rust session's Milestone 1e (curved-l2-plan.md §4). Untrusted, proposed:
* `elimZ`: QPropose's elimination over `ℚ[x][y]` (coefficients `ZC`): `R₁ = Res_z(q_a, q_b) =
  U₁q_a + V₁q_b`, `R₂ = Res_z(q_a, q_c) = U₂q_a + V₂q_c`, and `b₂q_a − a₂q_b = N + D z`;
* `chain`: a pseudo-remainder sequence in `y`, tracking cofactors, ending at `R(x) = S R₁ + T R₂`
  and its last `y`-linear remainder `L = p₁ y + p₀`;
* a second projection on the quadrics sheared by the first `λ ≠ 0` of `shears` whose chain checks,
  and `G = gcd(R_A, R_B) = u R_A + v R_B`.

Checked: `bezout3` for both projections, `comb3` for `L`, `linOk` for `N + D z`, `gcdOk`, and every
sample's `onVert` and `dOk`.

**`proposeVert3_complete`**: every common point of the three quadrics is a proposed sample's point
(`vertPt = (x, y, z)`), from those checks alone, with `homSub_ev` (the substitution
`y = −p₀/p₁`, cleared by `p₁ᵐ`).
-/

namespace ForgeExact.DecideOnce.Curved

set_option linter.unusedSectionVars false

open ForgeExact ForgeExact.Ref ForgeExact.Alg ForgeExact.Quad ForgeExact.QRef Lean.Grind

/-! ## The untrusted proposer -/

def zcoef (q : Quadric) (k : Nat) : ZC := (restrT q).getD k default

def zsub (a b : ZC) : ZC := a + -b

structure ElimZ where
  R : ZC
  N : ZC
  D : ZC
  U : TPoly
  V : TPoly

/-- QPropose's closed-form elimination, over `ℚ[x][y]`. -/
def elimZ (qa qb : Quadric) : ElimZ :=
  let a0 := zcoef qa 0
  let a1 := zcoef qa 1
  let a2 := zcoef qa 2
  let b0 := zcoef qb 0
  let b1 := zcoef qb 1
  let b2 := zcoef qb 2
  let X := zsub (a2 * b0) (a0 * b2)
  let Y := zsub (a2 * b1) (a1 * b2)
  let Z := zsub (a1 * b0) (a0 * b1)
  let u0 := zsub (zsub (a0 * (b2 * b2)) (a1 * (b1 * b2))) (a2 * (b0 * b2)) + a2 * (b1 * b1)
  let u1 := -(a1 * (b2 * b2)) + a2 * (b1 * b2)
  let v0 := zsub (-(a0 * (a2 * b2)) + a1 * (a1 * b2)) (a1 * (a2 * b1)) + a2 * (a2 * b0)
  let v1 := zsub (a1 * (a2 * b2)) (a2 * (a2 * b1))
  { R := zsub (X * X) (Y * Z), N := zsub (b2 * a0) (a2 * b0), D := zsub (b2 * a1) (a2 * b1),
    U := [u0, u1], V := [v0, v1] }

def btrim (f : BPoly) : BPoly := (f.reverse.dropWhile fun c => Poly.isZero c.c).reverse
def bdeg (f : BPoly) : Nat := (btrim f).length - 1
def blc (f : BPoly) : YPoly := (btrim f).getLast?.getD 0
def bzeroQ (f : BPoly) : Bool := (btrim f).isEmpty

/-- The `ℚ[x]`-content of a list of bivariates, and the division by it. -/
def bcontent (fs : List BPoly) : Poly :=
  fs.foldl (fun g f => f.foldl (fun g c => if Poly.isZero c.c then g else Poly.gcd g c.c) g) []
def bdivc (c : Poly) (f : BPoly) : BPoly := f.map fun y => ⟨Poly.pdiv y.c c⟩

/-- Reduce `f` (with cofactors `fs`, `ft`) by `g`, until `deg f < deg g`. -/
def prem (g gs gt : BPoly) : Nat → BPoly → BPoly → BPoly → BPoly × BPoly × BPoly
  | 0, f, fs, ft => (f, fs, ft)
  | n + 1, f, fs, ft =>
    if bzeroQ f || bdeg f < bdeg g then (f, fs, ft)
    else
      let k := bdeg f - bdeg g
      let lg := blc g
      let lf := blc f
      let step := fun (a b : BPoly) => psub (pscale lg a) (pscale lf (pshiftUp k b))
      let f' := step f g
      let fs' := step fs gs
      let ft' := step ft gt
      let c := bcontent [f', fs', ft']
      let red := fun h => if Poly.deg c = 0 then h else bdivc c h
      prem g gs gt n (red f') (red fs') (red ft')

structure Chain where
  R : Poly
  S : BPoly
  T : BPoly
  L : BPoly
  LS : BPoly
  LT : BPoly

/-- The remainder sequence, to the last remainder constant in `y` and the last linear one. -/
def chain : Nat → (f fs ft g gs gt : BPoly) → Option (BPoly × BPoly × BPoly) → Option Chain
  | 0, _, _, _, _, _, _, _ => none
  | n + 1, f, fs, ft, g, gs, gt, lin =>
    if bzeroQ g then none
    else if bdeg g = 0 then
      let lin' := if bdeg f = 1 then some (f, fs, ft) else lin
      match lin' with
      | some (l, ls, lt) => some ⟨((btrim g).headD 0).c, gs, gt, btrim l, ls, lt⟩
      | none => none
    else
      let (r, rs, rt) := prem g gs gt 64 f fs ft
      let lin' := if bdeg g = 1 then some (g, gs, gt) else lin
      chain n g gs gt r rs rt lin'

def one : BPoly := [⟨[1]⟩]

/-- One projection: the two eliminations and the chain. -/
structure Proj where
  R : Poly
  A : TPoly
  B : TPoly
  C : TPoly
  L : BPoly
  AL : TPoly
  BL : TPoly
  CL : TPoly
  N : ZC
  D : ZC

def proj (qa qb qc : Quadric) : Option Proj := do
  let e1 := elimZ qa qb
  let e2 := elimZ qa qc
  let (f, fs, ft, g, gs, gt) :=
    if bdeg e1.R.b ≥ bdeg e2.R.b then (e1.R.b, one, [], e2.R.b, [], one)
    else (e2.R.b, [], one, e1.R.b, one, [])
  let ch ← chain 16 f fs ft g gs gt none
  let cof := fun (s t : BPoly) => (padd (pmul [⟨s⟩] e1.U) (pmul [⟨t⟩] e2.U), pmul [⟨s⟩] e1.V, pmul [⟨t⟩] e2.V)
  let (A, B, C) := cof ch.S ch.T
  let (AL, BL, CL) := cof ch.LS ch.LT
  some ⟨ch.R, A, B, C, ch.L, AL, BL, CL, e1.N, e1.D⟩

/-- `L` as a combination of the three quadrics, checked. -/
def comb3 (qa qb qc : Quadric) (A B C : TPoly) (L : BPoly) : Bool :=
  tzero (psub (padd (padd (pmul A (restrT qa)) (pmul B (restrT qb))) (pmul C (restrT qc))) [⟨L⟩])

/-- `b₂ q_a − a₂ q_b = N + D z`, checked. -/
def linOk (qa qb : Quadric) (N D : ZC) : Bool :=
  tzero (psub (padd (pmul [zcoef qb 2] (restrT qa)) (pmul [-(zcoef qa 2)] (restrT qb))) [N, D])

/-- `f(x, −p₀/p₁)·p₁^(len f − 1)`, a polynomial in `x` (Horner). -/
def homSub (p0 p1 : Poly) : BPoly → Poly
  | [] => []
  | c :: rest => padd (pmul c.c (ppow p1 rest.length)) (pmul (pneg p0) (homSub p0 p1 rest))

/-- `f` padded with zero coefficients to length `m`. -/
def bpad (m : Nat) (f : BPoly) : BPoly := f ++ List.replicate (m - f.length) 0

/-- The second projection: the first shear `λ ≠ 0` whose chain checks. -/
def tryShear (qa qb qc : Quadric) (l : Rat) : Option (Rat × Proj) :=
  match proj (qa.shear l) (qb.shear l) (qc.shear l) with
  | some p => if bezout3 (qa.shear l) (qb.shear l) (qc.shear l) p.A p.B p.C p.R then some (l, p) else none
  | none => none

def secondProj (qa qb qc : Quadric) : Option (Rat × Proj) :=
  (shears.filter (· ≠ 0)).findSome? (tryShear qa qb qc)

theorem secondProj_spec {qa qb qc : Quadric} {l : Rat} {p : Proj}
    (h : secondProj qa qb qc = some (l, p)) :
    bezout3 (qa.shear l) (qb.shear l) (qc.shear l) p.A p.B p.C p.R = true := by
  obtain ⟨l', _, hl'⟩ := List.exists_of_findSome?_eq_some h
  unfold tryShear at hl'
  split at hl'
  · split at hl'
    · rename_i hbz
      simp only [Option.some.injEq, Prod.mk.injEq] at hl'
      obtain ⟨rfl, rfl⟩ := hl'
      exact hbz
    · cases hl'
  · cases hl'

/-- The three-quadric vertices, each certified, or `none`. -/
def proposeVert3 (qa qb qc : Quadric) (fuel : Nat) : Option (List VSample) := do
  let pa ← proj qa qb qc
  if !(bezout3 qa qb qc pa.A pa.B pa.C pa.R && comb3 qa qb qc pa.AL pa.BL pa.CL pa.L &&
      linOk qa qb pa.N pa.D) then none
  let (_, pb) ← secondProj qa qb qc
  let (G, u, v) := Poly.xgcd pa.R pb.R
  if !gcdOk G u v pa.R pb.R then none
  let sp ← proposeSqfree G
  let ns ← rootsOf G fuel
  let p0 := (pa.L.getD 0 0).c
  let p1 := (pa.L.getD 1 0).c
  let m := max pa.N.b.length pa.D.b.length
  let Nz := homSub p0 p1 (bpad m pa.N.b)
  let Dz := homSub p0 p1 (bpad m pa.D.b)
  let mk := fun (n : Num) =>
    let Qof := fun (q : Quadric) => Poly.pdiv (homog3 q p0 p1 Nz Dz) sp
    (⟨sp, p0, p1, Nz, Dz, n, Qof qa, Qof qb, Qof qc⟩ : VSample)
  let vs := ns.map mk
  if (pa.L.length ≤ 2) && vs.all (fun v => v.onVert qa qb qc && v.dOk signFuel) then some vs else none

/-- The `x` of every common point of three quadrics, without a point per root (curved Rust's
finding, 058f1ad): two points over one `x` (a pair mirrored in `z`) have no `z` as a rational
function of `x`, but their `x` is still a root of the gcd of the two projections. Events need
only the `x`s. -/
def vertXs3 (qa qb qc : Quadric) (fuel : Nat) : Option (List Num) := do
  let pa ← proj qa qb qc
  if !bezout3 qa qb qc pa.A pa.B pa.C pa.R then none
  let (_, pb) ← secondProj qa qb qc
  let (G, u, v) := Poly.xgcd pa.R pb.R
  if !gcdOk G u v pa.R pb.R then none
  rootsOf G fuel

/-! ## The proofs -/

section Model

variable {K : Type} [Field K] [LE K] [LT K] [Std.IsLinearOrder K] [Std.LawfulOrderLT K]
  [OrderedRing K] (M : Model K)

theorem comb3_spec {qa qb qc : Quadric} {A B C : TPoly} {L : BPoly}
    (h : comb3 qa qb qc A B C L = true) {x y z : K}
    (ha : Quadric.evK M qa (x, y, z) = 0) (hb : Quadric.evK M qb (x, y, z) = 0)
    (hc : Quadric.evK M qc (x, y, z) = 0) : evalWith (yHom M x) L y = 0 := by
  have e := ev_tzero M _ h x y z
  rw [eval_psub, eval_padd, eval_padd, eval_pmul, eval_pmul, eval_pmul, restrT_ev, restrT_ev,
    restrT_ev, ha, hb, hc] at e
  simp only [evalWith_cons, evalWith_nil, zHom] at e
  grind

theorem linOk_spec {qa qb : Quadric} {N D : ZC} (h : linOk qa qb N D = true) {x y z : K}
    (ha : Quadric.evK M qa (x, y, z) = 0) (hb : Quadric.evK M qb (x, y, z) = 0) :
    (zHom M x y).f N + z * (zHom M x y).f D = 0 := by
  have e := ev_tzero M _ h x y z
  rw [eval_psub, eval_padd, eval_pmul, eval_pmul, restrT_ev, restrT_ev, ha, hb] at e
  simp only [evalWith_cons, evalWith_nil] at e
  grind

theorem yHom_f (x : K) (c : YPoly) : (yHom M x).f c = M.ev c.c x := rfl

theorem homSub_ev (p0 p1 : Poly) {x : K} (hp : M.ev p1 x ≠ 0) :
    ∀ f : BPoly, M.ev (homSub p0 p1 f) x =
      M.ev p1 x ^ (f.length - 1) * evalWith (yHom M x) f (-(M.ev p0 x) / M.ev p1 x)
  | [] => by simp only [homSub, Model.ev, evalWith_nil]; grind
  | c :: rest => by
    have ih := homSub_ev p0 p1 hp rest
    have hinv : M.ev p1 x * (M.ev p1 x)⁻¹ = 1 := Field.mul_inv_cancel hp
    simp only [homSub, Model.ev, eval_padd, eval_pmul, eval_ppow, eval_pneg, List.length_cons,
      Nat.add_sub_cancel, evalWith_cons, yHom_f] at ih ⊢
    rw [ih]
    simp only [Model.ev] at hinv hp
    cases rest with
    | nil => simp only [evalWith_nil, List.length_nil]; grind
    | cons d r =>
      generalize evalWith (yHom M x) (d :: r) (-evalWith M.ι p0 x / evalWith M.ι p1 x) = E
      simp only [List.length_cons, Nat.add_sub_cancel]
      rw [Semiring.pow_succ]
      grind

theorem lin_eval (L : BPoly) (h : L.length ≤ 2) (x y : K) :
    evalWith (yHom M x) L y = M.ev (L.getD 0 0).c x + y * M.ev (L.getD 1 0).c x := by
  have h0 : M.ev (0 : YPoly).c x = 0 := by show evalWith M.ι [] x = 0; rfl
  match L, h with
  | [], _ => simp only [evalWith_nil, List.getD_nil, h0]; grind
  | [c], _ => simp only [evalWith_cons, evalWith_nil, yHom_f, List.getD_cons_zero, List.getD_cons_succ,
      List.getD_nil, h0]
  | [c, d], _ => simp only [evalWith_cons, evalWith_nil, yHom_f, List.getD_cons_zero,
      List.getD_cons_succ]; grind
  | _ :: _ :: _ :: _, h => simp at h

theorem bpad_ev (m : Nat) (f : BPoly) (x y : K) :
    evalWith (yHom M x) (bpad m f) y = evalWith (yHom M x) f y := by
  unfold bpad
  generalize m - f.length = k
  induction k with
  | zero => simp
  | succ k ih =>
    rw [List.replicate_succ', ← List.append_assoc, eval_append_zero _ _ _ _ (by rfl)]
    exact ih

theorem bpad_length {m : Nat} {f : BPoly} (h : f.length ≤ m) : (bpad m f).length = m := by
  simp [bpad]; omega

/-- **Every vertex of three quadrics is a proposed sample's point.** -/
theorem vertXs3_complete {qa qb qc : Quadric} {fuel : Nat} {xs : List Num}
    (h : vertXs3 qa qb qc fuel = some xs) {x y z : K}
    (ha : Quadric.evK M qa (x, y, z) = 0) (hb : Quadric.evK M qb (x, y, z) = 0)
    (hc : Quadric.evK M qc (x, y, z) = 0) : ∃ n ∈ xs, Denotes M n x := by
  unfold vertXs3 at h
  simp only [Option.bind_eq_bind] at h
  cases hpa : proj qa qb qc with
  | none => simp [hpa] at h
  | some pa =>
    simp only [hpa, Option.bind_some] at h
    split at h
    · cases h
    rename_i hb3
    simp only [Bool.not_eq_true', Bool.not_eq_false] at hb3
    cases hpb : secondProj qa qb qc with
    | none => simp [hpb] at h
    | some lp =>
      obtain ⟨l, pb⟩ := lp
      simp only [hpb, Option.bind_some] at h
      have hbBz := secondProj_spec hpb
      split at h
      · cases h
      rename_i hg
      simp only [Bool.not_eq_true', Bool.not_eq_false] at hg
      have hRA := bezout3_spec M hb3 ha hb hc
      have hsh : ∀ q : Quadric, Quadric.evK M (q.shear l) (x, y - M.q l * z, z) =
          Quadric.evK M q (x, y, z) := by
        intro q
        rw [shear_ev]
        have e : y - M.q l * z + M.q l * z = y := by grind
        rw [e]
      have hRB := bezout3_spec M (x := x) (y := y - M.q l * z) (z := z) hbBz
        ((hsh qa).trans ha) ((hsh qb).trans hb) ((hsh qc).trans hc)
      exact rootsOf_complete M h (gcd_cover M hg hRA hRB)

theorem proposeVert3_complete {qa qb qc : Quadric} {fuel : Nat} {vs : List VSample}
    (h : proposeVert3 qa qb qc fuel = some vs) {x y z : K}
    (ha : Quadric.evK M qa (x, y, z) = 0) (hb : Quadric.evK M qb (x, y, z) = 0)
    (hc : Quadric.evK M qc (x, y, z) = 0) :
    ∃ v ∈ vs, Denotes M v.x x ∧ v.dOk signFuel = true ∧ v.onVert qa qb qc = true ∧
      vertPt M v.Ny v.Dy v.Nz v.Dz x = (x, y, z) := by
  unfold proposeVert3 at h
  simp only [Option.bind_eq_bind] at h
  cases hpa : proj qa qb qc with
  | none => simp [hpa] at h
  | some pa =>
    simp only [hpa, Option.bind_some] at h
    split at h
    · cases h
    · rename_i hck
      simp only [Bool.not_eq_true', Bool.not_eq_false, Bool.and_eq_true] at hck
      obtain ⟨⟨hb3, hc3⟩, hlin⟩ := hck
      cases hpb : secondProj qa qb qc with
      | none => simp [hpb] at h
      | some lp =>
        obtain ⟨l, pb⟩ := lp
        simp only [hpb, Option.bind_some] at h
        have hbBz := secondProj_spec hpb
        split at h
        · cases h
        · rename_i hg
          simp only [Bool.not_eq_true', Bool.not_eq_false] at hg
          cases hsp : proposeSqfree (Poly.xgcd pa.R pb.R).1 with
          | none => rw [hsp] at h; cases h
          | some sp =>
            simp only [hsp, Option.bind_some] at h
            cases hr : rootsOf (Poly.xgcd pa.R pb.R).1 fuel with
            | none => rw [hr] at h; cases h
            | some ns =>
              simp only [hr, Option.bind_some] at h
              split at h
              · rename_i hall
                simp only [Option.some.injEq] at h
                subst h
                simp only [Bool.and_eq_true, decide_eq_true_eq] at hall
                obtain ⟨hL2, hall⟩ := hall
                -- x is a root of both resultants, so of G
                have hRA := bezout3_spec M hb3 ha hb hc
                have hsh : ∀ q : Quadric, Quadric.evK M (q.shear l) (x, y - M.q l * z, z) =
                    Quadric.evK M q (x, y, z) := by
                  intro q
                  rw [shear_ev]
                  have e : y - M.q l * z + M.q l * z = y := by grind
                  rw [e]
                have hRB := bezout3_spec M (x := x) (y := y - M.q l * z) (z := z) hbBz
                  ((hsh qa).trans ha) ((hsh qb).trans hb) ((hsh qc).trans hc)
                have hG := gcd_cover M hg hRA hRB
                obtain ⟨n, hn, hd⟩ := rootsOf_complete M hr hG
                refine ⟨_, List.mem_map_of_mem hn, hd, ?_⟩
                have hok := List.all_eq_true.mp hall _ (List.mem_map_of_mem hn)
                simp only [Bool.and_eq_true] at hok
                refine ⟨hok.2, hok.1, ?_⟩
                obtain ⟨hp1, hDz⟩ := vdOk_ne M hd hok.2
                simp only at hp1 hDz ⊢
                -- y from the linear remainder
                have hL := comb3_spec M hc3 ha hb hc
                have hy : y = -(M.ev (pa.L.getD 0 0).c x) / M.ev (pa.L.getD 1 0).c x := by
                  have hLe := lin_eval M pa.L hL2 x y
                  rw [hLe] at hL
                  have hinv : M.ev (pa.L.getD 1 0).c x * (M.ev (pa.L.getD 1 0).c x)⁻¹ = 1 :=
                    Field.mul_inv_cancel hp1
                  grind
                -- z from N + D z
                have hlz := linOk_spec M hlin ha hb
                simp only [zHom] at hlz
                generalize hm : max pa.N.b.length pa.D.b.length = m at hDz ⊢
                have hNm : pa.N.b.length ≤ m := by rw [← hm]; exact Nat.le_max_left _ _
                have hDm : pa.D.b.length ≤ m := by rw [← hm]; exact Nat.le_max_right _ _
                rw [homSub_ev M _ _ hp1, bpad_length hDm] at hDz
                simp only [vertPt, Prod.mk.injEq, true_and]
                refine ⟨hy.symm, ?_⟩
                rw [homSub_ev M _ _ hp1, homSub_ev M _ _ hp1, bpad_length hNm, bpad_length hDm,
                  bpad_ev, bpad_ev]
                rw [bpad_ev] at hDz
                rw [← hy] at hDz ⊢
                have hD0 : evalWith (yHom M x) pa.D.b y ≠ 0 := by
                  intro h0; apply hDz; rw [h0]; grind
                have hinvD : evalWith (yHom M x) pa.D.b y * (evalWith (yHom M x) pa.D.b y)⁻¹ = 1 :=
                  Field.mul_inv_cancel hD0
                have hpow : M.ev (pa.L.getD 1 0).c x ^ (m - 1) ≠ 0 := by
                  intro h0; apply hDz; rw [h0]; grind
                have hinvP : M.ev (pa.L.getD 1 0).c x ^ (m - 1) *
                    (M.ev (pa.L.getD 1 0).c x ^ (m - 1))⁻¹ = 1 := Field.mul_inv_cancel hpow
                grind
              · cases h

end Model

-- two unit spheres and a z-cylinder (the Rust session's test): four vertices
def sphereB : Quadric := ⟨1, 1, 1, 0, 0, 0, -1, -2/3, -2/5, 1/4 + 1/9 + 1/25 - 1⟩
def cylB : Quadric := ⟨1, 1, 0, 0, 0, 0, -1/2, -1/4, 0, 1/16 + 1/64 - 9/16⟩
#guard (proposeVert3 sphere1 sphereB cylB 30).map List.length = some 4

end ForgeExact.DecideOnce.Curved
