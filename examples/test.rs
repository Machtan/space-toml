extern crate space_toml;
extern crate env_logger;
use space_toml::{Value};

const SOURCE: &'static str = r##"
[clients]
data = [ ["gamma", "delta"], [1, 2] ] # just an update to make sure parsers support it

# Line breaks are OK when inside arrays
hosts = [
  "alpha",
  "omega"
]

# Products

  [[products]]
  name = "Hammer"
  sku = 738594937

  [[products]]
  name = "Nail"
  sku = 284758393
  color = "gray"
  multi-line_array = [
      1,
      2,


  ]
  multi2 = [
      "]",
  #comment here
]
"##;

fn main() {
    env_logger::init().unwrap();
    let table = space_toml::parse(SOURCE).expect("Could not parse TOML");
    println!("Items:");
    for (k, v) in table.iter() {
        println!("{} = {:?}", k.to_string(), v);
    }
    if let Some(&Value::Array(ref arr)) = table.get("products") {
        println!("======= Products =======");
        println!("{:?}", arr);
        let mut string = String::new();
        arr.write(&mut string);
        println!("======== Written =======");
        println!("{}", string);
    }
    println!("======= Output =======");
    let mut output = String::new();
    table.write(&mut output);
    println!("{}", output);

    assert!(
        output == SOURCE,
         "======== expected =======\n{}\n======= got =======\n{}\n",
        SOURCE,
        output);
}