use std::{env, process::Command};

fn rewrite_version() {
    const ENV_VERSION_NAME: &str = "BUILD_VERSION";
    const PKG_VERSION_NAME: &str = "CARGO_PKG_VERSION";

    let version = env::var(ENV_VERSION_NAME).unwrap_or_else(|_| {
        let pkg_version = env::var(PKG_VERSION_NAME).expect("Failed to fetch package version");
        let git_output = Command::new("git")
            .args(["rev-parse", "--short", "HEAD"])
            .output()
            .map(|output| String::from_utf8(output.stdout).expect("Failed to fetch git version"));

        match git_output {
            Ok(git_version) => format!("v{}-g{}", pkg_version, git_version),
            Err(_) => format!("v{}", pkg_version),
        }
    });

    println!("cargo:rustc-env={}={}", PKG_VERSION_NAME, version);
}

fn main() {
    rewrite_version();
}
