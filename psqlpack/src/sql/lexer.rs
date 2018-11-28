/*
TODO: This isn't all that efficient. We could gain some efficiencies using a generated lexer.
TODO: Proper lookahead.
*/

use regex::Regex;
use rust_decimal::Decimal;

use std::iter::FromIterator;

use self::context::*;

// TODO: Add in some sort of message or reason.
#[derive(Debug)]
pub struct LexicalError<'input> {
    pub line: &'input str,
    pub line_number: usize,
    pub start_pos: usize,
    pub end_pos: usize,
    pub lexer_state: String,
}

mod context {
    use super::LexicalError;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum LexerState {
        Normal(NormalVariant),

        Comment1,
        Comment2,
        String,

        LiteralStart,
        LiteralEnd,
        LiteralBody,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum NormalVariant {
        Any,
        Definition,
        Body
    }

    pub struct Context {
        current_line: usize,
        current_position: usize,
        pub last_char: char,

        pub buffer: Vec<char>,
        pub literal: Vec<char>,

        state: Vec<LexerState>,
    }

    impl Context {
        pub fn new() -> Self {
            Context {
                current_line: 0,
                current_position: 0,
                last_char: '\0',

                buffer: Vec::new(),
                literal: Vec::new(),

                state: vec![LexerState::Normal(NormalVariant::Any)],
            }
        }

        pub fn new_line(&mut self) {
            self.current_line += 1;
            self.current_position = 0;
            self.last_char = '\0'; // Start fresh
        }

        pub fn next_char(&mut self, c: char) {
            self.current_position += 1;
            self.last_char = c;
        }

        pub fn create_error<'input>(&self, line: &'input str) -> LexicalError<'input> {
            LexicalError {
                line,
                line_number: self.current_line,
                start_pos: self.current_position,
                end_pos: self.current_position - self.buffer.len(),
                lexer_state: self.state
                    .iter()
                    .map(|s| match s {
                        LexerState::Normal(variant) => match variant {
                            NormalVariant::Any => "Normal(Any)",
                            NormalVariant::Definition => "Normal(Definition)",
                            NormalVariant::Body => "Normal(Body)",
                        },
                        LexerState::Comment1 => "CommentLine",
                        LexerState::Comment2 => "CommentBlock",
                        LexerState::String => "String",
                        LexerState::LiteralStart => "LiteralBegin",
                        LexerState::LiteralBody => "Literal",
                        LexerState::LiteralEnd => "LiteralEnd",
                    })
                    .collect::<Vec<_>>()
                    .join(" -> "),
            }
        }

        pub fn push_state(&mut self, state: LexerState) {
            self.state.push(state);
        }

        pub fn pop_state(&mut self) {
            self.state.pop();
        }

        pub fn replace_state(&mut self, state: LexerState) {
            self.state.pop();
            self.state.push(state);
        }

        pub fn peek_state(&self) -> LexerState {
            if let Some(item) = self.state.last() {
                return *item;
            } else {
                panic!("Nothing left in the stack");
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Token {
    ACTION,
    AS,
    ASC,
    BIGINT,
    BIGSERIAL,
    BIT,
    BOOL,
    BOOLEAN,
    BTREE,
    CASCADE,
    CONSTRAINT,
    CHAR,
    CHARACTER,
    CREATE,
    DATE,
    DEFAULT,
    DELETE,
    DESC,
    DOUBLE,
    ENUM,
    EXTENSION,
    FILLFACTOR,
    FIRST,
    FOREIGN,
    FULL,
    FUNCTION,
    GIN,
    GIST,
    HASH,
    IN,
    INDEX,
    INOUT,
    INT,
    INT2,
    INT4,
    INT8,
    INTEGER,
    KEY,
    LANGUAGE,
    LAST,
    MATCH,
    MONEY,
    NO,
    NOT,
    NULL,
    NULLS,
    NUMERIC,
    ON,
    OR,
    OUT,
    PARTIAL,
    PRECISION,
    PRIMARY,
    REAL,
    REFERENCES,
    REPLACE,
    RESTRICT,
    RETURNS,
    SCHEMA,
    SERIAL,
    SERIAL2,
    SERIAL4,
    SERIAL8,
    SET,
    SETOF,
    SIMPLE,
    SMALLINT,
    SMALLSERIAL,
    TABLE,
    TEXT,
    TIME,
    TIMESTAMP,
    TIMESTAMPTZ,
    TIMETZ,
    TYPE,
    UNIQUE,
    UPDATE,
    USING,
    UUID,
    VARBIT,
    VARCHAR,
    VARIADIC,
    VARYING,
    WITH,
    WITHOUT,
    ZONE,

    Identifier(String),
    Digit(i32),
    Decimal(Decimal),
    Boolean(bool),
    StringValue(String),
    Literal(String),

    LeftBracket,
    RightBracket,
    LeftSquare,
    RightSquare,

    Colon,
    Comma,
    Period,
    Semicolon,
    Equals,
}

lazy_static! {
    static ref IDENTIFIER: Regex = Regex::new("^[a-zA-Z][a-zA-Z0-9_]*$").unwrap();
    static ref DECIMAL: Regex = Regex::new("^\\d+\\.\\d+$").unwrap();
    static ref DIGIT: Regex = Regex::new("^\\d+$").unwrap();
}


macro_rules! tokenize_normal_buffer {
    ($context:ident, $line:ident, $tokens:ident) => {{
        if $context.buffer.len() > 0 {
            let token = match self::create_normal_token(&mut $context) {
                Some(t) => t,
                None => return Err($context.create_error($line)),
            };
            $tokens.push(token);
            $context.buffer.clear();
        }
    }};
}


macro_rules! match_keyword {
    ($value:ident, $enum_value:ident) => {{
        let raw = stringify!($enum_value);
        if raw.eq_ignore_ascii_case(&$value[..]) {
            return Some(Token::$enum_value);
        }
    }};
}

macro_rules! match_keyword_replace_state {
    ($context:ident, $variant:expr, $value:ident, $enum_value:ident) => {{
        let raw = stringify!($enum_value);
        if raw.eq_ignore_ascii_case(&$value[..]) {
            $context.replace_state(LexerState::Normal($variant));
            return Some(Token::$enum_value);
        }
    }};
}


fn create_normal_token(context: &mut Context) -> Option<Token> {
    let variant = if let LexerState::Normal(variant) = context.peek_state() {
        variant
    } else {
        return None;
    };

    let value = String::from_iter(context.buffer.clone());
    if "true".eq_ignore_ascii_case(&value[..]) {
        return Some(Token::Boolean(true));
    }
    if "false".eq_ignore_ascii_case(&value[..]) {
        return Some(Token::Boolean(false));
    }

    // Keywords - this is very naive and should be generated.
    match variant {
        NormalVariant::Any | NormalVariant::Definition => {
            match_keyword!(value, CREATE);
            match_keyword!(value, OR);
            match_keyword!(value, REPLACE);

            // Any of the below will switch state. This only gets reset on statement end.
            match_keyword_replace_state!(context, NormalVariant::Body, value, EXTENSION);
            match_keyword_replace_state!(context, NormalVariant::Body, value, FUNCTION);
            match_keyword_replace_state!(context, NormalVariant::Body, value, INDEX);
            match_keyword_replace_state!(context, NormalVariant::Body, value, SCHEMA);
            match_keyword_replace_state!(context, NormalVariant::Body, value, TABLE);
        }
        _ => {}
    }
    match variant {
        NormalVariant::Any | NormalVariant::Body => {
            match_keyword!(value, ACTION);
            match_keyword!(value, AS);
            match_keyword!(value, ASC);
            match_keyword!(value, BIGINT);
            match_keyword!(value, BIGSERIAL);
            match_keyword!(value, BIT);
            match_keyword!(value, BOOL);
            match_keyword!(value, BOOLEAN);
            match_keyword!(value, BTREE);
            match_keyword!(value, CASCADE);
            match_keyword!(value, CONSTRAINT);
            match_keyword!(value, CHAR);
            match_keyword!(value, CHARACTER);
            match_keyword!(value, DATE);
            match_keyword!(value, DEFAULT);
            match_keyword!(value, DELETE);
            match_keyword!(value, DESC);
            match_keyword!(value, DOUBLE);
            match_keyword!(value, ENUM);
            match_keyword!(value, FILLFACTOR);
            match_keyword!(value, FIRST);
            match_keyword!(value, FOREIGN);
            match_keyword!(value, FULL);
            match_keyword!(value, GIN);
            match_keyword!(value, GIST);
            match_keyword!(value, HASH);
            match_keyword!(value, IN);
            match_keyword!(value, INOUT);
            match_keyword!(value, INT);
            match_keyword!(value, INT2);
            match_keyword!(value, INT4);
            match_keyword!(value, INT8);
            match_keyword!(value, INTEGER);
            match_keyword!(value, KEY);
            match_keyword!(value, LANGUAGE);
            match_keyword!(value, LAST);
            match_keyword!(value, MATCH);
            match_keyword!(value, MONEY);
            match_keyword!(value, NO);
            match_keyword!(value, NOT);
            match_keyword!(value, NULL);
            match_keyword!(value, NULLS);
            match_keyword!(value, NUMERIC);
            match_keyword!(value, ON);
            match_keyword!(value, OR);
            match_keyword!(value, OUT);
            match_keyword!(value, PARTIAL);
            match_keyword!(value, PRECISION);
            match_keyword!(value, PRIMARY);
            match_keyword!(value, REAL);
            match_keyword!(value, REFERENCES);
            match_keyword!(value, RESTRICT);
            match_keyword!(value, RETURNS);
            match_keyword!(value, SERIAL);
            match_keyword!(value, SERIAL2);
            match_keyword!(value, SERIAL4);
            match_keyword!(value, SERIAL8);
            match_keyword!(value, SET);
            match_keyword!(value, SETOF);
            match_keyword!(value, SIMPLE);
            match_keyword!(value, SMALLINT);
            match_keyword!(value, SMALLSERIAL);
            match_keyword!(value, TEXT);
            match_keyword!(value, TIME);
            match_keyword!(value, TIMESTAMP);
            match_keyword!(value, TIMESTAMPTZ);
            match_keyword!(value, TIMETZ);
            match_keyword!(value, TYPE);
            match_keyword!(value, UNIQUE);
            match_keyword!(value, UPDATE);
            match_keyword!(value, USING);
            match_keyword!(value, UUID);
            match_keyword!(value, VARBIT);
            match_keyword!(value, VARCHAR);
            match_keyword!(value, VARIADIC);
            match_keyword!(value, VARYING);
            match_keyword!(value, WITH);
            match_keyword!(value, WITHOUT);
            match_keyword!(value, ZONE);
        }
        _ => {}
    }

    // Regex
    if IDENTIFIER.is_match(&value[..]) {
        return Some(Token::Identifier(value));
    }
    if DECIMAL.is_match(&value[..]) {
        return Some(Token::Decimal(value.parse::<Decimal>().unwrap()));
    }
    if DIGIT.is_match(&value[..]) {
        return Some(Token::Digit(value.parse::<i32>().unwrap()));
    }

    // Error
    None
}

pub fn tokenize(text: &str) -> Result<Vec<Token>, LexicalError> {

    // This tokenizer is whitespace dependent by default, i.e. whitespace is relevant.
    let mut tokens = Vec::new();
    let mut context = Context::new();

    // Loop through each character, halting on whitespace
    // Our outer loop works by newline
    let lines: Vec<&str> = text.split('\n').collect();
    for line in lines {
        context.new_line();

        for c in line.chars() {
            match context.peek_state() {
                LexerState::Normal(_) => {
                    // Check if we should be entering the comment state
                    if context.last_char == '-' && c == '-' {
                        // take off the previous item as it was a comment character and push the buffer
                        context.buffer.pop();
                        tokenize_normal_buffer!(context, line, tokens);
                        context.push_state(LexerState::Comment1);
                    } else if context.last_char == '/' && c == '*' {
                        // take off the previous item as it was a comment character and push the buffer
                        context.buffer.pop();
                        tokenize_normal_buffer!(context, line, tokens);
                        context.push_state(LexerState::Comment2);
                    } else if c == '\'' {
                        if context.buffer.is_empty() {
                            context.push_state(LexerState::String);
                        } else {
                            // Invalid state! Must be something like xx'dd
                            return Err(context.create_error(line));
                        }
                    } else if c == '$' {
                        if context.buffer.is_empty() {
                            context.push_state(LexerState::LiteralStart);
                        } else {
                            // Unsupported state in our lexer
                            return Err(context.create_error(line));
                        }
                    } else if c.is_whitespace() {
                        // Simple check for whitespace
                        tokenize_normal_buffer!(context, line, tokens);
                    } else {
                        // If it is a symbol then don't bother with the buffer
                        match c {
                            '(' => {
                                tokenize_normal_buffer!(context, line, tokens);
                                tokens.push(Token::LeftBracket);
                            }
                            ')' => {
                                tokenize_normal_buffer!(context, line, tokens);
                                tokens.push(Token::RightBracket);
                            }
                            ',' => {
                                tokenize_normal_buffer!(context, line, tokens);
                                tokens.push(Token::Comma);
                            }
                            ':' => {
                                tokenize_normal_buffer!(context, line, tokens);
                                tokens.push(Token::Colon);
                            }
                            ';' => {
                                tokenize_normal_buffer!(context, line, tokens);
                                tokens.push(Token::Semicolon);
                                context.replace_state(LexerState::Normal(NormalVariant::Any));
                            }
                            '=' => {
                                tokenize_normal_buffer!(context, line, tokens);
                                tokens.push(Token::Equals);
                            }
                            '.' => {
                                // If it is just a plain digit in the buffer, then allow it to continue.
                                if context.buffer.iter().all(|c: &char| c.is_digit(10)) {
                                    context.buffer.push(c);
                                } else {
                                    tokenize_normal_buffer!(context, line, tokens);
                                    tokens.push(Token::Period);
                                }
                            }
                            '[' => {
                                tokenize_normal_buffer!(context, line, tokens);
                                tokens.push(Token::LeftSquare);
                            }
                            ']' => {
                                tokenize_normal_buffer!(context, line, tokens);
                                tokens.push(Token::RightSquare);
                            }
                            _ => context.buffer.push(c),
                        }
                    }
                }
                LexerState::Comment1 => {
                    // Ignore comments
                }
                LexerState::Comment2 => {
                    if context.last_char == '*' && c == '/' {
                        context.pop_state();
                    }
                    // Ignore comments
                }
                LexerState::String => if c == '\'' {
                    tokens.push(Token::StringValue(String::from_iter(context.buffer.clone())));
                    context.buffer.clear();
                    context.pop_state();
                } else {
                    context.buffer.push(c);
                },
                LexerState::LiteralStart => {
                    if c == '$' {
                        context.replace_state(LexerState::LiteralBody);
                    } else {
                        context.literal.push(c);
                    }
                }
                LexerState::LiteralEnd => {
                    if c == '$' {
                        if context.literal.is_empty() {
                            context.pop_state();
                        } else {
                            // Error: literal name mismatch
                            return Err(context.create_error(line));
                        }
                    } else if context.literal.is_empty() {
                        // Error: literal name mismatch
                        return Err(context.create_error(line));
                    } else {
                        let l = context.literal.pop().unwrap();
                        if l != c {
                            // Error: literal name mismatch
                            return Err(context.create_error(line));
                        }
                    }
                }
                LexerState::LiteralBody => {
                    // We only escape from a literal body if the next few characters are
                    // in fact part of the literal. For example, we may be using $1 as a positional
                    // argument.
                    if c == '$' {
                        // We're in a maybe state... we don't want to do anything with the buffer
                        // yet. This is where look ahead is useful.
                        // Since we don't have a lookahead system implemented (yet) we do a soft
                        // change to MaybeLiteralEnd
                        // TODO
                        let data = String::from_iter(context.buffer.clone());
                        tokens.push(Token::Literal(data.trim().into()));
                        context.buffer.clear();
                        if !context.literal.is_empty() {
                            context.literal.reverse();
                        }
                        context.replace_state(LexerState::LiteralEnd);
                    } else {
                        context.buffer.push(c);
                    }
                }
            }

            // Move the current_position
            context.next_char(c);
        }

        // If we were a single line comment, we go back to a normal state on a new line
        match context.peek_state() {
            LexerState::Normal(_) => {
                // We may also have a full buffer
                tokenize_normal_buffer!(context, line, tokens);
            }
            LexerState::Comment1 => {
                // End of a line finishes the comment
                context.pop_state();
            }
            LexerState::Comment2 => {
                // Do nothing at the end of a line - it's a multi-line comment
            }
            LexerState::String | LexerState::LiteralStart | LexerState::LiteralEnd => {
                // If we're in these states at the end of a line it's an error
                // (e.g. at the moment we don't support multi-line strings)
                return Err(context.create_error(line));
            }
            LexerState::LiteralBody => {
                // Add a new line onto the buffer
                context.buffer.push('\n');
            }
        }
    }

    Ok(tokens)
}
