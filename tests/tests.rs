extern crate space_toml;
extern crate rustc_serialize;

use space_toml::{Table, Value};
use std::collections::BTreeMap;
use rustc_serialize::json::Json;

pub fn assert_data_preserved_on_lex(text: &str, verbose: bool) {
    let mut out = String::new();
    let mut tokens = space_toml::tokens(text);
    
    while let Some(res) = tokens.next() {
        let (pos, token) = res.expect("Lexing failed");
        if verbose {
            println!("{:?}: {:?}", pos, token);
        }
        token.write(&mut out);
    }
    assert!(text == out,
            "======== expected =======\n{}\n======= got =======\n{}\n",
            text,
            out);
}

pub fn assert_can_parse(text: &str) {
    space_toml::parse(text).expect("Parsing failed");
}

pub fn assert_format_preserved_on_write(text: &str) {
    let table = space_toml::parse(text).expect("Parsing failed");
    let mut out = String::new();
    table.write(&mut out);
    assert!(text == out,
            "======== expected =======\n{}\n======= got =======\n{}\n",
            text,
            out);
}

pub fn assert_can_lex(text: &str, verbose: bool) {
    let mut tokens = space_toml::tokens(text);
    while let Some(res) = tokens.next() {
        let (pos, token) = res.expect("Lexing failed");
        if verbose {
            println!("{:?}: {:?}", pos, token);
        }
    }
}

pub fn to_json(toml: &Value) -> Json {
    use space_toml::Value::*;    
    fn doit(s: &str, json: Json) -> Json {
        let mut map = BTreeMap::new();
        map.insert(format!("{}", "type"), Json::String(format!("{}", s)));
        map.insert(format!("{}", "value"), json);
        Json::Object(map)
    }
    match *toml {
        Value::String(ref s) => doit("string", Json::String(s.clean().to_string())),
        Int(ref i) => doit("integer", Json::String(format!("{}", i.value()))),
        Float(ref f) => doit("float", Json::String({
            let s = format!("{:.15}", f.value());
            let s = format!("{}", s.trim_right_matches('0'));
            if s.ends_with(".") {format!("{}0", s)} else {s}
        })),
        Bool(ref b) => doit("bool", Json::String(format!("{}", b))),
        DateTime(ref s) => doit("datetime", Json::String(s.to_string())),
        Array(ref arr) => {
            let is_table = match arr.iter().next() {
                Some(&Table(..)) => true,
                _ => false,
            };
            let json = Json::Array(arr.iter().map(to_json).collect());
            if is_table {json} else {doit("array", json)}
        }
        Table(ref table) => Json::Object(table.iter().map(|(k, v)| {
            (k.to_string(), to_json(v))
        }).collect()),
    }
}

pub fn serialize_json(table: &Table) -> Json {
    //let mut scope = Vec::new();
    let mut tree = BTreeMap::new();
    for (k, v) in table.iter() {
        tree.insert(k.to_string(), to_json(v));
    }
    Json::Object(tree)
}

pub fn compare_output(toml: &str, json: &str) {
    let table = match space_toml::parse(toml) {
        Ok(table) => table,
        Err(e) => {
            println!("Parsing failed:");
            println!("{:?}", e);
            println!("{}", e);
            panic!("");
        }
    };
    let json = Json::from_str(json).expect("JSON parsing failed");
    let toml_json = serialize_json(&table);
    assert!(json == toml_json,
            "expected\n{}\ngot\n{}\n",
            json.pretty(),
            toml_json.pretty());
}

macro_rules! simple_tests {
    ( $module:ident: $source:expr ) => {
        pub mod $module {
            use super::{assert_can_lex, assert_format_preserved_on_write, assert_can_parse, assert_data_preserved_on_lex};
            #[test]
            fn can_lex() {
                assert_can_lex($source, true);
            }

            #[test]
            fn lexer_preserves_format() {
                assert_data_preserved_on_lex($source, true);
            }

            #[test]
            fn can_parse() {
                assert_can_parse($source);
            }

            #[test]
            fn parser_preserves_format() {
                assert_format_preserved_on_write($source);
            }
        }
    }
}

macro_rules! test_valid {
    ( $name:ident : $toml:expr, $json:expr) => {
        pub mod $name {
            use super::{assert_can_lex, assert_format_preserved_on_write, compare_output, assert_data_preserved_on_lex, assert_can_parse};
           #[test]
            fn can_lex() {
                assert_can_lex($toml, true);
            }

            #[test]
            fn lexer_preserves_format() {
                assert_data_preserved_on_lex($toml, true);
            }

            #[test]
            fn can_parse() {
                assert_can_parse($toml);
            }

            #[test]
            fn parser_preserves_format() {
                assert_format_preserved_on_write($toml);
            }

            #[test]
            fn parses_correctly() {
                compare_output($toml, $json);
            }
        }
    }
}

simple_tests!(messy: include_str!("../samples/messy.toml"));
simple_tests!(hard: include_str!("../samples/hard_example.toml"));
simple_tests!(hard_unicode: include_str!("../samples/hard_example_unicode.toml"));
simple_tests!(official: include_str!("../samples/official.toml"));
simple_tests!(example: include_str!("../samples/example.toml"));

/*pub mod valid {
    pub use super::{assert_can_lex, assert_format_preserved_on_write, compare_output, assert_data_preserved_on_lex, assert_can_parse};
    test_valid!(array_empty:
       include_str!("valid/array-empty.toml"),
       include_str!("valid/array-empty.json"));
    test_valid!(array_nospaces:
        include_str!("valid/array-nospaces.toml"),
        include_str!("valid/array-nospaces.json"));
    test_valid!(arrays_hetergeneous:
        include_str!("valid/arrays-hetergeneous.toml"),
        include_str!("valid/arrays-hetergeneous.json"));
    test_valid!(arrays:
        include_str!("valid/arrays.toml"),
        include_str!("valid/arrays.json"));
    test_valid!(arrays_nested:
        include_str!("valid/arrays-nested.toml"),
        include_str!("valid/arrays-nested.json"));
    test_valid!(empty:
        include_str!("valid/empty.toml"),
        include_str!("valid/empty.json"));
    test_valid!(bool:
        include_str!("valid/bool.toml"),
        include_str!("valid/bool.json"));
    test_valid!(datetime:
        include_str!("valid/datetime.toml"),
        include_str!("valid/datetime.json"));
    test_valid!(example:
        include_str!("valid/example.toml"),
        include_str!("valid/example.json"));
    test_valid!(float:
        include_str!("valid/float.toml"),
        include_str!("valid/float.json"));
    test_valid!(implicit_and_explicit_after:
        include_str!("valid/implicit-and-explicit-after.toml"),
        include_str!("valid/implicit-and-explicit-after.json"));
    test_valid!(implicit_and_explicit_before:
        include_str!("valid/implicit-and-explicit-before.toml"),
        include_str!("valid/implicit-and-explicit-before.json"));
    test_valid!(implicit_groups:
        include_str!("valid/implicit-groups.toml"),
        include_str!("valid/implicit-groups.json"));
    test_valid!(integer:
        include_str!("valid/integer.toml"),
        include_str!("valid/integer.json"));
    test_valid!(key_equals_nospace:
        include_str!("valid/key-equals-nospace.toml"),
        include_str!("valid/key-equals-nospace.json"));
    test_valid!(key_space:
        include_str!("valid/key-space.toml"),
        include_str!("valid/key-space.json"));
    test_valid!(key_special_chars:
        include_str!("valid/key-special-chars.toml"),
        include_str!("valid/key-special-chars.json"));
    test_valid!(key_with_pound:
        include_str!("valid/key-with-pound.toml"),
        include_str!("valid/key-with-pound.json"));
    test_valid!(long_float:
        include_str!("valid/long-float.toml"),
        include_str!("valid/long-float.json"));
    test_valid!(long_integer:
        include_str!("valid/long-integer.toml"),
        include_str!("valid/long-integer.json"));
    test_valid!(multiline_string:
        include_str!("valid/multiline-string.toml"),
        include_str!("valid/multiline-string.json"));
    test_valid!(raw_multiline_string:
        include_str!("valid/raw-multiline-string.toml"),
        include_str!("valid/raw-multiline-string.json"));
    test_valid!(raw_string:
        include_str!("valid/raw-string.toml"),
        include_str!("valid/raw-string.json"));
    test_valid!(string_empty:
        include_str!("valid/string-empty.toml"),
        include_str!("valid/string-empty.json"));
    test_valid!(string_escapes:
        include_str!("valid/string-escapes.toml"),
        include_str!("valid/string-escapes.json"));
    test_valid!(string_simple:
        include_str!("valid/string-simple.toml"),
        include_str!("valid/string-simple.json"));
    test_valid!(string_with_pound:
        include_str!("valid/string-with-pound.toml"),
        include_str!("valid/string-with-pound.json"));
    test_valid!(table_array_implicit:
        include_str!("valid/table-array-implicit.toml"),
        include_str!("valid/table-array-implicit.json"));
    test_valid!(table_array_many:
        include_str!("valid/table-array-many.toml"),
        include_str!("valid/table-array-many.json"));
    test_valid!(table_array_nest:
        include_str!("valid/table-array-nest.toml"),
        include_str!("valid/table-array-nest.json"));
    test_valid!(table_array_one:
        include_str!("valid/table-array-one.toml"),
        include_str!("valid/table-array-one.json"));
    test_valid!(table_empty:
        include_str!("valid/table-empty.toml"),
        include_str!("valid/table-empty.json"));
    test_valid!(table_sub_empty:
        include_str!("valid/table-sub-empty.toml"),
        include_str!("valid/table-sub-empty.json"));
    test_valid!(table_whitespace:
        include_str!("valid/table-whitespace.toml"),
        include_str!("valid/table-whitespace.json"));
    test_valid!(table_with_pound:
        include_str!("valid/table-with-pound.toml"),
        include_str!("valid/table-with-pound.json"));
    test_valid!(unicode_escape:
        include_str!("valid/unicode-escape.toml"),
        include_str!("valid/unicode-escape.json"));
    test_valid!(unicode_literal:
        include_str!("valid/unicode-literal.toml"),
        include_str!("valid/unicode-literal.json"));
    test_valid!(hard_example:
        include_str!("valid/hard_example.toml"),
        include_str!("valid/hard_example.json"));
    test_valid!(example2:
        include_str!("valid/example2.toml"),
        include_str!("valid/example2.json"));
    test_valid!(example3:
        include_str!("valid/example-v0.3.0.toml"),
        include_str!("valid/example-v0.3.0.json"));
    test_valid!(example4:
        include_str!("valid/example-v0.4.0.toml"),
        include_str!("valid/example-v0.4.0.json"));
    test_valid!(example_bom:
        include_str!("valid/example-bom.toml"),
        include_str!("valid/example.json"));
}*/

