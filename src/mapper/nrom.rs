use crate::cart::Mirroring;
use crate::mapper::{ChrSource, Mapper};

pub struct NromMapper {
    prg_rom: Vec<u8>,
    chr: Vec<u8>,
    chr_is_ram: bool,
    mirroring: Mirroring,
}

impl NromMapper {
    pub fn new(prg_rom: Vec<u8>, chr_rom: Vec<u8>, mirroring: Mirroring) -> Self {
        let chr_is_ram = chr_rom.is_empty();
        let chr = if chr_is_ram {
            vec![0; 0x2000]
        } else {
            chr_rom
        };

        NromMapper {
            prg_rom,
            chr,
            chr_is_ram,
            mirroring,
        }
    }
}

impl Mapper for NromMapper {
    fn read_prg(&self, addr: u16) -> u8 {
        if !(0x8000..=0xFFFF).contains(&addr) {
            return 0;
        }

        if self.prg_rom.is_empty() {
            return 0;
        }

        let mut offset = (addr - 0x8000) as usize;
        if self.prg_rom.len() == 0x4000 {
            // Mirror 16KB PRG across both $8000-$BFFF and $C000-$FFFF
            offset %= 0x4000;
        }

        self.prg_rom[offset]
    }

    fn write_prg(&mut self, _addr: u16, _data: u8) {
        // NROM has no PRG RAM, ignore writes
    }

    fn read_chr(&self, addr: u16, _source: ChrSource) -> u8 {
        self.chr[addr as usize % self.chr.len()]
    }

    fn write_chr(&mut self, addr: u16, data: u8) {
        if self.chr_is_ram {
            let index = addr as usize % self.chr.len();
            self.chr[index] = data;
        }
    }

    fn mirroring(&self) -> Mirroring {
        self.mirroring.clone()
    }
}
