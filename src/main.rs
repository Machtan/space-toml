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

fn test_lexer(text: &str) {
    let mut out = String::new();
    let mut lexer = Lexer::new(text);
    print!("{:?}: ", lexer.current_position());
    while let Some(res) = lexer.next() {
        match res {
            Ok(token) => {
                println!("Token: {:?}", token);
                out.push_str(token.as_str());
            }
            Err(err) => {
                return err.show(text);
            }
        }
        print!("{:?}: ", lexer.current_position());
    }
    assert_eq!(text, &out);
    println!("Completely Preserved!");
}

fn main() {
    println!("T O M L !");

    let simple = r#"
    [ hello  ] # lol
    a = 2#3
    b = "hello world"
    "#;
    test_lexer(simple);
    
    let mut hard_file = File::open("samples/hard_example.toml")
        .expect("Sample file not found");
    let mut hard_example = String::new();
    hard_file.read_to_string(&mut hard_example)
        .expect("Could not read the sample");
    test_lexer(&hard_example);
    
    println!("Done!");
}
