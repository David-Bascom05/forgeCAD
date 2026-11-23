import ForgeLang.Builtins

/-!
# The `scale` motion: adding its name changes no document that binds none of it

`scale(about: POINT, by: k)` is a new built-in, a motion of `movebody`/`copybody` like `slide`, `spin`
and `place` (forge-lang's `wired::NAMES`). By `Builtins.extend_conservative` adding it can only
refuse a document that binds `scale` (`ShadowsBuiltin`) or give a meaning to a name `scale` read
unbound (before, `Unknown`: such a document did not run). `scale_conservative` is that theorem for
it; forge-lang's test `scale::scale_bound_nowhere` holds every `.fg` file in the repository to
binding no `scale` (its uses there are a drawing view's `scale:` argument label, which is not a
name read or bound), so every part means what it meant.

The name changes no grammar: `scale_not_keyword` checks that it is not one of the lexer's keywords
(`Impl.keywords`, the list the proven LL(1) parser is built on), so it lexes as an identifier, and
`parse_diff`'s digest is unchanged.
-/

namespace ForgeLang
namespace Builtins

/-- The uniform-scale motion's built-in. -/
def scaleNames : List Name := ["scale"]

/-- `extend_conservative` for `scale` added to any list of built-ins. -/
theorem scale_conservative {α : Type} (B : List Name) (binds : List Name) (ρ : List (Name × α))
    (h : ∀ x ∈ binds, x ∉ scaleNames) :
    shadows (B ++ scaleNames) binds = shadows B binds ∧
      (∀ x v, ρ.lookup x = some v → resolve (B ++ scaleNames) ρ x = resolve B ρ x) ∧
      (∀ x, x ∉ scaleNames → resolve (B ++ scaleNames) ρ x = resolve B ρ x) :=
  extend_conservative B scaleNames binds ρ h

/-- The only new refusal: a document that binds `scale`. -/
theorem scale_refuses_only_its_binding (B binds : List Name) (hb : "scale" ∈ binds) :
    "scale" ∈ shadows (B ++ scaleNames) binds :=
  shadows_new B scaleNames binds "scale" hb (by decide)

/-- `scale` is no keyword: it lexes as an identifier. -/
theorem scale_not_keyword : ∀ n ∈ scaleNames, codes n ∉ LL1.Impl.keywords := by
  decide

end Builtins
end ForgeLang
