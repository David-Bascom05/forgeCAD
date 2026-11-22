import ForgeExact.Snap

/-!
# The tolerance rule does not depend on the coordinates

`Sym` is an exact symmetry of ℚ³ that keeps axis boxes axis boxes, given by what it does to
points, planes and boxes and the laws those obey. For every such symmetry the relation, the
classes and the representative's name are the same before and after (`tolEq_sym`, `conn_sym`,
`rep_sym`). Instances: every translation (`translate`), the mirror in x (`mirrorX`) and the swaps
of x with y and of y with z (`swapXY`, `swapYZ`). Compositions of symmetries are symmetries
(`Sym.comp`), and these generate every signed permutation of the axes with every rational
translation: so the M1 mirror case is settled by the definition.
-/

namespace ForgeExact.Snap

open Ref Flat

structure Sym where
  f : V3 → V3
  plane : Plane → Plane
  box : Box → Box
  eval : ∀ P x, (plane P).eval (f x) = P.eval x
  norm : ∀ P, (plane P).n.norm2 = P.n.norm2
  proj : ∀ S c, Snap.proj (plane S) (f c) = f (Snap.proj S c)
  grow : ∀ b l, box (b.grow l) = (box b).grow l
  inter : ∀ a b, box (a.inter b) = (box a).inter (box b)
  nonempty : ∀ R, (box R).nonempty = R.nonempty
  corners : ∀ R c, c ∈ (box R).corners ↔ ∃ c0 ∈ R.corners, c = f c0

def Sym.comp (g h : Sym) : Sym where
  f := g.f ∘ h.f
  plane := g.plane ∘ h.plane
  box := g.box ∘ h.box
  eval P x := by simp only [Function.comp, g.eval, h.eval]
  norm P := by simp only [Function.comp, g.norm, h.norm]
  proj S c := by simp only [Function.comp, g.proj, h.proj]
  grow b l := by simp only [Function.comp, g.grow, h.grow]
  inter a b := by simp only [Function.comp, g.inter, h.inter]
  nonempty R := by simp only [Function.comp, g.nonempty, h.nonempty]
  corners R c := by
    simp only [Function.comp]
    rw [g.corners]
    constructor
    · rintro ⟨c1, hc1, rfl⟩
      obtain ⟨c0, hc0, rfl⟩ := (h.corners R c1).mp hc1
      exact ⟨c0, hc0, rfl⟩
    · rintro ⟨c0, hc0, rfl⟩
      exact ⟨h.f c0, (h.corners R _).mpr ⟨c0, hc0, rfl⟩, rfl⟩

/-- A carrier moved by the symmetry; its name stays. -/
def Sym.carrier (g : Sym) (a : Carrier) : Carrier := ⟨a.name, g.plane a.plane, g.box a.box⟩

theorem Sym.within (g : Sym) (T : Plane) (x : V3) (d : Rat) :
    (g.plane T).within (g.f x) d = T.within x d := by
  simp only [Plane.within, g.eval, g.norm]

theorem Sym.near_eq (g : Sym) (S T : Plane) (R : Box) (d : Rat) :
    near (g.plane S) (g.plane T) (g.box R) d = near S T R d := by
  unfold Snap.near
  apply Bool.eq_iff_iff.mpr
  simp only [List.all_eq_true]
  constructor
  · intro h c hc
    have := h (g.f c) ((g.corners R _).mpr ⟨c, hc, rfl⟩)
    rwa [g.proj, g.within] at this
  · intro h c hc
    obtain ⟨c0, hc0, rfl⟩ := (g.corners R c).mp hc
    rw [g.proj, g.within]; exact h c0 hc0

theorem Sym.region_eq (g : Sym) (l : Rat) (a b : Carrier) :
    region l (g.carrier a) (g.carrier b) = g.box (region l a b) := by
  simp only [Snap.region, Sym.carrier, g.inter, g.grow]

/-- **The relation does not depend on the coordinates.** -/
theorem tolEq_sym (g : Sym) (l : Rat) (a b : Carrier) :
    tolEq l (g.carrier a) (g.carrier b) = tolEq l a b := by
  unfold tolEq
  rw [g.region_eq, g.nonempty]
  simp only [Sym.carrier, g.near_eq]

theorem conn_sym_fwd (g : Sym) (l : Rat) {cs : List Carrier} {a b : Carrier}
    (h : Conn l cs a b) : Conn l (cs.map g.carrier) (g.carrier a) (g.carrier b) := by
  induction h with
  | refl => exact Conn.refl
  | step _ hc hbc ih =>
    exact Conn.step ih (List.mem_map_of_mem hc) (by rw [tolEq_sym]; exact hbc)

/-- A chain from `a`'s image in the moved list is the image of a chain from `a`. -/
theorem conn_sym_back (g : Sym) (l : Rat) {cs : List Carrier} {a : Carrier} :
    ∀ {b'}, Conn l (cs.map g.carrier) (g.carrier a) b' →
      ∃ b, b' = g.carrier b ∧ Conn l cs a b := by
  intro b' h
  induction h with
  | refl => exact ⟨a, rfl, Conn.refl⟩
  | step _ hd' hcd ih =>
    obtain ⟨c, rfl, hc⟩ := ih
    obtain ⟨d, hd, rfl⟩ := List.mem_map.mp hd'
    exact ⟨d, rfl, Conn.step hc hd (by rw [← tolEq_sym g]; exact hcd)⟩

theorem nodup_of_map {α β : Type} {f : α → β} : ∀ {l : List α}, (l.map f).Nodup → l.Nodup
  | [], _ => List.nodup_nil
  | a :: l, h => by
    rw [List.map_cons, List.nodup_cons] at h
    exact List.nodup_cons.mpr ⟨fun ha => h.1 (List.mem_map_of_mem ha), nodup_of_map h.2⟩

theorem names_map (g : Sym) (cs : List Carrier) :
    (cs.map g.carrier).map Carrier.name = cs.map Carrier.name := by
  rw [List.map_map]; rfl

/-- **The representative does not depend on the coordinates**: after any symmetry, the class of
a carrier's image is represented by the image of its representative, so the same name. -/
theorem rep_sym (g : Sym) (l : Rat) {cs : List Carrier} (hcs : cs.Nodup)
    (hnames : (cs.map Carrier.name).Nodup) {a : Carrier} (ha : a ∈ cs) :
    rep l (cs.map g.carrier) (g.carrier a) = g.carrier (rep l cs a) := by
  have hn' : ((cs.map g.carrier).map Carrier.name).Nodup := by rw [names_map]; exact hnames
  have hcs' : (cs.map g.carrier).Nodup := nodup_of_map hn'
  apply rep_unique l hcs' hn' (List.mem_map_of_mem ha) (conn_sym_fwd g l (rep_mem l hcs ha))
  intro b' hb'
  obtain ⟨b, rfl, hb⟩ := conn_sym_back g l hb'
  exact rep_least l hcs ha b hb

/-! ## The symmetries -/

theorem qmax_add (a b t : Rat) : qmax (a + t) (b + t) = qmax a b + t := by
  unfold qmax; split <;> split <;> grind
theorem qmin_add (a b t : Rat) : qmin (a + t) (b + t) = qmin a b + t := by
  unfold qmin; split <;> split <;> grind
theorem qmax_neg (a b : Rat) : qmax (-a) (-b) = -qmin a b := by
  unfold qmax qmin; split <;> split <;> grind
theorem qmin_neg (a b : Rat) : qmin (-a) (-b) = -qmax a b := by
  unfold qmax qmin; split <;> split <;> grind

@[simp] theorem sub_x (a b : V3) : (a - b).x = a.x - b.x := rfl
@[simp] theorem sub_y (a b : V3) : (a - b).y = a.y - b.y := rfl
@[simp] theorem sub_z (a b : V3) : (a - b).z = a.z - b.z := rfl

theorem v3_ext {a b : V3} (hx : a.x = b.x) (hy : a.y = b.y) (hz : a.z = b.z) : a = b := by
  obtain ⟨_, _, _⟩ := a; obtain ⟨_, _, _⟩ := b; simp_all

theorem box_ext {a b : Box} (hl : a.lo = b.lo) (hh : a.hi = b.hi) : a = b := by
  obtain ⟨_, _⟩ := a; obtain ⟨_, _⟩ := b; simp_all

/-- Translation by `t`. -/
def translate (t : V3) : Sym where
  f x := x + t
  plane P := ⟨P.n, P.c + P.n.dot t⟩
  box b := ⟨b.lo + t, b.hi + t⟩
  eval P x := by simp only [Plane.eval, V3.dot, add_x, add_y, add_z]; grind
  norm P := rfl
  proj S c := by
    have e : Plane.eval ⟨S.n, S.c + S.n.dot t⟩ (c + t) = S.eval c := by
      simp only [Plane.eval, V3.dot, add_x, add_y, add_z]; grind
    simp only [Snap.proj, e]
    apply v3_ext <;> simp only [sub_x, sub_y, sub_z, add_x, add_y, add_z, smul_x, smul_y, smul_z] <;>
      grind
  grow b l := by
    apply box_ext <;> apply v3_ext <;>
      simp only [Box.grow, sub_x, sub_y, sub_z, add_x, add_y, add_z] <;> grind
  inter a b := by
    apply box_ext <;> apply v3_ext <;>
      simp only [Box.inter, add_x, add_y, add_z, qmax_add, qmin_add]
  nonempty R := by simp only [Box.nonempty, add_x, add_y, add_z]; grind
  corners R c := by
    rw [Box.mem_corners]
    constructor
    · intro h
      refine ⟨c - t, (Box.mem_corners _ _).mpr ?_, v3_ext ?_ ?_ ?_⟩ <;>
        simp only [sub_x, sub_y, sub_z, add_x, add_y, add_z] at * <;> grind
    · rintro ⟨c0, hc0, rfl⟩
      rw [Box.mem_corners] at hc0
      simp only [add_x, add_y, add_z]; grind

/-- The mirror in x. -/
def mirrorX : Sym where
  f x := ⟨-x.x, x.y, x.z⟩
  plane P := ⟨⟨-P.n.x, P.n.y, P.n.z⟩, P.c⟩
  box b := ⟨⟨-b.hi.x, b.lo.y, b.lo.z⟩, ⟨-b.lo.x, b.hi.y, b.hi.z⟩⟩
  eval P x := by simp only [Plane.eval, V3.dot]; grind
  norm P := by simp only [V3.norm2, V3.dot]; grind
  proj S c := by
    have e : Plane.eval ⟨⟨-S.n.x, S.n.y, S.n.z⟩, S.c⟩ ⟨-c.x, c.y, c.z⟩ = S.eval c := by
      simp only [Plane.eval, V3.dot]; grind
    have e2 : V3.norm2 ⟨-S.n.x, S.n.y, S.n.z⟩ = S.n.norm2 := by
      simp only [V3.norm2, V3.dot]; grind
    simp only [Snap.proj, e, e2]
    apply v3_ext <;> simp only [sub_x, sub_y, sub_z, smul_x, smul_y, smul_z] <;> grind
  grow b l := by
    apply box_ext <;> apply v3_ext <;>
      simp only [Box.grow, sub_x, sub_y, sub_z, add_x, add_y, add_z] <;> grind
  inter a b := by
    apply box_ext <;> apply v3_ext <;> simp only [Box.inter, qmax_neg, qmin_neg] <;>
      first | rfl | rw [qmin_comm] | rw [qmax_comm]
  nonempty R := by simp only [Box.nonempty]; grind
  corners R c := by
    rw [Box.mem_corners]
    constructor
    · intro h
      refine ⟨⟨-c.x, c.y, c.z⟩, (Box.mem_corners _ _).mpr ?_, v3_ext ?_ rfl rfl⟩ <;>
        simp only at * <;> grind
    · rintro ⟨c0, hc0, rfl⟩
      rw [Box.mem_corners] at hc0
      simp only; grind

/-- The swap of x and y. -/
def swapXY : Sym where
  f x := ⟨x.y, x.x, x.z⟩
  plane P := ⟨⟨P.n.y, P.n.x, P.n.z⟩, P.c⟩
  box b := ⟨⟨b.lo.y, b.lo.x, b.lo.z⟩, ⟨b.hi.y, b.hi.x, b.hi.z⟩⟩
  eval P x := by simp only [Plane.eval, V3.dot]; grind
  norm P := by simp only [V3.norm2, V3.dot]; grind
  proj S c := by
    have e : Plane.eval ⟨⟨S.n.y, S.n.x, S.n.z⟩, S.c⟩ ⟨c.y, c.x, c.z⟩ = S.eval c := by
      simp only [Plane.eval, V3.dot]; grind
    have e2 : V3.norm2 ⟨S.n.y, S.n.x, S.n.z⟩ = S.n.norm2 := by
      simp only [V3.norm2, V3.dot]; grind
    simp only [Snap.proj, e, e2]
    apply v3_ext <;> simp only [sub_x, sub_y, sub_z, smul_x, smul_y, smul_z]
  grow b l := by
    apply box_ext <;> apply v3_ext <;> simp only [Box.grow, sub_x, sub_y, sub_z, add_x, add_y, add_z]
  inter a b := by apply box_ext <;> apply v3_ext <;> simp only [Box.inter]
  nonempty R := by simp only [Box.nonempty]; grind
  corners R c := by
    rw [Box.mem_corners]
    constructor
    · intro h
      exact ⟨⟨c.y, c.x, c.z⟩, (Box.mem_corners _ _).mpr ⟨h.2.1, h.1, h.2.2⟩, rfl⟩
    · rintro ⟨c0, hc0, rfl⟩
      rw [Box.mem_corners] at hc0
      exact ⟨hc0.2.1, hc0.1, hc0.2.2⟩

/-- The swap of y and z. -/
def swapYZ : Sym where
  f x := ⟨x.x, x.z, x.y⟩
  plane P := ⟨⟨P.n.x, P.n.z, P.n.y⟩, P.c⟩
  box b := ⟨⟨b.lo.x, b.lo.z, b.lo.y⟩, ⟨b.hi.x, b.hi.z, b.hi.y⟩⟩
  eval P x := by simp only [Plane.eval, V3.dot]; grind
  norm P := by simp only [V3.norm2, V3.dot]; grind
  proj S c := by
    have e : Plane.eval ⟨⟨S.n.x, S.n.z, S.n.y⟩, S.c⟩ ⟨c.x, c.z, c.y⟩ = S.eval c := by
      simp only [Plane.eval, V3.dot]; grind
    have e2 : V3.norm2 ⟨S.n.x, S.n.z, S.n.y⟩ = S.n.norm2 := by
      simp only [V3.norm2, V3.dot]; grind
    simp only [Snap.proj, e, e2]
    apply v3_ext <;> simp only [sub_x, sub_y, sub_z, smul_x, smul_y, smul_z]
  grow b l := by
    apply box_ext <;> apply v3_ext <;> simp only [Box.grow, sub_x, sub_y, sub_z, add_x, add_y, add_z]
  inter a b := by apply box_ext <;> apply v3_ext <;> simp only [Box.inter]
  nonempty R := by simp only [Box.nonempty]; grind
  corners R c := by
    rw [Box.mem_corners]
    constructor
    · intro h
      exact ⟨⟨c.x, c.z, c.y⟩, (Box.mem_corners _ _).mpr ⟨h.1, h.2.2, h.2.1⟩, rfl⟩
    · rintro ⟨c0, hc0, rfl⟩
      rw [Box.mem_corners] at hc0
      exact ⟨hc0.1, hc0.2.2, hc0.2.1⟩

end ForgeExact.Snap
