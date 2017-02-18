use ast::{Statement};
use lexer::{self};
extern crate lalrpop_util as __lalrpop_util;

mod __parse__statement_list {
    #![allow(non_snake_case, non_camel_case_types, unused_mut, unused_variables, unused_imports)]

    use ast::{Statement};
    use lexer::{self};
    extern crate lalrpop_util as __lalrpop_util;
    use super::__ToTriple;
    #[allow(dead_code)]
    pub enum __Symbol<> {
        Term_22_28_22(lexer::Token),
        Term_22_29_22(lexer::Token),
        Term_22_2c_22(lexer::Token),
        Term_22_2e_22(lexer::Token),
        Term_22_3b_22(lexer::Token),
        Term_22_3d_22(lexer::Token),
        TermCONSTRAINT(lexer::Token),
        TermCREATE(lexer::Token),
        TermDigit(i32),
        TermFILLFACTOR(lexer::Token),
        TermFOREIGN(lexer::Token),
        TermINT(lexer::Token),
        TermINTEGER(lexer::Token),
        TermIdent(String),
        TermKEY(lexer::Token),
        TermNOT(lexer::Token),
        TermNULL(lexer::Token),
        TermPRIMARY(lexer::Token),
        TermREFERENCES(lexer::Token),
        TermSERIAL(lexer::Token),
        TermSMALLINT(lexer::Token),
        TermTABLE(lexer::Token),
        TermUNIQUE(lexer::Token),
        TermUUID(lexer::Token),
        TermVARCHAR(lexer::Token),
        TermWITH(lexer::Token),
        Termerror(__lalrpop_util::ErrorRecovery<(), lexer::Token, ()>),
        Nt_22_3b_22_3f(::std::option::Option<lexer::Token>),
        Nt_28_22_2c_22_20constraint__list_29((lexer::Token, ())),
        Nt_28_22_2c_22_20constraint__list_29_3f(::std::option::Option<(lexer::Token, ())>),
        Nt_28Ident_20_22_2e_22_29((String, lexer::Token)),
        Nt_28Ident_20_22_2e_22_29_3f(::std::option::Option<(String, lexer::Token)>),
        Nt____statement__list(Vec<Statement>),
        Ntcolumn__definition(()),
        Ntcolumn__definition__list(()),
        Ntconstraint(()),
        Ntconstraint__list(()),
        Ntqualifier(()),
        Ntqualifier__list(()),
        Ntsql__type(()),
        Ntstatement(Statement),
        Ntstatement__list(Vec<Statement>),
        Nttable__name(()),
        Ntwith__option(()),
        Ntwith__option__list(()),
        Ntwith__qualifier(()),
        Ntwith__qualifier_3f(::std::option::Option<()>),
    }
    const __ACTION: &'static [i32] = &[
        // State 0
        0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 1
        0, 0, 0, 0, 0, 0, 0, -35, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 2
        0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 3
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 6, 0, 0, 0, 0, 0,
        // State 4
        0, 0, 0, 0, 0, 0, 0, -34, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 5
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 6
        9, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 7
        -37, 0, 0, 10, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 8
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 13, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 9
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 14, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 10
        0, -12, -12, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 11
        0, 15, 16, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 12
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 18, 19, 0, 0, 0, 0, 0, 0, 20, 21, 0, 0, 22, 23, 0, 0,
        // State 13
        -36, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 14
        0, 0, 0, 0, 24, 0, 0, -33, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 15
        0, 0, 0, 0, 0, 0, 28, 0, 0, 0, 0, 0, 0, 13, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 16
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 31, 32, 33, 0, 0, 0, 0, 34, 0, 0, 0, 0,
        // State 17
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, -24, -24, -24, 0, 0, 0, 0, -24, 0, 0, 0, 0,
        // State 18
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, -25, -25, -25, 0, 0, 0, 0, -25, 0, 0, 0, 0,
        // State 19
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, -26, -26, -26, 0, 0, 0, 0, -26, 0, 0, 0, 0,
        // State 20
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, -27, -27, -27, 0, 0, 0, 0, -27, 0, 0, 0, 0,
        // State 21
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, -28, -28, -28, 0, 0, 0, 0, -28, 0, 0, 0, 0,
        // State 22
        35, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 23
        0, 0, 0, 0, 0, 0, 0, -31, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 24
        0, -11, -11, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 25
        0, -17, -17, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 26
        0, 36, 37, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 27
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 38, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 28
        0, -23, -23, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, -23, -23, -23, 0, 0, 0, 0, -23, 0, 0, 0, 0,
        // State 29
        0, -10, -10, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 31, 32, 33, 0, 0, 0, 0, 34, 0, 0, 0, 0,
        // State 30
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 40, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 31
        0, -18, -18, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, -18, -18, -18, 0, 0, 0, 0, -18, 0, 0, 0, 0,
        // State 32
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 41, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 33
        0, -20, -20, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, -20, -20, -20, 0, 0, 0, 0, -20, 0, 0, 0, 0,
        // State 34
        0, 0, 0, 0, 0, 0, 0, 0, 42, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 35
        0, 0, 0, 0, 43, 0, 0, -32, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 36
        0, 0, 0, 0, 0, 0, 28, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 37
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 45, 0, 0, 0, 0, 0, 0, 46, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 38
        0, -22, -22, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, -22, -22, -22, 0, 0, 0, 0, -22, 0, 0, 0, 0,
        // State 39
        0, -19, -19, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, -19, -19, -19, 0, 0, 0, 0, -19, 0, 0, 0, 0,
        // State 40
        0, -21, -21, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, -21, -21, -21, 0, 0, 0, 0, -21, 0, 0, 0, 0,
        // State 41
        0, 47, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 42
        0, 0, 0, 0, 0, 0, 0, -30, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 43
        0, -16, -16, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 44
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 48, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 45
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 49, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 46
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, -29, -29, -29, 0, 0, 0, 0, -29, 0, 0, 0, 0,
        // State 47
        50, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 48
        51, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 49
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 52, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 50
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 53, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 51
        0, 54, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 52
        0, 55, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 53
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 56, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 54
        0, -14, -14, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 58, 0,
        // State 55
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 56
        0, -13, -13, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 57
        60, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 58
        61, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 59
        0, 0, 0, 0, 0, 0, 0, 0, 0, 64, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 60
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 65, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 61
        0, -40, -40, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 62
        0, 66, 67, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 63
        0, 0, 0, 0, 0, 68, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 64
        0, 69, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 65
        0, -41, -41, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 66
        0, 0, 0, 0, 0, 0, 0, 0, 0, 64, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 67
        0, 0, 0, 0, 0, 0, 0, 0, 71, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 68
        0, -15, -15, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 69
        0, -39, -39, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 70
        0, -38, -38, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ];
    const __EOF_ACTION: &'static [i32] = &[
        0,
        -35,
        -9,
        0,
        -34,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        -33,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        -31,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        -32,
        0,
        0,
        0,
        0,
        0,
        0,
        -30,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
    ];
    const __GOTO: &'static [i32] = &[
        // State 0
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 3, 0, 0, 0, 0, 0,
        // State 1
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 2
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0,
        // State 3
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 4
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 5
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 7, 0, 0, 0, 0,
        // State 6
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 7
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 8
        0, 0, 0, 0, 0, 0, 11, 12, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 9
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 10
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 11
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 12
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 17, 0, 0, 0, 0, 0, 0, 0,
        // State 13
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 14
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 15
        0, 0, 0, 0, 0, 0, 25, 0, 26, 27, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 16
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 29, 30, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 17
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 18
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 19
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 20
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 21
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 22
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 23
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 24
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 25
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 26
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 27
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 28
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 29
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 39, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 30
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 31
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 32
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 33
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 34
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 35
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 36
        0, 0, 0, 0, 0, 0, 0, 0, 44, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 37
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 38
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 39
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 40
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 41
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 42
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 43
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 44
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 45
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 46
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 47
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 48
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 49
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 50
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 51
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 52
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 53
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 54
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 57, 0,
        // State 55
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 59, 0, 0, 0, 0,
        // State 56
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 57
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 58
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 59
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 62, 63, 0, 0,
        // State 60
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 61
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 62
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 63
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 64
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 65
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 66
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 70, 0, 0, 0,
        // State 67
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 68
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 69
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 70
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ];
    fn __expected_tokens(__state: usize) -> Vec<::std::string::String> {
        const __TERMINAL: &'static [&'static str] = &[
            r###""(""###,
            r###"")""###,
            r###"",""###,
            r###"".""###,
            r###"";""###,
            r###""=""###,
            r###"CONSTRAINT"###,
            r###"CREATE"###,
            r###"Digit"###,
            r###"FILLFACTOR"###,
            r###"FOREIGN"###,
            r###"INT"###,
            r###"INTEGER"###,
            r###"Ident"###,
            r###"KEY"###,
            r###"NOT"###,
            r###"NULL"###,
            r###"PRIMARY"###,
            r###"REFERENCES"###,
            r###"SERIAL"###,
            r###"SMALLINT"###,
            r###"TABLE"###,
            r###"UNIQUE"###,
            r###"UUID"###,
            r###"VARCHAR"###,
            r###"WITH"###,
        ];
        __ACTION[(__state * 27)..].iter().zip(__TERMINAL).filter_map(|(&state, terminal)| {
            if state == 0 {
                None
            } else {
                Some(terminal.to_string())
            }
        }).collect()
    }
    pub fn parse_statement_list<
        'input,
        __TOKEN: __ToTriple<'input, Error=()>,
        __TOKENS: IntoIterator<Item=__TOKEN>,
    >(
        __tokens0: __TOKENS,
    ) -> Result<Vec<Statement>, __lalrpop_util::ParseError<(), lexer::Token, ()>>
    {
        let __tokens = __tokens0.into_iter();
        let mut __tokens = __tokens.map(|t| __ToTriple::to_triple(t));
        let mut __states = vec![0_i32];
        let mut __symbols = vec![];
        let mut __integer;
        let mut __lookahead;
        let mut __last_location = Default::default();
        '__shift: loop {
            __lookahead = match __tokens.next() {
                Some(Ok(v)) => v,
                None => break '__shift,
                Some(Err(e)) => return Err(__lalrpop_util::ParseError::User { error: e }),
            };
            __last_location = __lookahead.2.clone();
            __integer = match __lookahead.1 {
                lexer::Token::LeftBracket if true => 0,
                lexer::Token::RightBracket if true => 1,
                lexer::Token::Comma if true => 2,
                lexer::Token::Period if true => 3,
                lexer::Token::Semicolon if true => 4,
                lexer::Token::Equals if true => 5,
                lexer::Token::CONSTRAINT if true => 6,
                lexer::Token::CREATE if true => 7,
                lexer::Token::Digit(_) if true => 8,
                lexer::Token::FILLFACTOR if true => 9,
                lexer::Token::FOREIGN if true => 10,
                lexer::Token::INT if true => 11,
                lexer::Token::INTEGER if true => 12,
                lexer::Token::Identifier(_) if true => 13,
                lexer::Token::KEY if true => 14,
                lexer::Token::NOT if true => 15,
                lexer::Token::NULL if true => 16,
                lexer::Token::PRIMARY if true => 17,
                lexer::Token::REFERENCES if true => 18,
                lexer::Token::SERIAL if true => 19,
                lexer::Token::SMALLINT if true => 20,
                lexer::Token::TABLE if true => 21,
                lexer::Token::UNIQUE if true => 22,
                lexer::Token::UUID if true => 23,
                lexer::Token::VARCHAR if true => 24,
                lexer::Token::WITH if true => 25,
                _ => {
                    let __state = *__states.last().unwrap() as usize;
                    let __error = __lalrpop_util::ParseError::UnrecognizedToken {
                        token: Some(__lookahead),
                        expected: __expected_tokens(__state),
                    };
                    return Err(__error);
                }
            };
            '__inner: loop {
                let __state = *__states.last().unwrap() as usize;
                let __action = __ACTION[__state * 27 + __integer];
                if __action > 0 {
                    let __symbol = match __integer {
                        0 => match __lookahead.1 {
                            __tok @ lexer::Token::LeftBracket => __Symbol::Term_22_28_22(__tok),
                            _ => unreachable!(),
                        },
                        1 => match __lookahead.1 {
                            __tok @ lexer::Token::RightBracket => __Symbol::Term_22_29_22(__tok),
                            _ => unreachable!(),
                        },
                        2 => match __lookahead.1 {
                            __tok @ lexer::Token::Comma => __Symbol::Term_22_2c_22(__tok),
                            _ => unreachable!(),
                        },
                        3 => match __lookahead.1 {
                            __tok @ lexer::Token::Period => __Symbol::Term_22_2e_22(__tok),
                            _ => unreachable!(),
                        },
                        4 => match __lookahead.1 {
                            __tok @ lexer::Token::Semicolon => __Symbol::Term_22_3b_22(__tok),
                            _ => unreachable!(),
                        },
                        5 => match __lookahead.1 {
                            __tok @ lexer::Token::Equals => __Symbol::Term_22_3d_22(__tok),
                            _ => unreachable!(),
                        },
                        6 => match __lookahead.1 {
                            __tok @ lexer::Token::CONSTRAINT => __Symbol::TermCONSTRAINT(__tok),
                            _ => unreachable!(),
                        },
                        7 => match __lookahead.1 {
                            __tok @ lexer::Token::CREATE => __Symbol::TermCREATE(__tok),
                            _ => unreachable!(),
                        },
                        8 => match __lookahead.1 {
                            lexer::Token::Digit(__tok0) => __Symbol::TermDigit(__tok0),
                            _ => unreachable!(),
                        },
                        9 => match __lookahead.1 {
                            __tok @ lexer::Token::FILLFACTOR => __Symbol::TermFILLFACTOR(__tok),
                            _ => unreachable!(),
                        },
                        10 => match __lookahead.1 {
                            __tok @ lexer::Token::FOREIGN => __Symbol::TermFOREIGN(__tok),
                            _ => unreachable!(),
                        },
                        11 => match __lookahead.1 {
                            __tok @ lexer::Token::INT => __Symbol::TermINT(__tok),
                            _ => unreachable!(),
                        },
                        12 => match __lookahead.1 {
                            __tok @ lexer::Token::INTEGER => __Symbol::TermINTEGER(__tok),
                            _ => unreachable!(),
                        },
                        13 => match __lookahead.1 {
                            lexer::Token::Identifier(__tok0) => __Symbol::TermIdent(__tok0),
                            _ => unreachable!(),
                        },
                        14 => match __lookahead.1 {
                            __tok @ lexer::Token::KEY => __Symbol::TermKEY(__tok),
                            _ => unreachable!(),
                        },
                        15 => match __lookahead.1 {
                            __tok @ lexer::Token::NOT => __Symbol::TermNOT(__tok),
                            _ => unreachable!(),
                        },
                        16 => match __lookahead.1 {
                            __tok @ lexer::Token::NULL => __Symbol::TermNULL(__tok),
                            _ => unreachable!(),
                        },
                        17 => match __lookahead.1 {
                            __tok @ lexer::Token::PRIMARY => __Symbol::TermPRIMARY(__tok),
                            _ => unreachable!(),
                        },
                        18 => match __lookahead.1 {
                            __tok @ lexer::Token::REFERENCES => __Symbol::TermREFERENCES(__tok),
                            _ => unreachable!(),
                        },
                        19 => match __lookahead.1 {
                            __tok @ lexer::Token::SERIAL => __Symbol::TermSERIAL(__tok),
                            _ => unreachable!(),
                        },
                        20 => match __lookahead.1 {
                            __tok @ lexer::Token::SMALLINT => __Symbol::TermSMALLINT(__tok),
                            _ => unreachable!(),
                        },
                        21 => match __lookahead.1 {
                            __tok @ lexer::Token::TABLE => __Symbol::TermTABLE(__tok),
                            _ => unreachable!(),
                        },
                        22 => match __lookahead.1 {
                            __tok @ lexer::Token::UNIQUE => __Symbol::TermUNIQUE(__tok),
                            _ => unreachable!(),
                        },
                        23 => match __lookahead.1 {
                            __tok @ lexer::Token::UUID => __Symbol::TermUUID(__tok),
                            _ => unreachable!(),
                        },
                        24 => match __lookahead.1 {
                            __tok @ lexer::Token::VARCHAR => __Symbol::TermVARCHAR(__tok),
                            _ => unreachable!(),
                        },
                        25 => match __lookahead.1 {
                            __tok @ lexer::Token::WITH => __Symbol::TermWITH(__tok),
                            _ => unreachable!(),
                        },
                        _ => unreachable!(),
                    };
                    __states.push(__action - 1);
                    __symbols.push((__lookahead.0, __symbol, __lookahead.2));
                    continue '__shift;
                } else if __action < 0 {
                    if let Some(r) = __reduce(__action, Some(&__lookahead.0), &mut __states, &mut __symbols, ::std::marker::PhantomData::<()>) {
                        return r;
                    }
                } else {
                    let __state = *__states.last().unwrap() as usize;
                    let __error = __lalrpop_util::ParseError::UnrecognizedToken {
                        token: Some(__lookahead),
                        expected: __expected_tokens(__state),
                    };
                    return Err(__error)
                }
            }
        }
        loop {
            let __state = *__states.last().unwrap() as usize;
            let __action = __EOF_ACTION[__state];
            if __action < 0 {
                if let Some(r) = __reduce(__action, None, &mut __states, &mut __symbols, ::std::marker::PhantomData::<()>) {
                    return r;
                }
            } else {
                let __state = *__states.last().unwrap() as usize;
                let __error = __lalrpop_util::ParseError::UnrecognizedToken {
                    token: None,
                    expected: __expected_tokens(__state),
                };
                return Err(__error);
            }
        }
    }
    pub fn __reduce<
        'input,
    >(
        __action: i32,
        __lookahead_start: Option<&()>,
        __states: &mut ::std::vec::Vec<i32>,
        __symbols: &mut ::std::vec::Vec<((),__Symbol<>,())>,
        _: ::std::marker::PhantomData<()>,
    ) -> Option<Result<Vec<Statement>,__lalrpop_util::ParseError<(), lexer::Token, ()>>>
    {
        let __nonterminal = match -__action {
            1 => {
                // ";"? = ";" => ActionFn(33);
                let __sym0 = __pop_Term_22_3b_22(__symbols);
                let __start = __sym0.0.clone();
                let __end = __sym0.2.clone();
                let __nt = super::__action33::<>(__sym0);
                let __states_len = __states.len();
                __states.truncate(__states_len - 1);
                __symbols.push((__start, __Symbol::Nt_22_3b_22_3f(__nt), __end));
                0
            }
            2 => {
                // ";"? =  => ActionFn(34);
                let __start = __symbols.last().map(|s| s.2.clone()).unwrap_or_default();
                let __end = __lookahead_start.cloned().unwrap_or_else(|| __start.clone());
                let __nt = super::__action34::<>(&__start, &__end);
                let __states_len = __states.len();
                __states.truncate(__states_len - 0);
                __symbols.push((__start, __Symbol::Nt_22_3b_22_3f(__nt), __end));
                0
            }
            3 => {
                // ("," constraint_list) = ",", constraint_list => ActionFn(37);
                let __sym1 = __pop_Ntconstraint__list(__symbols);
                let __sym0 = __pop_Term_22_2c_22(__symbols);
                let __start = __sym0.0.clone();
                let __end = __sym1.2.clone();
                let __nt = super::__action37::<>(__sym0, __sym1);
                let __states_len = __states.len();
                __states.truncate(__states_len - 2);
                __symbols.push((__start, __Symbol::Nt_28_22_2c_22_20constraint__list_29(__nt), __end));
                1
            }
            4 => {
                // ("," constraint_list)? = ",", constraint_list => ActionFn(40);
                let __sym1 = __pop_Ntconstraint__list(__symbols);
                let __sym0 = __pop_Term_22_2c_22(__symbols);
                let __start = __sym0.0.clone();
                let __end = __sym1.2.clone();
                let __nt = super::__action40::<>(__sym0, __sym1);
                let __states_len = __states.len();
                __states.truncate(__states_len - 2);
                __symbols.push((__start, __Symbol::Nt_28_22_2c_22_20constraint__list_29_3f(__nt), __end));
                2
            }
            5 => {
                // ("," constraint_list)? =  => ActionFn(36);
                let __start = __symbols.last().map(|s| s.2.clone()).unwrap_or_default();
                let __end = __lookahead_start.cloned().unwrap_or_else(|| __start.clone());
                let __nt = super::__action36::<>(&__start, &__end);
                let __states_len = __states.len();
                __states.truncate(__states_len - 0);
                __symbols.push((__start, __Symbol::Nt_28_22_2c_22_20constraint__list_29_3f(__nt), __end));
                2
            }
            6 => {
                // (Ident ".") = Ident, "." => ActionFn(32);
                let __sym1 = __pop_Term_22_2e_22(__symbols);
                let __sym0 = __pop_TermIdent(__symbols);
                let __start = __sym0.0.clone();
                let __end = __sym1.2.clone();
                let __nt = super::__action32::<>(__sym0, __sym1);
                let __states_len = __states.len();
                __states.truncate(__states_len - 2);
                __symbols.push((__start, __Symbol::Nt_28Ident_20_22_2e_22_29(__nt), __end));
                3
            }
            7 => {
                // (Ident ".")? = Ident, "." => ActionFn(45);
                let __sym1 = __pop_Term_22_2e_22(__symbols);
                let __sym0 = __pop_TermIdent(__symbols);
                let __start = __sym0.0.clone();
                let __end = __sym1.2.clone();
                let __nt = super::__action45::<>(__sym0, __sym1);
                let __states_len = __states.len();
                __states.truncate(__states_len - 2);
                __symbols.push((__start, __Symbol::Nt_28Ident_20_22_2e_22_29_3f(__nt), __end));
                4
            }
            8 => {
                // (Ident ".")? =  => ActionFn(31);
                let __start = __symbols.last().map(|s| s.2.clone()).unwrap_or_default();
                let __end = __lookahead_start.cloned().unwrap_or_else(|| __start.clone());
                let __nt = super::__action31::<>(&__start, &__end);
                let __states_len = __states.len();
                __states.truncate(__states_len - 0);
                __symbols.push((__start, __Symbol::Nt_28Ident_20_22_2e_22_29_3f(__nt), __end));
                4
            }
            9 => {
                // __statement_list = statement_list => ActionFn(0);
                let __sym0 = __pop_Ntstatement__list(__symbols);
                let __start = __sym0.0.clone();
                let __end = __sym0.2.clone();
                let __nt = super::__action0::<>(__sym0);
                return Some(Ok(__nt));
            }
            10 => {
                // column_definition = Ident, sql_type, qualifier_list => ActionFn(7);
                let __sym2 = __pop_Ntqualifier__list(__symbols);
                let __sym1 = __pop_Ntsql__type(__symbols);
                let __sym0 = __pop_TermIdent(__symbols);
                let __start = __sym0.0.clone();
                let __end = __sym2.2.clone();
                let __nt = super::__action7::<>(__sym0, __sym1, __sym2);
                let __states_len = __states.len();
                __states.truncate(__states_len - 3);
                __symbols.push((__start, __Symbol::Ntcolumn__definition(__nt), __end));
                6
            }
            11 => {
                // column_definition_list = column_definition_list, ",", column_definition => ActionFn(5);
                let __sym2 = __pop_Ntcolumn__definition(__symbols);
                let __sym1 = __pop_Term_22_2c_22(__symbols);
                let __sym0 = __pop_Ntcolumn__definition__list(__symbols);
                let __start = __sym0.0.clone();
                let __end = __sym2.2.clone();
                let __nt = super::__action5::<>(__sym0, __sym1, __sym2);
                let __states_len = __states.len();
                __states.truncate(__states_len - 3);
                __symbols.push((__start, __Symbol::Ntcolumn__definition__list(__nt), __end));
                7
            }
            12 => {
                // column_definition_list = column_definition => ActionFn(6);
                let __sym0 = __pop_Ntcolumn__definition(__symbols);
                let __start = __sym0.0.clone();
                let __end = __sym0.2.clone();
                let __nt = super::__action6::<>(__sym0);
                let __states_len = __states.len();
                __states.truncate(__states_len - 1);
                __symbols.push((__start, __Symbol::Ntcolumn__definition__list(__nt), __end));
                7
            }
            13 => {
                // constraint = CONSTRAINT, Ident, PRIMARY, KEY, "(", Ident, ")", with_qualifier => ActionFn(48);
                let __sym7 = __pop_Ntwith__qualifier(__symbols);
                let __sym6 = __pop_Term_22_29_22(__symbols);
                let __sym5 = __pop_TermIdent(__symbols);
                let __sym4 = __pop_Term_22_28_22(__symbols);
                let __sym3 = __pop_TermKEY(__symbols);
                let __sym2 = __pop_TermPRIMARY(__symbols);
                let __sym1 = __pop_TermIdent(__symbols);
                let __sym0 = __pop_TermCONSTRAINT(__symbols);
                let __start = __sym0.0.clone();
                let __end = __sym7.2.clone();
                let __nt = super::__action48::<>(__sym0, __sym1, __sym2, __sym3, __sym4, __sym5, __sym6, __sym7);
                let __states_len = __states.len();
                __states.truncate(__states_len - 8);
                __symbols.push((__start, __Symbol::Ntconstraint(__nt), __end));
                8
            }
            14 => {
                // constraint = CONSTRAINT, Ident, PRIMARY, KEY, "(", Ident, ")" => ActionFn(49);
                let __sym6 = __pop_Term_22_29_22(__symbols);
                let __sym5 = __pop_TermIdent(__symbols);
                let __sym4 = __pop_Term_22_28_22(__symbols);
                let __sym3 = __pop_TermKEY(__symbols);
                let __sym2 = __pop_TermPRIMARY(__symbols);
                let __sym1 = __pop_TermIdent(__symbols);
                let __sym0 = __pop_TermCONSTRAINT(__symbols);
                let __start = __sym0.0.clone();
                let __end = __sym6.2.clone();
                let __nt = super::__action49::<>(__sym0, __sym1, __sym2, __sym3, __sym4, __sym5, __sym6);
                let __states_len = __states.len();
                __states.truncate(__states_len - 7);
                __symbols.push((__start, __Symbol::Ntconstraint(__nt), __end));
                8
            }
            15 => {
                // constraint = CONSTRAINT, Ident, FOREIGN, KEY, "(", Ident, ")", REFERENCES, table_name, "(", Ident, ")" => ActionFn(11);
                let __sym11 = __pop_Term_22_29_22(__symbols);
                let __sym10 = __pop_TermIdent(__symbols);
                let __sym9 = __pop_Term_22_28_22(__symbols);
                let __sym8 = __pop_Nttable__name(__symbols);
                let __sym7 = __pop_TermREFERENCES(__symbols);
                let __sym6 = __pop_Term_22_29_22(__symbols);
                let __sym5 = __pop_TermIdent(__symbols);
                let __sym4 = __pop_Term_22_28_22(__symbols);
                let __sym3 = __pop_TermKEY(__symbols);
                let __sym2 = __pop_TermFOREIGN(__symbols);
                let __sym1 = __pop_TermIdent(__symbols);
                let __sym0 = __pop_TermCONSTRAINT(__symbols);
                let __start = __sym0.0.clone();
                let __end = __sym11.2.clone();
                let __nt = super::__action11::<>(__sym0, __sym1, __sym2, __sym3, __sym4, __sym5, __sym6, __sym7, __sym8, __sym9, __sym10, __sym11);
                let __states_len = __states.len();
                __states.truncate(__states_len - 12);
                __symbols.push((__start, __Symbol::Ntconstraint(__nt), __end));
                8
            }
            16 => {
                // constraint_list = constraint_list, ",", constraint => ActionFn(8);
                let __sym2 = __pop_Ntconstraint(__symbols);
                let __sym1 = __pop_Term_22_2c_22(__symbols);
                let __sym0 = __pop_Ntconstraint__list(__symbols);
                let __start = __sym0.0.clone();
                let __end = __sym2.2.clone();
                let __nt = super::__action8::<>(__sym0, __sym1, __sym2);
                let __states_len = __states.len();
                __states.truncate(__states_len - 3);
                __symbols.push((__start, __Symbol::Ntconstraint__list(__nt), __end));
                9
            }
            17 => {
                // constraint_list = constraint => ActionFn(9);
                let __sym0 = __pop_Ntconstraint(__symbols);
                let __start = __sym0.0.clone();
                let __end = __sym0.2.clone();
                let __nt = super::__action9::<>(__sym0);
                let __states_len = __states.len();
                __states.truncate(__states_len - 1);
                __symbols.push((__start, __Symbol::Ntconstraint__list(__nt), __end));
                9
            }
            18 => {
                // qualifier = NULL => ActionFn(24);
                let __sym0 = __pop_TermNULL(__symbols);
                let __start = __sym0.0.clone();
                let __end = __sym0.2.clone();
                let __nt = super::__action24::<>(__sym0);
                let __states_len = __states.len();
                __states.truncate(__states_len - 1);
                __symbols.push((__start, __Symbol::Ntqualifier(__nt), __end));
                10
            }
            19 => {
                // qualifier = NOT, NULL => ActionFn(25);
                let __sym1 = __pop_TermNULL(__symbols);
                let __sym0 = __pop_TermNOT(__symbols);
                let __start = __sym0.0.clone();
                let __end = __sym1.2.clone();
                let __nt = super::__action25::<>(__sym0, __sym1);
                let __states_len = __states.len();
                __states.truncate(__states_len - 2);
                __symbols.push((__start, __Symbol::Ntqualifier(__nt), __end));
                10
            }
            20 => {
                // qualifier = UNIQUE => ActionFn(26);
                let __sym0 = __pop_TermUNIQUE(__symbols);
                let __start = __sym0.0.clone();
                let __end = __sym0.2.clone();
                let __nt = super::__action26::<>(__sym0);
                let __states_len = __states.len();
                __states.truncate(__states_len - 1);
                __symbols.push((__start, __Symbol::Ntqualifier(__nt), __end));
                10
            }
            21 => {
                // qualifier = PRIMARY, KEY => ActionFn(27);
                let __sym1 = __pop_TermKEY(__symbols);
                let __sym0 = __pop_TermPRIMARY(__symbols);
                let __start = __sym0.0.clone();
                let __end = __sym1.2.clone();
                let __nt = super::__action27::<>(__sym0, __sym1);
                let __states_len = __states.len();
                __states.truncate(__states_len - 2);
                __symbols.push((__start, __Symbol::Ntqualifier(__nt), __end));
                10
            }
            22 => {
                // qualifier_list = qualifier_list, qualifier => ActionFn(22);
                let __sym1 = __pop_Ntqualifier(__symbols);
                let __sym0 = __pop_Ntqualifier__list(__symbols);
                let __start = __sym0.0.clone();
                let __end = __sym1.2.clone();
                let __nt = super::__action22::<>(__sym0, __sym1);
                let __states_len = __states.len();
                __states.truncate(__states_len - 2);
                __symbols.push((__start, __Symbol::Ntqualifier__list(__nt), __end));
                11
            }
            23 => {
                // qualifier_list = qualifier => ActionFn(23);
                let __sym0 = __pop_Ntqualifier(__symbols);
                let __start = __sym0.0.clone();
                let __end = __sym0.2.clone();
                let __nt = super::__action23::<>(__sym0);
                let __states_len = __states.len();
                __states.truncate(__states_len - 1);
                __symbols.push((__start, __Symbol::Ntqualifier__list(__nt), __end));
                11
            }
            24 => {
                // sql_type = INT => ActionFn(16);
                let __sym0 = __pop_TermINT(__symbols);
                let __start = __sym0.0.clone();
                let __end = __sym0.2.clone();
                let __nt = super::__action16::<>(__sym0);
                let __states_len = __states.len();
                __states.truncate(__states_len - 1);
                __symbols.push((__start, __Symbol::Ntsql__type(__nt), __end));
                12
            }
            25 => {
                // sql_type = INTEGER => ActionFn(17);
                let __sym0 = __pop_TermINTEGER(__symbols);
                let __start = __sym0.0.clone();
                let __end = __sym0.2.clone();
                let __nt = super::__action17::<>(__sym0);
                let __states_len = __states.len();
                __states.truncate(__states_len - 1);
                __symbols.push((__start, __Symbol::Ntsql__type(__nt), __end));
                12
            }
            26 => {
                // sql_type = SERIAL => ActionFn(18);
                let __sym0 = __pop_TermSERIAL(__symbols);
                let __start = __sym0.0.clone();
                let __end = __sym0.2.clone();
                let __nt = super::__action18::<>(__sym0);
                let __states_len = __states.len();
                __states.truncate(__states_len - 1);
                __symbols.push((__start, __Symbol::Ntsql__type(__nt), __end));
                12
            }
            27 => {
                // sql_type = SMALLINT => ActionFn(19);
                let __sym0 = __pop_TermSMALLINT(__symbols);
                let __start = __sym0.0.clone();
                let __end = __sym0.2.clone();
                let __nt = super::__action19::<>(__sym0);
                let __states_len = __states.len();
                __states.truncate(__states_len - 1);
                __symbols.push((__start, __Symbol::Ntsql__type(__nt), __end));
                12
            }
            28 => {
                // sql_type = UUID => ActionFn(20);
                let __sym0 = __pop_TermUUID(__symbols);
                let __start = __sym0.0.clone();
                let __end = __sym0.2.clone();
                let __nt = super::__action20::<>(__sym0);
                let __states_len = __states.len();
                __states.truncate(__states_len - 1);
                __symbols.push((__start, __Symbol::Ntsql__type(__nt), __end));
                12
            }
            29 => {
                // sql_type = VARCHAR, "(", Digit, ")" => ActionFn(21);
                let __sym3 = __pop_Term_22_29_22(__symbols);
                let __sym2 = __pop_TermDigit(__symbols);
                let __sym1 = __pop_Term_22_28_22(__symbols);
                let __sym0 = __pop_TermVARCHAR(__symbols);
                let __start = __sym0.0.clone();
                let __end = __sym3.2.clone();
                let __nt = super::__action21::<>(__sym0, __sym1, __sym2, __sym3);
                let __states_len = __states.len();
                __states.truncate(__states_len - 4);
                __symbols.push((__start, __Symbol::Ntsql__type(__nt), __end));
                12
            }
            30 => {
                // statement = CREATE, TABLE, table_name, "(", column_definition_list, ",", constraint_list, ")", ";" => ActionFn(41);
                let __sym8 = __pop_Term_22_3b_22(__symbols);
                let __sym7 = __pop_Term_22_29_22(__symbols);
                let __sym6 = __pop_Ntconstraint__list(__symbols);
                let __sym5 = __pop_Term_22_2c_22(__symbols);
                let __sym4 = __pop_Ntcolumn__definition__list(__symbols);
                let __sym3 = __pop_Term_22_28_22(__symbols);
                let __sym2 = __pop_Nttable__name(__symbols);
                let __sym1 = __pop_TermTABLE(__symbols);
                let __sym0 = __pop_TermCREATE(__symbols);
                let __start = __sym0.0.clone();
                let __end = __sym8.2.clone();
                let __nt = super::__action41::<>(__sym0, __sym1, __sym2, __sym3, __sym4, __sym5, __sym6, __sym7, __sym8);
                let __states_len = __states.len();
                __states.truncate(__states_len - 9);
                __symbols.push((__start, __Symbol::Ntstatement(__nt), __end));
                13
            }
            31 => {
                // statement = CREATE, TABLE, table_name, "(", column_definition_list, ")", ";" => ActionFn(42);
                let __sym6 = __pop_Term_22_3b_22(__symbols);
                let __sym5 = __pop_Term_22_29_22(__symbols);
                let __sym4 = __pop_Ntcolumn__definition__list(__symbols);
                let __sym3 = __pop_Term_22_28_22(__symbols);
                let __sym2 = __pop_Nttable__name(__symbols);
                let __sym1 = __pop_TermTABLE(__symbols);
                let __sym0 = __pop_TermCREATE(__symbols);
                let __start = __sym0.0.clone();
                let __end = __sym6.2.clone();
                let __nt = super::__action42::<>(__sym0, __sym1, __sym2, __sym3, __sym4, __sym5, __sym6);
                let __states_len = __states.len();
                __states.truncate(__states_len - 7);
                __symbols.push((__start, __Symbol::Ntstatement(__nt), __end));
                13
            }
            32 => {
                // statement = CREATE, TABLE, table_name, "(", column_definition_list, ",", constraint_list, ")" => ActionFn(43);
                let __sym7 = __pop_Term_22_29_22(__symbols);
                let __sym6 = __pop_Ntconstraint__list(__symbols);
                let __sym5 = __pop_Term_22_2c_22(__symbols);
                let __sym4 = __pop_Ntcolumn__definition__list(__symbols);
                let __sym3 = __pop_Term_22_28_22(__symbols);
                let __sym2 = __pop_Nttable__name(__symbols);
                let __sym1 = __pop_TermTABLE(__symbols);
                let __sym0 = __pop_TermCREATE(__symbols);
                let __start = __sym0.0.clone();
                let __end = __sym7.2.clone();
                let __nt = super::__action43::<>(__sym0, __sym1, __sym2, __sym3, __sym4, __sym5, __sym6, __sym7);
                let __states_len = __states.len();
                __states.truncate(__states_len - 8);
                __symbols.push((__start, __Symbol::Ntstatement(__nt), __end));
                13
            }
            33 => {
                // statement = CREATE, TABLE, table_name, "(", column_definition_list, ")" => ActionFn(44);
                let __sym5 = __pop_Term_22_29_22(__symbols);
                let __sym4 = __pop_Ntcolumn__definition__list(__symbols);
                let __sym3 = __pop_Term_22_28_22(__symbols);
                let __sym2 = __pop_Nttable__name(__symbols);
                let __sym1 = __pop_TermTABLE(__symbols);
                let __sym0 = __pop_TermCREATE(__symbols);
                let __start = __sym0.0.clone();
                let __end = __sym5.2.clone();
                let __nt = super::__action44::<>(__sym0, __sym1, __sym2, __sym3, __sym4, __sym5);
                let __states_len = __states.len();
                __states.truncate(__states_len - 6);
                __symbols.push((__start, __Symbol::Ntstatement(__nt), __end));
                13
            }
            34 => {
                // statement_list = statement_list, statement => ActionFn(1);
                let __sym1 = __pop_Ntstatement(__symbols);
                let __sym0 = __pop_Ntstatement__list(__symbols);
                let __start = __sym0.0.clone();
                let __end = __sym1.2.clone();
                let __nt = super::__action1::<>(__sym0, __sym1);
                let __states_len = __states.len();
                __states.truncate(__states_len - 2);
                __symbols.push((__start, __Symbol::Ntstatement__list(__nt), __end));
                14
            }
            35 => {
                // statement_list = statement => ActionFn(2);
                let __sym0 = __pop_Ntstatement(__symbols);
                let __start = __sym0.0.clone();
                let __end = __sym0.2.clone();
                let __nt = super::__action2::<>(__sym0);
                let __states_len = __states.len();
                __states.truncate(__states_len - 1);
                __symbols.push((__start, __Symbol::Ntstatement__list(__nt), __end));
                14
            }
            36 => {
                // table_name = Ident, ".", Ident => ActionFn(46);
                let __sym2 = __pop_TermIdent(__symbols);
                let __sym1 = __pop_Term_22_2e_22(__symbols);
                let __sym0 = __pop_TermIdent(__symbols);
                let __start = __sym0.0.clone();
                let __end = __sym2.2.clone();
                let __nt = super::__action46::<>(__sym0, __sym1, __sym2);
                let __states_len = __states.len();
                __states.truncate(__states_len - 3);
                __symbols.push((__start, __Symbol::Nttable__name(__nt), __end));
                15
            }
            37 => {
                // table_name = Ident => ActionFn(47);
                let __sym0 = __pop_TermIdent(__symbols);
                let __start = __sym0.0.clone();
                let __end = __sym0.2.clone();
                let __nt = super::__action47::<>(__sym0);
                let __states_len = __states.len();
                __states.truncate(__states_len - 1);
                __symbols.push((__start, __Symbol::Nttable__name(__nt), __end));
                15
            }
            38 => {
                // with_option = FILLFACTOR, "=", Digit => ActionFn(15);
                let __sym2 = __pop_TermDigit(__symbols);
                let __sym1 = __pop_Term_22_3d_22(__symbols);
                let __sym0 = __pop_TermFILLFACTOR(__symbols);
                let __start = __sym0.0.clone();
                let __end = __sym2.2.clone();
                let __nt = super::__action15::<>(__sym0, __sym1, __sym2);
                let __states_len = __states.len();
                __states.truncate(__states_len - 3);
                __symbols.push((__start, __Symbol::Ntwith__option(__nt), __end));
                16
            }
            39 => {
                // with_option_list = with_option_list, ",", with_option => ActionFn(13);
                let __sym2 = __pop_Ntwith__option(__symbols);
                let __sym1 = __pop_Term_22_2c_22(__symbols);
                let __sym0 = __pop_Ntwith__option__list(__symbols);
                let __start = __sym0.0.clone();
                let __end = __sym2.2.clone();
                let __nt = super::__action13::<>(__sym0, __sym1, __sym2);
                let __states_len = __states.len();
                __states.truncate(__states_len - 3);
                __symbols.push((__start, __Symbol::Ntwith__option__list(__nt), __end));
                17
            }
            40 => {
                // with_option_list = with_option => ActionFn(14);
                let __sym0 = __pop_Ntwith__option(__symbols);
                let __start = __sym0.0.clone();
                let __end = __sym0.2.clone();
                let __nt = super::__action14::<>(__sym0);
                let __states_len = __states.len();
                __states.truncate(__states_len - 1);
                __symbols.push((__start, __Symbol::Ntwith__option__list(__nt), __end));
                17
            }
            41 => {
                // with_qualifier = WITH, "(", with_option_list, ")" => ActionFn(12);
                let __sym3 = __pop_Term_22_29_22(__symbols);
                let __sym2 = __pop_Ntwith__option__list(__symbols);
                let __sym1 = __pop_Term_22_28_22(__symbols);
                let __sym0 = __pop_TermWITH(__symbols);
                let __start = __sym0.0.clone();
                let __end = __sym3.2.clone();
                let __nt = super::__action12::<>(__sym0, __sym1, __sym2, __sym3);
                let __states_len = __states.len();
                __states.truncate(__states_len - 4);
                __symbols.push((__start, __Symbol::Ntwith__qualifier(__nt), __end));
                18
            }
            42 => {
                // with_qualifier? = with_qualifier => ActionFn(28);
                let __sym0 = __pop_Ntwith__qualifier(__symbols);
                let __start = __sym0.0.clone();
                let __end = __sym0.2.clone();
                let __nt = super::__action28::<>(__sym0);
                let __states_len = __states.len();
                __states.truncate(__states_len - 1);
                __symbols.push((__start, __Symbol::Ntwith__qualifier_3f(__nt), __end));
                19
            }
            43 => {
                // with_qualifier? =  => ActionFn(29);
                let __start = __symbols.last().map(|s| s.2.clone()).unwrap_or_default();
                let __end = __lookahead_start.cloned().unwrap_or_else(|| __start.clone());
                let __nt = super::__action29::<>(&__start, &__end);
                let __states_len = __states.len();
                __states.truncate(__states_len - 0);
                __symbols.push((__start, __Symbol::Ntwith__qualifier_3f(__nt), __end));
                19
            }
            _ => panic!("invalid action code {}", __action)
        };
        let __state = *__states.last().unwrap() as usize;
        let __next_state = __GOTO[__state * 20 + __nonterminal] - 1;
        __states.push(__next_state);
        None
    }
    fn __pop_Term_22_28_22<
    >(
        __symbols: &mut ::std::vec::Vec<((),__Symbol<>,())>
    ) -> ((), lexer::Token, ()) {
        match __symbols.pop().unwrap() {
            (__l, __Symbol::Term_22_28_22(__v), __r) => (__l, __v, __r),
            _ => panic!("symbol type mismatch")
        }
    }
    fn __pop_Term_22_29_22<
    >(
        __symbols: &mut ::std::vec::Vec<((),__Symbol<>,())>
    ) -> ((), lexer::Token, ()) {
        match __symbols.pop().unwrap() {
            (__l, __Symbol::Term_22_29_22(__v), __r) => (__l, __v, __r),
            _ => panic!("symbol type mismatch")
        }
    }
    fn __pop_Term_22_2c_22<
    >(
        __symbols: &mut ::std::vec::Vec<((),__Symbol<>,())>
    ) -> ((), lexer::Token, ()) {
        match __symbols.pop().unwrap() {
            (__l, __Symbol::Term_22_2c_22(__v), __r) => (__l, __v, __r),
            _ => panic!("symbol type mismatch")
        }
    }
    fn __pop_Term_22_2e_22<
    >(
        __symbols: &mut ::std::vec::Vec<((),__Symbol<>,())>
    ) -> ((), lexer::Token, ()) {
        match __symbols.pop().unwrap() {
            (__l, __Symbol::Term_22_2e_22(__v), __r) => (__l, __v, __r),
            _ => panic!("symbol type mismatch")
        }
    }
    fn __pop_Term_22_3b_22<
    >(
        __symbols: &mut ::std::vec::Vec<((),__Symbol<>,())>
    ) -> ((), lexer::Token, ()) {
        match __symbols.pop().unwrap() {
            (__l, __Symbol::Term_22_3b_22(__v), __r) => (__l, __v, __r),
            _ => panic!("symbol type mismatch")
        }
    }
    fn __pop_Term_22_3d_22<
    >(
        __symbols: &mut ::std::vec::Vec<((),__Symbol<>,())>
    ) -> ((), lexer::Token, ()) {
        match __symbols.pop().unwrap() {
            (__l, __Symbol::Term_22_3d_22(__v), __r) => (__l, __v, __r),
            _ => panic!("symbol type mismatch")
        }
    }
    fn __pop_TermCONSTRAINT<
    >(
        __symbols: &mut ::std::vec::Vec<((),__Symbol<>,())>
    ) -> ((), lexer::Token, ()) {
        match __symbols.pop().unwrap() {
            (__l, __Symbol::TermCONSTRAINT(__v), __r) => (__l, __v, __r),
            _ => panic!("symbol type mismatch")
        }
    }
    fn __pop_TermCREATE<
    >(
        __symbols: &mut ::std::vec::Vec<((),__Symbol<>,())>
    ) -> ((), lexer::Token, ()) {
        match __symbols.pop().unwrap() {
            (__l, __Symbol::TermCREATE(__v), __r) => (__l, __v, __r),
            _ => panic!("symbol type mismatch")
        }
    }
    fn __pop_TermDigit<
    >(
        __symbols: &mut ::std::vec::Vec<((),__Symbol<>,())>
    ) -> ((), i32, ()) {
        match __symbols.pop().unwrap() {
            (__l, __Symbol::TermDigit(__v), __r) => (__l, __v, __r),
            _ => panic!("symbol type mismatch")
        }
    }
    fn __pop_TermFILLFACTOR<
    >(
        __symbols: &mut ::std::vec::Vec<((),__Symbol<>,())>
    ) -> ((), lexer::Token, ()) {
        match __symbols.pop().unwrap() {
            (__l, __Symbol::TermFILLFACTOR(__v), __r) => (__l, __v, __r),
            _ => panic!("symbol type mismatch")
        }
    }
    fn __pop_TermFOREIGN<
    >(
        __symbols: &mut ::std::vec::Vec<((),__Symbol<>,())>
    ) -> ((), lexer::Token, ()) {
        match __symbols.pop().unwrap() {
            (__l, __Symbol::TermFOREIGN(__v), __r) => (__l, __v, __r),
            _ => panic!("symbol type mismatch")
        }
    }
    fn __pop_TermINT<
    >(
        __symbols: &mut ::std::vec::Vec<((),__Symbol<>,())>
    ) -> ((), lexer::Token, ()) {
        match __symbols.pop().unwrap() {
            (__l, __Symbol::TermINT(__v), __r) => (__l, __v, __r),
            _ => panic!("symbol type mismatch")
        }
    }
    fn __pop_TermINTEGER<
    >(
        __symbols: &mut ::std::vec::Vec<((),__Symbol<>,())>
    ) -> ((), lexer::Token, ()) {
        match __symbols.pop().unwrap() {
            (__l, __Symbol::TermINTEGER(__v), __r) => (__l, __v, __r),
            _ => panic!("symbol type mismatch")
        }
    }
    fn __pop_TermIdent<
    >(
        __symbols: &mut ::std::vec::Vec<((),__Symbol<>,())>
    ) -> ((), String, ()) {
        match __symbols.pop().unwrap() {
            (__l, __Symbol::TermIdent(__v), __r) => (__l, __v, __r),
            _ => panic!("symbol type mismatch")
        }
    }
    fn __pop_TermKEY<
    >(
        __symbols: &mut ::std::vec::Vec<((),__Symbol<>,())>
    ) -> ((), lexer::Token, ()) {
        match __symbols.pop().unwrap() {
            (__l, __Symbol::TermKEY(__v), __r) => (__l, __v, __r),
            _ => panic!("symbol type mismatch")
        }
    }
    fn __pop_TermNOT<
    >(
        __symbols: &mut ::std::vec::Vec<((),__Symbol<>,())>
    ) -> ((), lexer::Token, ()) {
        match __symbols.pop().unwrap() {
            (__l, __Symbol::TermNOT(__v), __r) => (__l, __v, __r),
            _ => panic!("symbol type mismatch")
        }
    }
    fn __pop_TermNULL<
    >(
        __symbols: &mut ::std::vec::Vec<((),__Symbol<>,())>
    ) -> ((), lexer::Token, ()) {
        match __symbols.pop().unwrap() {
            (__l, __Symbol::TermNULL(__v), __r) => (__l, __v, __r),
            _ => panic!("symbol type mismatch")
        }
    }
    fn __pop_TermPRIMARY<
    >(
        __symbols: &mut ::std::vec::Vec<((),__Symbol<>,())>
    ) -> ((), lexer::Token, ()) {
        match __symbols.pop().unwrap() {
            (__l, __Symbol::TermPRIMARY(__v), __r) => (__l, __v, __r),
            _ => panic!("symbol type mismatch")
        }
    }
    fn __pop_TermREFERENCES<
    >(
        __symbols: &mut ::std::vec::Vec<((),__Symbol<>,())>
    ) -> ((), lexer::Token, ()) {
        match __symbols.pop().unwrap() {
            (__l, __Symbol::TermREFERENCES(__v), __r) => (__l, __v, __r),
            _ => panic!("symbol type mismatch")
        }
    }
    fn __pop_TermSERIAL<
    >(
        __symbols: &mut ::std::vec::Vec<((),__Symbol<>,())>
    ) -> ((), lexer::Token, ()) {
        match __symbols.pop().unwrap() {
            (__l, __Symbol::TermSERIAL(__v), __r) => (__l, __v, __r),
            _ => panic!("symbol type mismatch")
        }
    }
    fn __pop_TermSMALLINT<
    >(
        __symbols: &mut ::std::vec::Vec<((),__Symbol<>,())>
    ) -> ((), lexer::Token, ()) {
        match __symbols.pop().unwrap() {
            (__l, __Symbol::TermSMALLINT(__v), __r) => (__l, __v, __r),
            _ => panic!("symbol type mismatch")
        }
    }
    fn __pop_TermTABLE<
    >(
        __symbols: &mut ::std::vec::Vec<((),__Symbol<>,())>
    ) -> ((), lexer::Token, ()) {
        match __symbols.pop().unwrap() {
            (__l, __Symbol::TermTABLE(__v), __r) => (__l, __v, __r),
            _ => panic!("symbol type mismatch")
        }
    }
    fn __pop_TermUNIQUE<
    >(
        __symbols: &mut ::std::vec::Vec<((),__Symbol<>,())>
    ) -> ((), lexer::Token, ()) {
        match __symbols.pop().unwrap() {
            (__l, __Symbol::TermUNIQUE(__v), __r) => (__l, __v, __r),
            _ => panic!("symbol type mismatch")
        }
    }
    fn __pop_TermUUID<
    >(
        __symbols: &mut ::std::vec::Vec<((),__Symbol<>,())>
    ) -> ((), lexer::Token, ()) {
        match __symbols.pop().unwrap() {
            (__l, __Symbol::TermUUID(__v), __r) => (__l, __v, __r),
            _ => panic!("symbol type mismatch")
        }
    }
    fn __pop_TermVARCHAR<
    >(
        __symbols: &mut ::std::vec::Vec<((),__Symbol<>,())>
    ) -> ((), lexer::Token, ()) {
        match __symbols.pop().unwrap() {
            (__l, __Symbol::TermVARCHAR(__v), __r) => (__l, __v, __r),
            _ => panic!("symbol type mismatch")
        }
    }
    fn __pop_TermWITH<
    >(
        __symbols: &mut ::std::vec::Vec<((),__Symbol<>,())>
    ) -> ((), lexer::Token, ()) {
        match __symbols.pop().unwrap() {
            (__l, __Symbol::TermWITH(__v), __r) => (__l, __v, __r),
            _ => panic!("symbol type mismatch")
        }
    }
    fn __pop_Termerror<
    >(
        __symbols: &mut ::std::vec::Vec<((),__Symbol<>,())>
    ) -> ((), __lalrpop_util::ErrorRecovery<(), lexer::Token, ()>, ()) {
        match __symbols.pop().unwrap() {
            (__l, __Symbol::Termerror(__v), __r) => (__l, __v, __r),
            _ => panic!("symbol type mismatch")
        }
    }
    fn __pop_Nt_22_3b_22_3f<
    >(
        __symbols: &mut ::std::vec::Vec<((),__Symbol<>,())>
    ) -> ((), ::std::option::Option<lexer::Token>, ()) {
        match __symbols.pop().unwrap() {
            (__l, __Symbol::Nt_22_3b_22_3f(__v), __r) => (__l, __v, __r),
            _ => panic!("symbol type mismatch")
        }
    }
    fn __pop_Nt_28_22_2c_22_20constraint__list_29<
    >(
        __symbols: &mut ::std::vec::Vec<((),__Symbol<>,())>
    ) -> ((), (lexer::Token, ()), ()) {
        match __symbols.pop().unwrap() {
            (__l, __Symbol::Nt_28_22_2c_22_20constraint__list_29(__v), __r) => (__l, __v, __r),
            _ => panic!("symbol type mismatch")
        }
    }
    fn __pop_Nt_28_22_2c_22_20constraint__list_29_3f<
    >(
        __symbols: &mut ::std::vec::Vec<((),__Symbol<>,())>
    ) -> ((), ::std::option::Option<(lexer::Token, ())>, ()) {
        match __symbols.pop().unwrap() {
            (__l, __Symbol::Nt_28_22_2c_22_20constraint__list_29_3f(__v), __r) => (__l, __v, __r),
            _ => panic!("symbol type mismatch")
        }
    }
    fn __pop_Nt_28Ident_20_22_2e_22_29<
    >(
        __symbols: &mut ::std::vec::Vec<((),__Symbol<>,())>
    ) -> ((), (String, lexer::Token), ()) {
        match __symbols.pop().unwrap() {
            (__l, __Symbol::Nt_28Ident_20_22_2e_22_29(__v), __r) => (__l, __v, __r),
            _ => panic!("symbol type mismatch")
        }
    }
    fn __pop_Nt_28Ident_20_22_2e_22_29_3f<
    >(
        __symbols: &mut ::std::vec::Vec<((),__Symbol<>,())>
    ) -> ((), ::std::option::Option<(String, lexer::Token)>, ()) {
        match __symbols.pop().unwrap() {
            (__l, __Symbol::Nt_28Ident_20_22_2e_22_29_3f(__v), __r) => (__l, __v, __r),
            _ => panic!("symbol type mismatch")
        }
    }
    fn __pop_Nt____statement__list<
    >(
        __symbols: &mut ::std::vec::Vec<((),__Symbol<>,())>
    ) -> ((), Vec<Statement>, ()) {
        match __symbols.pop().unwrap() {
            (__l, __Symbol::Nt____statement__list(__v), __r) => (__l, __v, __r),
            _ => panic!("symbol type mismatch")
        }
    }
    fn __pop_Ntcolumn__definition<
    >(
        __symbols: &mut ::std::vec::Vec<((),__Symbol<>,())>
    ) -> ((), (), ()) {
        match __symbols.pop().unwrap() {
            (__l, __Symbol::Ntcolumn__definition(__v), __r) => (__l, __v, __r),
            _ => panic!("symbol type mismatch")
        }
    }
    fn __pop_Ntcolumn__definition__list<
    >(
        __symbols: &mut ::std::vec::Vec<((),__Symbol<>,())>
    ) -> ((), (), ()) {
        match __symbols.pop().unwrap() {
            (__l, __Symbol::Ntcolumn__definition__list(__v), __r) => (__l, __v, __r),
            _ => panic!("symbol type mismatch")
        }
    }
    fn __pop_Ntconstraint<
    >(
        __symbols: &mut ::std::vec::Vec<((),__Symbol<>,())>
    ) -> ((), (), ()) {
        match __symbols.pop().unwrap() {
            (__l, __Symbol::Ntconstraint(__v), __r) => (__l, __v, __r),
            _ => panic!("symbol type mismatch")
        }
    }
    fn __pop_Ntconstraint__list<
    >(
        __symbols: &mut ::std::vec::Vec<((),__Symbol<>,())>
    ) -> ((), (), ()) {
        match __symbols.pop().unwrap() {
            (__l, __Symbol::Ntconstraint__list(__v), __r) => (__l, __v, __r),
            _ => panic!("symbol type mismatch")
        }
    }
    fn __pop_Ntqualifier<
    >(
        __symbols: &mut ::std::vec::Vec<((),__Symbol<>,())>
    ) -> ((), (), ()) {
        match __symbols.pop().unwrap() {
            (__l, __Symbol::Ntqualifier(__v), __r) => (__l, __v, __r),
            _ => panic!("symbol type mismatch")
        }
    }
    fn __pop_Ntqualifier__list<
    >(
        __symbols: &mut ::std::vec::Vec<((),__Symbol<>,())>
    ) -> ((), (), ()) {
        match __symbols.pop().unwrap() {
            (__l, __Symbol::Ntqualifier__list(__v), __r) => (__l, __v, __r),
            _ => panic!("symbol type mismatch")
        }
    }
    fn __pop_Ntsql__type<
    >(
        __symbols: &mut ::std::vec::Vec<((),__Symbol<>,())>
    ) -> ((), (), ()) {
        match __symbols.pop().unwrap() {
            (__l, __Symbol::Ntsql__type(__v), __r) => (__l, __v, __r),
            _ => panic!("symbol type mismatch")
        }
    }
    fn __pop_Ntstatement<
    >(
        __symbols: &mut ::std::vec::Vec<((),__Symbol<>,())>
    ) -> ((), Statement, ()) {
        match __symbols.pop().unwrap() {
            (__l, __Symbol::Ntstatement(__v), __r) => (__l, __v, __r),
            _ => panic!("symbol type mismatch")
        }
    }
    fn __pop_Ntstatement__list<
    >(
        __symbols: &mut ::std::vec::Vec<((),__Symbol<>,())>
    ) -> ((), Vec<Statement>, ()) {
        match __symbols.pop().unwrap() {
            (__l, __Symbol::Ntstatement__list(__v), __r) => (__l, __v, __r),
            _ => panic!("symbol type mismatch")
        }
    }
    fn __pop_Nttable__name<
    >(
        __symbols: &mut ::std::vec::Vec<((),__Symbol<>,())>
    ) -> ((), (), ()) {
        match __symbols.pop().unwrap() {
            (__l, __Symbol::Nttable__name(__v), __r) => (__l, __v, __r),
            _ => panic!("symbol type mismatch")
        }
    }
    fn __pop_Ntwith__option<
    >(
        __symbols: &mut ::std::vec::Vec<((),__Symbol<>,())>
    ) -> ((), (), ()) {
        match __symbols.pop().unwrap() {
            (__l, __Symbol::Ntwith__option(__v), __r) => (__l, __v, __r),
            _ => panic!("symbol type mismatch")
        }
    }
    fn __pop_Ntwith__option__list<
    >(
        __symbols: &mut ::std::vec::Vec<((),__Symbol<>,())>
    ) -> ((), (), ()) {
        match __symbols.pop().unwrap() {
            (__l, __Symbol::Ntwith__option__list(__v), __r) => (__l, __v, __r),
            _ => panic!("symbol type mismatch")
        }
    }
    fn __pop_Ntwith__qualifier<
    >(
        __symbols: &mut ::std::vec::Vec<((),__Symbol<>,())>
    ) -> ((), (), ()) {
        match __symbols.pop().unwrap() {
            (__l, __Symbol::Ntwith__qualifier(__v), __r) => (__l, __v, __r),
            _ => panic!("symbol type mismatch")
        }
    }
    fn __pop_Ntwith__qualifier_3f<
    >(
        __symbols: &mut ::std::vec::Vec<((),__Symbol<>,())>
    ) -> ((), ::std::option::Option<()>, ()) {
        match __symbols.pop().unwrap() {
            (__l, __Symbol::Ntwith__qualifier_3f(__v), __r) => (__l, __v, __r),
            _ => panic!("symbol type mismatch")
        }
    }
}
pub use self::__parse__statement_list::parse_statement_list;

pub fn __action0<
    'input,
>(
    (_, __0, _): ((), Vec<Statement>, ()),
) -> Vec<Statement>
{
    (__0)
}

pub fn __action1<
    'input,
>(
    (_, v, _): ((), Vec<Statement>, ()),
    (_, stmt, _): ((), Statement, ()),
) -> Vec<Statement>
{
    {
        let mut v = v;
        v.push(stmt);
        v
    }
}

pub fn __action2<
    'input,
>(
    (_, __0, _): ((), Statement, ()),
) -> Vec<Statement>
{
    vec!(__0)
}

pub fn __action3<
    'input,
>(
    (_, __0, _): ((), lexer::Token, ()),
    (_, __1, _): ((), lexer::Token, ()),
    (_, __2, _): ((), (), ()),
    (_, __3, _): ((), lexer::Token, ()),
    (_, __4, _): ((), (), ()),
    (_, __5, _): ((), ::std::option::Option<(lexer::Token, ())>, ()),
    (_, __6, _): ((), lexer::Token, ()),
    (_, __7, _): ((), ::std::option::Option<lexer::Token>, ()),
) -> Statement
{
    Statement::Table
}

pub fn __action4<
    'input,
>(
    (_, __0, _): ((), ::std::option::Option<(String, lexer::Token)>, ()),
    (_, __1, _): ((), String, ()),
) -> ()
{
    ()
}

pub fn __action5<
    'input,
>(
    (_, __0, _): ((), (), ()),
    (_, __1, _): ((), lexer::Token, ()),
    (_, __2, _): ((), (), ()),
) -> ()
{
    ()
}

pub fn __action6<
    'input,
>(
    (_, __0, _): ((), (), ()),
) -> ()
{
    ()
}

pub fn __action7<
    'input,
>(
    (_, __0, _): ((), String, ()),
    (_, __1, _): ((), (), ()),
    (_, __2, _): ((), (), ()),
) -> ()
{
    ()
}

pub fn __action8<
    'input,
>(
    (_, __0, _): ((), (), ()),
    (_, __1, _): ((), lexer::Token, ()),
    (_, __2, _): ((), (), ()),
) -> ()
{
    ()
}

pub fn __action9<
    'input,
>(
    (_, __0, _): ((), (), ()),
) -> ()
{
    ()
}

pub fn __action10<
    'input,
>(
    (_, __0, _): ((), lexer::Token, ()),
    (_, __1, _): ((), String, ()),
    (_, __2, _): ((), lexer::Token, ()),
    (_, __3, _): ((), lexer::Token, ()),
    (_, __4, _): ((), lexer::Token, ()),
    (_, __5, _): ((), String, ()),
    (_, __6, _): ((), lexer::Token, ()),
    (_, __7, _): ((), ::std::option::Option<()>, ()),
) -> ()
{
    ()
}

pub fn __action11<
    'input,
>(
    (_, __0, _): ((), lexer::Token, ()),
    (_, __1, _): ((), String, ()),
    (_, __2, _): ((), lexer::Token, ()),
    (_, __3, _): ((), lexer::Token, ()),
    (_, __4, _): ((), lexer::Token, ()),
    (_, __5, _): ((), String, ()),
    (_, __6, _): ((), lexer::Token, ()),
    (_, __7, _): ((), lexer::Token, ()),
    (_, __8, _): ((), (), ()),
    (_, __9, _): ((), lexer::Token, ()),
    (_, __10, _): ((), String, ()),
    (_, __11, _): ((), lexer::Token, ()),
) -> ()
{
    ()
}

pub fn __action12<
    'input,
>(
    (_, __0, _): ((), lexer::Token, ()),
    (_, __1, _): ((), lexer::Token, ()),
    (_, __2, _): ((), (), ()),
    (_, __3, _): ((), lexer::Token, ()),
) -> ()
{
    ()
}

pub fn __action13<
    'input,
>(
    (_, __0, _): ((), (), ()),
    (_, __1, _): ((), lexer::Token, ()),
    (_, __2, _): ((), (), ()),
) -> ()
{
    ()
}

pub fn __action14<
    'input,
>(
    (_, __0, _): ((), (), ()),
) -> ()
{
    ()
}

pub fn __action15<
    'input,
>(
    (_, __0, _): ((), lexer::Token, ()),
    (_, __1, _): ((), lexer::Token, ()),
    (_, __2, _): ((), i32, ()),
) -> ()
{
    ()
}

pub fn __action16<
    'input,
>(
    (_, __0, _): ((), lexer::Token, ()),
) -> ()
{
    ()
}

pub fn __action17<
    'input,
>(
    (_, __0, _): ((), lexer::Token, ()),
) -> ()
{
    ()
}

pub fn __action18<
    'input,
>(
    (_, __0, _): ((), lexer::Token, ()),
) -> ()
{
    ()
}

pub fn __action19<
    'input,
>(
    (_, __0, _): ((), lexer::Token, ()),
) -> ()
{
    ()
}

pub fn __action20<
    'input,
>(
    (_, __0, _): ((), lexer::Token, ()),
) -> ()
{
    ()
}

pub fn __action21<
    'input,
>(
    (_, __0, _): ((), lexer::Token, ()),
    (_, __1, _): ((), lexer::Token, ()),
    (_, __2, _): ((), i32, ()),
    (_, __3, _): ((), lexer::Token, ()),
) -> ()
{
    ()
}

pub fn __action22<
    'input,
>(
    (_, __0, _): ((), (), ()),
    (_, __1, _): ((), (), ()),
) -> ()
{
    ()
}

pub fn __action23<
    'input,
>(
    (_, __0, _): ((), (), ()),
) -> ()
{
    ()
}

pub fn __action24<
    'input,
>(
    (_, __0, _): ((), lexer::Token, ()),
) -> ()
{
    ()
}

pub fn __action25<
    'input,
>(
    (_, __0, _): ((), lexer::Token, ()),
    (_, __1, _): ((), lexer::Token, ()),
) -> ()
{
    ()
}

pub fn __action26<
    'input,
>(
    (_, __0, _): ((), lexer::Token, ()),
) -> ()
{
    ()
}

pub fn __action27<
    'input,
>(
    (_, __0, _): ((), lexer::Token, ()),
    (_, __1, _): ((), lexer::Token, ()),
) -> ()
{
    ()
}

pub fn __action28<
    'input,
>(
    (_, __0, _): ((), (), ()),
) -> ::std::option::Option<()>
{
    Some(__0)
}

pub fn __action29<
    'input,
>(
    __lookbehind: &(),
    __lookahead: &(),
) -> ::std::option::Option<()>
{
    None
}

pub fn __action30<
    'input,
>(
    (_, __0, _): ((), (String, lexer::Token), ()),
) -> ::std::option::Option<(String, lexer::Token)>
{
    Some(__0)
}

pub fn __action31<
    'input,
>(
    __lookbehind: &(),
    __lookahead: &(),
) -> ::std::option::Option<(String, lexer::Token)>
{
    None
}

pub fn __action32<
    'input,
>(
    (_, __0, _): ((), String, ()),
    (_, __1, _): ((), lexer::Token, ()),
) -> (String, lexer::Token)
{
    (__0, __1)
}

pub fn __action33<
    'input,
>(
    (_, __0, _): ((), lexer::Token, ()),
) -> ::std::option::Option<lexer::Token>
{
    Some(__0)
}

pub fn __action34<
    'input,
>(
    __lookbehind: &(),
    __lookahead: &(),
) -> ::std::option::Option<lexer::Token>
{
    None
}

pub fn __action35<
    'input,
>(
    (_, __0, _): ((), (lexer::Token, ()), ()),
) -> ::std::option::Option<(lexer::Token, ())>
{
    Some(__0)
}

pub fn __action36<
    'input,
>(
    __lookbehind: &(),
    __lookahead: &(),
) -> ::std::option::Option<(lexer::Token, ())>
{
    None
}

pub fn __action37<
    'input,
>(
    (_, __0, _): ((), lexer::Token, ()),
    (_, __1, _): ((), (), ()),
) -> (lexer::Token, ())
{
    (__0, __1)
}

pub fn __action38<
    'input,
>(
    __0: ((), lexer::Token, ()),
    __1: ((), lexer::Token, ()),
    __2: ((), (), ()),
    __3: ((), lexer::Token, ()),
    __4: ((), (), ()),
    __5: ((), ::std::option::Option<(lexer::Token, ())>, ()),
    __6: ((), lexer::Token, ()),
    __7: ((), lexer::Token, ()),
) -> Statement
{
    let __start0 = __7.0.clone();
    let __end0 = __7.2.clone();
    let __temp0 = __action33(
        __7,
    );
    let __temp0 = (__start0, __temp0, __end0);
    __action3(
        __0,
        __1,
        __2,
        __3,
        __4,
        __5,
        __6,
        __temp0,
    )
}

pub fn __action39<
    'input,
>(
    __0: ((), lexer::Token, ()),
    __1: ((), lexer::Token, ()),
    __2: ((), (), ()),
    __3: ((), lexer::Token, ()),
    __4: ((), (), ()),
    __5: ((), ::std::option::Option<(lexer::Token, ())>, ()),
    __6: ((), lexer::Token, ()),
) -> Statement
{
    let __start0 = __6.2.clone();
    let __end0 = __6.2.clone();
    let __temp0 = __action34(
        &__start0,
        &__end0,
    );
    let __temp0 = (__start0, __temp0, __end0);
    __action3(
        __0,
        __1,
        __2,
        __3,
        __4,
        __5,
        __6,
        __temp0,
    )
}

pub fn __action40<
    'input,
>(
    __0: ((), lexer::Token, ()),
    __1: ((), (), ()),
) -> ::std::option::Option<(lexer::Token, ())>
{
    let __start0 = __0.0.clone();
    let __end0 = __1.2.clone();
    let __temp0 = __action37(
        __0,
        __1,
    );
    let __temp0 = (__start0, __temp0, __end0);
    __action35(
        __temp0,
    )
}

pub fn __action41<
    'input,
>(
    __0: ((), lexer::Token, ()),
    __1: ((), lexer::Token, ()),
    __2: ((), (), ()),
    __3: ((), lexer::Token, ()),
    __4: ((), (), ()),
    __5: ((), lexer::Token, ()),
    __6: ((), (), ()),
    __7: ((), lexer::Token, ()),
    __8: ((), lexer::Token, ()),
) -> Statement
{
    let __start0 = __5.0.clone();
    let __end0 = __6.2.clone();
    let __temp0 = __action40(
        __5,
        __6,
    );
    let __temp0 = (__start0, __temp0, __end0);
    __action38(
        __0,
        __1,
        __2,
        __3,
        __4,
        __temp0,
        __7,
        __8,
    )
}

pub fn __action42<
    'input,
>(
    __0: ((), lexer::Token, ()),
    __1: ((), lexer::Token, ()),
    __2: ((), (), ()),
    __3: ((), lexer::Token, ()),
    __4: ((), (), ()),
    __5: ((), lexer::Token, ()),
    __6: ((), lexer::Token, ()),
) -> Statement
{
    let __start0 = __4.2.clone();
    let __end0 = __5.0.clone();
    let __temp0 = __action36(
        &__start0,
        &__end0,
    );
    let __temp0 = (__start0, __temp0, __end0);
    __action38(
        __0,
        __1,
        __2,
        __3,
        __4,
        __temp0,
        __5,
        __6,
    )
}

pub fn __action43<
    'input,
>(
    __0: ((), lexer::Token, ()),
    __1: ((), lexer::Token, ()),
    __2: ((), (), ()),
    __3: ((), lexer::Token, ()),
    __4: ((), (), ()),
    __5: ((), lexer::Token, ()),
    __6: ((), (), ()),
    __7: ((), lexer::Token, ()),
) -> Statement
{
    let __start0 = __5.0.clone();
    let __end0 = __6.2.clone();
    let __temp0 = __action40(
        __5,
        __6,
    );
    let __temp0 = (__start0, __temp0, __end0);
    __action39(
        __0,
        __1,
        __2,
        __3,
        __4,
        __temp0,
        __7,
    )
}

pub fn __action44<
    'input,
>(
    __0: ((), lexer::Token, ()),
    __1: ((), lexer::Token, ()),
    __2: ((), (), ()),
    __3: ((), lexer::Token, ()),
    __4: ((), (), ()),
    __5: ((), lexer::Token, ()),
) -> Statement
{
    let __start0 = __4.2.clone();
    let __end0 = __5.0.clone();
    let __temp0 = __action36(
        &__start0,
        &__end0,
    );
    let __temp0 = (__start0, __temp0, __end0);
    __action39(
        __0,
        __1,
        __2,
        __3,
        __4,
        __temp0,
        __5,
    )
}

pub fn __action45<
    'input,
>(
    __0: ((), String, ()),
    __1: ((), lexer::Token, ()),
) -> ::std::option::Option<(String, lexer::Token)>
{
    let __start0 = __0.0.clone();
    let __end0 = __1.2.clone();
    let __temp0 = __action32(
        __0,
        __1,
    );
    let __temp0 = (__start0, __temp0, __end0);
    __action30(
        __temp0,
    )
}

pub fn __action46<
    'input,
>(
    __0: ((), String, ()),
    __1: ((), lexer::Token, ()),
    __2: ((), String, ()),
) -> ()
{
    let __start0 = __0.0.clone();
    let __end0 = __1.2.clone();
    let __temp0 = __action45(
        __0,
        __1,
    );
    let __temp0 = (__start0, __temp0, __end0);
    __action4(
        __temp0,
        __2,
    )
}

pub fn __action47<
    'input,
>(
    __0: ((), String, ()),
) -> ()
{
    let __start0 = __0.0.clone();
    let __end0 = __0.0.clone();
    let __temp0 = __action31(
        &__start0,
        &__end0,
    );
    let __temp0 = (__start0, __temp0, __end0);
    __action4(
        __temp0,
        __0,
    )
}

pub fn __action48<
    'input,
>(
    __0: ((), lexer::Token, ()),
    __1: ((), String, ()),
    __2: ((), lexer::Token, ()),
    __3: ((), lexer::Token, ()),
    __4: ((), lexer::Token, ()),
    __5: ((), String, ()),
    __6: ((), lexer::Token, ()),
    __7: ((), (), ()),
) -> ()
{
    let __start0 = __7.0.clone();
    let __end0 = __7.2.clone();
    let __temp0 = __action28(
        __7,
    );
    let __temp0 = (__start0, __temp0, __end0);
    __action10(
        __0,
        __1,
        __2,
        __3,
        __4,
        __5,
        __6,
        __temp0,
    )
}

pub fn __action49<
    'input,
>(
    __0: ((), lexer::Token, ()),
    __1: ((), String, ()),
    __2: ((), lexer::Token, ()),
    __3: ((), lexer::Token, ()),
    __4: ((), lexer::Token, ()),
    __5: ((), String, ()),
    __6: ((), lexer::Token, ()),
) -> ()
{
    let __start0 = __6.2.clone();
    let __end0 = __6.2.clone();
    let __temp0 = __action29(
        &__start0,
        &__end0,
    );
    let __temp0 = (__start0, __temp0, __end0);
    __action10(
        __0,
        __1,
        __2,
        __3,
        __4,
        __5,
        __6,
        __temp0,
    )
}

pub trait __ToTriple<'input, > {
    type Error;
    fn to_triple(value: Self) -> Result<((),lexer::Token,()),Self::Error>;
}

impl<'input, > __ToTriple<'input, > for lexer::Token {
    type Error = ();
    fn to_triple(value: Self) -> Result<((),lexer::Token,()),()> {
        Ok(((), value, ()))
    }
}
impl<'input, > __ToTriple<'input, > for Result<(lexer::Token),()> {
    type Error = ();
    fn to_triple(value: Self) -> Result<((),lexer::Token,()),()> {
        value.map(|v| ((), v, ()))
    }
}
