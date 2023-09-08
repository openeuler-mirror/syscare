use std::ffi::OsString;
use std::io::{BufRead, BufReader, Read};
use std::os::unix::prelude::OsStringExt;

pub struct RawLines<R> {
    reader: BufReader<R>,
}

impl<R: Read> Iterator for RawLines<R> {
    type Item = std::io::Result<OsString>;

    fn next(&mut self) -> Option<Self::Item> {
        const CHAR_LF: [u8; 1] = [b'\n'];
        const CHAR_CR: [u8; 1] = [b'\r'];

        let mut buf = Vec::<u8>::new();
        match self.reader.read_until(CHAR_LF[0], &mut buf) {
            Ok(0) => None,
            Ok(_) => {
                // Drop "\n" or "\r\n" on the buf tail
                if buf.ends_with(&CHAR_LF) {
                    buf.pop();
                    if buf.ends_with(&CHAR_CR) {
                        buf.pop();
                    }
                }
                // Drop remaining capacity to save some memory
                buf.shrink_to_fit();
                Some(Ok(OsString::from_vec(buf)))
            }
            Err(e) => Some(Err(e)),
        }
    }
}

impl<R: Read> From<R> for RawLines<R> {
    fn from(read: R) -> Self {
        Self {
            reader: BufReader::new(read),
        }
    }
}
