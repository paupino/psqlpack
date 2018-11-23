use regex::Regex;
use rust_decimal::Decimal;

use std::iter::FromIterator;

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

#[derive(Clone, Debug, PartialEq, Eq)]
enum LexerState {
    Normal,
    Comment1,
    Comment2,
    String,

    LiteralStart,
    LiteralEnd,
    LiteralBody,
}

// TODO: Add in some sort of message or reason.
#[derive(Debug)]
pub struct LexicalError<'input> {
    pub line: &'input str,
    pub line_number: usize,
    pub start_pos: usize,
    pub end_pos: usize,
}

lazy_static! {
    static ref IDENTIFIER: Regex = Regex::new("^[a-zA-Z][a-zA-Z0-9_]*$").unwrap();
    static ref DECIMAL: Regex = Regex::new("^\\d+\\.\\d+$").unwrap();
    static ref DIGIT: Regex = Regex::new("^\\d+$").unwrap();
}


macro_rules! tokenize_buffer {
    ($tokens:ident, $buffer:ident, $line:ident, $current_line:ident, $current_position:ident) => {{
        if $buffer.len() > 0 {
            let token = match self::create_token(String::from_iter($buffer.clone())) {
                Some(t) => t,
                None => {
                    return Err(LexicalError {
                        line: $line,
                        line_number: $current_line,
                        start_pos: $current_position - $buffer.len(),
                        end_pos: $current_position
                    });
                },
            };
            $tokens.push(token);
            $buffer.clear();
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

fn create_token(value: String) -> Option<Token> {
    if "true".eq_ignore_ascii_case(&value[..]) {
        return Some(Token::Boolean(true));
    }
    if "false".eq_ignore_ascii_case(&value[..]) {
        return Some(Token::Boolean(false));
    }

    // Keywords
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
    match_keyword!(value, CREATE);
    match_keyword!(value, DATE);
    match_keyword!(value, DEFAULT);
    match_keyword!(value, DELETE);
    match_keyword!(value, DESC);
    match_keyword!(value, DOUBLE);
    match_keyword!(value, ENUM);
    match_keyword!(value, EXTENSION);
    match_keyword!(value, FILLFACTOR);
    match_keyword!(value, FIRST);
    match_keyword!(value, FOREIGN);
    match_keyword!(value, FULL);
    match_keyword!(value, FUNCTION);
    match_keyword!(value, GIN);
    match_keyword!(value, GIST);
    match_keyword!(value, HASH);
    match_keyword!(value, IN);
    match_keyword!(value, INDEX);
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
    match_keyword!(value, REPLACE);
    match_keyword!(value, RESTRICT);
    match_keyword!(value, RETURNS);
    match_keyword!(value, SCHEMA);
    match_keyword!(value, SERIAL);
    match_keyword!(value, SERIAL2);
    match_keyword!(value, SERIAL4);
    match_keyword!(value, SERIAL8);
    match_keyword!(value, SET);
    match_keyword!(value, SETOF);
    match_keyword!(value, SIMPLE);
    match_keyword!(value, SMALLINT);
    match_keyword!(value, SMALLSERIAL);
    match_keyword!(value, TABLE);
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
    // TODO: Consider creating a Context struct to keep state.
    let mut tokens = Vec::new();
    let mut current_line = 0;
    let mut current_position;
    let mut buffer = Vec::new();
    let mut literal = Vec::new();
    let mut state = LexerState::Normal;
    let mut last_char: char;

    // Loop through each character, halting on whitespace
    // Our outer loop works by newline
    let lines: Vec<&str> = text.split('\n').collect();
    for line in lines {
        current_line += 1;
        current_position = 0;
        last_char = '\0'; // Start fresh

        for c in line.chars() {
            match state {
                LexerState::Normal => {
                    // Check if we should be entering the comment state
                    if last_char == '-' && c == '-' {
                        // take off the previous item as it was a comment character and push the buffer
                        if !buffer.is_empty() {
                            buffer.pop();
                        }
                        tokenize_buffer!(tokens, buffer, line, current_line, current_position);
                        state = LexerState::Comment1;
                    } else if last_char == '/' && c == '*' {
                        // take off the previous item as it was a comment character and push the buffer
                        if !buffer.is_empty() {
                            buffer.pop();
                        }
                        tokenize_buffer!(tokens, buffer, line, current_line, current_position);
                        state = LexerState::Comment2;
                    } else if c == '\'' {
                        if buffer.is_empty() {
                            state = LexerState::String;
                        } else {
                            // Invalid state! Must be something like xx'dd
                            return Err(LexicalError {
                                line,
                                line_number: current_line,
                                start_pos: current_position,
                                end_pos: current_position,
                            });
                        }
                    } else if c == '$' {
                        if buffer.is_empty() {
                            state = LexerState::LiteralStart;
                        } else {
                            // Unsupported state in our lexer
                            return Err(LexicalError {
                                line,
                                line_number: current_line,
                                start_pos: current_position,
                                end_pos: current_position,
                            });
                        }
                    } else if c.is_whitespace() {
                        // Simple check for whitespace
                        tokenize_buffer!(tokens, buffer, line, current_line, current_position);
                    } else {
                        // If it is a symbol then don't bother with the buffer
                        match c {
                            '(' => {
                                tokenize_buffer!(tokens, buffer, line, current_line, current_position);
                                tokens.push(Token::LeftBracket);
                            }
                            ')' => {
                                tokenize_buffer!(tokens, buffer, line, current_line, current_position);
                                tokens.push(Token::RightBracket);
                            }
                            ',' => {
                                tokenize_buffer!(tokens, buffer, line, current_line, current_position);
                                tokens.push(Token::Comma);
                            }
                            ':' => {
                                tokenize_buffer!(tokens, buffer, line, current_line, current_position);
                                tokens.push(Token::Colon);
                            }
                            ';' => {
                                tokenize_buffer!(tokens, buffer, line, current_line, current_position);
                                tokens.push(Token::Semicolon);
                            }
                            '=' => {
                                tokenize_buffer!(tokens, buffer, line, current_line, current_position);
                                tokens.push(Token::Equals);
                            }
                            '.' => {
                                // If it is just a plain digit in the buffer, then allow it to continue.
                                if buffer.iter().all(|c: &char| c.is_digit(10)) {
                                    buffer.push(c);
                                } else {
                                    tokenize_buffer!(tokens, buffer, line, current_line, current_position);
                                    tokens.push(Token::Period);
                                }
                            }
                            '[' => {
                                tokenize_buffer!(tokens, buffer, line, current_line, current_position);
                                tokens.push(Token::LeftSquare);
                            }
                            ']' => {
                                tokenize_buffer!(tokens, buffer, line, current_line, current_position);
                                tokens.push(Token::RightSquare);
                            }
                            _ => buffer.push(c),
                        }
                    }
                }
                LexerState::Comment1 => {
                    // Ignore comments
                }
                LexerState::Comment2 => {
                    if last_char == '*' && c == '/' {
                        state = LexerState::Normal;
                    }
                    // Ignore comments
                }
                LexerState::String => if c == '\'' {
                    tokens.push(Token::StringValue(String::from_iter(buffer.clone())));
                    buffer.clear();
                    state = LexerState::Normal;
                } else {
                    buffer.push(c);
                },
                LexerState::LiteralStart => {
                    if c == '$' {
                        state = LexerState::LiteralBody;
                    } else {
                        literal.push(c);
                    }
                }
                LexerState::LiteralEnd => {
                    if c == '$' {
                        if literal.is_empty() {
                            state = LexerState::Normal;
                        } else {
                            // Error: literal name mismatch
                            return Err(LexicalError {
                                line,
                                line_number: current_line,
                                start_pos: current_position,
                                end_pos: current_position,
                            });
                        }
                    } else if literal.is_empty() {
                        // Error: literal name mismatch
                        return Err(LexicalError {
                            line,
                            line_number: current_line,
                            start_pos: current_position,
                            end_pos: current_position,
                        });
                    } else {
                        let l = literal.pop().unwrap();
                        if l != c {
                            // Error: literal name mismatch
                            return Err(LexicalError {
                                line,
                                line_number: current_line,
                                start_pos: current_position,
                                end_pos: current_position,
                            });
                        }
                    }
                }
                LexerState::LiteralBody => {
                    if c == '$' {
                        let data = String::from_iter(buffer.clone());
                        tokens.push(Token::Literal(data.trim().into()));
                        buffer.clear();
                        if !literal.is_empty() {
                            literal.reverse();
                        }
                        state = LexerState::LiteralEnd;
                    } else {
                        buffer.push(c);
                    }
                }
            }

            // Move the current_position
            current_position += 1;
            last_char = c;
        }

        // If we were a single line comment, we go back to a normal state on a new line
        match state {
            LexerState::Normal => {
                // We may also have a full buffer
                tokenize_buffer!(tokens, buffer, line, current_line, current_position);
            }
            LexerState::Comment1 => {
                // End of a line finishes the comment
                state = LexerState::Normal;
            }
            LexerState::Comment2 => {
                // Do nothing at the end of a line - it's a multi-line comment
            }
            LexerState::String | LexerState::LiteralStart | LexerState::LiteralEnd => {
                // If we're in these states at the end of a line it's an error
                // (e.g. at the moment we don't support multi-line strings)
                return Err(LexicalError {
                    line,
                    line_number: current_line,
                    start_pos: current_position,
                    end_pos: current_position,
                });
            }
            LexerState::LiteralBody => {
                // Add a new line onto the buffer
                buffer.push('\n');
            }
        }
    }

    Ok(tokens)
}
