const ENV_VERSION_NAME: &str = "SYSCARE_BUILD_VERSION";

fn main() {
    if let Ok(value) = std::env::var(ENV_VERSION_NAME) {
        println!("cargo:rustc-env=CARGO_PKG_VERSION={}", value);
    }
}
