use crate::cart::Mirroring;
use crate::mapper::{ChrSource, Mapper};

pub struct NsfMapper {
    prg_rom: Vec<u8>,
    chr: Vec<u8>,
    chr_is_ram: bool,

    mirroring: Mirroring,

    banks: [usize; 8],
}

impl NsfMapper {
    pub fn new(prg_rom: Vec<u8>, chr_rom: Vec<u8>, mirroring: Mirroring) -> Self {
        assert!(!prg_rom.is_empty(), "PRG ROM must contain at least 4kB");

        let total_banks = prg_rom.len() / 0x1000;

        let last_bank = total_banks - 1;

        let mut banks = [0usize; 8];
        banks[7] = last_bank;

        let chr_is_ram = chr_rom.is_empty();
        let chr = if chr_is_ram { vec![0; 0x2000] } else { chr_rom };

        NsfMapper {
            prg_rom,
            banks,
            chr,
            chr_is_ram,
            mirroring,
        }
    }

    fn prg_offset(&self, addr: u16) -> usize {
        let offset_within_slice = (addr.wrapping_sub(0x8000)) as usize & 0x0FFF;
        let slice_idx = ((addr.wrapping_sub(0x8000)) as usize >> 12) & 0x07;

        let bank = self.banks[slice_idx] % (self.prg_rom.len() / 0x1000);

        (bank * 0x1000) + offset_within_slice
    }
}

impl Mapper for NsfMapper {
    fn read_prg(&self, addr: u16) -> u8 {
        if !(0x8000..=0xFFFF).contains(&addr) {
            return 0;
        }
        if self.prg_rom.is_empty() {
            return 0;
        }

        let off = self.prg_offset(addr);
        self.prg_rom[off]
    }

    fn write_prg(&mut self, addr: u16, data: u8) {
        if (0x5FF8..=0x5FFF).contains(&addr) {
            let idx = (addr - 0x5FF8) as usize;
            let total_banks = self.prg_rom.len() / 0x1000;
            self.banks[idx] = (data as usize) % total_banks;
        }
    }

    fn read_chr(&self, addr: u16, _source: ChrSource) -> u8 {
        self.chr[(addr as usize) % self.chr.len()]
    }

    fn write_chr(&mut self, addr: u16, data: u8) {
        if self.chr_is_ram {
            let idx = (addr as usize) % self.chr.len();
            self.chr[idx] = data;
        }
    }

    fn mirroring(&self) -> Mirroring {
        self.mirroring.clone()
    }
}
