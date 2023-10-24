extern crate dunce;
use std::{env, path::PathBuf};
const ENV_VERSION_NAME: &str = "SYSCARE_VERSION";

fn main() {
    if let Ok(value) = std::env::var(ENV_VERSION_NAME) {
        let library_name = "upatch-tool-lib";
        let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
        let library_dir = dunce::canonicalize(root.join("../build/upatch/upatch-tool")).unwrap();
        println!("cargo:rust-link-lib=static={}", library_name);
        println!("cargo:rustc-link-search=native={}",env::join_paths(&[library_dir]).unwrap().to_str().unwrap());
        if value.is_empty() {
            return;
        }
        println!("cargo:rustc-env=CARGO_PKG_VERSION={}", value.to_lowercase());
    }
}
