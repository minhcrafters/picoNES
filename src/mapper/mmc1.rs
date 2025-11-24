use crate::cart::Mirroring;
use crate::mapper::{ChrSource, Mapper};

const PRG_BANK_SIZE: usize = 0x4000;
const CHR_BANK_SIZE_4K: usize = 0x1000;
const SRAM_BANK_SIZE: usize = 0x2000;

#[derive(Default, PartialEq)]
enum PrgMode {
    Bank32kb,
    FixFirstPage,
    #[default]
    FixLastPage,
}

#[derive(Default, PartialEq)]
enum ChrMode {
    #[default]
    Bank8kb,
    Bank4kb,
}

pub struct Mmc1Mapper {
    prg_rom: Vec<u8>,
    chr: Vec<u8>,
    chr_is_ram: bool,
    prg_ram: Vec<u8>,

    prg_mode: PrgMode,
    chr_mode: ChrMode,
    prg_select: usize,
    prg_256kb_bank: usize,
    prg_last_bank: usize,
    chr_select0: usize,
    chr_select1: usize,
    last_wrote_chr_select1: bool,

    shift_reg: u8,
    shift_writes: u8,

    prg_ram_disabled: bool,
    prg_banks: [usize; 2],
    chr_banks: [usize; 2],
    sram_bank: usize,

    has_512kb_prg: bool,
    mirroring: Mirroring,
}

impl Mmc1Mapper {
    pub fn new(prg_rom: Vec<u8>, chr_rom: Vec<u8>, mirroring: Mirroring) -> Self {
        let chr_is_ram = chr_rom.is_empty();
        let chr = if chr_is_ram { vec![0; 0x2000] } else { chr_rom };

        let prg_bank_count = std::cmp::max(1, prg_rom.len() / PRG_BANK_SIZE);
        let has_512kb_prg = prg_rom.len() > 256 * 1024;
        let prg_last_bank = if has_512kb_prg {
            prg_bank_count / 2 - 1
        } else {
            prg_bank_count - 1
        };

        let mut mapper = Mmc1Mapper {
            prg_rom,
            chr,
            chr_is_ram,
            prg_ram: vec![0; SRAM_BANK_SIZE],
            prg_mode: PrgMode::FixLastPage,
            chr_mode: ChrMode::Bank8kb,
            prg_select: 0,
            prg_256kb_bank: 0,
            prg_last_bank,
            chr_select0: 0,
            chr_select1: 0,
            last_wrote_chr_select1: false,
            shift_reg: 0,
            shift_writes: 0,
            prg_ram_disabled: false,
            prg_banks: [0; 2],
            chr_banks: [0; 2],
            sram_bank: 0,
            has_512kb_prg,
            mirroring,
        };

        mapper.update_prg_banks();
        mapper.update_all_banks();
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

    fn write_ctrl(&mut self, val: u8) {
        self.mirroring = match val & 0b11 {
            0 => Mirroring::SingleScreenLower,
            1 => Mirroring::SingleScreenUpper,
            2 => Mirroring::Vertical,
            _ => Mirroring::Horizontal,
        };

        self.prg_mode = match (val >> 2) & 0b11 {
            2 => PrgMode::FixFirstPage,
            3 => PrgMode::FixLastPage,
            _ => PrgMode::Bank32kb,
        };
        self.update_prg_banks();

        self.chr_mode = match (val >> 4) != 0 {
            false => ChrMode::Bank8kb,
            true => ChrMode::Bank4kb,
        };
        self.update_all_banks();
    }

    fn update_prg_banks(&mut self) {
        if self.prg_rom.is_empty() {
            self.prg_banks = [0; 2];
            return;
        }

        let prg_count = self.prg_bank_count();
        let (mut bank0, mut bank1) = match self.prg_mode {
            PrgMode::Bank32kb => {
                let bank = self.prg_select & !1;
                (bank, bank + 1)
            }
            PrgMode::FixFirstPage => (0, self.prg_select),
            PrgMode::FixLastPage => (self.prg_select, self.prg_last_bank),
        };

        bank0 = (bank0 | self.prg_256kb_bank) % prg_count;
        bank1 = (bank1 | self.prg_256kb_bank) % prg_count;

        self.prg_banks[0] = bank0 * PRG_BANK_SIZE;
        self.prg_banks[1] = bank1 * PRG_BANK_SIZE;
    }

    fn update_chr_banks(&mut self) {
        if self.chr.is_empty() {
            self.chr_banks = [0; 2];
            return;
        }

        let chr_count = self.chr_bank_count();
        match self.chr_mode {
            ChrMode::Bank8kb => {
                let base = (self.chr_select0 & !1) % chr_count;
                self.chr_banks[0] = base * CHR_BANK_SIZE_4K;
                self.chr_banks[1] = ((base + 1) % chr_count) * CHR_BANK_SIZE_4K;
            }
            ChrMode::Bank4kb => {
                self.chr_banks[0] = (self.chr_select0 % chr_count) * CHR_BANK_SIZE_4K;
                self.chr_banks[1] = (self.chr_select1 % chr_count) * CHR_BANK_SIZE_4K;
            }
        }
    }

    fn update_sram_bank(&mut self, sxrom_select: usize) {
        let banks = self.prg_ram.len() / SRAM_BANK_SIZE;
        let bank = match banks {
            0 | 1 => 0,
            2 => (sxrom_select >> 3) & 0b01,
            4 => (sxrom_select >> 2) & 0b11,
            _ => 0,
        };
        self.sram_bank = bank * SRAM_BANK_SIZE;
    }

    fn update_all_banks(&mut self) {
        self.update_chr_banks();

        let sxrom_select = if self.last_wrote_chr_select1 && self.chr_mode == ChrMode::Bank4kb {
            self.chr_select1
        } else {
            self.chr_select0
        };

        if self.has_512kb_prg {
            self.prg_256kb_bank = sxrom_select & 0b1_0000;
            self.update_prg_banks();
        }

        self.update_sram_bank(sxrom_select);
    }
}

impl Mapper for Mmc1Mapper {
    fn read_prg(&self, addr: u16) -> u8 {
        match addr {
            0x6000..=0x7FFF => {
                if self.prg_ram_disabled || self.prg_ram.is_empty() {
                    0xFF
                } else {
                    let index = self.sram_bank + (addr as usize - 0x6000);
                    self.prg_ram.get(index).copied().unwrap_or(0xFF)
                }
            }
            0x8000..=0xFFFF => {
                if self.prg_rom.is_empty() {
                    0
                } else {
                    let bank = if addr < 0xC000 { self.prg_banks[0] } else { self.prg_banks[1] };
                    let offset = bank + (addr as usize & 0x3FFF);
                    self.prg_rom.get(offset).copied().unwrap_or(0)
                }
            }
            _ => 0,
        }
    }

    fn write_prg(&mut self, addr: u16, val: u8) {
        match addr {
            0x6000..=0x7FFF => {
                if !self.prg_ram_disabled && !self.prg_ram.is_empty() {
                    let index = self.sram_bank + (addr as usize - 0x6000);
                    if index < self.prg_ram.len() {
                        self.prg_ram[index] = val;
                    }
                }
            }
            0x8000..=0xFFFF => {
                if val & 0b1000_0000 != 0 {
                    self.shift_reg = 0;
                    self.shift_writes = 0;
                    self.prg_mode = PrgMode::FixLastPage;
                    self.update_prg_banks();
                } else if self.shift_writes < 5 {
                    self.shift_reg = (self.shift_reg >> 1) | ((val & 1) << 4);
                    self.shift_writes += 1;
                }

                if self.shift_writes >= 5 {
                    match addr {
                        0x8000..=0x9FFF => self.write_ctrl(self.shift_reg),
                        0xA000..=0xBFFF => {
                            self.chr_select0 = (self.shift_reg & 0b1_1111) as usize;
                            self.last_wrote_chr_select1 = false;
                            self.update_all_banks();
                        }
                        0xC000..=0xDFFF => {
                            self.chr_select1 = (self.shift_reg & 0b1_1111) as usize;
                            self.last_wrote_chr_select1 = true;
                            self.update_all_banks();
                        }
                        0xE000..=0xFFFF => {
                            self.prg_ram_disabled = self.shift_reg & 0x10 != 0;
                            self.prg_select = (self.shift_reg & 0x0F) as usize;
                            self.update_prg_banks();
                        }
                        _ => {}
                    }

                    self.shift_writes = 0;
                    self.shift_reg = 0;
                }
            }
            _ => {}
        }
    }

    fn read_chr(&self, addr: u16, _source: ChrSource) -> u8 {
        if self.chr.is_empty() {
            0
        } else {
            let bank = if addr < 0x1000 { self.chr_banks[0] } else { self.chr_banks[1] };
            let offset = bank + (addr as usize & 0x0FFF);
            self.chr.get(offset).copied().unwrap_or(0)
        }
    }

    fn write_chr(&mut self, addr: u16, val: u8) {
        if self.chr_is_ram && !self.chr.is_empty() {
            let bank = if addr < 0x1000 { self.chr_banks[0] } else { self.chr_banks[1] };
            let offset = bank + (addr as usize & 0x0FFF);
            if offset < self.chr.len() {
                self.chr[offset] = val;
            }
        }
    }

    fn mirroring(&self) -> Mirroring {
        self.mirroring.clone()
    }
}
