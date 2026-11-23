/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import IEEE754.Exact.DivRem2
import IEEE754.Exact.QHenrici

/-!
# dol-curved-rust `Q` (forge-alg `lib.rs`, 1afff77) on `Big` fields, tied to `QZ`

`QB` is `Q` with its `Big` fields; `toZ` reads them as integers. Each operation is written with
the `Big` operations the Rust calls (`add`, `mul`, `neg`, `sub`, `signum`, `is_zero`, `divrem`, and
`bcmp(a, b) = signum(a − b)`), and `gcd` is any `G` with `G`'s value `gcd(|a|, |b|)`
(`Exact/Gcd.lean`'s `gcdBig_spec`). Each `_tie` theorem says: on canonical fields, the result's fields
are canonical and read as `QZ`'s operation on the read inputs, so `QZ`'s `_spec`s (`Normal`, the value)
carry over.
-/

namespace IEEE754.Exact

open Big

namespace QB

/-- `Q` with `Big` fields. -/
structure QB where
  n : Big
  d : Big

/-- The fields read as integers. -/
abbrev toZ (q : QB) : QZ.QZ := ⟨val q.n, val q.d⟩

/-- Both fields canonical. -/
def Can (q : QB) : Prop := Canon q.n ∧ Canon q.d

/-- `Big::from_i64(1)`. -/
def one : Big := S 1

/-- `Big::abs`, at its value. -/
def babs (x : Big) : Big := if signum x < 0 then neg x else x

/-- `bcmp(a, b) = (a − b).signum()`. -/
def bcmp (a b : Big) : ℤ := signum (sub a b)

/-- `Q::zero`. -/
def zero : QB := ⟨S 0, one⟩

/-- A `gcd` on `Big`: canonical, of value `gcd(|a|, |b|)`. -/
def GcdOk (G : Big → Big → Big) : Prop :=
  ∀ a b, Canon a → Canon b → Canon (G a b) ∧ val (G a b) = (Int.gcd (val a) (val b) : ℤ)

/-! ## The `Big` facts used -/

lemma canon_one : Canon one := by simp [Canon, one]

lemma val_one : val one = 1 := rfl

lemma isz {x : Big} (hx : Canon x) : isZero x = true ↔ val x = 0 := isZero_spec hx

lemma sneg {x : Big} (hx : Canon x) : signum x < 0 ↔ val x < 0 := by
  rw [signum_spec hx]; exact Int.sign_neg_iff

lemma isone {x : Big} (hx : Canon x) : bcmp x one = 0 ↔ val x = 1 := by
  unfold bcmp
  rw [signum_spec (sub_spec hx canon_one).1, (sub_spec hx canon_one).2, val_one, Int.sign_eq_zero_iff_zero]
  omega

lemma babs_spec {x : Big} (hx : Canon x) : Canon (babs x) ∧ val (babs x) = |val x| := by
  unfold babs
  split_ifs with h
  · rw [sneg hx] at h
    exact ⟨(neg_spec hx).1, by rw [(neg_spec hx).2, abs_of_neg h]⟩
  · rw [sneg hx] at h
    exact ⟨hx, by rw [abs_of_nonneg (not_lt.1 h)]⟩

/-- `TruncDiv` fixes the quotient. -/
lemma truncDiv_q {x y q r : ℤ} (hy : y ≠ 0) (h : Big.TruncDiv x y q r) : q = Int.tdiv x y := by
  obtain ⟨e1, l1, p1, n1⟩ := h
  obtain ⟨e2, l2, p2, n2⟩ := Big.truncDiv_int (a := x) hy
  set q' := Int.tdiv x y
  set r' := Int.tmod x y
  have hr : |r - r'| < |y| := by
    rcases le_total 0 x with hx | hx
    · have a1 := p1 hx; have a2 := p2 hx
      rw [abs_of_nonneg a1] at l1; rw [abs_of_nonneg a2] at l2
      exact abs_sub_lt_iff.2 ⟨by linarith, by linarith⟩
    · have a1 := n1 hx; have a2 := n2 hx
      rw [abs_of_nonpos a1] at l1; rw [abs_of_nonpos a2] at l2
      exact abs_sub_lt_iff.2 ⟨by linarith, by linarith⟩
  have hd : (q - q') * y = r' - r := by linarith
  have : |q - q'| * |y| < 1 * |y| := by rw [← abs_mul, hd, abs_sub_comm, one_mul]; exact hr
  have := lt_of_mul_lt_mul_right this (abs_nonneg y)
  have : q - q' = 0 := by rw [abs_lt] at this; omega
  linarith

/-- `divrem(x, y).0` is `Int.tdiv` (`0` for `y = 0`, as `Int.tdiv _ 0`). -/
lemma divq_spec {x y : Big} (hx : Canon x) (hy : Canon y) :
    Canon (divrem x y).1 ∧ val (divrem x y).1 = Int.tdiv (val x) (val y) := by
  obtain ⟨c1, -, h, h0⟩ := divrem_spec hx hy
  refine ⟨c1, ?_⟩
  by_cases hy0 : val y = 0
  · rw [(h0 hy0).1, hy0, Int.tdiv_zero]
  · exact truncDiv_q hy0 (h hy0)

/-! ## `Q::ratio` -/

/-- `Q::ratio(n, d)`. -/
def ratio (G : Big → Big → Big) (n d : Big) : QB :=
  if isZero d || isZero n then zero else
    let n1 := if signum d < 0 then neg n else n
    let d1 := if signum d < 0 then neg d else d
    if bcmp d1 one = 0 then ⟨n1, d1⟩ else
      let gg := G n1 d1
      if bcmp gg one = 0 then ⟨n1, d1⟩ else
        ⟨if signum n1 < 0 then neg (divrem (babs n1) gg).1 else (divrem n1 gg).1, (divrem d1 gg).1⟩

lemma zero_tie : Can zero ∧ toZ zero = QZ.zero := ⟨⟨canon_zero, canon_one⟩, rfl⟩

lemma ite_canon {c : Prop} [Decidable c] {x y : Big} (hx : Canon x) (hy : Canon y) : Canon (if c then x else y) := by
  split_ifs <;> assumption

theorem ratio_tie {G : Big → Big → Big} (hG : GcdOk G) {n d : Big} (hn : Canon n) (hd : Canon d) :
    Can (ratio G n d) ∧ toZ (ratio G n d) = QZ.ratio (val n) (val d) := by
  have h0 : (isZero d || isZero n) = true ↔ val d = 0 ∨ val n = 0 := by
    rw [Bool.or_eq_true, isz hd, isz hn]
  unfold ratio QZ.ratio
  by_cases hz : val d = 0 ∨ val n = 0
  · rw [if_pos (h0.2 hz), if_pos hz]; exact zero_tie
  rw [if_neg (fun h => hz (h0.1 h)), if_neg hz]
  -- the signs fixed
  have cN : Canon (if signum d < 0 then neg n else n) := ite_canon (neg_spec hn).1 hn
  have cD : Canon (if signum d < 0 then neg d else d) := ite_canon (neg_spec hd).1 hd
  have vN : val (if signum d < 0 then neg n else n) = if val d < 0 then -val n else val n := by
    by_cases h : val d < 0
    · rw [if_pos ((sneg hd).2 h), if_pos h, (neg_spec hn).2]
    · rw [if_neg (fun h' => h ((sneg hd).1 h')), if_neg h]
  have vD : val (if signum d < 0 then neg d else d) = if val d < 0 then -val d else val d := by
    by_cases h : val d < 0
    · rw [if_pos ((sneg hd).2 h), if_pos h, (neg_spec hd).2]
    · rw [if_neg (fun h' => h ((sneg hd).1 h')), if_neg h]
  dsimp only
  generalize (if signum d < 0 then neg n else n) = N at cN vN ⊢
  generalize (if signum d < 0 then neg d else d) = D at cD vD ⊢
  rw [← vN, ← vD]
  obtain ⟨cg, vg⟩ := hG N D cN cD
  have eg : QZ.g (val N) (val D) = val (G N D) := by rw [vg]; rfl
  rw [eg]
  by_cases h1 : val D = 1
  · rw [if_pos ((isone cD).2 h1), if_pos h1]; exact ⟨⟨cN, cD⟩, rfl⟩
  rw [if_neg (fun h => h1 ((isone cD).1 h)), if_neg h1]
  by_cases h2 : val (G N D) = 1
  · rw [if_pos ((isone cg).2 h2), if_pos h2]; exact ⟨⟨cN, cD⟩, rfl⟩
  rw [if_neg (fun h => h2 ((isone cg).1 h)), if_neg h2]
  obtain ⟨cqd, vqd⟩ := divq_spec cD cg
  obtain ⟨cb, vb⟩ := babs_spec cN
  obtain ⟨cqa, vqa⟩ := divq_spec cb cg
  obtain ⟨cqn, vqn⟩ := divq_spec cN cg
  by_cases h3 : val N < 0
  · rw [if_pos ((sneg cN).2 h3), if_pos h3]
    refine ⟨⟨(neg_spec cqa).1, cqd⟩, ?_⟩
    simp only [toZ, (neg_spec cqa).2, vqa, vb, vqd]
  · rw [if_neg (fun h => h3 ((sneg cN).1 h)), if_neg h3]
    exact ⟨⟨cqn, cqd⟩, by simp only [toZ, vqn, vqd]⟩

/-! ## `Q::add`, `neg`, `sub`, `mul`, `div`, `inv` -/

/-- `Q::add`, Henrici's sum. -/
def qadd (G : Big → Big → Big) (x y : QB) : QB :=
  if isZero x.n then y else if isZero y.n then x else
    let gg := G x.d y.d
    if bcmp gg one = 0 then ⟨Big.add (Big.mul x.n y.d) (Big.mul y.n x.d), Big.mul x.d y.d⟩ else
      let d1g := (divrem x.d gg).1
      let d2g := (divrem y.d gg).1
      let t := Big.add (Big.mul x.n d2g) (Big.mul y.n d1g)
      if isZero t then zero else
        let h := G t gg
        if bcmp h one = 0 then ⟨t, Big.mul d1g y.d⟩ else ⟨(divrem t h).1, Big.mul d1g (divrem y.d h).1⟩

/-- `add` and `mul` of two canonical `Big`s: canonical, at their values. -/
lemma am {a b : Big} (ha : Canon a) (hb : Canon b) :
    (Canon (Big.add a b) ∧ val (Big.add a b) = val a + val b) ∧
      (Canon (Big.mul a b) ∧ val (Big.mul a b) = val a * val b) := ⟨add_spec ha hb, mul_spec ha hb⟩

theorem add_tie {G : Big → Big → Big} (hG : GcdOk G) {x y : QB} (hx : Can x) (hy : Can y) :
    Can (qadd G x y) ∧ toZ (qadd G x y) = QZ.add (toZ x) (toZ y) := by
  obtain ⟨xn, xd⟩ := hx
  obtain ⟨yn, yd⟩ := hy
  unfold qadd QZ.add
  by_cases hx0 : val x.n = 0
  · rw [if_pos ((isz xn).2 hx0), if_pos hx0]; exact ⟨⟨yn, yd⟩, rfl⟩
  rw [if_neg (fun h => hx0 ((isz xn).1 h)), if_neg hx0]
  by_cases hy0 : val y.n = 0
  · rw [if_pos ((isz yn).2 hy0), if_pos hy0]; exact ⟨⟨xn, xd⟩, rfl⟩
  rw [if_neg (fun h => hy0 ((isz yn).1 h)), if_neg hy0]
  dsimp only
  obtain ⟨cg, vg⟩ := hG x.d y.d xd yd
  have eg : QZ.g (val x.d) (val y.d) = val (G x.d y.d) := by rw [vg]; rfl
  rw [eg]
  generalize G x.d y.d = gg at cg vg ⊢
  by_cases h1 : val gg = 1
  · rw [if_pos ((isone cg).2 h1), if_pos h1]
    obtain ⟨⟨c1, v1⟩, ⟨c2, v2⟩⟩ := am xn yd
    obtain ⟨-, ⟨c3, v3⟩⟩ := am yn xd
    obtain ⟨⟨c4, v4⟩, -⟩ := am c2 c3
    obtain ⟨-, ⟨c5, v5⟩⟩ := am xd yd
    exact ⟨⟨c4, c5⟩, by simp only [QZ.QZ.mk.injEq]; rw [v4, v2, v3, v5]; exact ⟨rfl, rfl⟩⟩
  rw [if_neg (fun h => h1 ((isone cg).1 h)), if_neg h1]
  obtain ⟨c1, v1⟩ := divq_spec xd cg
  obtain ⟨c2, v2⟩ := divq_spec yd cg
  rw [← v1, ← v2]
  generalize (divrem x.d gg).1 = d1g at c1 v1 ⊢
  generalize (divrem y.d gg).1 = d2g at c2 v2 ⊢
  obtain ⟨-, ⟨m1, w1⟩⟩ := am xn c2
  obtain ⟨-, ⟨m2, w2⟩⟩ := am yn c1
  obtain ⟨⟨ct, vt⟩, -⟩ := am m1 m2
  rw [w1, w2] at vt
  rw [← vt]
  generalize Big.add (Big.mul x.n d2g) (Big.mul y.n d1g) = t at ct vt ⊢
  by_cases ht : val t = 0
  · rw [if_pos ((isz ct).2 ht), if_pos ht]; exact zero_tie
  rw [if_neg (fun h => ht ((isz ct).1 h)), if_neg ht]
  obtain ⟨ch, vh⟩ := hG t gg ct cg
  have eh : QZ.g (val t) (val gg) = val (G t gg) := by rw [vh]; rfl
  rw [eh]
  generalize G t gg = h at ch vh ⊢
  by_cases h2 : val h = 1
  · rw [if_pos ((isone ch).2 h2), if_pos h2]
    obtain ⟨-, ⟨c3, v3⟩⟩ := am c1 yd
    exact ⟨⟨ct, c3⟩, by simp [v3]⟩
  rw [if_neg (fun h' => h2 ((isone ch).1 h')), if_neg h2]
  obtain ⟨c3, v3⟩ := divq_spec ct ch
  obtain ⟨c4, v4⟩ := divq_spec yd ch
  obtain ⟨-, ⟨c5, v5⟩⟩ := am c1 c4
  exact ⟨⟨c3, c5⟩, by simp only [QZ.QZ.mk.injEq]; exact ⟨v3, by rw [v5, v4]⟩⟩

/-- `Q::neg`. -/
def qneg (x : QB) : QB := ⟨Big.neg x.n, x.d⟩

theorem neg_tie {x : QB} (hx : Can x) : Can (qneg x) ∧ toZ (qneg x) = QZ.neg (toZ x) :=
  ⟨⟨(neg_spec hx.1).1, hx.2⟩, by simp only [qneg, toZ, QZ.neg, (neg_spec hx.1).2]⟩

/-- `Q::sub`: `self.add(&o.neg())`. -/
def qsub (G : Big → Big → Big) (x y : QB) : QB := qadd G x (qneg y)

theorem sub_tie {G : Big → Big → Big} (hG : GcdOk G) {x y : QB} (hx : Can x) (hy : Can y) :
    Can (qsub G x y) ∧ toZ (qsub G x y) = QZ.add (toZ x) (QZ.neg (toZ y)) := by
  obtain ⟨c, e⟩ := neg_tie hy
  rw [qsub, ← e]; exact add_tie hG hx c

/-- `Q::mul`, Henrici's product. -/
def qmul (G : Big → Big → Big) (x y : QB) : QB :=
  if isZero x.n || isZero y.n then zero else
    let g1 := G x.n y.d
    let g2 := G y.n x.d
    let p1 : Big × Big := if bcmp g1 one = 0 then (x.n, y.d) else ((divrem x.n g1).1, (divrem y.d g1).1)
    let p2 : Big × Big := if bcmp g2 one = 0 then (y.n, x.d) else ((divrem y.n g2).1, (divrem x.d g2).1)
    ⟨Big.mul p1.1 p2.1, Big.mul p2.2 p1.2⟩

/-- One of `mul`'s reductions: `(a, b)` kept when `g = 1`, else both divided by `g`. -/
lemma red_tie {a b g : Big} (ha : Canon a) (hb : Canon b) (hg : Canon g) :
    let p : Big × Big := if bcmp g one = 0 then (a, b) else ((divrem a g).1, (divrem b g).1)
    Canon p.1 ∧ Canon p.2 ∧
      ((val p.1, val p.2) : ℤ × ℤ) =
        (if val g = 1 then (val a, val b) else (Int.tdiv (val a) (val g), Int.tdiv (val b) (val g))) := by
  intro p
  by_cases h : val g = 1
  · simp only [p, if_pos ((isone hg).2 h), if_pos h]; exact ⟨ha, hb, trivial⟩
  · simp only [p, if_neg (fun h' => h ((isone hg).1 h')), if_neg h]
    exact ⟨(divq_spec ha hg).1, (divq_spec hb hg).1, by rw [(divq_spec ha hg).2, (divq_spec hb hg).2]⟩

theorem mul_tie {G : Big → Big → Big} (hG : GcdOk G) {x y : QB} (hx : Can x) (hy : Can y) :
    Can (qmul G x y) ∧ toZ (qmul G x y) = QZ.mul (toZ x) (toZ y) := by
  obtain ⟨xn, xd⟩ := hx
  obtain ⟨yn, yd⟩ := hy
  unfold qmul QZ.mul
  have h0 : (isZero x.n || isZero y.n) = true ↔ val x.n = 0 ∨ val y.n = 0 := by
    rw [Bool.or_eq_true, isz xn, isz yn]
  by_cases hz : val x.n = 0 ∨ val y.n = 0
  · rw [if_pos (h0.2 hz), if_pos hz]; exact zero_tie
  rw [if_neg (fun h => hz (h0.1 h)), if_neg hz]
  obtain ⟨cg1, vg1⟩ := hG x.n y.d xn yd
  obtain ⟨cg2, vg2⟩ := hG y.n x.d yn xd
  have e1 : QZ.g (val x.n) (val y.d) = val (G x.n y.d) := by rw [vg1]; rfl
  have e2 : QZ.g (val y.n) (val x.d) = val (G y.n x.d) := by rw [vg2]; rfl
  rw [e1, e2]
  obtain ⟨a1, a2, r1⟩ := red_tie xn yd cg1
  obtain ⟨b1, b2, r2⟩ := red_tie yn xd cg2
  obtain ⟨-, ⟨c1, v1⟩⟩ := am a1 b1
  obtain ⟨-, ⟨c2, v2⟩⟩ := am b2 a2
  have r1a := congrArg Prod.fst r1
  have r1b := congrArg Prod.snd r1
  have r2a := congrArg Prod.fst r2
  have r2b := congrArg Prod.snd r2
  dsimp only at r1a r1b r2a r2b
  refine ⟨⟨c1, c2⟩, ?_⟩
  dsimp only
  rw [QZ.QZ.mk.injEq, v1, v2, r1a, r1b, r2a, r2b]
  exact ⟨rfl, rfl⟩

/-- `Q::div`: `0` for `o = 0`, else `ratio(n₁d₂, d₁n₂)`. -/
def qdiv (G : Big → Big → Big) (x y : QB) : QB :=
  if isZero y.n then zero else ratio G (Big.mul x.n y.d) (Big.mul x.d y.n)

theorem div_tie {G : Big → Big → Big} (hG : GcdOk G) {x y : QB} (hx : Can x) (hy : Can y) :
    Can (qdiv G x y) ∧
      toZ (qdiv G x y) = if (toZ y).n = 0 then QZ.zero else QZ.ratio ((toZ x).n * (toZ y).d) ((toZ x).d * (toZ y).n) := by
  unfold qdiv
  by_cases h : val y.n = 0
  · rw [if_pos ((isz hy.1).2 h), if_pos h]; exact zero_tie
  rw [if_neg (fun h' => h ((isz hy.1).1 h')), if_neg h]
  obtain ⟨-, ⟨c1, v1⟩⟩ := am hx.1 hy.2
  obtain ⟨-, ⟨c2, v2⟩⟩ := am hx.2 hy.1
  rw [← v1, ← v2]; exact ratio_tie hG c1 c2

/-- `Q::inv`: `ratio(d, n)`. -/
def qinv (G : Big → Big → Big) (x : QB) : QB := ratio G x.d x.n

theorem inv_tie {G : Big → Big → Big} (hG : GcdOk G) {x : QB} (hx : Can x) :
    Can (qinv G x) ∧ toZ (qinv G x) = QZ.ratio (toZ x).d (toZ x).n := ratio_tie hG hx.2 hx.1

/-- `Q::cmp_q`: `bcmp(n₁, n₂)` on equal denominators, else `bcmp(n₁d₂, n₂d₁)`. -/
def qcmp (x y : QB) : ℤ :=
  if bcmp x.d y.d = 0 then bcmp x.n y.n else bcmp (Big.mul x.n y.d) (Big.mul y.n x.d)

lemma bcmp_spec {a b : Big} (ha : Canon a) (hb : Canon b) : bcmp a b = Int.sign (val a - val b) := by
  unfold bcmp; rw [signum_spec (sub_spec ha hb).1, (sub_spec ha hb).2]

/-- **`Q::cmp_q`** is the sign of `x − y` for normal `x, y`. -/
theorem cmp_tie {x y : QB} (hx : Can x) (hy : Can y) (nx : QZ.Normal (toZ x)) (ny : QZ.Normal (toZ y)) :
    qcmp x y = Int.sign (val x.n * val y.d - val y.n * val x.d) := by
  obtain ⟨xn, xd⟩ := hx
  obtain ⟨yn, yd⟩ := hy
  unfold qcmp
  rw [bcmp_spec xd yd]
  by_cases h : val x.d = val y.d
  · rw [if_pos (by rw [h, sub_self, Int.sign_zero]), bcmp_spec xn yn, h, ← sub_mul]
    have : 0 < val y.d := by rw [← h]; exact nx.1
    rw [Int.sign_mul, Int.sign_eq_one_of_pos this, mul_one]
  · rw [if_neg (by rw [Int.sign_eq_zero_iff_zero]; omega)]
    obtain ⟨-, ⟨c1, v1⟩⟩ := am xn yd
    obtain ⟨-, ⟨c2, v2⟩⟩ := am yn xd
    rw [bcmp_spec c1 c2, v1, v2]

/-! ## So, on `Big` fields -/

/-- **`Q::add` on `Big`**: canonical fields, normal, the sum. -/
theorem qadd_spec {G : Big → Big → Big} (hG : GcdOk G) {x y : QB} (hx : Can x) (hy : Can y)
    (nx : QZ.Normal (toZ x)) (ny : QZ.Normal (toZ y)) :
    Can (qadd G x y) ∧ QZ.Normal (toZ (qadd G x y)) ∧
      QZ.val (toZ (qadd G x y)) = QZ.val (toZ x) + QZ.val (toZ y) := by
  obtain ⟨c, e⟩ := add_tie hG hx hy; rw [e]; exact ⟨c, QZ.add_spec nx ny⟩

/-- **`Q::sub` on `Big`.** -/
theorem qsub_spec {G : Big → Big → Big} (hG : GcdOk G) {x y : QB} (hx : Can x) (hy : Can y)
    (nx : QZ.Normal (toZ x)) (ny : QZ.Normal (toZ y)) :
    Can (qsub G x y) ∧ QZ.Normal (toZ (qsub G x y)) ∧
      QZ.val (toZ (qsub G x y)) = QZ.val (toZ x) - QZ.val (toZ y) := by
  obtain ⟨c, e⟩ := sub_tie hG hx hy
  obtain ⟨n1, v1⟩ := QZ.neg_spec ny
  obtain ⟨n2, v2⟩ := QZ.add_spec nx n1
  rw [e]; exact ⟨c, n2, by rw [v2, v1, sub_eq_add_neg]⟩

/-- **`Q::mul` on `Big`.** -/
theorem qmul_spec {G : Big → Big → Big} (hG : GcdOk G) {x y : QB} (hx : Can x) (hy : Can y)
    (nx : QZ.Normal (toZ x)) (ny : QZ.Normal (toZ y)) :
    Can (qmul G x y) ∧ QZ.Normal (toZ (qmul G x y)) ∧
      QZ.val (toZ (qmul G x y)) = QZ.val (toZ x) * QZ.val (toZ y) := by
  obtain ⟨c, e⟩ := mul_tie hG hx hy; rw [e]; exact ⟨c, QZ.mul_spec nx ny⟩

/-- **`Q::ratio` on `Big`.** -/
theorem ratio_spec' {G : Big → Big → Big} (hG : GcdOk G) {n d : Big} (hn : Canon n) (hd : Canon d) :
    Can (ratio G n d) ∧ QZ.Normal (toZ (ratio G n d)) ∧ QZ.val (toZ (ratio G n d)) = (val n : ℚ) / val d := by
  obtain ⟨c, e⟩ := ratio_tie hG hn hd; rw [e]; exact ⟨c, QZ.ratio_spec _ _⟩

end QB

end IEEE754.Exact
