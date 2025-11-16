use crate::cart::Mirroring;
use crate::mapper::Mapper;
use std::cell::RefCell;

pub struct CnromMapper {
    prg_rom: Vec<u8>,
    chr_rom: Vec<u8>,
    mirroring: Mirroring,
    chr_bank: RefCell<u8>,
}

impl CnromMapper {
    pub fn new(prg_rom: Vec<u8>, chr_rom: Vec<u8>, mirroring: Mirroring) -> Self {
        CnromMapper {
            prg_rom,
            chr_rom,
            mirroring,
            chr_bank: RefCell::new(0),
        }
    }
}

impl Mapper for CnromMapper {
    fn read_prg(&self, mut addr: u16) -> u8 {
        addr -= 0x8000;
        if self.prg_rom.len() == 0x4000 && addr >= 0x4000 {
            addr %= 0x4000;
        }
        self.prg_rom[addr as usize]
    }

    fn write_prg(&self, addr: u16, data: u8) {
        if addr >= 0x8000 {
            let num_banks = self.chr_rom.len() / 0x2000;
            *self.chr_bank.borrow_mut() = data % num_banks as u8;
        }
    }

    fn read_chr(&self, addr: u16) -> u8 {
        let bank = *self.chr_bank.borrow() as usize % (self.chr_rom.len() / 0x2000);
        self.chr_rom[bank * 0x2000 + addr as usize]
    }

    fn write_chr(&self, _addr: u16, _data: u8) {
        // CHR ROM, ignore writes
    }

    fn mirroring(&self) -> Mirroring {
        self.mirroring.clone()
    }
}