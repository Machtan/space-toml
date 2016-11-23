extern crate space_toml;

pub fn assert_format_preserved_on_write(text: &str, verbose: bool) {
    let mut out = String::new();
    let mut tokens = space_toml::tokens(text);
    
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
}

pub fn assert_can_lex(text: &str, verbose: bool) {
    let mut tokens = space_toml::tokens(text);
    while let Some(res) = tokens.next() {
        match res {
            Ok((pos, token)) => {
                if verbose {
                    println!("{:?}: {:?}", pos, token);
                }
            }
            Err(err) => {
                err.show(text);
                panic!("Lexing failed");
            }
        }
    }
} 

macro_rules! simple_tests {
    ( $module:ident: $source:ident ) => {
        pub mod $module {
            use super::{$source, assert_can_lex, assert_format_preserved_on_write};
            #[test]
            fn can_lex() {
                assert_can_lex($source, true);
            }

            #[test]
            fn can_preserve() {
                assert_format_preserved_on_write($source, true);
            }
        }
    }
}

pub const MESSY: &'static str = include_str!("../samples/messy.toml");
pub const HARD: &'static str = include_str!("../samples/hard_example.toml");
pub const HARD_UNICODE: &'static str = include_str!("../samples/hard_example_unicode.toml");
pub const OFFICIAL: &'static str = include_str!("../samples/official.toml");
pub const EXAMPLE: &'static str = include_str!("../samples/example.toml");


simple_tests!(messy: MESSY);
simple_tests!(hard: HARD);
simple_tests!(hard_unicode: HARD_UNICODE);
simple_tests!(official: OFFICIAL);
simple_tests!(example: EXAMPLE);

