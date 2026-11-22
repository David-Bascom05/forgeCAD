import ForgeExact.Topology

/-!
# Regularised booleans, decided cell by cell

The first theorem of the exact reference (research/kernel-audit/README.md §6).

* `reg S` is the regularisation of `S`: the closure of its interior.
* `combine op A` is a boolean of a family of sets, decided pointwise: a point is in it iff `op`
  holds of the pattern of its memberships. Union, intersection and difference of two sets are
  three choices of `op` (`Op`). A whole feature history (New, Add, Cut over many tools) is
  another; so is a fillet's body (the body, pipe solids, wedges, caps), with more sets.
* A `Decomposition` compatible with the family `A` is a finite family of cells, each lying inside
  or outside every `A i`, that covers every point off a nowhere-dense wall (for B-reps: the union
  of the carrier surfaces).

`reg_kept` : for every compatible decomposition and every `op`, the regularised union of the cells
kept by `op` (classified at any one of their points) equals the regularised boolean.

It rests on `reg_eq_of_sdiff`: two sets that differ only inside a nowhere-dense set have the same
regularisation. That is what a fillet reuses: the closing (grow then shrink) and any union of
cells computed for it differ only on the cells' walls, so they regularise alike; and since a
fillet's body is again a pointwise boolean of finitely many solids, `reg_kept` itself applies to
the arrangement of all their surfaces.

No `sorry`; `Axioms.lean` checks that only the standard axioms are used.
-/

namespace ForgeExact

open Topology

universe u v w

variable {X : Type u} [Topology X]

/-- Regularisation: the closure of the interior. -/
def reg (S : Pred X) : Pred X := closure (interior S)

/-- If `S` lies in `T` off a nowhere-dense set `Z`, then `reg S ⊆ reg T`. -/
theorem reg_sub_of_diff_sub {S T Z : Pred X} (hZ : NowhereDense Z) (h : (S.diff T).Sub Z) :
    (reg S).Sub (reg T) := by
  intro x hx U hU hxU
  -- U meets the interior of S, in an open set W
  obtain ⟨z, hzU, hzS⟩ := hx U hU hxU
  have hW : IsOpen (Pred.inter U (interior S)) := isOpen_inter _ _ hU (isOpen_interior S)
  -- W is not inside closure Z, since closure Z has empty interior
  have : ∃ y, (U y ∧ interior S y) ∧ ¬closure Z y := by
    apply Classical.byContradiction
    intro hno
    apply hZ z
    refine ⟨_, hW, fun y hy => ?_, ⟨hzU, hzS⟩⟩
    exact Classical.byContradiction fun hc => hno ⟨y, hy, hc⟩
  obtain ⟨y, ⟨hyU, hyS⟩, hyZ⟩ := this
  -- y has an open neighbourhood V missing Z
  have : ∃ V, IsOpen V ∧ V y ∧ ∀ w, V w → ¬Z w := by
    apply Classical.byContradiction
    intro hno
    apply hyZ
    intro V hV hyV
    apply Classical.byContradiction
    intro hmiss
    exact hno ⟨V, hV, hyV, fun w hw hzw => hmiss ⟨w, hw, hzw⟩⟩
  obtain ⟨V, hV, hyV, hVZ⟩ := this
  -- U ∩ int S ∩ V is open, contains y, and lies in T: so y ∈ U ∩ int T
  refine ⟨y, hyU, Pred.inter (Pred.inter U (interior S)) V, isOpen_inter _ _ hW hV, ?_,
    ⟨⟨hyU, hyS⟩, hyV⟩⟩
  intro w ⟨⟨_, hwS⟩, hwV⟩
  apply Classical.byContradiction
  intro hwT
  exact hVZ w hwV (h w ⟨interior_sub S w hwS, hwT⟩)

/-- Two sets whose symmetric difference lies in a nowhere-dense set have one regularisation. -/
theorem reg_eq_of_sdiff {S T Z : Pred X} (hZ : NowhereDense Z) (h : (S.sdiff T).Sub Z) :
    reg S = reg T :=
  Pred.ext fun x =>
    ⟨reg_sub_of_diff_sub hZ (fun y hy => h y (Or.inl hy)) x,
     reg_sub_of_diff_sub hZ (fun y hy => h y (Or.inr hy)) x⟩

/-- A boolean of the family `A`, decided pointwise by `op` on the membership pattern. -/
def combine {ι : Type v} (op : (ι → Prop) → Prop) (A : ι → Pred X) : Pred X :=
  fun x => op fun i => A i x

/-- The three booleans of two sets. -/
inductive Op
  | union
  | inter
  | diff
  deriving DecidableEq, Repr

/-- The pointwise rule of each op. -/
def Op.eval : Op → Prop → Prop → Prop
  | .union, a, b => a ∨ b
  | .inter, a, b => a ∧ b
  | .diff, a, b => a ∧ ¬b

/-- An op as a rule on the membership pattern of a pair indexed by `Bool`
(`true` is the first operand). -/
def Op.rule (o : Op) (m : Bool → Prop) : Prop := o.eval (m true) (m false)

/-- The pair `A, B` as a family indexed by `Bool`. -/
def pair (A B : Pred X) : Bool → Pred X := fun b => if b then A else B

/-- The regularised boolean of two sets. -/
def regBool (o : Op) (A B : Pred X) : Pred X := reg (combine o.rule (pair A B))

theorem regBool_union (A B : Pred X) : regBool .union A B = reg (A.union B) := rfl
theorem regBool_inter (A B : Pred X) : regBool .inter A B = reg (A.inter B) := rfl
theorem regBool_diff (A B : Pred X) : regBool .diff A B = reg (A.diff B) := rfl

/-- A finite cell decomposition compatible with the family `A`: every cell lies inside or outside
every `A i`, and the cells cover every point off a nowhere-dense wall. Cells need not be open,
disjoint or nonempty. -/
structure Decomposition {ι : Type v} (A : ι → Pred X) where
  /-- the cells' index -/
  κ : Type w
  /-- the cells, listed: there are finitely many -/
  cells : List κ
  complete : ∀ c, c ∈ cells
  /-- the cells -/
  cell : κ → Pred X
  /-- the wall: for B-reps, the union of the carrier surfaces -/
  wall : Pred X
  wall_nowhereDense : NowhereDense wall
  cover : ∀ x, ¬wall x → ∃ c, cell c x
  compatible : ∀ c i, (cell c).Sub (A i) ∨ ∀ x, cell c x → ¬A i x

variable {ι : Type v} {A : ι → Pred X}

/-- Membership in every `A i` is constant on a cell. -/
theorem Decomposition.mem_iff (D : Decomposition.{u, v, w} A) {c : D.κ} {x y : X}
    (hx : D.cell c x) (hy : D.cell c y) (i : ι) : A i x ↔ A i y := by
  rcases D.compatible c i with h | h
  · exact ⟨fun _ => h y hy, fun _ => h x hx⟩
  · exact ⟨fun h' => absurd h' (h x hx), fun h' => absurd h' (h y hy)⟩

/-- So the boolean's verdict is constant on a cell. -/
theorem Decomposition.op_iff (D : Decomposition.{u, v, w} A) (op : (ι → Prop) → Prop)
    {c : D.κ} {x y : X} (hx : D.cell c x) (hy : D.cell c y) :
    op (fun i => A i x) ↔ op (fun i => A i y) := by
  have : (fun i => A i x) = (fun i => A i y) :=
    funext fun i => propext (D.mem_iff hx hy i)
  rw [this]

/-- The union of the kept cells. -/
def Decomposition.kept (D : Decomposition.{u, v, w} A) (keep : D.κ → Prop) : Pred X :=
  fun x => ∃ c, keep c ∧ D.cell c x

/-- Off the wall, the kept cells are exactly the boolean. -/
theorem Decomposition.kept_sdiff_sub (D : Decomposition.{u, v, w} A)
    (op : (ι → Prop) → Prop) (keep : D.κ → Prop)
    (hkeep : ∀ c x, D.cell c x → (keep c ↔ op fun i => A i x)) :
    ((D.kept keep).sdiff (combine op A)).Sub D.wall := by
  intro x hx
  apply Classical.byContradiction
  intro hw
  rcases hx with ⟨⟨c, hc, hxc⟩, hC⟩ | ⟨hC, hK⟩
  · exact hC ((hkeep c x hxc).mp hc)
  · obtain ⟨c, hxc⟩ := D.cover x hw
    exact hK ⟨c, (hkeep c x hxc).mpr hC, hxc⟩

/-- **The first theorem.** For every decomposition compatible with the family and every pointwise
boolean `op`, the regularised union of the kept cells is the regularised boolean. `keep` is any
classification of the cells that agrees with `op` at the points of each cell. -/
theorem reg_kept (D : Decomposition.{u, v, w} A) (op : (ι → Prop) → Prop) (keep : D.κ → Prop)
    (hkeep : ∀ c x, D.cell c x → (keep c ↔ op fun i => A i x)) :
    reg (D.kept keep) = reg (combine op A) :=
  reg_eq_of_sdiff D.wall_nowhereDense (D.kept_sdiff_sub op keep hkeep)

/-- The form the reference computes: each cell is classified at one sample point of it (a cell
with no points may be classified either way). -/
theorem reg_kept_sampled (D : Decomposition.{u, v, w} A) (op : (ι → Prop) → Prop)
    (keep : D.κ → Prop) (pt : D.κ → X)
    (hpt : ∀ c, (∃ x, D.cell c x) → D.cell c (pt c) ∧ (keep c ↔ op fun i => A i (pt c))) :
    reg (D.kept keep) = reg (combine op A) :=
  reg_kept D op keep fun c x hx =>
    let ⟨hp, hk⟩ := hpt c ⟨x, hx⟩
    hk.trans (D.op_iff op hp hx)

/-- The two-body case: union, intersection and difference. -/
theorem reg_kept_bool (o : Op) {A B : Pred X} (D : Decomposition.{u, 0, w} (pair A B))
    (pt : D.κ → X) (hpt : ∀ c, D.cell c (pt c)) :
    reg (D.kept fun c => o.eval (A (pt c)) (B (pt c))) = regBool o A B :=
  reg_kept D o.rule _ fun c _ hx => D.op_iff o.rule (hpt c) hx

/-- Regularisation is idempotent: a regularised result is a fixed point, so the steps of a
composite operation (a fillet's closing, then a boolean) may each be regularised. -/
theorem reg_reg (S : Pred X) : reg (reg S) = reg S := by
  apply Pred.ext
  intro x
  constructor
  · intro hx
    exact closure_closure _ x (closure_mono (interior_sub _) x hx)
  · intro hx
    exact closure_mono (sub_interior (isOpen_interior S) (sub_closure _)) x hx

end ForgeExact
