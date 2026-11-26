import ForgeCertify.Theorem

/-!
# Direction-free: the three frames

`den` is ray parity along `+x`, so a result face on a plane along `x` (`n.x = 0`) is invisible to
it. The driver also runs `check` on the data with the coordinates cycled once and twice
(`cyc : (x, y, z) ↦ (y, z, x)`), where `+x` rays run along the original `+y` and `+z`. Here:
`cyc` is a homeomorphism of the reference's topology, so `reg` commutes with it, and the three
accepted checks give the theorem for ray parity along each of the three axes, in the original
coordinates: a missing, extra or misplaced face on any plane is seen by some frame.
-/

namespace ForgeCertify

open ForgeExact ForgeExact.Ref Topology

def uncyc (p : V3) : V3 := ⟨p.z, p.x, p.y⟩

theorem cyc_uncyc (p : V3) : cyc (uncyc p) = p := rfl
theorem uncyc_cyc (p : V3) : uncyc (cyc p) = p := rfl

/-- `h ∘ f` for a plane `h`, when `f` is `cyc` or `uncyc`: still a plane. -/
theorem plane_comp_cyc (h : Plane) : ∃ h' : Plane, ∀ x, h'.eval x = h.eval (cyc x) :=
  ⟨⟨uncyc h.n, h.c⟩, fun x => by simp only [Plane.eval, V3.dot, uncyc, cyc]; grind⟩

theorem plane_comp_uncyc (h : Plane) : ∃ h' : Plane, ∀ x, h'.eval x = h.eval (uncyc x) :=
  ⟨⟨cyc h.n, h.c⟩, fun x => by simp only [Plane.eval, V3.dot, uncyc, cyc]; grind⟩

/-- A map along which every plane pulls back to a plane is continuous. -/
theorem isOpen_comp {f : V3 → V3} (hf : ∀ h : Plane, ∃ h' : Plane, ∀ x, h'.eval x = h.eval (f x))
    {U : Pred V3} (hU : IsOpen U) : IsOpen (fun x => U (f x)) := by
  intro x hx
  obtain ⟨hs, hpos, hsub⟩ := hU (f x) hx
  have hm : ∀ h : Plane, ∃ h' : Plane, ∀ x, h'.eval x = h.eval (f x) := hf
  let pb : Plane → Plane := fun h => Classical.choose (hm h)
  have pbe : ∀ h x, (pb h).eval x = h.eval (f x) := fun h => Classical.choose_spec (hm h)
  refine ⟨hs.map pb, fun h' hh => ?_, fun y hy => hsub (f y) fun h hm' => ?_⟩
  · obtain ⟨h, hmem, rfl⟩ := List.mem_map.mp hh
    rw [pbe]; exact hpos h hmem
  · have := hy (pb h) (List.mem_map.mpr ⟨h, hm', rfl⟩); rwa [pbe] at this

/-- `reg` commutes with a homeomorphism (here: maps with plane pull-backs both ways). -/
theorem reg_comp {f g : V3 → V3} (fg : ∀ p, f (g p) = p) (gf : ∀ p, g (f p) = p)
    (hf : ∀ h : Plane, ∃ h' : Plane, ∀ x, h'.eval x = h.eval (f x))
    (hg : ∀ h : Plane, ∃ h' : Plane, ∀ x, h'.eval x = h.eval (g x)) (S : Pred V3) :
    reg (fun x => S (f x)) = fun x => reg S (f x) := by
  have int : ∀ T : Pred V3, interior (fun x => T (f x)) = fun x => interior T (f x) := by
    intro T; funext x; apply propext; constructor
    · rintro ⟨U, hU, hsub, hx⟩
      exact ⟨fun z => U (g z), isOpen_comp hg hU, fun z hz => by have : T (f (g z)) := hsub _ hz; rwa [fg] at this,
        show U (g (f x)) by rw [gf]; exact hx⟩
    · rintro ⟨V, hV, hsub, hx⟩
      exact ⟨fun z => V (f z), isOpen_comp hf hV, fun z hz => hsub _ hz, hx⟩
  have clo : ∀ T : Pred V3, closure (fun x => T (f x)) = fun x => closure T (f x) := by
    intro T; funext x; apply propext; constructor
    · intro h V hV hx
      obtain ⟨y, hy, hT⟩ := h (fun z => V (f z)) (isOpen_comp hf hV) hx
      exact ⟨f y, hy, hT⟩
    · intro h U hU hx
      obtain ⟨z, hz, hT⟩ := h (fun z => U (g z)) (isOpen_comp hg hU) (show U (g (f x)) by rw [gf]; exact hx)
      exact ⟨g z, hz, show T (f (g z)) by rw [fg]; exact hT⟩
  unfold reg; rw [int, clo]

theorem reg_comp_cyc (S : Pred V3) : reg (fun x => S (cyc x)) = fun x => reg S (cyc x) :=
  reg_comp cyc_uncyc uncyc_cyc plane_comp_cyc plane_comp_uncyc S

/-- Ray parity along `+y` (`k = 1`) or `+z` (`k = 2`): the `+x` parity of the cycled faces at
the cycled point. -/
def cycN : Nat → V3 → V3
  | 0 => id
  | k + 1 => fun p => cyc (cycN k p)

def cycFaceN : Nat → Face → Face
  | 0 => id
  | k + 1 => fun f => cycFace (cycFaceN k f)

def cycOpdN : Nat → Opd → Opd
  | 0 => id
  | k + 1 => fun o => cycOpd (cycOpdN k o)

theorem cycOpdN_faces (k : Nat) (o : Opd) : (cycOpdN k o).faces = (o.faces.map (cycFaceN k)) := by
  induction k with
  | zero => simp [cycOpdN, cycFaceN]
  | succ k ih => simp [cycOpdN, cycFaceN, cycOpd, ih, List.map_map, Function.comp_def]

/-- The membership of `x` by ray parity along axis `k` (0: `x`, 1: `y`, 2: `z`). -/
def denAxis (k : Nat) (fs : List Face) : Pred V3 := fun x => den (fs.map (cycFaceN k)) (cycN k x)

def opSetAxis (k : Nat) (op : List Bool → Bool) (ins : List Opd) : Pred V3 :=
  fun x => opSet op (ins.map (cycOpdN k)) (cycN k x)

theorem reg_comp_cycN (k : Nat) (S : Pred V3) : reg (fun x => S (cycN k x)) = fun x => reg S (cycN k x) := by
  induction k generalizing S with
  | zero => rfl
  | succ k ih =>
    exact (ih fun p => S (cyc p)).trans (funext fun x => congrFun (reg_comp_cyc S) (cycN k x))

/-- One frame: the check of the data cycled `k` times proves the theorem for ray parity along
axis `k`, in the original coordinates. -/
theorem check_sound_axis (k : Nat) (op : List Bool → Bool) (ins : List Opd) (res : List Face)
    (hc : check op (ins.map (cycOpdN k)) (res.map (cycFaceN k)) = true) :
    reg (denAxis k res) = reg (opSetAxis k op ins) := by
  unfold denAxis opSetAxis
  rw [reg_comp_cycN, reg_comp_cycN, check_sound _ _ _ hc]

/-- The data the driver checks in frame `k` (it cycles once per frame). -/
theorem frame_data (k : Nat) (ins : List Opd) (res : List Face) :
    ((List.range k).foldl (fun (d : List Opd × List Face) _ => (d.1.map cycOpd, d.2.map cycFace)) (ins, res)) =
      (ins.map (cycOpdN k), res.map (cycFaceN k)) := by
  induction k with
  | zero => simp [cycOpdN, cycFaceN]
  | succ k ih =>
    rw [List.range_succ, List.foldl_append, ih]
    simp [List.map_map, Function.comp_def, cycOpdN, cycFaceN]

/-- **Direction-free soundness.** The three frames accepted ⇒ for ray parity along each axis,
the result is the exact regularised boolean. -/
theorem check_sound_xyz (op : List Bool → Bool) (ins : List Opd) (res : List Face)
    (hc : ∀ k < 3, check op (ins.map (cycOpdN k)) (res.map (cycFaceN k)) = true) :
    ∀ k < 3, reg (denAxis k res) = reg (opSetAxis k op ins) :=
  fun k hk => check_sound_axis k op ins res (hc k hk)

/-- Axis 0 is the plain statement. -/
theorem denAxis_zero (fs : List Face) : denAxis 0 fs = den fs := by
  funext x; simp [denAxis, cycFaceN, cycN]

theorem opSetAxis_zero (op : List Bool → Bool) (ins : List Opd) : opSetAxis 0 op ins = opSet op ins := by
  funext x; simp [opSetAxis, cycOpdN, cycN]

end ForgeCertify
