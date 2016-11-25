#![feature(slice_patterns)]
#![deny(missing_docs)]
//! Parses and edits TOML documents while preserving the formatting of the original document.
#[macro_use]
extern crate log;
extern crate env_logger;

pub mod debug;

mod utils;
mod lexer;
mod parse;
mod key;
mod scope;
mod table;
mod array;
mod value;

pub use lexer::{tokens, Tokens, Token};
/// An error found when lexing a TOML document.
pub type LexError<'a> = lexer::Error<'a>;
/// The kinds of errors that can be found when lexing a TOML document.
pub type LexerErrorKind = lexer::ErrorKind;
pub use table::{CreatePathError, Table};
pub use array::Array;
pub use value::{Value, Int, Float, TomlString};
pub use parse::{parse, Error, ErrorKind, Result};
