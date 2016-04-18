
use std::iter::Iterator;
use std::borrow::Cow;

use self::lexer::{Lexer, }

pub struct Parser<'a> {
    text: &'a str,
}
impl<'a> Parser<'a> {
    pub fn new(text: &'a str) -> Parser<'a> {
        Parser {
            text: text
        }
    }
}

type TomlValue = String;

enum ParseItem<'a> {
    Scope(Vec<Cow<'a, str>>),
    Assignment(Cow<'a, str>, TomlValue)
}

enum ParseError {
    LexerError()
}

impl<'a> Iterator for Parse<'a> {
    type Item = 
}

