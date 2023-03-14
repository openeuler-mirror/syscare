const ENV_VERSION_NAME: &str = "SYSCARE_VERSION";

fn main() {
    if let Ok(value) = std::env::var(ENV_VERSION_NAME) {
        if value.is_empty() {
            return;
        }
        println!("cargo:rustc-env=CARGO_PKG_VERSION={}", value.to_lowercase());
    }
}
