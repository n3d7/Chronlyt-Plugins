use std::{env, fs, path::PathBuf};

use chronlyt_plugin_contracts::WIT_SOURCE;

fn main() {
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo must provide OUT_DIR"));
    let wit_dir = out_dir.join("wit");
    fs::create_dir_all(&wit_dir).expect("create generated WIT directory");
    fs::write(wit_dir.join("chronlyt-plugin.wit"), WIT_SOURCE)
        .expect("materialize canonical Chronlyt WIT");

    let wit_path = wit_dir
        .to_str()
        .expect("Cargo OUT_DIR must be valid UTF-8 for wit-bindgen");
    let bindings =
        format!("wit_bindgen::generate!({{ path: {wit_path:?}, world: \"chronlyt-plugin\" }});\n");
    fs::write(out_dir.join("bindings.rs"), bindings).expect("write bindings macro source");

    println!(
        "cargo:rerun-if-changed=../../crates/chronlyt-plugin-contracts/wit/chronlyt-plugin.wit"
    );
}
