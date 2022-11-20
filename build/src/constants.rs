use crate::cmd::ExternCommand;

// CLI defaults
pub const CLI_DEFAULT_PATCH_VERSION:       &str = "1";
pub const CLI_DEFAULT_PATCH_SUMMARY:       &str = "syscare patch";
pub const CLI_DEFAULT_WORK_DIR:            &str = ".";
pub const CLI_DEFAULT_OUTPUT_DIR:          &str = ".";
pub const CLI_DEFAULT_SKIP_COMPILER_CHECK: &str = "false";

// External commands
pub const MAKE:         ExternCommand = ExternCommand::new("make");
pub const RPM:          ExternCommand = ExternCommand::new("rpm");
pub const RPM_BUILD:    ExternCommand = ExternCommand::new("rpmbuild");
pub const KPATCH_BUILD: ExternCommand = ExternCommand::new("kpatch-build");
pub const UPATCH_BUILD: ExternCommand = ExternCommand::new("/usr/libexec/syscare/upatch-build");

// Patch
pub const PATCH_FILE_EXTENSION:  &str  = "patch";
pub const PATCH_FILE_PERMISSION: &str  = "640";
pub const PATCH_DIR_PERMISSION:  &str  = "750";
pub const PATCH_INSTALL_PATH:    &str  = "/usr/lib/syscare/patches";
pub const PATCH_INFO_FILE_NAME:  &str  = "patch_info";
pub const PATCH_VERSION_DIGITS:  usize = 8;

// Package
pub const PKG_BUILD_ROOT_DIR_NAME:          &str = "rpmbuild";
pub const PKG_FILE_EXTENSION:               &str = "rpm";
pub const PKG_FLAG_PATCH_PKG:               &str = "patch";
pub const PKG_FLAG_SOURCE_PKG:              &str = "(none)";
pub const PKG_PATCH_VERSION_FILE_NAME:      &str = "syscare-patch-version";
pub const PKG_PATCH_TARGET_FILE_NAME:       &str = "syscare-patch-target";
pub const PKG_SPEC_FILE_EXTENSION:          &str = "spec";
pub const PKG_SPEC_TAG_NAME_RELEASE:        &str = "Release:";
pub const PKG_SPEC_TAG_NAME_SOURCE:         &str = "Source";
pub const PKG_SPEC_TAG_NAME_BUILD_REQUIRES: &str = "BuildRequires:";
pub const PKG_SPEC_TAG_VALUE_GROUP:         &str = "Patch";

// Kernel
pub const KERNEL_PKG_NAME:          &str = "kernel";
pub const KERNEL_SOURCE_DIR_PREFIX: &str = "linux-";
pub const KERNEL_CONFIG_NAME:       &str = ".config";
pub const KERNEL_DEFCONFIG_NAME:    &str = "openeuler_defconfig";
pub const KERNEL_ELF_NAME:          &str = "vmlinux";
