
use std::iter::{Iterator, Peekable};
use std::borrow::Cow;

use lexer::{Lexer, LexerError, Token};
use structure::{TomlTable, TomlArray, TomlKey, TomlValue, clean_string, Scope};
use debug;

#[derive(Debug)]
pub enum ParseError {
    LexerError(LexerError),
    InvalidScope { start: usize, pos: usize },
    UnfinishedScope { start: usize },
    UnfinishedItem { start: usize },
    UnfinishedValue { start: usize },
    InvalidValue { start: usize, pos: usize },
    MissingEquals { start: usize, pos: usize },
    DoubleCommaInArray { start: usize, pos: usize },
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
            UnfinishedValue { start } => {
                let (line, col) = debug::get_position(text, start);
                println!("Unifinished value starting at {}:{} :", line, col);
                debug::show_unclosed(text, start);
            }
            MissingEquals { start, pos } => {
                let (line, col) = debug::get_position(text, pos);
                println!("'=' expected at {}:{} :", line, col);
                debug::show_invalid_part(text, start, pos);
            }
            InvalidValue { start, pos } => {
                let (line, col) = debug::get_position(text, pos);
                println!("Invalid value found at {}:{} :", line, col);
                debug::show_invalid_part(text, start, pos);
            }
            DoubleCommaInArray { start, pos } => {
                let (line, col) = debug::get_position(text, pos);
                println!("Invalid comma in array at {}:{} :", line, col);
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

enum NextArrayState {
    Closed,
    Whitespace,
    Comment,
    Newline,
    Value,
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
        use lexer::Token::*;
        use self::ParseError::*;
        println!("scope, Array: {}", array);
        let mut was_key = false;
        let mut scope = Scope::new();
        let mut key_found = false;
        let mut closed = false;
        while let Some(res) = self.lexer.next() {
            let (pos, token) = res?;
            match token {
                Dot => {
                    if ! was_key {
                        return Err(InvalidScope { start: start, pos: pos });
                    } else {
                        scope.push_dot();
                        was_key = false;
                    }
                }
                SingleBracketClose if ! array => {
                    if ! key_found {
                        return Err(InvalidScope { start: start, pos: pos });
                    } else if ! was_key {
                        return Err(InvalidScope { start: start, pos: pos});
                    }
                    closed = true;
                    break;
                }
                DoubleBracketClose if array => {
                    if ! key_found {
                        return Err(InvalidScope { start: start, pos: pos });
                    } else if ! was_key {
                        return Err(InvalidScope { start: start, pos: pos});
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
                    return Err(InvalidScope { start: start, pos: pos });
                }
            }
        }
        if ! closed {
            return Err(UnfinishedScope { start: start });
        }
        Ok(scope)
    }
    
    fn read_array(&mut self, start: usize) -> Result<TomlValue<'a>, ParseError> {
        use self::ParseError::*;
        use lexer::Token::*;
        let mut array = TomlArray::new();
        let mut next_state = NextArrayState::Value;
        let mut is_reading_value = true;
        let mut was_comma = false;
        loop {
            if is_reading_value {
                next_state = NextArrayState::Value;
                
                match self.peek_or(UnfinishedItem { start: start })? {
                    &(_, SingleBracketClose) => next_state = NextArrayState::Closed,
                    &(pos, Comma) => {
                        if ! was_comma {
                            array.push_comma();
                            was_comma = true;
                        } else {
                            return Err(DoubleCommaInArray { start: start, pos: pos });
                        }
                    }
                    _ => unimplemented!(),
                }
                
                
                match next_state {
                    NextArrayState::Closed => {
                        self.lexer.next();
                        break;
                    }
                    NextArrayState::Whitespace => {
                        if let (_, Whitespace(text)) = self.lexer.next().unwrap()? {
                            array.push_space(text);
                        }
                    }
                    _ => unimplemented!(),
                }
            } else {
                
            }
        }
        
        Ok(TomlValue::Array(array))
    }
    
    fn read_inline_table(&mut self, start: usize) -> Result<TomlValue<'a>, ParseError> {
        unimplemented!();
    }
    
    fn read_value(&mut self, start: usize)
            -> Result<TomlValue<'a>, ParseError> {
        use self::ParseError::*;
        use lexer::Token::*;
        println!("Reading value...");
        let next = self.next_or(UnfinishedValue { start: start })?;
        match next {
            (_, Int(text)) => Ok(TomlValue::int(text)),
            (_, Float(text)) => Ok(TomlValue::float(text)),
            (_, String { text, literal, multiline }) => Ok(TomlValue::string(text, literal, multiline)),
            (_, Bool(value)) => Ok(TomlValue::bool(value)),
            (pos, SingleBracketOpen) => {
                Ok(self.read_array(pos)?)
            }
            (pos, CurlyOpen) => {
                Ok(self.read_inline_table(pos)?)
            }
            (pos, _) => Err(InvalidValue { start: start, pos: pos }),
        }        
    }
    
    fn read_item(&mut self, start: usize, key: TomlKey<'a>)
            -> Result<(TomlKey<'a>, Option<&'a str>, Option<&'a str>, TomlValue<'a>), ParseError> {
        use self::ParseError::*;
        use lexer::Token::*;
        let mut before_eq = None;
        let mut next = self.next_or(UnfinishedItem { start: start })?;
        if let Whitespace(text) = next.1 {
            before_eq = Some(text);
            next = self.next_or(UnfinishedItem { start: start })?;
        }
        
        if let Equals = next.1 {
        } else {
            return Err(MissingEquals { start: start, pos: next.0 });
        }
        
        let mut after_eq = None;
        let mut has_whitespace_after = false;
        if let &(_, Whitespace(_)) = self.peek_or(UnfinishedItem { start: start })? {
            has_whitespace_after = true;
        }
        if has_whitespace_after {
            next = self.lexer.next().unwrap()?;
            if let Whitespace(text) = next.1 {
                after_eq = Some(text);
            }
        }
        
        println!("{:?} {:?} = {:?} ", key, before_eq, after_eq);
        let value_start = self.peek_or(UnfinishedItem { start: start })?.0;
        let value = self.read_value(value_start)?;
        Ok((key, before_eq, after_eq, value))
    }
    
    fn next_or(&mut self, err: ParseError) -> Result<(usize, Token<'a>), ParseError> {
        match self.lexer.next() {
            Some(val) => {
                Ok(val?)
            },
            None => Err(err)
        }
    }
    
    fn peek_or(&mut self, err: ParseError) -> Result<&(usize, Token<'a>), ParseError> {
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
        use lexer::Token::*;
        use self::ParseError::*;
        let mut top_table = TomlTable::new(false);
        let mut cur_table = top_table;
        while let Some(res) = self.lexer.next() {
            let res = res?;
            match res {
                (_, Whitespace(text)) | (_, Newline(text)) => {
                    cur_table.push_space(text);
                }
                (pos, SingleBracketOpen) => {
                    let scope = self.read_scope(false, pos)?;
                    println!("Scope: {:?}", scope);
                }
                (pos, DoubleBracketOpen) => {
                    let scope = self.read_scope(true, pos)?;
                    println!("Scope: {:?}", scope);
                }
                (_, Comment(text)) => {
                    cur_table.push_comment(text);
                }
                (pos, Key(_)) | (pos, String { .. }) => {
                    let key = if let Key(text) = res.1 {
                        TomlKey::Plain(text)
                    } else if let String { text, literal, multiline } = res.1 {
                        TomlKey::String {
                            text: text, literal: literal, multiline: multiline
                        }
                    } else {
                        unreachable!();
                    };
                    let (key, before_eq, after_eq, value) = self.read_item(pos, key)?;
                    
                }
                other => {
                    panic!("Unexpected element: {:?}", other);
                }
            }
        }
        unimplemented!();
    }
}