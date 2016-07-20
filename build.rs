extern crate serde_codegen;

use std::env;
use std::path::Path;

pub fn main() {
    let out_dir = env::var_os("OUT_DIR").unwrap();

    serde_codegen::expand(&Path::new("src/config.in.rs"), &Path::new(&out_dir).join("config.rs")).unwrap();
}
