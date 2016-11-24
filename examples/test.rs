extern crate space_toml;

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
"##;

fn main() {
    let table = space_toml::parse(SOURCE).expect("Could not parse TOML");
    println!("Items:");
    for (k, v) in table.iter() {
        println!("{} = {:?}", k.to_string(), v);
    }
    println!("Products: {:?}", table.get("products"));
}