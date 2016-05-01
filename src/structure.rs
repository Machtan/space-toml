extern crate chrono;

use std::fmt::Debug;
use std::collections::HashMap;
use chrono::{DateTime, UTC};
use std::str::FromStr;
use std::borrow::Cow;

/// Parses and cleans the given TOML string.
pub fn clean_string<'a>(text: &'a str, literal: bool, multiline: bool) -> Cow<'a, str> {
    if literal {
        return Cow::Borrowed(text);
    }
    let mut string = String::new();
    let mut escaped = false;
    let mut escaped_whitespace = false;
    let mut chars = text.char_indices().peekable();
    if multiline { // Ignore first newline in multiline strings
        if let Some(&(_, '\n')) = chars.peek() {
            chars.next();
        }
    }
    while let Some((i, ch)) = chars.next() {
        if escaped {
            match ch {
                ch if ch.is_whitespace() => {
                    escaped_whitespace = true;
                },
                'n' => {
                    string.push('\n');
                    escaped = false;
                }
                't' => {
                    string.push('\t');
                    escaped = false;
                }
                'b' => {
                    string.push(0x0008 as char);
                    escaped = false;
                }
                'f' => {
                    string.push(0x000C as char);
                    escaped = false;
                }
                '"' => {
                    string.push('"');
                    escaped = false;
                }
                '\\' => {
                    string.push('\\');
                    escaped = false;
                }
                'u' => {
                    for i in 0..4 {
                        chars.next();
                    }
                    escaped = false;
                }
                'U' => {
                    for i in 0..8 {
                        chars.next();
                    }
                    escaped = false;
                }
                ch if escaped_whitespace => {
                    string.push(ch);
                    escaped = false;
                }
                _ => panic!("Invalid escape character found when parsing (lexer error)"),
            }
        } else {
            if ch == '\\' {
                escaped = true;
                escaped_whitespace = false;
            } else {
                string.push(ch);
            }
        }
    }
    
    Cow::Owned(string)
}

#[derive(Debug)]
enum ScopeItem<'a> {
    Space(&'a str),
    Comment(&'a str),
    Part(usize),
}

#[derive(Debug)]
pub struct Scope<'a> {
    ordering: Vec<ScopeItem<'a>>,
    scope: Vec<TomlKey<'a>>,
}

#[derive(Debug)]
enum ArrayItem<'a> {
    Space(&'a str),
    Comment(&'a str),
    Item(usize),
}

#[derive(Debug)]
pub struct TomlArray<'a, T: TomlDecodable> {
    values: Vec<TomlItem<'a, T>>,
    order: Vec<ArrayItem<'a>>,
}

pub trait TomlDecodable: Debug {
    fn from(toml: &str) -> Self;
}

impl TomlDecodable for f64 {
    fn from(toml: &str) -> Self {
        f64::from_str(toml).expect("Invalid float value found (lexer error?)")
    }
}

impl TomlDecodable for i64 {
    fn from(toml: &str) -> Self {
        i64::from_str(toml).expect("Invalid float value found (lexer error?)")
    }
}

impl TomlDecodable for bool {
    fn from(toml: &str) -> Self {
        match toml {
            "true" => true,
            "false" => false,
            _ => panic!("Invalid bool value found (lexer error?)"),
        }
    }
}

impl TomlDecodable for String {
    fn from(toml: &str) -> Self {
        if toml.starts_with("'''") {
            return (&toml[3..toml.len()-3]).to_string();
        } else if toml.starts_with("'") {
            return (&toml[1..toml.len()-1]).to_string();
        } else if toml.starts_with(r#"""""#) {
            // TODO: Implement escaping
            return (&toml[3..toml.len()-3]).to_string();
        } else if toml.starts_with(r#"""#) {
            // TODO: Implement escaping
            return (&toml[1..toml.len()-1]).to_string();
        } else {
            panic!("Invalid toml string decoded (lexer error!)");
        }
    }
}

#[derive(Debug)]
pub enum TomlItem<'a, T: TomlDecodable> {
    Token(&'a str),
    Value(T),
    Cached(&'a str, T),
}

#[derive(Debug)]
pub enum TomlValue<'a> {
    String(TomlItem<'a, String>),
    Bool(TomlItem<'a, bool>),
    Int(TomlItem<'a, i64>),
    Float(TomlItem<'a, f64>),
    //DateTime(DateTime<UTC>),
    Table(TomlTable<'a>),
    StringArray(TomlArray<'a, String>),
    BoolArray(TomlArray<'a, bool>),
    IntArray(TomlArray<'a, i64>),
    FloatArray(TomlArray<'a, f64>),
    //DateTimeArray(TomlArray<DateTime<UTC>>),
    //TableArray(Vec<TomlTable>),
}

#[derive(Debug)]
enum TableItem<'a> {
    Space(&'a str),
    Comment(&'a str),
    Entry(ValueEntry<'a>),
    SubTable(Scope<'a>, TomlKey<'a>), // Only used in the top-level table
    SubTableArray(Scope<'a>, TomlKey<'a>), // Only used in the top-level table
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum TomlKey<'a> {
    Plain(&'a str),
    String { text: &'a str, literal: bool, multiline: bool },
}
impl<'a> TomlKey<'a> {
    pub fn show(&self) {
        use self::TomlKey::*;
        match *self {
            Plain(text) => println!("{}", text),
            String { text, literal, multiline } => {
                let clean = clean_string(text, literal, multiline);
                match (literal, multiline) {
                    (true, true) => println!("'''{}'''", clean),
                    (true, false) => println!("'{}'", clean),
                    (false, true) => println!(r#""""{}""""#, clean),
                    (false, false) => println!(r#""{}""#, clean)
                }
            }
        }
    }
}

#[derive(Debug)]
struct ValueEntry<'a> {
    key: TomlItem<'a, String>,
    before_eq: &'a str,
    after_eq: &'a str,
    after_value: &'a str,
    comment: Option<&'a str>
}

#[derive(Debug)]
pub struct TomlTable<'a> {
    inline: bool,
    ordering: Vec<TableItem<'a>>,
    map: HashMap<TomlKey<'a>, TomlValue<'a>>,
}
impl<'a> TomlTable<'a> {
    pub fn new(inline: bool) -> TomlTable<'a> {
        TomlTable {
            inline: inline,
            ordering: Vec::new(),
            map: HashMap::new(),
        }
    }
    
    pub fn push_space(&mut self, space: &'a str) {
        self.ordering.push(TableItem::Space(space));
    }
    
    pub fn push_comment(&mut self, comment: &'a str) {
        self.ordering.push(TableItem::Comment(comment));
    }
}

#[derive(Debug)]
pub enum TomlError {
    UnexpectedCharacter(usize),
    UnclosedScope(usize),
    UnexpectedLinebreak(usize),
    EmptyScope(usize),
    InvalidKeyChar { start: usize, invalid: char, index: usize },
    MissingScopeSeparator { start: usize, missing: usize },
}