#![allow(clippy::all)]
#![allow(unused_parens)]
use lalrpop_util::lalrpop_mod;

lalrpop_mod!(pub parser, "/src/sql/parser.rs");
