pub mod ast;
pub mod lexer;
mod bootstrap;

pub use bootstrap::parser;

#[cfg(test)]
mod tests;
