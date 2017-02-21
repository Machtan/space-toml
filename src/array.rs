
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
pub struct ArrayData<'a> {
    items: Vec<Value<'a>>,
    order: Vec<ArrayItem<'a>>,
    /// Whether this is an inline (value-position) array or an array of tables.
    is_inline: bool,
}

impl<'a> ArrayData<'a> {
    /// Creates a new TOML array.
    pub fn new_inline() -> ArrayData<'a> {
        ArrayData {
            items: Vec::new(),
            order: Vec::new(),
            is_inline: true,
        }
    }

    /// Creates a TOML array that should contain tables.
    pub fn new_of_tables() -> ArrayData<'a> {
        ArrayData {
            items: Vec::new(),
            order: Vec::new(),
            is_inline: false,
        }
    }

    /// Returns whether this is an inline (value-position) array.
    /// Example: `array = ["some", "values"]`.
    pub fn is_inline(&self) -> bool {
        self.is_inline
    }

    /// Returns the items of this array.
    pub fn items(&self) -> &Vec<Value<'a>> {
        &self.items
    }

    pub fn push_value(&mut self, value: Value<'a>) -> Result<&mut Value<'a>, String> {
        if let Some(first) = self.items.get(0) {
            if !first.is_same_type(&value) {
                return Err(format!("Attempted to insert a value of type {:?} into an array of \
                                    type {:?}",
                                   value,
                                   first));
            }
        }
        // Is this specifically a noninline array of tables? Check the type again.
        if !self.is_inline && !value.is_table() {
            return Err(format!("Attempted to insert a value of type {:?} into an array of tables",
                               value));
        }
        self.order.push(ArrayItem::Item);
        self.items.push(value);
        let index = self.items.len() - 1;
        Ok(&mut self.items[index])
    }

    /// Pushes an amount of whitespace to the array format order.
    pub fn push_space(&mut self, space: &'a str) {
        self.order.push(ArrayItem::Space(space));
    }

    /// Pushes a comma to the array format order.
    pub fn push_comma(&mut self) {
        self.order.push(ArrayItem::Comma);
    }

    /// Pushes a comment to the array format order.
    pub fn push_comment(&mut self, comment: &'a str) {
        self.order.push(ArrayItem::Comment(comment));
    }

    /// Returns an iterator over the items in this array.
    pub fn iter(&self) -> slice::Iter<Value<'a>> {
        self.items.iter()
    }

    /// Returns whether this array is empty of values (it might still contain formatting info).
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Returns the last element of this array.
    pub fn last(&mut self) -> Option<&mut Value<'a>> {
        if self.is_empty() {
            None
        } else {
            let index = self.items.len() - 1;
            Some(&mut self.items[index])
        }
    }

    /// Returns whether the given value has the same type as the other elements of this array.
    pub fn can_insert_type(&self, value: &Value) -> bool {
        if let Some(first) = self.items.get(0) {
            first.is_same_type(value)
        } else {
            true
        }
    }

    /// Returns whether this array has a trailing comma.
    fn has_trailing_comma(&self) -> bool {
        for item in self.order.iter().rev() {
            match *item {
                ArrayItem::Comma => return true,
                ArrayItem::Item => return false,
                _ => {}
            }
        }
        false
    }

    /// Pushes a new value to the array and returns a reference to it.
    /// Errors if the value is of a different type than the first element of the array.
    /// TODO: This should be split into an internal and external function.
    pub fn push<V: Into<Value<'a>>>(&mut self, value: V) -> Result<&mut Value<'a>, String> {
        let value = value.into();
        if self.is_inline && !self.has_trailing_comma() {
            self.push_comma();
            self.push_space(" ");
        }
        self.push_value(value)
    }

    /// Writes this TOML value to a string.
    pub fn write(&self, out: &mut String) {
        use self::ArrayItem::*;
        if self.is_inline {
            out.push('[');
        }
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
        if self.is_inline {
            out.push(']');
        }
    }
}
