extern crate syntex;
extern crate serde_codegen;

use std::env;
use std::path::Path;



pub fn main() {
    let out_dir = env::var_os("OUT_DIR").unwrap();

    let mut registry = syntex::Registry::new();
    serde_codegen::register(&mut registry);
    registry.expand("", &Path::new("src/config.in.rs"), &Path::new(&out_dir).join("config.rs")).unwrap();

    let mut registry = syntex::Registry::new();
    serde_codegen::register(&mut registry);
    registry.expand("", &Path::new("src/input/random.in.rs"), &Path::new(&out_dir).join("input.random.rs")).unwrap();
}
