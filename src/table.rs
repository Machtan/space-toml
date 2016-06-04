
use key::TomlKey;
use value::TomlValue;
use scope::Scope;
use std::collections::HashMap;

/// A format item for a TOML table.
#[derive(Debug)]
enum TableItem<'a> {
    Space(&'a str),
    Newline(&'a str),
    Comment(&'a str),
    Entry {
        key: TomlKey<'a>,
        before_eq: &'a str,
        after_eq: &'a str,
    },
    /// For inline tables
    Comma,
}

/// An error found when creating a new table from a given key path.
#[derive(Debug)]
pub enum CreatePathError {
    // TODO: Add data
    InvalidScopeTable,
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
    fn insert_spaced<K: Into<TomlKey<'a>>>(&mut self,
                                           key: K,
                                           value: TomlValue<'a>,
                                           before_eq: Option<&'a str>,
                                           after_eq: Option<&'a str>);
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

    /// Pushes a table / table-array scope to the format order.
    /// Note: Only for the top-level table.
    fn push_scope(&mut self, scope: Scope<'a>) {
        self.visual_scopes.push(scope);
    }

    /// Inserts the given key as an entry to the table with the given sapce.
    fn insert_spaced<K: Into<TomlKey<'a>>>(&mut self,
                                           key: K,
                                           value: TomlValue<'a>,
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

impl<'a> TomlTable<'a> {
    /// Creates a new table.
    fn new(inline: bool) -> TomlTable<'a> {
        TomlTable {
            inline: inline,
            order: Vec::new(),
            items: HashMap::new(),
            visual_scopes: Vec::new(),
        }
    }

    /// Creates a new regular TOML table.
    pub fn new_regular() -> TomlTable<'a> {
        TomlTable::new(false)
    }

    /// Creates a new inline TOML table.
    pub fn new_inline() -> TomlTable<'a> {
        TomlTable::new(true)
    }

    fn find_or_insert_with_slice<F, T>(&mut self,
                                       path: &[TomlKey<'a>],
                                       default: F)
                                       -> Result<&mut TomlValue<'a>, CreatePathError>
        where F: FnOnce() -> T,
              T: Into<TomlValue<'a>>
    {
        match path {
            [key] => Ok(self.items.entry(key).or_insert_with(|| default().into())),
            [key, _..] => {
                match *self.items
                    .entry(key)
                    .or_insert_with(|| TomlValue::Table(TomlTable::new(false))) {
                    TomlValue::Table(ref mut table) => {
                        table.find_or_insert_with_slice(&path[1..], default)
                    }
                    _ => Err(CreatePathError::InvalidScopeTable),
                }
            }
            [] => {
                unreachable!();
            }
        }
    }

    // TODO: Better errors
    /// Returns the table at the given path, potentially creating tables at all the path links.
    pub fn find_or_insert_with<I, P, F, T>(&mut self,
                                           path: P,
                                           default: F)
                                           -> Result<&mut TomlValue<'a>, CreatePathError>
        where P: IntoIterator<Item = I>,
              I: Into<TomlKey<'a>>,
              F: FnOnce() -> T,
              T: Into<TomlValue<'a>>
    {
        let path: Vec<TomlKey<'a>> = path.into_iter().map(|k| k.into()).collect();
        if path.is_empty() {
            return Err(CreatePathError::InvalidScopeTable);
        }
        self.find_or_insert_with_slice(&path, default)
    }

    pub fn find_or_insert_table<I, P>(&mut self,
                                      path: P)
                                      -> Result<&mut TomlTable<'a>, CreatePathError>
        where P: IntoIterator<Item = I>,
              I: Into<TomlKey<'a>>
    {
        let path: Vec<TomlKey<'a>> = path.into_iter().map(|k| k.into()).collect();
        if path.is_empty() {
            return Err(CreatePathError::InvalidScopeTable);
        }
        let value =
            self.find_or_insert_with_slice(&path, || TomlValue::Table(TomlTable::new(false)))?;
        match *value {
            TomlValue::Table(ref mut table) => Ok(table),
            _ => unreachable!(),
        }
    }

    /// Attempts to find a value at the given path in the table.
    pub fn find(&self, path: &[TomlKey<'a>]) -> Option<&TomlValue<'a>> {
        if path.is_empty() {
            None
        } else if path.len() == 1 {
            self.items.get(&path[0])
        } else {
            let first = &path[0];
            let rest = &path[1..];

            match self.items.get(first) {
                Some(&TomlValue::Table(ref table)) => table.find(rest),
                Some(_) => {
                    // TODO: Return an error here
                    None
                }
                None => None,
            }
        }
    }

    /// Attempts to find a value at the given path in the table.
    pub fn find_mut(&mut self, path: &[TomlKey<'a>]) -> Option<&mut TomlValue<'a>> {
        if path.is_empty() {
            None
        } else if path.len() == 1 {
            self.items.get_mut(&path[0])
        } else {
            let first = &path[0];
            let rest = &path[1..];

            match self.items.get_mut(first) {
                Some(&mut TomlValue::Table(ref mut table)) => table.find_mut(rest),
                Some(_) => {
                    // TODO: Return an error here
                    None
                }
                None => None,
            }
        }
    }

    /// Unimplemented.
    pub fn find_or_create_array_table(&mut self, path: &[TomlKey<'a>]) -> &mut TomlTable<'a> {
        if path.is_empty() {
            self
        } else {
            unimplemented!();
        }
    }

    pub fn get<K: Into<TomlKey<'a>>>(&self, key: K) -> Option<&TomlValue<'a>> {
        self.items.get(&key.into())
    }

    pub fn get_mut<K: Into<TomlKey<'a>>>(&mut self, key: K) -> Option<&mut TomlValue<'a>> {
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
                Space(_) | Comment(_) | Newline(_) => {}
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
        where K: Into<TomlKey<'a>>,
              V: Into<TomlValue<'a>>
    {
        use self::TableItem::*;
        let key = key.into();
        let value = value.into();
        if self.items.contains_key(&key) {
            self.items.insert(key, value);
        } else {
            if !self.inline {
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
            self.find(scope.path()).unwrap().write(out);
        }
    }
}
