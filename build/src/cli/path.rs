use std::path::PathBuf;

#[derive(Clone)]
#[derive(Debug)]
pub enum CliPath {
    File(String),
    Directory(String),
}

impl CliPath {
    pub fn is_file(&self) -> bool {
        match self {
            CliPath::File(_)      => true,
            CliPath::Directory(_) => false,
        }
    }

    pub fn is_dir(&self) -> bool {
        match self {
            CliPath::File(_)      => false,
            CliPath::Directory(_) => true,
        }
    }
}

impl From<String> for CliPath {
    fn from(val: String) -> Self {
        match PathBuf::from(&val).is_dir() {
            true  => Self::Directory(val),
            false => Self::File(val),
        }
    }
}

impl std::fmt::Display for CliPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CliPath::File(path)      => f.write_str(path),
            CliPath::Directory(path) => f.write_str(path),
        }
    }
}
