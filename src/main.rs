#![allow(unused)]
#![feature(question_mark)]
#![feature(plugin)]
#![plugin(phf_macros)]

extern crate phf;
extern crate chrono;

use std::io::Read;
use std::fs::File;

mod lexer;
use lexer::Lexer;

fn main() {
    use lexer::Token::*;
    println!("T O M L !");
    /*let mut file = File::open("samples/official.toml")
        .expect("Sample file not found");
    let mut source = String::new();
    file.read_to_string(&mut source)
        .expect("Could not read the sample");*/
    //let toml = read_toml(&source);
    //println!("Toml:\n{:?}", toml);
    //println!("Scope: {:?}", read_scope("[hello]", 0));
    let simple = r#"
    [ hello  ]
    a = 2
    b = "hello world"
    "#;
    let mut lexer = Lexer::new(simple);
    print!("{:?}: ", lexer.current_position());
    while let Some(res) = lexer.next() {
        match res {
            Ok(token) => {
                println!("Token: {:?}", token);
            }
            Err(err) => {
                println!("Parse error: {:?}", err);
            }
        }
        print!("{:?}: ", lexer.current_position());
    }
    println!("Done!");
}
