use std::collections::HashMap;

use serde::{Serialize, Deserialize};

use super::patch_type::PatchType;

pub const PATCH_INFO_KEY_NAME:        &str = "name";
pub const PATCH_INFO_KEY_TYPE:        &str = "type";
pub const PATCH_INFO_KEY_ARCH:        &str = "arch";
pub const PATCH_INFO_KEY_TARGET:      &str = "target";
pub const PATCH_INFO_KEY_ELF_NAME:    &str = "elf_name";
pub const PATCH_INFO_KEY_LICENSE:     &str = "license";
pub const PATCH_INFO_KEY_VERSION:     &str = "version";
pub const PATCH_INFO_KEY_RELEASE:     &str = "release";
pub const PATCH_INFO_KEY_DESCRIPTION: &str = "description";
pub const PATCH_INFO_KEY_BUILDER:     &str = "builder";

#[derive(Debug)]
#[derive(Serialize, Deserialize)]
pub struct PatchInfo {
    name:        String,
    kind:        PatchType,
    arch:        String,
    target:      String,
    elf_name:    String,
    license:     String,
    version:     String,
    release:     String,
    description: String,
    builder:     String,
    patch_list:  Vec<String>,
}

impl PatchInfo {
    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn get_type(&self) -> &PatchType {
        &self.kind
    }

    pub fn get_arch(&self) -> &str {
        &self.arch
    }

    pub fn get_target(&self) -> &str {
        &self.target
    }

    pub fn get_elf_name(&self) -> &str {
        &self.elf_name
    }

    pub fn get_license(&self) -> &str {
        &self.license
    }

    pub fn get_version(&self) -> &str {
        &self.version
    }

    pub fn get_release(&self) -> &str {
        &self.release
    }

    pub fn get_description(&self) -> &str {
        &self.description
    }

    pub fn get_builder(&self) -> &str {
        &self.builder
    }

    pub fn get_patch_list(&self) -> &[String] {
        &self.patch_list
    }
}

impl std::str::FromStr for PatchInfo {
    type Err = std::io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut info_map = HashMap::new();
        let mut additional_info = Vec::new();
        let lines = s.split('\n').collect::<Vec<&str>>();
        for line in lines {
            if line.is_empty() {
                continue;
            }

            let record = line.split(':').collect::<Vec<&str>>();
            if record.len() != 2 {
                additional_info.push(line.trim());
                continue;
            }

            let k = record[0].trim();
            let v = record[1].trim();
            match v.is_empty() {
                false => {
                    info_map.insert(k, v);
                },
                true => {
                    additional_info.push(line.trim());
                }
            }
        }

        let parse_key = |k: &str| -> std::io::Result<String> {
            match info_map.get(k) {
                Some(v) => Ok(v.to_string()),
                None => Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("cannot find property \"{}\"", k)
                ))
            }
        };

        Ok(Self {
            name:        parse_key(PATCH_INFO_KEY_NAME)?,
            kind:        parse_key(PATCH_INFO_KEY_TYPE)?.parse()?,
            arch:        parse_key(PATCH_INFO_KEY_ARCH)?,
            version:     parse_key(PATCH_INFO_KEY_VERSION)?,
            release:     parse_key(PATCH_INFO_KEY_RELEASE)?,
            target:      parse_key(PATCH_INFO_KEY_TARGET)?,
            elf_name:    parse_key(PATCH_INFO_KEY_ELF_NAME)?,
            license:     parse_key(PATCH_INFO_KEY_LICENSE)?,
            description: parse_key(PATCH_INFO_KEY_DESCRIPTION)?,
            builder:     parse_key(PATCH_INFO_KEY_BUILDER)?,
            patch_list:  additional_info.into_iter().map(str::to_owned).collect()
        })
    }
}

impl std::fmt::Display for PatchInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:<12}: {}\n", PATCH_INFO_KEY_NAME,        self.get_name()))?;
        f.write_fmt(format_args!("{:<12}: {}\n", PATCH_INFO_KEY_TYPE,        self.get_type()))?;
        f.write_fmt(format_args!("{:<12}: {}\n", PATCH_INFO_KEY_ARCH,        self.get_arch()))?;
        f.write_fmt(format_args!("{:<12}: {}\n", PATCH_INFO_KEY_VERSION,     self.get_version()))?;
        f.write_fmt(format_args!("{:<12}: {}\n", PATCH_INFO_KEY_RELEASE,     self.get_release()))?;
        f.write_fmt(format_args!("{:<12}: {}\n", PATCH_INFO_KEY_TARGET,      self.get_target()))?;
        f.write_fmt(format_args!("{:<12}: {}\n", PATCH_INFO_KEY_ELF_NAME,    self.get_elf_name()))?;
        f.write_fmt(format_args!("{:<12}: {}\n", PATCH_INFO_KEY_LICENSE,     self.get_license()))?;
        f.write_fmt(format_args!("{:<12}: {}\n", PATCH_INFO_KEY_DESCRIPTION, self.get_description()))?;
        f.write_fmt(format_args!("{:<12}: {}\n", PATCH_INFO_KEY_BUILDER,     self.get_builder()))?;
        for line in &self.patch_list {
            f.write_fmt(format_args!("\n{}", line))?;
        }
        Ok(())
    }
}
