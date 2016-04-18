use std::iter::{Iterator, Peekable};
use std::str::CharIndices;
use std::io::Write;

type CharStream<'a> = Peekable<CharIndices<'a>>;

/// Minimal scope awareness in order to distinguish keys from values when lexing 
#[derive(Debug)]
enum LexerScope {
    Key,
    Value,
}

#[derive(Debug)]
pub struct Lexer<'a> {
    text: &'a str,
    chars: CharStream<'a>,
    start: usize,
    finished: bool,
    scope: LexerScope,
    scope_depth: i64,
}

fn get_position(text: &str, byte_offset: usize) -> (usize, usize) {
    let text = &text[..byte_offset];
    let mut line = 1;
    let mut col = 1;
    
    for ch in text.chars() {
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    (line, col)
}

impl<'a> Lexer<'a> {
    pub fn new(text: &'a str) -> Lexer<'a> {
        Lexer {
            text: text,
            chars: text.char_indices().peekable(),
            start: 0,
            finished: false,
            scope: LexerScope::Key,
            scope_depth: 0,
        }
    }
    
    pub fn current_position(&self) -> (usize, usize) {
        get_position(self.text, self.start)
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
    fn read_whitespace(&mut self) -> Result<Token<'a>, LexerError> {
        use self::Token::*;
        while let Some(&(i, ch)) = self.chars.peek() {
            match ch {
                ' ' | '\t' => {
                    self.chars.next();
                }
                ch => {
                    let part = &self.text[self.start..i];
                    self.start = i;
                    return Ok(Whitespace(part));
                }
            }
        }
        return Ok(Whitespace(&self.text[self.start..]));
    }
    
    #[inline]
    fn read_key(&mut self) -> Result<Token<'a>, LexerError> {
        use self::Token::*;
        while let Some(&(i, ch)) = self.chars.peek() {
            match ch {
                'a' ... 'z' | 'A' ... 'Z' | '0' ... '9' | '_' | '-' => {
                    self.chars.next();
                }
                ',' | ']' | ' ' | '\t' | '\n' | '#' => {
                    let part = &self.text[self.start..i];
                    self.start = i;
                    return Ok(Key(part));
                }
                ch => {
                    let part = &self.text[self.start..i];
                    self.start = i;
                    return Ok(Key(part));
                }
            }
        }
        return Ok(Key(&self.text[self.start..]));
    }
    
    #[inline]
    fn read_comment(&mut self) -> Result<Token<'a>, LexerError> {
        use self::Token::*;
        while let Some(&(i, ch)) = self.chars.peek() {
            if self.next_is(i, "\r\n") {
                let part = &self.text[self.start..i];
                self.start = i;
                return Ok(Comment(part));
            } else if ch == '\n' {
                let part = &self.text[self.start..i];
                self.start = i;
                return Ok(Comment(part));
            } else {
                self.chars.next();
            }
        }
        return Ok(Comment(&self.text[self.start..]));
    }
    
    #[inline]
    fn read_bracket(&mut self, open: bool) -> Result<Token<'a>, LexerError> {
        use self::Token::*;
        self.start += 1;
        self.scope_depth += if open { 1 } else { -1 };
        if self.scope_depth < 0 {
            self.scope_depth = 0;
        }
        // Only check for array of tables when in key scope
        let ch = if open { '[' } else { ']' };
        if let LexerScope::Key = self.scope {
            if self.peek_is(ch) {
                self.chars.next(); // eat it
                self.start += 1;
                return Ok(if open { 
                    DoubleBracketOpen
                } else { 
                    DoubleBracketClose
                });
            } else {
                return Ok(if open {
                    SingleBracketOpen
                } else {
                    SingleBracketClose
                });
            }
        } else {
            return Ok(if open {
                SingleBracketOpen
            } else {
                SingleBracketClose
            });
        }
    }
    
    #[inline]
    fn read_string(&mut self, literal: bool) -> Result<Token<'a>, LexerError> {
        use self::Token::*;
        use self::LexerError::*;
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
                    let part = &self.text[self.start .. i+3];
                    self.start = i + 3;
                    return Ok(MultilineLiteral(part));
                } else if ch == '\'' && (! multiline) {
                    let part = &self.text[self.start .. i+1];
                    self.start = i + 1;
                    return Ok(Literal(part));
                }
            }
            Err(UnclosedLiteral { start: self.start })
        } else {
            while let Some((i, ch)) = self.chars.next() {
                if ! escaped {
                    if multiline && self.next_is(i, "\"\"\"") {
                        self.chars.next();
                        self.chars.next();
                        let part = &self.text[self.start .. i+3];
                        self.start = i + 3;
                        return Ok(MultilineString(part));
                    } else if ch == '"' && (! multiline) {
                        let part = &self.text[self.start .. i+1];
                        self.start = i + 1;
                        return Ok(String(part));
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
                            panic!("Should validate x4 unicode");
                        } 
                        'U' => {
                            panic!("Should validate x8 unicode");
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
            -> Result<Token<'a>, LexerError> {
        use self::Token::*;
        use self::LexerError::*;
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
                    return self.read_float(false, false, false);
                }
                'e' | 'E' => {
                    self.chars.next();
                    return self.read_float(true, true, false);
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
                    return Ok(Int(part));
                }
                ch => {
                    return Err(InvalidIntCharacter {
                        start: self.start, pos: i
                    });
                }
            }
        }
        let part = &self.text[self.start..];
        Ok(Int(part))
    }
    
    #[inline]
    fn read_float(&mut self, mut exponent_found: bool, mut at_sign: bool, 
            mut was_number: bool)
            -> Result<Token<'a>, LexerError> {
        panic!("No float support yet!");
    }
    
    #[inline]
    fn read_value(&mut self, i: usize, ch: char) -> Result<Token<'a>, LexerError> {
        use self::Token::*;
        use self::LexerError::*;
        
        match ch {
            't' => {
                if self.next_is(i, "true") {
                    for i in 0..3 {
                        self.chars.next();
                    }
                    self.start = i + 4;
                    return Ok(Bool(true));
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
                    return Ok(Bool(false));
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

#[derive(Debug)]
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
    String(&'a str),
    MultilineString(&'a str),
    Literal(&'a str),
    MultilineLiteral(&'a str),
    DateTime(&'a str),
    Int(&'a str),
    Float(&'a str),
    Bool(bool),
}
impl<'a> Token<'a> {
    pub fn as_str(&self) -> &'a str {
        use self::Token::*;
        match *self {
            Whitespace(s) | Comment(s) | Newline(s) | Key(s)
            | String(s) | MultilineString(s) | Literal(s) | MultilineLiteral(s)
            | DateTime(s) | Int(s) | Float(s) => s,
            SingleBracketOpen => "[",
            DoubleBracketOpen => "[[",
            SingleBracketClose => "]",
            DoubleBracketClose => "]]",
            CurlyOpen => "{",
            CurlyClose => "}",
            Equals => "=",
            Comma => ",",
            Dot => ".",
            Bool(true) => "true",
            Bool(false) => "false",
        }
    }
}

fn show_unclosed(text: &str, start: usize) {
    let (line, col) = get_position(text, start);
    let line_text = text.lines().skip(line-1).next().unwrap();
    println!("{}", line_text);
    let mut pre = String::new();
    let line_len = line_text.chars().count();
    for _ in 0 .. col-1 {
        pre.push(' ');
    }
    let mut post = String::new();
    if col < line_len {
        for _ in 0 .. (line_len - col) {
            post.push('~');
        }
    }
    println!("{}^{}", pre, post);
}

fn show_invalid_character(text: &str, pos: usize) {
    let (line, col) = get_position(text, pos);
    let line_text = text.lines().skip(line-1).next().unwrap();
    println!("{}", line_text);
    let mut pre = String::new();
    let line_len = line_text.chars().count();
    for _ in 0 .. col-1 {
        pre.push(' ');
    }
    println!("{}^", pre);
}

#[derive(Debug)]
pub enum LexerError {
    InvalidWhitespace { pos: usize },
    InvalidKeyCharacter { pos: usize },
    UnclosedLiteral { start: usize },
    UnclosedString { start: usize },
    InvalidValueCharacter { start: usize, pos: usize },
    InvalidIntCharacter { start: usize, pos: usize },
    InvalidEscapeCharacter { start: usize, pos: usize },
    UnderscoreNotAfterNumber { start: usize, pos: usize },
}
impl LexerError {
    pub fn show(&self, text: &str) {
        use self::LexerError::*;
        match *self {
            UnclosedString { start } => {
                let (line, col) = get_position(text, start);
                println!("Unclosed string starting at {}:{} :", line, col);
                show_unclosed(text, start);
            }
            InvalidEscapeCharacter { pos, .. } => {
                let (line, col) = get_position(text, pos);
                println!("Invalid escape character at {}:{} :", line, col);
                show_invalid_character(text, pos);
            }
            _ => println!("Error: {:?}", *self),
        }
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Result<Token<'a>, LexerError>;
    
    fn next(&mut self) -> Option<Self::Item> {
        use self::LexerError::*;
        use self::Token::*;
        
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
                    return Some(Ok(CurlyOpen));
                }
                '}' => {
                    self.start += 1;
                    return Some(Ok(CurlyClose));
                }
                '\r' => {
                    self.start += 1;
                    if self.peek_is('\n') {
                        self.chars.next();
                        let part = &self.text[self.start..self.start+2];
                        self.start += 1;
                        // New line, new key
                        if self.scope_depth == 0 {
                            self.scope = LexerScope::Key; 
                        }
                        return Some(Ok(Newline(part)));
                    } else {
                        self.finished = true;
                        return Some(Err(InvalidWhitespace { pos: i }));
                    }
                }
                '\n' => {
                    self.start += 1;
                    // New line, new key
                    if self.scope_depth == 0 {
                        self.scope = LexerScope::Key; 
                    }
                    return Some(Ok(Newline("\n")));
                }
                '=' => {
                    self.start += 1;
                    self.scope = LexerScope::Value;
                    return Some(Ok(Equals));
                }
                '"' => {
                    return Some(self.read_string(false));
                },
                '\'' => {
                    return Some(self.read_string(true));
                }
                ',' => {
                    self.start += 1;
                    return Some(Ok(Comma));
                }
                '.' => {
                    self.start += 1;
                    return Some(Ok(Dot));
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