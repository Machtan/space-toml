
use table::{TableData};
use scope::Scope;
use key::Key;
use value::Value;

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

    pub fn get_or_insert_with<F: FnOnce() -> Value<'src>>(&mut self,
                                                        key: Key<'src>,
                                                        default: F)
                                                        -> &mut Value<'src> {
        self.data.items.entry(key).or_insert_with(default)
    }

    /// Inserts the given key as an entry to the table with the given sapce.
    pub fn insert_spaced<K: Into<Key<'src>>>(&mut self,
                                           key: K,
                                           value: Value<'src>,
                                           before_eq: Option<&'src str>,
                                           after_eq: Option<&'src str>) {
        self.data.insert_spaced(key, value, before_eq, after_eq)
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
pub enum InsertTableError {
    PathItemNotTable(String),
    EmptyPath,
}

/// A line-separating text sequence.
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
    pub fn find_or_insert_table<'doc>(&'doc mut self, path: &[Key<'src>]) -> Result<Table<'src, 'doc>, InsertTableError>
    {
        let (table_ref, order) = self.find_or_insert_table_internal(path)?;
        Ok(Table::new(table_ref, order))
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
