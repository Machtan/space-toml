extern crate space_toml;
extern crate env_logger;
extern crate rustc_serialize;

use rustc_serialize::json::Json;
use space_toml::{Value, Table};

use std::env;
use std::process;
use std::io::Read;
use std::fs::File;
use std::collections::BTreeMap;

pub fn to_json(toml: &Value) -> Json {
    use space_toml::Value::*;
    fn doit(s: &str, json: Json) -> Json {
        let mut map = BTreeMap::new();
        map.insert(format!("{}", "type"), Json::String(format!("{}", s)));
        map.insert(format!("{}", "value"), json);
        Json::Object(map)
    }
    match *toml {
        Value::String(ref s) => {
            //println!("Converting string {:?} to JSON", s);
            doit("string", Json::String(s.clean().to_string()))
        }
        Int(ref i) => doit("integer", Json::String(format!("{}", i.value()))),
        Float(ref f) => {
            doit("float",
                 Json::String({
                     let s = format!("{:.15}", f.value());
                     let s = format!("{}", s.trim_right_matches('0'));
                     if s.ends_with(".") {
                         format!("{}0", s)
                     } else {
                         s
                     }
                 }))
        }
        Bool(ref b) => doit("bool", Json::String(format!("{}", b))),
        DateTime(ref s) => doit("datetime", Json::String(s.to_string())),
        Array(ref arr) => {
            let is_table = match arr.iter().next() {
                Some(&Table(..)) => true,
                _ => false,
            };
            let json = Json::Array(arr.iter().map(to_json).collect());
            if is_table { json } else { doit("array", json) }
        }
        Table(ref table) => {
            Json::Object(table.iter()
                .map(|(k, v)| (k.to_string(), to_json(v)))
                .collect())
        }
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

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() < 2 {
        println!("Usage: cargo run --example test -- [toml file] [json file]");
        process::exit(1);
    }
    let mut tomlfile = File::open(&args[0]).expect("Could not open TOML file");
    let mut toml = String::new();
    tomlfile.read_to_string(&mut toml).expect("Could not read TOML file");

    let mut jsonfile = File::open(&args[1]).expect("Could not open JSON file");
    let mut json = String::new();
    jsonfile.read_to_string(&mut json).expect("Could not read JSON file");

    env_logger::init().unwrap();

    let table = match space_toml::parse(&toml) {
        Ok(table) => table,
        Err(e) => {
            println!("Parsing failed:");
            println!("{:?}", e);
            println!("{}", e);
            panic!("");
        }
    };

    println!("======= Output =======");
    let mut output = String::new();
    table.write(&mut output);
    println!("{}", output);

    assert!(output == toml, "======== expected =======\n{}", toml);

    let json = Json::from_str(&json).expect("JSON parsing failed");
    let toml_json = serialize_json(&table);
    assert!(json == toml_json,
            "expected\n{}\ngot\n{}\n",
            json.pretty(),
            toml_json.pretty());

}
