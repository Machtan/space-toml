extern crate space_toml;

use space_toml::{Table};
use std::env;
use std::process;
use std::io::{Read};
use std::fs::File;

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() < 1 {
        println!("Usage: cargo run --example cargo -- [path/to/Cargo.toml]");
        process::exit(1);
    }
    let ref path = args[0];
    let mut file = File::open(path).expect("Could not open file");
    let mut source = String::new();
    file.read_to_string(&mut source).expect("Could not read file");
    let mut table = space_toml::parse(&source).expect("Could not read TOML");
    {
        // Ensure that we don't borrow the table for too long
        let mut dependencies = table.find_or_insert_table(&["dependencies"]).expect("Invalid file structure");
        let mut dep = Table::new_inline();
        dep.insert("version", env!("CARGO_PKG_VERSION"));
        dependencies.insert("space_toml", dep);
        
        let mut dep = Table::new_inline();
        dep.insert("path", "../rsdl2");
        dependencies.insert("rsdl2", dep);
    }

    let mut output = String::new();
    table.write(&mut output);
    println!("~~~ Modified ~~~");
    println!("{}", output);
}