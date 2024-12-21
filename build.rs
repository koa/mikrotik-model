use mikrotik_model_generator::generator;
use quote::ToTokens;
use std::path::Path;
use std::{env, fs};

fn main() {
    let file = generator();
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("mikrotik-model.rs");
    fs::write(dest_path, prettyplease::unparse(&file)).unwrap();
}
