extern crate space_toml;

use space_toml::{Table};

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
    
    inline = { alice = "some", key = 2 }
    
    multi = """\
    Hello World!
    This is \"multiline\"!\
    """
    
    date1 = 1979-05-27T07:32:00Z
    date2 = 1979-05-27T00:32:00-07:00
    date3 = 1979-05-27T00:32:00.999999-07:00
    
    "#;
    
    match space_toml::parse(simple) {
        Ok(mut table) => {
            println!("Yay!");
            println!("Written output:");
            let mut out = String::new();
            table.write(&mut out);
            println!("{}", out);
            assert_eq!(simple, &out);
            println!("Parsed table written and validated!");
            
            table.find_or_insert_with(&["hello"], || Table::new_regular())
                .expect("Could not find table 'hello'")
                .table_mut()
                .unwrap()
                .insert("test", "value");
            
            table.find_or_insert_table(&["bob", "something"])
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
            println!("{}", err);
        }
    }
    
    println!("~~~ Done! ~~~");
}
