use std::collections::HashMap;
use chrono::*;
use std::fmt::Debug;

/*
NOTES:
All tables / arrays of tables will be listed in the ordering
of the top-level map.
(This means that whenever a new scope label is encountered, it is added to the 
order member of the top-level TomlTable)

Also:
Trying hungarian notation (ish)
u => updated
n => count
r => result
*/

const VALID_KEY_CHARS: phf::Set<char> = phf_set! {
   'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 
   'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z', 
   'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O',
   'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z',
   '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 
   '_', '-',
};

#[derive(Debug)]
enum OrderItem {
    Space(String),
    Comment(String),
    Entry { 
        key: Vec<String>,
        before_eq: String,
        after_eq: String,
        after_value: String,
    },
    Table { 
        scope: String,      // "[ lol. bax . b]"
        key: Vec<String>,   // ["lol", "bax", "b"]
    },
    TableArray {
        scope: String,
        key: Vec<String>,
    }
}

#[derive(Debug)]
struct ValueEntry {
    key: String,
    before_eq: String,
    after_eq: String,
    value: TomlValue,
    after_value: String,
}

#[derive(Debug)]
pub struct TomlArray<T: Debug> {
    values: Vec<T>,
    spacing: Vec<(String, String)>, // Comments can also go here
}

#[derive(Debug)]
pub enum TomlString {
    String(String),
    Literal(String),
    MultilineString(String),
    MultilineLiteral(String),
}

#[derive(Debug)]
pub enum TomlValue {
    String(TomlString),
    Bool(bool),
    Int(i64),
    Float(f64),
    DateTime(DateTime<UTC>),
    Table(TomlTable),
    StringArray(TomlArray<String>),
    BoolArray(TomlArray<bool>),
    IntArray(TomlArray<i64>),
    FloatArray(TomlArray<f64>),
    DateTimeArray(TomlArray<DateTime<UTC>>),
    TableArray(Vec<TomlTable>),
}

#[derive(Debug)]
pub enum MapKind {
    Normal,
    Inline,
}

#[derive(Debug)]
pub struct TomlTable {
    kind: MapKind,
    ordering: Vec<OrderItem>,
    map: HashMap<String, TomlValue>,
}
impl TomlTable {
    pub fn new(kind: MapKind) -> TomlTable {
        TomlTable {
            kind: kind,
            ordering: Vec::new(),
            map: HashMap::new(),
        }
    }
    
    fn push_space(&mut self, space: String) {
        self.ordering.push(OrderItem::Space(space));
    }
    
    fn push_comment(&mut self, comment: String) {
        self.ordering.push(OrderItem::Comment(comment));
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

#[derive(Debug)]
pub enum Scope {
    Table { text: String, scope: Vec<String> },
    TableArray { text: String, scope: Vec<String> },
}

fn read_array_of_tables<'a>(source: &'a str, index: usize)
        -> Result<(Vec<TomlTable>, &'a str), TomlError> {
    let mut array = Vec::new();
    Ok((array, source))
}

fn read_line<'a>(source: &'a str) -> (String, &'a str) {
    let mut peek = source.char_indices().peekable();
    while let Some((i, ch)) = peek.next() {
        match ch {
            '\r' => {
                if let Some(&(_, '\n')) = peek.peek() {
                    return ((&source[..i]).to_owned(), &source[i+2..]);
                }
            },
            '\n' => {
                return ((&source[..i]).to_owned(), &source[i+1..]);
            },
            _ => {}
        }
    }
    (source.to_owned(), "")
}

fn read_value<'a>(source: &'a str, index: usize)
        -> Result<(ValueEntry, &'a str), TomlError> {
        
    unimplemented!();
    //Ok(source)
}

fn read_table<'a>(source: &'a str, index: usize)
        -> Result<(TomlTable, &'a str), TomlError> {
    
    unimplemented!();
    //Ok(source)
}

fn read_whitespace_lines<'a>(source: &'a str) -> (String, &'a str) {
    for (i, ch) in source.char_indices() {
        if ch.is_whitespace() {
            continue;
        } else {
            return ((&source[..i]).to_owned(), &source[i..]);
        }
    }
    (source.to_owned(), "")
}

fn read_whitespace_no_lines<'a>(source: &'a str, index: usize)
        -> Result<(String, &'a str), TomlError> {
    for (i, ch) in source.char_indices() {
        if ch == '\n' {
            return Err(TomlError::UnexpectedLinebreak(index + i));
        } else if ! ch.is_whitespace() {
            return Ok(((&source[..i]).to_owned(), &source[i..]));
        }
    }
    Ok((source.to_owned(), ""))
}

fn read_key<'a>(source: &'a str, index: usize)
        -> Result<(String, &'a str), TomlError> {
    use self::TomlError::*;
    for (i, ch) in source.char_indices() {
        if ch.is_whitespace() {
            return Ok(((&source[..i]).to_owned(), &source[i..]));
        } else {
            if ! VALID_KEY_CHARS.contains(&ch) {
                return Err(InvalidKeyChar {
                    start: index,
                    invalid: ch,
                    index: index + i
                });
            }
        }
    }
    Ok((source.to_owned(), ""))
}

enum ScopeParseState {
    ReadingKey,
    ReadingSeparator,
}

fn read_scope<'a>(source: &'a str, index: usize)
        -> Result<(Scope, &'a str), TomlError> {
    use self::TomlError::*;
    use self::ScopeParseState::*;
    let second = source.chars().skip(1).take(1).next().unwrap_or(' ');
    let is_array = second == '[';
    let n_braces = if is_array {2} else {1};
    let mut n_found_braces = 0;
    let mut keys = Vec::new();
    let mut scope = "";
    let mut closed = false;
    let mut remainder = source;
    let mut u_index = source.len() - remainder.len();
    let mut state = ReadingKey;
    loop {
        let (_, rem) = read_whitespace_no_lines(remainder, u_index)?;
        remainder = rem;
        u_index = source.len() - remainder.len();
        let mut chars = remainder.chars();
        if let Some(ch) = chars.next() {
            if ch == ']' {
                if n_found_braces == n_braces {
                    return Err(UnexpectedCharacter(u_index));
                } else {
                    if n_braces == 2 {
                        if let Some(']') = chars.next() {
                            remainder = &source[u_index + 1 ..];
                            closed = true;
                            break;
                        } else {
                            return Err(UnclosedScope(index));
                        }
                    } else {
                        remainder = &source[u_index + 1 ..];
                        closed = true;
                        break;
                    }
                }
            } 
            match state {
                ReadingKey => {
                    if ch == '\'' {
                        unimplemented!();
                    } else if ch == '"' {
                        unimplemented!();
                    } else {
                        let (key, rem) = read_key(remainder, u_index)?;
                        keys.push(key);
                        remainder = rem;
                        state = ReadingSeparator;
                    }
                },
                ReadingSeparator => {
                    if ch == '.' {
                        state = ReadingKey;
                        remainder = remainder[1..];
                    } else {
                        return Err(MissingScopeSeparator {
                            start: index,
                            missing: u_index,
                        });
                    }
                }
            } 
            
        } else {
            return Err(UnclosedScope(u_index));
        }
    }
    u_index = source.len() - remainder.len();
    if ! closed {
        Err(TomlError::UnclosedScope(index))
    } else if keys.is_empty() {
        Err(TomlError::EmptyScope(index))
    } else if let ReadingKey == state {
    
    } else {
        let r_scope = if is_array {
            Scope::TableArray { text: scope.to_owned(), scope: keys }
        } else {
            Scope::Table { text: scope.to_owned(), scope: keys }
        };
        Ok((r_scope, remainder))
    }
}

fn read_toml(source: &str) -> Result<TomlTable, TomlError> {
    let mut toml = TomlTable::new(MapKind::Normal);
    let mut index = 0;
    let mut remainder = source;
    while remainder.len() > 0 {
        let (space, rem) = read_whitespace_lines(source);
        if ! space.is_empty() {
            println!("Space: '{}'", &space);
            toml.push_space(space);
        }
        index = source.len() - remainder.len();
        if let Some(ch) = remainder.chars().take(1).next() {
            match ch {
                '[' => {
                    let next = remainder.chars().skip(1).take(1).next().unwrap_or(' ');
            
                    if next == '[' {
                        let (tables, rem) = read_array_of_tables(remainder, index)?;
                        remainder = rem;
                    } else {
                        let (table, rem) = read_table(remainder, index)?;
                        remainder = rem;
                    }
                },
                '#' => {
                    let (comment, rem) = read_line(&remainder[1..]);
                    toml.push_comment(comment);
                    remainder = rem;
                },
                _ => {
                    let (value, rem) = read_value(remainder, index)?;
                    remainder = rem;
                }
            }
        }
        index = source.len() - remainder.len();
    }
    Ok(toml)
}