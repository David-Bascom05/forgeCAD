//! Generated from grammar.ll1 by tests/grammar.rs; do not edit. After a grammar change:
//! `FG_REGEN=1 cargo test -p forge-lang --test grammar`, then the actions in syntax.rs.
//! Terminals and nonterminals are numbered as research/forge-lang/lean/gen_grammar.py
//! numbers them, so NULLABLE, FIRST and FOLLOW are the sets the Lean check certifies.

pub const NT: usize = 47;
pub const NN: usize = 63;
pub const NP: usize = 139;
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
    pub const DOT: u8 = 19;
    pub const LBRACK: u8 = 20;
    pub const RBRACK: u8 = 21;
    pub const NUM: u8 = 22;
    pub const COMMA: u8 = 23;
    pub const COLON: u8 = 24;
    pub const ELSE: u8 = 25;
    pub const LT: u8 = 26;
    pub const LE: u8 = 27;
    pub const GT: u8 = 28;
    pub const GE: u8 = 29;
    pub const PLUSMINUS: u8 = 30;
    pub const PERCENT: u8 = 31;
    pub const OR: u8 = 32;
    pub const AND: u8 = 33;
    pub const NOT: u8 = 34;
    pub const EQEQ: u8 = 35;
    pub const NE: u8 = 36;
    pub const DOTDOT: u8 = 37;
    pub const DOTDOTEQ: u8 = 38;
    pub const PLUS: u8 = 39;
    pub const MINUS: u8 = 40;
    pub const STAR: u8 = 41;
    pub const SLASH: u8 = 42;
    pub const CARET: u8 = 43;
    pub const STR: u8 = 44;
    pub const TRUE: u8 = 45;
    pub const FALSE: u8 = 46;
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
    /// stmt := 'expect' ID expargs expcmp
    StmtExpect,
    /// stmt := 'sketch' ID '(' args ')' block
    StmtSketch,
    /// stmt := 'path' ID '(' args ')' block
    StmtPath,
    /// stmt := expr assigntail
    StmtExpr,
    /// assigntail := '=' expr
    AssigntailAssign,
    /// assigntail := %empty
    AssigntailEmpty,
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
    P::StmtExpr,
    P::AssigntailAssign,
    P::AssigntailEmpty,
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
    P::ArgsArg,
    P::ArgsEmpty,
    P::ArgsrestComma,
    P::ArgsrestEmpty,
    P::ArgExpr,
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

pub const LHS: [u8; NP] = [0, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 7, 7, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13, 13, 14, 14, 15, 16, 16, 17, 17, 18, 18, 19, 19, 20, 20, 21, 21, 22, 22, 23, 23, 24, 24, 25, 25, 25, 25, 25, 26, 26, 27, 27, 28, 28, 29, 29, 30, 31, 31, 32, 33, 33, 34, 35, 35, 36, 37, 37, 38, 38, 39, 40, 40, 41, 41, 41, 41, 41, 41, 42, 43, 43, 43, 44, 45, 45, 45, 46, 47, 47, 47, 48, 48, 49, 50, 50, 51, 52, 52, 52, 52, 53, 53, 53, 53, 53, 53, 53, 53, 54, 54, 55, 55, 56, 56, 57, 57, 57, 58, 58, 59, 59, 60, 60, 61, 61, 62, 62];

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
    &[5, 6, 7, 160],
    &[8, 136, 7, 160],
    &[9, 6, 10, 141, 11, 129],
    &[12, 6, 146, 13, 160, 129],
    &[14, 160, 129, 147],
    &[15, 160],
    &[16, 6, 149, 152],
    &[17, 6, 10, 156, 11, 129],
    &[18, 6, 10, 156, 11, 129],
    &[160, 135],
    &[7, 160],
    &[],
    &[6, 138, 137],
    &[19, 6, 138, 137],
    &[],
    &[20, 139, 140, 21],
    &[],
    &[22],
    &[6],
    &[23, 139, 140],
    &[],
    &[143, 142],
    &[],
    &[23, 141],
    &[],
    &[6, 144, 145],
    &[24, 6],
    &[],
    &[7, 160],
    &[],
    &[23, 6, 146],
    &[],
    &[25, 148],
    &[],
    &[129],
    &[14, 160, 129, 147],
    &[10, 150, 11],
    &[],
    &[160, 151],
    &[],
    &[23, 150],
    &[],
    &[153, 160, 154],
    &[],
    &[7],
    &[26],
    &[27],
    &[28],
    &[29],
    &[30, 176, 155],
    &[],
    &[31],
    &[],
    &[158, 157],
    &[],
    &[23, 156],
    &[],
    &[160, 159],
    &[24, 160],
    &[],
    &[162, 161],
    &[14, 162, 25, 160],
    &[],
    &[164, 163],
    &[32, 164, 163],
    &[],
    &[166, 165],
    &[33, 166, 165],
    &[],
    &[34, 166],
    &[167],
    &[170, 168],
    &[169, 170, 168],
    &[],
    &[35],
    &[36],
    &[26],
    &[27],
    &[28],
    &[29],
    &[172, 171],
    &[37, 172],
    &[38, 172],
    &[],
    &[174, 173],
    &[39, 174, 173],
    &[40, 174, 173],
    &[],
    &[176, 175],
    &[41, 176, 175],
    &[42, 176, 175],
    &[],
    &[40, 176],
    &[177],
    &[179, 178],
    &[43, 176],
    &[],
    &[181, 180],
    &[10, 156, 11, 180],
    &[19, 6, 180],
    &[20, 160, 21, 180],
    &[],
    &[22],
    &[44],
    &[6],
    &[45],
    &[46],
    &[10, 182, 11],
    &[20, 184, 21],
    &[1, 132, 189, 2],
    &[160, 183],
    &[],
    &[23, 182],
    &[],
    &[160, 185],
    &[],
    &[12, 6, 146, 13, 162, 188],
    &[23, 186],
    &[],
    &[160, 187],
    &[],
    &[23, 186],
    &[],
    &[14, 162],
    &[],
    &[6, 24, 160, 132, 190],
    &[],
    &[23, 132, 189],
    &[],
];

pub const NULLABLE: [bool; NN] = [false, false, true, true, true, false, false, true, false, true, true, false, true, true, true, false, true, true, true, true, false, true, true, true, true, false, true, true, true, true, false, true, false, true, false, true, false, true, false, false, true, false, false, true, false, true, false, true, false, false, true, false, true, false, true, true, true, true, true, true, true, true, true];

/// Bit k: terminal k.
pub const FIRST: [u128; NN] = [
    0x71040057d77b,
    0x2,
    0x71040057d762,
    0x18,
    0x18,
    0x18,
    0x71040057d762,
    0x80,
    0x40,
    0x80000,
    0x100000,
    0x400040,
    0x800000,
    0x40,
    0x800000,
    0x40,
    0x1000000,
    0x80,
    0x800000,
    0x2000000,
    0x4002,
    0x400,
    0x710400500442,
    0x800000,
    0x3c000080,
    0x3c000080,
    0x40000000,
    0x80000000,
    0x710400500442,
    0x800000,
    0x710400500442,
    0x1000000,
    0x710400500442,
    0x4000,
    0x710400500442,
    0x100000000,
    0x710400500442,
    0x200000000,
    0x710400500442,
    0x710000500442,
    0x183c000000,
    0x183c000000,
    0x710000500442,
    0x6000000000,
    0x710000500442,
    0x18000000000,
    0x710000500442,
    0x60000000000,
    0x710000500442,
    0x700000500442,
    0x80000000000,
    0x700000500442,
    0x180400,
    0x700000500442,
    0x710400500442,
    0x800000,
    0x710400500442,
    0x801000,
    0x710400500442,
    0x800000,
    0x4000,
    0x40,
    0x800000,
];

/// Bit k: terminal k; FOLLOW_END: the end marker may follow (the start symbol only).
pub const FOLLOW: [u128; NN] = [
    0x0,
    0x200001d,
    0x5,
    0x5,
    0x710400d7d767,
    0x710400d7d77f,
    0x1d,
    0x1d,
    0x80,
    0x80,
    0x80080,
    0xa00000,
    0x200000,
    0x800,
    0x800,
    0x800800,
    0x800880,
    0x800800,
    0x2000,
    0x1d,
    0x1d,
    0x3c00009d,
    0x800,
    0x800,
    0x1d,
    0x710400500442,
    0x1d,
    0x1d,
    0x800,
    0x800,
    0x800800,
    0x800800,
    0x41a0189f,
    0x41a0189f,
    0x43a0589f,
    0x43a0589f,
    0x143a0589f,
    0x143a0589f,
    0x343a0589f,
    0x343a0589f,
    0x343a0589f,
    0x710000500442,
    0x1b7fa0589f,
    0x1b7fa0589f,
    0x7b7fa0589f,
    0x7b7fa0589f,
    0x1fb7fa0589f,
    0x1fb7fa0589f,
    0x7fbffa0589f,
    0x7fbffa0589f,
    0x7fbffa0589f,
    0xffbffa0589f,
    0xffbffa0589f,
    0xffbffb85c9f,
    0x800,
    0x800,
    0x200000,
    0x200000,
    0x200000,
    0x200000,
    0x200000,
    0x4,
    0x4,
];
pub const FOLLOW_END: [bool; NN] = [true, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false];

/// TABLE[nonterminal][lookahead]: the rule to expand by, or NONE.
pub const TABLE: [[u8; NT]; NN] = [
    [0, 0, NONE, 0, 0, 0, 0, NONE, 0, 0, 0, NONE, 0, NONE, 0, 0, 0, 0, 0, NONE, 0, NONE, 0, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 0, NONE, NONE, NONE, NONE, NONE, 0, NONE, NONE, NONE, 0, 0, 0],
    [NONE, 1, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [3, 2, 3, NONE, NONE, 2, 2, NONE, 2, 2, 2, NONE, 2, NONE, 2, 2, 2, 2, 2, NONE, 2, NONE, 2, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 2, NONE, NONE, NONE, NONE, NONE, 2, NONE, NONE, NONE, 2, 2, 2],
    [5, NONE, 5, 4, 4, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [7, 7, 7, 6, 6, 7, 7, NONE, 7, 7, 7, NONE, 7, NONE, 7, 7, 7, 7, 7, NONE, 7, NONE, 7, 7, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 7, NONE, NONE, NONE, NONE, NONE, 7, NONE, NONE, NONE, 7, 7, 7],
    [NONE, NONE, NONE, 8, 9, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, 19, NONE, NONE, NONE, 10, 19, NONE, 11, 12, 19, NONE, 13, NONE, 14, 15, 16, 17, 18, NONE, 19, NONE, 19, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 19, NONE, NONE, NONE, NONE, NONE, 19, NONE, NONE, NONE, 19, 19, 19],
    [21, NONE, 21, 21, 21, NONE, NONE, 20, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, NONE, NONE, NONE, NONE, NONE, 22, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, NONE, NONE, NONE, NONE, NONE, NONE, 24, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 23, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, NONE, NONE, NONE, NONE, NONE, NONE, 26, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 26, 25, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, NONE, NONE, NONE, NONE, NONE, 28, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 27, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 30, NONE, 29, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, NONE, NONE, NONE, NONE, NONE, 31, NONE, NONE, NONE, NONE, 32, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 34, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 33, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, NONE, NONE, NONE, NONE, NONE, 35, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, NONE, NONE, NONE, NONE, NONE, NONE, 37, NONE, NONE, NONE, 37, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 37, 36, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, NONE, NONE, NONE, NONE, NONE, NONE, 38, NONE, NONE, NONE, 39, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 39, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 41, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 40, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [43, NONE, 43, 43, 43, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 42, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, 44, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 45, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [47, NONE, 47, 47, 47, NONE, NONE, 47, NONE, NONE, 46, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 47, 47, 47, 47, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, 48, NONE, NONE, NONE, NONE, 48, NONE, NONE, NONE, 48, 49, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 48, NONE, 48, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 48, NONE, NONE, NONE, NONE, NONE, 48, NONE, NONE, NONE, 48, 48, 48],
    [NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 51, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 50, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [53, NONE, 53, 53, 53, NONE, NONE, 52, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 52, 52, 52, 52, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, NONE, NONE, NONE, NONE, NONE, NONE, 54, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 55, 56, 57, 58, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [60, NONE, 60, 60, 60, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 59, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [62, NONE, 62, 62, 62, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 61, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, 63, NONE, NONE, NONE, NONE, 63, NONE, NONE, NONE, 63, 64, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 63, NONE, 63, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 63, NONE, NONE, NONE, NONE, NONE, 63, NONE, NONE, NONE, 63, 63, 63],
    [NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 66, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 65, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, 67, NONE, NONE, NONE, NONE, 67, NONE, NONE, NONE, 67, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 67, NONE, 67, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 67, NONE, NONE, NONE, NONE, NONE, 67, NONE, NONE, NONE, 67, 67, 67],
    [NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 69, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 69, 68, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, 70, NONE, NONE, NONE, NONE, 70, NONE, NONE, NONE, 70, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 70, NONE, 70, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 70, NONE, NONE, NONE, NONE, NONE, 70, NONE, NONE, NONE, 70, 70, 70],
    [72, 72, 72, 72, 72, NONE, NONE, 72, NONE, NONE, NONE, 72, 72, NONE, 71, NONE, NONE, NONE, NONE, NONE, NONE, 72, NONE, 72, 72, NONE, NONE, NONE, NONE, NONE, 72, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, 73, NONE, NONE, NONE, NONE, 73, NONE, NONE, NONE, 73, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 73, NONE, 73, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 73, NONE, NONE, NONE, NONE, NONE, 73, NONE, NONE, NONE, 73, 73, 73],
    [75, 75, 75, 75, 75, NONE, NONE, 75, NONE, NONE, NONE, 75, 75, NONE, 75, NONE, NONE, NONE, NONE, NONE, NONE, 75, NONE, 75, 75, 75, NONE, NONE, NONE, NONE, 75, NONE, 74, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, 76, NONE, NONE, NONE, NONE, 76, NONE, NONE, NONE, 76, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 76, NONE, 76, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 76, NONE, NONE, NONE, NONE, NONE, 76, NONE, NONE, NONE, 76, 76, 76],
    [78, 78, 78, 78, 78, NONE, NONE, 78, NONE, NONE, NONE, 78, 78, NONE, 78, NONE, NONE, NONE, NONE, NONE, NONE, 78, NONE, 78, 78, 78, NONE, NONE, NONE, NONE, 78, NONE, 78, 77, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, 80, NONE, NONE, NONE, NONE, 80, NONE, NONE, NONE, 80, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 80, NONE, 80, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 79, NONE, NONE, NONE, NONE, NONE, 80, NONE, NONE, NONE, 80, 80, 80],
    [NONE, 81, NONE, NONE, NONE, NONE, 81, NONE, NONE, NONE, 81, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 81, NONE, 81, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 81, NONE, NONE, NONE, 81, 81, 81],
    [83, 83, 83, 83, 83, NONE, NONE, 83, NONE, NONE, NONE, 83, 83, NONE, 83, NONE, NONE, NONE, NONE, NONE, NONE, 83, NONE, 83, 83, 83, 82, 82, 82, 82, 83, NONE, 83, 83, NONE, 82, 82, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 86, 87, 88, 89, NONE, NONE, NONE, NONE, NONE, 84, 85, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, 90, NONE, NONE, NONE, NONE, 90, NONE, NONE, NONE, 90, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 90, NONE, 90, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 90, NONE, NONE, NONE, 90, 90, 90],
    [93, 93, 93, 93, 93, NONE, NONE, 93, NONE, NONE, NONE, 93, 93, NONE, 93, NONE, NONE, NONE, NONE, NONE, NONE, 93, NONE, 93, 93, 93, 93, 93, 93, 93, 93, NONE, 93, 93, NONE, 93, 93, 91, 92, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, 94, NONE, NONE, NONE, NONE, 94, NONE, NONE, NONE, 94, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 94, NONE, 94, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 94, NONE, NONE, NONE, 94, 94, 94],
    [97, 97, 97, 97, 97, NONE, NONE, 97, NONE, NONE, NONE, 97, 97, NONE, 97, NONE, NONE, NONE, NONE, NONE, NONE, 97, NONE, 97, 97, 97, 97, 97, 97, 97, 97, NONE, 97, 97, NONE, 97, 97, 97, 97, 95, 96, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, 98, NONE, NONE, NONE, NONE, 98, NONE, NONE, NONE, 98, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 98, NONE, 98, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 98, NONE, NONE, NONE, 98, 98, 98],
    [101, 101, 101, 101, 101, NONE, NONE, 101, NONE, NONE, NONE, 101, 101, NONE, 101, NONE, NONE, NONE, NONE, NONE, NONE, 101, NONE, 101, 101, 101, 101, 101, 101, 101, 101, NONE, 101, 101, NONE, 101, 101, 101, 101, 101, 101, 99, 100, NONE, NONE, NONE, NONE],
    [NONE, 103, NONE, NONE, NONE, NONE, 103, NONE, NONE, NONE, 103, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 103, NONE, 103, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 102, NONE, NONE, NONE, 103, 103, 103],
    [NONE, 104, NONE, NONE, NONE, NONE, 104, NONE, NONE, NONE, 104, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 104, NONE, 104, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 104, 104, 104],
    [106, 106, 106, 106, 106, NONE, NONE, 106, NONE, NONE, NONE, 106, 106, NONE, 106, NONE, NONE, NONE, NONE, NONE, NONE, 106, NONE, 106, 106, 106, 106, 106, 106, 106, 106, 106, 106, 106, NONE, 106, 106, 106, 106, 106, 106, 106, 106, 105, NONE, NONE, NONE],
    [NONE, 107, NONE, NONE, NONE, NONE, 107, NONE, NONE, NONE, 107, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 107, NONE, 107, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 107, 107, 107],
    [111, 111, 111, 111, 111, NONE, NONE, 111, NONE, NONE, 108, 111, 111, NONE, 111, NONE, NONE, NONE, NONE, 109, 110, 111, NONE, 111, 111, 111, 111, 111, 111, 111, 111, 111, 111, 111, NONE, 111, 111, 111, 111, 111, 111, 111, 111, 111, NONE, NONE, NONE],
    [NONE, 119, NONE, NONE, NONE, NONE, 114, NONE, NONE, NONE, 117, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 118, NONE, 112, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 113, 115, 116],
    [NONE, 120, NONE, NONE, NONE, NONE, 120, NONE, NONE, NONE, 120, 121, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 120, NONE, 120, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 120, NONE, NONE, NONE, NONE, NONE, 120, NONE, NONE, NONE, 120, 120, 120],
    [NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 123, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 122, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, 124, NONE, NONE, NONE, NONE, 124, NONE, NONE, NONE, 124, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 124, 125, 124, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 124, NONE, NONE, NONE, NONE, NONE, 124, NONE, NONE, NONE, 124, 124, 124],
    [NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 126, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 128, NONE, 127, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, 129, NONE, NONE, NONE, NONE, 129, NONE, NONE, NONE, 129, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 129, 130, 129, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 129, NONE, NONE, NONE, NONE, NONE, 129, NONE, NONE, NONE, 129, 129, 129],
    [NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 132, NONE, 131, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 133, NONE, NONE, NONE, NONE, NONE, NONE, 134, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, NONE, 136, NONE, NONE, NONE, 135, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, NONE, 138, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 137, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
];

/// A nullable nonterminal's rule that derives the empty string, or NONE.
pub const EMPTY: [u8; NN] = [NONE, NONE, 3, 5, 7, NONE, NONE, 21, NONE, 24, 26, NONE, 30, 32, 34, NONE, 37, 39, 41, 43, NONE, 47, 49, 51, 53, NONE, 60, 62, 64, 66, NONE, 69, NONE, 72, NONE, 75, NONE, 78, NONE, NONE, 83, NONE, NONE, 93, NONE, 97, NONE, 101, NONE, NONE, 106, NONE, 111, NONE, 121, 123, 125, 128, 130, 132, 134, 136, 138];
