import ForgeLang.LL1.Impl
import ForgeLang.LL1.Driver

/-! `Driver.lean` on the parser's grammar and table.rs's sets (`Impl.lean`): the recursive descent
with the inherited FIRST set is the table parser, errors included, and what it accepts with no
miss is the proven parser's one tree. `tests/grammar.rs`'s `the_empty_rules_are_the_lean_ones`
ties `emptyRule` to table.rs's EMPTY, `Impl.table_choose` ties `choose` to TABLE. -/

namespace ForgeLang.LL1.Impl
open Driver

theorem impl_rd_eq_run (w : List Tok) (r : Res) :
    (∃ n, rd grammar certT n w = some r) ↔ (∃ n, run grammar certT n ([.n grammar.start], init w) = some r) :=
  rd_eq_run w r

theorem impl_rd_accepts {n : Nat} {w : List Tok} {st : St}
    (h : rdS grammar certT n (.n grammar.start) [] (init w) = some (.ok st)) (hm : st.miss = none) (hw : st.w = []) :
    ∃ t, parse grammar certT t.size (.n grammar.start) w = some (t, []) ∧ st.out = (preT t).reverse :=
  rd_accepts_parse grammar_ll1 h hm hw

end ForgeLang.LL1.Impl
