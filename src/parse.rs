
use std::iter::{Iterator, Peekable};
use std::fmt;
use std::result;
use std::error;

use lexer::{self, Token, Tokens};
use key::{Key, KeyPrivate};
use table::{Table, TablePrivate, CreatePathError};
use scope::Scope;
use array::{Array, ArrayPrivate};
use value::{Value, ValuePrivate};
use debug;

/// Parses the given text as a TOML document and returns the top-level table for the document.
pub fn parse<'a>(text: &'a str) -> Result<'a, Table> {
    Parser::new(text).parse()
}

/// The kinds of errors found when parsing TOML documents.
#[derive(Debug, Clone)]
pub enum ErrorKind<'a> {
    /// The lexer found an error in the input text.
    Lex(lexer::Error<'a>),
    /// A part of this table or table array scope is invalid.
    InvalidScope {
        /// The byte index of the scope ([)
        start: usize,
        /// The byte index of the invalid part/token
        pos: usize,
    },
    /// The scope starting here wasn't completed.
    UnfinishedScope {
        /// The byte index of the scope
        start: usize,
    },
    /// The item starting here wasn't completed.
    UnfinishedItem {
        /// The byte index of the item
        start: usize,
    },
    /// The value starting here wasn't finished.
    UnfinishedValue {
        /// The byte index of the value
        start: usize,
    },
    /// This doesn't represent a valid TOML value.
    InvalidValue {
        /// The byte index of the start of the value (an array or an inline table)
        start: usize,
        /// The byte index of the invalid TOML expression
        pos: usize,
    },
    /// An equals sign was expected after a key.
    MissingEquals {
        /// The byte index of the key
        start: usize,
        /// The byte index at which something different than an equals sign was found
        pos: usize,
    },
    /// A value is missing between two commas in an array.
    DoubleCommaInArray {
        /// The byte index of the array ([)
        start: usize,
        /// The byte index of the second comma
        pos: usize,
    },
    /// A comma is missing between two values in an array.
    MissingComma {
        /// The byte index of the array ([)
        start: usize,
        /// The byte index of the value found where a comma was expected
        pos: usize,
    },
    /// This isn't a valid item inside a table.
    InvalidTableItem {
        /// The byte index of the item
        pos: usize,
    },
    // TODO: Support this!
    /// This table was defined twice
    TableDefinedTwice {
        /// The byte index of the second definition
        pos: usize,
        /// The byte index of the original definition
        original: usize,
    },
    /// This key path was defined twice.
    KeyDefinedTwice {
        /// The byte index of the second definition
        pos: usize,
        /// The byte index of the original definition
        original: usize,
    },
    /// This path is invalid (?).
    InvalidScopePath,
    /// A comma was found before any values.
    NonFinalComma {
        /// The byte index of the comma.
        pos: usize,
    },
    /// A value type that isn't of the same type as the previous array elements was found
    /// (TOML arrays are homogenous).
    WrongValueTypeInArray {
        /// The byte index of the array ([)
        start: usize,
        /// The byte index of the invalid value
        pos: usize,
    },
}

/// An error found when parsing a TOML document.
#[derive(Debug, Clone)]
pub struct Error<'a> {
    /// What kind of error this is.
    pub kind: ErrorKind<'a>,
    /// The text that was being parsed.
    pub text: &'a str,
}

impl<'a> Error<'a> {
    fn new(text: &'a str, kind: ErrorKind<'a>) -> Error<'a> {
        Error {
            kind: kind,
            text: text,
        }
    }
}

impl<'a> fmt::Display for Error<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::ErrorKind::*;
        match self.kind {
            Lex(ref err) => {
                err.fmt(f)
            }
            InvalidScope { start, pos } => {
                let (line, col) = debug::get_position(self.text, pos);
                println!("Invalid scope found at {}:{} :", line, col);
                debug::write_invalid_part(self.text, start, pos, f)
            }
            UnfinishedScope { start } => {
                let (line, col) = debug::get_position(self.text, start);
                println!("Unifinished scope starting at {}:{} :", line, col);
                debug::write_unclosed(self.text, start, f)
            }
            UnfinishedItem { start } => {
                let (line, col) = debug::get_position(self.text, start);
                println!("No value found for key at {}:{} :", line, col);
                debug::write_unclosed(self.text, start, f)
            }
            UnfinishedValue { start } => {
                let (line, col) = debug::get_position(self.text, start);
                println!("Unifinished value starting at {}:{} :", line, col);
                debug::write_unclosed(self.text, start, f)
            }
            MissingEquals { start, pos } => {
                let (line, col) = debug::get_position(self.text, pos);
                println!("'=' expected at {}:{} :", line, col);
                debug::write_invalid_part(self.text, start, pos, f)
            }
            InvalidValue { start, pos } => {
                let (line, col) = debug::get_position(self.text, pos);
                println!("Invalid value found at {}:{} :", line, col);
                debug::write_invalid_part(self.text, start, pos, f)
            }
            DoubleCommaInArray { start, pos } => {
                let (line, col) = debug::get_position(self.text, pos);
                println!("Invalid comma in array at {}:{} :", line, col);
                debug::write_invalid_part(self.text, start, pos, f)
            }
            MissingComma { start, pos } => {
                let (line, col) = debug::get_position(self.text, pos);
                println!("Expected comma in array at {}:{} :", line, col);
                debug::write_invalid_part(self.text, start, pos, f)
            }
            InvalidTableItem { pos } => {
                let (line, col) = debug::get_position(self.text, pos);
                println!("Invalid top_level item found at {}:{} :", line, col);
                debug::write_invalid_character(self.text, pos, f)
            }
            WrongValueTypeInArray { start, pos } => {
                let (line, col) = debug::get_position(self.text, pos);
                println!("Value of invalid type found in array at {}:{} :", line, col);
                debug::write_invalid_part(self.text, start, pos, f)
            }
            _ => {
                unimplemented!();
            }
        }
    }
}

impl<'a> From<lexer::Error<'a>> for Error<'a> {
    fn from(err: lexer::Error) -> Error {
        Error::new(err.text, ErrorKind::Lex(err))
    }
}

impl<'a> error::Error for Error<'a> {
    fn description(&self) -> &str {
        use self::ErrorKind::*;
        match self.kind {
            Lex(ref err) => err.description(),
            _ => "An error found while parsing TOML"
        }
    }
}

/// The result of parsing a TOML document.
pub type Result<'a, T> = result::Result<T, Error<'a>>;

struct Parser<'a> {
    text: &'a str,
    tokens: Peekable<Tokens<'a>>,
}
impl<'a> Parser<'a> {
    fn new(text: &'a str) -> Parser<'a> {
        Parser { text: text, tokens: lexer::tokens(text).peekable() }
    }

    /// Returns an error of the given kind.
    fn err<T>(&mut self, kind: ErrorKind<'a>) -> Result<'a, T> {
        Err(Error::new(self.text, kind))
    }

    fn read_scope(&mut self,
                  scope: &mut Scope<'a>,
                  array: bool,
                  start: usize)
                  -> Result<'a, ()> {
        use lexer::Token::*;
        use self::ErrorKind::*;
        let mut was_key = false;
        let mut key_found = false;
        let mut closed = false;
        while let Some(res) = self.tokens.next() {
            let (pos, token) = res?;
            match token {
                Dot => {
                    if !was_key {
                        return self.err(InvalidScope {
                            start: start,
                            pos: pos,
                        });
                    } else {
                        scope.push_dot();
                        was_key = false;
                    }
                }
                SingleBracketClose if !array => {
                    if (!key_found) || (!was_key) {
                        return self.err(InvalidScope {
                            start: start,
                            pos: pos,
                        });
                    }
                    closed = true;
                    break;
                }
                DoubleBracketClose if array => {
                    if (!key_found) || (!was_key) {
                        return self.err(InvalidScope {
                            start: start,
                            pos: pos,
                        });
                    }
                    closed = true;
                    break;
                }
                Whitespace(text) => {
                    scope.push_space(text);
                }
                PlainKey(text) => {
                    key_found = true;
                    was_key = true;
                    scope.push_key(Key::from_key(text));
                }
                String { text, literal, multiline } => {
                    key_found = true;
                    was_key = true;
                    scope.push_key(Key::from_string(text, literal, multiline));
                }
                _ => {
                    return self.err(InvalidScope {
                        start: start,
                        pos: pos,
                    });
                }
            }
        }
        if !closed {
            return self.err(UnfinishedScope { start: start });
        }
        Ok(())
    }

    fn read_array(&mut self, start: usize) -> Result<'a, Value<'a>> {
        use self::ErrorKind::*;
        use lexer::Token::*;
        let mut array = Array::new();
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
                        if !was_comma {
                            self.tokens.next();
                            array.push_comma();
                            was_comma = true;
                        } else {
                            return self.err(DoubleCommaInArray {
                                start: start,
                                pos: pos,
                            });
                        }
                    }
                    (_, Whitespace(text)) |
                    (_, Newline(text)) => {
                        self.tokens.next();
                        array.push_space(text);
                    }
                    (_, Comment(text)) => {
                        self.tokens.next();
                        array.push_comment(text);
                    }
                    (pos, _) => {
                        if was_comma {
                            return self.err(NonFinalComma { pos: pos });
                        }
                        let value = self.read_value(start)?;
                        match array.push(value) {
                            Ok(()) => {}
                            Err(_) => {
                                return self.err(WrongValueTypeInArray {
                                    start: start,
                                    pos: pos,
                                });
                            }
                        }
                        is_reading_value = false;
                    }
                }
            } else {
                match self.peek_or(UnfinishedItem { start: start })? {
                    (_, Comma) => {
                        array.push_comma();
                        self.tokens.next();
                        is_reading_value = true;
                    }
                    (_, SingleBracketClose) => {
                        self.tokens.next();
                        break;
                    }
                    (_, Whitespace(text)) |
                    (_, Newline(text)) => {
                        self.tokens.next();
                        array.push_space(text);
                    }
                    (_, Comment(text)) => {
                        self.tokens.next();
                        array.push_comment(text);
                    }
                    (pos, _) => {
                        return self.err(MissingComma {
                            start: start,
                            pos: pos,
                        });
                    }
                }
            }
        }

        Ok(Value::Array(array))
    }

    fn read_inline_table(&mut self,
                         start: usize,
                         table: &mut Table<'a>)
                         -> Result<'a, ()> {
        use self::ErrorKind::*;
        use lexer::Token::*;
        let mut reading_key = true;
        let mut was_comma = false;
        while let Some(res) = self.tokens.next() {
            if reading_key {
                let res = res?;
                match res {
                    (_, Whitespace(text)) => {
                        table.push_space(text);
                    }
                    (pos, Comma) => {
                        if !was_comma {
                            was_comma = true;
                        } else {
                            return self.err(DoubleCommaInArray {
                                start: start,
                                pos: pos,
                            });
                        }
                    }
                    (pos, PlainKey(text)) => {
                        let key = Key::Plain(text);
                        let (key, before_eq, after_eq, value) = self.read_item(pos, key)?;
                        // TODO: Check for duplicate keys
                        table.insert_spaced(key, value, before_eq, after_eq);
                        reading_key = false;
                    }
                    (pos, String { text, literal, multiline }) => {
                        if was_comma {
                            return self.err(NonFinalComma { pos: pos });
                        }
                        let key = Key::String {
                            text: text,
                            literal: literal,
                            multiline: multiline,
                        };
                        let (key, before_eq, after_eq, value) = self.read_item(pos, key)?;
                        // TODO: Check for duplicate keys
                        table.insert_spaced(key, value, before_eq, after_eq);
                        reading_key = false;
                    }
                    (_, CurlyClose) => {
                        return Ok(());
                    }
                    (pos, _) => return self.err(InvalidTableItem { pos: pos }),
                }
            } else {
                match res? {
                    (_, Whitespace(text)) => {
                        table.push_space(text);
                    }
                    (_, Comma) => {
                        table.push_comma();
                        reading_key = true;
                    }
                    (_, CurlyClose) => {
                        return Ok(());
                    }
                    (pos, _) => {
                        return self.err(MissingComma {
                            start: start,
                            pos: pos,
                        });
                    }
                }
            }
        }
        self.err(UnfinishedValue { start: start })
    }

    fn read_value(&mut self, start: usize) -> Result<'a, Value<'a>> {
        use self::ErrorKind::*;
        use lexer::Token::*;
        let next = self.next_or(UnfinishedValue { start: start })?;
        match next {
            (_, Int(text)) => Ok(Value::new_int(text)),
            (_, Float(text)) => Ok(Value::new_float(text)),
            (_, String { text, literal, multiline }) => {
                Ok(Value::new_string(text, literal, multiline))
            }
            (_, Bool(value)) => Ok(Value::new_bool(value)),
            (_, DateTime(text)) => Ok(Value::new_datetime(text)),
            (pos, SingleBracketOpen) => Ok(self.read_array(pos)?),
            (pos, CurlyOpen) => {
                let mut table = Table::new_inline();
                self.read_inline_table(pos, &mut table)?;
                Ok(Value::Table(table))
            }
            (pos, _) => {
                self.err(InvalidValue {
                    start: start,
                    pos: pos,
                })
            }
        }
    }

    fn read_item
        (&mut self,
         start: usize,
         key: Key<'a>)
         -> Result<'a, (Key<'a>, Option<&'a str>, Option<&'a str>, Value<'a>)> {
        use self::ErrorKind::*;
        use lexer::Token::*;
        let mut before_eq = None;
        let mut next = self.next_or(UnfinishedItem { start: start })?;
        if let Whitespace(text) = next.1 {
            before_eq = Some(text);
            next = self.next_or(UnfinishedItem { start: start })?;
        }

        if let Equals = next.1 {
        } else {
            return self.err(MissingEquals {
                start: start,
                pos: next.0,
            });
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

        let value_start = self.peek_or(UnfinishedItem { start: start })?.0;
        let value = self.read_value(value_start)?;
        Ok((key, before_eq, after_eq, value))
    }

    fn next_or(&mut self, err: ErrorKind<'a>) -> Result<'a, (usize, Token<'a>)> {
        match self.tokens.next() {
            Some(val) => Ok(val?),
            None => self.err(err),
        }
    }

    fn peek_or(&mut self, err: ErrorKind<'a>) -> Result<'a, (usize, Token<'a>)> {
        if let Some(res) = self.tokens.peek() {
            return match *res {
                Err(ref e) => Err(Error::from(e.clone())),
                Ok(token) => Ok(token),
            };
        } 
        self.err(err)
    }

    fn read_table(&mut self, table: &mut Table<'a>) -> Result<'a, ()> {
        use lexer::Token::*;
        use self::ErrorKind::*;
        while self.tokens.peek().is_some() {
            match *self.tokens.peek().unwrap() {
                Err(ref e) => {
                    return Err(Error::from(e.clone()));
                }
                Ok((_, SingleBracketOpen)) |
                Ok((_, DoubleBracketOpen)) => {
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
                    table.push_newline(text.starts_with('\r'));
                }
                (_, Comment(text)) => {
                    table.push_comment(text);
                }
                (pos, PlainKey(text)) => {
                    let key = Key::Plain(text);
                    let (key, before_eq, after_eq, value) = self.read_item(pos, key)?;
                    // TODO: Check for duplicate keys
                    table.insert_spaced(key, value, before_eq, after_eq);
                }
                (pos, String { text, literal, multiline }) => {
                    let key = Key::String {
                        text: text,
                        literal: literal,
                        multiline: multiline,
                    };
                    let (key, before_eq, after_eq, value) = self.read_item(pos, key)?;
                    // TODO: Check for duplicate keys
                    table.insert_spaced(key, value, before_eq, after_eq);
                }
                (pos, _) => {
                    return self.err(InvalidTableItem { pos: pos });
                }
            }
        }
        Ok(())
    }

    fn parse(mut self) -> Result<'a, Table<'a>> {
        use lexer::Token::*;
        use self::ErrorKind::*;
        let mut top_table = Table::new_regular();
        while let Some(res) = self.tokens.next() {
            match res? {
                (_, Whitespace(text)) => {
                    top_table.push_space(text);
                }
                (_, Newline(text)) => {
                    top_table.push_newline(text.starts_with('\r'));
                }
                (pos, SingleBracketOpen) => {
                    let mut scope = Scope::new();
                    self.read_scope(&mut scope, false, pos)?;

                    // TODO: Validate that the scope hasn't been used before
                    {
                        let mut table = match top_table.find_or_insert_table(scope.path()) {
                            Err(CreatePathError::InvalidScopeTable) => {
                                return self.err(InvalidScopePath);
                            }
                            Err(CreatePathError::EmptyPath) => {
                                unreachable!();
                            }
                            Ok(table) => table
                        };
                        self.read_table(&mut table)?;
                    }
                    top_table.push_scope(scope);

                }
                (pos, DoubleBracketOpen) => {
                    let mut scope = Scope::new();
                    self.read_scope(&mut scope, true, pos)?;
                    // let mut table = top_table.find_or_create_table(scope.path());

                    // println!("Table: {:?}", table);
                }
                (_, Comment(text)) => {
                    top_table.push_comment(text);
                }
                (pos, PlainKey(text)) => {
                    let key = Key::Plain(text);
                    let (key, before_eq, after_eq, value) = self.read_item(pos, key)?;
                    top_table.insert_spaced(key, value, before_eq, after_eq);
                }
                (pos, String { text, literal, multiline }) => {
                    let key = Key::String {
                        text: text,
                        literal: literal,
                        multiline: multiline,
                    };
                    let (key, before_eq, after_eq, value) = self.read_item(pos, key)?;
                    top_table.insert_spaced(key, value, before_eq, after_eq);
                }
                (pos, _) => {
                    return self.err(InvalidTableItem { pos: pos });
                }
            }
        }
        Ok(top_table)
    }
}
