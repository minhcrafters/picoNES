use crate::cart::Mirroring;
use crate::mapper::{ChrSource, Mapper};

const PRG_BANK_SIZE: usize = 0x2000;
const CHR_BANK_SIZE_1K: usize = 0x0400;
const CHR_BANK_SIZE_2K: usize = 0x0800;

pub struct Mmc3Mapper {
    prg_rom: Vec<u8>,
    chr: Vec<u8>,
    chr_is_ram: bool,
    prg_ram: Vec<u8>,
    prg_ram_enable: bool,
    prg_ram_write_protect: bool,

    bank_registers: [u8; 8],
    selected_register: u8,
    chr_inversion: bool,
    prg_mode: bool,
    prg_banks: [usize; 4],
    chr_banks: [usize; 8],

    mirroring: Mirroring,
    mirroring_locked: bool,

    irq_latch: u8,
    irq_counter: u8,
    irq_reload: bool,
    irq_enabled: bool,
    irq_pending: bool,
}

impl Mmc3Mapper {
    pub fn new(prg_rom: Vec<u8>, chr_rom: Vec<u8>, mirroring: Mirroring) -> Self {
        let chr_is_ram = chr_rom.is_empty();
        let chr = if chr_is_ram { vec![0; 0x2000] } else { chr_rom };

        let mut bank_registers = [0u8; 8];
        bank_registers[0] = 0;
        bank_registers[1] = 2;
        bank_registers[2] = 4;
        bank_registers[3] = 5;
        bank_registers[4] = 6;
        bank_registers[5] = 7;
        bank_registers[6] = 0;
        bank_registers[7] = 1;

        let mut mapper = Mmc3Mapper {
            prg_rom,
            chr,
            chr_is_ram,
            prg_ram: vec![0; 0x2000],
            prg_ram_enable: true,
            prg_ram_write_protect: false,
            bank_registers,
            selected_register: 0,
            chr_inversion: false,
            prg_mode: false,
            prg_banks: [0; 4],
            chr_banks: [0; 8],
            mirroring: mirroring.clone(),
            mirroring_locked: matches!(mirroring, Mirroring::FourScreen),
            irq_latch: 0,
            irq_counter: 0,
            irq_reload: false,
            irq_enabled: false,
            irq_pending: false,
        };

        mapper.sync_chr_banks();
        mapper.sync_prg_banks();
        mapper
    }

    fn prg_bank_count(&self) -> usize {
        let count = self.prg_rom.len() / PRG_BANK_SIZE;
        if count == 0 { 1 } else { count }
    }

    fn chr_bank_count(&self) -> usize {
        let count = self.chr.len() / CHR_BANK_SIZE_1K;
        if count == 0 { 1 } else { count }
    }

    fn prg_bank_offset(&self, bank_index: u8) -> usize {
        if self.prg_rom.is_empty() {
            0
        } else {
            let count = self.prg_bank_count();
            let index = (bank_index as usize) % count;
            index * PRG_BANK_SIZE
        }
    }

    fn chr_bank_address(&self, value: u8, bank_size: usize) -> usize {
        if self.chr.is_empty() {
            0
        } else {
            let mut index = value as usize;
            if bank_size == CHR_BANK_SIZE_2K {
                index &= !1;
            }
            let count = self.chr_bank_count();
            index %= count;
            let base = (index * CHR_BANK_SIZE_1K) % self.chr.len();
            base & !(bank_size - 1)
        }
    }

    fn sync_prg_banks(&mut self) {
        if self.prg_rom.is_empty() {
            self.prg_banks = [0; 4];
            return;
        }

        let count = self.prg_bank_count();
        let last_bank = count - 1;
        let second_last = if count >= 2 { count - 2 } else { last_bank };
        let last_offset = last_bank * PRG_BANK_SIZE;
        let second_last_offset = second_last * PRG_BANK_SIZE;

        if self.prg_mode {
            self.prg_banks[0] = second_last_offset;
            self.prg_banks[1] = self.prg_bank_offset(self.bank_registers[7]);
            self.prg_banks[2] = self.prg_bank_offset(self.bank_registers[6]);
            self.prg_banks[3] = last_offset;
        } else {
            self.prg_banks[0] = self.prg_bank_offset(self.bank_registers[6]);
            self.prg_banks[1] = self.prg_bank_offset(self.bank_registers[7]);
            self.prg_banks[2] = second_last_offset;
            self.prg_banks[3] = last_offset;
        }
    }

    fn set_chr_pair(&mut self, slot: usize, value: u8) {
        if self.chr.is_empty() {
            self.chr_banks[slot] = 0;
            self.chr_banks[slot + 1] = 0;
            return;
        }
        let base = self.chr_bank_address(value, CHR_BANK_SIZE_2K);
        self.chr_banks[slot] = base;
        self.chr_banks[slot + 1] = (base + CHR_BANK_SIZE_1K) % self.chr.len();
    }

    fn set_chr_single(&mut self, slot: usize, value: u8) {
        if self.chr.is_empty() {
            self.chr_banks[slot] = 0;
            return;
        }
        self.chr_banks[slot] = self.chr_bank_address(value, CHR_BANK_SIZE_1K);
    }

    fn sync_chr_banks(&mut self) {
        if self.chr.is_empty() {
            self.chr_banks = [0; 8];
            return;
        }

        if !self.chr_inversion {
            self.set_chr_pair(0, self.bank_registers[0]);
            self.set_chr_pair(2, self.bank_registers[1]);
            self.set_chr_single(4, self.bank_registers[2]);
            self.set_chr_single(5, self.bank_registers[3]);
            self.set_chr_single(6, self.bank_registers[4]);
            self.set_chr_single(7, self.bank_registers[5]);
        } else {
            self.set_chr_single(0, self.bank_registers[2]);
            self.set_chr_single(1, self.bank_registers[3]);
            self.set_chr_single(2, self.bank_registers[4]);
            self.set_chr_single(3, self.bank_registers[5]);
            self.set_chr_pair(4, self.bank_registers[0]);
            self.set_chr_pair(6, self.bank_registers[1]);
        }
    }

    fn prg_addr(&self, addr: u16) -> Option<usize> {
        if self.prg_rom.is_empty() {
            return None;
        }

        let slot = match addr {
            0x8000..=0x9FFF => 0,
            0xA000..=0xBFFF => 1,
            0xC000..=0xDFFF => 2,
            0xE000..=0xFFFF => 3,
            _ => return None,
        };

        let base = self.prg_banks[slot] % self.prg_rom.len();
        let offset = (addr as usize) & (PRG_BANK_SIZE - 1);
        Some((base + offset) % self.prg_rom.len())
    }

    fn chr_addr(&self, addr: u16) -> usize {
        if self.chr.is_empty() {
            return (addr as usize) & 0x1FFF;
        }

        let slot = ((addr as usize) / CHR_BANK_SIZE_1K).min(7);
        let base = self.chr_banks[slot] % self.chr.len();
        let offset = (addr as usize) & (CHR_BANK_SIZE_1K - 1);
        (base + offset) % self.chr.len()
    }

    fn write_bank_select(&mut self, data: u8) {
        self.selected_register = data & 0x07;
        self.prg_mode = data & 0x40 != 0;
        self.chr_inversion = data & 0x80 != 0;
        self.sync_prg_banks();
        self.sync_chr_banks();
    }

    fn write_bank_data(&mut self, data: u8) {
        let target = (self.selected_register & 0x07) as usize;
        let value = if target <= 1 { data & 0xFE } else { data };
        self.bank_registers[target] = value;

        if target <= 5 {
            self.sync_chr_banks();
        } else {
            self.sync_prg_banks();
        }
    }

    fn update_mirroring(&mut self, data: u8) {
        if self.mirroring_locked {
            return;
        }
        self.mirroring = if data & 0x01 == 0 {
            Mirroring::Vertical
        } else {
            Mirroring::Horizontal
        };
    }

    fn update_prg_ram_protection(&mut self, data: u8) {
        self.prg_ram_enable = data & 0x80 != 0;
        self.prg_ram_write_protect = data & 0x40 != 0;
    }

    fn reload_irq_counter(&mut self) {
        self.irq_counter = self.irq_latch;
    }

    fn clock_irq_counter(&mut self) {
        if self.irq_reload || self.irq_counter == 0 {
            self.reload_irq_counter();
            self.irq_reload = false;
        } else {
            self.irq_counter = self.irq_counter.wrapping_sub(1);
        }

        if self.irq_counter == 0 && self.irq_enabled {
            self.irq_pending = true;
        }
    }
}

impl Mapper for Mmc3Mapper {
    fn read_prg(&self, addr: u16) -> u8 {
        match addr {
            0x6000..=0x7FFF => {
                if self.prg_ram_enable {
                    self.prg_ram[(addr - 0x6000) as usize]
                } else {
                    0xFF
                }
            }
            0x8000..=0xFFFF => {
                if let Some(index) = self.prg_addr(addr) {
                    self.prg_rom[index]
                } else {
                    0
                }
            }
            _ => 0,
        }
    }

    fn write_prg(&mut self, addr: u16, data: u8) {
        match addr {
            0x6000..=0x7FFF => {
                if self.prg_ram_enable && !self.prg_ram_write_protect {
                    let index = (addr - 0x6000) as usize;
                    self.prg_ram[index] = data;
                }
            }
            0x8000..=0x9FFF => {
                if addr & 1 == 0 {
                    self.write_bank_select(data);
                } else {
                    self.write_bank_data(data);
                }
            }
            0xA000..=0xBFFF => {
                if addr & 1 == 0 {
                    self.update_mirroring(data);
                } else {
                    self.update_prg_ram_protection(data);
                }
            }
            0xC000..=0xDFFF => {
                if addr & 1 == 0 {
                    self.irq_latch = data;
                } else {
                    self.irq_reload = true;
                }
            }
            0xE000..=0xFFFF => {
                if addr & 1 == 0 {
                    self.irq_enabled = false;
                    self.irq_pending = false;
                } else {
                    self.irq_enabled = true;
                }
            }
            _ => {}
        }
    }

    fn read_chr(&self, addr: u16, _source: ChrSource) -> u8 {
        if self.chr.is_empty() {
            0
        } else {
            let index = self.chr_addr(addr);
            self.chr[index]
        }
    }

    fn write_chr(&mut self, addr: u16, data: u8) {
        if self.chr_is_ram && !self.chr.is_empty() {
            let index = self.chr_addr(addr);
            self.chr[index] = data;
        }
    }

    fn mirroring(&self) -> Mirroring {
        self.mirroring.clone()
    }

    fn handle_scanline(&mut self, rendering_enabled: bool) {
        if rendering_enabled {
            self.clock_irq_counter();
        }
    }

    fn poll_irq(&self) -> Option<u8> {
        if self.irq_pending { Some(0) } else { None }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mapper::Mapper;

    fn patterned_prg(banks: usize) -> Vec<u8> {
        let mut data = vec![0u8; banks * PRG_BANK_SIZE];
        for bank in 0..banks {
            let start = bank * PRG_BANK_SIZE;
            for i in 0..PRG_BANK_SIZE {
                data[start + i] = bank as u8;
            }
        }
        data
    }

    #[test]
    fn prg_bank_mode_switches_slots() {
        let prg_rom = patterned_prg(4);
        let chr_rom = vec![0; 0x2000];
        let mut mapper = Mmc3Mapper::new(prg_rom, chr_rom, Mirroring::Vertical);

        mapper.write_prg(0x8000, 0x06);
        mapper.write_prg(0x8001, 0x03);
        mapper.write_prg(0x8000, 0x07);
        mapper.write_prg(0x8001, 0x00);

        assert_eq!(mapper.read_prg(0x8000), 3);
        assert_eq!(mapper.read_prg(0xA000), 0);
        assert_eq!(mapper.read_prg(0xC000), 2);
        assert_eq!(mapper.read_prg(0xE000), 3);

        mapper.write_prg(0x8000, 0x46);
        mapper.write_prg(0x8001, 0x01);

        assert_eq!(mapper.read_prg(0x8000), 2);
        assert_eq!(mapper.read_prg(0xC000), 1);
    }

    #[test]
    fn irq_counter_respects_latch_and_enable() {
        let prg_rom = patterned_prg(2);
        let chr_rom = vec![0; 0x2000];
        let mut mapper = Mmc3Mapper::new(prg_rom, chr_rom, Mirroring::Horizontal);

        mapper.write_prg(0xC000, 1);
        mapper.write_prg(0xC001, 0);
        mapper.write_prg(0xE001, 0);

        mapper.handle_scanline(true);
        assert!(mapper.poll_irq().is_none());

        mapper.handle_scanline(true);
        assert!(mapper.poll_irq().is_some());

        mapper.write_prg(0xE000, 0);
        assert!(mapper.poll_irq().is_none());

        mapper.write_prg(0xE001, 0);
        mapper.write_prg(0xC001, 0);
        mapper.handle_scanline(false);
        mapper.handle_scanline(true);
        assert!(mapper.poll_irq().is_none());
        mapper.handle_scanline(true);
        assert!(mapper.poll_irq().is_some());
    }

    #[test]
    fn irq_disable_does_not_reset_counter() {
        let prg_rom = patterned_prg(2);
        let chr_rom = vec![0; 0x2000];
        let mut mapper = Mmc3Mapper::new(prg_rom, chr_rom, Mirroring::Vertical);

        mapper.write_prg(0xC000, 2);
        mapper.write_prg(0xC001, 0);
        mapper.write_prg(0xE001, 0);

        mapper.handle_scanline(true); // counter reloads to 2
        mapper.handle_scanline(true); // counter decrements to 1
        mapper.write_prg(0xE000, 0);
        assert!(mapper.poll_irq().is_none());

        mapper.write_prg(0xE001, 0);
        mapper.handle_scanline(true);
        assert!(mapper.poll_irq().is_some());
    }

    fn patterned_chr() -> Vec<u8> {
        let mut chr = vec![0u8; 0x2000];
        for bank in 0..8 {
            let start = bank * CHR_BANK_SIZE_1K;
            for i in 0..CHR_BANK_SIZE_1K {
                chr[start + i] = bank as u8;
            }
        }
        chr
    }

    fn select_register(mapper: &mut Mmc3Mapper, reg: u8) {
        mapper.write_prg(0x8000, reg & 0x07);
    }

    #[test]
    fn chr_banks_map_correct_regions() {
        let prg_rom = vec![0; 0x8000];
        let chr_rom = patterned_chr();
        let mut mapper = Mmc3Mapper::new(prg_rom, chr_rom, Mirroring::Vertical);

        select_register(&mut mapper, 0);
        mapper.write_prg(0x8001, 0x02);
        assert_eq!(mapper.read_chr(0x0000, ChrSource::Cpu), 2);
        assert_eq!(mapper.read_chr(0x0400, ChrSource::Cpu), 3);

        select_register(&mut mapper, 2);
        mapper.write_prg(0x8001, 0x07);
        assert_eq!(mapper.read_chr(0x1000, ChrSource::Cpu), 7);

        select_register(&mut mapper, 3);
        mapper.write_prg(0x8001, 0x01);
        assert_eq!(mapper.read_chr(0x1400, ChrSource::Cpu), 1);
    }

    #[test]
    fn chr_inversion_swaps_regions() {
        let prg_rom = vec![0; 0x8000];
        let chr_rom = patterned_chr();
        let mut mapper = Mmc3Mapper::new(prg_rom, chr_rom, Mirroring::Vertical);

        mapper.write_prg(0x8000, 0x80 | 0x00);
        mapper.write_prg(0x8001, 0x04);
        assert_eq!(mapper.read_chr(0x1000, ChrSource::Cpu), 4);

        mapper.write_prg(0x8000, 0x80 | 0x01);
        mapper.write_prg(0x8001, 0x06);
        assert_eq!(mapper.read_chr(0x1800, ChrSource::Cpu), 6);
        assert_eq!(mapper.read_chr(0x1C00, ChrSource::Cpu), 7);

        mapper.write_prg(0x8000, 0x82);
        mapper.write_prg(0x8001, 0x03);
        assert_eq!(mapper.read_chr(0x0000, ChrSource::Cpu), 3);
    }
}
