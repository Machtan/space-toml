#![allow(unused)]
#![feature(question_mark)]
#![feature(plugin)]
#![plugin(phf_macros)]

extern crate phf;
extern crate chrono;

use std::io::Read;
use std::fs::File;

mod lexer;
mod structure;
mod parser;

use lexer::Lexer;

fn test_lexer(text: &str, verbose: bool) {
    let mut out = String::new();
    let mut lexer = Lexer::new(text);
    if verbose {
        print!("{:03}:{:03} : ", lexer.current_position().0, lexer.current_position().1);
    }
    while let Some(res) = lexer.next() {
        match res {
            Ok(token) => {
                if verbose {
                    println!("{:?}", token);
                }
                out.push_str(token.as_str());
            }
            Err(err) => {
                return err.show(text);
            }
        }
        if verbose {
            print!("{:03}:{:03} : ", lexer.current_position().0, lexer.current_position().1);
        }
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
    
    [ bob. "something" ]
    flt7 = 6.626e-34
    bool = false # it's true though!
    arr = [ 1, 2   ,    3 
    ,]
    
    inline = { alice = "some", key = 2 }
    "#;
    test_lexer(simple, false);
    
    let mut hard_file = File::open("samples/hard_example.toml")
        .expect("Sample file not found");
    let mut hard_example = String::new();
    hard_file.read_to_string(&mut hard_example)
        .expect("Could not read the sample");
    test_lexer(&hard_example, false);
    
    let mut hard_file = File::open("samples/hard_example_unicode.toml")
        .expect("Sample file not found");
    let mut hard_example = String::new();
    hard_file.read_to_string(&mut hard_example)
        .expect("Could not read the sample");
    test_lexer(&hard_example, false);
    
    println!("Done!");
}
