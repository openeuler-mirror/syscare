use memmap2::Mmap;

use super::super::{Endian, ReadInteger, SectionRead, OperateRead};

#[derive(Debug)]
pub struct SectionHeader {
    mmap: Mmap,
    endian: Endian,
}

impl SectionHeader {
    pub fn from(mmap: Mmap, endian: Endian) -> Self {
        Self {
            mmap,
            endian,
        }
    }
}

impl SectionRead for SectionHeader {}

impl OperateRead for SectionHeader {
    fn get<T: ReadInteger<T>>(&self, start: usize) -> T {
        self.endian.read_integer::<T>(&self.mmap[start..(start + std::mem::size_of::<T>())])
    }
}