use lalrpop_util::{Spanned};

#[derive(Debug)]
struct Token {
    value: String,
    line_number: u32,
    start_position: u32,
    end_position: u32,
    symbol: Symbol,
}

#[derive(Debug)]
enum Symbol {
    CREATE,
    TABLE,
    Identifier
}

pub enum LexicalError {
    
}

use std::str::CharIndices;

pub struct Lexer<'input> {
    chars: CharIndices<'input>,
}

impl<'input> Lexer<'input> {
    pub fn new(input: &'input str) -> Self {
        Lexer { chars: input.char_indices() }
    }
}

impl<'input> Iterator for Lexer<'input> {
    type Item = Spanned<Token, usize, LexicalError>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut buffer = Vec::new();
        let mut current_line = 0;
        let mut current_position;

        loop {
            match self.chars.next() {
                Some((i, c)) => {
                    println!("{}", c);
                },
                None => return None, // End of file
            }
        }
    }
}