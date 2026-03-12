use bitflags::bitflags;

// Define Bitmasks for Physical Dimensions
bitflags! {
    pub struct Arch: u8 {
        const AARCH64 = 1 << 0;
        const X86_64  = 1 << 1;
        const RISCV64 = 1 << 2;
        const ALL     = Self::AARCH64.bits() | Self::X86_64.bits() | Self::RISCV64.bits();
    }

    pub struct CompilerFamily: u8 {
        const GNU   = 1 << 0;
        const CLANG = 1 << 1;
        const ALL   = Self::GNU.bits() | Self::CLANG.bits();
    }

    pub struct CompilerLanguage: u8 {
        const C   = 1 << 0;
        const CXX = 1 << 1;
        const ALL = Self::C.bits() | Self::CXX.bits();
    }
}

// Version structure with comparison support
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct CompilerVersion {
    major: u16,
    minor: u16,
}

impl CompilerVersion {
    pub const fn new(major: u16, minor: u16) -> Self {
        Self { major, minor }
    }

    /// Parses a version from a string like "11.2" or "9.4.1"
    /// Returns None if the format is invalid or numbers are out of u16 range.
    #[inline]
    pub fn parse_str(str: &str) -> Option<Self> {
        let mut parts = str.split('.');

        // Parse to u16, trimming any accidental whitespace
        let major = parts.next()?.trim().parse::<u16>().ok()?;
        let minor = parts.next().unwrap_or("0").trim().parse::<u16>().ok()?;

        Some(Self::new(major, minor))
    }
}

// Compiler instance representing the current environment
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Compiler {
    arch: Arch,
    family: CompilerFamily,
    language: CompilerLanguage,
    version: CompilerVersion,
}

impl Compiler {
    pub const fn new(
        arch: Arch,
        family: CompilerFamily,
        language: CompilerLanguage,
        version: CompilerVersion,
    ) -> Self {
        Self {
            arch,
            family,
            language,
            version,
        }
    }
}

// Internal structure for flag rules
struct CompileFlag {
    name: &'static str,
    archs: Arch,                 // Bitmask for supported architectures
    families: CompilerFamily,    // Bitmask for supported compiler families
    languages: CompilerLanguage, // Bitmask for supported languages
    min_version: Option<CompilerVersion>,
}

impl CompileFlag {
    /// Checks if the flag should be active for a given compiler configuration.
    /// Uses bitwise AND (intersects) for O(1) dimension checking.
    #[inline]
    fn is_active(&self, compiler: &Compiler) -> bool {
        self.archs.intersects(compiler.arch)
            && self.families.intersects(compiler.family)
            && self.languages.intersects(compiler.language)
            && self.min_version.map_or(true, |min| compiler.version >= min)
    }
}

/// Returns a lazy iterator of applicable compiler flags.
/// This implementation performs zero heap allocations.
pub fn get_compile_flags(compiler: &Compiler) -> impl Iterator<Item = &'static str> + '_ {
    const COMPILE_FLAGS: &[CompileFlag] = &[
        // Debugging and Section Generation
        CompileFlag {
            name: "-gdwarf",
            archs: Arch::ALL,
            families: CompilerFamily::ALL,
            languages: CompilerLanguage::ALL,
            min_version: None,
        },
        CompileFlag {
            name: "-ffunction-sections",
            archs: Arch::ALL,
            families: CompilerFamily::ALL,
            languages: CompilerLanguage::ALL,
            min_version: None,
        },
        CompileFlag {
            name: "-fdata-sections",
            archs: Arch::ALL,
            families: CompilerFamily::ALL,
            languages: CompilerLanguage::ALL,
            min_version: None,
        },
        // Symbol and Variable Behavior
        CompileFlag {
            name: "-fno-common",
            archs: Arch::ALL,
            families: CompilerFamily::ALL,
            languages: CompilerLanguage::ALL,
            min_version: None,
        },
        CompileFlag {
            name: "-fmerge-constants",
            archs: Arch::ALL,
            families: CompilerFamily::GNU,
            languages: CompilerLanguage::ALL,
            min_version: Some(CompilerVersion::new(4, 8)),
        },
        // Vectorization Control
        CompileFlag {
            name: "-fno-tree-slp-vectorize",
            archs: Arch::ALL,
            families: CompilerFamily::GNU,
            languages: CompilerLanguage::ALL,
            min_version: Some(CompilerVersion::new(4, 9)),
        },
        CompileFlag {
            name: "-fno-slp-vectorize",
            archs: Arch::ALL,
            families: CompilerFamily::CLANG,
            languages: CompilerLanguage::ALL,
            min_version: None,
        },
        // Clang Toolchain and Safety
        CompileFlag {
            name: "-fno-integrated-as",
            archs: Arch::ALL,
            families: CompilerFamily::CLANG,
            languages: CompilerLanguage::ALL,
            min_version: None,
        },
        CompileFlag {
            name: "-Werror=uninitialized",
            archs: Arch::ALL,
            families: CompilerFamily::CLANG,
            languages: CompilerLanguage::ALL,
            min_version: None,
        },
        // Architecture Specific: -mno-outline-atomics
        CompileFlag {
            name: "-mno-outline-atomics",
            archs: Arch::AARCH64,
            families: CompilerFamily::GNU,
            languages: CompilerLanguage::ALL,
            min_version: Some(CompilerVersion::new(9, 4)),
        },
        CompileFlag {
            name: "-mno-outline-atomics",
            archs: Arch::AARCH64,
            families: CompilerFamily::CLANG,
            languages: CompilerLanguage::ALL,
            min_version: Some(CompilerVersion::new(10, 0)),
        },
    ];

    COMPILE_FLAGS
        .iter()
        .filter(move |f| f.is_active(compiler))
        .map(|f| f.name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_parse() {
        assert_eq!(
            CompilerVersion::parse_str("11"),
            Some(CompilerVersion::new(11, 0))
        );
        assert_eq!(
            CompilerVersion::parse_str("11.2"),
            Some(CompilerVersion::new(11, 2))
        );
        assert_eq!(
            CompilerVersion::parse_str("9.4.1"),
            Some(CompilerVersion::new(9, 4))
        );
        assert_eq!(
            CompilerVersion::parse_str(" 11 . 2 "),
            Some(CompilerVersion::new(11, 2))
        );
        assert_eq!(CompilerVersion::parse_str("11."), None);
        assert_eq!(CompilerVersion::parse_str("11.invalid"), None);
        assert_eq!(CompilerVersion::parse_str("invalid"), None);
        assert_eq!(CompilerVersion::parse_str(""), None);
    }

    #[test]
    fn test_gcc_modern_aarch64() {
        // GCC 9.4 on Aarch64 should include -mno-outline-atomics and -fmerge-constants
        let compiler = Compiler::new(
            Arch::AARCH64,
            CompilerFamily::GNU,
            CompilerLanguage::C,
            CompilerVersion::new(9, 4),
        );
        let flags: Vec<_> = get_compile_flags(&compiler).collect();

        assert!(flags.contains(&"-gdwarf"));
        assert!(flags.contains(&"-ffunction-sections"));
        assert!(flags.contains(&"-fdata-sections"));
        assert!(flags.contains(&"-fmerge-constants"));
        assert!(flags.contains(&"-fno-common"));
        assert!(flags.contains(&"-fno-tree-slp-vectorize"));
        assert!(flags.contains(&"-mno-outline-atomics"));

        assert!(!flags.contains(&"-fno-slp-vectorize")); // Clang specific
        assert!(!flags.contains(&"-fno-integrated-as")); // Clang specific
        assert!(!flags.contains(&"-Werror=uninitialized")); // Clang specific
    }

    #[test]
    fn test_clang_x86_64() {
        // Clang on x86 should have Clang-specific vectorization and safety flags
        let compiler = Compiler::new(
            Arch::X86_64,
            CompilerFamily::CLANG,
            CompilerLanguage::CXX,
            CompilerVersion::new(12, 0),
        );
        let flags: Vec<_> = get_compile_flags(&compiler).collect();

        assert!(flags.contains(&"-fno-slp-vectorize"));
        assert!(flags.contains(&"-Werror=uninitialized"));
        assert!(flags.contains(&"-fno-integrated-as"));
        assert!(!flags.contains(&"-mno-outline-atomics")); // Only for AARCH64
    }

    #[test]
    fn test_old_gcc_riscv() {
        // Older GCC (e.g., 4.7) should miss flags that require 4.8 or 4.9
        let compiler = Compiler::new(
            Arch::RISCV64,
            CompilerFamily::GNU,
            CompilerLanguage::C,
            CompilerVersion::new(4, 7),
        );
        let flags: Vec<_> = get_compile_flags(&compiler).collect();

        assert!(flags.contains(&"-gdwarf"));
        assert!(!flags.contains(&"-fmerge-constants")); // Requires 4.8
        assert!(!flags.contains(&"-fno-tree-slp-vectorize")); // Requires 4.9
    }

    #[test]
    fn test_version_thresholds() {
        let arch = Arch::AARCH64;
        let fam = CompilerFamily::CLANG;
        let lang = CompilerLanguage::C;

        // Clang 9.0: Should NOT have outline-atomics
        let v9 = Compiler::new(arch, fam, lang, CompilerVersion::new(9, 0));
        assert!(!get_compile_flags(&v9).any(|f| f == "-mno-outline-atomics"));

        // Clang 10.0: Should have outline-atomics
        let v10 = Compiler::new(arch, fam, lang, CompilerVersion::new(10, 0));
        assert!(get_compile_flags(&v10).any(|f| f == "-mno-outline-atomics"));
    }
}
