
use std::iter::Iterator;
use std::borrow::Cow;

use lexer::{Lexer, LexerError, Token};
use structure::TomlTable;

pub enum ParseError {
    LexerError(LexerError)
}
impl ParseError {
    pub fn show(&self, text: &str) {
        use self::ParseError::*;
        match *self {
            LexerError(ref err) => {
                err.show(text);
            }
        }
    }
}
impl From<LexerError> for ParseError {
    fn from(err: LexerError) -> ParseError {
        ParseError::LexerError(err)
    }
}

pub struct Parser<'a> {
    text: &'a str,
    lexer: Lexer<'a>,
}
impl<'a> Parser<'a> {
    pub fn new(text: &'a str) -> Parser<'a> {
        Parser {
            text: text,
            lexer: Lexer::new(text),
        }
    }
    
    fn read_scope(&mut self) -> Result<, ParseError> {
        unimplemented!();
    }
    
    pub fn parse(&mut self) -> Result<TomlTable<'a>, ParseError> {
        use lexer::Token::*;
        let mut top_table = TomlTable::new(false);
        let mut cur_table = top_table;
        while let Some(res) = self.lexer.next() {
            match res {
                Ok(Whitespace(text)) | Ok(Newline(text)) => {
                    cur_table.push_space(text);
                }
                Ok(Comment(text)) => {
                    cur_table.push_comment(text);
                }
                Ok(BracketOpen) => {
                    self.read_scope()?;
                }
                Err(e) => {
                    println!("Parse error!");
                    e.show(self.text);
                }
                _ => unimplemented!()
            }
        }
        unimplemented!();
    }
}