import ForgeBoolProof.Contracts
import ForgeCurved.Rooms
import ForgeCurved.Bridge

/-!
# The composition: §D's contracts give §E (`boolean_correct_of_contracts`)

Assuming D1–D9 and the boundary lemma `KeepRuleBd`, `Pipeline.run` is `BooleanCorrect`. The
steps:
* a refusal at any step is true by that step's contract;
* D1–D7 chain their postconditions, and D7's two halves (the code's `KeepOK`, the mathematics'
  `KeepRuleBd`) give D8's precondition `KeptIsBd` (`keptIsBd_congr`);
* **`res_keepRoom`**: on an arrangement of two valid exact solids' faces (D5's `ArrOK`), the
  closure of the kept rooms is the exact result `reg (opSet op ⟦A⟧ ⟦B⟧)`. This is F5's
  `res_eq_reg`, with its hypotheses discharged here: the faces are nowhere dense (each body's
  carriers' zeros are closed with empty interior), no room meets either solid's frontier (it lies
  on the faces, `Bounds.frontier_sub`), so membership is constant on a room (`room_class`);
* **`apply_correct`**: D8's postcondition then gives `ValidSolid`, F1, and
  `⟦R⟧ = reg (opSet op ⟦A⟧ ⟦B⟧)` by uniqueness of the bounded solid (`denoteExact_eq`);
* D9 passes `R` on.

Also here: the flat fallback (`fallback_correct`: today's path, then A8 on a retryable refusal; a
wrong `Ok` of today's path is excluded by today's contracts alone), and sequential regen
(`seq_correct`: a fold of booleans, by induction, given that each boolean keeps `InputSize`).
-/

set_option linter.unusedSectionVars false

namespace ForgeBoolProof

open Set Filter Topology ForgeCurved.BodyModel ForgeCurved.FaceLocal ForgeCurved.FacetSides
  ForgeCurved.F6 ForgeCurved.Rooms

variable {E : Type} [NormedAddCommGroup E] [NormedSpace ℝ E]

/-- The faces of two bodies are nowhere dense. -/
theorem faces_nowhereDense (Aex Bex : BodyModel E) :
    interior (closure (Faces Aex ∪ Faces Bex)) = ∅ := by
  have hsub : Faces Aex ∪ Faces Bex ⊆ Zeros Aex ∪ Zeros Bex :=
    union_subset_union (faces_sub_zeros Aex) (faces_sub_zeros Bex)
  have hcl : IsClosed (Zeros Aex ∪ Zeros Bex) := (zeros_closed _).union (zeros_closed _)
  have hint : interior (Zeros Aex ∪ Zeros Bex) = ∅ := by
    rw [interior_union_isClosed_of_interior_empty (zeros_closed Aex) (zeros_interior Bex)]
    exact zeros_interior Aex
  exact eq_empty_of_subset_empty (hint ▸ interior_mono (closure_minimal hsub hcl))

/-- Membership in a solid whose frontier is on the faces is constant on a room. -/
theorem room_const {Aex Bex : BodyModel E} {a : Arr E} (ha : ArrOK Aex Bex a) {S : Set E}
    (hS : frontier S ⊆ a.FacesG) (r : Nat) {x y : E} (hx : x ∈ a.G.room r) (hy : y ∈ a.G.room r) :
    (x ∈ S ↔ y ∈ S) := by
  rcases room_class (ha.conn r) ((ha.data.room_faces r).mono_right hS) with h | h
  · exact ⟨fun _ => h hy, fun _ => h hx⟩
  · exact ⟨fun h' => absurd h' (disjoint_left.mp h hx), fun h' => absurd h' (disjoint_left.mp h hy)⟩

/-- **The closure of the kept rooms is the exact result.** -/
theorem res_keepRoom {Aex Bex : BodyModel E} {a : Arr E} (hvA : ValidExact Aex)
    (hvB : ValidExact Bex) (ha : ArrOK Aex Bex a) (op : Op) :
    Res a.G (keepRoom a.G (denoteExact Aex) (denoteExact Bex) op) =
      reg (opSet op (denoteExact Aex) (denoteExact Bex)) := by
  set SA := denoteExact Aex
  set SB := denoteExact Bex
  have hfrA : frontier SA ⊆ a.FacesG := by
    rw [ha.faces]; exact (bounds_denoteExact hvA).frontier_sub.trans subset_union_left
  have hfrB : frontier SB ⊆ a.FacesG := by
    rw [ha.faces]; exact (bounds_denoteExact hvB).frontier_sub.trans subset_union_right
  let Aset : Bool → ForgeExact.Pred E := fun b x => match b with
    | true => x ∈ SA
    | false => x ∈ SB
  have hZ : interior (closure a.FacesG) = ∅ := by rw [ha.faces]; exact faces_nowhereDense Aex Bex
  have hdisj : ∀ r r', keepRoom a.G SA SB op r ≠ keepRoom a.G SA SB op r' →
      Disjoint (a.G.room r) (a.G.room r') := by
    intro r r' h
    apply ha.data.room_disj
    rintro rfl
    exact h rfl
  have hfr : ∀ r i, Disjoint (a.G.room r) (frontier {x | Aset i x}) := by
    intro r i
    cases i
    · exact (ha.data.room_faces r).mono_right hfrB
    · exact (ha.data.room_faces r).mono_right hfrA
  have hkeep : ∀ r, (a.G.room r).Nonempty → ∃ x₀ ∈ a.G.room r,
      (keepRoom a.G SA SB op r = true ↔ ForgeExact.combine (ForgeExact.Op.rule op) Aset x₀) := by
    rintro r ⟨x₀, hx₀⟩
    refine ⟨x₀, hx₀, ?_⟩
    simp only [keepRoom, decide_eq_true_eq]
    show (∃ x ∈ a.G.room r, op.eval (x ∈ SA) (x ∈ SB)) ↔ op.eval (x₀ ∈ SA) (x₀ ∈ SB)
    constructor
    · rintro ⟨x, hx, hev⟩
      rwa [propext (room_const ha hfrA r hx hx₀), propext (room_const ha hfrB r hx hx₀)] at hev
    · exact fun h => ⟨x₀, hx₀, h⟩
  have h := res_eq_reg a.G (keepRoom a.G SA SB op) (ForgeExact.Op.rule op) Aset a.FacesG hZ
    ha.data.room_open hdisj ha.data.room_cover ha.conn hfr hkeep
  rw [h]
  have hset : {y | ForgeExact.combine (ForgeExact.Op.rule op) Aset y} = opSet op SA SB := by
    cases op <;> rfl
  ext x
  show ForgeExact.Topology.closure (ForgeExact.Topology.interior _) x ↔ x ∈ closure (interior _)
  rw [ForgeCurved.closure_iff, ForgeCurved.interior_eq, hset]

/-- **The closure of the kept rooms has its frontier on the arrangement's faces**: off them a
point is in a room, which is kept (then the point is interior) or misses the closure. -/
theorem frontier_res_sub {G : FaceGeo E} {FacesG : Set E} {cf : Nat → E → ℝ}
    (hd : FacetData G FacesG cf) (keep : Nat → Bool) : frontier (Res G keep) ⊆ FacesG := by
  intro x hx
  by_contra hxF
  obtain ⟨r, hr⟩ := hd.room_cover x hxF
  cases hk : keep r
  · have := room_disj_res (sideLocal_of_data hd keep) hk
    exact disjoint_left.mp this hr (isClosed_closure.frontier_subset hx)
  · have hint : x ∈ interior (Res G keep) :=
      interior_maximal ((room_sub_kept hk).trans subset_closure) (hd.room_open r) hr
    exact hx.2 hint

/-- **D8's face obligations give `Bounds`**: the closure of the kept rooms is closed, regular
(the closure of an open set), bounded (it is the exact result, inside `⟦A⟧ ∪ ⟦B⟧`), and its
frontier is on the arrangement's faces. -/
theorem bounds_of_faces {Aex Bex Rs : BodyModel E} {a : Arr E} (hvA : ValidExact Aex)
    (hvB : ValidExact Bex) (ha : ArrOK Aex Bex a) (op : Op)
    (h : BoundsFaces Rs a.FacesG (Res a.G (keepRoom a.G (denoteExact Aex) (denoteExact Bex) op))) :
    Bounds Rs (Res a.G (keepRoom a.G (denoteExact Aex) (denoteExact Bex) op)) := by
  set keep := keepRoom a.G (denoteExact Aex) (denoteExact Bex) op
  have hKo : IsOpen (Kept a.G keep) :=
    isOpen_iUnion fun r => isOpen_iUnion fun _ => ha.data.room_open r
  refine ⟨isClosed_closure, ?_, ?_, fun x hx => h.frontier_on ⟨frontier_res_sub ha.data keep hx, hx⟩,
    h.faces_sub, h.local_formula, h.oriented, h.seeded⟩
  · apply Set.Subset.antisymm
    · exact (closure_mono interior_subset).trans_eq closure_closure
    · exact closure_mono (interior_maximal subset_closure hKo)
  · have hsub : opSet op (denoteExact Aex) (denoteExact Bex) ⊆ denoteExact Aex ∪ denoteExact Bex := by
      cases op
      · exact Subset.rfl
      · exact fun x hx => Or.inl hx.1
      · exact fun x hx => Or.inl hx.1
    have hb := ((bounds_denoteExact hvA).bounded.union (bounds_denoteExact hvB).bounded).subset hsub
    rw [res_keepRoom hvA hvB ha op]
    exact hb.closure.subset (closure_mono interior_subset)

section Compose

variable {Body K Q : Type} {δ : ℝ} {abs : Body → BodyModel E} {sc : Scope Body K Q}
  {Names : Body → Body → Op → Body → Prop} {Size : Body → Prop}

/-- **D8's postcondition is the end-to-end postcondition.** -/
theorem apply_correct {A B R : Body} {op : Op} {Aex Bex : BodyModel E} {a : Arr E}
    (hAex : exactOf δ (abs A) = some Aex) (hBex : exactOf δ (abs B) = some Bex)
    (hvA : ValidExact Aex) (hvB : ValidExact Bex) (ha : ArrOK Aex Bex a)
    (hpost : ApplyPost δ abs Names A B op a (denoteExact Aex) (denoteExact Bex) R) :
    Correct δ abs sc Names A B op (.ok R) := by
  obtain ⟨Rs, hRs, hiso, hwf, hbf, hfa, hnames⟩ := hpost
  have hbd := bounds_of_faces hvA hvB ha op hbf
  refine ⟨⟨Rs, hRs, hiso, hwf, _, hbd⟩, hfa, ?_, hnames⟩
  rw [denote_spec hRs, denoteExact_eq hwf hbd, res_keepRoom hvA hvB ha op, denote_spec hAex,
    denote_spec hBex]

/-- `KeptIsBd` depends only on the flags of the regions. -/
theorem keptIsBd_congr {a : Arr E} {k k' keepR : Nat → Bool} (hk : ∀ f ∈ a.regions, k f = k' f)
    (h : KeptIsBd a k' keepR) : KeptIsBd a k keepR := by
  refine ⟨fun g hg => ?_, fun f hf f' hf' hne h1 h2 h3 => h.2 f hf f' hf' hne ((hk f hf) ▸ h1)
    ((hk f' hf') ▸ h2) h3⟩
  rw [h.1 g hg]
  constructor
  · rintro ⟨f, hf, hg', hk'⟩; exact ⟨f, hf, hg', (hk f hf).trans hk'⟩
  · rintro ⟨f, hf, hg', hk'⟩; exact ⟨f, hf, hg', (hk f hf).symm.trans hk'⟩

/-- **§D ⇒ §E: the composition.** Given the nine contracts and the boundary lemma, today's
pipeline is correct: every `ok` is a valid, faithful solid equal to the exact result and correctly
named, and every refusal is true. -/
theorem boolean_correct_of_contracts {P : Pipeline Body (Refusal E K Q)} {V : View P E}
    (h1 : D1 δ abs sc Size P V) (h2 : D2 δ abs sc Size P V) (h3 : D3 δ abs sc Size P V)
    (h4 : D4 δ abs sc Size P V) (h5 : D5 δ abs sc Size P V) (h6 : D6 δ abs sc Size P V)
    (h7 : D7 δ abs sc Size P V) (h8 : D8 δ abs sc Names Size P V) (h9 : D9 δ abs sc Size P)
    (hK : KeepRuleBd (E := E)) :
    BooleanCorrect δ abs sc Names Size P.run := by
  intro A B op hA hB
  obtain ⟨Aex, hAex, -, hvA⟩ := hA.1
  obtain ⟨Bex, hBex, -, hvB⟩ := hB.1
  have hc : Ctx δ abs Size A B Aex Bex := ⟨hA, hB, hAex, hBex, hvA, hvB⟩
  unfold Pipeline.run
  have s1 := h1 A B Aex Bex op hc
  rcases e1 : P.index A B with e | pr
  · simp only; rw [e1] at s1; exact s1
  simp only
  rw [e1] at s1
  have s2 := h2 A B Aex Bex op pr hc s1
  rcases e2 : P.meet A B pr with e | cv
  · simp only; rw [e2] at s2; exact s2
  simp only
  rw [e2] at s2
  have s3 := h3 A B Aex Bex op _ cv hc s1 s2
  rcases e3 : P.touch A B cv with e | tc
  · simp only; rw [e3] at s3; exact s3
  simp only
  rw [e3] at s3
  have s4 := h4 A B Aex Bex op _ cv tc hc s1 s2 s3
  rcases e4 : P.trim A B cv tc with e | sg
  · simp only; rw [e4] at s4; exact s4
  simp only
  rw [e4] at s4
  have s5 := h5 A B Aex Bex op sg hc s4
  rcases e5 : P.imprint A B sg with e | fs
  · simp only; rw [e5] at s5; exact s5
  simp only
  rw [e5] at s5
  have s6 := h6 A B Aex Bex op fs hc s5
  rcases e6 : P.classify A B fs with e | cs
  · simp only; rw [e6] at s6; exact s6
  simp only
  rw [e6] at s6
  have s7 := h7 A B Aex Bex op fs cs hc s5 s6
  rcases e7 : P.keep op fs cs with e | kp
  · simp only; rw [e7] at s7; exact s7
  simp only
  rw [e7] at s7
  have hbd : KeptIsBd (V.arr fs) (V.kept kp)
      (keepRoom (V.arr fs).G (denoteExact Aex) (denoteExact Bex) op) :=
    keptIsBd_congr s7 (hK Aex Bex (V.arr fs) op (V.cls cs) hvA hvB s5 s6)
  have s8 := h8 A B Aex Bex op fs kp hc s5 hbd
  rcases e8 : P.apply A B op fs kp with e | R
  · simp only; rw [e8] at s8; exact s8
  simp only
  rw [e8] at s8
  have s9 := h9 A B Aex Bex op R hc
  rcases e9 : P.pinch A B op R with e | R'
  · rw [e9] at s9; exact s9
  rw [e9] at s9
  change R' = R at s9
  subst s9
  exact apply_correct hAex hBex hvA hvB s5 s8

/-! ## The flat fallback (old-path-statements.md §E, the owner's clarification) -/

/-- Today's path first; on a refusal the fallback may retry (`retry`: `Undecided`, or a refusal
A8 is entered on), A8 runs from the **original inputs**; otherwise the refusal stands. -/
def withFallback (today a8 : Body → Body → Op → Except (Refusal E K Q) Body)
    (retry : Refusal E K Q → Bool) (A B : Body) (op : Op) : Except (Refusal E K Q) Body :=
  match today A B op with
  | .ok R => .ok R
  | .error e => if retry e then a8 A B op else .error e

/-- What today's path must give on its own: a correct `ok` (no rescue by the fallback), and a
refusal that is either retried or true. -/
def TodayOK (δ : ℝ) (abs : Body → BodyModel E) (sc : Scope Body K Q)
    (Names : Body → Body → Op → Body → Prop) (Size : Body → Prop)
    (today : Body → Body → Op → Except (Refusal E K Q) Body) (retry : Refusal E K Q → Bool) :
    Prop :=
  ∀ A B op, Input δ abs Size A → Input δ abs Size B →
    match today A B op with
    | .ok R => Correct δ abs sc Names A B op (.ok R)
    | .error e => retry e = true ∨ RefusalTrue δ abs sc A B op e

/-- **The fallback composition.** -/
theorem fallback_correct {today a8 : Body → Body → Op → Except (Refusal E K Q) Body}
    {retry : Refusal E K Q → Bool} (hT : TodayOK δ abs sc Names Size today retry)
    (hA8 : BooleanCorrect δ abs sc Names Size a8) :
    BooleanCorrect δ abs sc Names Size (withFallback today a8 retry) := by
  intro A B op hA hB
  have h := hT A B op hA hB
  unfold withFallback
  rcases e : today A B op with err | R
  · rw [e] at h
    simp only
    by_cases hr : retry err = true
    · rw [if_pos hr]; exact hA8 A B op hA hB
    · rw [if_neg hr]; exact h.resolve_left hr
  · rw [e] at h; simp only; exact h

/-- The pipeline's `Correct` gives `TodayOK` for any retry set. -/
theorem todayOK_of_correct {today : Body → Body → Op → Except (Refusal E K Q) Body}
    (h : BooleanCorrect δ abs sc Names Size today) (retry : Refusal E K Q → Bool) :
    TodayOK δ abs sc Names Size today retry := by
  intro A B op hA hB
  have := h A B op hA hB
  rcases e : today A B op with err | R
  · simp only; rw [e] at this; exact Or.inr this
  · simp only; rw [e] at this; exact this

/-! ## Sequential regen: a fold of booleans -/

/-- Apply the tools in order, stopping at the first refusal. -/
def seq (boolean : Body → Body → Op → Except (Refusal E K Q) Body) (A : Body) :
    List (Op × Body) → Except (Refusal E K Q) Body
  | [] => .ok A
  | (op, T) :: ts => match boolean A T op with
    | .ok R => seq boolean R ts
    | .error e => .error e

/-- The exact fold. -/
def seqSet (S : Set E) : List (Op × Set E) → Set E
  | [] => S
  | (op, T) :: ts => seqSet (reg (opSet op S T)) ts

/-- **Sequential regen is correct**, given that each boolean keeps `InputSize`
(`boolean_preserves_InputSize`): an `ok` is a valid, faithful solid equal to the exact fold, and
a refusal is true of the step that made it. -/
theorem seq_correct {boolean : Body → Body → Op → Except (Refusal E K Q) Body}
    (hb : BooleanCorrect δ abs sc Names Size boolean)
    (hsize : ∀ A B op R, Size A → Size B → boolean A B op = .ok R → Size R) :
    ∀ (ts : List (Op × Body)) (A : Body), Input δ abs Size A → (∀ t ∈ ts, Input δ abs Size t.2) →
      match seq boolean A ts with
      | .ok R => Input δ abs Size R ∧
          denote δ (abs R) = seqSet (denote δ (abs A)) (ts.map fun t => (t.1, denote δ (abs t.2)))
      | .error e => ∃ A' T op, RefusalTrue δ abs sc A' T op e
  | [], A, hA, _ => by simp only [seq]; exact ⟨hA, rfl⟩
  | (op, T) :: ts, A, hA, hts => by
    have hT : Input δ abs Size T := hts (op, T) (List.mem_cons_self ..)
    have hc := hb A T op hA hT
    simp only [seq]
    rcases e : boolean A T op with err | R
    · rw [e] at hc; exact ⟨A, T, op, hc⟩
    · rw [e] at hc
      obtain ⟨hvR, hfR, hdR, -⟩ := hc
      have hR : Input δ abs Size R := ⟨hvR, hfR, hsize A T op R hA.2.2 hT.2.2 e⟩
      have ih := seq_correct hb hsize ts R hR (fun t ht => hts t (List.mem_cons_of_mem _ ht))
      simp only
      rcases e2 : seq boolean R ts with err | R'
      · rw [e2] at ih; exact ih
      · rw [e2] at ih
        obtain ⟨hi, hd⟩ := ih
        refine ⟨hi, ?_⟩
        rw [hd, hdR]
        rfl

end Compose

end ForgeBoolProof
