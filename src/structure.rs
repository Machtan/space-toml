//! Data types created by the parser and given to the client.
use std::collections::HashMap;
use std::borrow::{Cow, Borrow};
use std::hash;

/// A TOML string value.
/// "Normal\nwith escapes" 'Literal' 
/// """multi-line normal""" '''multi-line literal'''
#[derive(Debug, Clone)]
pub enum TomlString<'a> {
    Text { text: &'a str, literal: bool, multiline: bool },
    User(&'a str),
    
}
impl<'a> TomlString<'a> {
    /// Creates a new TOML string from the values of the tokens given by the lexer.
    pub fn new(text: &'a str, literal: bool, multiline: bool) -> TomlString<'a> {
        TomlString::Text {
            text: text,
            literal: literal,
            multiline: multiline,
        }
    }
    
    /// Creates a new TOML string from a user string.
    /// This means that the string is formatted differently when written
    /// (it has no 'set' format like the other string variant).
    fn from_user(text: &'a str) -> TomlString<'a> {
        TomlString::User(text)
    }
}

/// A TOML float value.
/// 2.34.
#[derive(Debug)]
pub enum TomlFloat<'a> {
    Text(&'a str),
    Value(f64),
}

/// A TOML integer value.
/// `3` `32_000`.
#[derive(Debug)]
pub enum TomlInt<'a> {
    Text(&'a str),
    Value(i64),
}

/// Escapes a user-provided string as a TOML string.
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

/// Creates a TOML key from a user-supplied key. 
/// If the key is valid as a 'plain' TOML key, it is borrowed,
/// but otherwise an escaped string will be created.
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
    while let Some((_, ch)) = chars.next() {
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
                    for _ in 0..4 {
                        chars.next();
                    }
                    escaped = false;
                }
                'U' => {
                    for _ in 0..8 {
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

/// A format item for a TOML scope (table or array of tables).
#[derive(Debug, Clone)]
enum ScopeItem<'a> {
    Dot,
    Space(&'a str),
    Part(usize),
}

/// A toml scope.
/// '''[ hello . world ]'''.
#[derive(Debug, Clone)]
pub struct Scope<'a> {
    ordering: Vec<ScopeItem<'a>>,
    keys: Vec<TomlKey<'a>>,
}

impl<'a> Scope<'a> {
    /// Creates a new scope.
    pub fn new() -> Scope<'a> {
        Scope { ordering: Vec::new(), keys: Vec::new() }
    }
    
    /// Pushes a path separator '.' to the scope format order.
    pub fn push_dot(&mut self) {
        self.ordering.push(ScopeItem::Dot);
    }
    
     /// Pushes a space to the scope format order.
    pub fn push_space(&mut self, text: &'a str) {
        self.ordering.push(ScopeItem::Space(text));
    }
    
    /// Pushes a key to the scope format order.
    pub fn push_key(&mut self, key: TomlKey<'a>) {
        let new_index = self.keys.len();
        self.keys.push(key);
        self.ordering.push(ScopeItem::Part(new_index));
    }
    
    /// Returns a reference to the path this scope describes.
    pub fn path(&self) -> &Vec<TomlKey<'a>> {
        &self.keys
    }
    
    /// Writes this scope to a string in the TOML format.
    pub fn write(&self, out: &mut String) {
        use self::ScopeItem::*;
        out.push('[');
        for item in &self.ordering {
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

/// A 'visual' item within a TOML array.
#[derive(Debug)]
enum ArrayItem<'a> {
    Space(&'a str),
    Comment(&'a str),
    /// An index into the contained items of the array.
    Item(usize),
    Comma,
}

/// A homogenous array of TOML values (+ the array's visual representation).
#[derive(Debug)]
pub struct TomlArray<'a> {
    items: Vec<TomlValue<'a>>,
    order: Vec<ArrayItem<'a>>,
}

/// A protected interface for the `TomlArray`.
pub trait TomlArrayPrivate<'a> {
    fn push(&mut self, value: TomlValue<'a>) -> Result<(), String>;
    fn push_space(&mut self, space: &'a str);
    fn push_comma(&mut self);
    fn push_comment(&mut self, comment: &'a str);
}

impl<'a> TomlArrayPrivate<'a> for TomlArray<'a> {
    /// Pushes a new value to the array. 
    /// Errors if the value is of a different type than the first element of the array.
    fn push(&mut self, value: TomlValue<'a>) -> Result<(), String> {
        if let Some(first) = self.items.get(0) {
            if ! first.is_same_type(&value) {
                return Err(format!("Attempted to insert a value of type {:?} into an array of type {:?}", value, first));
            }
        }
        self.order.push(ArrayItem::Item(self.items.len()));
        self.items.push(value);
        Ok(())
    }
    
    /// Pushes an amount of whitespace to the array format order.
    fn push_space(&mut self, space: &'a str) {
        self.order.push(ArrayItem::Space(space));
    }
    
    /// Pushes a comma to the array format order.
    fn push_comma(&mut self) {
        self.order.push(ArrayItem::Comma);
    }
    
    /// Pushes a comment and a newline (CR currently NOT handled) to the array format order.
    fn push_comment(&mut self, comment: &'a str) {
        self.order.push(ArrayItem::Comment(comment));
        self.order.push(ArrayItem::Space("\n"));
    }
}

impl<'a> TomlArray<'a> {
    /// Creates a new TOML array.
    pub fn new() -> TomlArray<'a> {
        TomlArray {
            items: Vec::new(),
            order: Vec::new(),
        }
    }
    
    /// Returns the items of this array.
    pub fn items(&self) -> &Vec<TomlValue<'a>> {
        &self.items
    }
    
    /// Returns whether this array is empty of values (it might still contain formatting info).
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
    
    /// Writes this TOML value to a string.
    pub fn write(&self, out: &mut String) {
        use self::ArrayItem::*;
        out.push('[');
        for item in &self.order {
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

/// A value in the TOML system.
#[derive(Debug)]
pub enum TomlValue<'a> {
    String(TomlString<'a>),
    Bool(bool),
    Int(TomlInt<'a>),
    Float(TomlFloat<'a>),
    /// This is not validated and just given as a string. Use at your own risk.
    DateTime(&'a str),
    Table(TomlTable<'a>),
    Array(TomlArray<'a>)
}

/// Writes the TOML representation of a TOML string to another string.
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

/// A protected interface for `TomlValue`.
pub trait TomlValuePrivate<'a> {
    fn int(text: &'a str) -> TomlValue<'a>;
    fn bool(value: bool) -> TomlValue<'a>;
    fn string(text: &'a str, literal: bool, multiline: bool) -> TomlValue<'a>;
    fn float(text: &'a str) -> TomlValue<'a>;
    fn datetime(text: &'a str) -> TomlValue<'a>;
}

impl<'a> TomlValuePrivate<'a> for TomlValue<'a> {
    /// Wraps a new integer.
    fn int(text: &'a str) -> TomlValue<'a> {
        TomlValue::Int(TomlInt::Text(text))
    }
    
    /// Wraps a new bool.
    fn bool(value: bool) -> TomlValue<'a> {
        TomlValue::Bool(value)
    }
    
    /// Wraps a new string.
    fn string(text: &'a str, literal: bool, multiline: bool) -> TomlValue<'a> {
        TomlValue::String(TomlString::new(text, literal, multiline))
    }
    
    /// Wraps a new float.
    fn float(text: &'a str) -> TomlValue<'a> {
        TomlValue::Float(TomlFloat::Text(text))
    }
    
    /// Wraps a new datetime.
    fn datetime(text: &'a str) -> TomlValue<'a> {
        TomlValue::DateTime(text)
    }
}

impl<'a> TomlValue<'a> {
    /// Checks whether this value has the same variant as the given value.
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
    
    /// Writes this TOML value to a string.
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
            DateTime(text) => out.push_str(text),
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

/// A TOML key. Used for both scope path elements, and for identifying table entries.
/// `key = "something"`
/// `[ key. other_key . third-key ]`
#[derive(Debug, Eq, Clone, Copy)]
pub enum TomlKey<'a> {
    Plain(&'a str),
    String { text: &'a str, literal: bool, multiline: bool },
    User(&'a str),
}

/// Protected interface for the `TomlKey`.
pub trait TomlKeyPrivate<'a> {
    fn from_key(key: &'a str) -> TomlKey<'a>;
    fn from_string(text: &'a str, literal: bool, multiline: bool) -> TomlKey<'a>;
}

impl<'a> TomlKeyPrivate<'a> for TomlKey<'a> {
    /// Wraps a plain TOML key.
    fn from_key(key: &'a str) -> TomlKey<'a> {
        TomlKey::Plain(key)
    }
    
    /// Wraps a TOML string as a key.
    fn from_string(text: &'a str, literal: bool, multiline: bool) -> TomlKey<'a> {
        TomlKey::String { text: text, literal: literal, multiline: multiline }
    }
}

impl<'a> TomlKey<'a> {
    /// Writes the TOML representation of this value to a string.
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
    
    /// Returns the key encoded as a Rust string.
    pub fn normalized(&self) -> Cow<'a, str> {
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

/// A format item for a TOML table.
#[derive(Debug)]
enum TableItem<'a> {
    Space(&'a str),
    Newline(&'a str),
    Comment(&'a str),
    Entry { key: TomlKey<'a>, before_eq: &'a str, after_eq: &'a str, },
    /// For inline tables
    Comma,
}

/// An error found when creating a new table from a given key path.
#[derive(Debug)]
pub enum CreatePathError {
    // TODO: Add data
    InvalidScopeTable
}

/// A TOML table.
#[derive(Debug)]
pub struct TomlTable<'a> {
    inline: bool,
    order: Vec<TableItem<'a>>,
    items: HashMap<TomlKey<'a>, TomlValue<'a>>,
    visual_scopes: Vec<Scope<'a>>,
}

/// A protected interface for a the TOML table.
pub trait TomlTablePrivate<'a> {
    fn push_space(&mut self, space: &'a str);
    fn push_comma(&mut self);
    fn push_newline(&mut self, cr: bool);
    fn push_comment(&mut self, comment: &'a str);
    fn push_scope(&mut self, scope: Scope<'a>);
    fn insert_spaced<K: Into<TomlKey<'a>>>(&mut self, key: K, value: TomlValue<'a>, 
            before_eq: Option<&'a str>, after_eq: Option<&'a str>);
    
}

impl<'a> TomlTablePrivate<'a> for TomlTable<'a> {
    /// Pushes a space to the format order.
    fn push_space(&mut self, space: &'a str) {
        self.order.push(TableItem::Space(space));
    }
    
    /// Pushes a comma to the format order.
    /// Note: Only for inline tables.
    fn push_comma(&mut self) {
        self.order.push(TableItem::Comma);
    }
    
    /// Pushes a newline to the format order.
    /// Note: Only for regular tables.
    fn push_newline(&mut self, cr: bool) {
        self.order.push(TableItem::Newline(if cr { "\r\n" } else { "\n" }));
    }
    
    /// Pushes a comment to the format order.
    /// Note: Only for regular tables.
    fn push_comment(&mut self, comment: &'a str) {
        self.order.push(TableItem::Comment(comment));
    }
    
    /// Pushes a table / table-array scope to the format order.
    /// Note: Only for the top-level table.
    fn push_scope(&mut self, scope: Scope<'a>) {
        self.visual_scopes.push(scope);
    }
    
    /// Inserts the given key as an entry to the table with the given sapce.
    fn insert_spaced<K: Into<TomlKey<'a>>>(&mut self, key: K, value: TomlValue<'a>, 
            before_eq: Option<&'a str>, after_eq: Option<&'a str>) {
        let key = key.into();
        let entry = TableItem::Entry { 
            key: key, before_eq: before_eq.unwrap_or(""), 
            after_eq: after_eq.unwrap_or("")
        };
        self.order.push(entry);
        self.items.insert(key, value);
    }
}

impl<'a> TomlTable<'a> {
    /// Creates a new table.
    pub fn new(inline: bool) -> TomlTable<'a> {
        TomlTable {
            inline: inline,
            order: Vec::new(),
            items: HashMap::new(),
            visual_scopes: Vec::new(),
        }
    }
    
    /// Returns the table at the given path, potentially creating tables at all the path links.
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
                match *self.items.entry(first).or_insert_with(
                        || TomlValue::Table(TomlTable::new(false))) {
                    TomlValue::Table(ref mut table) => {
                        table.get_or_create_table(iter)
                    }
                    _ => {
                        Err(CreatePathError::InvalidScopeTable)
                    }
                }
            }
        }
    }
    
    /// Attempts to find a value at the given path in the table.
    pub fn find_value(&self, path: &[TomlKey<'a>])
            -> Option<&TomlValue<'a>> {
        if path.is_empty() {
            None
        } else if path.len() == 1 {
            self.items.get(&path[0])
        } else {
            let first = &path[0];
            let rest = &path[1..];

            match self.items.get(first) {
                Some(&TomlValue::Table(ref table)) => {
                    table.find_value(rest)
                }
                Some(_) => {
                    // TODO: Return an error here
                    None
                }
                None => None,
            }
        }
    }
    
    /// Unimplemented.
    pub fn get_or_create_array_table(&mut self, path: &[TomlKey<'a>]) -> &mut TomlTable<'a> {
        if path.is_empty() {
            self
        } else {
            unimplemented!();
        }
    }
    
    /// Returns whether the table is empty. The table might still contain format items.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
    
    /// Returns whether the inline table has a trailing comma.
    fn has_trailing_comma(&self) -> bool {
        use self::TableItem::*;
        for item in self.order.iter().rev() {
            match *item {
                Space(_) | Comment(_) | Newline(_) => {},
                Entry { .. } => return false,
                /// For inline tables
                Comma => return true, 
            }
        }
        false
    }
    
    /// Returns the last indentation of a key/value pair in the table.
    fn last_indent(&mut self) -> &'a str {
        use self::TableItem::*;
        let mut last_was_entry = false;
        let mut after_newline = false;
        let mut first_space = None;
        for item in self.order.iter().rev() {
            match *item {
                Entry { .. } => {
                    last_was_entry = true;
                }
                Space(text) => {
                    if after_newline && first_space.is_none() {
                        first_space = Some(text);
                    }
                    if last_was_entry {
                        return text
                    }
                }
                Comment(_) | Comma => last_was_entry = false,
                Newline(_) => {
                    last_was_entry = false;
                    after_newline = true;
                }
            }
        }
        first_space.unwrap_or("")
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
    
    /// Inserts a new item into the table.
    /// Note: This function attempts to be smart with the formatting.
    pub fn insert<K, V>(&mut self, key: K, value: V) 
            where K: Into<TomlKey<'a>>, V: Into<TomlValue<'a>> {
        use self::TableItem::*;
        let key = key.into();
        let value = value.into();
        if self.items.contains_key(&key) {
            self.items.insert(key, value);
        } else {
            if ! self.inline {
                let entry = Entry { 
                    key: key, before_eq: " ", 
                    after_eq: " "
                };
                self.items.insert(key, value);
                let mut values = Vec::new();
                let indent = self.last_indent();
                if indent != "" {
                    values.push(Space(indent));
                }
                values.push(entry);
                values.push(Newline("\n")); // TODO: cr
                self.push_before_space(values);
            } else {
                let had_comma = self.has_trailing_comma();
                if ! had_comma {
                    self.order.push(Comma);
                    self.order.push(Space(" "));
                } else if ! self.order.is_empty() { // Pad with space
                    let last = self.order.len() - 1;
                    if let Comma = self.order[last] {
                        self.order.push(Space(" "));
                    }
                }
                self.insert_spaced(key, value, Some(" "), Some(" "));
                if had_comma {
                    self.order.push(Comma);
                }
            }
        }
    }
    
    /// Writes the TOML representation of this value to a string.
    pub fn write(&self, out: &mut String) {
        use self::TableItem::*;
        if self.inline {
            out.push('{');
        }
        for item in &self.order {
            match *item {
                Space(text) | Newline(text) => out.push_str(text),
                Comment(text) => {
                    out.push('#');
                    out.push_str(text);
                }
                Entry { key, before_eq, after_eq } => {
                    key.write(out);
                    out.push_str(before_eq);
                    out.push('=');
                    out.push_str(after_eq);
                    self.items.get(&key).unwrap().write(out);
                }
                Comma => out.push(','), 
            }
        }
        if self.inline {
            out.push('}');
        }
        for scope in &self.visual_scopes {
            scope.write(out);
            self.find_value(scope.path()).unwrap().write(out);
        }
    }
}