
use std::borrow::{Borrow, Cow};
use table::Table;
use array::Array;
use utils::{write_string, escape_string, clean_string};

/// A TOML string value.
/// "Normal\nwith escapes" 'Literal'
/// """multi-line normal""" '''multi-line literal'''
#[derive(Debug, Clone)]
pub enum TomlString<'a> {
    Text {
        text: &'a str,
        literal: bool,
        multiline: bool,
    },
    User(Cow<'a, str>),
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
    fn from_user<T: Into<Cow<'a, str>>>(text: T) -> TomlString<'a> {
        TomlString::User(text.into())
    }


    fn clean(&self) -> Cow<'a, str> {
        use self::TomlString::*;
        match *self {
            Text { text, literal, multiline } => clean_string(text, literal, multiline),
            User(ref cow) => cow.clone(),
        }
    }
}

/// A TOML float value.
/// 2.34.
#[derive(Debug)]
pub enum Float<'a> {
    Text(&'a str),
    Value(f64),
}

impl<'a> Float<'a> {
    fn value(&self) -> f64 {
        use self::Float::*;
        match *self {
            Text(text) => text.parse().expect("Unparseable TOML float"),
            Value(value) => value,
        }
    }
}

/// A TOML integer value.
/// `3` `32_000`.
#[derive(Debug)]
pub enum Int<'a> {
    Text(&'a str),
    Value(i64),
}

impl<'a> Int<'a> {
    fn value(&self) -> i64 {
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
    Table(Table<'a>),
    /// An array of values or tables
    Array(Array<'a>),
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
            _ => false,
        }
    }

    /// Returns a reference to the table in this item (if valid).
    pub fn table(&self) -> Option<&Table<'a>> {
        if let Value::Table(ref table) = *self {
            Some(table)
        } else {
            None
        }
    }

    /// Returns a mutable reference to the table in this item (if valid).
    pub fn table_mut(&mut self) -> Option<&mut Table<'a>> {
        if let Value::Table(ref mut table) = *self {
            Some(table)
        } else {
            None
        }
    }

    /// Returns a mutable reference to the array in this item (if valid).
    pub fn array_mut(&mut self) -> Option<&mut Array<'a>> {
        if let Value::Array(ref mut array) = *self {
            Some(array)
        } else {
            None
        }
    }

    /// Returns reference to the array in this item (if valid).
    pub fn array(&mut self) -> Option<&Array<'a>> {
        if let Value::Array(ref mut array) = *self {
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

    // String(TomlString<'a>),
    // Bool(bool),
    // Int(Int<'a>),
    // Float(Float<'a>),
    // This is not validated and just given as a string. Use at your own risk.
    // DateTime(&'a str),
    // Table(Table<'a>),
    // Array(Array<'a>),

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
            Bool(b) => {
                out.push_str(if b {
                    "true"
                } else {
                    "false"
                })
            }
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

impl<'a> From<Table<'a>> for Value<'a> {
    fn from(other: Table<'a>) -> Value<'a> {
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

impl<'a> From<Array<'a>> for Value<'a> {
    fn from(other: Array<'a>) -> Value<'a> {
        Value::Array(other)
    }
}
