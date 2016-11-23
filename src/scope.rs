
use key::Key;

/// A format item for a TOML scope (table or array of tables).
#[derive(Debug, Clone)]
enum ScopeItem<'a> {
    Dot,
    Space(&'a str),
    Part(usize),
}

/// A toml scope.
/// '''[ hello . world ]'''.
#[derive(Debug, Clone)]
pub struct Scope<'a> {
    ordering: Vec<ScopeItem<'a>>,
    keys: Vec<Key<'a>>,
}

impl<'a> Scope<'a> {
    /// Creates a new scope.
    pub fn new() -> Scope<'a> {
        Scope {
            ordering: Vec::new(),
            keys: Vec::new(),
        }
    }

    /// Pushes a path separator '.' to the scope format order.
    pub fn push_dot(&mut self) {
        self.ordering.push(ScopeItem::Dot);
    }

    /// Pushes a space to the scope format order.
    pub fn push_space(&mut self, text: &'a str) {
        self.ordering.push(ScopeItem::Space(text));
    }

    /// Pushes a key to the scope format order.
    pub fn push_key(&mut self, key: Key<'a>) {
        let new_index = self.keys.len();
        self.keys.push(key);
        self.ordering.push(ScopeItem::Part(new_index));
    }

    /// Returns a reference to the path this scope describes.
    pub fn path(&self) -> &Vec<Key<'a>> {
        &self.keys
    }

    /// Writes this scope to a string in the TOML format.
    pub fn write(&self, out: &mut String) {
        use self::ScopeItem::*;
        out.push('[');
        for item in &self.ordering {
            match *item {
                Dot => out.push('.'),
                Space(text) => out.push_str(text),
                Part(index) => {
                    self.keys[index].write(out);
                }
            }
        }
        out.push(']');
    }
}
