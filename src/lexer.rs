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
    scope_depth: u64,
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

enum LexerState {
    Empty,
    ReadingWhitespace { next_index: usize },
    ReadingKey,
    ReadingInt { was_number: bool, datetime_possible: bool },
    ReadingDatetime,
    ReadingFloat,
    ReadingComment,
    ReadingFloatExponent { sign_pos: bool, was_number: bool },
    ReadingString { literal: bool, multiline: bool, escaped: bool },
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Result<Token<'a>, LexerError>;
    
    fn next(&mut self) -> Option<Self::Item> {
        use self::LexerState::*;
        use self::LexerError::*;
        use self::Token::*;
        
        if self.finished {
            return None;
        }
        let mut state = LexerState::Empty;
        'state: loop {
            match state {
                Empty => {
                    if let Some((i, ch)) = self.chars.next() {
                        match ch {
                            ' ' | '\t' => {
                                state = ReadingWhitespace { next_index: i + 1 };
                            }
                            '[' => {
                                self.start += 1;
                                self.scope_depth += 1;
                                // Only check for array of tables when in key scope
                                if let LexerScope::Key = self.scope {
                                    if self.peek_is('[') {
                                        self.chars.next(); // eat it
                                        self.start += 1;
                                        return Some(Ok(DoubleBracketOpen));
                                    } else {
                                        return Some(Ok(SingleBracketOpen));
                                    }
                                } else {
                                    return Some(Ok(SingleBracketOpen));
                                }
                            }
                            '#' => {
                                state = ReadingComment;
                            }
                            ']' => {
                                self.start += 1;
                                self.scope_depth -= 1;
                                // Only check for array of tables when in key scope
                                if let LexerScope::Key = self.scope {
                                    if self.peek_is(']') {
                                        self.chars.next(); // eat it
                                        self.start += 1;
                                        return Some(Ok(DoubleBracketClose));
                                    } else {
                                        return Some(Ok(SingleBracketClose));
                                    }
                                } else {
                                    return Some(Ok(SingleBracketClose));
                                }
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
                                if self.next_is(self.start + 1, "\"\"") {
                                    self.chars.next();
                                    self.chars.next();
                                    self.start += 2;
                                    state = ReadingString { 
                                        literal: false, multiline: true, escaped: false
                                    };
                                } else {
                                    state = ReadingString {
                                        literal: false, multiline: false, escaped: false
                                    };
                                }
                            },
                            '\'' => {
                                if self.next_is(self.start + 1, "\'\'") {
                                    self.chars.next();
                                    self.chars.next();
                                    self.start += 2;
                                    state = ReadingString { 
                                        literal: true, multiline: true, escaped: false
                                    };
                                } else {
                                    state = ReadingString {
                                        literal: true, multiline: false, escaped: false
                                    };
                                }
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
                                        match ch {
                                            't' => {
                                                if self.next_is(i, "true") {
                                                    for i in 0..3 {
                                                        self.chars.next();
                                                    }
                                                    self.start = i + 4;
                                                    return Some(Ok(Bool(true)));
                                                }
                                                self.finished = true;
                                                return Some(Err(InvalidValueCharacter {
                                                    start: self.start, pos: i
                                                }));
                                            }
                                            'f' => {
                                                if self.next_is(i, "false") {
                                                    for i in 0..4 {
                                                        self.chars.next();
                                                    }
                                                    self.start = i + 5;
                                                    return Some(Ok(Bool(false)));
                                                }
                                                self.finished = true;
                                                return Some(Err(InvalidValueCharacter {
                                                    start: self.start, pos: i
                                                }));
                                            }
                                            '-' | '+' => {
                                                state = ReadingInt { 
                                                    was_number: false, 
                                                    datetime_possible: false
                                                };
                                            }
                                            '0' ... '9' => {
                                                state = ReadingInt {
                                                    was_number: true,
                                                    datetime_possible: true
                                                };
                                            }
                                            ch => {
                                                self.finished = true;
                                                return Some(Err(InvalidValueCharacter {
                                                    start: self.start, pos: i
                                                }));
                                            }
                                        }
                                    }
                                    LexerScope::Key => {
                                        match ch {
                                            'a' ... 'z' | 'A' ... 'Z' | 
                                            '_' | '-' => {
                                                state = ReadingKey;
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
                        break;
                    }
                }
                ReadingWhitespace { next_index } => {
                    if self.peek_is(' ')  || self.peek_is('\t') {
                        self.chars.next();
                        state = ReadingWhitespace { next_index: next_index + 1 };
                    } else {
                        let part = &self.text[self.start..next_index];
                        self.start = next_index;
                        state = Empty;
                        return Some(Ok(Whitespace(part)));
                    }
                }
                ReadingKey => {
                    if let Some(&(i, ch)) = self.chars.peek() {
                        match ch {
                            'a' ... 'z' | 'A' ... 'Z' | '0' ... '9' | '_' | '-' => {
                                self.chars.next();
                            }
                            ',' | ']' | ' ' | '\t' | '\n' | '#' => {
                                let part = &self.text[self.start..i];
                                self.start = i;
                                state = Empty;
                                return Some(Ok(Key(part)));
                            }
                            ch => {
                                let part = &self.text[self.start..i];
                                self.start = i;
                                state = Empty;
                                return Some(Ok(Key(part)));
                            }
                        }
                    } else {
                        self.finished = true;
                        return Some(Ok(Key(&self.text[self.start..])));
                    }
                }
                ReadingString { literal: true , multiline, .. } => {
                    while let Some((i, ch)) = self.chars.next() {
                        if multiline && self.next_is(i, "'''") {
                            self.chars.next();
                            self.chars.next();
                            let part = &self.text[self.start .. i+3];
                            self.start = i + 3;
                            state = Empty;
                            return Some(Ok(MultilineLiteral(part)));
                        } else if ch == '\'' && (! multiline) {
                            let part = &self.text[self.start .. i+1];
                            self.start = i + 1;
                            state = Empty;
                            return Some(Ok(Literal(part)));
                        }
                    }
                    self.finished = true;
                    return Some(Err(UnclosedLiteral { start: self.start }));
                }
                ReadingString { literal: false , multiline, escaped: false } => {
                    while let Some((i, ch)) = self.chars.next() {
                        if multiline && self.next_is(i, "\"\"\"") {
                            self.chars.next();
                            self.chars.next();
                            let part = &self.text[self.start .. i+3];
                            self.start = i + 3;
                            state = Empty;
                            return Some(Ok(MultilineString(part)));
                        } else if ch == '"' && (! multiline) {
                            let part = &self.text[self.start .. i+1];
                            self.start = i + 1;
                            state = Empty;
                            return Some(Ok(String(part)));
                        } else if ch == '\\' {
                            state = ReadingString { 
                                literal: false, multiline: multiline, 
                                escaped: true
                            };
                            continue 'state;
                        }
                    }
                    self.finished = true;
                    return Some(Err(UnclosedString { start: self.start }));
                }
                ReadingString { literal: false , multiline, escaped: true } => {
                    while let Some((i, ch)) = self.chars.next() {
                        match ch {
                            ' ' | '\t' | '\n' => {}
                            'b' | 't' | 'n' | 'f' | 'r' | '"' | '\\' => {
                                state = ReadingString {
                                    literal: false, multiline: multiline,
                                    escaped: false
                                };
                                continue 'state;
                            }
                            'u' => {
                                panic!("Should validate x4 unicode");
                            } 
                            'U' => {
                                panic!("Should validate x8 unicode");
                            }
                            _ => {
                                self.finished = true;
                                return Some(Err(InvalidEscapeCharacter {
                                    start: self.start, pos: i
                                }));
                            }
                        }
                    }
                    self.finished = true;
                    return Some(Err(UnclosedString { start: self.start }));
                }
                ReadingInt { was_number, datetime_possible } => {
                    if let Some(&(i, ch)) = self.chars.peek() {
                        match ch {
                            '0' ... '9' => {
                                state = ReadingInt { 
                                    was_number: true, 
                                    datetime_possible: datetime_possible
                                };
                                self.chars.next();
                            }
                            '-' if datetime_possible => {
                                state = ReadingDatetime
                            }
                            '.' => {
                                self.chars.next();
                                state = ReadingFloat;
                            }
                            'e' | 'E' => {
                                self.chars.next();
                                state = ReadingFloatExponent {
                                    sign_pos: true, was_number: false
                                };
                            }
                            '_' if was_number => {
                                state = ReadingInt {
                                    was_number: false,
                                    datetime_possible: false,
                                };
                                self.chars.next();
                            }
                            '_' => {
                                self.finished = true;
                                return Some(Err(UnderscoreNotAfterNumber {
                                    start: self.start, pos: i
                                }));
                            }
                            ',' | ' ' | '\t' | '\n' | ']' | '#' => {
                                let part = &self.text[self.start..i];
                                self.start = i;
                                state = Empty;
                                return Some(Ok(Int(part)));
                            }
                            ch => {
                                self.finished = true;
                                return Some(Err(InvalidIntCharacter {
                                    start: self.start, pos: i
                                }));
                            }
                        }
                    }
                }
                ReadingDatetime => {
                    break;
                }
                ReadingFloat => {
                    break;
                }
                ReadingFloatExponent { sign_pos: true, was_number } => {
                    break;
                }
                ReadingFloatExponent { sign_pos: false, was_number } => {
                    break;
                }
                ReadingComment => {
                    while let Some(&(i, ch)) = self.chars.peek() {
                        if self.next_is(i, "\r\n") {
                            let part = &self.text[self.start..i];
                            self.start = i;
                            return Some(Ok(Comment(part)));
                        } else if ch == '\n' {
                            let part = &self.text[self.start..i];
                            self.start = i;
                            return Some(Ok(Comment(part)));
                        } else {
                            self.chars.next();
                        }
                    }
                    self.finished = true;
                    return Some(Ok(Comment(&self.text[self.start..])));
                }
            }
        }
        
        self.finished = true;
        None
    }
}