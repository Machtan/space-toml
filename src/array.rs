
use value::Value;
use std::slice;

/// A 'visual' item within a TOML array.
#[derive(Debug)]
enum ArrayItem<'a> {
    Space(&'a str),
    Comment(&'a str),
    /// An index into the contained items of the array.
    Item,
    Comma,
}

/// A homogenous array of TOML values (+ the array's visual representation).
#[derive(Debug)]
pub struct Array<'a> {
    items: Vec<Value<'a>>,
    order: Vec<ArrayItem<'a>>,
}

/// A protected interface for the `Array`.
pub trait ArrayPrivate<'a> {
    fn push(&mut self, value: Value<'a>) -> Result<(), String>;
    fn push_space(&mut self, space: &'a str);
    fn push_comma(&mut self);
    fn push_comment(&mut self, comment: &'a str);
}

impl<'a> ArrayPrivate<'a> for Array<'a> {
    /// Pushes a new value to the array.
    /// Errors if the value is of a different type than the first element of the array.
    fn push(&mut self, value: Value<'a>) -> Result<(), String> {
        if let Some(first) = self.items.get(0) {
            if !first.is_same_type(&value) {
                return Err(format!("Attempted to insert a value of type {:?} into an array of \
                                    type {:?}",
                                   value,
                                   first));
            }
        }
        self.order.push(ArrayItem::Item);
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

impl<'a> Array<'a> {
    /// Creates a new TOML array.
    pub fn new() -> Array<'a> {
        Array {
            items: Vec::new(),
            order: Vec::new(),
        }
    }

    /// Returns the items of this array.
    pub fn items(&self) -> &Vec<Value<'a>> {
        &self.items
    }

    /// Returns an iterator over the items in this array.
    pub fn iter(&self) -> slice::Iter<Value<'a>> {
        self.items.iter()
    }

    /// Returns whether this array is empty of values (it might still contain formatting info).
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Writes this TOML value to a string.
    pub fn write(&self, out: &mut String) {
        use self::ArrayItem::*;
        out.push('[');
        let mut item_no = 0;
        for item in &self.order {
            match *item {
                Space(text) => out.push_str(text),
                Comment(text) => {
                    out.push('#');
                    out.push_str(text);
                }
                Item => {
                    self.items[item_no].write(out);
                    item_no += 1;
                }
                Comma => out.push(','),
            }
        }
        out.push(']');
    }
}
