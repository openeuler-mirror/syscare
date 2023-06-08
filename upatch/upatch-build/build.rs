const ENV_VERSION: &str = "UPATCH_VERSION";

fn main() {
    if let Ok(value) = std::env::var(ENV_VERSION) {
        if !value.is_empty() {
            println!("cargo:rustc-env=CARGO_PKG_VERSION={}", value);
        }
    }
}
