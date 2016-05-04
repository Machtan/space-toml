use std::iter::{Iterator, Peekable};
use std::str::CharIndices;
use std::io::Write;
use debug;

type CharStream<'a> = Peekable<CharIndices<'a>>;

/// Minimal scope awareness in order to distinguish keys from values when lexing 
#[derive(Debug)]
enum LexerScope {
    Key,
    Value,
}

pub fn tokens<'a>(text: &'a str) -> Tokens<'a> {
    Tokens::new(text)
}

#[derive(Debug)]
pub struct Tokens<'a> {
    text: &'a str,
    chars: CharStream<'a>,
    start: usize,
    finished: bool,
    scope: LexerScope,
    scope_stack: Vec<char>,
}

impl<'a> Tokens<'a> {
    fn new(text: &'a str) -> Tokens<'a> {
        Tokens {
            text: text,
            chars: text.char_indices().peekable(),
            start: 0,
            finished: false,
            scope: LexerScope::Key,
            scope_stack: Vec::new(),
        }
    }
    
    fn current_position(&self) -> (usize, usize) {
        debug::get_position(self.text, self.start)
    }
    
    #[inline]
    fn next_is(&self, start: usize, pat: &str) -> bool {
        (&self.text[start..]).starts_with(pat)
    }
    
    #[inline]
    fn peek_is(&mut self, ch: char) -> bool {
        if let Some(&(_, c)) = self.chars.peek() {
            c == ch
        } else {
            false
        }
    }
    
    #[inline]
    fn scope_is_table(&self) -> bool {
        if self.scope_stack.is_empty() {
            return false;
        } else {
            let last = self.scope_stack.len() - 1;
            if self.scope_stack[last] == '{' {
                true
            } else {
                false
            }
        }
    }
    
    #[inline]
    fn read_whitespace(&mut self) -> Result<(usize, Token<'a>), TokenError> {
        use self::Token::*;
        let start = self.start;
        while let Some(&(i, ch)) = self.chars.peek() {
            match ch {
                ' ' | '\t' => {
                    self.chars.next();
                }
                ch => {
                    let part = &self.text[self.start..i];
                    self.start = i;
                    return Ok((start, Whitespace(part)));
                }
            }
        }
        return Ok((start, Whitespace(&self.text[self.start..])));
    }
    
    #[inline]
    fn read_key(&mut self) -> Result<(usize, Token<'a>), TokenError> {
        use self::Token::*;
        let start = self.start;
        while let Some(&(i, ch)) = self.chars.peek() {
            match ch {
                'a' ... 'z' | 'A' ... 'Z' | '0' ... '9' | '_' | '-' => {
                    self.chars.next();
                }
                ',' | ']' | ' ' | '\t' | '\n' | '#' => {
                    let part = &self.text[self.start..i];
                    self.start = i;
                    return Ok((start, Key(part)));
                }
                ch => {
                    let part = &self.text[self.start..i];
                    self.start = i;
                    return Ok((start, Key(part)));
                }
            }
        }
        return Ok((start, Key(&self.text[self.start..])));
    }
    
    #[inline]
    fn read_comment(&mut self) -> Result<(usize, Token<'a>), TokenError> {
        use self::Token::*;
        let start = self.start;
        while let Some(&(i, ch)) = self.chars.peek() {
            if self.next_is(i, "\r\n") {
                let part = &self.text[self.start..i];
                self.start = i;
                return Ok((start, Comment(part)));
            } else if ch == '\n' {
                let part = &self.text[self.start..i];
                self.start = i;
                return Ok((start, Comment(part)));
            } else {
                self.chars.next();
            }
        }
        return Ok((start, Comment(&self.text[self.start..])));
    }
    
    #[inline]
    fn read_bracket(&mut self, open: bool) -> Result<(usize, Token<'a>), TokenError> {
        use self::Token::*;
        use self::TokenError::*;
        let start = self.start;
        self.start += 1;
        // Only check for array of tables when in key scope
        let ch = if open { '[' } else { ']' };
        if let LexerScope::Key = self.scope {
            if self.peek_is(ch) {
                self.chars.next(); // eat it
                self.start += 1;
                if open { 
                    return Ok((start, DoubleBracketOpen));
                } else { 
                    return Ok((start, DoubleBracketClose));
                }
            } else {
                if open {
                    //print!("Open: stack: {:?} -> ", self.scope_stack);
                    self.scope_stack.push('[');
                    //println!("{:?}", self.scope_stack);
                    return Ok((start, SingleBracketOpen));
                } else {
                    //print!("Close: stack: {:?} -> ", self.scope_stack);
                    if self.scope_stack.is_empty() {
                        //println!("Error!");
                        self.finished = true;
                        return Err(UnmatchedClosingBrace { pos: self.start-1 });
                    } else {
                        self.scope_stack.pop();
                        //println!("{:?}", self.scope_stack);
                    }
                    return Ok((start, SingleBracketClose));
                }
            }
        } else {
            if open {
                self.scope_stack.push('[');
                return Ok((start, SingleBracketOpen));
            } else {
                if self.scope_stack.is_empty() {
                    //println!("Error!");
                    self.finished = true;
                    return Err(UnmatchedClosingBrace { pos: self.start-1 });
                } else {
                    self.scope_stack.pop();
                    //println!("{:?}", self.scope_stack);
                }
                return Ok((start, SingleBracketClose));
            }
        }
    }
    
    #[inline]
    fn read_string(&mut self, literal: bool) -> Result<(usize, Token<'a>), TokenError> {
        use self::Token::*;
        use self::TokenError::*;
        let start = self.start;
        let mut escaped = false;
        let multiline = if ! literal {
            if self.next_is(self.start + 1, "\"\"") {
                self.chars.next();
                self.chars.next();
                true
            } else {
                false
            }
        } else {
            if self.next_is(self.start + 1, "''") {
                self.chars.next();
                self.chars.next();
                true
            } else {
                false
            }
        };
        if literal {
            while let Some((i, ch)) = self.chars.next() {
                if multiline && self.next_is(i, "'''") {
                    self.chars.next();
                    self.chars.next();
                    let part = &self.text[self.start+3 .. i]; // Remove apostrophes
                    self.start = i + 3;
                    return Ok((start, String { 
                        text: part, literal: true, multiline: true
                    }));
                } else if ch == '\'' && (! multiline) {
                    let part = &self.text[self.start+1 .. i];
                    self.start = i + 1;
                    return Ok((start, String { 
                        text: part, literal: true, multiline: false
                    }));
                }
            }
            Err(UnclosedLiteral { start: self.start })
        } else {
            while let Some((i, ch)) = self.chars.next() {
                if ! escaped {
                    if multiline && self.next_is(i, "\"\"\"") {
                        self.chars.next();
                        self.chars.next();
                        let part = &self.text[self.start+3 .. i];
                        self.start = i + 3;
                        return Ok((start, String { 
                            text: part, literal: false, multiline: true
                        }));
                    } else if ch == '"' && (! multiline) {
                        let part = &self.text[self.start+1 .. i];
                        self.start = i + 1;
                        return Ok((start, String { 
                            text: part, literal: false, multiline: false
                        }));
                    } else if ch == '\\' {
                        escaped = true;
                    }
                } else {
                    match ch {
                        ' ' | '\t' | '\n' => {}
                        'b' | 't' | 'n' | 'f' | 'r' | '"' | '\\' => {
                            escaped = false;
                        }
                        'u' => {
                            println!("Should validate x4 unicode");
                            for i in 0..4 {
                                self.chars.next();
                            }
                        } 
                        'U' => {
                            println!("Should validate x8 unicode");
                            for i in 0..8 {
                                self.chars.next();
                            }
                        }
                        _ => {
                            return Err(InvalidEscapeCharacter {
                                start: self.start, pos: i
                            });
                        }
                    }
                }
            }
            Err(UnclosedString { start: self.start })
        }
    }
    
    #[inline]
    fn read_int(&mut self, mut was_number: bool, mut datetime_possible: bool)
            -> Result<(usize, Token<'a>), TokenError> {
        use self::Token::*;
        use self::TokenError::*;
        let start = self.start;
        while let Some(&(i, ch)) = self.chars.peek() {
            match ch {
                '0' ... '9' => {
                    was_number = true;
                    self.chars.next();
                }
                '-' if datetime_possible => {
                    panic!("No datetime support yet!");
                }
                '.' => {
                    self.chars.next();
                    return self.read_float(false, false);
                }
                'e' | 'E' => {
                    self.chars.next();
                    if self.peek_is('-') || self.peek_is('+') {
                        self.chars.next();
                    }
                    return self.read_float(true, false);
                }
                '_' if was_number => {
                    self.chars.next();
                    was_number = false;
                    datetime_possible = false;
                }
                '_' => {
                    self.finished = true;
                    return Err(UnderscoreNotAfterNumber {
                        start: self.start, pos: i
                    });
                }
                ',' | ' ' | '\t' | '\n' | ']' | '#' => {
                    let part = &self.text[self.start..i];
                    self.start = i;
                    return Ok((start, Int(part)));
                }
                ch => {
                    return Err(InvalidIntCharacter {
                        start: self.start, pos: i
                    });
                }
            }
        }
        let part = &self.text[self.start..];
        Ok((start, Int(part)))
    }
    
    #[inline]
    fn read_float(&mut self, mut exponent_found: bool, mut was_number: bool)
            -> Result<(usize, Token<'a>), TokenError> {
        use self::Token::*;
        use self::TokenError::*;
        let start = self.start;
        
        while let Some(&(i, ch)) = self.chars.peek() {
            match ch {
                'e' | 'E' if exponent_found => {
                    return Err(InvalidFloatCharacter { start: self.start, pos: i });
                }
                'e' | 'E' => {
                    self.chars.next();
                    if self.peek_is('-') || self.peek_is('+') {
                        self.chars.next();
                    }
                    exponent_found = true;
                }
                '0' ... '9' => {
                    self.chars.next();
                    was_number = true;
                }
                '_' if was_number => {
                    self.chars.next();
                    was_number = false;
                }
                '_' => {
                    self.finished = true;
                    return Err(UnderscoreNotAfterNumber {
                        start: self.start, pos: i
                    });
                }
                ',' | ' ' | '\t' | '\n' | ']' | '#' => {
                    let part = &self.text[self.start..i];
                    self.start = i;
                    return Ok((start, Float(part)));
                }
                ch => {
                    return Err(InvalidFloatCharacter {
                        start: self.start, pos: i
                    });
                }
            }
        }
        let part = &self.text[self.start..];
        Ok((start, Float(part)))
    }
    
    #[inline]
    fn read_value(&mut self, i: usize, ch: char) -> Result<(usize, Token<'a>), TokenError> {
        use self::Token::*;
        use self::TokenError::*;
        let start = self.start;
        
        match ch {
            't' => {
                if self.next_is(i, "true") {
                    for i in 0..3 {
                        self.chars.next();
                    }
                    self.start = i + 4;
                    return Ok((start, Bool(true)));
                }
                self.finished = true;
                Err(InvalidValueCharacter {
                    start: self.start, pos: i
                })
            }
            'f' => {
                if self.next_is(i, "false") {
                    for i in 0..4 {
                        self.chars.next();
                    }
                    self.start = i + 5;
                    return Ok((start, Bool(false)));
                }
                self.finished = true;
                Err(InvalidValueCharacter {
                    start: self.start, pos: i
                })
            }
            '-' | '+' => {
                self.read_int(false, false)
            }
            '0' ... '9' => {
                self.read_int(true, true)
            }
            ch => {
                self.finished = true;
                Err(InvalidValueCharacter {
                    start: self.start, pos: i
                })
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Token<'a> {
    Whitespace(&'a str),
    SingleBracketOpen,
    DoubleBracketOpen,
    SingleBracketClose,
    DoubleBracketClose,
    CurlyOpen,
    CurlyClose,
    Comment(&'a str),
    Equals,
    Comma,
    Dot,
    Newline(&'a str),
    Key(&'a str), // Unquoted key
    String { text: &'a str, literal: bool, multiline: bool },
    DateTime(&'a str),
    Int(&'a str),
    Float(&'a str),
    Bool(bool),
}

impl<'a> Token<'a> {    
    pub fn write(&self, out: &mut String) {
        use self::Token::*;
        match *self {
            Whitespace(s) | Comment(s) | Newline(s) | Key(s)
            | DateTime(s) | Int(s) | Float(s) => out.push_str(s),
            SingleBracketOpen => out.push_str("["),
            DoubleBracketOpen => out.push_str("[["),
            SingleBracketClose => out.push_str("]"),
            DoubleBracketClose => out.push_str("]]"),
            CurlyOpen => out.push_str("{"),
            CurlyClose => out.push_str("}"),
            Equals => out.push_str("="),
            Comma => out.push_str(","),
            Dot => out.push_str("."),
            Bool(true) => out.push_str("true"),
            Bool(false) => out.push_str("false"),
            String { text, literal, multiline } => {
                out.push_str(match (literal, multiline) {
                    (true, true) => "'''",
                    (true, false) => "'",
                    (false, true) => r#"""""#,
                    (false, false) => r#"""#,
                });
                out.push_str(text);
                out.push_str(match (literal, multiline) {
                    (true, true) => "'''",
                    (true, false) => "'",
                    (false, true) => r#"""""#,
                    (false, false) => r#"""#,
                });
            }
        }
    }
}



#[derive(Debug, Clone)]
pub enum TokenError {
    InvalidWhitespace { pos: usize },
    UnclosedLiteral { start: usize },
    UnclosedString { start: usize },
    UnmatchedClosingBrace { pos: usize },
    InvalidKeyCharacter { pos: usize },
    InvalidValueCharacter { start: usize, pos: usize },
    InvalidIntCharacter { start: usize, pos: usize },
    InvalidEscapeCharacter { start: usize, pos: usize },
    InvalidFloatCharacter { start: usize, pos: usize },
    UnderscoreNotAfterNumber { start: usize, pos: usize },
}
impl TokenError {
    pub fn show(&self, text: &str) {
        use self::TokenError::*;
        match *self {
            UnclosedString { start } => {
                let (line, col) = debug::get_position(text, start);
                println!("Unclosed string starting at {}:{} :", line, col);
                debug::show_unclosed(text, start);
            }
            UnclosedLiteral { start } => {
                let (line, col) = debug::get_position(text, start);
                println!("Unclosed string starting at {}:{} :", line, col);
                debug::show_unclosed(text, start);
            }
            InvalidEscapeCharacter { pos, .. } => {
                let (line, col) = debug::get_position(text, pos);
                println!("Invalid escape character at {}:{} :", line, col);
                debug::show_invalid_character(text, pos);
            }
            InvalidValueCharacter { start, pos } => {
                let (line, col) = debug::get_position(text, pos);
                println!("Invalid character in value at {}:{} :", line, col);
                debug::show_invalid_part(text, start, pos);
            }
            InvalidIntCharacter { start, pos } => {
                let (line, col) = debug::get_position(text, pos);
                println!("Invalid character in integer at {}:{} :", line, col);
                debug::show_invalid_part(text, start, pos);
            }
            InvalidFloatCharacter { start, pos } => {
                let (line, col) = debug::get_position(text, pos);
                println!("Invalid character in float at {}:{} :", line, col);
                debug::show_invalid_part(text, start, pos);
            }
            UnmatchedClosingBrace { pos } => {
                let (line, col) = debug::get_position(text, pos);
                println!("Unmatched brace found at {}:{} :", line, col);
                debug::show_invalid_character(text, pos);
            }
            InvalidKeyCharacter { pos } => {
                let (line, col) = debug::get_position(text, pos);
                println!("Invalid key character at {}:{} :", line, col);
                debug::show_invalid_character(text, pos);
            }
            _ => println!("Error: {:?}", *self),
        }
    }
}

impl<'a> Iterator for Tokens<'a> {
    type Item = Result<(usize, Token<'a>), TokenError>;
    
    fn next(&mut self) -> Option<Self::Item> {
        use self::TokenError::*;
        use self::Token::*;
        let start = self.start;
        
        if self.finished {
            return None;
        }
        
        if let Some((i, ch)) = self.chars.next() {
            match ch {
                ' ' | '\t' => {
                    return Some(self.read_whitespace());
                }
                '[' => {
                    return Some(self.read_bracket(true));
                }
                '#' => {
                    return Some(self.read_comment());
                }
                ']' => {
                    return Some(self.read_bracket(false));
                }
                '{' => {
                    self.start += 1;
                    self.scope_stack.push('{');
                    self.scope = LexerScope::Key;
                    return Some(Ok((start, CurlyOpen)));
                }
                '}' => {
                    if self.scope_stack.is_empty() {
                        self.finished = true;
                        return Some(Err(UnmatchedClosingBrace {
                            pos: self.start-1
                        }));
                    } else {
                        self.scope_stack.pop();
                    }
                    self.start += 1;
                    return Some(Ok((start, CurlyClose)));
                }
                '\r' => {
                    self.start += 1;
                    if self.peek_is('\n') {
                        self.chars.next();
                        let part = &self.text[self.start..self.start+2];
                        self.start += 1;
                        // New line, new key
                        if self.scope_stack.is_empty() {
                            self.scope = LexerScope::Key; 
                        }
                        return Some(Ok((start, Newline(part))));
                    } else {
                        self.finished = true;
                        return Some(Err(InvalidWhitespace { pos: i }));
                    }
                }
                '\n' => {
                    self.start += 1;
                    // New line, new key
                    if self.scope_stack.is_empty() {
                        self.scope = LexerScope::Key; 
                    }
                    return Some(Ok((start, Newline("\n"))));
                }
                '=' => {
                    self.start += 1;
                    self.scope = LexerScope::Value;
                    return Some(Ok((start, Equals)));
                }
                '"' => {
                    return Some(self.read_string(false));
                },
                '\'' => {
                    return Some(self.read_string(true));
                }
                ',' => {
                    if self.scope_is_table() {
                        self.scope = LexerScope::Key;
                    }
                    self.start += 1;
                    return Some(Ok((start, Comma)));
                }
                '.' => {
                    self.start += 1;
                    return Some(Ok((start, Dot)));
                }
                ch => {
                    match self.scope {
                        LexerScope::Value => {
                            return Some(self.read_value(i, ch));
                            
                        }
                        LexerScope::Key => {
                            match ch {
                                'a' ... 'z' | 'A' ... 'Z' | 
                                '_' | '-' => {
                                    return Some(self.read_key())
                                }
                                ch => {
                                    return Some(Err(InvalidKeyCharacter {
                                        pos: i
                                    }));
                                }
                            }
                        }
                    }
                }
            }
        } else {
            // Remember to finish when all characters are exhausted
            self.finished = true;
            return None;
        }
    }
}