use crate::{cart::Cart, joypad::Joypad, mapper::Mapper, memory::Memory, ppu::PPU};

// Address ranges per https://www.nesdev.org/wiki/CPU_memory_map
const CPU_RAM_MIRROR_MASK: u16 = 0x07FF;
const CPU_RAM_MIRRORS_END: u16 = 0x1FFF;
const PPU_REGISTERS_MIRRORS_END: u16 = 0x3FFF;
const DISABLED_APU_IO_END: u16 = 0x401F;
const CARTRIDGE_SPACE_START: u16 = 0x4020;

pub struct Bus<'call> {
    cpu_vram: [u8; 2048],
    mapper: &'call mut dyn Mapper,
    ppu: PPU<'call>,

    cycles: usize,
    gameloop_callback: Box<dyn FnMut(&PPU, &mut Joypad, &mut Joypad) + 'call>,

    joypad1: Joypad,
    joypad2: Joypad,
}

impl<'a> Bus<'a> {
    pub fn new<F>(cart: &'_ mut Cart, gameloop_callback: F) -> Bus<'_>
    where
        F: FnMut(&PPU, &mut Joypad, &mut Joypad) + 'static,
    {
        let mapper_ptr: *mut dyn Mapper = cart.mapper.as_mut() as *mut dyn Mapper;

        // Create a &mut dyn Mapper for PPU::new using unsafe from the raw pointer
        let ppu = unsafe {
            // Safety: we know cart.mapper lives for at least the lifetime 'a we claim,
            // and we ensure no simultaneous conflicting borrows at runtime.
            PPU::new(&mut *mapper_ptr)
        };

        Bus {
            cpu_vram: [0; 2048],
            mapper: unsafe { &mut *mapper_ptr },
            ppu,
            cycles: 0,
            gameloop_callback: Box::new(gameloop_callback),
            joypad1: Joypad::new(),
            joypad2: Joypad::new(),
        }
    }

    fn mirror_cpu_vram_addr(addr: u16) -> usize {
        (addr & CPU_RAM_MIRROR_MASK) as usize
    }

    fn normalize_ppu_register_addr(addr: u16) -> u16 {
        0x2000 + (addr & 0x0007)
    }
}

impl<'a> Memory for Bus<'a> {
    fn read(&mut self, addr: u16) -> u8 {
        match addr {
            0x0000..=CPU_RAM_MIRRORS_END => self.cpu_vram[Self::mirror_cpu_vram_addr(addr)],
            0x2000..=PPU_REGISTERS_MIRRORS_END => match Self::normalize_ppu_register_addr(addr) {
                0x2002 => self.ppu.read_status(),
                0x2004 => self.ppu.read_oam_data(),
                0x2007 => self.ppu.read_data(),
                _ => 0,
            },
            0x4000..=0x4013 => 0,
            0x4014 => 0,
            0x4015 => 0,
            0x4016 => self.joypad1.read(),
            0x4017 => 0, // self.joypad2.read(),
            0x4018..=DISABLED_APU_IO_END => 0,
            CARTRIDGE_SPACE_START..=0xFFFF => self.mapper.read_prg(addr),
        }
    }

    fn write(&mut self, addr: u16, data: u8) {
        match addr {
            0x0000..=CPU_RAM_MIRRORS_END => {
                self.cpu_vram[Self::mirror_cpu_vram_addr(addr)] = data;
            }
            0x2000..=PPU_REGISTERS_MIRRORS_END => match Self::normalize_ppu_register_addr(addr) {
                0x2000 => self.ppu.write_to_ctrl(data),
                0x2001 => self.ppu.write_to_mask(data),
                0x2003 => self.ppu.write_to_oam_addr(data),
                0x2004 => self.ppu.write_to_oam_data(data),
                0x2005 => self.ppu.write_to_scroll(data),
                0x2006 => self.ppu.write_to_ppu_addr(data),
                0x2007 => self.ppu.write_to_data(data),
                0x2002 => panic!("attempt to write to PPU status register"),
                _ => {}
            },
            0x4000..=0x4013 => {
                // ignore APU writes for now
            }
            0x4014 => {
                let mut buffer: [u8; 256] = [0; 256];
                let hi: u16 = (data as u16) << 8;
                for i in 0..256u16 {
                    buffer[i as usize] = self.read(hi + i);
                }

                self.ppu.write_oam_dma(&buffer);

                // todo: handle this eventually
                // let add_cycles: u16 = if self.cycles % 2 == 1 { 514 } else { 513 };
                // self.tick(add_cycles); //todo this will cause weird effects as PPU will have 513/514 * 3 ticks
            }
            0x4015 => {
                // ignore APU status
            }
            0x4016 => self.joypad1.write(data),
            0x4017 => {
                // ignore joypad 2 / APU frame counter for now
                // self.joypad2.write(data);
            }
            0x4018..=DISABLED_APU_IO_END => {
                // disabled APU and IO functionality
            }
            CARTRIDGE_SPACE_START..=0xFFFF => self.mapper.write_prg(addr, data),
        }
    }

    fn tick(&mut self, cycles: u8) {
        self.cycles += cycles as usize;
        let nmi_before = self.ppu.nmi_interrupt.is_some();
        self.ppu.tick(cycles * 3);
        let nmi_after = self.ppu.nmi_interrupt.is_some();

        if !nmi_before && nmi_after {
            (self.gameloop_callback)(&self.ppu, &mut self.joypad1, &mut self.joypad2);
        }
    }

    fn poll_nmi_status(&mut self) -> Option<u8> {
        self.ppu.poll_nmi_interrupt()
    }

    fn poll_irq_status(&mut self) -> Option<u8> {
        self.mapper.poll_irq()
    }

    fn load(&mut self, start_addr: u16, data: &[u8]) {
        for i in 0..(data.len() as u16) {
            self.write(start_addr + i, data[i as usize]);
        }
        self.write_u16(0xFFFC, start_addr);
    }
}
