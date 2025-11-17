use crate::cart::Mirroring;
use crate::mapper::{ChrSource, Mapper};

const PRG_BANK_SIZE: usize = 0x4000;

pub struct UxromMapper {
    prg_rom: Vec<u8>,
    chr: Vec<u8>,
    chr_is_ram: bool,
    prg_ram: Vec<u8>,
    bank_select: u8,
    mirroring: Mirroring,
}

impl UxromMapper {
    pub fn new(prg_rom: Vec<u8>, chr_rom: Vec<u8>, mirroring: Mirroring) -> Self {
        let chr_is_ram = chr_rom.is_empty();
        let chr = if chr_is_ram {
            vec![0; 0x2000]
        } else {
            chr_rom
        };

        UxromMapper {
            prg_rom,
            chr,
            chr_is_ram,
            prg_ram: vec![0; 0x2000],
            bank_select: 0,
            mirroring,
        }
    }

    fn prg_bank_count(&self) -> usize {
        let count = self.prg_rom.len() / PRG_BANK_SIZE;
        if count == 0 { 1 } else { count }
    }

    fn prg_bank_offset(&self, bank: usize) -> usize {
        let count = self.prg_bank_count();
        (bank % count) * PRG_BANK_SIZE
    }
}

impl Mapper for UxromMapper {
    fn read_prg(&self, addr: u16) -> u8 {
        match addr {
            0x6000..=0x7FFF => self.prg_ram[(addr - 0x6000) as usize],
            0x8000..=0xBFFF => {
                if self.prg_rom.is_empty() {
                    0
                } else {
                    let offset = self.prg_bank_offset(self.bank_select as usize);
                    let index = offset + (addr as usize - 0x8000);
                    self.prg_rom[index % self.prg_rom.len()]
                }
            }
            0xC000..=0xFFFF => {
                if self.prg_rom.is_empty() {
                    0
                } else {
                    let last_bank = self.prg_bank_count() - 1;
                    let offset = self.prg_bank_offset(last_bank);
                    let index = offset + (addr as usize - 0xC000);
                    self.prg_rom[index % self.prg_rom.len()]
                }
            }
            _ => 0,
        }
    }

    fn write_prg(&mut self, addr: u16, data: u8) {
        match addr {
            0x6000..=0x7FFF => {
                self.prg_ram[(addr - 0x6000) as usize] = data;
            }
            0x8000..=0xFFFF => {
                let count = self.prg_bank_count() as u8;
                self.bank_select = if count == 0 { 0 } else { data % count };
            }
            _ => {}
        }
    }

    fn read_chr(&self, addr: u16, _source: ChrSource) -> u8 {
        if self.chr.is_empty() {
            0
        } else {
            self.chr[addr as usize % self.chr.len()]
        }
    }

    fn write_chr(&mut self, addr: u16, data: u8) {
        if self.chr_is_ram && !self.chr.is_empty() {
            let index = addr as usize % self.chr.len();
            self.chr[index] = data;
        }
    }

    fn mirroring(&self) -> Mirroring {
        self.mirroring.clone()
    }
}
