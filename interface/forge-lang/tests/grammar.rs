//! The grammar in grammar.ll1 is LL(1): at every choice, the alternatives' first tokens (and,
//! where one can be empty, the tokens that may follow) are disjoint. An LL(1) grammar is
//! unambiguous, so every text has one tree.
//!
//! The parser's table, src/syntax/table.rs, is generated from grammar.ll1 here: nullable, FIRST
//! and FOLLOW numbered as research/forge-lang/lean/gen_grammar.py numbers them (the sets the Lean
//! check certifies) and the LL(1) table built from them. `the_table_is_current` fails when the file
//! is stale; `FG_REGEN=1 cargo test -p forge-lang --test grammar` writes it.

mod ll1;

use ll1::{Grammar, generate, read};
use std::collections::BTreeSet;

const TABLE_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/src/syntax/table.rs");

fn forge() -> Grammar {
    read(&std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/grammar.ll1")).expect("grammar"))
}

#[test]
fn the_grammar_is_ll1() {
    let g = forge();
    let conflicts = g.conflicts();
    assert!(conflicts.is_empty(), "LL(1) conflicts:\n{}", conflicts.join("\n"));
    assert!(g.nts.len() > 50);
}

#[test]
fn the_table_is_current() {
    let want = generate(&forge());
    if std::env::var_os("FG_REGEN").is_some() {
        std::fs::write(TABLE_PATH, &want).expect("write table");
        return;
    }
    let have = std::fs::read_to_string(TABLE_PATH).unwrap_or_default();
    assert!(
        have == want,
        "src/syntax/table.rs is stale: FG_REGEN=1 cargo test -p forge-lang --test grammar"
    );
}

/// The table the parser runs is the one built from the grammar's sets, and every terminal is a
/// token the lexer makes.
#[test]
fn the_parser_runs_the_generated_table() {
    use forge_lang::syntax::{KEYWORDS, OPS, table};
    let g = forge();
    assert_eq!(table::TERMS.len(), g.terms.len());
    let lexed: BTreeSet<&str> = KEYWORDS.iter().chain(OPS.iter()).copied().collect();
    let quoted: BTreeSet<&str> = table::TERMS
        .iter()
        .copied()
        .filter(|t| !["NUM", "STR", "ID", "NL", "EOF"].contains(t))
        .collect();
    assert_eq!(
        lexed, quoted,
        "the lexer's keywords and operators are the grammar's quoted terminals"
    );
    let tbl = g.table();
    for (a, row) in tbl.iter().enumerate() {
        for (x, c) in row.iter().enumerate() {
            assert_eq!(table::TABLE[a][x], c.map_or(table::NONE, |p| p as u8));
        }
    }
}

/// The parser accepts the grammar's language: every random sentence of grammar.ll1 parses, or
/// fails one of the checks the actions make beyond the grammar, never on syntax.
#[test]
fn every_sentence_of_the_grammar_parses() {
    use forge_lang::error::{Error, SyntaxError};
    let g = forge();
    let minlen = ll1::min_lengths(&g);
    let mut rng = ll1::Rng(0x0123_4567_89ab_cdef);
    let mut parsed = 0;
    for _ in 0..5000 {
        let s = ll1::sentence(&g, &mut rng, &minlen);
        match forge_lang::syntax::parse(&s) {
            Ok(_) => parsed += 1,
            Err(Error::Syntax(
                SyntaxError::AssignToNonName { .. }
                | SyntaxError::KeywordNotName { .. }
                | SyntaxError::PositionalAfterKeyword { .. }
                | SyntaxError::ParamWithoutOwner { .. }
                | SyntaxError::Expected {
                    what: "an instance index",
                    ..
                },
            )) => {}
            Err(e) => panic!("a sentence of the grammar did not parse: {e:?}\n{s:?}"),
        }
    }
    assert!(parsed > 3000, "only {parsed} parsed");
}
