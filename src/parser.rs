
use std::iter::Iterator;
use std::borrow::Cow;

use lexer::{Lexer, LexerError, Token};
use structure::{TomlTable, TomlKey};

pub enum ParseError {
    LexerError(LexerError),
    EmptyScope,
}
impl ParseError {
    pub fn show(&self, text: &str) {
        use self::ParseError::*;
        match *self {
            LexerError(ref err) => {
                err.show(text);
            }
            EmptyScope => {
                println!("Empty scope found :c");
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
    
    fn read_scope(&mut self, array: bool) -> Result<Vec<TomlKey<'a>>, ParseError> {
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
                Ok(SingleBracketOpen) => {
                    let scope = self.read_scope(false)?;
                }
                Ok(DoubleBracketOpen) => {
                    let scope = self.read_scope(true)?;
                }
                Ok(Comment(text)) => {
                    cur_table.push_comment(text);
                }
                Ok(Key(text)) => {
                    
                }
                Ok(String { text, literal, multiline }) => {
        
                }
                Err(err) => {
                    println!("Parse error!");
                    err.show(self.text);
                }
                Ok(other) => {
                    panic!("Unexpected element: {:?}", other);
                }
            }
        }
        unimplemented!();
    }
}