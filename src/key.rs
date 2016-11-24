use std::string::ToString;
use std::borrow::{Borrow, Cow};
use std::hash;
use utils::{write_string, create_key, clean_string};

/// A TOML key. Used for both scope path elements, and for identifying table entries.
/// `key = "something"`
/// `[ key. other_key . third-key ]`
#[derive(Debug, Eq, Clone, Copy)]
pub enum Key<'a> {
    Plain(&'a str),
    String {
        text: &'a str,
        literal: bool,
        multiline: bool,
    },
    User(&'a str),
}

/// Protected interface for the `Key`.
pub trait KeyPrivate<'a> {
    fn from_key(key: &'a str) -> Key<'a>;
    fn from_string(text: &'a str, literal: bool, multiline: bool) -> Key<'a>;
}

impl<'a> KeyPrivate<'a> for Key<'a> {
    /// Wraps a plain TOML key.
    fn from_key(key: &'a str) -> Key<'a> {
        Key::Plain(key)
    }

    /// Wraps a TOML string as a key.
    fn from_string(text: &'a str, literal: bool, multiline: bool) -> Key<'a> {
        Key::String {
            text: text,
            literal: literal,
            multiline: multiline,
        }
    }
}

impl<'a> Key<'a> {
    /// Writes the TOML representation of this value to a string.
    pub fn write(&self, out: &mut String) {
        use self::Key::*;
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
        use self::Key::*;
        match *self {
            Plain(text) | User(text) => Cow::Borrowed(text),
            String { text, literal, multiline } => clean_string(text, literal, multiline),
        }
    }
}

impl<'a> ToString for Key<'a> {
    fn to_string(&self) -> String {
        self.normalized().to_string()
    }
}

impl<'a> PartialEq for Key<'a> {
    fn eq(&self, other: &Key<'a>) -> bool {
        self.normalized() == other.normalized()
    }

    fn ne(&self, other: &Key<'a>) -> bool {
        self.normalized() != other.normalized()
    }
}

impl<'a> hash::Hash for Key<'a> {
    fn hash<H>(&self, state: &mut H)
        where H: hash::Hasher
    {
        self.normalized().hash(state);
    }
}

impl<'a, 'b> From<&'b Key<'a>> for Key<'a> {
    fn from(other: &Key<'a>) -> Key<'a> {
        *other
    }
}

impl<'a> From<&'a str> for Key<'a> {
    fn from(other: &'a str) -> Key<'a> {
        Key::User(other)
    }
}

// TODO: Undo this ugly hack by properly using generics
impl<'a, 'b> From<&'b &'a str> for Key<'a> {
    fn from(other: &&'a str) -> Key<'a> {
        Key::User(*other)
    }
}
