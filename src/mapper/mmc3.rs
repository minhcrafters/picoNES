use crate::cart::Mirroring;
use crate::mapper::Mapper;
use std::cell::RefCell;

pub struct Mmc3Mapper {
    prg_rom: Vec<u8>,
    chr_rom: Vec<u8>,
    prg_ram: RefCell<Vec<u8>>,
    prg_ram_enabled: RefCell<bool>, // $A001 bit: PRG RAM chip enable (0=disabled, 1=enabled)
    prg_ram_write_protect: RefCell<bool>, // $A001 bit: write-protect
    bank_select: RefCell<u8>,
    bank_registers: [RefCell<u8>; 8],
    mirroring: RefCell<u8>,
    irq_latch: RefCell<u8>,
    irq_counter: RefCell<u8>,
    irq_reload: RefCell<bool>,
    irq_enabled: RefCell<bool>,
}

impl Mmc3Mapper {
    pub fn new(prg_rom: Vec<u8>, chr_rom: Vec<u8>) -> Self {
        Mmc3Mapper {
            prg_rom,
            chr_rom,
            prg_ram: RefCell::new(vec![0; 0x2000]),
            prg_ram_enabled: RefCell::new(true),
            prg_ram_write_protect: RefCell::new(false),
            bank_select: RefCell::new(0),
            bank_registers: [
                RefCell::new(0),
                RefCell::new(0),
                RefCell::new(0),
                RefCell::new(0),
                RefCell::new(0),
                RefCell::new(0),
                RefCell::new(0),
                RefCell::new(0),
            ],
            mirroring: RefCell::new(0),
            irq_latch: RefCell::new(0),
            irq_counter: RefCell::new(0),
            irq_reload: RefCell::new(false),
            irq_enabled: RefCell::new(false),
        }
    }

    fn prg_bank_count(&self) -> usize {
        self.prg_rom.len() / 0x2000
    }

    fn chr_bank_count_1kb(&self) -> usize {
        // number of 1KB CHR banks (safe-guard zero)
        let n = self.chr_rom.len() / 0x400;
        if n == 0 { 1 } else { n }
    }
}

impl Mapper for Mmc3Mapper {
    fn read_prg(&self, addr: u16) -> u8 {
        let addr_u = addr as usize;
        match addr_u {
            0x6000..=0x7FFF => {
                // If PRG RAM is disabled, real hardware returns "open bus".
                // Here we return 0xFF to avoid a panic — adjust if you emulate open-bus.
                if !*self.prg_ram_enabled.borrow() {
                    0xFF
                } else {
                    self.prg_ram.borrow()[addr_u - 0x6000]
                }
            }
            0x8000..=0xFFFF => {
                let prg_mode = ((*self.bank_select.borrow() >> 6) & 1) as usize;
                let banks = self.prg_bank_count();
                let last = banks.saturating_sub(1);
                let second_last = banks.saturating_sub(2);

                // R6 and R7 should ignore top two bits (MMC3 has only 6 PRG ROM address lines).
                let reg6 = (*self.bank_registers[6].borrow() & 0x3F) as usize;
                let reg7 = (*self.bank_registers[7].borrow() & 0x3F) as usize;

                let bank_num = match addr_u {
                    0x8000..=0x9FFF => {
                        if prg_mode == 0 {
                            reg6
                        } else {
                            second_last
                        }
                    }
                    0xA000..=0xBFFF => reg7,
                    0xC000..=0xDFFF => {
                        if prg_mode == 0 {
                            second_last
                        } else {
                            reg6
                        }
                    }
                    0xE000..=0xFFFF => last,
                    _ => 0,
                };

                // guard against out-of-range bank numbers
                let bank_index = if banks == 0 { 0 } else { bank_num % banks };
                let offset = (addr_u & 0x1FFF) as usize;
                self.prg_rom[bank_index * 0x2000 + offset]
            }
            _ => {
                panic!("Invalid PRG read address: {:04X}", addr);
            }
        }
    }

    fn write_prg(&self, addr: u16, data: u8) {
        let a = addr as usize;
        match a {
            0x6000..=0x7FFF => {
                if *self.prg_ram_enabled.borrow() && !*self.prg_ram_write_protect.borrow() {
                    self.prg_ram.borrow_mut()[a - 0x6000] = data;
                } else {
                    // ignore writes when RAM disabled or write-protected
                }
            }
            0x8000..=0x9FFF => {
                // even = bank select, odd = bank data
                if (a & 1) == 0 {
                    // bank select: CPMx xRRR
                    *self.bank_select.borrow_mut() = data;
                } else {
                    let reg = ((*self.bank_select.borrow()) & 0x7) as usize;
                    // apply hardware masks:
                    // R0/R1 are 2KB banks and ignore LSB (bit0)
                    // R6/R7 ignore top two bits (mask 0x3F)
                    let val = match reg {
                        0 | 1 => data & !1,   // clear LSB
                        6 | 7 => data & 0x3F, // mask top two bits
                        _ => data,
                    };
                    *self.bank_registers[reg].borrow_mut() = val;
                }
            }
            0xA000..=0xBFFF => {
                if (a & 1) == 0 {
                    // nametable arrangement: 0 = horizontal, 1 = vertical
                    *self.mirroring.borrow_mut() = data & 1;
                } else {
                    // PRG RAM protect: bits control enable/protect.
                    // Bit 7 = PRG RAM chip enable (1 = enable), bit 6 = write protect (1 = protect).
                    let enable = (data & 0x80) != 0;
                    let write_protect = (data & 0x40) != 0;
                    *self.prg_ram_enabled.borrow_mut() = enable;
                    *self.prg_ram_write_protect.borrow_mut() = write_protect;
                }
            }
            0xC000..=0xDFFF => {
                if (a & 1) == 0 {
                    // IRQ latch
                    *self.irq_latch.borrow_mut() = data;
                } else {
                    // IRQ reload: writing any value clears the counter immediately and schedules reload
                    *self.irq_reload.borrow_mut() = true;
                    // clear counter immediately (spec says the counter is cleared)
                    *self.irq_counter.borrow_mut() = 0;
                }
            }
            0xE000..=0xFFFF => {
                if (a & 1) == 0 {
                    // IRQ disable and acknowledge any pending interrupt
                    *self.irq_enabled.borrow_mut() = false;
                    // many emulators also clear the pending IRQ flag here; integration with CPU needed
                } else {
                    // IRQ enable
                    *self.irq_enabled.borrow_mut() = true;
                }
            }
            _ => {}
        }
    }

    fn read_chr(&self, addr: u16) -> u8 {
        let a = addr as usize & 0x1FFF; // CHR addr is 13-bit
        let chr_mode = ((*self.bank_select.borrow() >> 7) & 1) != 0;
        let banks_1kb = self.chr_bank_count_1kb();

        // helper: safe index into chr_rom
        let index_for = |bank_1kb: usize, offset_within_1kb: usize| -> u8 {
            let bank = if banks_1kb == 0 {
                0
            } else {
                bank_1kb % banks_1kb
            };
            self.chr_rom
                .get(bank * 0x400 + offset_within_1kb)
                .copied()
                .unwrap_or(0)
        };

        if !chr_mode {
            // CHR mode 0:
            // 0x0000-0x07FF -> 2KB bank R0 (R0 ignores LSB)
            // 0x0800-0x0FFF -> 2KB bank R1 (R1 ignores LSB)
            // 0x1000-0x13FF -> 1KB R2
            // 0x1400-0x17FF -> 1KB R3
            // 0x1800-0x1BFF -> 1KB R4
            // 0x1C00-0x1FFF -> 1KB R5
            match a {
                0x0000..=0x07FF => {
                    let bank_1kb = ((*self.bank_registers[0].borrow() & !1) as usize) * 1; // value in 1KB units (even)
                    let offset = a & 0x7FF;
                    // split into two 1KB halves:
                    let half = offset / 0x400;
                    index_for(bank_1kb + half, offset & 0x3FF)
                }
                0x0800..=0x0FFF => {
                    let bank_1kb = ((*self.bank_registers[1].borrow() & !1) as usize) * 1;
                    let offset = a & 0x7FF;
                    let half = offset / 0x400;
                    index_for(bank_1kb + half, offset & 0x3FF)
                }
                0x1000..=0x13FF => {
                    let bank_1kb = *self.bank_registers[2].borrow() as usize;
                    index_for(bank_1kb, a & 0x3FF)
                }
                0x1400..=0x17FF => {
                    let bank_1kb = *self.bank_registers[3].borrow() as usize;
                    index_for(bank_1kb, a & 0x3FF)
                }
                0x1800..=0x1BFF => {
                    let bank_1kb = *self.bank_registers[4].borrow() as usize;
                    index_for(bank_1kb, a & 0x3FF)
                }
                0x1C00..=0x1FFF => {
                    let bank_1kb = *self.bank_registers[5].borrow() as usize;
                    index_for(bank_1kb, a & 0x3FF)
                }
                _ => 0,
            }
        } else {
            // CHR mode 1 (inversion):
            // 0x0000-0x03FF -> 1KB R2
            // 0x0400-0x07FF -> 1KB R3
            // 0x0800-0x0BFF -> 1KB R4
            // 0x0C00-0x0FFF -> 1KB R5
            // 0x1000-0x17FF -> 2KB R0 (R0 ignores LSB)
            // 0x1800-0x1FFF -> 2KB R1 (R1 ignores LSB)
            match a {
                0x0000..=0x03FF => {
                    let bank = *self.bank_registers[2].borrow() as usize;
                    index_for(bank, a & 0x3FF)
                }
                0x0400..=0x07FF => {
                    let bank = *self.bank_registers[3].borrow() as usize;
                    index_for(bank, a & 0x3FF)
                }
                0x0800..=0x0BFF => {
                    let bank = *self.bank_registers[4].borrow() as usize;
                    index_for(bank, a & 0x3FF)
                }
                0x0C00..=0x0FFF => {
                    let bank = *self.bank_registers[5].borrow() as usize;
                    index_for(bank, a & 0x3FF)
                }
                0x1000..=0x17FF => {
                    let bank_1kb = ((*self.bank_registers[0].borrow() & !1) as usize) * 1;
                    let offset = a & 0x7FF;
                    let half = offset / 0x400;
                    index_for(bank_1kb + half, offset & 0x3FF)
                }
                0x1800..=0x1FFF => {
                    let bank_1kb = ((*self.bank_registers[1].borrow() & !1) as usize) * 1;
                    let offset = a & 0x7FF;
                    let half = offset / 0x400;
                    index_for(bank_1kb + half, offset & 0x3FF)
                }
                _ => 0,
            }
        }
    }

    fn write_chr(&self, _addr: u16, _data: u8) {
        // CHR RAM not supported in this mapper implementation.
    }

    fn mirroring(&self) -> Mirroring {
        // $A000 even: bit0 => 0: horizontal, 1: vertical
        if *self.mirroring.borrow() == 0 {
            Mirroring::Horizontal
        } else {
            Mirroring::Vertical
        }
    }
}
