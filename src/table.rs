
use key::Key;
use value::Value;
use scope::Scope;
use std::collections::{HashMap, hash_map};

/// A format item for a TOML table.
#[derive(Debug)]
pub enum TableItem<'a> {
    Space(&'a str),
    Newline(&'a str),
    Comment(&'a str),
    Entry {
        key: Key<'a>,
        before_eq: &'a str,
        after_eq: &'a str,
    },
    /// For inline tables
    Comma,
}
impl<'a> TableItem<'a> {
    fn is_newline(&self) -> bool {
        if let &TableItem::Newline(_) = self {
            true
        } else {
            false
        }
    }
}

/// An error found when creating a new table from a given key path.
#[derive(Debug)]
pub enum CreatePathError {
    // TODO: Add data
    /// A part of the requested path was not a Table, eg. looking for
    /// 'settings.targets.bin', 'settings.targets' is an array instead of a table,
    /// so the path cannot be followed.
    InvalidScopeTable,
    /// The given path is empty
    EmptyPath,
}

/// A TOML table.
#[derive(Debug)]
pub struct TableData<'a> {
    pub inline: bool,
    pub order: Vec<TableItem<'a>>,
    pub items: HashMap<Key<'a>, Value<'a>>,
}

impl<'a> TableData<'a> {
    /// Creates a new table.
    fn new(inline: bool) -> TableData<'a> {
        TableData {
            inline: inline,
            order: Vec::new(),
            items: HashMap::new(),
        }
    }

    /// Creates a new regular TOML table.
    pub fn new_regular() -> TableData<'a> {
        TableData::new(false)
    }

    /// Creates a new inline TOML table.
    pub fn new_inline() -> TableData<'a> {
        TableData::new(true)
    }

    /// Pushes a space to the format order.
    pub fn push_space(&mut self, space: &'a str) {
        self.order.push(TableItem::Space(space));
    }

    /// Pushes a comma to the format order.
    /// Note: Only for inline tables.
    pub fn push_comma(&mut self) {
        self.order.push(TableItem::Comma);
    }

    /// Pushes a newline to the format order.
    /// Note: Only for regular tables.
    pub fn push_newline(&mut self, cr: bool) {
        self.order.push(TableItem::Newline(if cr { "\r\n" } else { "\n" }));
    }

    /// Pushes a comment to the format order.
    /// Note: Only for regular tables.
    pub fn push_comment(&mut self, comment: &'a str) {
        self.order.push(TableItem::Comment(comment));
    }

    /// Inserts the given key as an entry to the table with the given sapce.
    pub fn insert_spaced<K: Into<Key<'a>>>(&mut self,
                                           key: K,
                                           value: Value<'a>,
                                           before_eq: Option<&'a str>,
                                           after_eq: Option<&'a str>) {
        let key = key.into();
        let entry = TableItem::Entry {
            key: key,
            before_eq: before_eq.unwrap_or(""),
            after_eq: after_eq.unwrap_or(""),
        };
        self.order.push(entry);
        self.items.insert(key, value);
    }

    /// Attempts to find a value at the given path in the table.
    pub fn find(&self, path: &[Key<'a>]) -> Option<&Value<'a>> {
        panic!("Broken!");
        if path.is_empty() {
            None
        } else if path.len() == 1 {
            self.items.get(&path[0])
        } else {
            let first = &path[0];
            let rest = &path[1..];

            match self.items.get(first) {
                Some(&Value::Table(ref table)) => table.find(rest),
                Some(_) => {
                    // TODO: Return an error here
                    None
                }
                None => None,
            }
        }
    }

    /// Attempts to find a value at the given path in the table.
    pub fn find_mut(&mut self, path: &[Key<'a>]) -> Option<&mut Value<'a>> {
        panic!("Broken!");
        if path.is_empty() {
            None
        } else if path.len() == 1 {
            self.items.get_mut(&path[0])
        } else {
            let first = &path[0];
            let rest = &path[1..];

            match self.items.get_mut(first) {
                Some(&mut Value::Table(ref mut table)) => table.find_mut(rest),
                Some(_) => {
                    // TODO: Return an error here
                    None
                }
                None => None,
            }
        }
    }

    /// Unimplemented.
    pub fn find_or_create_array_table(&mut self, path: &[Key<'a>]) -> &mut TableData<'a> {
        if path.is_empty() {
            self
        } else {
            unimplemented!();
        }
    }

    /// Returns a reference to the value at the given key in this table, if present.
    pub fn get<K: Into<Key<'a>>>(&self, key: K) -> Option<&Value<'a>> {
        self.items.get(&key.into())
    }

    /// Returns a mutable reference to the value at the given key in this table, if
    /// present.
    pub fn get_mut<K: Into<Key<'a>>>(&mut self, key: K) -> Option<&mut Value<'a>> {
        self.items.get_mut(&key.into())
    }

    /// Returns whether the given key exists in the table.
    pub fn contains_key<K: Into<Key<'a>>>(&self, key: K) -> bool {
        self.items.contains_key(&key.into())
    }

    /// Returns whether the table is empty. The table might still contain format items.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Removes an item from this table if present.
    pub fn remove(&mut self, key: &Key<'a>) -> Option<Value<'a>> {
        self.items.remove(key)
    }

    /// Returns whether the inline table has a trailing comma.
    pub fn has_trailing_comma(&self) -> bool {
        use self::TableItem::*;
        for item in self.order.iter().rev() {
            match *item {
                Space(_) | Comment(_) | Newline(_) => {}
                Entry { .. } => return false,
                /// For inline tables
                Comma => return true, 
            }
        }
        false
    }

    /// Returns the last indentation of a key/value pair in the table.
    pub fn last_indent(&mut self) -> &'a str {
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
                        return text;
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

    /// Iterates over the keys and values in the table.
    pub fn iter(&self) -> hash_map::Iter<Key<'a>, Value<'a>> {
        self.items.iter()
    }

    /// Iterates mutably over the keys and values in the table.
    pub fn iter_mut(&mut self) -> hash_map::IterMut<Key<'a>, Value<'a>> {
        self.items.iter_mut()
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

    /// Ensures that there is a newline before the first key/value pair
    pub fn ensure_newline_after_scope(&mut self) {
        if self.inline {
            return;
        }
        if !self.order.iter().any(|item| item.is_newline()) {
            self.push_newline(false); // TODO: Add CR on windows?
        }
    }

    /// Inserts a new item into the table.
    /// Note: This function attempts to be smart with the formatting.
    pub fn insert<K, V>(&mut self, key: K, value: V)
        where K: Into<Key<'a>>,
              V: Into<Value<'a>>
    {
        use self::TableItem::*;
        let key = key.into();
        let value = value.into();
        if self.items.contains_key(&key) {
            self.items.insert(key, value);
        } else {
            if !self.inline {
                self.ensure_newline_after_scope();
                let entry = Entry {
                    key: key,
                    before_eq: " ",
                    after_eq: " ",
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
                if !self.items.is_empty() {
                    if !had_comma {
                        self.order.push(Comma);
                        self.order.push(Space(" "));
                    } else if !self.order.is_empty() {
                        // Pad with space
                        let last = self.order.len() - 1;
                        if let Comma = self.order[last] {
                            self.order.push(Space(" "));
                        }
                    }
                }

                self.insert_spaced(key, value, Some(" "), Some(" "));
                if had_comma {
                    self.order.push(Comma);
                }
            }
        }
    }

    /// Returns whether this table is inline.
    pub fn is_inline(&self) -> bool {
        self.inline
    }

    pub fn write(&self, out: &mut String) {
        self.write_with_root(out);
    }

    /// Writes the TOML representation of this value to a string.
    pub fn write_with_root(&self, out: &mut String) {
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
    }
    
    /*fn find_or_insert_with_slice<F, T>(&mut self,
                                       path: &[Key<'a>],
                                       default: F)
                                       -> Result<&mut Value<'a>, CreatePathError>
        where F: FnOnce() -> T,
              T: Into<Value<'a>>
    {
        match *path {
            [key] => {
                let has_entry = self.items.contains_key(&key);
                if !has_entry {
                    let value = default().into();
                    if value.is_noninline_table() {
                        self.push_table(vec![key.clone()]);
                    } else if value.is_noninline_array_of_tables() {
                        self.push_array_of_tables(vec![key.clone()], 0);
                    }
                    self.items.insert(key.clone(), value);
                }
                Ok(self.items.get_mut(&key).unwrap())
            }
            [key, _..] => {
                let has_entry = self.items.contains_key(&key);
                if !has_entry {
                    let value = Value::Table(TableData::new_regular());
                    self.items.insert(key.clone(), value);
                    self.push_table(vec![key.clone()]);
                    self.items.get_mut(&key).unwrap().table_mut().unwrap().find_or_insert_with_slice(&path[1..], default)
                } else {
                    match *self.items.get_mut(&key).unwrap() {
                        Value::Table(ref mut table) => {
                            table.find_or_insert_with(&path[1..], default)
                        }
                        Value::Array(ref mut array) => {
                            if array.is_empty() {
                                array.push(TableData::new_regular()).unwrap();
                            }
                            if ! array.is_inline() {
                                if let Some(&mut Value::Table(ref mut table)) = array.last() {
                                    table.find_or_insert_with_slice(&path[1..], default)
                                } else {
                                    unreachable!();
                                }
                            } else {
                                Err(CreatePathError::InvalidScopeTable)
                            }
                        }
                        _ => Err(CreatePathError::InvalidScopeTable),
                    }
                }
            }
            [] => {
                unreachable!();
            }
        }
    }
    
    // TODO: Better errors
    /// Returns the table at the given path, creating intermediate tables if they don't 
    /// exist, by using the supplied function.
    fn find_or_insert_with<I, P, F, T>(&mut self,
                                           path: P,
                                           default: F)
                                           -> Result<&mut Value<'a>, CreatePathError>
        where P: IntoIterator<Item = I>,
              I: Into<Key<'a>>,
              F: FnOnce() -> T,
              T: Into<Value<'a>>
    {
        let path: Vec<Key<'a>> = path.into_iter().map(|k| k.into()).collect();
        if path.is_empty() {
            return Err(CreatePathError::EmptyPath);
        }
        self.find_or_insert_with_slice(&path, default)
    }*/
}

/*pub trait TableDataPrivate {
    fn find_or_insert_table<'a, I, P>(&mut self,
                                      path: P)
                                      -> Result<&mut TableData<'a>, CreatePathError>
        where P: IntoIterator<Item = I>,
              I: Into<Key<'a>>;
}

impl<'b> TableDataPrivate for TableData<'b> {
    /// Returns the table at the given path, creating intermediate tables if they don't 
    /// exist. The intermediate tables are by default non-inlined. For a more custom 
    /// version see 'find_or_insert_with'.
    fn find_or_insert_table<'a, I, P>(&mut self,
                                      path: P)
                                      -> Result<&mut TableData<'a>, CreatePathError>
        where P: IntoIterator<Item = I>,
              I: Into<Key<'a>>
    {
        let path: Vec<Key<'a>> = path.into_iter().map(|k| k.into()).collect();
        if path.is_empty() {
            return Err(CreatePathError::InvalidScopeTable);
        }
        let value =
            self.find_or_insert_with_slice(&path, || Value::Table(TableData::new(false, None)))?;
        match *value {
            Value::Table(ref mut table) => Ok(table),
            _ => unreachable!(),
        }
    }
}*/
