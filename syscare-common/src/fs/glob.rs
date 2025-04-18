use std::{
    ffi::{OsStr, OsString},
    fs,
    io::Result,
    path::{Path, PathBuf},
};

use crate::{ffi::OsStrExt, os_str::CharByte};

const WILDCARD_ONE: char = '?';
const WILDCARD_ALL: char = '*';
const WILDCARD_RECURSIVE: &str = "**";

pub struct Glob {
    components: Vec<OsString>,
    stack: Vec<(PathBuf, usize)>, // current path, component index
}

impl Glob {
    fn match_chars<I, P>(name: I, mut pattern: P) -> bool
    where
        I: Iterator<Item = CharByte>,
        P: Iterator<Item = CharByte>,
    {
        for current in name {
            match pattern.next() {
                Some(matching) => {
                    if matching == WILDCARD_ONE {
                        continue; // matched one char
                    }
                    if matching == WILDCARD_ALL {
                        return true; // matched all chars
                    }
                    if current != matching {
                        return false; // not matched
                    }
                }
                None => return false, // pattern not enough
            }
        }

        true
    }

    fn match_component_name(name: &OsStr, component: &OsStr) -> bool {
        if !Self::match_chars(name.chars(), component.chars()) {
            return false;
        }
        // If pattern contains "*", we have to do reverse match to
        // make sure tail chars were also matched to the pattern.
        if component.contains(WILDCARD_ALL) {
            return Self::match_chars(name.chars().rev(), component.chars().rev());
        }

        true
    }

    fn match_component(&mut self, path: PathBuf, index: usize) -> Result<Option<PathBuf>> {
        let last_index = self.components.len() - 1;

        for dir_entry in fs::read_dir(&path)? {
            let next_path = dir_entry?.path();
            let next_name = next_path.file_name().unwrap_or_default();

            let component = self.components[index].as_os_str();
            if !Self::match_component_name(next_name, component) {
                continue; // not matched, skip
            }

            if index == last_index {
                return Ok(Some(next_path));
            }

            if next_path.is_dir() {
                self.stack.push((next_path, index + 1));
            }
        }

        Ok(None)
    }

    fn match_recursive_wildcard(&mut self, path: PathBuf, index: usize) -> Result<()> {
        // push files & directories in current directory to stack
        for dir_entry in fs::read_dir(&path)? {
            let next_path = dir_entry?.path();
            if next_path.is_dir() {
                self.stack.push((next_path, index)); // recursive match, keep index
            }
        }
        // push current path back to stack, match next component
        self.stack.push((path, index + 1));

        Ok(())
    }

    fn match_multiple_wildcard(&mut self, path: PathBuf, index: usize) -> Result<()> {
        // push files & directories in current directory to stack
        for dir_entry in fs::read_dir(&path)? {
            let next_path = dir_entry?.path();
            if next_path.is_dir() {
                self.stack.push((next_path, index + 1));
            }
        }
        // push current path back to stack, match next component
        self.stack.push((path, index + 1));

        Ok(())
    }
}

impl Iterator for Glob {
    type Item = Result<PathBuf>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some((mut curr_path, mut index)) = self.stack.pop() {
            let last_index = self.components.len() - 1;

            // iterate all of components over matching path
            while index <= last_index {
                let component = self.components[index].as_os_str();

                if component == WILDCARD_RECURSIVE {
                    if let Err(e) = self.match_recursive_wildcard(curr_path, index) {
                        return Some(Err(e));
                    }
                } else if component == OsStr::new(&WILDCARD_ALL.to_string()) {
                    if let Err(e) = self.match_multiple_wildcard(curr_path, index) {
                        return Some(Err(e));
                    }
                } else if component.contains(WILDCARD_ONE) || component.contains(WILDCARD_ALL) {
                    let result = self.match_component(curr_path, index).transpose();
                    if result.is_some() {
                        return result;
                    }
                } else {
                    // normal component, push to current path
                    curr_path.push(component);

                    if (index == last_index) && curr_path.exists() {
                        return Some(Ok(curr_path.clone()));
                    }

                    index += 1;
                    continue;
                }

                break;
            }
        }

        None
    }
}

pub fn glob<P: AsRef<Path>>(path: P) -> Glob {
    let match_dir = path.as_ref().to_path_buf();
    let matching = match_dir
        .components()
        .map(|c| c.as_os_str().to_os_string())
        .collect::<Vec<_>>();

    Glob {
        components: matching,
        stack: vec![(match_dir, 0)],
    }
}
