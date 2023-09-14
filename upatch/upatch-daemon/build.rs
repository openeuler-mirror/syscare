use std::path::Path;

const ENV_VERSION_NAME: &str = "SYSCARE_VERSION";
const UPATCH_LIB: &str = "../upatch-compile/lib";
const UPATCH_COMMON: &str = "../upatch-compile/common";

fn main() {
    if let Ok(value) = std::env::var(ENV_VERSION_NAME) {
        if value.is_empty() {
            return;
        }
        println!("cargo:rustc-env=CARGO_PKG_VERSION={}", value.to_lowercase());
    }
    cc::Build::new()
        .file(Path::new(UPATCH_LIB).join("upatch.c"))
        .includes(&[UPATCH_COMMON, UPATCH_LIB])
        .compile("libupatch.a");
}
