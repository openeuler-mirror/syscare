use memoffset::offset_of;

use super::{OperateRead, OperateWrite};

pub trait HeaderRead: OperateRead {
    fn get_e_shnum(&self) -> u16 {
        self.get(
            offset_of!(FileHeader64, e_shnum)
        )
    }

    fn get_e_shoff(&self) -> u64 {
        self.get(
            offset_of!(FileHeader64, e_shoff)
        )
    }

    fn get_e_shentsize(&self) -> u16 {
        self.get(
            offset_of!(FileHeader64, e_shentsize)
        )
    }
}

pub trait HeaderWrite: OperateWrite {
    fn set_e_shnum(&mut self, e_shnum: u16) -> () {
        self.set(
            offset_of!(FileHeader64, e_shnum),
            e_shnum
        )
    }
}

#[repr(C)]
pub struct FileHeader64 {
    pub e_ident:        u128,
    pub e_type:         u16,
    pub e_machine:      u16,
    pub e_version:      u32,
    pub e_entry:        u64,
    pub e_phoff:        u64,
    pub e_shoff:        u64,
    pub e_flags:        u32,
    pub e_ehsize:       u16,
    pub e_phentsize:    u16,
    pub e_phnum:        u16,
    pub e_shentsize:    u16,
    pub e_shnum:        u16,
    pub e_shstrndx:     u16,
}