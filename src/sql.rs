use ast::{Statement};
extern crate lalrpop_util as __lalrpop_util;

mod __parse__statement_list {
    #![allow(non_snake_case, non_camel_case_types, unused_mut, unused_variables, unused_imports)]

    use ast::{Statement};
    extern crate lalrpop_util as __lalrpop_util;
    #[allow(dead_code)]
    pub enum __Symbol<'input> {
        Term_22CREATE_22(&'input str),
        Term_22TABLE_22(&'input str),
        Termr_23_22_5ba_2dzA_2dZ_5d_5ba_2dzA_2dZ0_2d9___5d_2a_22_23(&'input str),
        Termerror(__lalrpop_util::ErrorRecovery<usize, (usize, &'input str), ()>),
        Nt____statement__list(Vec<Statement>),
        Ntidentifier(()),
        Ntstatement(Statement),
        Ntstatement__list(Vec<Statement>),
    }
    const __ACTION: &'static [i32] = &[
        // State 0
        4, 0, 0, 0,
        // State 1
        -5, 0, 0, 0,
        // State 2
        4, 0, 0, 0,
        // State 3
        0, 6, 0, 0,
        // State 4
        -4, 0, 0, 0,
        // State 5
        0, 0, 8, 0,
        // State 6
        -3, 0, 0, 0,
        // State 7
        -2, 0, 0, 0,
    ];
    const __EOF_ACTION: &'static [i32] = &[
        0,
        -5,
        -1,
        0,
        -4,
        0,
        -3,
        -2,
    ];
    const __GOTO: &'static [i32] = &[
        // State 0
        0, 0, 2, 3,
        // State 1
        0, 0, 0, 0,
        // State 2
        0, 0, 5, 0,
        // State 3
        0, 0, 0, 0,
        // State 4
        0, 0, 0, 0,
        // State 5
        0, 7, 0, 0,
        // State 6
        0, 0, 0, 0,
        // State 7
        0, 0, 0, 0,
    ];
    fn __expected_tokens(__state: usize) -> Vec<::std::string::String> {
        const __TERMINAL: &'static [&'static str] = &[
            r###""CREATE""###,
            r###""TABLE""###,
            r###"r#"[a-zA-Z][a-zA-Z0-9_]*"#"###,
        ];
        __ACTION[(__state * 4)..].iter().zip(__TERMINAL).filter_map(|(&state, terminal)| {
            if state == 0 {
                None
            } else {
                Some(terminal.to_string())
            }
        }).collect()
    }
    pub fn parse_statement_list<
        'input,
    >(
        input: &'input str,
    ) -> Result<Vec<Statement>, __lalrpop_util::ParseError<usize, (usize, &'input str), ()>>
    {
        let mut __tokens = super::__intern_token::__Matcher::new(input);
        let mut __states = vec![0_i32];
        let mut __symbols = vec![];
        let mut __integer;
        let mut __lookahead;
        let mut __last_location = Default::default();
        '__shift: loop {
            __lookahead = match __tokens.next() {
                Some(Ok(v)) => v,
                None => break '__shift,
                Some(Err(e)) => return Err(e),
            };
            __last_location = __lookahead.2.clone();
            __integer = match __lookahead.1 {
                (0, _) if true => 0,
                (1, _) if true => 1,
                (2, _) if true => 2,
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
                let __action = __ACTION[__state * 4 + __integer];
                if __action > 0 {
                    let __symbol = match __integer {
                        0 => match __lookahead.1 {
                            (0, __tok0) => __Symbol::Term_22CREATE_22(__tok0),
                            _ => unreachable!(),
                        },
                        1 => match __lookahead.1 {
                            (1, __tok0) => __Symbol::Term_22TABLE_22(__tok0),
                            _ => unreachable!(),
                        },
                        2 => match __lookahead.1 {
                            (2, __tok0) => __Symbol::Termr_23_22_5ba_2dzA_2dZ_5d_5ba_2dzA_2dZ0_2d9___5d_2a_22_23(__tok0),
                            _ => unreachable!(),
                        },
                        _ => unreachable!(),
                    };
                    __states.push(__action - 1);
                    __symbols.push((__lookahead.0, __symbol, __lookahead.2));
                    continue '__shift;
                } else if __action < 0 {
                    if let Some(r) = __reduce(input, __action, Some(&__lookahead.0), &mut __states, &mut __symbols, ::std::marker::PhantomData::<()>) {
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
                if let Some(r) = __reduce(input, __action, None, &mut __states, &mut __symbols, ::std::marker::PhantomData::<()>) {
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
        input: &'input str,
        __action: i32,
        __lookahead_start: Option<&usize>,
        __states: &mut ::std::vec::Vec<i32>,
        __symbols: &mut ::std::vec::Vec<(usize,__Symbol<'input>,usize)>,
        _: ::std::marker::PhantomData<()>,
    ) -> Option<Result<Vec<Statement>,__lalrpop_util::ParseError<usize, (usize, &'input str), ()>>>
    {
        let __nonterminal = match -__action {
            1 => {
                // __statement_list = statement_list => ActionFn(0);
                let __sym0 = __pop_Ntstatement__list(__symbols);
                let __start = __sym0.0.clone();
                let __end = __sym0.2.clone();
                let __nt = super::__action0::<>(input, __sym0);
                return Some(Ok(__nt));
            }
            2 => {
                // identifier = r#"[a-zA-Z][a-zA-Z0-9_]*"# => ActionFn(4);
                let __sym0 = __pop_Termr_23_22_5ba_2dzA_2dZ_5d_5ba_2dzA_2dZ0_2d9___5d_2a_22_23(__symbols);
                let __start = __sym0.0.clone();
                let __end = __sym0.2.clone();
                let __nt = super::__action4::<>(input, __sym0);
                let __states_len = __states.len();
                __states.truncate(__states_len - 1);
                __symbols.push((__start, __Symbol::Ntidentifier(__nt), __end));
                1
            }
            3 => {
                // statement = "CREATE", "TABLE", identifier => ActionFn(3);
                let __sym2 = __pop_Ntidentifier(__symbols);
                let __sym1 = __pop_Term_22TABLE_22(__symbols);
                let __sym0 = __pop_Term_22CREATE_22(__symbols);
                let __start = __sym0.0.clone();
                let __end = __sym2.2.clone();
                let __nt = super::__action3::<>(input, __sym0, __sym1, __sym2);
                let __states_len = __states.len();
                __states.truncate(__states_len - 3);
                __symbols.push((__start, __Symbol::Ntstatement(__nt), __end));
                2
            }
            4 => {
                // statement_list = statement_list, statement => ActionFn(1);
                let __sym1 = __pop_Ntstatement(__symbols);
                let __sym0 = __pop_Ntstatement__list(__symbols);
                let __start = __sym0.0.clone();
                let __end = __sym1.2.clone();
                let __nt = super::__action1::<>(input, __sym0, __sym1);
                let __states_len = __states.len();
                __states.truncate(__states_len - 2);
                __symbols.push((__start, __Symbol::Ntstatement__list(__nt), __end));
                3
            }
            5 => {
                // statement_list = statement => ActionFn(2);
                let __sym0 = __pop_Ntstatement(__symbols);
                let __start = __sym0.0.clone();
                let __end = __sym0.2.clone();
                let __nt = super::__action2::<>(input, __sym0);
                let __states_len = __states.len();
                __states.truncate(__states_len - 1);
                __symbols.push((__start, __Symbol::Ntstatement__list(__nt), __end));
                3
            }
            _ => panic!("invalid action code {}", __action)
        };
        let __state = *__states.last().unwrap() as usize;
        let __next_state = __GOTO[__state * 4 + __nonterminal] - 1;
        __states.push(__next_state);
        None
    }
    fn __pop_Term_22CREATE_22<
      'input,
    >(
        __symbols: &mut ::std::vec::Vec<(usize,__Symbol<'input>,usize)>
    ) -> (usize, &'input str, usize) {
        match __symbols.pop().unwrap() {
            (__l, __Symbol::Term_22CREATE_22(__v), __r) => (__l, __v, __r),
            _ => panic!("symbol type mismatch")
        }
    }
    fn __pop_Term_22TABLE_22<
      'input,
    >(
        __symbols: &mut ::std::vec::Vec<(usize,__Symbol<'input>,usize)>
    ) -> (usize, &'input str, usize) {
        match __symbols.pop().unwrap() {
            (__l, __Symbol::Term_22TABLE_22(__v), __r) => (__l, __v, __r),
            _ => panic!("symbol type mismatch")
        }
    }
    fn __pop_Termr_23_22_5ba_2dzA_2dZ_5d_5ba_2dzA_2dZ0_2d9___5d_2a_22_23<
      'input,
    >(
        __symbols: &mut ::std::vec::Vec<(usize,__Symbol<'input>,usize)>
    ) -> (usize, &'input str, usize) {
        match __symbols.pop().unwrap() {
            (__l, __Symbol::Termr_23_22_5ba_2dzA_2dZ_5d_5ba_2dzA_2dZ0_2d9___5d_2a_22_23(__v), __r) => (__l, __v, __r),
            _ => panic!("symbol type mismatch")
        }
    }
    fn __pop_Termerror<
      'input,
    >(
        __symbols: &mut ::std::vec::Vec<(usize,__Symbol<'input>,usize)>
    ) -> (usize, __lalrpop_util::ErrorRecovery<usize, (usize, &'input str), ()>, usize) {
        match __symbols.pop().unwrap() {
            (__l, __Symbol::Termerror(__v), __r) => (__l, __v, __r),
            _ => panic!("symbol type mismatch")
        }
    }
    fn __pop_Nt____statement__list<
      'input,
    >(
        __symbols: &mut ::std::vec::Vec<(usize,__Symbol<'input>,usize)>
    ) -> (usize, Vec<Statement>, usize) {
        match __symbols.pop().unwrap() {
            (__l, __Symbol::Nt____statement__list(__v), __r) => (__l, __v, __r),
            _ => panic!("symbol type mismatch")
        }
    }
    fn __pop_Ntidentifier<
      'input,
    >(
        __symbols: &mut ::std::vec::Vec<(usize,__Symbol<'input>,usize)>
    ) -> (usize, (), usize) {
        match __symbols.pop().unwrap() {
            (__l, __Symbol::Ntidentifier(__v), __r) => (__l, __v, __r),
            _ => panic!("symbol type mismatch")
        }
    }
    fn __pop_Ntstatement<
      'input,
    >(
        __symbols: &mut ::std::vec::Vec<(usize,__Symbol<'input>,usize)>
    ) -> (usize, Statement, usize) {
        match __symbols.pop().unwrap() {
            (__l, __Symbol::Ntstatement(__v), __r) => (__l, __v, __r),
            _ => panic!("symbol type mismatch")
        }
    }
    fn __pop_Ntstatement__list<
      'input,
    >(
        __symbols: &mut ::std::vec::Vec<(usize,__Symbol<'input>,usize)>
    ) -> (usize, Vec<Statement>, usize) {
        match __symbols.pop().unwrap() {
            (__l, __Symbol::Ntstatement__list(__v), __r) => (__l, __v, __r),
            _ => panic!("symbol type mismatch")
        }
    }
}
pub use self::__parse__statement_list::parse_statement_list;
mod __intern_token {
    extern crate lalrpop_util as __lalrpop_util;
    pub struct __Matcher<'input> {
        text: &'input str,
        consumed: usize,
    }

    fn __tokenize(text: &str) -> Option<(usize, usize)> {
        let mut __chars = text.char_indices();
        let mut __current_match: Option<(usize, usize)> = None;
        let mut __current_state: usize = 0;
        loop {
            match __current_state {
                0 => {
                    let (__index, __ch) = match __chars.next() { Some(p) => p, None => return __current_match };
                    match __ch as u32 {
                        65 ... 66 => {
                            __current_match = Some((2, __index + __ch.len_utf8()));
                            __current_state = 1;
                            continue;
                        }
                        67 => /* 'C' */ {
                            __current_match = Some((2, __index + 1));
                            __current_state = 2;
                            continue;
                        }
                        68 ... 83 => {
                            __current_match = Some((2, __index + __ch.len_utf8()));
                            __current_state = 1;
                            continue;
                        }
                        84 => /* 'T' */ {
                            __current_match = Some((2, __index + 1));
                            __current_state = 3;
                            continue;
                        }
                        85 ... 90 => {
                            __current_match = Some((2, __index + __ch.len_utf8()));
                            __current_state = 1;
                            continue;
                        }
                        97 ... 122 => {
                            __current_match = Some((2, __index + __ch.len_utf8()));
                            __current_state = 1;
                            continue;
                        }
                        _ => {
                            return __current_match;
                        }
                    }
                }
                1 => {
                    let (__index, __ch) = match __chars.next() { Some(p) => p, None => return __current_match };
                    match __ch as u32 {
                        48 ... 57 => {
                            __current_match = Some((2, __index + __ch.len_utf8()));
                            __current_state = 5;
                            continue;
                        }
                        65 ... 90 => {
                            __current_match = Some((2, __index + __ch.len_utf8()));
                            __current_state = 5;
                            continue;
                        }
                        95 => /* '_' */ {
                            __current_match = Some((2, __index + 1));
                            __current_state = 5;
                            continue;
                        }
                        97 ... 122 => {
                            __current_match = Some((2, __index + __ch.len_utf8()));
                            __current_state = 5;
                            continue;
                        }
                        _ => {
                            return __current_match;
                        }
                    }
                }
                2 => {
                    let (__index, __ch) = match __chars.next() { Some(p) => p, None => return __current_match };
                    match __ch as u32 {
                        48 ... 57 => {
                            __current_match = Some((2, __index + __ch.len_utf8()));
                            __current_state = 5;
                            continue;
                        }
                        65 ... 81 => {
                            __current_match = Some((2, __index + __ch.len_utf8()));
                            __current_state = 5;
                            continue;
                        }
                        82 => /* 'R' */ {
                            __current_match = Some((2, __index + 1));
                            __current_state = 6;
                            continue;
                        }
                        83 ... 90 => {
                            __current_match = Some((2, __index + __ch.len_utf8()));
                            __current_state = 5;
                            continue;
                        }
                        95 => /* '_' */ {
                            __current_match = Some((2, __index + 1));
                            __current_state = 5;
                            continue;
                        }
                        97 ... 122 => {
                            __current_match = Some((2, __index + __ch.len_utf8()));
                            __current_state = 5;
                            continue;
                        }
                        _ => {
                            return __current_match;
                        }
                    }
                }
                3 => {
                    let (__index, __ch) = match __chars.next() { Some(p) => p, None => return __current_match };
                    match __ch as u32 {
                        48 ... 57 => {
                            __current_match = Some((2, __index + __ch.len_utf8()));
                            __current_state = 5;
                            continue;
                        }
                        65 => /* 'A' */ {
                            __current_match = Some((2, __index + 1));
                            __current_state = 7;
                            continue;
                        }
                        66 ... 90 => {
                            __current_match = Some((2, __index + __ch.len_utf8()));
                            __current_state = 5;
                            continue;
                        }
                        95 => /* '_' */ {
                            __current_match = Some((2, __index + 1));
                            __current_state = 5;
                            continue;
                        }
                        97 ... 122 => {
                            __current_match = Some((2, __index + __ch.len_utf8()));
                            __current_state = 5;
                            continue;
                        }
                        _ => {
                            return __current_match;
                        }
                    }
                }
                4 => {
                    let (__index, __ch) = match __chars.next() { Some(p) => p, None => return __current_match };
                    match __ch as u32 {
                        _ => {
                            return __current_match;
                        }
                    }
                }
                5 => {
                    let (__index, __ch) = match __chars.next() { Some(p) => p, None => return __current_match };
                    match __ch as u32 {
                        48 ... 57 => {
                            __current_match = Some((2, __index + __ch.len_utf8()));
                            __current_state = 5;
                            continue;
                        }
                        65 ... 90 => {
                            __current_match = Some((2, __index + __ch.len_utf8()));
                            __current_state = 5;
                            continue;
                        }
                        95 => /* '_' */ {
                            __current_match = Some((2, __index + 1));
                            __current_state = 5;
                            continue;
                        }
                        97 ... 122 => {
                            __current_match = Some((2, __index + __ch.len_utf8()));
                            __current_state = 5;
                            continue;
                        }
                        _ => {
                            return __current_match;
                        }
                    }
                }
                6 => {
                    let (__index, __ch) = match __chars.next() { Some(p) => p, None => return __current_match };
                    match __ch as u32 {
                        48 ... 57 => {
                            __current_match = Some((2, __index + __ch.len_utf8()));
                            __current_state = 5;
                            continue;
                        }
                        65 ... 68 => {
                            __current_match = Some((2, __index + __ch.len_utf8()));
                            __current_state = 5;
                            continue;
                        }
                        69 => /* 'E' */ {
                            __current_match = Some((2, __index + 1));
                            __current_state = 8;
                            continue;
                        }
                        70 ... 90 => {
                            __current_match = Some((2, __index + __ch.len_utf8()));
                            __current_state = 5;
                            continue;
                        }
                        95 => /* '_' */ {
                            __current_match = Some((2, __index + 1));
                            __current_state = 5;
                            continue;
                        }
                        97 ... 122 => {
                            __current_match = Some((2, __index + __ch.len_utf8()));
                            __current_state = 5;
                            continue;
                        }
                        _ => {
                            return __current_match;
                        }
                    }
                }
                7 => {
                    let (__index, __ch) = match __chars.next() { Some(p) => p, None => return __current_match };
                    match __ch as u32 {
                        48 ... 57 => {
                            __current_match = Some((2, __index + __ch.len_utf8()));
                            __current_state = 5;
                            continue;
                        }
                        65 => /* 'A' */ {
                            __current_match = Some((2, __index + 1));
                            __current_state = 5;
                            continue;
                        }
                        66 => /* 'B' */ {
                            __current_match = Some((2, __index + 1));
                            __current_state = 9;
                            continue;
                        }
                        67 ... 90 => {
                            __current_match = Some((2, __index + __ch.len_utf8()));
                            __current_state = 5;
                            continue;
                        }
                        95 => /* '_' */ {
                            __current_match = Some((2, __index + 1));
                            __current_state = 5;
                            continue;
                        }
                        97 ... 122 => {
                            __current_match = Some((2, __index + __ch.len_utf8()));
                            __current_state = 5;
                            continue;
                        }
                        _ => {
                            return __current_match;
                        }
                    }
                }
                8 => {
                    let (__index, __ch) = match __chars.next() { Some(p) => p, None => return __current_match };
                    match __ch as u32 {
                        48 ... 57 => {
                            __current_match = Some((2, __index + __ch.len_utf8()));
                            __current_state = 5;
                            continue;
                        }
                        65 => /* 'A' */ {
                            __current_match = Some((2, __index + 1));
                            __current_state = 10;
                            continue;
                        }
                        66 ... 90 => {
                            __current_match = Some((2, __index + __ch.len_utf8()));
                            __current_state = 5;
                            continue;
                        }
                        95 => /* '_' */ {
                            __current_match = Some((2, __index + 1));
                            __current_state = 5;
                            continue;
                        }
                        97 ... 122 => {
                            __current_match = Some((2, __index + __ch.len_utf8()));
                            __current_state = 5;
                            continue;
                        }
                        _ => {
                            return __current_match;
                        }
                    }
                }
                9 => {
                    let (__index, __ch) = match __chars.next() { Some(p) => p, None => return __current_match };
                    match __ch as u32 {
                        48 ... 57 => {
                            __current_match = Some((2, __index + __ch.len_utf8()));
                            __current_state = 5;
                            continue;
                        }
                        65 ... 75 => {
                            __current_match = Some((2, __index + __ch.len_utf8()));
                            __current_state = 5;
                            continue;
                        }
                        76 => /* 'L' */ {
                            __current_match = Some((2, __index + 1));
                            __current_state = 11;
                            continue;
                        }
                        77 ... 90 => {
                            __current_match = Some((2, __index + __ch.len_utf8()));
                            __current_state = 5;
                            continue;
                        }
                        95 => /* '_' */ {
                            __current_match = Some((2, __index + 1));
                            __current_state = 5;
                            continue;
                        }
                        97 ... 122 => {
                            __current_match = Some((2, __index + __ch.len_utf8()));
                            __current_state = 5;
                            continue;
                        }
                        _ => {
                            return __current_match;
                        }
                    }
                }
                10 => {
                    let (__index, __ch) = match __chars.next() { Some(p) => p, None => return __current_match };
                    match __ch as u32 {
                        48 ... 57 => {
                            __current_match = Some((2, __index + __ch.len_utf8()));
                            __current_state = 5;
                            continue;
                        }
                        65 ... 83 => {
                            __current_match = Some((2, __index + __ch.len_utf8()));
                            __current_state = 5;
                            continue;
                        }
                        84 => /* 'T' */ {
                            __current_match = Some((2, __index + 1));
                            __current_state = 12;
                            continue;
                        }
                        85 ... 90 => {
                            __current_match = Some((2, __index + __ch.len_utf8()));
                            __current_state = 5;
                            continue;
                        }
                        95 => /* '_' */ {
                            __current_match = Some((2, __index + 1));
                            __current_state = 5;
                            continue;
                        }
                        97 ... 122 => {
                            __current_match = Some((2, __index + __ch.len_utf8()));
                            __current_state = 5;
                            continue;
                        }
                        _ => {
                            return __current_match;
                        }
                    }
                }
                11 => {
                    let (__index, __ch) = match __chars.next() { Some(p) => p, None => return __current_match };
                    match __ch as u32 {
                        48 ... 57 => {
                            __current_match = Some((2, __index + __ch.len_utf8()));
                            __current_state = 5;
                            continue;
                        }
                        65 ... 68 => {
                            __current_match = Some((2, __index + __ch.len_utf8()));
                            __current_state = 5;
                            continue;
                        }
                        69 => /* 'E' */ {
                            __current_match = Some((1, __index + 1));
                            __current_state = 13;
                            continue;
                        }
                        70 ... 90 => {
                            __current_match = Some((2, __index + __ch.len_utf8()));
                            __current_state = 5;
                            continue;
                        }
                        95 => /* '_' */ {
                            __current_match = Some((2, __index + 1));
                            __current_state = 5;
                            continue;
                        }
                        97 ... 122 => {
                            __current_match = Some((2, __index + __ch.len_utf8()));
                            __current_state = 5;
                            continue;
                        }
                        _ => {
                            return __current_match;
                        }
                    }
                }
                12 => {
                    let (__index, __ch) = match __chars.next() { Some(p) => p, None => return __current_match };
                    match __ch as u32 {
                        48 ... 57 => {
                            __current_match = Some((2, __index + __ch.len_utf8()));
                            __current_state = 5;
                            continue;
                        }
                        65 ... 68 => {
                            __current_match = Some((2, __index + __ch.len_utf8()));
                            __current_state = 5;
                            continue;
                        }
                        69 => /* 'E' */ {
                            __current_match = Some((0, __index + 1));
                            __current_state = 14;
                            continue;
                        }
                        70 ... 90 => {
                            __current_match = Some((2, __index + __ch.len_utf8()));
                            __current_state = 5;
                            continue;
                        }
                        95 => /* '_' */ {
                            __current_match = Some((2, __index + 1));
                            __current_state = 5;
                            continue;
                        }
                        97 ... 122 => {
                            __current_match = Some((2, __index + __ch.len_utf8()));
                            __current_state = 5;
                            continue;
                        }
                        _ => {
                            return __current_match;
                        }
                    }
                }
                13 => {
                    let (__index, __ch) = match __chars.next() { Some(p) => p, None => return __current_match };
                    match __ch as u32 {
                        48 ... 57 => {
                            __current_match = Some((2, __index + __ch.len_utf8()));
                            __current_state = 5;
                            continue;
                        }
                        65 ... 90 => {
                            __current_match = Some((2, __index + __ch.len_utf8()));
                            __current_state = 5;
                            continue;
                        }
                        95 => /* '_' */ {
                            __current_match = Some((2, __index + 1));
                            __current_state = 5;
                            continue;
                        }
                        97 ... 122 => {
                            __current_match = Some((2, __index + __ch.len_utf8()));
                            __current_state = 5;
                            continue;
                        }
                        _ => {
                            return __current_match;
                        }
                    }
                }
                14 => {
                    let (__index, __ch) = match __chars.next() { Some(p) => p, None => return __current_match };
                    match __ch as u32 {
                        48 ... 57 => {
                            __current_match = Some((2, __index + __ch.len_utf8()));
                            __current_state = 5;
                            continue;
                        }
                        65 ... 90 => {
                            __current_match = Some((2, __index + __ch.len_utf8()));
                            __current_state = 5;
                            continue;
                        }
                        95 => /* '_' */ {
                            __current_match = Some((2, __index + 1));
                            __current_state = 5;
                            continue;
                        }
                        97 ... 122 => {
                            __current_match = Some((2, __index + __ch.len_utf8()));
                            __current_state = 5;
                            continue;
                        }
                        _ => {
                            return __current_match;
                        }
                    }
                }
                _ => { panic!("invalid state {}", __current_state); }
            }
        }
    }

    impl<'input> __Matcher<'input> {
        pub fn new(s: &'input str) -> __Matcher<'input> {
            __Matcher { text: s, consumed: 0 }
        }
    }

    impl<'input> Iterator for __Matcher<'input> {
        type Item = Result<(usize, (usize, &'input str), usize), __lalrpop_util::ParseError<usize,(usize, &'input str),()>>;

        fn next(&mut self) -> Option<Self::Item> {
            let __text = self.text.trim_left();
            let __whitespace = self.text.len() - __text.len();
            let __start_offset = self.consumed + __whitespace;
            if __text.is_empty() {
                self.text = __text;
                self.consumed = __start_offset;
                None
            } else {
                match __tokenize(__text) {
                    Some((__index, __length)) => {
                        let __result = &__text[..__length];
                        let __remaining = &__text[__length..];
                        let __end_offset = __start_offset + __length;
                        self.text = __remaining;
                        self.consumed = __end_offset;
                        Some(Ok((__start_offset, (__index, __result), __end_offset)))
                    }
                    None => {
                        Some(Err(__lalrpop_util::ParseError::InvalidToken { location: __start_offset }))
                    }
                }
            }
        }
    }
}

#[allow(unused_variables)]
pub fn __action0<
    'input,
>(
    input: &'input str,
    (_, __0, _): (usize, Vec<Statement>, usize),
) -> Vec<Statement>
{
    (__0)
}

#[allow(unused_variables)]
pub fn __action1<
    'input,
>(
    input: &'input str,
    (_, v, _): (usize, Vec<Statement>, usize),
    (_, stmt, _): (usize, Statement, usize),
) -> Vec<Statement>
{
    {
        let mut v = v;
        v.push(stmt);
        v
    }
}

#[allow(unused_variables)]
pub fn __action2<
    'input,
>(
    input: &'input str,
    (_, __0, _): (usize, Statement, usize),
) -> Vec<Statement>
{
    vec!(__0)
}

#[allow(unused_variables)]
pub fn __action3<
    'input,
>(
    input: &'input str,
    (_, __0, _): (usize, &'input str, usize),
    (_, __1, _): (usize, &'input str, usize),
    (_, __2, _): (usize, (), usize),
) -> Statement
{
    Statement::Table
}

#[allow(unused_variables)]
pub fn __action4<
    'input,
>(
    input: &'input str,
    (_, __0, _): (usize, &'input str, usize),
) -> ()
{
    ()
}

pub trait __ToTriple<'input, > {
    type Error;
    fn to_triple(value: Self) -> Result<(usize,(usize, &'input str),usize),Self::Error>;
}

impl<'input, > __ToTriple<'input, > for (usize, (usize, &'input str), usize) {
    type Error = ();
    fn to_triple(value: Self) -> Result<(usize,(usize, &'input str),usize),()> {
        Ok(value)
    }
}
impl<'input, > __ToTriple<'input, > for Result<(usize, (usize, &'input str), usize),()> {
    type Error = ();
    fn to_triple(value: Self) -> Result<(usize,(usize, &'input str),usize),()> {
        value
    }
}
