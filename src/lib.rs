#![feature(question_mark)]
#![feature(slice_patterns)]

pub mod debug;

mod utils;
mod tokens;
mod parse;
mod key;
mod scope;
mod table;
mod array;
mod value;

pub use tokens::{tokens, Tokens, Token, TokenError};
pub use table::{CreatePathError, TomlTable};
pub use array::TomlArray;
pub use value::TomlValue;
pub use parse::{parse, ParseError};
