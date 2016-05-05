
use std::iter::{Iterator, Peekable};
use std::borrow::Cow;

use tokens::{self, TokenError, Token, Tokens};
use structure::{TomlTable, TomlArray, TomlKey, TomlValue, clean_string, Scope, CreatePathError};
use debug;

pub fn parse<'a>(text: &'a str) -> Result<TomlTable<'a>, ParseError> {
    let mut parser = Parser::new(text);
    parser.parse()
}

#[derive(Debug)]
pub enum ParseError {
    TokenError(TokenError),
    InvalidScope { start: usize, pos: usize },
    UnfinishedScope { start: usize },
    UnfinishedItem { start: usize },
    UnfinishedValue { start: usize },
    InvalidValue { start: usize, pos: usize },
    MissingEquals { start: usize, pos: usize },
    DoubleCommaInArray { start: usize, pos: usize },
    MissingComma { start: usize, pos: usize },
    InvalidTableItem { pos: usize },
    TableDefinedTwice { pos: usize, original: usize },
    KeyDefinedTwice { pos: usize, original: usize },
    InvalidScopePath,
}
impl ParseError {
    pub fn show(&self, text: &str) {
        use self::ParseError::*;
        match *self {
            TokenError(ref err) => {
                print!("Tokens: ");
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
            MissingComma { start, pos } => {
                let (line, col) = debug::get_position(text, pos);
                println!("Expected comma in array at {}:{} :", line, col);
                debug::show_invalid_part(text, start, pos);
            }
            InvalidTableItem { pos } => {
                let (line, col) = debug::get_position(text, pos);
                println!("Invalid top_level item found at {}:{} :", line, col);
                debug::show_invalid_character(text, pos);
            }
            _ => {
                unimplemented!();
            }
        }
    }
}
impl From<TokenError> for ParseError {
    fn from(err: TokenError) -> ParseError {
        ParseError::TokenError(err)
    }
}
impl From<CreatePathError> for ParseError {
    fn from(err: CreatePathError) -> ParseError {
        ParseError::InvalidScopePath
    }
}

struct Parser<'a> {
    text: &'a str,
    tokens: Peekable<Tokens<'a>>,
}
impl<'a> Parser<'a> {
    fn new(text: &'a str) -> Parser<'a> {
        Parser {
            text: text,
            tokens: tokens::tokens(text).peekable(),
        }
    }
    
    fn read_scope(&mut self, scope: &mut Scope<'a>, array: bool, start: usize)
            -> Result<(), ParseError> {
        use tokens::Token::*;
        use self::ParseError::*;
        println!("scope, Array: {}", array);
        let mut was_key = false;
        let mut key_found = false;
        let mut closed = false;
        while let Some(res) = self.tokens.next() {
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
        Ok(())
    }
    
    fn read_array(&mut self, start: usize) -> Result<TomlValue<'a>, ParseError> {
        use self::ParseError::*;
        use tokens::Token::*;
        let mut array = TomlArray::new();
        let mut is_reading_value = true;
        let mut was_comma = false;
        loop {
            if is_reading_value {
                match self.peek_or(UnfinishedItem { start: start })? {
                    (_, SingleBracketClose) => {
                        self.tokens.next();
                        break;
                    }
                    (pos, Comma) => {
                        if ! was_comma {
                            self.tokens.next();
                            array.push_comma();
                            was_comma = true;
                        } else {
                            return Err(DoubleCommaInArray { start: start, pos: pos });
                        }
                    }
                    (_, Whitespace(text)) | (_, Newline(text)) => {
                        self.tokens.next();
                        array.push_space(text);
                    }
                    (_, Comment(text)) => {
                        self.tokens.next();
                        array.push_comment(text);
                    }
                    _ => {
                        let value = self.read_value(start)?;
                        array.push(value);
                        was_comma = false;
                        is_reading_value = false;
                    }
                }
            } else {
                match self.peek_or(UnfinishedItem { start: start })? {
                    (_, Comma) => {
                        array.push_comma();
                        self.tokens.next();
                        was_comma = true;
                        is_reading_value = true;
                    }
                    (_, SingleBracketClose) => {
                        self.tokens.next();
                        break;
                    }
                    (_, Whitespace(text)) | (_, Newline(text)) => {
                        self.tokens.next();
                        array.push_space(text);
                    }
                    (_, Comment(text)) => {
                        self.tokens.next();
                        array.push_comment(text);
                    }
                    (pos, _) => {
                        return Err(MissingComma { start: start, pos: pos });
                    }
                }
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
        use tokens::Token::*;
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
        use tokens::Token::*;
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
        if let (_, Whitespace(_)) = self.peek_or(UnfinishedItem { start: start })? {
            has_whitespace_after = true;
        }
        if has_whitespace_after {
            next = self.tokens.next().unwrap()?;
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
        match self.tokens.next() {
            Some(val) => {
                Ok(val?)
            },
            None => Err(err)
        }
    }
    
    fn peek_or(&mut self, err: ParseError) -> Result<(usize, Token<'a>), ParseError> {
        match self.tokens.peek() {
            Some(res) => {
                match res {
                    &Err(ref e) => {
                        Err(ParseError::from(e.clone()))
                    },
                    &Ok(token) => Ok(token),
                }
            }
            None => Err(err)
        }
    }
    
    fn read_table(&mut self, table: &mut TomlTable<'a>) -> Result<(), ParseError> {
        use tokens::Token::*;
        use self::ParseError::*;
        while self.tokens.peek().is_some() {
            match self.tokens.peek().unwrap() {
                &Err(ref e) => {
                    return Err(ParseError::from(e.clone())); 
                }
                &Ok((_, SingleBracketOpen)) | &Ok((_, DoubleBracketOpen)) => {
                    return Ok(());
                }
                _ => {}
            }
            let res = self.tokens.next().unwrap()?;
            match res {
                (_, Whitespace(text)) => {
                    table.push_space(text);
                }
                (_, Newline(text)) => {
                    table.push_newline(text.starts_with("\r"));
                }
                (_, Comment(text)) => {
                    table.push_comment(text);
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
                    println!("{:?} = {:?}", key, value);
                    // TODO: Check for duplicate keys
                    table.insert_spaced(key, value, before_eq, after_eq);
                }
                (pos, _) => {
                    return Err(InvalidTableItem { pos: pos });
                }
            }
        }
        Ok(())
    }
    
    fn parse(&mut self) -> Result<TomlTable<'a>, ParseError> {
        use tokens::Token::*;
        use self::ParseError::*;
        let mut top_table = TomlTable::new(false);
        while let Some(res) = self.tokens.next() {
            match res? {
                (_, Whitespace(text)) => {
                    top_table.push_space(text);
                }
                (_, Newline(text)) => {
                    top_table.push_newline(text.starts_with("\r"));
                }
                (pos, SingleBracketOpen) => {
                    let mut scope = Scope::new();
                    self.read_scope(&mut scope, false, pos)?;
                    
                    // TODO: Validate that the scope hasn't been used before
                    println!("Scope: {:?}", scope);
                    {
                        let mut table = top_table.get_or_create_table(scope.path())?;
                        self.read_table(&mut table)?;
                        println!("Table: {:?}", table);
                    }
                    top_table.push_scope(scope);
                    
                }
                (pos, DoubleBracketOpen) => {
                    let mut scope = Scope::new();
                    let scope = self.read_scope(&mut scope, true, pos)?;
                    println!("Scope: {:?}", scope);
                    //let mut table = top_table.get_or_create_table(scope.path());
                    
                    //println!("Table: {:?}", table);
                }
                (_, Comment(text)) => {
                    top_table.push_comment(text);
                }
                (pos, Key(_)) | (pos, String { .. }) => {
                    let key = if let Key(text) = res?.1 {
                        TomlKey::Plain(text)
                    } else if let String { text, literal, multiline } = res?.1 {
                        TomlKey::String {
                            text: text, literal: literal, multiline: multiline
                        }
                    } else {
                        unreachable!();
                    };
                    let (key, before_eq, after_eq, value) = self.read_item(pos, key)?;
                    println!("{:?} = {:?}", key, value);
                    top_table.insert_spaced(key, value, before_eq, after_eq);
                }
                (pos, _) => {
                    return Err(InvalidTableItem { pos: pos });
                }
            }
        }
        Ok(top_table)
    }
}