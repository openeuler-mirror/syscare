use memmap2::Mmap;

use super::super::{Endian, ReadInteger, HeaderRead, OperateRead};

#[derive(Debug)]
pub struct Header {
    mmap: Mmap,
    endian: Endian,
}

impl Header {
    pub fn from(mmap: Mmap, endian: Endian) -> Self {
        Self {
            mmap,
            endian
        }
    }
}

impl HeaderRead for Header {}

impl OperateRead for Header {
    fn get<T: ReadInteger<T>>(&self, start: usize) -> T {
        self.endian.read_integer::<T>(&self.mmap[start..(start + std::mem::size_of::<T>())])
    }
}