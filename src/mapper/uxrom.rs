use crate::cart::Mirroring;
use crate::mapper::Mapper;
use std::cell::RefCell;

pub struct UxromMapper {
    prg_rom: Vec<u8>,
    chr_rom: Vec<u8>,
    mirroring: Mirroring,
    bank: RefCell<u8>,
}

impl UxromMapper {
    pub fn new(prg_rom: Vec<u8>, chr_rom: Vec<u8>, mirroring: Mirroring) -> Self {
        UxromMapper {
            prg_rom,
            chr_rom,
            mirroring,
            bank: RefCell::new(0),
        }
    }
}

impl Mapper for UxromMapper {
    fn read_prg(&self, addr: u16) -> u8 {
        let addr = addr as usize;
        match addr {
            0x8000..=0xBFFF => {
                let bank = *self.bank.borrow() as usize % (self.prg_rom.len() / 0x4000);
                self.prg_rom[bank * 0x4000 + (addr - 0x8000)]
            }
            0xC000..=0xFFFF => {
                let last_bank = (self.prg_rom.len() / 0x4000) - 1;
                self.prg_rom[last_bank * 0x4000 + (addr - 0xC000)]
            }
            _ => panic!("Invalid PRG read address: {:x}", addr),
        }
    }

    fn write_prg(&self, addr: u16, data: u8) {
        if addr >= 0x8000 {
            let num_banks = self.prg_rom.len() / 0x4000;
            *self.bank.borrow_mut() = data % num_banks as u8;
        }
    }

    fn read_chr(&self, addr: u16) -> u8 {
        self.chr_rom[addr as usize]
    }

    fn write_chr(&self, _addr: u16, _data: u8) {
        // CHR ROM, ignore writes
    }

    fn mirroring(&self) -> Mirroring {
        self.mirroring.clone()
    }
}