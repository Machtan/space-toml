use std::fmt::Debug;
use std::collections::HashMap;
use std::str::FromStr;
use std::borrow::{Cow, Borrow};
use std::char;
use std::hash;

#[derive(Debug, Clone)]
pub enum TomlString<'a> {
    Text { text: &'a str, literal: bool, multiline: bool },
    User(&'a str),
    
}
impl<'a> TomlString<'a> {
    pub fn new(text: &'a str, literal: bool, multiline: bool) -> TomlString<'a> {
        TomlString::Text {
            text: text,
            literal: literal,
            multiline: multiline,
        }
    }
    
    fn from_user(text: &'a str) -> TomlString<'a> {
        TomlString::User(text)
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

fn escape_string(text: &str) -> String {
    let mut escaped = String::new();
    escaped.push('"');
    for ch in text.chars() {
        match ch {
            '\n' => escaped.push_str("\\n"),
            '\t' => escaped.push_str("\\t"),
            '\r' => escaped.push_str("\\r"),
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            other => {
                escaped.push(other);
            }
        }
    }
    escaped.push('"');
    escaped
}

fn create_key<'a>(text: &'a str) -> Cow<'a, str> {
    let mut chars = text.chars();
    let mut simple = true;
    match chars.next().unwrap() {
        'a' ... 'z' | 'A' ... 'Z' | '_' | '-' => {
            for ch in text.chars() {
                match ch {
                    'a' ... 'z' | 'A' ... 'Z' | '0' ... '9' | '_' | '-' => {}
                    _ => simple = false,
                }
            }
        },
        _ => simple = false,
    }
    if simple {
        Cow::Borrowed(text)
    } else {
        Cow::Owned(escape_string(text))
    }
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
                    string.push('\u{0008}');
                    escaped = false;
                }
                'f' => {
                    string.push('\u{000C}');
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
            String(TomlString::Text { text, literal, multiline}) => {
                write_string(text, literal, multiline, out);
            },
            String(TomlString::User(text)) => {
                out.push_str(&escape_string(text));
            }
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

impl<'a> From<&'a str> for TomlValue<'a> {
    fn from(other: &'a str) -> TomlValue<'a> {
        TomlValue::String(TomlString::from_user(other))
    }
}

#[derive(Debug, Eq, Clone, Copy)]
pub enum TomlKey<'a> {
    Plain(&'a str),
    String { text: &'a str, literal: bool, multiline: bool },
    User(&'a str),
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
            User(text) => {
                out.push_str(create_key(text).borrow());
            }
        }
    }
    
    fn normalized(&self) -> Cow<'a, str> {
        use self::TomlKey::*;
        match *self {
            Plain(text) | User(text) => Cow::Borrowed(text),
            String { text, literal, multiline } => clean_string(text, literal, multiline),
        }
    }
}

impl<'a> PartialEq for TomlKey<'a> {
    fn eq(&self, other: &TomlKey<'a>) -> bool {
        self.normalized() == other.normalized()
    }
    
    fn ne(&self, other: &TomlKey<'a>) -> bool {
        self.normalized() != other.normalized()
    }
}

impl<'a> hash::Hash for TomlKey<'a> {
    fn hash<H>(&self, state: &mut H) where H: hash::Hasher {
        self.normalized().hash(state);
    }
}

impl<'a, 'b> From<&'b TomlKey<'a>> for TomlKey<'a> {
    fn from(other: &TomlKey<'a>) -> TomlKey<'a> {
        *other
    }
}
// TODO: Make keys as simple as possible in the TOML representation
impl<'a> From<&'a str> for TomlKey<'a> {
    fn from(other: &'a str) -> TomlKey<'a> {
        TomlKey::User(other)
    }
}

// TODO: Undo this ugly hack by properly using generics
impl<'a, 'b> From<&'b &'a str> for TomlKey<'a> {
    fn from(other: &&'a str) -> TomlKey<'a> {
        TomlKey::User(*other)
    }
}

#[derive(Debug)]
enum TableItem<'a> {
    Space(&'a str),
    Newline(&'a str),
    Comment(&'a str),
    Entry(ValueEntry<'a>),
    /// For inline tables
    Comma,
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
    subtables: Vec<(Scope<'a>)>, // Only used in the top-level table
}
impl<'a> TomlTable<'a> {
    pub fn new(inline: bool) -> TomlTable<'a> {
        TomlTable {
            inline: inline,
            order: Vec::new(),
            items: HashMap::new(),
            subtables: Vec::new(),
        }
    }
    
    pub fn push_space(&mut self, space: &'a str) {
        self.order.push(TableItem::Space(space));
    }
    
    pub fn push_newline(&mut self, cr: bool) {
        self.order.push(TableItem::Newline(if cr { "\r\n" } else { "\n" }));
    }
    
    pub fn push_comment(&mut self, comment: &'a str) {
        self.order.push(TableItem::Comment(comment));
    }
    
    pub fn push_scope(&mut self, scope: Scope<'a>) {
        self.subtables.push(scope);
    }
    
    pub fn insert_spaced<K: Into<TomlKey<'a>>>(&mut self, key: K, value: TomlValue<'a>, 
            before_eq: Option<&'a str>, after_eq: Option<&'a str>) {
        let key = key.into();
        let entry = ValueEntry { 
            key: key.clone(), before_eq: before_eq.unwrap_or(""), 
            after_eq: after_eq.unwrap_or("")
        };
        self.order.push(TableItem::Entry(entry));
        self.items.insert(key, value);
    }
    
    pub fn get_or_create_table<I, P>(&mut self, path: P)
            -> Result<&mut TomlTable<'a>, CreatePathError> 
            where P: IntoIterator<Item=I>, I: Into<TomlKey<'a>> {
        let mut iter = path.into_iter();
        match iter.next() {
            None => {
                Ok(self)
            }
            Some(first) => {
                let first = first.into();
                match self.items.entry(first).or_insert(TomlValue::Table(TomlTable::new(false))) {
                    &mut TomlValue::Table(ref mut table) => {
                        table.get_or_create_table(iter)
                    }
                    _ => {
                        Err(CreatePathError::InvalidScopeTable)
                    }
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
    
    fn has_trailing_comma(&self) -> bool {
        use self::TableItem::*;
        for item in self.order.iter().rev() {
            match *item {
                Space(_) | Comment(_) | Newline(_) => {},
                Entry(_) => return false,
                /// For inline tables
                Comma => return true, 
            }
        }
        false
    }
    
    fn last_indent(&mut self) -> &'a str {
        use self::TableItem::*;
        let mut last_was_entry = false;
        for item in self.order.iter().rev() {
            match *item {
                Entry(_) => last_was_entry = true,
                Space(text) => {
                    if last_was_entry {
                        return text
                    }
                }
                Comment(_) | Comma | Newline(_) => last_was_entry = false,
            }
        }
        ""
    }
    
    /// Pushes the given items before the last space in the table
    fn push_before_space(&mut self, items: Vec<TableItem<'a>>) {
        if self.order.is_empty() {
            self.order.extend(items);
        } else {
            let last = self.order.len() - 1;
            let last_is_space = if let TableItem::Space(_) = self.order[last] {
                true
            } else {
                false
            };
            if last_is_space {
                let pop = self.order.pop().unwrap();
                for item in items {
                    self.order.push(item);
                }
                self.order.push(pop);
            } else {
                for item in items {
                    self.order.push(item);
                }
            }
        }
    }
    
    pub fn insert<K, V>(&mut self, key: K, value: V) 
            where K: Into<TomlKey<'a>>, V: Into<TomlValue<'a>> {
        use self::TableItem::*;
        let key = key.into();
        let value = value.into();
        if self.items.contains_key(&key) {
            self.items.insert(key, value);
        } else {
            if ! self.inline {
                let indent = self.last_indent();
                let entry = ValueEntry { 
                    key: key.clone(), before_eq: " ", 
                    after_eq: " "
                };
                self.items.insert(key, value);
                let mut values = Vec::new();
                let indent = self.last_indent();
                if indent != "" {
                    values.push(Space(indent));
                }
                values.push(Entry(entry));
                values.push(Newline("\n")); // TODO: cr
                self.push_before_space(values);
            } else {
                if ! self.has_trailing_comma() {
                    self.order.push(Comma);
                    self.order.push(Space(" "));
                }
                self.insert_spaced(key, value, Some(" "), Some(" "));
            }
        }
    }
    
    pub fn write(&self, out: &mut String) {
        use self::TableItem::*;
        if self.inline {
            out.push('{');
        }
        for item in self.order.iter() {
            match *item {
                Space(text) | Newline(text) => out.push_str(text),
                Comment(text) => {
                    out.push('#');
                    out.push_str(text);
                }
                Entry(ref entry) => {
                    entry.write(out);
                    self.items.get(&entry.key).unwrap().write(out);
                }
                Comma => out.push(','), 
            }
        }
        if self.inline {
            out.push('}');
        }
        for scope in self.subtables.iter() {
            scope.write(out);
            self.get_path(scope.path()).unwrap().write(out);
        }
    }
}