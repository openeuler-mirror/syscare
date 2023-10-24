use std::path::Path;

const ENV_VERSION_NAME: &str = "SYSCARE_VERSION";

const UPATCH_TOOL_LIB: &str = "../upatch/upatch-tool";

fn main() {
    if let Ok(value) = std::env::var(ENV_VERSION_NAME) {
        println!("cargo:rustc-env=CARGO_PKG_VERSION={}", value.to_lowercase());
    }

    cc::Build::new()
        .files(&[
            Path::new(UPATCH_TOOL_LIB).join("upatch-common.c"),
            Path::new(UPATCH_TOOL_LIB).join("upatch-elf.c"),
            Path::new(UPATCH_TOOL_LIB).join("upatch-ioctl.c"),
            Path::new(UPATCH_TOOL_LIB).join("upatch-meta.c"),
            Path::new(UPATCH_TOOL_LIB).join("upatch-resolve.c"),
            Path::new(UPATCH_TOOL_LIB).join("upatch-tool-lib.c"),
        ])
        .includes(&[UPATCH_TOOL_LIB])
        .compile("upatch-tool");
}
