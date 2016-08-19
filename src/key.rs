use std::string::ToString;
use std::borrow::{Borrow, Cow};
use std::hash;
use utils::{write_string, create_key, clean_string};

/// A TOML key. Used for both scope path elements, and for identifying table entries.
/// `key = "something"`
/// `[ key. other_key . third-key ]`
#[derive(Debug, Eq, Clone, Copy)]
pub enum TomlKey<'a> {
    Plain(&'a str),
    String {
        text: &'a str,
        literal: bool,
        multiline: bool,
    },
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
        TomlKey::String {
            text: text,
            literal: literal,
            multiline: multiline,
        }
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
    
    /// Returns the key as a String.
    pub fn to_string(&self) -> String {
        self.normalized().to_string()
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
    fn hash<H>(&self, state: &mut H)
        where H: hash::Hasher
    {
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
