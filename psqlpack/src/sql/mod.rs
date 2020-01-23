pub mod ast;
mod bootstrap;
pub mod lexer;

pub use bootstrap::parser;

#[cfg(test)]
mod tests;
