
use std::iter::Iterator;
use std::borrow::Cow;

use lexer::{Lexer, LexerError, Token};
use structure::{TomlTable, TomlKey, clean_string};
use debug;

#[derive(Debug)]
pub enum ParseError {
    LexerError(LexerError),
    InvalidScope { start: usize, pos: usize },
}
impl ParseError {
    pub fn show(&self, text: &str) {
        use self::ParseError::*;
        match *self {
            LexerError(ref err) => {
                err.show(text);
            }
            InvalidScope { start, pos } => {
                let (line, col) = debug::get_position(text, pos);
                println!("Invalid scope found at {}:{} :", line, col);
                debug::show_invalid_part(text, start, pos);
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
        use lexer::TokenData::*;
        let mut top_table = TomlTable::new(false);
        let mut cur_table = top_table;
        while let Some(res) = self.lexer.next() {
            match res {
                Ok(res) => {
                    match res.data {
                        Whitespace(text) | Newline(text) => {
                            cur_table.push_space(text);
                        }
                        SingleBracketOpen => {
                            let scope = self.read_scope(false)?;
                        }
                        DoubleBracketOpen => {
                            let scope = self.read_scope(true)?;
                        }
                        Comment(text) => {
                            cur_table.push_comment(text);
                        }
                        Key(text) => {
                            let key = TomlKey::Plain(text);
                            println!("Key:");
                            key.show();
                        }
                        String { text, literal, multiline } => {
                            let key = TomlKey::String {
                                text: text, literal: literal, multiline: multiline
                            };
                            println!("Key:");
                            key.show();
                        }
                        other => {
                            panic!("Unexpected element: {:?}", other);
                        }
                    }
                }
                Err(err) => {
                    println!("Parse error!");
                    err.show(self.text);
                }
            }
            
        }
        unimplemented!();
    }
}