
use std::iter::{Iterator, Peekable};

use tokens::{self, TokenError, Token, Tokens};
use key::{TomlKey, TomlKeyPrivate};
use table::{TomlTable, TomlTablePrivate, CreatePathError};
use scope::Scope;
use array::{TomlArray, TomlArrayPrivate};
use value::{TomlValue, TomlValuePrivate};
use debug;

/// Parses the given text as a TOML document and returns the top-level table for the document.
pub fn parse(text: &str) -> Result<TomlTable, ParseError> {
    let mut parser = Parser::new(text);
    parser.parse()
}

/// An error found when parsing a TOML document.
#[derive(Debug)]
pub enum ParseError {
    /// The tokenizer/validator found an error in the input text.
    TokenError(TokenError),
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
impl ParseError {
    // TODO: Implement error instead
    /// Prints a nice error message.
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
            WrongValueTypeInArray { start, pos } => {
                let (line, col) = debug::get_position(text, pos);
                println!("Value of invalid type found in array at {}:{} :", line, col);
                debug::show_invalid_part(text, start, pos);
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
        let _ = err;
        ParseError::InvalidScopePath
    }
}

struct Parser<'a> {
    tokens: Peekable<Tokens<'a>>,
}
impl<'a> Parser<'a> {
    fn new(text: &'a str) -> Parser<'a> {
        Parser { tokens: tokens::tokens(text).peekable() }
    }

    fn read_scope(&mut self,
                  scope: &mut Scope<'a>,
                  array: bool,
                  start: usize)
                  -> Result<(), ParseError> {
        use tokens::Token::*;
        use self::ParseError::*;
        let mut was_key = false;
        let mut key_found = false;
        let mut closed = false;
        while let Some(res) = self.tokens.next() {
            let (pos, token) = res?;
            match token {
                Dot => {
                    if !was_key {
                        return Err(InvalidScope {
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
                        return Err(InvalidScope {
                            start: start,
                            pos: pos,
                        });
                    }
                    closed = true;
                    break;
                }
                DoubleBracketClose if array => {
                    if (!key_found) || (!was_key) {
                        return Err(InvalidScope {
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
                _ => {
                    return Err(InvalidScope {
                        start: start,
                        pos: pos,
                    });
                }
            }
        }
        if !closed {
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
                        if !was_comma {
                            self.tokens.next();
                            array.push_comma();
                            was_comma = true;
                        } else {
                            return Err(DoubleCommaInArray {
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
                            return Err(NonFinalComma { pos: pos });
                        }
                        let value = self.read_value(start)?;
                        match array.push(value) {
                            Ok(()) => {}
                            Err(_) => {
                                return Err(WrongValueTypeInArray {
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
                        return Err(MissingComma {
                            start: start,
                            pos: pos,
                        });
                    }
                }
            }
        }

        Ok(TomlValue::Array(array))
    }

    fn read_inline_table(&mut self,
                         start: usize,
                         table: &mut TomlTable<'a>)
                         -> Result<(), ParseError> {
        use self::ParseError::*;
        use tokens::Token::*;
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
                            return Err(DoubleCommaInArray {
                                start: start,
                                pos: pos,
                            });
                        }
                    }
                    (pos, Key(_)) |
                    (pos, String { .. }) => {
                        if was_comma {
                            return Err(NonFinalComma { pos: pos });
                        }
                        let key = if let Key(text) = res.1 {
                            TomlKey::Plain(text)
                        } else if let String { text, literal, multiline } = res.1 {
                            TomlKey::String {
                                text: text,
                                literal: literal,
                                multiline: multiline,
                            }
                        } else {
                            unreachable!();
                        };
                        let (key, before_eq, after_eq, value) = self.read_item(pos, key)?;
                        // TODO: Check for duplicate keys
                        table.insert_spaced(key, value, before_eq, after_eq);
                        reading_key = false;
                    }
                    (_, CurlyClose) => {
                        return Ok(());
                    }
                    (pos, _) => return Err(InvalidTableItem { pos: pos }),
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
                        return Err(MissingComma {
                            start: start,
                            pos: pos,
                        });
                    }
                }
            }
        }
        Err(UnfinishedValue { start: start })
    }

    fn read_value(&mut self, start: usize) -> Result<TomlValue<'a>, ParseError> {
        use self::ParseError::*;
        use tokens::Token::*;
        let next = self.next_or(UnfinishedValue { start: start })?;
        match next {
            (_, Int(text)) => Ok(TomlValue::new_int(text)),
            (_, Float(text)) => Ok(TomlValue::new_float(text)),
            (_, String { text, literal, multiline }) => {
                Ok(TomlValue::new_string(text, literal, multiline))
            }
            (_, Bool(value)) => Ok(TomlValue::new_bool(value)),
            (_, DateTime(text)) => Ok(TomlValue::new_datetime(text)),
            (pos, SingleBracketOpen) => Ok(self.read_array(pos)?),
            (pos, CurlyOpen) => {
                let mut table = TomlTable::new_inline();
                self.read_inline_table(pos, &mut table)?;
                Ok(TomlValue::Table(table))
            }
            (pos, _) => {
                Err(InvalidValue {
                    start: start,
                    pos: pos,
                })
            }
        }
    }

    fn read_item
        (&mut self,
         start: usize,
         key: TomlKey<'a>)
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
            return Err(MissingEquals {
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

    fn next_or(&mut self, err: ParseError) -> Result<(usize, Token<'a>), ParseError> {
        match self.tokens.next() {
            Some(val) => Ok(val?),
            None => Err(err),
        }
    }

    fn peek_or(&mut self, err: ParseError) -> Result<(usize, Token<'a>), ParseError> {
        match self.tokens.peek() {
            Some(res) => {
                match *res {
                    Err(ref e) => Err(ParseError::from(e.clone())),
                    Ok(token) => Ok(token),
                }
            }
            None => Err(err),
        }
    }

    fn read_table(&mut self, table: &mut TomlTable<'a>) -> Result<(), ParseError> {
        use tokens::Token::*;
        use self::ParseError::*;
        while self.tokens.peek().is_some() {
            match *self.tokens.peek().unwrap() {
                Err(ref e) => {
                    return Err(ParseError::from(e.clone()));
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
                (pos, Key(_)) |
                (pos, String { .. }) => {
                    let key = if let Key(text) = res.1 {
                        TomlKey::Plain(text)
                    } else if let String { text, literal, multiline } = res.1 {
                        TomlKey::String {
                            text: text,
                            literal: literal,
                            multiline: multiline,
                        }
                    } else {
                        unreachable!();
                    };
                    let (key, before_eq, after_eq, value) = self.read_item(pos, key)?;
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
        let mut top_table = TomlTable::new_regular();
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
                        let mut table = if let TomlValue::Table(ref mut table) =
                                               *top_table.find_or_insert_with(scope.path(),
                                                 || TomlValue::Table(TomlTable::new_regular()))? {
                            table
                        } else {
                            // TODO: improve this error
                            return Err(InvalidScopePath);
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
                (pos, Key(text)) => {
                    let key = TomlKey::Plain(text);
                    let (key, before_eq, after_eq, value) = self.read_item(pos, key)?;
                    top_table.insert_spaced(key, value, before_eq, after_eq);
                }
                (pos, String { text, literal, multiline }) => {
                    let key = TomlKey::String {
                        text: text,
                        literal: literal,
                        multiline: multiline,
                    };
                    let (key, before_eq, after_eq, value) = self.read_item(pos, key)?;
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
