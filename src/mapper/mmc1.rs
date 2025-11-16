use crate::cart::Mirroring;
use crate::mapper::Mapper;
use std::cell::RefCell;

pub struct Mmc1Mapper {
    prg_rom: Vec<u8>,
    chr_rom: Vec<u8>,
    prg_ram: RefCell<Vec<u8>>,
    shift_register: RefCell<u8>,
    control: RefCell<u8>,
    chr_bank0: RefCell<u8>,
    chr_bank1: RefCell<u8>,
    prg_bank: RefCell<u8>,
}

impl Mmc1Mapper {
    pub fn new(prg_rom: Vec<u8>, chr_rom: Vec<u8>) -> Self {
        Mmc1Mapper {
            prg_rom,
            chr_rom,
            prg_ram: RefCell::new(vec![0; 0x2000]),
            shift_register: RefCell::new(0x10),
            control: RefCell::new(0x0C),
            chr_bank0: RefCell::new(0),
            chr_bank1: RefCell::new(0),
            prg_bank: RefCell::new(0),
        }
    }

    fn prg_bank_mode(&self) -> u8 {
        *self.control.borrow() & 0x0C
    }

    fn chr_bank_mode(&self) -> bool {
        *self.control.borrow() & 0x10 != 0
    }

    fn mirroring_mode(&self) -> u8 {
        *self.control.borrow() & 0x03
    }
}

impl Mapper for Mmc1Mapper {
    fn read_prg(&self, addr: u16) -> u8 {
        let addr = addr as usize;
        match addr {
            0x6000..=0x7FFF => self.prg_ram.borrow()[addr - 0x6000],
            0x8000..=0xFFFF => {
                let bank = match self.prg_bank_mode() {
                    0x00 | 0x04 => {
                        // 32KB
                        let bank = *self.prg_bank.borrow() & 0xFE;
                        (bank as usize / 2) * 0x8000 + (addr - 0x8000)
                    }
                    0x08 => {
                        // 16KB fixed at 0xC000
                        if addr < 0xC000 {
                            *self.prg_bank.borrow() as usize * 0x4000 + (addr - 0x8000)
                        } else {
                            (self.prg_rom.len() - 0x4000) + (addr - 0xC000)
                        }
                    }
                    0x0C => {
                        // 16KB fixed at 0x8000
                        if addr < 0xC000 {
                            (self.prg_rom.len() - 0x8000) + (addr - 0x8000)
                        } else {
                            *self.prg_bank.borrow() as usize * 0x4000 + (addr - 0xC000)
                        }
                    }
                    _ => panic!("Invalid PRG bank mode"),
                };
                self.prg_rom[bank]
            }
            _ => panic!("Invalid PRG read address: {:x}", addr),
        }
    }

    fn write_prg(&self, addr: u16, data: u8) {
        let addr = addr as usize;
        match addr {
            0x6000..=0x7FFF => {
                self.prg_ram.borrow_mut()[addr - 0x6000] = data;
            }
            0x8000..=0xFFFF => {
                if data & 0x80 != 0 {
                    // Reset
                    *self.shift_register.borrow_mut() = 0x10;
                    *self.control.borrow_mut() |= 0x0C;
                } else {
                    let mut shift = self.shift_register.borrow_mut();
                    *shift >>= 1;
                    *shift |= (data & 1) << 4;
                    if *shift & 1 != 0 {
                        // Apply
                        let value = *shift >> 1;
                        let reg = (addr >> 13) & 0x3;
                        match reg {
                            0 => *self.control.borrow_mut() = value,
                            1 => *self.chr_bank0.borrow_mut() = value,
                            2 => *self.chr_bank1.borrow_mut() = value,
                            3 => *self.prg_bank.borrow_mut() = value,
                            _ => {}
                        }
                        *shift = 0x10;
                    }
                }
            }
            _ => {}
        }
    }

    fn read_chr(&self, addr: u16) -> u8 {
        let addr = addr as usize;
        let bank = if self.chr_bank_mode() {
            // 4KB
            if addr < 0x1000 {
                *self.chr_bank0.borrow() as usize * 0x1000 + addr
            } else {
                *self.chr_bank1.borrow() as usize * 0x1000 + (addr - 0x1000)
            }
        } else {
            // 8KB
            let bank = *self.chr_bank0.borrow() & 0xFE;
            (bank as usize / 2) * 0x2000 + addr
        };
        self.chr_rom[bank]
    }

    fn write_chr(&self, _addr: u16, _data: u8) {
        // CHR ROM, ignore
    }

    fn mirroring(&self) -> Mirroring {
        match self.mirroring_mode() {
            0 => Mirroring::FourScreen, // One screen lower, but approximate
            1 => Mirroring::FourScreen, // One screen upper
            2 => Mirroring::Vertical,
            3 => Mirroring::Horizontal,
            _ => Mirroring::Horizontal,
        }
    }
}
