
use std::borrow::{Borrow, Cow};
use table::TomlTable;
use array::TomlArray;
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
pub enum TomlFloat<'a> {
    Text(&'a str),
    Value(f64),
}

impl<'a> TomlFloat<'a> {
    fn value(&self) -> f64 {
        use self::TomlFloat::*;
        match *self {
            Text(text) => text.parse().expect("Unparseable TOML float"),
            Value(value) => value,
        }
    }
}

/// A TOML integer value.
/// `3` `32_000`.
#[derive(Debug)]
pub enum TomlInt<'a> {
    Text(&'a str),
    Value(i64),
}

impl<'a> TomlInt<'a> {
    fn value(&self) -> i64 {
        use self::TomlInt::*;
        match *self {
            Text(text) => text.parse().expect("Unparseable TOML float"),
            Value(value) => value,
        }
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
    Array(TomlArray<'a>),
}

/// A protected interface for `TomlValue`.
pub trait TomlValuePrivate<'a> {
    fn new_int(text: &'a str) -> TomlValue<'a>;
    fn new_bool(value: bool) -> TomlValue<'a>;
    fn new_string(text: &'a str, literal: bool, multiline: bool) -> TomlValue<'a>;
    fn new_float(text: &'a str) -> TomlValue<'a>;
    fn new_datetime(text: &'a str) -> TomlValue<'a>;
}

impl<'a> TomlValuePrivate<'a> for TomlValue<'a> {
    /// Wraps a new integer.
    fn new_int(text: &'a str) -> TomlValue<'a> {
        TomlValue::Int(TomlInt::Text(text))
    }

    /// Wraps a new bool.
    fn new_bool(value: bool) -> TomlValue<'a> {
        TomlValue::Bool(value)
    }

    /// Wraps a new string.
    fn new_string(text: &'a str, literal: bool, multiline: bool) -> TomlValue<'a> {
        TomlValue::String(TomlString::new(text, literal, multiline))
    }

    /// Wraps a new float.
    fn new_float(text: &'a str) -> TomlValue<'a> {
        TomlValue::Float(TomlFloat::Text(text))
    }

    /// Wraps a new datetime.
    fn new_datetime(text: &'a str) -> TomlValue<'a> {
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
            _ => false,
        }
    }

    /// Returns a reference to the table in this item (if valid).
    pub fn table(&self) -> Option<&TomlTable<'a>> {
        if let TomlValue::Table(ref table) = *self {
            Some(table)
        } else {
            None
        }
    }

    /// Returns a mutable reference to the table in this item (if valid).
    pub fn table_mut(&mut self) -> Option<&mut TomlTable<'a>> {
        if let TomlValue::Table(ref mut table) = *self {
            Some(table)
        } else {
            None
        }
    }

    /// Returns a mutable reference to the array in this item (if valid).
    pub fn array_mut(&mut self) -> Option<&mut TomlArray<'a>> {
        if let TomlValue::Array(ref mut array) = *self {
            Some(array)
        } else {
            None
        }
    }

    /// Returns reference to the array in this item (if valid).
    pub fn array(&mut self) -> Option<&TomlArray<'a>> {
        if let TomlValue::Array(ref mut array) = *self {
            Some(array)
        } else {
            None
        }
    }

    /// Returns the string value of this item (if valid).
    pub fn string(&self) -> Option<Cow<'a, str>> {
        if let TomlValue::String(ref string) = *self {
            Some(string.clean())
        } else {
            None
        }
    }

    /// Returns the boolean value of this item (if valid).
    pub fn bool(&self) -> Option<bool> {
        if let TomlValue::Bool(value) = *self {
            Some(value)
        } else {
            None
        }
    }

    /// Returns the integer value of this item (if valid).
    pub fn int(&self) -> Option<i64> {
        if let TomlValue::Int(ref int) = *self {
            Some(int.value())
        } else {
            None
        }
    }

    /// Returns the float value of this item (if valid).
    pub fn float(&self) -> Option<f64> {
        if let TomlValue::Float(ref float) = *self {
            Some(float.value())
        } else {
            None
        }
    }

    /// Returns the datetime value of this item (if valid).
    pub fn datetime(&self) -> Option<&'a str> {
        if let TomlValue::DateTime(value) = *self {
            Some(value)
        } else {
            None
        }
    }

    // String(TomlString<'a>),
    // Bool(bool),
    // Int(TomlInt<'a>),
    // Float(TomlFloat<'a>),
    // This is not validated and just given as a string. Use at your own risk.
    // DateTime(&'a str),
    // Table(TomlTable<'a>),
    // Array(TomlArray<'a>),

    /// Writes this TOML value to a string.
    pub fn write(&self, out: &mut String) {
        use self::TomlValue::*;
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

impl<'a> From<Cow<'a, str>> for TomlValue<'a> {
    fn from(other: Cow<'a, str>) -> TomlValue<'a> {
        TomlValue::String(TomlString::from_user(other))
    }
}

impl<'a> From<String> for TomlValue<'a> {
    fn from(other: String) -> TomlValue<'a> {
        TomlValue::String(TomlString::from_user(other))
    }
}

impl<'a> From<TomlTable<'a>> for TomlValue<'a> {
    fn from(other: TomlTable<'a>) -> TomlValue<'a> {
        TomlValue::Table(other)
    }
}

impl<'a> From<i64> for TomlValue<'a> {
    fn from(other: i64) -> TomlValue<'a> {
        TomlValue::Int(TomlInt::Value(other))
    }
}

impl<'a> From<i32> for TomlValue<'a> {
    fn from(other: i32) -> TomlValue<'a> {
        TomlValue::Int(TomlInt::Value(other as i64))
    }
}

impl<'a> From<f32> for TomlValue<'a> {
    fn from(other: f32) -> TomlValue<'a> {
        TomlValue::Float(TomlFloat::Value(other as f64))
    }
}

impl<'a> From<f64> for TomlValue<'a> {
    fn from(other: f64) -> TomlValue<'a> {
        TomlValue::Float(TomlFloat::Value(other))
    }
}

impl<'a> From<TomlArray<'a>> for TomlValue<'a> {
    fn from(other: TomlArray<'a>) -> TomlValue<'a> {
        TomlValue::Array(other)
    }
}
