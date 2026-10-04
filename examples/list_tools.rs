use ralph_rs::tools::Registry;
use std::path::PathBuf;

fn main() {
    let reg = Registry::standard(PathBuf::from(".").into());
    for def in reg.definitions() {
        println!("--- {} ---", def.name);
        println!("{}", def.description);
        println!(
            "{}\n",
            serde_json::to_string_pretty(&def.input_schema).unwrap()
        );
    }
}
