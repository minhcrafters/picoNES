use crate::cart::Mirroring;
use crate::mapper::{ChrSource, Mapper};

const CHR_BANK_SIZE: usize = 0x2000;

pub struct CnromMapper {
    prg_rom: Vec<u8>,
    chr: Vec<u8>,
    chr_is_ram: bool,
    prg_ram: Vec<u8>,
    chr_bank: u8,
    mirroring: Mirroring,
}

impl CnromMapper {
    pub fn new(prg_rom: Vec<u8>, chr_rom: Vec<u8>, mirroring: Mirroring) -> Self {
        let chr_is_ram = chr_rom.is_empty();
        let chr = if chr_is_ram {
            vec![0; CHR_BANK_SIZE]
        } else {
            chr_rom
        };

        CnromMapper {
            prg_rom,
            chr,
            chr_is_ram,
            prg_ram: vec![0; 0x2000],
            chr_bank: 0,
            mirroring,
        }
    }

    fn chr_bank_count(&self) -> usize {
        let count = self.chr.len() / CHR_BANK_SIZE;
        if count == 0 { 1 } else { count }
    }
}

impl Mapper for CnromMapper {
    fn read_prg(&self, addr: u16) -> u8 {
        match addr {
            0x6000..=0x7FFF => self.prg_ram[(addr - 0x6000) as usize],
            0x8000..=0xFFFF => {
                if self.prg_rom.is_empty() {
                    0
                } else {
                    let offset = (addr - 0x8000) as usize;
                    self.prg_rom[offset % self.prg_rom.len()]
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
                let count = self.chr_bank_count() as u8;
                self.chr_bank = if count == 0 { 0 } else { data % count };
            }
            _ => {}
        }
    }

    fn read_chr(&self, addr: u16, _source: ChrSource) -> u8 {
        if self.chr.is_empty() {
            0
        } else {
            let bank = (self.chr_bank as usize % self.chr_bank_count()) * CHR_BANK_SIZE;
            let offset = (addr as usize) & 0x1FFF;
            let index = bank + offset;
            self.chr[index % self.chr.len()]
        }
    }

    fn write_chr(&mut self, addr: u16, data: u8) {
        if self.chr_is_ram && !self.chr.is_empty() {
            let bank = (self.chr_bank as usize % self.chr_bank_count()) * CHR_BANK_SIZE;
            let offset = (addr as usize) & 0x1FFF;
            let index = bank + offset;
            let len = self.chr.len();
            let idx = index % len;
            self.chr[idx] = data;
        }
    }

    fn mirroring(&self) -> Mirroring {
        self.mirroring.clone()
    }
}
