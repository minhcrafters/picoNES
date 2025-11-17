use crate::cart::Mirroring;
use crate::mapper::{ChrSource, Mapper};

const PRG_RAM_BANK_SIZE: usize = 0x2000;
const CHR_REGISTER_COUNT: usize = 12;

const SPRITE_CHR_MAP: [[usize; 8]; 4] = [
    [7, 7, 7, 7, 7, 7, 7, 7],
    [3, 3, 3, 3, 7, 7, 7, 7],
    [1, 1, 3, 3, 5, 5, 7, 7],
    [0, 1, 2, 3, 4, 5, 6, 7],
];

const BG_CHR_MAP: [[usize; 8]; 4] = [
    [11, 11, 11, 11, 11, 11, 11, 11],
    [11, 11, 11, 11, 11, 11, 11, 11],
    [9, 9, 11, 11, 9, 9, 11, 11],
    [8, 9, 10, 11, 8, 9, 10, 11],
];

pub struct Mmc5Mapper {
    prg_rom: Vec<u8>,
    chr: Vec<u8>,
    chr_is_ram: bool,
    prg_ram: Vec<u8>,
    mirroring: Mirroring,

    prg_mode: u8,
    chr_mode: u8,
    prg_bank_regs: [u8; 4],
    prg_bank_offsets: [usize; 4],
    prg_ram_bank: u8,
    prg_ram_protect: (u8, u8),
    prg_ram_write_enabled: bool,

    chr_regs: [u16; CHR_REGISTER_COUNT],
    chr_sprite_offsets: [usize; 8],
    chr_bg_offsets: [usize; 8],
    chr_upper_bits: u8,
    chr_io_background: bool,

    exram: [u8; 0x400],
    exram_mode: u8,
    nametable_mapping: [u8; 4],
    fill_tile: u8,
    fill_attr: u8,

    irq_scanline: u8,
    irq_enabled: bool,
    irq_pending: bool,
    in_frame: bool,
    current_scanline: u8,

    multiplier_a: u8,
    multiplier_b: u8,
}

impl Mmc5Mapper {
    pub fn new(prg_rom: Vec<u8>, chr_rom: Vec<u8>, mirroring: Mirroring) -> Self {
        let chr_is_ram = chr_rom.is_empty();
        let chr = if chr_is_ram { vec![0; 0x2000] } else { chr_rom };

        let mut mapper = Mmc5Mapper {
            prg_rom,
            chr,
            chr_is_ram,
            prg_ram: vec![0; 0x10000],
            mirroring,
            prg_mode: 3,
            chr_mode: 3,
            prg_bank_regs: [0; 4],
            prg_bank_offsets: [0; 4],
            prg_ram_bank: 0,
            prg_ram_protect: (0, 0),
            prg_ram_write_enabled: false,
            chr_regs: [0; CHR_REGISTER_COUNT],
            chr_sprite_offsets: [0; 8],
            chr_bg_offsets: [0; 8],
            chr_upper_bits: 0,
            chr_io_background: false,
            exram: [0; 0x400],
            exram_mode: 0,
            nametable_mapping: [0; 4],
            fill_tile: 0,
            fill_attr: 0,
            irq_scanline: 0,
            irq_enabled: false,
            irq_pending: false,
            in_frame: false,
            current_scanline: 0,
            multiplier_a: 0,
            multiplier_b: 0,
        };
        mapper.sync_prg_banks();
        mapper.sync_chr_banks();
        mapper
    }

    fn prg_bank_count(&self) -> usize {
        let count = self.prg_rom.len() / 0x2000;
        if count == 0 { 1 } else { count }
    }

    fn chr_bank_span(&self) -> usize {
        match self.chr_mode & 0x03 {
            0 => 0x2000,
            1 => 0x1000,
            2 => 0x0800,
            _ => 0x0400,
        }
    }

    fn sync_prg_banks(&mut self) {
        let mode = self.prg_mode & 0x03;
        let slot_to_register = match mode {
            0 => [3, 3, 3, 3],
            1 => [1, 1, 3, 3],
            2 => [1, 1, 2, 3],
            _ => [0, 1, 2, 3],
        };

        for (slot, reg_index) in slot_to_register.iter().enumerate() {
            let value = if (*reg_index as usize) < self.prg_bank_regs.len() {
                self.prg_bank_regs[*reg_index as usize]
            } else {
                0
            };

            let is_rom = *reg_index == 3 || (value & 0x80) != 0;
            if is_rom && !self.prg_rom.is_empty() {
                let bank = (value & 0x7F) as usize % self.prg_bank_count();
                self.prg_bank_offsets[slot] = bank * 0x2000;
            } else {
                self.prg_bank_offsets[slot] = usize::MAX;
            }
        }
    }

    fn sync_chr_banks(&mut self) {
        let size = self.chr_bank_span();
        for chunk in 0..8 {
            let sprite_reg = SPRITE_CHR_MAP[(self.chr_mode & 0x03) as usize][chunk];
            self.chr_sprite_offsets[chunk] = self.compute_chr_offset(sprite_reg, chunk, size);
            let bg_reg = BG_CHR_MAP[(self.chr_mode & 0x03) as usize][chunk];
            self.chr_bg_offsets[chunk] = self.compute_chr_offset(bg_reg, chunk, size);
        }
    }

    fn compute_chr_offset(&self, reg_index: usize, chunk: usize, span: usize) -> usize {
        if self.chr.is_empty() {
            return 0;
        }
        let value = self.chr_regs[reg_index] as usize;
        let base = (value * span) % self.chr.len();
        let chunks_per_bank = span / 0x400;
        let offset = base + ((chunk % chunks_per_bank) * 0x400);
        offset % self.chr.len()
    }

    fn resolve_prg_addr(&self, addr: u16) -> Option<usize> {
        if self.prg_rom.is_empty() {
            return None;
        }
        let slot = ((addr - 0x8000) / 0x2000) as usize;
        if slot >= self.prg_bank_offsets.len() {
            return None;
        }
        let offset = self.prg_bank_offsets[slot];
        if offset == usize::MAX {
            None
        } else {
            let within = (addr as usize) & 0x1FFF;
            Some((offset + within) % self.prg_rom.len())
        }
    }

    fn resolve_prg_ram_addr(&self, addr: u16) -> Option<usize> {
        if self.prg_ram.is_empty() {
            return None;
        }
        let bank = (self.prg_ram_bank as usize & 0x0F) % (self.prg_ram.len() / PRG_RAM_BANK_SIZE);
        let base = bank * PRG_RAM_BANK_SIZE;
        let offset = (addr - 0x6000) as usize % PRG_RAM_BANK_SIZE;
        Some(base + offset)
    }

    fn update_prg_ram_write(&mut self) {
        self.prg_ram_write_enabled = self.prg_ram_protect == (2, 1);
    }

    fn write_chr_register(&mut self, reg: usize, value: u8) {
        if reg >= CHR_REGISTER_COUNT {
            return;
        }
        let upper = (self.chr_upper_bits & 0x03) as u16;
        self.chr_regs[reg] = (upper << 8) | value as u16;
        self.chr_io_background = reg >= 8;
        self.sync_chr_banks();
    }

    fn exram_accessible(&self) -> bool {
        matches!(self.exram_mode, 0 | 1)
    }

    fn read_exram(&self, offset: usize) -> u8 {
        self.exram[offset % self.exram.len()]
    }

    fn write_exram(&mut self, offset: usize, value: u8) {
        if matches!(self.exram_mode, 0 | 1 | 2) {
            let idx = offset % self.exram.len();
            self.exram[idx] = value;
        }
    }

    fn nametable_slot(&self, addr: u16) -> (u8, usize) {
        let quadrant = ((addr - 0x2000) / 0x400) as usize & 0x03;
        let offset = (addr - 0x2000) as usize & 0x3FF;
        (self.nametable_mapping[quadrant], offset)
    }

    fn fill_value(&self, offset: usize) -> u8 {
        if offset >= 0x3C0 {
            (self.fill_attr & 0x03) * 0x55
        } else {
            self.fill_tile
        }
    }

    fn tile_exram_offset(
        &self,
        _table_index: usize,
        tile_column: usize,
        tile_row: usize,
    ) -> usize {
        ((tile_row % 30) * 32 + (tile_column % 32)) % self.exram.len()
    }
}

impl Mapper for Mmc5Mapper {
    fn read_prg(&self, addr: u16) -> u8 {
        match addr {
            0x6000..=0x7FFF => {
                if let Some(index) = self.resolve_prg_ram_addr(addr) {
                    self.prg_ram[index]
                } else {
                    0
                }
            }
            0x8000..=0xFFFF => {
                if let Some(index) = self.resolve_prg_addr(addr) {
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
            0x5100 => {
                self.prg_mode = data & 0x03;
                self.sync_prg_banks();
            }
            0x5101 => {
                self.chr_mode = data & 0x03;
                self.sync_chr_banks();
            }
            0x5102 => {
                self.prg_ram_protect.0 = data & 0x03;
                self.update_prg_ram_write();
            }
            0x5103 => {
                self.prg_ram_protect.1 = data & 0x03;
                self.update_prg_ram_write();
            }
            0x5104 => {
                self.exram_mode = data & 0x03;
            }
            0x5105 => {
                self.nametable_mapping[0] = data & 0x03;
                self.nametable_mapping[1] = (data >> 2) & 0x03;
                self.nametable_mapping[2] = (data >> 4) & 0x03;
                self.nametable_mapping[3] = (data >> 6) & 0x03;
            }
            0x5106 => self.fill_tile = data,
            0x5107 => self.fill_attr = data & 0x03,
            0x5113 => self.prg_ram_bank = data & 0x0F,
            0x5114..=0x5117 => {
                let idx = (addr - 0x5114) as usize;
                self.prg_bank_regs[idx] = data;
                self.sync_prg_banks();
            }
            0x5120..=0x512B => {
                let idx = (addr - 0x5120) as usize;
                self.write_chr_register(idx, data);
            }
            0x5130 => {
                self.chr_upper_bits = data & 0x03;
            }
            0x5203 => self.irq_scanline = data,
            0x5204 => {
                self.irq_enabled = data & 0x80 != 0;
                if !self.irq_enabled {
                    self.irq_pending = false;
                }
            }
            0x5205 => self.multiplier_a = data,
            0x5206 => self.multiplier_b = data,
            0x5C00..=0x5FFF => {
                let offset = (addr - 0x5C00) as usize;
                self.write_exram(offset, data);
            }
            0x6000..=0x7FFF => {
                if self.prg_ram_write_enabled {
                    if let Some(index) = self.resolve_prg_ram_addr(addr) {
                        self.prg_ram[index] = data;
                    }
                }
            }
            _ => {}
        }
    }

    fn read_chr(&self, addr: u16, source: ChrSource) -> u8 {
        if self.chr.is_empty() {
            return 0;
        }
        let chunk = (addr as usize) / 0x400;
        let within = (addr as usize) & 0x3FF;
        let base = match source {
            ChrSource::Sprite => self.chr_sprite_offsets[chunk],
            ChrSource::Background => self.chr_bg_offsets[chunk],
            ChrSource::Cpu => {
                if self.chr_io_background {
                    self.chr_bg_offsets[chunk]
                } else {
                    self.chr_sprite_offsets[chunk]
                }
            }
        };
        let index = (base + within) % self.chr.len();
        self.chr[index]
    }

    fn write_chr(&mut self, addr: u16, data: u8) {
        if self.chr_is_ram && !self.chr.is_empty() {
            let chunk = (addr as usize) / 0x400;
            let within = (addr as usize) & 0x3FF;
            let base = self.chr_sprite_offsets[chunk];
            let index = (base + within) % self.chr.len();
            self.chr[index] = data;
        }
    }

    fn mirroring(&self) -> Mirroring {
        self.mirroring.clone()
    }

    fn handle_scanline(&mut self, rendering_enabled: bool) {
        if rendering_enabled {
            self.in_frame = true;
            if self.irq_enabled && self.irq_scanline != 0 && self.current_scanline == self.irq_scanline
            {
                self.irq_pending = true;
            }
            self.current_scanline = self.current_scanline.wrapping_add(1);
        } else if self.in_frame {
            self.in_frame = false;
            self.current_scanline = 0;
        }
    }

    fn poll_irq(&self) -> Option<u8> {
        if self.irq_pending && self.irq_enabled {
            Some(0)
        } else {
            None
        }
    }

    fn ppu_read_nametable(&self, addr: u16, vram: &[u8]) -> Option<u8> {
        if !(0x2000..=0x3EFF).contains(&addr) {
            return None;
        }
        let (mapping, offset) = self.nametable_slot(addr);
        match mapping {
            0 => Some(vram[offset % 0x400]),
            1 => {
                let idx = 0x400 + (offset % 0x400);
                Some(vram[idx % vram.len()])
            }
            2 => {
                if self.exram_accessible() {
                    Some(self.read_exram(offset))
                } else {
                    Some(0)
                }
            }
            3 => Some(self.fill_value(offset)),
            _ => None,
        }
    }

    fn ppu_write_nametable(&mut self, addr: u16, value: u8, vram: &mut [u8]) -> bool {
        if !(0x2000..=0x3EFF).contains(&addr) {
            return false;
        }
        let (mapping, offset) = self.nametable_slot(addr);
        match mapping {
            0 => {
                vram[offset % 0x400] = value;
                true
            }
            1 => {
                let idx = 0x400 + (offset % 0x400);
                vram[idx % vram.len()] = value;
                true
            }
            2 => {
                if self.exram_accessible() {
                    self.write_exram(offset, value);
                    true
                } else {
                    false
                }
            }
            3 => true,
            _ => false,
        }
    }

    fn peek_nametable(&self, addr: u16, vram: &[u8]) -> Option<u8> {
        self.ppu_read_nametable(addr, vram)
    }

    fn background_tile_override(
        &self,
        table_index: usize,
        tile_column: usize,
        tile_row: usize,
        tile_index: u8,
        _pattern_addr: u16,
    ) -> Option<[u8; 16]> {
        if self.exram_mode != 1 || self.chr.is_empty() {
            return None;
        }
        let offset = self.tile_exram_offset(table_index, tile_column, tile_row);
        let entry = self.exram[offset];
        let bank_bits = (entry as usize) & 0x3F;
        let bank = ((self.chr_upper_bits as usize & 0x03) << 6) | bank_bits;
        let tile_number = ((bank << 8) | tile_index as usize) % (self.chr.len() / 16);
        let base = tile_number * 16;
        if base + 16 > self.chr.len() {
            return None;
        }
        let mut data = [0u8; 16];
        data.copy_from_slice(&self.chr[base..base + 16]);
        Some(data)
    }

    fn background_palette_override(
        &self,
        table_index: usize,
        tile_column: usize,
        tile_row: usize,
    ) -> Option<u8> {
        if self.exram_mode != 1 {
            return None;
        }
        let offset = self.tile_exram_offset(table_index, tile_column, tile_row);
        let entry = self.exram[offset];
        Some((entry >> 6) & 0x03)
    }
}
