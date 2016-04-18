use std::iter::{Iterator, Peekable};
use std::str::CharIndices;

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
        self.get_position(self.start)
    }
    
    fn get_position(&self, byte_offset: usize) -> (usize, usize) {
        let text = &self.text[..byte_offset];
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
    DateTime(&'a str),
    Int(&'a str),
    Float(&'a str),
}

#[derive(Debug)]
pub enum LexerError {
    InvalidWhitespace(char, (usize, usize)),
    InvalidKeyCharacter(char, (usize, usize)),
}

enum LexerState {
    Empty,
    ReadingWhitespace { next_index: usize },
    ReadingKey,
    ReadingInt,
    ReadingIntOrDatetime,
    ReadingFloat,
    ReadingFloatExponent,
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
        loop {
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
                                    return Some(Err(InvalidWhitespace(
                                        '\r', self.get_position(i)
                                    )));
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
                                self.start += 1;
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
                                self.start += 1;
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
                                            '-' | '+' | '0' ... '9' => {
                                                state = ReadingIntOrDatetime;
                                            }
                                            ch => {
                                                unimplemented!();
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
                                                return Some(Err(InvalidKeyCharacter(
                                                    ch, self.current_position()
                                                )));
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
                    break;
                }
                ReadingString { literal: false , multiline, escaped: false } => {
                    break;
                }
                ReadingString { literal: false , multiline, escaped: true } => {
                    break;
                }
                ReadingIntOrDatetime => {
                    break;
                }
                ReadingInt => {
                    break;
                }
                ReadingFloat => {
                    break;
                }
                ReadingFloatExponent => {
                    break;
                }
            }
        }
        
        self.finished = true;
        None
    }
}