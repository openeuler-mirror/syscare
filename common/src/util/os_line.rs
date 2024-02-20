use std::ffi::OsString;
use std::io::BufRead;
use std::os::unix::prelude::OsStringExt;

pub struct OsLines<R> {
    buf: R,
}

impl<R: BufRead> Iterator for OsLines<R> {
    type Item = std::io::Result<OsString>;

    fn next(&mut self) -> Option<Self::Item> {
        const CHAR_LF: [u8; 1] = [b'\n'];
        const CHAR_CR: [u8; 1] = [b'\r'];

        let mut buf = Vec::new();
        match self.buf.read_until(CHAR_LF[0], &mut buf) {
            Ok(0) => None,
            Ok(_) => {
                // Drop "\n" or "\r\n" on the buf tail
                if buf.ends_with(&CHAR_LF) {
                    buf.pop();
                    if buf.ends_with(&CHAR_CR) {
                        buf.pop();
                    }
                }
                buf.shrink_to_fit();
                Some(Ok(OsString::from_vec(buf)))
            }
            Err(_) => self.buf.read_to_end(&mut buf).ok().map(|_| {
                buf.shrink_to_fit();
                Ok(OsString::from_vec(buf))
            }),
        }
    }
}

pub trait BufReadOsLines: BufRead {
    fn os_lines(self) -> OsLines<Self>
    where
        Self: Sized,
    {
        OsLines { buf: self }
    }
}

impl<R: BufRead> BufReadOsLines for R {}

#[test]
fn test() {
    use super::fs;
    use std::io::BufReader;

    let buf_reader =
        BufReader::new(fs::open_file("/proc/self/cmdline").expect("Failed to open procfs"));
    let lines = buf_reader.lines();
    for str in lines.flatten() {
        println!("{:?}", str);
        assert!(!str.is_empty());
    }

    let buf_reader =
        BufReader::new(fs::open_file("/proc/self/cmdline").expect("Failed to open procfs"));
    let os_lines = buf_reader.os_lines();
    for str in os_lines.flatten() {
        println!("{:?}", str);
        assert!(!str.is_empty());
    }
}
