use std::fmt::Debug;
use std::collections::HashMap;
use std::str::FromStr;
use std::borrow::Cow;
use std::char;

#[derive(Debug, Clone)]
pub struct TomlString<'a> { 
    text: &'a str, 
    literal: bool,
    multiline: bool
}
impl<'a> TomlString<'a> {
    pub fn new(text: &'a str, literal: bool, multiline: bool) -> TomlString<'a> {
        TomlString {
            text: text,
            literal: literal,
            multiline: multiline,
        }
    }
}

#[derive(Debug)]
pub enum TomlFloat<'a> {
    Text(&'a str),
    Value(f64),
}

#[derive(Debug)]
pub enum TomlInt<'a> {
    Text(&'a str),
    Value(i64),
}


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
                    string.push(char::from_u32(0x0008u32).unwrap());
                    escaped = false;
                }
                'f' => {
                    string.push(char::from_u32(0x000Cu32).unwrap());
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

#[derive(Debug, Clone)]
enum ScopeItem<'a> {
    Dot,
    Space(&'a str),
    Part(usize),
}

#[derive(Debug, Clone)]
pub struct Scope<'a> {
    ordering: Vec<ScopeItem<'a>>,
    keys: Vec<TomlKey<'a>>,
}

impl<'a> Scope<'a> {
    pub fn new() -> Scope<'a> {
        Scope { ordering: Vec::new(), keys: Vec::new() }
    }
    
    pub fn push_dot(&mut self) {
        self.ordering.push(ScopeItem::Dot);
    }
    
    pub fn push_space(&mut self, text: &'a str) {
        self.ordering.push(ScopeItem::Space(text));
    }
    
    pub fn push_key(&mut self, key: TomlKey<'a>) {
        let new_index = self.keys.len();
        self.keys.push(key);
        self.ordering.push(ScopeItem::Part(new_index));
    }
    
    pub fn path(&self) -> &[TomlKey<'a>] {
        &self.keys
    }
    
    pub fn write(&self, out: &mut String) {
        use self::ScopeItem::*;
        out.push('[');
        for item in self.ordering.iter() {
            match *item {
                Dot => out.push('.'),
                Space(text) => out.push_str(text),
                Part(index) => {
                    self.keys[index].write(out);
                }
            }
        }
        out.push(']');
    }
}

#[derive(Debug)]
enum ArrayItem<'a> {
    Space(&'a str),
    Comment(&'a str),
    Item(usize),
    Comma,
}

#[derive(Debug)]
pub struct TomlArray<'a> {
    items: Vec<TomlValue<'a>>,
    order: Vec<ArrayItem<'a>>,
}

impl<'a> TomlArray<'a> {
    pub fn new() -> TomlArray<'a> {
        TomlArray {
            items: Vec::new(),
            order: Vec::new(),
        }
    }
    
    pub fn push(&mut self, value: TomlValue<'a>) -> Result<(), String> {
        if let Some(first) = self.items.get(0) {
            if ! first.is_same_type(&value) {
                return Err(format!("Attempted to insert a value of type {:?} into an array of type {:?}", value, first));
            }
        }
        self.order.push(ArrayItem::Item(self.items.len()));
        self.items.push(value);
        Ok(())
    }
    
    pub fn push_space(&mut self, space: &'a str) {
        self.order.push(ArrayItem::Space(space));
    }
    
    pub fn push_comma(&mut self) {
        self.order.push(ArrayItem::Comma);
    }
    
    /// This also pushes a newline.
    pub fn push_comment(&mut self, comment: &'a str) {
        self.order.push(ArrayItem::Comment(comment));
        self.order.push(ArrayItem::Space("\n"));
    }
    
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
    
    pub fn write(&self, out: &mut String) {
        use self::ArrayItem::*;
        out.push('[');
        for item in self.order.iter() {
            match *item {
                Space(text) => out.push_str(text),
                Comment(text) => {
                    out.push('#');
                    out.push_str(text);
                }
                Item(index) => {
                    self.items[index].write(out);
                }
                Comma => out.push(','),
            }
        }
        out.push(']');
    }
}

#[derive(Debug)]
pub enum TomlValue<'a> {
    String(TomlString<'a>),
    Bool(bool),
    Int(TomlInt<'a>),
    Float(TomlFloat<'a>),
    //DateTime(DateTime<UTC>),
    Table(TomlTable<'a>),
    Array(TomlArray<'a>)
    //DateTimeArray(TomlArray<DateTime<UTC>>),
    //TableArray(Vec<TomlTable>),
}

fn write_string(text: &str, literal: bool, multiline: bool, out: &mut String) {
    match (literal, multiline) {
        (true, true) => out.push_str("'''"),
        (true, false) => out.push_str("'"),
        (false, true) => out.push_str(r#"""""#),
        (false, false) => out.push_str(r#"""#),
    }
    out.push_str(text);
    match (literal, multiline) {
        (true, true) => out.push_str("'''"),
        (true, false) => out.push_str("'"),
        (false, true) => out.push_str(r#"""""#),
        (false, false) => out.push_str(r#"""#),
    }
}

impl<'a> TomlValue<'a> {
    /// Creates a new integer
    pub fn int(text: &'a str) -> TomlValue<'a> {
        TomlValue::Int(TomlInt::Text(text))
    }
    
    pub fn bool(value: bool) -> TomlValue<'a> {
        TomlValue::Bool(value)
    }
    
    pub fn string(text: &'a str, literal: bool, multiline: bool) -> TomlValue<'a> {
        TomlValue::String(TomlString::new(text, literal, multiline))
    }
    
    pub fn float(text: &'a str) -> TomlValue<'a> {
        TomlValue::Float(TomlFloat::Text(text))
    }
    
    pub fn is_same_type(&self, other: &TomlValue) -> bool {
        use self::TomlValue::*;
        match (self, other) {
            (&String(_), &String(_)) => true,
            (&Bool(_), &Bool(_)) => true,
            (&Int(_), &Int(_)) => true,
            (&Float(_), &Float(_)) => true,
            (&Table(_), &Table(_)) => true,
            (&Array(_), &Array(_)) => true,
            _ => false
        }
    }
    
    pub fn write(&self, out: &mut String) {
        use self::TomlValue::*;
        match *self {
            String(ref string) => {
                write_string(string.text, string.literal, string.multiline, out);
            },
            Bool(b) => out.push_str(if b {"true"} else {"false"}),
            Int(TomlInt::Text(text)) => out.push_str(text),
            Int(TomlInt::Value(v)) => out.push_str(&format!("{}", v)),
            Float(TomlFloat::Text(text)) => out.push_str(text),
            Float(TomlFloat::Value(v)) => out.push_str(&format!("{}", v)),
            Table(ref table) => table.write(out),
            Array(ref array) => array.write(out),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum TomlKey<'a> {
    Plain(&'a str),
    String { text: &'a str, literal: bool, multiline: bool },
}
impl<'a> TomlKey<'a> {
    pub fn from_key(key: &'a str) -> TomlKey<'a> {
        TomlKey::Plain(key)
    }
    
    pub fn from_string(text: &'a str, literal: bool, multiline: bool) -> TomlKey<'a> {
        TomlKey::String { text: text, literal: literal, multiline: multiline }
    }
    
    pub fn write(&self, out: &mut String) {
        use self::TomlKey::*;
        match *self {
            Plain(text) => out.push_str(text),
            String { text, literal, multiline } => {
                write_string(text, literal, multiline, out);
            }
        }
    }
}

#[derive(Debug)]
enum TableItem<'a> {
    Space(&'a str),
    Comment(&'a str),
    Entry(ValueEntry<'a>),
    /// For inline tables
    Comma, 
    Scope(Scope<'a>), // Only used in the top-level table
}

#[derive(Debug)]
struct ValueEntry<'a> {
    key: TomlKey<'a>,
    before_eq: &'a str,
    after_eq: &'a str,
}

impl<'a> ValueEntry<'a> {
    pub fn write(&self, out: &mut String) {
        self.key.write(out);
        out.push_str(self.before_eq);
        out.push('=');
        out.push_str(self.after_eq);
    }
}

#[derive(Debug)]
pub enum CreatePathError {
    InvalidScopeTable
}

#[derive(Debug)]
pub struct TomlTable<'a> {
    inline: bool,
    order: Vec<TableItem<'a>>,
    items: HashMap<TomlKey<'a>, TomlValue<'a>>,
}
impl<'a> TomlTable<'a> {
    pub fn new(inline: bool) -> TomlTable<'a> {
        TomlTable {
            inline: inline,
            order: Vec::new(),
            items: HashMap::new(),
        }
    }
    
    pub fn push_space(&mut self, space: &'a str) {
        self.order.push(TableItem::Space(space));
    }
    
    pub fn push_comment(&mut self, comment: &'a str) {
        self.order.push(TableItem::Comment(comment));
    }
    
    pub fn push_scope(&mut self, scope: Scope<'a>) {
        self.order.push(TableItem::Scope(scope));
    }
    
    pub fn insert_spaced(&mut self, key: TomlKey<'a>, value: TomlValue<'a>, 
            before_eq: Option<&'a str>, after_eq: Option<&'a str>) {
        let entry = ValueEntry { 
            key: key.clone(), before_eq: before_eq.unwrap_or(""), 
            after_eq: after_eq.unwrap_or("")
        };
        self.order.push(TableItem::Entry(entry));
        self.items.insert(key, value);
    }
    
    pub fn get_or_create_table(&mut self, path: &[TomlKey<'a>])
            -> Result<&mut TomlTable<'a>, CreatePathError> {
        if path.is_empty() {
            Ok(self)
        } else {
            let first = path[0].clone();
            let rest = &path[1..];

            match self.items.entry(first).or_insert(TomlValue::Table(TomlTable::new(false))) {
                &mut TomlValue::Table(ref mut table) => {
                    table.get_or_create_table(rest)
                }
                _ => {
                    Err(CreatePathError::InvalidScopeTable)
                }
            }
        }
    }
    
    pub fn get_path(&self, path: &[TomlKey<'a>])
            -> Option<&TomlValue<'a>> {
        if path.is_empty() {
            None
        } else if path.len() == 1 {
            self.items.get(&path[0])
        } else {
            let ref first = path[0];
            let rest = &path[1..];

            match self.items.get(&first) {
                Some(&TomlValue::Table(ref table)) => {
                    table.get_path(rest)
                }
                Some(_) => {
                    // TODO: Return an error here
                    None
                }
                None => None,
            }
        }
    }
    
    pub fn get_or_create_array_table(&mut self, path: &[TomlKey<'a>]) -> &mut TomlTable<'a> {
        if path.is_empty() {
            self
        } else {
            unimplemented!();
        }
    }
    
    
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
    
    pub fn insert(&mut self, key: TomlKey<'a>, value: TomlValue<'a>) {
        unimplemented!();
        if self.items.contains_key(&key) {
            
        } else {
            
        }
    }
    
    pub fn write(&self, out: &mut String) {
        use self::TableItem::*;
        if self.inline {
            out.push('{');
        }
        for item in self.order.iter() {
            match *item {
                Space(text) => out.push_str(text),
                Comment(text) => {
                    out.push('#');
                    out.push_str(text);
                }
                Entry(ref entry) => {
                    entry.write(out);
                    self.items.get(&entry.key).unwrap().write(out);
                }
                Comma => out.push(','), 
                Scope(ref scope) => {
                    scope.write(out);
                    // TODO: Un-hack ;)
                    self.get_path(scope.path()).unwrap().write(out);
                }
            }
        }
        if self.inline {
            out.push('}');
        }
    }
}