
use key::Key;
use value::Value;
use scope::Scope;
use std::collections::{HashMap, hash_map};

/// A format item for a TOML table.
#[derive(Debug)]
enum TableItem<'a> {
    Space(&'a str),
    Newline(&'a str),
    Comment(&'a str),
    Entry {
        key: Key<'a>,
        before_eq: &'a str,
        after_eq: &'a str,
    },
    /// A [table] or an [[array_of_tables]]
    Scope(Key<'a>),
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
pub struct Table<'a> {
    inline: bool,
    scope: Option<Scope<'a>>,
    order: Vec<TableItem<'a>>,
    items: HashMap<Key<'a>, Value<'a>>,
}

/// A protected interface for a the TOML table.
pub trait TablePrivate<'a> {
    fn new(inline: bool, scope: Option<Scope<'a>>) -> Table<'a>;
    fn set_scope(&mut self, scope: Scope<'a>);
    fn push_table(&mut self, key: Key<'a>);
    fn push_space(&mut self, space: &'a str);
    fn push_comma(&mut self);
    fn push_newline(&mut self, cr: bool);
    fn push_comment(&mut self, comment: &'a str);
    fn insert_spaced<K: Into<Key<'a>>>(&mut self,
                                           key: K,
                                           value: Value<'a>,
                                           before_eq: Option<&'a str>,
                                           after_eq: Option<&'a str>);
}

impl<'a> TablePrivate<'a> for Table<'a> {
    /// Creates a new table.
    fn new(inline: bool, scope: Option<Scope<'a>>) -> Table<'a> {
        Table {
            inline: inline,
            order: Vec::new(),
            items: HashMap::new(),
            scope: scope,
        }
    }

    /// Sets the scope for this table.
    fn set_scope(&mut self, scope: Scope<'a>) {
        self.scope = Some(scope);
    }

    /// Pushes a table/array of tables to the format order.
    fn push_table(&mut self, key: Key<'a>) {
        self.order.push(TableItem::Scope(key));
    }

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
        self.order.push(TableItem::Newline(if cr {
            "\r\n"
        } else {
            "\n"
        }));
    }

    /// Pushes a comment to the format order.
    /// Note: Only for regular tables.
    fn push_comment(&mut self, comment: &'a str) {
        self.order.push(TableItem::Comment(comment));
    }

    /// Inserts the given key as an entry to the table with the given sapce.
    fn insert_spaced<K: Into<Key<'a>>>(&mut self,
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
}

impl<'a> Table<'a> {
    /// Creates a new regular TOML table.
    pub fn new_regular() -> Table<'a> {
        Table::new(false, None)
    }

    /// Creates a new inline TOML table.
    pub fn new_inline() -> Table<'a> {
        Table::new(true, None)
    }

    fn find_or_insert_with_slice<F, T>(&mut self,
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
                    if value.is_noninline_table() || value.is_noninline_array_of_tables() {
                        self.push_table(key.clone());
                    }
                    self.items.insert(key.clone(), value);
                }
                Ok(self.items.get_mut(&key).unwrap())
            }
            [key, _..] => {
                let has_entry = self.items.contains_key(&key);
                if !has_entry {
                    let value = Value::Table(Table::new_regular());
                    self.items.insert(key.clone(), value);
                    self.push_table(key.clone());
                    self.items.get_mut(&key).unwrap().table_mut().unwrap().find_or_insert_with_slice(&path[1..], default)
                } else {
                    match *self.items.get_mut(&key).unwrap() {
                        Value::Table(ref mut table) => {
                            table.find_or_insert_with(&path[1..], default)
                        }
                        Value::Array(ref mut array) => {
                            if array.is_empty() {
                                array.push(Table::new_regular()).unwrap();
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
    pub fn find_or_insert_with<I, P, F, T>(&mut self,
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
    }

    /// Returns the table at the given path, creating intermediate tables if they don't 
    /// exist. The intermediate tables are by default non-inlined. For a more custom 
    /// version see 'find_or_insert_with'.
    pub fn find_or_insert_table<I, P>(&mut self,
                                      path: P)
                                      -> Result<&mut Table<'a>, CreatePathError>
        where P: IntoIterator<Item = I>,
              I: Into<Key<'a>>
    {
        let path: Vec<Key<'a>> = path.into_iter().map(|k| k.into()).collect();
        if path.is_empty() {
            return Err(CreatePathError::InvalidScopeTable);
        }
        let value =
            self.find_or_insert_with_slice(&path, || Value::Table(Table::new(false, None)))?;
        match *value {
            Value::Table(ref mut table) => Ok(table),
            _ => unreachable!(),
        }
    }

    /// Attempts to find a value at the given path in the table.
    pub fn find(&self, path: &[Key<'a>]) -> Option<&Value<'a>> {
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
    pub fn find_or_create_array_table(&mut self, path: &[Key<'a>]) -> &mut Table<'a> {
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

    /// Returns whether the table is empty. The table might still contain format items.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Returns whether the inline table has a trailing comma.
    fn has_trailing_comma(&self) -> bool {
        use self::TableItem::*;
        for item in self.order.iter().rev() {
            match *item {
                Space(_) | Comment(_) | Newline(_) | Scope(_) => {}
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
                Scope(_key) => {
                    last_was_entry = false;
                    // (since entries belong to their tables, this shouldn't have any 
                    // negative effects)
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
    fn ensure_newline_after_scope(&mut self) {
        if self.inline {
            return;
        }
        if ! self.order.iter().any(|item| item.is_newline()) {
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

    /// Writes the TOML representation of this value to a string.
    pub fn write(&self, out: &mut String) {
        use self::TableItem::*;
        if let Some(ref scope) = self.scope {
            scope.write(out);
        }
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
                Scope(ref key) => {
                    //TODO: Pass scope along in order to facilitate changed scopes from 
                    // the loaded format
                    match *self.items.get(key).expect("scope key not in table") {
                        Value::Table(ref table) => {
                            table.write(out);
                        }
                        Value::Array(ref array) => {
                            if ! array.is_inline() {
                                array.write(out);
                            } else {
                                panic!("Broken invariant: Scoped array is not array of tables");
                            }
                        }
                        _ => {}
                    }
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
}
