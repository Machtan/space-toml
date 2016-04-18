#[derive(Debug)]
enum OrderItem {
    Space(String),
    Comment(String),
    Entry { 
        key: Vec<String>,
        before_eq: String,
        after_eq: String,
        after_value: String,
    },
    Table { 
        scope: String,      // "[ lol. bax . b]"
        key: Vec<String>,   // ["lol", "bax", "b"]
    },
    TableArray {
        scope: String,
        key: Vec<String>,
    }
}

#[derive(Debug)]
struct ValueEntry {
    key: String,
    before_eq: String,
    after_eq: String,
    value: TomlValue,
    after_value: String,
}

#[derive(Debug)]
pub struct TomlArray<T: Debug> {
    values: Vec<T>,
    spacing: Vec<(String, String)>, // Comments can also go here
}

#[derive(Debug)]
pub enum TomlString {
    String(String),
    Literal(String),
    MultilineString(String),
    MultilineLiteral(String),
}

#[derive(Debug)]
pub enum TomlValue {
    String(TomlString),
    Bool(bool),
    Int(i64),
    Float(f64),
    DateTime(DateTime<UTC>),
    Table(TomlTable),
    StringArray(TomlArray<String>),
    BoolArray(TomlArray<bool>),
    IntArray(TomlArray<i64>),
    FloatArray(TomlArray<f64>),
    DateTimeArray(TomlArray<DateTime<UTC>>),
    TableArray(Vec<TomlTable>),
}

#[derive(Debug)]
pub enum MapKind {
    Normal,
    Inline,
}

#[derive(Debug)]
pub struct TomlTable {
    kind: MapKind,
    ordering: Vec<OrderItem>,
    map: HashMap<String, TomlValue>,
}
impl TomlTable {
    pub fn new(kind: MapKind) -> TomlTable {
        TomlTable {
            kind: kind,
            ordering: Vec::new(),
            map: HashMap::new(),
        }
    }
    
    fn push_space(&mut self, space: String) {
        self.ordering.push(OrderItem::Space(space));
    }
    
    fn push_comment(&mut self, comment: String) {
        self.ordering.push(OrderItem::Comment(comment));
    }
}

#[derive(Debug)]
pub enum TomlError {
    UnexpectedCharacter(usize),
    UnclosedScope(usize),
    UnexpectedLinebreak(usize),
    EmptyScope(usize),
    InvalidKeyChar { start: usize, invalid: char, index: usize },
    MissingScopeSeparator { start: usize, missing: usize },
}

#[derive(Debug)]
pub enum Scope {
    Table { text: String, scope: Vec<String> },
    TableArray { text: String, scope: Vec<String> },
}