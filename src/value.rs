
use std::borrow::{Borrow, Cow};
use table::TableData;
use array::ArrayData;
use utils::{write_string, escape_string, clean_string};

/// A TOML string value.
/// "Normal\nwith escapes" 'Literal'
/// """multi-line normal""" '''multi-line literal'''
#[derive(Debug, Clone)]
pub enum TomlString<'a> {
    /// A formatted TOML string, without quotes.
    Text {
        /// The text inside the quotes.
        text: &'a str,
        /// Whether this is a literal string (`'`-quoted, with no escape characters
        /// allowed).
        literal: bool,
        /// Whether this is a multiline (triple-quoted) string.
        multiline: bool,
    },
    /// A user-supplied string.
    User(Cow<'a, str>),
}

pub trait TomlStringPrivate {
    /// Creates a new TOML string from the values of the tokens given by the lexer.
    fn new<'a>(text: &'a str, literal: bool, multiline: bool) -> TomlString<'a> {
        TomlString::Text {
            text: text,
            literal: literal,
            multiline: multiline,
        }
    }
}

impl<'a> TomlStringPrivate for TomlString<'a> {}

impl<'a> TomlString<'a> {
    /// Creates a new TOML string from a user-supplied string.
    /// This means that the string is formatted differently when written
    /// (it has no 'set' format like the other string variant).
    pub fn from_user<T: Into<Cow<'a, str>>>(text: T) -> TomlString<'a> {
        TomlString::User(text.into())
    }

    /// Returns the string with escape characters converted to proper UTF-8 characters.
    pub fn clean(&self) -> Cow<'a, str> {
        use self::TomlString::*;
        match *self {
            Text { text, literal, multiline } => clean_string(text, literal, multiline),
            User(ref cow) => cow.clone(),
        }
    }
}

/// A TOML floating point number.
/// example: `2.34`.
#[derive(Debug)]
pub enum Float<'a> {
    /// A formatted float read from a document.
    /// If you create this yourself, you can write invalid TOML documents :D.
    Text(&'a str),
    /// A user-inserted value.
    Value(f64),
}

impl<'a> Float<'a> {
    /// Returns the value of this number.
    pub fn value(&self) -> f64 {
        use self::Float::*;
        match *self {
            Text(text) => text.parse().expect("Unparseable TOML float"),
            Value(value) => value,
        }
    }
}

/// A TOML integer.
/// example: `3` `32_000`.
#[derive(Debug)]
pub enum Int<'a> {
    /// A formatted integer read from a document.
    /// If you create this yourself, you can write invalid TOML documents :D.
    Text(&'a str),
    /// A user-inserted value.
    Value(i64),
}

impl<'a> Int<'a> {
    /// Returns the value of this number.
    pub fn value(&self) -> i64 {
        use self::Int::*;
        match *self {
            Text(text) => text.parse().expect("Unparseable TOML float"),
            Value(value) => value,
        }
    }
}


/// A value in the TOML system.
#[derive(Debug)]
pub enum Value<'a> {
    /// A string value
    String(TomlString<'a>),
    /// A boolean value
    Bool(bool),
    /// An integer
    Int(Int<'a>),
    /// A floating-point number
    Float(Float<'a>),
    /// This is not validated and just given as a string. Use at your own risk.
    DateTime(&'a str),
    /// A table, regular or inlined
    Table(TableData<'a>),
    /// An array of values or tables
    Array(ArrayData<'a>),
}

/// A protected interface for `Value`.
pub trait ValuePrivate<'a> {
    fn new_int(text: &'a str) -> Value<'a>;
    fn new_bool(value: bool) -> Value<'a>;
    fn new_string(text: &'a str, literal: bool, multiline: bool) -> Value<'a>;
    fn new_float(text: &'a str) -> Value<'a>;
    fn new_datetime(text: &'a str) -> Value<'a>;
}

impl<'a> ValuePrivate<'a> for Value<'a> {
    /// Wraps a new integer.
    fn new_int(text: &'a str) -> Value<'a> {
        Value::Int(Int::Text(text))
    }

    /// Wraps a new bool.
    fn new_bool(value: bool) -> Value<'a> {
        Value::Bool(value)
    }

    /// Wraps a new string.
    fn new_string(text: &'a str, literal: bool, multiline: bool) -> Value<'a> {
        Value::String(TomlString::new(text, literal, multiline))
    }

    /// Wraps a new float.
    fn new_float(text: &'a str) -> Value<'a> {
        Value::Float(Float::Text(text))
    }

    /// Wraps a new datetime.
    fn new_datetime(text: &'a str) -> Value<'a> {
        Value::DateTime(text)
    }
}

impl<'a> Value<'a> {
    /// Checks whether this value has the same variant as the given value.
    pub fn is_same_type(&self, other: &Value) -> bool {
        use self::Value::*;
        match (self, other) {
            (&String(_), &String(_)) => true,
            (&Bool(_), &Bool(_)) => true,
            (&Int(_), &Int(_)) => true,
            (&Float(_), &Float(_)) => true,
            (&Table(_), &Table(_)) => true,
            (&Array(_), &Array(_)) => true,
            (&DateTime(_), &DateTime(_)) => true,
            _ => false,
        }
    }

    /// Returns a reference to the table in this item (if valid).
    pub fn table(&self) -> Option<&TableData<'a>> {
        if let Value::Table(ref table) = *self {
            Some(table)
        } else {
            None
        }
    }

    /// Returns a mutable reference to the table in this item (if valid).
    pub fn table_mut(&mut self) -> Option<&mut TableData<'a>> {
        if let Value::Table(ref mut table) = *self {
            Some(table)
        } else {
            None
        }
    }

    /// Returns a mutable reference to the array in this item (if valid).
    pub fn array_mut(&mut self) -> Option<&mut ArrayData<'a>> {
        if let Value::Array(ref mut array) = *self {
            Some(array)
        } else {
            None
        }
    }

    /// Returns reference to the array in this item (if valid).
    pub fn array(&self) -> Option<&ArrayData<'a>> {
        if let Value::Array(ref array) = *self {
            Some(array)
        } else {
            None
        }
    }

    /// Returns the string value of this item (if valid).
    pub fn string(&self) -> Option<Cow<'a, str>> {
        if let Value::String(ref string) = *self {
            Some(string.clean())
        } else {
            None
        }
    }

    /// Returns the boolean value of this item (if valid).
    pub fn bool(&self) -> Option<bool> {
        if let Value::Bool(value) = *self {
            Some(value)
        } else {
            None
        }
    }

    /// Returns the integer value of this item (if valid).
    pub fn int(&self) -> Option<i64> {
        if let Value::Int(ref int) = *self {
            Some(int.value())
        } else {
            None
        }
    }

    /// Returns the float value of this item (if valid).
    pub fn float(&self) -> Option<f64> {
        if let Value::Float(ref float) = *self {
            Some(float.value())
        } else {
            None
        }
    }

    /// Returns the datetime value of this item (if valid).
    pub fn datetime(&self) -> Option<&'a str> {
        if let Value::DateTime(value) = *self {
            Some(value)
        } else {
            None
        }
    }

    /// Returns whether this value is a regular (non-inline) table.
    pub fn is_noninline_table(&self) -> bool {
        if let Value::Table(ref table) = *self {
            !table.is_inline()
        } else {
            false
        }
    }

    /// Returns whether this is a regular (non-inline) array of tables.
    pub fn is_noninline_array_of_tables(&self) -> bool {
        if let Value::Array(ref array) = *self {
            !array.is_inline()
        } else {
            false
        }
    }

    /// Returns whether this value is a table.
    pub fn is_table(&self) -> bool {
        if let Value::Table(_) = *self {
            true
        } else {
            false
        }
    }

    // String(TomlString<'a>),
    // Bool(bool),
    // Int(Int<'a>),
    // Float(Float<'a>),
    // This is not validated and just given as a string. Use at your own risk.
    // DateTime(&'a str),
    // Table(TableData<'a>),
    // Array(ArrayData<'a>),

    /// Writes this TOML value to a string.
    pub fn write(&self, out: &mut String) {
        use self::Value::*;
        match *self {
            String(TomlString::Text { text, literal, multiline }) => {
                write_string(text, literal, multiline, out);
            }
            String(TomlString::User(ref text)) => {
                out.push_str(&escape_string(text.borrow()));
            }
            Bool(b) => out.push_str(if b { "true" } else { "false" }),
            DateTime(text) => out.push_str(text),
            Int(self::Int::Text(text)) => out.push_str(text),
            Int(self::Int::Value(v)) => out.push_str(&format!("{}", v)),
            Float(self::Float::Text(text)) => out.push_str(text),
            Float(self::Float::Value(v)) => out.push_str(&format!("{}", v)),
            Table(ref table) => table.write(out),
            Array(ref array) => array.write(out),
        }
    }
}

impl<'a> From<&'a str> for Value<'a> {
    fn from(other: &'a str) -> Value<'a> {
        Value::String(TomlString::from_user(other))
    }
}

impl<'a> From<Cow<'a, str>> for Value<'a> {
    fn from(other: Cow<'a, str>) -> Value<'a> {
        Value::String(TomlString::from_user(other))
    }
}

impl<'a> From<String> for Value<'a> {
    fn from(other: String) -> Value<'a> {
        Value::String(TomlString::from_user(other))
    }
}

impl<'a> From<TableData<'a>> for Value<'a> {
    fn from(other: TableData<'a>) -> Value<'a> {
        Value::Table(other)
    }
}

impl<'a> From<i64> for Value<'a> {
    fn from(other: i64) -> Value<'a> {
        Value::Int(Int::Value(other))
    }
}

impl<'a> From<i32> for Value<'a> {
    fn from(other: i32) -> Value<'a> {
        Value::Int(Int::Value(other as i64))
    }
}

impl<'a> From<f32> for Value<'a> {
    fn from(other: f32) -> Value<'a> {
        Value::Float(Float::Value(other as f64))
    }
}

impl<'a> From<f64> for Value<'a> {
    fn from(other: f64) -> Value<'a> {
        Value::Float(Float::Value(other))
    }
}

impl<'a> From<ArrayData<'a>> for Value<'a> {
    fn from(other: ArrayData<'a>) -> Value<'a> {
        Value::Array(other)
    }
}
