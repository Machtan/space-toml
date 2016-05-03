
use std::iter::{Iterator, Peekable};
use std::borrow::Cow;

use lexer::{Lexer, LexerError, Token};
use structure::{TomlTable, TomlKey, TomlValue, clean_string, Scope};
use debug;

#[derive(Debug)]
pub enum ParseError {
    LexerError(LexerError),
    InvalidScope { start: usize, pos: usize },
    UnfinishedScope { start: usize },
    UnfinishedItem { start: usize },
    MissingEquals { start: usize, pos: usize },
}
impl ParseError {
    pub fn show(&self, text: &str) {
        use self::ParseError::*;
        match *self {
            LexerError(ref err) => {
                print!("Lexer: ");
                err.show(text);
            }
            InvalidScope { start, pos } => {
                let (line, col) = debug::get_position(text, pos);
                println!("Invalid scope found at {}:{} :", line, col);
                debug::show_invalid_part(text, start, pos);
            }
            UnfinishedScope { start } => {
                let (line, col) = debug::get_position(text, start);
                println!("Unifinished scope starting at {}:{} :", line, col);
                debug::show_unclosed(text, start);
            }
            UnfinishedItem { start } => {
                let (line, col) = debug::get_position(text, start);
                println!("No value found for key at {}:{} :", line, col);
                debug::show_unclosed(text, start);
            }
            MissingEquals { start, pos } => {
                let (line, col) = debug::get_position(text, pos);
                println!("'=' expected at {}:{} :", line, col);
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
    lexer: Peekable<Lexer<'a>>,
}
impl<'a> Parser<'a> {
    pub fn new(text: &'a str) -> Parser<'a> {
        Parser {
            text: text,
            lexer: Lexer::new(text).peekable(),
        }
    }
    
    fn read_scope(&mut self, array: bool, start: usize)
            -> Result<Scope<'a>, ParseError> {
        use lexer::TokenData::*;
        use self::ParseError::*;
        println!("scope, Array: {}", array);
        let mut was_key = false;
        let mut scope = Scope::new();
        let mut key_found = false;
        let mut closed = false;
        while let Some(res) = self.lexer.next() {
            let res = res?;
            match res.data {
                Dot => {
                    if ! was_key {
                        return Err(InvalidScope { start: start, pos: res.start });
                    } else {
                        scope.push_dot();
                        was_key = false;
                    }
                }
                SingleBracketClose if ! array => {
                    if ! key_found {
                        return Err(InvalidScope { start: start, pos: res.start });
                    } else if ! was_key {
                        return Err(InvalidScope { start: start, pos: res.start});
                    }
                    closed = true;
                    break;
                }
                DoubleBracketClose if array => {
                    if ! key_found {
                        return Err(InvalidScope { start: start, pos: res.start });
                    } else if ! was_key {
                        return Err(InvalidScope { start: start, pos: res.start});
                    }
                    closed = true;
                    break;
                }
                Whitespace(text) => {
                    scope.push_space(text);
                }
                Key(text) => {
                    key_found = true;
                    was_key = true;
                    scope.push_key(TomlKey::from_key(text));
                }
                String { text, literal, multiline } => {
                    key_found = true;
                    was_key = true;
                    scope.push_key(TomlKey::from_string(text, literal, multiline));
                }
                other => {
                    return Err(InvalidScope { start: start, pos: res.start });
                }
            }
        }
        if ! closed {
            return Err(UnfinishedScope { start: start });
        }
        Ok(scope)
    }
    
    fn read_value(&mut self, start: usize)
            -> Result<TomlValue<'a>, ParseError> {
        println!("Reading value...");
        unimplemented!();
    }
    
    fn read_item(&mut self, start: usize, key: TomlKey<'a>)
            -> Result<(TomlKey<'a>, Option<&'a str>, Option<&'a str>, TomlValue<'a>), ParseError> {
        use self::ParseError::*;
        use lexer::TokenData::*;
        let mut before_eq = None;
        let mut next = self.next_or(UnfinishedItem { start: start })?;
        if let Whitespace(text) = next.data {
            before_eq = Some(text);
            next = self.next_or(UnfinishedItem { start: start })?;
        }
        
        if let Equals = next.data {
        } else {
            return Err(MissingEquals { start: start, pos: next.start });
        }
        
        let mut after_eq = None;
        let mut has_whitespace_after = false;
        if let Whitespace(_) = self.peek_or(UnfinishedItem { start: start })?.data {
            has_whitespace_after = true;
        }
        if has_whitespace_after {
            next = self.lexer.next().unwrap()?;
            if let Whitespace(text) = next.data {
                after_eq = Some(text);
            }
        }
        
        println!("{:?} {:?} = {:?} ", key, before_eq, after_eq);
        
        let value = self.read_value(start)?;
        Ok((key, before_eq, after_eq, value))
    }
    
    fn next_or(&mut self, err: ParseError) -> Result<Token<'a>, ParseError> {
        match self.lexer.next() {
            Some(val) => {
                Ok(val?)
            },
            None => Err(err)
        }
    }
    
    fn peek_or(&mut self, err: ParseError) -> Result<&Token<'a>, ParseError> {
        match self.lexer.peek() {
            Some(res) => {
                match res {
                    &Err(ref e) => {
                        Err(ParseError::from(e.clone()))
                    },
                    &Ok(ref token) => Ok(token),
                }
            }
            None => Err(err)
        }
    }
    
    pub fn parse(&mut self) -> Result<TomlTable<'a>, ParseError> {
        use lexer::TokenData::*;
        use self::ParseError::*;
        let mut top_table = TomlTable::new(false);
        let mut cur_table = top_table;
        while let Some(res) = self.lexer.next() {
            let res = res?;
            match res.data {
                Whitespace(text) | Newline(text) => {
                    cur_table.push_space(text);
                }
                SingleBracketOpen => {
                    let scope = self.read_scope(false, res.start)?;
                    println!("Scope: {:?}", scope);
                }
                DoubleBracketOpen => {
                    let scope = self.read_scope(true, res.start)?;
                    println!("Scope: {:?}", scope);
                }
                Comment(text) => {
                    cur_table.push_comment(text);
                }
                Key(_) | String { .. } => {
                    let key = if let Key(text) = res.data {
                        TomlKey::Plain(text)
                    } else if let String { text, literal, multiline } = res.data {
                        TomlKey::String {
                            text: text, literal: literal, multiline: multiline
                        }
                    } else {
                        unreachable!();
                    };
                    let (key, before_eq, after_eq, value) = self.read_item(res.start, key)?;
                    
                }
                other => {
                    panic!("Unexpected element: {:?}", other);
                }
            }
        }
        unimplemented!();
    }
}