use regex::Regex;
use std::ascii::AsciiExt;
use std::iter::FromIterator;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Token {
    
    BIGINT,
    BIGSERIAL,
    BIT,
    BOOL,
    BOOLEAN,
    CONSTRAINT,
    CHAR,
    CHARACTER,
    CREATE,
    DATE,
    DOUBLE,
    FILLFACTOR,
    FOREIGN,
    INT,
    INT2,
    INT4,
    INT8,
    INTEGER,
    KEY,
    MONEY,
    NOT,
    NULL,
    NUMERIC,
    PRECISION,
    PRIMARY,
    REAL,
    REFERENCES,
    SERIAL,
    SERIAL2,
    SERIAL4,
    SERIAL8,
    SMALLINT,
    SMALLSERIAL,
    TABLE,
    TEXT,
    TIME,
    TIMESTAMP,
    TIMESTAMPTZ,
    TIMETZ,
    UNIQUE,
    UUID,
    VARBIT,
    VARCHAR,
    VARYING,
    WITH,
    WITHOUT,
    ZONE,

    Identifier(String),
    Digit(i32),

    LeftBracket,
    RightBracket,
    Comma,
    Period,
    Semicolon,
    Equals,
}

#[derive(Debug)]
pub struct LexicalError<'input> {
    pub line: &'input str,
    pub line_number: i32,
    pub start_pos: i32,
    pub end_pos: i32,
}

lazy_static! {

    static ref IDENTIFIER: Regex = Regex::new("^[a-zA-Z][a-zA-Z0-9_]+$").unwrap();
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
                        start_pos: $current_position - $buffer.len() as i32,
                        end_pos: $current_position as i32
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
        //println!("Match {}, Value: {}", raw.eq_ignore_ascii_case(&$value[..]), $value);
        if raw.eq_ignore_ascii_case(&$value[..]) {
            return Some(Token::$enum_value);
        }
    }};
}

fn create_token<'input>(value: String) -> Option<Token> {

    // Keywords
    match_keyword!(value, BIGINT);
    match_keyword!(value, BIGSERIAL);
    match_keyword!(value, BIT);
    match_keyword!(value, BOOL);
    match_keyword!(value, BOOLEAN);
    match_keyword!(value, CONSTRAINT);
    match_keyword!(value, CHAR);
    match_keyword!(value, CHARACTER);
    match_keyword!(value, CREATE);
    match_keyword!(value, DATE);
    match_keyword!(value, DOUBLE);
    match_keyword!(value, FILLFACTOR);
    match_keyword!(value, FOREIGN);
    match_keyword!(value, INT);
    match_keyword!(value, INT2);
    match_keyword!(value, INT4);
    match_keyword!(value, INT8);
    match_keyword!(value, INTEGER);
    match_keyword!(value, KEY);
    match_keyword!(value, MONEY);
    match_keyword!(value, NOT);
    match_keyword!(value, NULL);
    match_keyword!(value, NUMERIC);
    match_keyword!(value, PRECISION);
    match_keyword!(value, PRIMARY);
    match_keyword!(value, REAL);
    match_keyword!(value, REFERENCES);
    match_keyword!(value, SERIAL);
    match_keyword!(value, SERIAL2);
    match_keyword!(value, SERIAL4);
    match_keyword!(value, SERIAL8);
    match_keyword!(value, SMALLINT);
    match_keyword!(value, SMALLSERIAL);
    match_keyword!(value, TABLE);
    match_keyword!(value, TEXT);
    match_keyword!(value, TIME);
    match_keyword!(value, TIMESTAMP);
    match_keyword!(value, TIMESTAMPTZ);
    match_keyword!(value, TIMETZ);
    match_keyword!(value, UNIQUE);
    match_keyword!(value, UUID);
    match_keyword!(value, VARBIT);
    match_keyword!(value, VARCHAR);
    match_keyword!(value, VARYING);
    match_keyword!(value, WITH);
    match_keyword!(value, WITHOUT);
    match_keyword!(value, ZONE);

    // Regex
    if IDENTIFIER.is_match(&value[..]) {
        return Some(Token::Identifier(value));
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
    let mut current_line = 0;
    let mut current_position;
    let mut buffer = Vec::new();

    // Loop through each character, halting on whitespace
    // Our outer loop works by newline
    let lines: Vec<&str> = text.split('\n').collect();
    for line in lines {
        current_line += 1;
        current_position = 0;

        for c in line.chars() {
            // Simple check for whitespace
            if c.is_whitespace() {
                tokenize_buffer!(tokens, buffer, line, current_line, current_position);
            } else {

                // If it is a symbol then don't bother with the buffer
                match c {
                    '(' => {
                        tokenize_buffer!(tokens, buffer, line, current_line, current_position);
                        tokens.push(Token::LeftBracket);
                    }, 
                    ')' => {
                        tokenize_buffer!(tokens, buffer, line, current_line, current_position);
                        tokens.push(Token::RightBracket);
                    },
                    ',' => {
                        tokenize_buffer!(tokens, buffer, line, current_line, current_position);
                        tokens.push(Token::Comma);
                    }, 
                    ';' => {
                        tokenize_buffer!(tokens, buffer, line, current_line, current_position);
                        tokens.push(Token::Semicolon);
                    },
                    '=' => {
                        tokenize_buffer!(tokens, buffer, line, current_line, current_position);
                        tokens.push(Token::Equals);
                    }, 
                    '.' => {
                        tokenize_buffer!(tokens, buffer, line, current_line, current_position);
                        tokens.push(Token::Period);
                    }, 
                    _ => buffer.push(c),
                }
            }

            // Move the current_position
            current_position += 1;
        }

        // We may also have a full buffer
        tokenize_buffer!(tokens, buffer, line, current_line, current_position);
    }

    Ok(tokens)
}