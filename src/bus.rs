use crate::{
    apu::APU,
    cart::Cart,
    cpu::CPU,
    joypad::Joypad,
    mapper::Mapper,
    memory::Memory,
    ppu::{PPU, framebuffer::Framebuffer, render},
};

// Address ranges per https://www.nesdev.org/wiki/CPU_memory_map
const CPU_RAM_MIRROR_MASK: u16 = 0x07FF;
const CPU_RAM_MIRRORS_END: u16 = 0x1FFF;
const PPU_REGISTERS_MIRRORS_END: u16 = 0x3FFF;
const DISABLED_APU_IO_END: u16 = 0x401F;
const CARTRIDGE_SPACE_START: u16 = 0x4020;

pub struct Bus {
    pub cpu: CPU,
    pub cart: Cart,
    pub ppu: PPU,
    pub apu: APU,
    joypads: [Joypad; 2],
}

impl Bus {
    pub fn new(cart: Cart, apu: APU) -> Bus {
        Bus {
            cpu: CPU::new(),
            cart,
            ppu: PPU::new(),
            apu,
            joypads: [Joypad::new(), Joypad::new()],
        }
    }

    fn mirror_cpu_vram_addr(addr: u16) -> usize {
        (addr & CPU_RAM_MIRROR_MASK) as usize
    }

    fn normalize_ppu_register_addr(addr: u16) -> u16 {
        addr & 0b00100000_00000111
    }

    pub fn mapper_mut(&mut self) -> &mut dyn Mapper {
        self.cart.mapper.as_mut()
    }

    pub fn joypad_mut(&mut self, idx: usize) -> Option<&mut Joypad> {
        self.joypads.get_mut(idx)
    }

    pub fn joypad(&self, idx: usize) -> Option<&Joypad> {
        self.joypads.get(idx)
    }

    pub fn joypads_mut(&mut self) -> (&mut Joypad, &mut Joypad) {
        let (left, right) = self.joypads.split_at_mut(1);
        (&mut left[0], &mut right[0])
    }

    pub fn ppu_clock(&mut self) -> bool {
        let mapper = self.cart.mapper.as_mut();
        self.ppu.clock(mapper)
    }

    pub fn apu_clock(&mut self) {
        if let Some(addr) = self.apu.clock() {
            let value = self.read(addr);
            self.apu.provide_dmc_sample(value);
        }
    }

    pub fn poll_nmi(&mut self) -> bool {
        self.ppu.poll_nmi_interrupt().is_some()
    }

    pub fn poll_irq(&mut self) -> bool {
        self.apu.poll_irq().is_some() || self.cart.mapper.poll_irq().is_some()
    }

    pub fn peek(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=CPU_RAM_MIRRORS_END => self.cpu.vram[Self::mirror_cpu_vram_addr(addr)],
            CARTRIDGE_SPACE_START..=0xFFFF => self.cart.mapper.peek_prg(addr),
            _ => 0,
        }
    }

    pub fn render_frame(&mut self, framebuffer: &mut Framebuffer) {
        let mapper = self.cart.mapper.as_mut();
        render::render(&self.ppu, mapper, framebuffer);
        self.ppu.reset_scroll_segments_for_new_frame();
    }

    pub fn cpu_clock(&mut self) -> bool {
        let cpu_ptr = std::ptr::addr_of_mut!(self.cpu);
        unsafe { (*cpu_ptr).clock(self) }
    }

    pub fn cpu_reset(&mut self) {
        let cpu_ptr = std::ptr::addr_of_mut!(self.cpu);
        unsafe { (*cpu_ptr).reset(self) }
    }

    pub fn cpu_nmi(&mut self) {
        let cpu_ptr = std::ptr::addr_of_mut!(self.cpu);
        unsafe { (*cpu_ptr).nmi(self) }
    }

    pub fn cpu_irq(&mut self) {
        let cpu_ptr = std::ptr::addr_of_mut!(self.cpu);
        unsafe { (*cpu_ptr).irq(self) }
    }
}

impl Memory for Bus {
    fn read(&mut self, addr: u16) -> u8 {
        match addr {
            0x0000..=CPU_RAM_MIRRORS_END => self.cpu.vram[Self::mirror_cpu_vram_addr(addr)],
            0x2000..=PPU_REGISTERS_MIRRORS_END => match Self::normalize_ppu_register_addr(addr) {
                0x2002 => self.ppu.read_status(),
                0x2004 => self.ppu.read_oam_data(),
                0x2007 => {
                    let mapper = self.cart.mapper.as_mut();
                    self.ppu.read_data(mapper)
                }
                _ => 0,
            },
            0x4000..=0x4013 => 0,
            0x4014 => 0,
            0x4015 => self.apu.read_status(),
            0x4016 => self.joypads[0].read(),
            0x4017 => self.joypads[1].read(),
            0x4018..=DISABLED_APU_IO_END => 0,
            CARTRIDGE_SPACE_START..=0xFFFF => self.cart.mapper.read_prg(addr),
        }
    }

    fn write(&mut self, addr: u16, data: u8) {
        match addr {
            0x0000..=CPU_RAM_MIRRORS_END => {
                self.cpu.vram[Self::mirror_cpu_vram_addr(addr)] = data;
            }
            0x2000..=PPU_REGISTERS_MIRRORS_END => {
                let reg = Self::normalize_ppu_register_addr(addr);

                // if reg == 0x2000 || reg == 0x2005 || reg == 0x2006 {
                //     eprintln!(
                //         "[PPU WRITE] addr={:04X} norm={:04X} data={:02X}",
                //         addr, reg, data
                //     );
                // }

                match reg {
                    0x2000 => self.ppu.write_to_ctrl(data),
                    0x2001 => self.ppu.write_to_mask(data),
                    0x2003 => self.ppu.write_to_oam_addr(data),
                    0x2004 => self.ppu.write_to_oam_data(data),
                    0x2005 => self.ppu.write_to_scroll(data),
                    0x2006 => self.ppu.write_to_ppu_addr(data),
                    0x2007 => {
                        let mapper = self.cart.mapper.as_mut();
                        self.ppu.write_to_data(mapper, data);
                    }
                    _ => {}
                }
            }
            0x4000..=0x4013 => {
                self.apu.write_register(addr, data);
            }
            0x4014 => {
                let mut buffer: [u8; 256] = [0; 256];
                let hi: u16 = (data as u16) << 8;
                for i in 0..256u16 {
                    buffer[i as usize] = self.read(hi + i);
                }

                self.ppu.write_oam_dma(&buffer);
            }
            0x4015 => {
                self.apu.write_status(data);
            }
            0x4016 => {
                self.joypads[0].write(data);
                self.joypads[1].write(data);
            }
            0x4017 => {
                self.apu.write_frame_counter(data);
            }
            0x4018..=DISABLED_APU_IO_END => {
                // disabled APU and IO functionality
            }
            CARTRIDGE_SPACE_START..=0xFFFF => self.cart.mapper.write_prg(addr, data),
        }
    }
}
