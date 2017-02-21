
use table::{TableData};
use scope::Scope;
use key::Key;
use value::Value;
use std::iter::IntoIterator;
use std::collections::hash_map;

/// A TOML table. This is a map from strings to a TOML values.
pub struct Table<'src: 'doc, 'doc> {
    data: &'doc mut TableData<'src>,
    order: &'doc mut Vec<DocumentItem<'src>>,
}
impl<'src, 'doc> Table<'src, 'doc> {
    fn new(data: &'doc mut TableData<'src>,
           order: &'doc mut Vec<DocumentItem<'src>>)
           -> Table<'src, 'doc> {
        Table {
            data: data,
            order: order,
        }
    }
    
    /// Returns the value for the given key, optionally inserting a value
    /// using the provided function if the entry is empty.
    pub fn get_or_insert_with<F: FnOnce() -> Value<'src>>(&mut self,
                                                        key: Key<'src>,
                                                        default: F)
                                                        -> &mut Value<'src> {
        self.data.items.entry(key).or_insert_with(default)
    }

    /// Inserts the given key as an entry to the table with the given spacing.
    pub fn insert_spaced<K, V>(&mut self,
                                           key: K,
                                           value: V,
                                           before_eq: Option<&'src str>,
                                           after_eq: Option<&'src str>) 
                                         where K: Into<Key<'src>>,
                                               V: Into<Value<'src>>
                                         {
        
        // TODO: validate spacing
        self.data.insert_spaced(key, value, before_eq, after_eq)
    }
    
    /// Inserts the given key as an entry to the table with default spacing.
    pub fn insert<K, V>(&mut self, key: K, value: V)
        where K: Into<Key<'src>>,
              V: Into<Value<'src>>
    {
        self.data.insert_spaced(key, value, Some(" "), Some(" "))
    }
    
    /// Inserts a new item into the table.
    /// Note: This function attempts to be smart with the formatting.
    pub fn insert_smart<K, V>(&mut self, key: K, value: V)
        where K: Into<Key<'src>>,
              V: Into<Value<'src>>
    {
        self.data.insert(key, value)
    }
    
    /// Returns a reference to the value at the given key in this table, if present.
    pub fn get<K: Into<Key<'src>>>(&self, key: K) -> Option<&Value<'src>> {
        self.data.get(key)
    }

    /// Returns a mutable reference to the value at the given key in this table, if
    /// present.
    pub fn get_mut<K: Into<Key<'src>>>(&mut self, key: K) -> Option<&mut Value<'src>> {
        self.data.get_mut(key)
    }

    /// Returns whether the given key exists in the table.
    pub fn contains_key<K: Into<Key<'src>>>(&self, key: K) -> bool {
        self.data.contains_key(key)
    }

    /// Returns whether the table is empty. The table might still contain format items.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Removes an item from this table if present.
    pub fn remove(&mut self, key: &Key<'src>) -> Option<Value<'src>> {
        self.data.remove(key)
    }
    
    /// Iterates over the keys and values in the table.
    pub fn iter(&self) -> hash_map::Iter<Key<'src>, Value<'src>> {
        self.data.iter()
    }

    /// Iterates mutably over the keys and values in the table.
    pub fn iter_mut(&mut self) -> hash_map::IterMut<Key<'src>, Value<'src>> {
        self.data.iter_mut()
    }
}

pub trait TablePrivate<'src, 'doc> {
    fn data(&mut self) -> &mut TableData<'src>;
}

impl<'src, 'doc> TablePrivate<'src, 'doc> for Table<'src, 'doc> {
    fn data(&mut self) -> &mut TableData<'src> {
        &mut self.data
    }
}

/// An error found when creating or following a table path.
#[derive(Debug)]
pub enum InsertTableError {
    PathItemNotTable(String),
    EmptyPath,
}

/// A line-separating text sequence.
#[derive(Debug, Clone, Copy)]
pub enum Newline {
    /// '\n'
    Lf,
    /// '\r\n'
    CrLf, 
}

enum DocumentItem<'src> {
    Whitespace(&'src str),
    Newline(Newline),
    Comment(&'src str),
    Table(Scope<'src>),
    ArrayScope(Scope<'src>),
}

/// A representation of a formatted TOML document.
/// It corresponds to the top-level table, and is used to read and edit the document,
/// while preserving its formatting.
pub struct Document<'src> {
    tree: TableData<'src>,
    order: Vec<DocumentItem<'src>>,
}

impl<'src> Document<'src> {
    /// Creates an empty document.
    pub fn new() -> Document<'src> {
        Document {
            tree: TableData::new_regular(),
            order: Vec::new(),
        }
    }
    
    /// Returns the top-level table of the document.
    pub fn root<'doc>(&'doc mut self) -> Table<'src, 'doc> {
        Table::new(&mut self.tree, &mut self.order)
    }
    
    /// Adds an amount of whitespace to the document.
    /// Errors if the given strings contains characters other than valid
    /// TOML whitespace, that is spaces or tabs.
    pub fn push_space(&mut self, space: &'src str) -> Result<(), String> {
        if space.chars().all(|c| c == ' ' || c == '\t') {
            self.order.push(DocumentItem::Whitespace(space));
            Ok(())
        } else {
            Err("Found invalid TOML whitespace character!".to_string())
        }
    }

    /// Adds a newline character to the document.
    pub fn push_newline(&mut self, newline: Newline) {
        self.order.push(DocumentItem::Newline(newline));
    }
    
    /// Adds a table scope to the document.
    pub fn push_table_scope(&mut self, scope: Scope<'src>) {
        unimplemented!();
    }

    /// Adds an array-of-tables scope to the document.
    pub fn push_array_scope(&mut self, scope: Scope<'src>) {
        unimplemented!();
    }
    
    /// Adds a comment to the document.
    pub fn push_comment(&mut self, text: &'src str) {
        unimplemented!();
    }
    
    fn find_or_insert_table_internal<'doc>(&'doc mut self, path: &[Key<'src>]) -> Result<(&'doc mut TableData<'src>, &'doc mut Vec<DocumentItem<'src>>), InsertTableError> {
        match *path {
            [key] => {
                unimplemented!();
            }
            [key, _..] => {
                unimplemented!();
            }
            [] => {
                Err(InsertTableError::EmptyPath)
            }
        }
    }

    /// Finds or inserts a table at the given path.
    pub fn find_or_insert_table<'doc, I, V>(&'doc mut self, path: I) 
        -> Result<Table<'src, 'doc>, InsertTableError> 
        where I: IntoIterator<Item=V>, V: Into<Key<'src>>
    {
        let slice = path.into_iter().map(|v| v.into()).collect::<Vec<_>>();
        let (table_ref, order) = self.find_or_insert_table_internal(&slice)?;
        Ok(Table::new(table_ref, order))
    }
    
    /// Writes this document to a string.
    pub fn write(&self, string: &mut String) {
        unimplemented!();
    }
}

/// Private API for the Document struct.
pub trait DocumentPrivate<'src> {
    /// Pushes a space to the document order without validating.
    fn push_space_unchecked(&mut self, space: &'src str);
    
    /// Pushes a table scope to the document order without validating.
    fn push_table_scope_unchecked(&mut self, scope: Scope<'src>);
    
    /// Pushes an array-of-tables scope to the document order without validating.
    fn push_array_scope_unchecked(&mut self, scope: Scope<'src>);
    
    /// Pushes a comment to the document order without validating.
    fn push_comment_unchecked(&mut self, text: &'src str);
}

impl<'src> DocumentPrivate<'src> for Document<'src> {
    fn push_space_unchecked(&mut self, space: &'src str) {
        self.order.push(DocumentItem::Whitespace(space));
    }
    
    fn push_table_scope_unchecked(&mut self, scope: Scope<'src>) {
        self.order.push(DocumentItem::Table(scope));
    }
    
    fn push_array_scope_unchecked(&mut self, scope: Scope<'src>) {
        self.order.push(DocumentItem::ArrayScope(scope));
    }
    
    fn push_comment_unchecked(&mut self, text: &'src str) {
        self.order.push(DocumentItem::Comment(text));
    }
}
