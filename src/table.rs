use tabledata::TableData;
use document::DocumentItem;
use value::Value;
use key::Key;
use std::collections::hash_map;

/// A TOML table. This is a map from strings to a TOML values.
pub struct Table<'src: 'doc, 'doc> {
    data: &'doc mut TableData<'src>,
    order: &'doc mut Vec<DocumentItem<'src>>,
}
impl<'src, 'doc> Table<'src, 'doc> {
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
    /// Creates a new table wrapper.
    fn new(data: &'doc mut TableData<'src>,
               order: &'doc mut Vec<DocumentItem<'src>>)
               -> Table<'src, 'doc>;
    
    /// Returns a reference to the internal data of this wrapper.
    fn data(&mut self) -> &mut TableData<'src>;
}

impl<'src, 'doc> TablePrivate<'src, 'doc> for Table<'src, 'doc> {
    fn new(data: &'doc mut TableData<'src>,
           order: &'doc mut Vec<DocumentItem<'src>>)
           -> Table<'src, 'doc> {
        Table {
            data: data,
            order: order,
        }
    }
    
    fn data(&mut self) -> &mut TableData<'src> {
        &mut self.data
    }
}