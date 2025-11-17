use crate::cart::Mirroring;
use crate::mapper::{ChrSource, Mapper};

const PRG_BANK_SIZE: usize = 0x4000;
const CHR_BANK_SIZE_4K: usize = 0x1000;

pub struct Mmc1Mapper {
    prg_rom: Vec<u8>,
    chr: Vec<u8>,
    chr_is_ram: bool,
    prg_ram: Vec<u8>,

    shift_register: u8,
    shift_count: u8,

    control: u8,
    chr_bank0: u8,
    chr_bank1: u8,
    prg_bank: u8,
    prg_ram_disabled: bool,

    mirroring: Mirroring,
}

impl Mmc1Mapper {
    pub fn new(prg_rom: Vec<u8>, chr_rom: Vec<u8>, mirroring: Mirroring) -> Self {
        let chr_is_ram = chr_rom.is_empty();
        let chr = if chr_is_ram {
            vec![0; 0x2000]
        } else {
            chr_rom
        };

        let mut mapper = Mmc1Mapper {
            prg_rom,
            chr,
            chr_is_ram,
            prg_ram: vec![0; 0x2000],
            shift_register: 0,
            shift_count: 0,
            control: 0x0C,
            chr_bank0: 0,
            chr_bank1: 0,
            prg_bank: 0,
            prg_ram_disabled: false,
            mirroring,
        };

        mapper.update_mirroring();
        mapper
    }

    fn prg_bank_count(&self) -> usize {
        let count = self.prg_rom.len() / PRG_BANK_SIZE;
        if count == 0 { 1 } else { count }
    }

    fn chr_bank_count(&self) -> usize {
        let count = self.chr.len() / CHR_BANK_SIZE_4K;
        if count == 0 { 1 } else { count }
    }

    fn reset_shift_register(&mut self) {
        self.shift_register = 0;
        self.shift_count = 0;
    }

    fn update_mirroring(&mut self) {
        self.mirroring = match self.control & 0x03 {
            0 => Mirroring::SingleScreenLower,
            1 => Mirroring::SingleScreenUpper,
            2 => Mirroring::Vertical,
            _ => Mirroring::Horizontal,
        };
    }

    fn apply_register_write(&mut self, target: u8, value: u8) {
        match target {
            0 => {
                self.control = value & 0x1F;
                self.update_mirroring();
            }
            1 => {
                self.chr_bank0 = value & 0x1F;
            }
            2 => {
                self.chr_bank1 = value & 0x1F;
            }
            3 => {
                self.prg_bank = value & 0x0F;
                self.prg_ram_disabled = value & 0x10 != 0;
            }
            _ => {}
        }
    }

    fn calc_prg_offset(&self, addr: u16) -> usize {
        if self.prg_rom.is_empty() {
            return 0;
        }

        let bank_mode = (self.control >> 2) & 0x03;
        let bank_count = self.prg_bank_count();
        let offset = (addr & 0x3FFF) as usize;

        let bank_index = match bank_mode {
            0 | 1 => {
                // 32KB mode, ignore lowest bit
                let bank = (self.prg_bank & 0xFE) as usize;
                let slot = if addr < 0xC000 { 0 } else { 1 };
                (bank + slot) % bank_count
            }
            2 => {
                if addr < 0xC000 {
                    0
                } else {
                    (self.prg_bank as usize) % bank_count
                }
            }
            _ => {
                if addr < 0xC000 {
                    (self.prg_bank as usize) % bank_count
                } else {
                    bank_count - 1
                }
            }
        };

        (bank_index * PRG_BANK_SIZE + offset) % self.prg_rom.len()
    }

    fn calc_chr_offset(&self, addr: u16) -> usize {
        if self.chr.is_empty() {
            return 0;
        }

        let bank_mode = (self.control >> 4) & 0x01;
        let bank_count = self.chr_bank_count();
        let offset = (addr & 0x0FFF) as usize;

        let bank_index = if bank_mode == 0 {
            let base_bank = (self.chr_bank0 & 0x1E) as usize;
            if addr < 0x1000 {
                base_bank % bank_count
            } else {
                (base_bank + 1) % bank_count
            }
        } else if addr < 0x1000 {
            (self.chr_bank0 as usize) % bank_count
        } else {
            (self.chr_bank1 as usize) % bank_count
        };

        (bank_index * CHR_BANK_SIZE_4K + offset) % self.chr.len()
    }
}

impl Mapper for Mmc1Mapper {
    fn read_prg(&self, addr: u16) -> u8 {
        match addr {
            0x6000..=0x7FFF => {
                if self.prg_ram_disabled {
                    0xFF
                } else {
                    self.prg_ram[(addr - 0x6000) as usize]
                }
            }
            0x8000..=0xFFFF => {
                if self.prg_rom.is_empty() {
                    0
                } else {
                    let index = self.calc_prg_offset(addr);
                    self.prg_rom[index]
                }
            }
            _ => 0,
        }
    }

    fn write_prg(&mut self, addr: u16, data: u8) {
        match addr {
            0x6000..=0x7FFF => {
                if !self.prg_ram_disabled {
                    self.prg_ram[(addr - 0x6000) as usize] = data;
                }
            }
            0x8000..=0xFFFF => {
                if data & 0x80 != 0 {
                    self.control |= 0x0C;
                    self.update_mirroring();
                    self.reset_shift_register();
                } else {
                    self.shift_register |= (data & 1) << self.shift_count;
                    self.shift_count += 1;

                    if self.shift_count == 5 {
                        let target = ((addr >> 13) & 0x03) as u8;
                        self.apply_register_write(target, self.shift_register);
                        self.reset_shift_register();
                    }
                }
            }
            _ => {}
        }
    }

    fn read_chr(&self, addr: u16, _source: ChrSource) -> u8 {
        if self.chr.is_empty() {
            0
        } else {
            let index = self.calc_chr_offset(addr);
            self.chr[index]
        }
    }

    fn write_chr(&mut self, addr: u16, data: u8) {
        if self.chr_is_ram && !self.chr.is_empty() {
            let index = self.calc_chr_offset(addr);
            self.chr[index] = data;
        }
    }

    fn mirroring(&self) -> Mirroring {
        self.mirroring.clone()
    }
}
