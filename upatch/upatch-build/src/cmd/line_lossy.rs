use std::io::BufRead;

pub struct LossyLines<B> {
    buf: B,
}

impl <B: BufRead> Iterator for LossyLines<B> {
    type Item = std::io::Result<String>;

    fn next(&mut self) -> Option<Self::Item> {
        const CHAR_LF: [u8; 1] = [b'\n'];
        const CHAR_CR: [u8; 1] = [b'\r'];

        let mut buf = Vec::<u8>::new();
        match self.buf.read_until(CHAR_LF[0], buf.as_mut()) {
            Ok(0) => None,
            Ok(_) => {
                if buf.ends_with(&CHAR_LF) {
                    buf.pop();
                    if buf.ends_with(&CHAR_CR) {
                        buf.pop();
                    }
                }
                Some(Ok(String::from_utf8_lossy(&buf).to_string()))
            },
            Err(e) => Some(Err(e))
        }
    }
}

impl<B: BufRead> From<B> for LossyLines<B> {
    fn from(buf: B) -> Self {
        Self { buf }
    }
}