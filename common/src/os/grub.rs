use std::collections::HashMap;
use std::ffi::{OsString, OsStr};
use std::path::{PathBuf, Path};
use std::io::{BufRead, BufReader};
use std::os::unix::prelude::OsStrExt as UnixOsStrExt;

use lazy_static::lazy_static;
use log::debug;
use regex::bytes::Regex;

use crate::util::fs;
use crate::util::raw_line::RawLines;
use crate::util::os_str::OsStrExt;

use super::{disk, mounts};

#[derive(Debug)]
enum BootType {
    CSM,
    UEFI,
}

#[derive(Debug)]
pub struct GrubMenuEntry {
    name:   OsString,
    root:   PathBuf,
    kernel: PathBuf,
    initrd: PathBuf,
}

impl GrubMenuEntry {
    pub fn get_name(&self) -> &OsStr {
        &self.name
    }

    pub fn get_root(&self) -> &Path {
        &self.root
    }

    pub fn get_kernel(&self) -> PathBuf {
        // Path is stripped by regular expression, thus, it would always start with '/'
        self.root.join(self.kernel.strip_prefix("/").unwrap())
    }

    pub fn get_initrd(&self) -> PathBuf {
        // Path is stripped by regular expression, thus, it would always start with '/'
        self.root.join(self.initrd.strip_prefix("/").unwrap())
    }
}

struct GrubConfigParser<R> {
    lines:        RawLines<R>,
    is_matching:  bool,
    entry_name:   Option<OsString>,
    entry_root:   Option<PathBuf>,
    entry_kernel: Option<PathBuf>,
    entry_initrd: Option<PathBuf>,
}

impl<R: BufRead> GrubConfigParser<R> {
    pub fn new(reader: R) -> Self {
        Self {
            lines:        RawLines::from(reader),
            is_matching:  false,
            entry_name:   None,
            entry_root:   None,
            entry_kernel: None,
            entry_initrd: None,
        }
    }

    #[inline(always)]
    fn parse_name(str: &OsStr) -> Option<OsString> {
        lazy_static! {
            static ref RE: Regex = Regex::new(r"'([^']*)'").unwrap();
        }
        RE.captures(str.as_bytes())
            .and_then(|captures| captures.get(1))
            .map(|matched| {
                OsStr::from_bytes(matched.as_bytes()).to_os_string()
            })
    }

    #[inline(always)]
    fn parse_uuid(str: &OsStr) -> Option<OsString> {
        lazy_static! {
            static ref RE: Regex = Regex::new(r"[a-f\d]{4}(?:[a-f\d]{4}-){4}[a-f\d]{12}").unwrap();
        }
        RE.find(str.as_bytes()).map(|matched| {
            OsStr::from_bytes(matched.as_bytes()).to_os_string()
        })
    }

    #[inline(always)]
    fn parse_path(str: &OsStr) -> Option<PathBuf> {
        lazy_static! {
            static ref RE: Regex = Regex::new(r"/\.?\w+([\w\-\.])*").unwrap();
        }
        RE.find(str.as_bytes()).map(|matched| {
            PathBuf::from(OsStr::from_bytes(matched.as_bytes()))
        })
    }

    #[inline(always)]
    fn parse_mount_point(str: &OsStr) -> Option<PathBuf> {
        let find_dev = Self::parse_uuid(str).and_then(|uuid| {
            disk::find_by_uuid(uuid).ok()
        });

        if let (Some(dev_name), Ok(mounts)) = (find_dev, mounts::enumerate()) {
            for mount in mounts {
                if mount.source == dev_name {
                    return Some(mount.mount_point);
                }
            }
        }
        return None;
    }
}

impl<R: BufRead> Iterator for GrubConfigParser<R> {
    type Item = GrubMenuEntry;

    fn next(&mut self) -> Option<Self::Item> {
        for read_line in &mut self.lines {
            if let Ok(line) = read_line {
                if line.starts_with("#") {
                    continue;
                }

                let str = line.trim();
                if str.is_empty() {
                    continue;
                }

                if !self.is_matching {
                    if str.starts_with("menuentry '") {
                        self.entry_name  = Self::parse_name(str);
                        self.is_matching = true;
                    }
                    continue;
                }
                if str.starts_with("search") {
                    self.entry_root = Self::parse_mount_point(str);
                }
                else if str.starts_with("linux") {
                    self.entry_kernel = Self::parse_path(str);
                }
                else if str.starts_with("initrd") {
                    self.entry_initrd = Self::parse_path(str);
                }
                else if str.starts_with("}") {
                    let entry = match (&self.entry_name, &self.entry_root, &self.entry_kernel, &self.entry_initrd) {
                        (Some(name), Some(root), Some(kernel), Some(initrd)) => {
                            Some(GrubMenuEntry {
                                name:   name.to_os_string(),
                                root:   root.to_path_buf(),
                                kernel: kernel.to_path_buf(),
                                initrd: initrd.to_path_buf(),
                            })
                        },
                        _ => None
                    };
                    self.is_matching  = false;
                    self.entry_name   = None;
                    self.entry_root   = None;
                    self.entry_kernel = None;
                    self.entry_initrd = None;

                    return entry;
                }
            }
        }
        return None;
    }
}

struct GrubEnvParser<R> {
    lines: RawLines<R>
}

impl<R: BufRead> GrubEnvParser<R> {
    pub fn new(reader: R) -> Self {
        Self {
            lines: RawLines::from(reader),
        }
    }
}

impl<R: BufRead> Iterator for GrubEnvParser<R> {
    type Item = (OsString, OsString);

    fn next(&mut self) -> Option<Self::Item> {
        for read_line in &mut self.lines {
            if let Ok(line) = read_line {
                if line.starts_with("#") {
                    continue;
                }

                let str = line.trim();
                if str.is_empty() {
                    continue;
                }

                let mut kv = line.split('=');
                if let (Some(key), Some(value)) = (kv.next(), kv.next()) {
                    return Some((key.trim().to_os_string(), value.trim().to_os_string()));
                }

                continue;
            }
        }
        return None;
    }
}

#[inline(always)]
fn get_boot_type() -> BootType {
    const UEFI_SYS_INTERFACE: &str = "/sys/firmware/efi";

    match fs::metadata(UEFI_SYS_INTERFACE) {
        Ok(_)  => BootType::UEFI,
        Err(_) => BootType::CSM,
    }
}

fn get_grub_path() -> PathBuf {
    const CSM_GRUB_PATH:  &str = "/boot/grub2";
    const UEFI_GRUB_PATH: &str = "/boot/efi/EFI";

    let boot_type = get_boot_type();
    debug!("Boot type: {:?}", boot_type);

    match boot_type {
        BootType::CSM  => PathBuf::from(CSM_GRUB_PATH),
        BootType::UEFI => PathBuf::from(UEFI_GRUB_PATH),
    }
}

fn find_grub_config<P: AsRef<Path>>(grub_root: P) -> std::io::Result<PathBuf> {
    const GRUB_CFG_NAME: &str = "grub.cfg";

    fs::find_file(
        grub_root,
        GRUB_CFG_NAME,
        fs::FindOptions { fuzz: false, recursive: true }
    )
}

fn find_grub_env<P: AsRef<Path>>(grub_root: P) -> std::io::Result<PathBuf> {
    const GRUB_ENV_NAME: &str = "grubenv";

    fs::find_file(
        grub_root,
        GRUB_ENV_NAME,
        fs::FindOptions { fuzz: false, recursive: true }
    )
}

pub fn read_menu_entries() -> std::io::Result<Vec<GrubMenuEntry>> {
    let grub_root   = get_grub_path();
    let grub_config = find_grub_config(grub_root)?;

    let result = GrubConfigParser::new(
        BufReader::new(fs::open_file(grub_config)?)
    ).collect();

    Ok(result)
}

pub fn read_grub_env() -> std::io::Result<HashMap<OsString, OsString>> {
    let grub_root = get_grub_path();
    let grub_env  = find_grub_env(grub_root).unwrap();

    let result = GrubEnvParser::new(
        BufReader::new(fs::open_file(grub_env)?)
    ).collect();

    Ok(result)
}

pub fn get_boot_entry() -> std::io::Result<GrubMenuEntry> {
    let config = read_grub_env()?;
    let default_entry_name = config.get(OsStr::new("saved_entry")).ok_or(std::io::Error::new(
        std::io::ErrorKind::Other,
        "Cannot read grub default entry name",
    ))?;

    for entry in read_menu_entries()? {
        if entry.get_name() == default_entry_name {
            return Ok(entry);
        }
    }

    return Err(std::io::Error::new(
        std::io::ErrorKind::Other,
        format!("Cannot find grub default entry \"{}\"", default_entry_name.to_string_lossy())
    ));
}
