//! Generated from grammar.ll1 by tests/grammar.rs; do not edit. After a grammar change:
//! `FG_REGEN=1 cargo test -p forge-lang --test grammar`, then the actions in syntax.rs.
//! Terminals and nonterminals are numbered as research/forge-lang/lean/gen_grammar.py
//! numbers them, so NULLABLE, FIRST and FOLLOW are the sets the Lean check certifies.

pub const NT: usize = 51;
pub const NN: usize = 69;
pub const NP: usize = 155;
/// No rule: a syntax error on this lookahead.
pub const NONE: u8 = 255;
/// In a rule's right side, a symbol at or above this is a nonterminal.
pub const NT_BASE: u8 = 128;

pub mod t {
    pub const EOF: u8 = 0;
    pub const LBRACE: u8 = 1;
    pub const RBRACE: u8 = 2;
    pub const NL: u8 = 3;
    pub const SEMI: u8 = 4;
    pub const SHARED: u8 = 5;
    pub const ID: u8 = 6;
    pub const ASSIGN: u8 = 7;
    pub const PARAM: u8 = 8;
    pub const FN: u8 = 9;
    pub const LPAREN: u8 = 10;
    pub const RPAREN: u8 = 11;
    pub const FOR: u8 = 12;
    pub const IN: u8 = 13;
    pub const IF: u8 = 14;
    pub const RETURN: u8 = 15;
    pub const EXPECT: u8 = 16;
    pub const SKETCH: u8 = 17;
    pub const PATH: u8 = 18;
    pub const COMPONENT: u8 = 19;
    pub const DRAWING: u8 = 20;
    pub const HARNESS: u8 = 21;
    pub const DOT: u8 = 22;
    pub const LBRACK: u8 = 23;
    pub const RBRACK: u8 = 24;
    pub const NUM: u8 = 25;
    pub const COMMA: u8 = 26;
    pub const COLON: u8 = 27;
    pub const ELSE: u8 = 28;
    pub const LT: u8 = 29;
    pub const LE: u8 = 30;
    pub const GT: u8 = 31;
    pub const GE: u8 = 32;
    pub const PLUSMINUS: u8 = 33;
    pub const PERCENT: u8 = 34;
    pub const AT: u8 = 35;
    pub const OR: u8 = 36;
    pub const AND: u8 = 37;
    pub const NOT: u8 = 38;
    pub const EQEQ: u8 = 39;
    pub const NE: u8 = 40;
    pub const DOTDOT: u8 = 41;
    pub const DOTDOTEQ: u8 = 42;
    pub const PLUS: u8 = 43;
    pub const MINUS: u8 = 44;
    pub const STAR: u8 = 45;
    pub const SLASH: u8 = 46;
    pub const CARET: u8 = 47;
    pub const STR: u8 = 48;
    pub const TRUE: u8 = 49;
    pub const FALSE: u8 = 50;
}

/// A terminal as written: the quoted text, or its class name.
pub const TERMS: [&str; NT] = [
    "EOF",
    "{",
    "}",
    "NL",
    ";",
    "shared",
    "ID",
    "=",
    "param",
    "fn",
    "(",
    ")",
    "for",
    "in",
    "if",
    "return",
    "expect",
    "sketch",
    "path",
    "component",
    "drawing",
    "harness",
    ".",
    "[",
    "]",
    "NUM",
    ",",
    ":",
    "else",
    "<",
    "<=",
    ">",
    ">=",
    "+-",
    "%",
    "at",
    "or",
    "and",
    "not",
    "==",
    "!=",
    "..",
    "..=",
    "+",
    "-",
    "*",
    "/",
    "^",
    "STR",
    "true",
    "false",
];

pub const NONTERMS: [&str; NN] = [
    "program",
    "block",
    "stmts",
    "stmtsrest",
    "seps",
    "sep",
    "stmt",
    "assigntail",
    "drawtail",
    "dargs",
    "harntail",
    "path",
    "pathrest",
    "keys",
    "key",
    "keyrest",
    "fparams",
    "fparamrest",
    "fparam",
    "fptype",
    "fpdefault",
    "forvars",
    "elsepart",
    "elsebody",
    "expargs",
    "exprs",
    "exprsrest",
    "expcmp",
    "cmpop",
    "exptol",
    "pct",
    "expat",
    "expmore",
    "atpath",
    "args",
    "argsrest",
    "arg",
    "argkw",
    "expr",
    "ifpart",
    "or",
    "orrest",
    "and",
    "andrest",
    "not",
    "cmp",
    "cmprest",
    "relop",
    "range",
    "rangetail",
    "add",
    "addrest",
    "mul",
    "mulrest",
    "unary",
    "power",
    "powtail",
    "postfix",
    "postrest",
    "atom",
    "tuple",
    "tuplerest",
    "listbody",
    "listtail",
    "listmore",
    "listrest",
    "compif",
    "mapbody",
    "maprest",
];

/// A rule, named by its left side and the first symbol of its right side.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum P {
    /// program := seps stmts EOF
    ProgramSeps,
    /// block := '{' seps stmts '}'
    BlockLbrace,
    /// stmts := stmt stmtsrest
    StmtsStmt,
    /// stmts := %empty
    StmtsEmpty,
    /// stmtsrest := sep seps stmts
    StmtsrestSep,
    /// stmtsrest := %empty
    StmtsrestEmpty,
    /// seps := sep seps
    SepsSep,
    /// seps := %empty
    SepsEmpty,
    /// sep := NL
    SepNl,
    /// sep := ';'
    SepSemi,
    /// stmt := 'shared' ID '=' expr
    StmtShared,
    /// stmt := 'param' path '=' expr
    StmtParam,
    /// stmt := 'fn' ID '(' fparams ')' block
    StmtFn,
    /// stmt := 'for' ID forvars 'in' expr block
    StmtFor,
    /// stmt := 'if' expr block elsepart
    StmtIf,
    /// stmt := 'return' expr
    StmtReturn,
    /// stmt := 'expect' ID expargs expcmp expat
    StmtExpect,
    /// stmt := 'sketch' ID '(' args ')' block
    StmtSketch,
    /// stmt := 'path' ID '(' args ')' block
    StmtPath,
    /// stmt := 'component' ID '(' args ')' block
    StmtComponent,
    /// stmt := 'drawing' drawtail
    StmtDrawing,
    /// stmt := 'harness' harntail
    StmtHarness,
    /// stmt := expr assigntail
    StmtExpr,
    /// assigntail := '=' expr
    AssigntailAssign,
    /// assigntail := %empty
    AssigntailEmpty,
    /// drawtail := ID dargs block
    DrawtailId,
    /// drawtail := %empty
    DrawtailEmpty,
    /// dargs := '(' args ')'
    DargsLparen,
    /// dargs := %empty
    DargsEmpty,
    /// harntail := ID dargs block
    HarntailId,
    /// harntail := '(' args ')'
    HarntailLparen,
    /// path := ID keys pathrest
    PathId,
    /// pathrest := '.' ID keys pathrest
    PathrestDot,
    /// pathrest := %empty
    PathrestEmpty,
    /// keys := '[' key keyrest ']'
    KeysLbrack,
    /// keys := %empty
    KeysEmpty,
    /// key := NUM
    KeyNum,
    /// key := ID
    KeyId,
    /// keyrest := ',' key keyrest
    KeyrestComma,
    /// keyrest := %empty
    KeyrestEmpty,
    /// fparams := fparam fparamrest
    FparamsFparam,
    /// fparams := %empty
    FparamsEmpty,
    /// fparamrest := ',' fparams
    FparamrestComma,
    /// fparamrest := %empty
    FparamrestEmpty,
    /// fparam := ID fptype fpdefault
    FparamId,
    /// fptype := ':' ID
    FptypeColon,
    /// fptype := %empty
    FptypeEmpty,
    /// fpdefault := '=' expr
    FpdefaultAssign,
    /// fpdefault := %empty
    FpdefaultEmpty,
    /// forvars := ',' ID forvars
    ForvarsComma,
    /// forvars := %empty
    ForvarsEmpty,
    /// elsepart := 'else' elsebody
    ElsepartElse,
    /// elsepart := %empty
    ElsepartEmpty,
    /// elsebody := block
    ElsebodyBlock,
    /// elsebody := 'if' expr block elsepart
    ElsebodyIf,
    /// expargs := '(' exprs ')'
    ExpargsLparen,
    /// expargs := %empty
    ExpargsEmpty,
    /// exprs := expr exprsrest
    ExprsExpr,
    /// exprs := %empty
    ExprsEmpty,
    /// exprsrest := ',' exprs
    ExprsrestComma,
    /// exprsrest := %empty
    ExprsrestEmpty,
    /// expcmp := cmpop expr exptol
    ExpcmpCmpop,
    /// expcmp := %empty
    ExpcmpEmpty,
    /// cmpop := '='
    CmpopAssign,
    /// cmpop := '<'
    CmpopLt,
    /// cmpop := '<='
    CmpopLe,
    /// cmpop := '>'
    CmpopGt,
    /// cmpop := '>='
    CmpopGe,
    /// exptol := '+-' unary pct
    ExptolPlusminus,
    /// exptol := %empty
    ExptolEmpty,
    /// pct := '%'
    PctPercent,
    /// pct := %empty
    PctEmpty,
    /// expat := 'at' atpath '=' expr expmore
    ExpatAt,
    /// expat := %empty
    ExpatEmpty,
    /// expmore := ',' atpath '=' expr expmore
    ExpmoreComma,
    /// expmore := %empty
    ExpmoreEmpty,
    /// atpath := ID keys pathrest
    AtpathId,
    /// args := arg argsrest
    ArgsArg,
    /// args := %empty
    ArgsEmpty,
    /// argsrest := ',' args
    ArgsrestComma,
    /// argsrest := %empty
    ArgsrestEmpty,
    /// arg := expr argkw
    ArgExpr,
    /// arg := 'at' ':' expr
    ArgAt,
    /// arg := 'component' ':' expr
    ArgComponent,
    /// argkw := ':' expr
    ArgkwColon,
    /// argkw := %empty
    ArgkwEmpty,
    /// expr := or ifpart
    ExprOr,
    /// ifpart := 'if' or 'else' expr
    IfpartIf,
    /// ifpart := %empty
    IfpartEmpty,
    /// or := and orrest
    OrAnd,
    /// orrest := 'or' and orrest
    OrrestOr,
    /// orrest := %empty
    OrrestEmpty,
    /// and := not andrest
    AndNot,
    /// andrest := 'and' not andrest
    AndrestAnd,
    /// andrest := %empty
    AndrestEmpty,
    /// not := 'not' not
    NotNot,
    /// not := cmp
    NotCmp,
    /// cmp := range cmprest
    CmpRange,
    /// cmprest := relop range cmprest
    CmprestRelop,
    /// cmprest := %empty
    CmprestEmpty,
    /// relop := '=='
    RelopEqeq,
    /// relop := '!='
    RelopNe,
    /// relop := '<'
    RelopLt,
    /// relop := '<='
    RelopLe,
    /// relop := '>'
    RelopGt,
    /// relop := '>='
    RelopGe,
    /// range := add rangetail
    RangeAdd,
    /// rangetail := '..' add
    RangetailDotdot,
    /// rangetail := '..=' add
    RangetailDotdoteq,
    /// rangetail := %empty
    RangetailEmpty,
    /// add := mul addrest
    AddMul,
    /// addrest := '+' mul addrest
    AddrestPlus,
    /// addrest := '-' mul addrest
    AddrestMinus,
    /// addrest := %empty
    AddrestEmpty,
    /// mul := unary mulrest
    MulUnary,
    /// mulrest := '*' unary mulrest
    MulrestStar,
    /// mulrest := '/' unary mulrest
    MulrestSlash,
    /// mulrest := %empty
    MulrestEmpty,
    /// unary := '-' unary
    UnaryMinus,
    /// unary := power
    UnaryPower,
    /// power := postfix powtail
    PowerPostfix,
    /// powtail := '^' unary
    PowtailCaret,
    /// powtail := %empty
    PowtailEmpty,
    /// postfix := atom postrest
    PostfixAtom,
    /// postrest := '(' args ')' postrest
    PostrestLparen,
    /// postrest := '.' ID postrest
    PostrestDot,
    /// postrest := '[' expr ']' postrest
    PostrestLbrack,
    /// postrest := %empty
    PostrestEmpty,
    /// atom := NUM
    AtomNum,
    /// atom := STR
    AtomStr,
    /// atom := ID
    AtomId,
    /// atom := 'true'
    AtomTrue,
    /// atom := 'false'
    AtomFalse,
    /// atom := '(' tuple ')'
    AtomLparen,
    /// atom := '[' listbody ']'
    AtomLbrack,
    /// atom := '{' seps mapbody '}'
    AtomLbrace,
    /// tuple := expr tuplerest
    TupleExpr,
    /// tuple := %empty
    TupleEmpty,
    /// tuplerest := ',' tuple
    TuplerestComma,
    /// tuplerest := %empty
    TuplerestEmpty,
    /// listbody := expr listtail
    ListbodyExpr,
    /// listbody := %empty
    ListbodyEmpty,
    /// listtail := 'for' ID forvars 'in' or compif
    ListtailFor,
    /// listtail := ',' listmore
    ListtailComma,
    /// listtail := %empty
    ListtailEmpty,
    /// listmore := expr listrest
    ListmoreExpr,
    /// listmore := %empty
    ListmoreEmpty,
    /// listrest := ',' listmore
    ListrestComma,
    /// listrest := %empty
    ListrestEmpty,
    /// compif := 'if' or
    CompifIf,
    /// compif := %empty
    CompifEmpty,
    /// mapbody := ID ':' expr seps maprest
    MapbodyId,
    /// mapbody := %empty
    MapbodyEmpty,
    /// maprest := ',' seps mapbody
    MaprestComma,
    /// maprest := %empty
    MaprestEmpty,
}

pub const RULES: [P; NP] = [
    P::ProgramSeps,
    P::BlockLbrace,
    P::StmtsStmt,
    P::StmtsEmpty,
    P::StmtsrestSep,
    P::StmtsrestEmpty,
    P::SepsSep,
    P::SepsEmpty,
    P::SepNl,
    P::SepSemi,
    P::StmtShared,
    P::StmtParam,
    P::StmtFn,
    P::StmtFor,
    P::StmtIf,
    P::StmtReturn,
    P::StmtExpect,
    P::StmtSketch,
    P::StmtPath,
    P::StmtComponent,
    P::StmtDrawing,
    P::StmtHarness,
    P::StmtExpr,
    P::AssigntailAssign,
    P::AssigntailEmpty,
    P::DrawtailId,
    P::DrawtailEmpty,
    P::DargsLparen,
    P::DargsEmpty,
    P::HarntailId,
    P::HarntailLparen,
    P::PathId,
    P::PathrestDot,
    P::PathrestEmpty,
    P::KeysLbrack,
    P::KeysEmpty,
    P::KeyNum,
    P::KeyId,
    P::KeyrestComma,
    P::KeyrestEmpty,
    P::FparamsFparam,
    P::FparamsEmpty,
    P::FparamrestComma,
    P::FparamrestEmpty,
    P::FparamId,
    P::FptypeColon,
    P::FptypeEmpty,
    P::FpdefaultAssign,
    P::FpdefaultEmpty,
    P::ForvarsComma,
    P::ForvarsEmpty,
    P::ElsepartElse,
    P::ElsepartEmpty,
    P::ElsebodyBlock,
    P::ElsebodyIf,
    P::ExpargsLparen,
    P::ExpargsEmpty,
    P::ExprsExpr,
    P::ExprsEmpty,
    P::ExprsrestComma,
    P::ExprsrestEmpty,
    P::ExpcmpCmpop,
    P::ExpcmpEmpty,
    P::CmpopAssign,
    P::CmpopLt,
    P::CmpopLe,
    P::CmpopGt,
    P::CmpopGe,
    P::ExptolPlusminus,
    P::ExptolEmpty,
    P::PctPercent,
    P::PctEmpty,
    P::ExpatAt,
    P::ExpatEmpty,
    P::ExpmoreComma,
    P::ExpmoreEmpty,
    P::AtpathId,
    P::ArgsArg,
    P::ArgsEmpty,
    P::ArgsrestComma,
    P::ArgsrestEmpty,
    P::ArgExpr,
    P::ArgAt,
    P::ArgComponent,
    P::ArgkwColon,
    P::ArgkwEmpty,
    P::ExprOr,
    P::IfpartIf,
    P::IfpartEmpty,
    P::OrAnd,
    P::OrrestOr,
    P::OrrestEmpty,
    P::AndNot,
    P::AndrestAnd,
    P::AndrestEmpty,
    P::NotNot,
    P::NotCmp,
    P::CmpRange,
    P::CmprestRelop,
    P::CmprestEmpty,
    P::RelopEqeq,
    P::RelopNe,
    P::RelopLt,
    P::RelopLe,
    P::RelopGt,
    P::RelopGe,
    P::RangeAdd,
    P::RangetailDotdot,
    P::RangetailDotdoteq,
    P::RangetailEmpty,
    P::AddMul,
    P::AddrestPlus,
    P::AddrestMinus,
    P::AddrestEmpty,
    P::MulUnary,
    P::MulrestStar,
    P::MulrestSlash,
    P::MulrestEmpty,
    P::UnaryMinus,
    P::UnaryPower,
    P::PowerPostfix,
    P::PowtailCaret,
    P::PowtailEmpty,
    P::PostfixAtom,
    P::PostrestLparen,
    P::PostrestDot,
    P::PostrestLbrack,
    P::PostrestEmpty,
    P::AtomNum,
    P::AtomStr,
    P::AtomId,
    P::AtomTrue,
    P::AtomFalse,
    P::AtomLparen,
    P::AtomLbrack,
    P::AtomLbrace,
    P::TupleExpr,
    P::TupleEmpty,
    P::TuplerestComma,
    P::TuplerestEmpty,
    P::ListbodyExpr,
    P::ListbodyEmpty,
    P::ListtailFor,
    P::ListtailComma,
    P::ListtailEmpty,
    P::ListmoreExpr,
    P::ListmoreEmpty,
    P::ListrestComma,
    P::ListrestEmpty,
    P::CompifIf,
    P::CompifEmpty,
    P::MapbodyId,
    P::MapbodyEmpty,
    P::MaprestComma,
    P::MaprestEmpty,
];

pub const LHS: [u8; NP] = [0, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 12, 12, 13, 13, 14, 14, 15, 15, 16, 16, 17, 17, 18, 19, 19, 20, 20, 21, 21, 22, 22, 23, 23, 24, 24, 25, 25, 26, 26, 27, 27, 28, 28, 28, 28, 28, 29, 29, 30, 30, 31, 31, 32, 32, 33, 34, 34, 35, 35, 36, 36, 36, 37, 37, 38, 39, 39, 40, 41, 41, 42, 43, 43, 44, 44, 45, 46, 46, 47, 47, 47, 47, 47, 47, 48, 49, 49, 49, 50, 51, 51, 51, 52, 53, 53, 53, 54, 54, 55, 56, 56, 57, 58, 58, 58, 58, 59, 59, 59, 59, 59, 59, 59, 59, 60, 60, 61, 61, 62, 62, 63, 63, 63, 64, 64, 65, 65, 66, 66, 67, 67, 68, 68];

pub const RHS: [&[u8]; NP] = [
    &[132, 130, 0],
    &[1, 132, 130, 2],
    &[134, 131],
    &[],
    &[133, 132, 130],
    &[],
    &[133, 132],
    &[],
    &[3],
    &[4],
    &[5, 6, 7, 166],
    &[8, 139, 7, 166],
    &[9, 6, 10, 144, 11, 129],
    &[12, 6, 149, 13, 166, 129],
    &[14, 166, 129, 150],
    &[15, 166],
    &[16, 6, 152, 155, 159],
    &[17, 6, 10, 162, 11, 129],
    &[18, 6, 10, 162, 11, 129],
    &[19, 6, 10, 162, 11, 129],
    &[20, 136],
    &[21, 138],
    &[166, 135],
    &[7, 166],
    &[],
    &[6, 137, 129],
    &[],
    &[10, 162, 11],
    &[],
    &[6, 137, 129],
    &[10, 162, 11],
    &[6, 141, 140],
    &[22, 6, 141, 140],
    &[],
    &[23, 142, 143, 24],
    &[],
    &[25],
    &[6],
    &[26, 142, 143],
    &[],
    &[146, 145],
    &[],
    &[26, 144],
    &[],
    &[6, 147, 148],
    &[27, 6],
    &[],
    &[7, 166],
    &[],
    &[26, 6, 149],
    &[],
    &[28, 151],
    &[],
    &[129],
    &[14, 166, 129, 150],
    &[10, 153, 11],
    &[],
    &[166, 154],
    &[],
    &[26, 153],
    &[],
    &[156, 166, 157],
    &[],
    &[7],
    &[29],
    &[30],
    &[31],
    &[32],
    &[33, 182, 158],
    &[],
    &[34],
    &[],
    &[35, 161, 7, 166, 160],
    &[],
    &[26, 161, 7, 166, 160],
    &[],
    &[6, 141, 140],
    &[164, 163],
    &[],
    &[26, 162],
    &[],
    &[166, 165],
    &[35, 27, 166],
    &[19, 27, 166],
    &[27, 166],
    &[],
    &[168, 167],
    &[14, 168, 28, 166],
    &[],
    &[170, 169],
    &[36, 170, 169],
    &[],
    &[172, 171],
    &[37, 172, 171],
    &[],
    &[38, 172],
    &[173],
    &[176, 174],
    &[175, 176, 174],
    &[],
    &[39],
    &[40],
    &[29],
    &[30],
    &[31],
    &[32],
    &[178, 177],
    &[41, 178],
    &[42, 178],
    &[],
    &[180, 179],
    &[43, 180, 179],
    &[44, 180, 179],
    &[],
    &[182, 181],
    &[45, 182, 181],
    &[46, 182, 181],
    &[],
    &[44, 182],
    &[183],
    &[185, 184],
    &[47, 182],
    &[],
    &[187, 186],
    &[10, 162, 11, 186],
    &[22, 6, 186],
    &[23, 166, 24, 186],
    &[],
    &[25],
    &[48],
    &[6],
    &[49],
    &[50],
    &[10, 188, 11],
    &[23, 190, 24],
    &[1, 132, 195, 2],
    &[166, 189],
    &[],
    &[26, 188],
    &[],
    &[166, 191],
    &[],
    &[12, 6, 149, 13, 168, 194],
    &[26, 192],
    &[],
    &[166, 193],
    &[],
    &[26, 192],
    &[],
    &[14, 168],
    &[],
    &[6, 27, 166, 132, 196],
    &[],
    &[26, 132, 195],
    &[],
];

pub const NULLABLE: [bool; NN] = [false, false, true, true, true, false, false, true, true, true, false, false, true, true, false, true, true, true, false, true, true, true, true, false, true, true, true, true, false, true, true, true, true, false, true, true, false, true, false, true, false, true, false, true, false, false, true, false, false, true, false, true, false, true, false, false, true, false, true, false, true, true, true, true, true, true, true, true, true];

/// Bit k: terminal k.
pub const FIRST: [u128; NN] = [
    0x7104002bfd77b,
    0x2,
    0x7104002bfd762,
    0x18,
    0x18,
    0x18,
    0x7104002bfd762,
    0x80,
    0x40,
    0x400,
    0x440,
    0x40,
    0x400000,
    0x800000,
    0x2000040,
    0x4000000,
    0x40,
    0x4000000,
    0x40,
    0x8000000,
    0x80,
    0x4000000,
    0x10000000,
    0x4002,
    0x400,
    0x7104002800442,
    0x4000000,
    0x1e0000080,
    0x1e0000080,
    0x200000000,
    0x400000000,
    0x800000000,
    0x4000000,
    0x40,
    0x7104802880442,
    0x4000000,
    0x7104802880442,
    0x8000000,
    0x7104002800442,
    0x4000,
    0x7104002800442,
    0x1000000000,
    0x7104002800442,
    0x2000000000,
    0x7104002800442,
    0x7100002800442,
    0x181e0000000,
    0x181e0000000,
    0x7100002800442,
    0x60000000000,
    0x7100002800442,
    0x180000000000,
    0x7100002800442,
    0x600000000000,
    0x7100002800442,
    0x7000002800442,
    0x800000000000,
    0x7000002800442,
    0xc00400,
    0x7000002800442,
    0x7104002800442,
    0x4000000,
    0x7104002800442,
    0x4001000,
    0x7104002800442,
    0x4000000,
    0x4000,
    0x40,
    0x4000000,
];

/// Bit k: terminal k; FOLLOW_END: the end marker may follow (the start symbol only).
pub const FOLLOW: [u128; NN] = [
    0x0,
    0x1000001d,
    0x5,
    0x5,
    0x7104006bfd767,
    0x7104006bfd77f,
    0x1d,
    0x1d,
    0x1d,
    0x2,
    0x1d,
    0x80,
    0x80,
    0x400080,
    0x5000000,
    0x1000000,
    0x800,
    0x800,
    0x4000800,
    0x4000880,
    0x4000800,
    0x2000,
    0x1d,
    0x1d,
    0x9e000009d,
    0x800,
    0x800,
    0x80000001d,
    0x7104002800442,
    0x80000001d,
    0x80000001d,
    0x1d,
    0x1d,
    0x80,
    0x800,
    0x800,
    0x4000800,
    0x4000800,
    0xa0d00189f,
    0xa0d00189f,
    0xa1d00589f,
    0xa1d00589f,
    0x1a1d00589f,
    0x1a1d00589f,
    0x3a1d00589f,
    0x3a1d00589f,
    0x3a1d00589f,
    0x7100002800442,
    0x1bbfd00589f,
    0x1bbfd00589f,
    0x7bbfd00589f,
    0x7bbfd00589f,
    0x1fbbfd00589f,
    0x1fbbfd00589f,
    0x7fbffd00589f,
    0x7fbffd00589f,
    0x7fbffd00589f,
    0xffbffd00589f,
    0xffbffd00589f,
    0xffbffdc05c9f,
    0x800,
    0x800,
    0x1000000,
    0x1000000,
    0x1000000,
    0x1000000,
    0x1000000,
    0x4,
    0x4,
];
pub const FOLLOW_END: [bool; NN] = [true, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false];

/// TABLE[nonterminal][lookahead]: the rule to expand by, or NONE.
pub const TABLE: [[u8; NT]; NN] = [
    [0, 0, NONE, 0, 0, 0, 0, NONE, 0, 0, 0, NONE, 0, NONE, 0, 0, 0, 0, 0, 0, 0, 0, NONE, 0, NONE, 0, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 0, NONE, NONE, NONE, NONE, NONE, 0, NONE, NONE, NONE, 0, 0, 0],
    [NONE, 1, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [3, 2, 3, NONE, NONE, 2, 2, NONE, 2, 2, 2, NONE, 2, NONE, 2, 2, 2, 2, 2, 2, 2, 2, NONE, 2, NONE, 2, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 2, NONE, NONE, NONE, NONE, NONE, 2, NONE, NONE, NONE, 2, 2, 2],
    [5, NONE, 5, 4, 4, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [7, 7, 7, 6, 6, 7, 7, NONE, 7, 7, 7, NONE, 7, NONE, 7, 7, 7, 7, 7, 7, 7, 7, NONE, 7, NONE, 7, 7, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 7, NONE, NONE, NONE, NONE, NONE, 7, NONE, NONE, NONE, 7, 7, 7],
    [NONE, NONE, NONE, 8, 9, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, 22, NONE, NONE, NONE, 10, 22, NONE, 11, 12, 22, NONE, 13, NONE, 14, 15, 16, 17, 18, 19, 20, 21, NONE, 22, NONE, 22, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 22, NONE, NONE, NONE, NONE, NONE, 22, NONE, NONE, NONE, 22, 22, 22],
    [24, NONE, 24, 24, 24, NONE, NONE, 23, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [26, NONE, 26, 26, 26, NONE, 25, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, 28, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 27, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, NONE, NONE, NONE, NONE, NONE, 29, NONE, NONE, NONE, 30, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, NONE, NONE, NONE, NONE, NONE, 31, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, NONE, NONE, NONE, NONE, NONE, NONE, 33, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 32, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, NONE, NONE, NONE, NONE, NONE, NONE, 35, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 35, 34, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, NONE, NONE, NONE, NONE, NONE, 37, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 36, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 39, NONE, 38, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, NONE, NONE, NONE, NONE, NONE, 40, NONE, NONE, NONE, NONE, 41, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 43, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 42, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, NONE, NONE, NONE, NONE, NONE, 44, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, NONE, NONE, NONE, NONE, NONE, NONE, 46, NONE, NONE, NONE, 46, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 46, 45, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, NONE, NONE, NONE, NONE, NONE, NONE, 47, NONE, NONE, NONE, 48, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 48, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 50, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 49, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [52, NONE, 52, 52, 52, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 51, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, 53, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 54, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [56, NONE, 56, 56, 56, NONE, NONE, 56, NONE, NONE, 55, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 56, 56, 56, 56, NONE, NONE, 56, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, 57, NONE, NONE, NONE, NONE, 57, NONE, NONE, NONE, 57, 58, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 57, NONE, 57, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 57, NONE, NONE, NONE, NONE, NONE, 57, NONE, NONE, NONE, 57, 57, 57],
    [NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 60, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 59, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [62, NONE, 62, 62, 62, NONE, NONE, 61, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 61, 61, 61, 61, NONE, NONE, 62, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, NONE, NONE, NONE, NONE, NONE, NONE, 63, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 64, 65, 66, 67, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [69, NONE, 69, 69, 69, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 68, NONE, 69, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [71, NONE, 71, 71, 71, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 70, 71, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [73, NONE, 73, 73, 73, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 72, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [75, NONE, 75, 75, 75, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 74, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, NONE, NONE, NONE, NONE, NONE, 76, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, 77, NONE, NONE, NONE, NONE, 77, NONE, NONE, NONE, 77, 78, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 77, NONE, NONE, NONE, 77, NONE, 77, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 77, NONE, NONE, 77, NONE, NONE, NONE, NONE, NONE, 77, NONE, NONE, NONE, 77, 77, 77],
    [NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 80, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 79, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, 81, NONE, NONE, NONE, NONE, 81, NONE, NONE, NONE, 81, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 83, NONE, NONE, NONE, 81, NONE, 81, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 82, NONE, NONE, 81, NONE, NONE, NONE, NONE, NONE, 81, NONE, NONE, NONE, 81, 81, 81],
    [NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 85, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 85, 84, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, 86, NONE, NONE, NONE, NONE, 86, NONE, NONE, NONE, 86, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 86, NONE, 86, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 86, NONE, NONE, NONE, NONE, NONE, 86, NONE, NONE, NONE, 86, 86, 86],
    [88, 88, 88, 88, 88, NONE, NONE, 88, NONE, NONE, NONE, 88, 88, NONE, 87, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 88, NONE, 88, 88, NONE, NONE, NONE, NONE, NONE, 88, NONE, 88, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, 89, NONE, NONE, NONE, NONE, 89, NONE, NONE, NONE, 89, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 89, NONE, 89, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 89, NONE, NONE, NONE, NONE, NONE, 89, NONE, NONE, NONE, 89, 89, 89],
    [91, 91, 91, 91, 91, NONE, NONE, 91, NONE, NONE, NONE, 91, 91, NONE, 91, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 91, NONE, 91, 91, 91, NONE, NONE, NONE, NONE, 91, NONE, 91, 90, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, 92, NONE, NONE, NONE, NONE, 92, NONE, NONE, NONE, 92, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 92, NONE, 92, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 92, NONE, NONE, NONE, NONE, NONE, 92, NONE, NONE, NONE, 92, 92, 92],
    [94, 94, 94, 94, 94, NONE, NONE, 94, NONE, NONE, NONE, 94, 94, NONE, 94, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 94, NONE, 94, 94, 94, NONE, NONE, NONE, NONE, 94, NONE, 94, 94, 93, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, 96, NONE, NONE, NONE, NONE, 96, NONE, NONE, NONE, 96, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 96, NONE, 96, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 95, NONE, NONE, NONE, NONE, NONE, 96, NONE, NONE, NONE, 96, 96, 96],
    [NONE, 97, NONE, NONE, NONE, NONE, 97, NONE, NONE, NONE, 97, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 97, NONE, 97, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 97, NONE, NONE, NONE, 97, 97, 97],
    [99, 99, 99, 99, 99, NONE, NONE, 99, NONE, NONE, NONE, 99, 99, NONE, 99, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 99, NONE, 99, 99, 99, 98, 98, 98, 98, 99, NONE, 99, 99, 99, NONE, 98, 98, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 102, 103, 104, 105, NONE, NONE, NONE, NONE, NONE, NONE, 100, 101, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, 106, NONE, NONE, NONE, NONE, 106, NONE, NONE, NONE, 106, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 106, NONE, 106, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 106, NONE, NONE, NONE, 106, 106, 106],
    [109, 109, 109, 109, 109, NONE, NONE, 109, NONE, NONE, NONE, 109, 109, NONE, 109, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 109, NONE, 109, 109, 109, 109, 109, 109, 109, 109, NONE, 109, 109, 109, NONE, 109, 109, 107, 108, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, 110, NONE, NONE, NONE, NONE, 110, NONE, NONE, NONE, 110, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 110, NONE, 110, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 110, NONE, NONE, NONE, 110, 110, 110],
    [113, 113, 113, 113, 113, NONE, NONE, 113, NONE, NONE, NONE, 113, 113, NONE, 113, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 113, NONE, 113, 113, 113, 113, 113, 113, 113, 113, NONE, 113, 113, 113, NONE, 113, 113, 113, 113, 111, 112, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, 114, NONE, NONE, NONE, NONE, 114, NONE, NONE, NONE, 114, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 114, NONE, 114, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 114, NONE, NONE, NONE, 114, 114, 114],
    [117, 117, 117, 117, 117, NONE, NONE, 117, NONE, NONE, NONE, 117, 117, NONE, 117, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 117, NONE, 117, 117, 117, 117, 117, 117, 117, 117, NONE, 117, 117, 117, NONE, 117, 117, 117, 117, 117, 117, 115, 116, NONE, NONE, NONE, NONE],
    [NONE, 119, NONE, NONE, NONE, NONE, 119, NONE, NONE, NONE, 119, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 119, NONE, 119, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 118, NONE, NONE, NONE, 119, 119, 119],
    [NONE, 120, NONE, NONE, NONE, NONE, 120, NONE, NONE, NONE, 120, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 120, NONE, 120, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 120, 120, 120],
    [122, 122, 122, 122, 122, NONE, NONE, 122, NONE, NONE, NONE, 122, 122, NONE, 122, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 122, NONE, 122, 122, 122, 122, 122, 122, 122, 122, 122, 122, 122, 122, NONE, 122, 122, 122, 122, 122, 122, 122, 122, 121, NONE, NONE, NONE],
    [NONE, 123, NONE, NONE, NONE, NONE, 123, NONE, NONE, NONE, 123, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 123, NONE, 123, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 123, 123, 123],
    [127, 127, 127, 127, 127, NONE, NONE, 127, NONE, NONE, 124, 127, 127, NONE, 127, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 125, 126, 127, NONE, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, NONE, 127, 127, 127, 127, 127, 127, 127, 127, 127, NONE, NONE, NONE],
    [NONE, 135, NONE, NONE, NONE, NONE, 130, NONE, NONE, NONE, 133, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 134, NONE, 128, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 129, 131, 132],
    [NONE, 136, NONE, NONE, NONE, NONE, 136, NONE, NONE, NONE, 136, 137, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 136, NONE, 136, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 136, NONE, NONE, NONE, NONE, NONE, 136, NONE, NONE, NONE, 136, 136, 136],
    [NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 139, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 138, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, 140, NONE, NONE, NONE, NONE, 140, NONE, NONE, NONE, 140, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 140, 141, 140, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 140, NONE, NONE, NONE, NONE, NONE, 140, NONE, NONE, NONE, 140, 140, 140],
    [NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 142, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 144, NONE, 143, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, 145, NONE, NONE, NONE, NONE, 145, NONE, NONE, NONE, 145, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 145, 146, 145, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 145, NONE, NONE, NONE, NONE, NONE, 145, NONE, NONE, NONE, 145, 145, 145],
    [NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 148, NONE, 147, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 149, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 150, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, NONE, 152, NONE, NONE, NONE, 151, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, NONE, 154, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 153, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
];

/// A nullable nonterminal's rule that derives the empty string, or NONE.
pub const EMPTY: [u8; NN] = [NONE, NONE, 3, 5, 7, NONE, NONE, 24, 26, 28, NONE, NONE, 33, 35, NONE, 39, 41, 43, NONE, 46, 48, 50, 52, NONE, 56, 58, 60, 62, NONE, 69, 71, 73, 75, NONE, 78, 80, NONE, 85, NONE, 88, NONE, 91, NONE, 94, NONE, NONE, 99, NONE, NONE, 109, NONE, 113, NONE, 117, NONE, NONE, 122, NONE, 127, NONE, 137, 139, 141, 144, 146, 148, 150, 152, 154];
