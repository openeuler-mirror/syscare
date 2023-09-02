use std::path::PathBuf;

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum RpmPath {
    Directory(PathBuf),
    File(PathBuf),
}

impl std::fmt::Display for RpmPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RpmPath::Directory(path) => f.write_fmt(format_args!("%dir {}", path.display())),
            RpmPath::File(path) => f.write_fmt(format_args!("{}", path.display())),
        }
    }
}

#[test]
fn test() {
    let dir = RpmPath::Directory(PathBuf::from("/test/path"));
    println!("dir:\n{}\n", dir);
    assert_eq!(dir.to_string(), "%dir /test/path");

    let file = RpmPath::File(PathBuf::from("/test/path"));
    println!("file:\n{}\n", file);
    assert_eq!(file.to_string(), "/test/path");
}
