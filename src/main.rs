#![allow(unused)]
#![feature(question_mark)]

use std::io::Read;
use std::fs::File;

mod debug;
mod tokens;
mod structure;
mod parse;

fn test_lexer(text: &str, verbose: bool) {
    let mut out = String::new();
    let mut tokens = tokens::tokens(text);
    
    while let Some(res) = tokens.next() {
        match res {
            Ok((pos, token)) => {
                if verbose {
                    println!("{:?}: {:?}", pos, token);
                }
                token.write(&mut out);
            }
            Err(err) => {
                return err.show(text);
            }
        }
    }
    assert_eq!(text, &out);
    println!("Completely Preserved!");
}

fn main() {
    println!("T O M L !");

    let simple = r#"
        
    [ hello  ] # lol
    a =    2#3
    b = "hello world"
    
    [ bob. "something" ]
    flt7 = 6.626e-34
    bool = false # it's true though!
    arr = [ 1, 2   ,    3 
    ,]
    
    #inline = { alice = "some", key = 2 }
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
    
    match parse::parse(simple) {
        Ok(mut table) => {
            println!("Yay!");
            println!("Written output:");
            let mut out = String::new();
            table.write(&mut out);
            println!("{}", out);
            assert_eq!(simple, &out);
            println!("Parsed table written and validated!");
            table.get_or_create_table(&["hello"])
                .expect("Could not find table 'hello'")
                .insert("test", "value");
            table.get_or_create_table(&["bob", "something"])
                .expect("Could not find bob.something")
                .insert("Hello snorri", "Would you care,\n for a cuppa\"\" value?");
            table.insert("What_now_Smorri", "More strings, since other values aren't implemented yet");
            table.insert("test", "This should be more indented, despite also being programatically inserted");
            let mut changed = String::new();
            table.write(&mut changed);
            println!("Changed:");
            println!("{}", changed);
        }
        Err(err) => {
            println!("Parse error:");
            err.show(simple);
        }
    }
    
    println!("~~~ Done! ~~~");
}
