//! Generated from grammar.ll1 by tests/grammar.rs; do not edit. After a grammar change:
//! `FG_REGEN=1 cargo test -p forge-lang --test grammar`, then the actions in syntax.rs.
//! Terminals and nonterminals are numbered as research/forge-lang/lean/gen_grammar.py
//! numbers them, so NULLABLE, FIRST and FOLLOW are the sets the Lean check certifies.

pub const NT: usize = 49;
pub const NN: usize = 66;
pub const NP: usize = 147;
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
    pub const DOT: u8 = 20;
    pub const LBRACK: u8 = 21;
    pub const RBRACK: u8 = 22;
    pub const NUM: u8 = 23;
    pub const COMMA: u8 = 24;
    pub const COLON: u8 = 25;
    pub const ELSE: u8 = 26;
    pub const LT: u8 = 27;
    pub const LE: u8 = 28;
    pub const GT: u8 = 29;
    pub const GE: u8 = 30;
    pub const PLUSMINUS: u8 = 31;
    pub const PERCENT: u8 = 32;
    pub const AT: u8 = 33;
    pub const OR: u8 = 34;
    pub const AND: u8 = 35;
    pub const NOT: u8 = 36;
    pub const EQEQ: u8 = 37;
    pub const NE: u8 = 38;
    pub const DOTDOT: u8 = 39;
    pub const DOTDOTEQ: u8 = 40;
    pub const PLUS: u8 = 41;
    pub const MINUS: u8 = 42;
    pub const STAR: u8 = 43;
    pub const SLASH: u8 = 44;
    pub const CARET: u8 = 45;
    pub const STR: u8 = 46;
    pub const TRUE: u8 = 47;
    pub const FALSE: u8 = 48;
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

pub const LHS: [u8; NP] = [0, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 7, 7, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13, 13, 14, 14, 15, 16, 16, 17, 17, 18, 18, 19, 19, 20, 20, 21, 21, 22, 22, 23, 23, 24, 24, 25, 25, 25, 25, 25, 26, 26, 27, 27, 28, 28, 29, 29, 30, 31, 31, 32, 32, 33, 33, 33, 34, 34, 35, 36, 36, 37, 38, 38, 39, 40, 40, 41, 41, 42, 43, 43, 44, 44, 44, 44, 44, 44, 45, 46, 46, 46, 47, 48, 48, 48, 49, 50, 50, 50, 51, 51, 52, 53, 53, 54, 55, 55, 55, 55, 56, 56, 56, 56, 56, 56, 56, 56, 57, 57, 58, 58, 59, 59, 60, 60, 60, 61, 61, 62, 62, 63, 63, 64, 64, 65, 65];

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
    &[5, 6, 7, 163],
    &[8, 136, 7, 163],
    &[9, 6, 10, 141, 11, 129],
    &[12, 6, 146, 13, 163, 129],
    &[14, 163, 129, 147],
    &[15, 163],
    &[16, 6, 149, 152, 156],
    &[17, 6, 10, 159, 11, 129],
    &[18, 6, 10, 159, 11, 129],
    &[19, 6, 10, 159, 11, 129],
    &[163, 135],
    &[7, 163],
    &[],
    &[6, 138, 137],
    &[20, 6, 138, 137],
    &[],
    &[21, 139, 140, 22],
    &[],
    &[23],
    &[6],
    &[24, 139, 140],
    &[],
    &[143, 142],
    &[],
    &[24, 141],
    &[],
    &[6, 144, 145],
    &[25, 6],
    &[],
    &[7, 163],
    &[],
    &[24, 6, 146],
    &[],
    &[26, 148],
    &[],
    &[129],
    &[14, 163, 129, 147],
    &[10, 150, 11],
    &[],
    &[163, 151],
    &[],
    &[24, 150],
    &[],
    &[153, 163, 154],
    &[],
    &[7],
    &[27],
    &[28],
    &[29],
    &[30],
    &[31, 179, 155],
    &[],
    &[32],
    &[],
    &[33, 158, 7, 163, 157],
    &[],
    &[24, 158, 7, 163, 157],
    &[],
    &[6, 138, 137],
    &[161, 160],
    &[],
    &[24, 159],
    &[],
    &[163, 162],
    &[33, 25, 163],
    &[19, 25, 163],
    &[25, 163],
    &[],
    &[165, 164],
    &[14, 165, 26, 163],
    &[],
    &[167, 166],
    &[34, 167, 166],
    &[],
    &[169, 168],
    &[35, 169, 168],
    &[],
    &[36, 169],
    &[170],
    &[173, 171],
    &[172, 173, 171],
    &[],
    &[37],
    &[38],
    &[27],
    &[28],
    &[29],
    &[30],
    &[175, 174],
    &[39, 175],
    &[40, 175],
    &[],
    &[177, 176],
    &[41, 177, 176],
    &[42, 177, 176],
    &[],
    &[179, 178],
    &[43, 179, 178],
    &[44, 179, 178],
    &[],
    &[42, 179],
    &[180],
    &[182, 181],
    &[45, 179],
    &[],
    &[184, 183],
    &[10, 159, 11, 183],
    &[20, 6, 183],
    &[21, 163, 22, 183],
    &[],
    &[23],
    &[46],
    &[6],
    &[47],
    &[48],
    &[10, 185, 11],
    &[21, 187, 22],
    &[1, 132, 192, 2],
    &[163, 186],
    &[],
    &[24, 185],
    &[],
    &[163, 188],
    &[],
    &[12, 6, 146, 13, 165, 191],
    &[24, 189],
    &[],
    &[163, 190],
    &[],
    &[24, 189],
    &[],
    &[14, 165],
    &[],
    &[6, 25, 163, 132, 193],
    &[],
    &[24, 132, 192],
    &[],
];

pub const NULLABLE: [bool; NN] = [false, false, true, true, true, false, false, true, false, true, true, false, true, true, true, false, true, true, true, true, false, true, true, true, true, false, true, true, true, true, false, true, true, false, true, false, true, false, true, false, true, false, false, true, false, false, true, false, true, false, true, false, false, true, false, true, false, true, true, true, true, true, true, true, true, true];

/// Bit k: terminal k.
pub const FIRST: [u128; NN] = [
    0x1c41000afd77b,
    0x2,
    0x1c41000afd762,
    0x18,
    0x18,
    0x18,
    0x1c41000afd762,
    0x80,
    0x40,
    0x100000,
    0x200000,
    0x800040,
    0x1000000,
    0x40,
    0x1000000,
    0x40,
    0x2000000,
    0x80,
    0x1000000,
    0x4000000,
    0x4002,
    0x400,
    0x1c41000a00442,
    0x1000000,
    0x78000080,
    0x78000080,
    0x80000000,
    0x100000000,
    0x200000000,
    0x1000000,
    0x40,
    0x1c41200a80442,
    0x1000000,
    0x1c41200a80442,
    0x2000000,
    0x1c41000a00442,
    0x4000,
    0x1c41000a00442,
    0x400000000,
    0x1c41000a00442,
    0x800000000,
    0x1c41000a00442,
    0x1c40000a00442,
    0x6078000000,
    0x6078000000,
    0x1c40000a00442,
    0x18000000000,
    0x1c40000a00442,
    0x60000000000,
    0x1c40000a00442,
    0x180000000000,
    0x1c40000a00442,
    0x1c00000a00442,
    0x200000000000,
    0x1c00000a00442,
    0x300400,
    0x1c00000a00442,
    0x1c41000a00442,
    0x1000000,
    0x1c41000a00442,
    0x1001000,
    0x1c41000a00442,
    0x1000000,
    0x4000,
    0x40,
    0x1000000,
];

/// Bit k: terminal k; FOLLOW_END: the end marker may follow (the start symbol only).
pub const FOLLOW: [u128; NN] = [
    0x0,
    0x400001d,
    0x5,
    0x5,
    0x1c41001afd767,
    0x1c41001afd77f,
    0x1d,
    0x1d,
    0x80,
    0x80,
    0x100080,
    0x1400000,
    0x400000,
    0x800,
    0x800,
    0x1000800,
    0x1000880,
    0x1000800,
    0x2000,
    0x1d,
    0x1d,
    0x27800009d,
    0x800,
    0x800,
    0x20000001d,
    0x1c41000a00442,
    0x20000001d,
    0x20000001d,
    0x1d,
    0x1d,
    0x80,
    0x800,
    0x800,
    0x1000800,
    0x1000800,
    0x28340189f,
    0x28340189f,
    0x28740589f,
    0x28740589f,
    0x68740589f,
    0x68740589f,
    0xe8740589f,
    0xe8740589f,
    0xe8740589f,
    0x1c40000a00442,
    0x6eff40589f,
    0x6eff40589f,
    0x1eeff40589f,
    0x1eeff40589f,
    0x7eeff40589f,
    0x7eeff40589f,
    0x1fefff40589f,
    0x1fefff40589f,
    0x1fefff40589f,
    0x3fefff40589f,
    0x3fefff40589f,
    0x3fefff705c9f,
    0x800,
    0x800,
    0x400000,
    0x400000,
    0x400000,
    0x400000,
    0x400000,
    0x4,
    0x4,
];
pub const FOLLOW_END: [bool; NN] = [true, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false];

/// TABLE[nonterminal][lookahead]: the rule to expand by, or NONE.
pub const TABLE: [[u8; NT]; NN] = [
    [0, 0, NONE, 0, 0, 0, 0, NONE, 0, 0, 0, NONE, 0, NONE, 0, 0, 0, 0, 0, 0, NONE, 0, NONE, 0, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 0, NONE, NONE, NONE, NONE, NONE, 0, NONE, NONE, NONE, 0, 0, 0],
    [NONE, 1, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [3, 2, 3, NONE, NONE, 2, 2, NONE, 2, 2, 2, NONE, 2, NONE, 2, 2, 2, 2, 2, 2, NONE, 2, NONE, 2, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 2, NONE, NONE, NONE, NONE, NONE, 2, NONE, NONE, NONE, 2, 2, 2],
    [5, NONE, 5, 4, 4, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [7, 7, 7, 6, 6, 7, 7, NONE, 7, 7, 7, NONE, 7, NONE, 7, 7, 7, 7, 7, 7, NONE, 7, NONE, 7, 7, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 7, NONE, NONE, NONE, NONE, NONE, 7, NONE, NONE, NONE, 7, 7, 7],
    [NONE, NONE, NONE, 8, 9, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, 20, NONE, NONE, NONE, 10, 20, NONE, 11, 12, 20, NONE, 13, NONE, 14, 15, 16, 17, 18, 19, NONE, 20, NONE, 20, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 20, NONE, NONE, NONE, NONE, NONE, 20, NONE, NONE, NONE, 20, 20, 20],
    [22, NONE, 22, 22, 22, NONE, NONE, 21, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, NONE, NONE, NONE, NONE, NONE, 23, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, NONE, NONE, NONE, NONE, NONE, NONE, 25, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 24, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, NONE, NONE, NONE, NONE, NONE, NONE, 27, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 27, 26, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, NONE, NONE, NONE, NONE, NONE, 29, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 28, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 31, NONE, 30, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, NONE, NONE, NONE, NONE, NONE, 32, NONE, NONE, NONE, NONE, 33, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 35, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 34, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, NONE, NONE, NONE, NONE, NONE, 36, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, NONE, NONE, NONE, NONE, NONE, NONE, 38, NONE, NONE, NONE, 38, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 38, 37, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, NONE, NONE, NONE, NONE, NONE, NONE, 39, NONE, NONE, NONE, 40, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 40, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 42, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 41, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [44, NONE, 44, 44, 44, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 43, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, 45, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 46, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [48, NONE, 48, 48, 48, NONE, NONE, 48, NONE, NONE, 47, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 48, 48, 48, 48, NONE, NONE, 48, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, 49, NONE, NONE, NONE, NONE, 49, NONE, NONE, NONE, 49, 50, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 49, NONE, 49, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 49, NONE, NONE, NONE, NONE, NONE, 49, NONE, NONE, NONE, 49, 49, 49],
    [NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 52, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 51, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [54, NONE, 54, 54, 54, NONE, NONE, 53, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 53, 53, 53, 53, NONE, NONE, 54, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, NONE, NONE, NONE, NONE, NONE, NONE, 55, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 56, 57, 58, 59, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [61, NONE, 61, 61, 61, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 60, NONE, 61, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [63, NONE, 63, 63, 63, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 62, 63, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [65, NONE, 65, 65, 65, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 64, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [67, NONE, 67, 67, 67, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 66, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, NONE, NONE, NONE, NONE, NONE, 68, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, 69, NONE, NONE, NONE, NONE, 69, NONE, NONE, NONE, 69, 70, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 69, NONE, 69, NONE, 69, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 69, NONE, NONE, 69, NONE, NONE, NONE, NONE, NONE, 69, NONE, NONE, NONE, 69, 69, 69],
    [NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 72, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 71, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, 73, NONE, NONE, NONE, NONE, 73, NONE, NONE, NONE, 73, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 75, NONE, 73, NONE, 73, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 74, NONE, NONE, 73, NONE, NONE, NONE, NONE, NONE, 73, NONE, NONE, NONE, 73, 73, 73],
    [NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 77, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 77, 76, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, 78, NONE, NONE, NONE, NONE, 78, NONE, NONE, NONE, 78, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 78, NONE, 78, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 78, NONE, NONE, NONE, NONE, NONE, 78, NONE, NONE, NONE, 78, 78, 78],
    [80, 80, 80, 80, 80, NONE, NONE, 80, NONE, NONE, NONE, 80, 80, NONE, 79, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 80, NONE, 80, 80, NONE, NONE, NONE, NONE, NONE, 80, NONE, 80, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, 81, NONE, NONE, NONE, NONE, 81, NONE, NONE, NONE, 81, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 81, NONE, 81, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 81, NONE, NONE, NONE, NONE, NONE, 81, NONE, NONE, NONE, 81, 81, 81],
    [83, 83, 83, 83, 83, NONE, NONE, 83, NONE, NONE, NONE, 83, 83, NONE, 83, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 83, NONE, 83, 83, 83, NONE, NONE, NONE, NONE, 83, NONE, 83, 82, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, 84, NONE, NONE, NONE, NONE, 84, NONE, NONE, NONE, 84, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 84, NONE, 84, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 84, NONE, NONE, NONE, NONE, NONE, 84, NONE, NONE, NONE, 84, 84, 84],
    [86, 86, 86, 86, 86, NONE, NONE, 86, NONE, NONE, NONE, 86, 86, NONE, 86, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 86, NONE, 86, 86, 86, NONE, NONE, NONE, NONE, 86, NONE, 86, 86, 85, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, 88, NONE, NONE, NONE, NONE, 88, NONE, NONE, NONE, 88, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 88, NONE, 88, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 87, NONE, NONE, NONE, NONE, NONE, 88, NONE, NONE, NONE, 88, 88, 88],
    [NONE, 89, NONE, NONE, NONE, NONE, 89, NONE, NONE, NONE, 89, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 89, NONE, 89, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 89, NONE, NONE, NONE, 89, 89, 89],
    [91, 91, 91, 91, 91, NONE, NONE, 91, NONE, NONE, NONE, 91, 91, NONE, 91, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 91, NONE, 91, 91, 91, 90, 90, 90, 90, 91, NONE, 91, 91, 91, NONE, 90, 90, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 94, 95, 96, 97, NONE, NONE, NONE, NONE, NONE, NONE, 92, 93, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, 98, NONE, NONE, NONE, NONE, 98, NONE, NONE, NONE, 98, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 98, NONE, 98, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 98, NONE, NONE, NONE, 98, 98, 98],
    [101, 101, 101, 101, 101, NONE, NONE, 101, NONE, NONE, NONE, 101, 101, NONE, 101, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 101, NONE, 101, 101, 101, 101, 101, 101, 101, 101, NONE, 101, 101, 101, NONE, 101, 101, 99, 100, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, 102, NONE, NONE, NONE, NONE, 102, NONE, NONE, NONE, 102, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 102, NONE, 102, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 102, NONE, NONE, NONE, 102, 102, 102],
    [105, 105, 105, 105, 105, NONE, NONE, 105, NONE, NONE, NONE, 105, 105, NONE, 105, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 105, NONE, 105, 105, 105, 105, 105, 105, 105, 105, NONE, 105, 105, 105, NONE, 105, 105, 105, 105, 103, 104, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, 106, NONE, NONE, NONE, NONE, 106, NONE, NONE, NONE, 106, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 106, NONE, 106, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 106, NONE, NONE, NONE, 106, 106, 106],
    [109, 109, 109, 109, 109, NONE, NONE, 109, NONE, NONE, NONE, 109, 109, NONE, 109, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 109, NONE, 109, 109, 109, 109, 109, 109, 109, 109, NONE, 109, 109, 109, NONE, 109, 109, 109, 109, 109, 109, 107, 108, NONE, NONE, NONE, NONE],
    [NONE, 111, NONE, NONE, NONE, NONE, 111, NONE, NONE, NONE, 111, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 111, NONE, 111, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 110, NONE, NONE, NONE, 111, 111, 111],
    [NONE, 112, NONE, NONE, NONE, NONE, 112, NONE, NONE, NONE, 112, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 112, NONE, 112, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 112, 112, 112],
    [114, 114, 114, 114, 114, NONE, NONE, 114, NONE, NONE, NONE, 114, 114, NONE, 114, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 114, NONE, 114, 114, 114, 114, 114, 114, 114, 114, 114, 114, 114, 114, NONE, 114, 114, 114, 114, 114, 114, 114, 114, 113, NONE, NONE, NONE],
    [NONE, 115, NONE, NONE, NONE, NONE, 115, NONE, NONE, NONE, 115, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 115, NONE, 115, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 115, 115, 115],
    [119, 119, 119, 119, 119, NONE, NONE, 119, NONE, NONE, 116, 119, 119, NONE, 119, NONE, NONE, NONE, NONE, NONE, 117, 118, 119, NONE, 119, 119, 119, 119, 119, 119, 119, 119, 119, 119, 119, 119, NONE, 119, 119, 119, 119, 119, 119, 119, 119, 119, NONE, NONE, NONE],
    [NONE, 127, NONE, NONE, NONE, NONE, 122, NONE, NONE, NONE, 125, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 126, NONE, 120, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 121, 123, 124],
    [NONE, 128, NONE, NONE, NONE, NONE, 128, NONE, NONE, NONE, 128, 129, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 128, NONE, 128, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 128, NONE, NONE, NONE, NONE, NONE, 128, NONE, NONE, NONE, 128, 128, 128],
    [NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 131, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 130, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, 132, NONE, NONE, NONE, NONE, 132, NONE, NONE, NONE, 132, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 132, 133, 132, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 132, NONE, NONE, NONE, NONE, NONE, 132, NONE, NONE, NONE, 132, 132, 132],
    [NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 134, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 136, NONE, 135, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, 137, NONE, NONE, NONE, NONE, 137, NONE, NONE, NONE, 137, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 137, 138, 137, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 137, NONE, NONE, NONE, NONE, NONE, 137, NONE, NONE, NONE, 137, 137, 137],
    [NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 140, NONE, 139, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 141, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 142, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, NONE, 144, NONE, NONE, NONE, 143, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
    [NONE, NONE, 146, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, 145, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE, NONE],
];

/// A nullable nonterminal's rule that derives the empty string, or NONE.
pub const EMPTY: [u8; NN] = [NONE, NONE, 3, 5, 7, NONE, NONE, 22, NONE, 25, 27, NONE, 31, 33, 35, NONE, 38, 40, 42, 44, NONE, 48, 50, 52, 54, NONE, 61, 63, 65, 67, NONE, 70, 72, NONE, 77, NONE, 80, NONE, 83, NONE, 86, NONE, NONE, 91, NONE, NONE, 101, NONE, 105, NONE, 109, NONE, NONE, 114, NONE, 119, NONE, 129, 131, 133, 136, 138, 140, 142, 144, 146];
