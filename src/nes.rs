use crate::{apu::APU, bus::Bus, cart::Cart, cpu::CPU, joypad::Joypad, mapper::Mapper};

pub struct ClockResult {
    pub frame_complete: bool,
    pub instruction_complete: bool,
}

pub struct Nes {
    pub cpu: CPU,
    pub bus: Bus,
    pub system_clock: u64,
}

impl Nes {
    pub fn new(cart: Cart, apu: APU) -> Self {
        Nes {
            cpu: CPU::new(),
            bus: Bus::new(cart, apu),
            system_clock: 0,
        }
    }

    pub fn reset(&mut self) {
        self.cpu.reset(&mut self.bus);
    }

    pub fn clock(&mut self) -> ClockResult {
        let frame_complete = self.bus.clock_ppu();
        let mut instruction_complete = false;

        if self.system_clock % 3 == 0 {
            instruction_complete = self.cpu.clock(&mut self.bus);
            self.bus.clock_apu();
        }

        if self.bus.poll_nmi() {
            self.cpu.nmi(&mut self.bus);
        }

        if self.bus.poll_irq() {
            self.cpu.irq(&mut self.bus);
        }

        self.system_clock = self.system_clock.wrapping_add(1);

        ClockResult {
            frame_complete,
            instruction_complete,
        }
    }

    pub fn step_frame(&mut self) {
        let start_frame = self.bus.ppu.frame_count;
        while self.bus.ppu.frame_count == start_frame {
            self.clock();
        }
    }

    pub fn joypad_mut(&mut self, index: usize) -> Option<&mut Joypad> {
        self.bus.joypad_mut(index)
    }

    pub fn mapper_mut(&mut self) -> &mut dyn Mapper {
        self.bus.mapper_mut()
    }

    pub fn joypads_mut(&mut self) -> (&mut Joypad, &mut Joypad) {
        self.bus.joypads_mut()
    }
}
