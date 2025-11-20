#[derive(Clone, Debug)]
pub struct ScrollRegister {
    v: u16,
    t: u16,
    x: u8,
    w: bool,
}

impl ScrollRegister {
    pub fn new() -> Self {
        ScrollRegister {
            v: 0,
            t: 0,
            x: 0,
            w: false,
        }
    }

    pub fn update_ctrl(&mut self, value: u8) {
        let nt = (value & 0b11) as u16;
        self.t = (self.t & !0x0C00) | (nt << 10);
        self.t &= 0x3FFF;
    }

    pub fn write(&mut self, value: u8) -> bool {
        if !self.w {
            let coarse_x = (value >> 3) & 0x1F;
            self.t = (self.t & !0x001F) | coarse_x as u16;

            self.x = value & 0x07;
            self.t &= 0x3FFF;

            self.w = true;
            false
        } else {
            let coarse_y = ((value >> 3) & 0x1F) as u16;
            let fine_y = (value & 0x07) as u16;

            const COARSE_Y_MASK: u16 = 0x03E0;
            const FINE_Y_MASK: u16 = 0x7000;
            self.t &= !(COARSE_Y_MASK | FINE_Y_MASK);

            self.t |= coarse_y << 5;
            self.t |= fine_y << 12;
            self.t &= 0x3FFF;

            self.w = false;
            true
        }
    }

    pub fn write_ppu_addr(&mut self, value: u8) -> bool {
        if !self.w {
            let high = (value & 0x3F) as u16;
            self.t = (self.t & 0x00FF) | (high << 8);
            self.t &= 0x3FFF;
            self.w = true;
            false
        } else {
            self.t = (self.t & 0xFF00) | (value as u16);
            self.t &= 0x3FFF;
            self.v = self.t;
            self.w = false;
            true
        }
    }

    pub fn increment(&mut self, step: u8) {
        self.v = (self.v.wrapping_add(step as u16)) & 0x3FFF;
    }

    pub fn addr(&self) -> u16 {
        self.v & 0x3FFF
    }

    pub fn scroll_x(&self) -> usize {
        let coarse_x = (self.t & 0x001F) as usize;
        (coarse_x << 3) | (self.x as usize)
    }

    pub fn scroll_y(&self) -> usize {
        let coarse_y = ((self.t >> 5) & 0x1F) as usize;
        let fine_y = ((self.t >> 12) & 0x07) as usize;
        (coarse_y << 3) | fine_y
    }

    pub fn base_nametable(&self) -> usize {
        ((self.t >> 10) & 0x03) as usize
    }

    pub fn reset_latch(&mut self) {
        self.w = false;
    }

    pub fn rendering_enabled(&self, show_bg: bool, show_sprites: bool) -> bool {
        show_bg || show_sprites
    }

    pub fn increment_x(&mut self) {
        if (self.v & 0x001F) == 31 {
            self.v &= !0x001F;
            self.v ^= 0x0400;
        } else {
            self.v += 1;
        }
    }

    pub fn increment_y(&mut self) {
        if (self.v & 0x7000) != 0x7000 {
            self.v += 0x1000;
        } else {
            self.v &= !0x7000;
            let mut y = (self.v & 0x03E0) >> 5;
            if y == 29 {
                y = 0;
                self.v ^= 0x0800;
            } else if y == 31 {
                y = 0;
            } else {
                y += 1;
            }
            self.v = (self.v & !0x03E0) | (y << 5);
        }
    }

    pub fn copy_horizontal_bits(&mut self) {
        self.v = (self.v & !0x041F) | (self.t & 0x041F);
    }

    pub fn copy_vertical_bits(&mut self) {
        self.v = (self.v & !0x7BE0) | (self.t & 0x7BE0);
    }

    pub fn v_debug(&self) -> u16 {
        self.v
    }

    pub fn t_debug(&self) -> u16 {
        self.t
    }

    pub fn fine_x_debug(&self) -> u8 {
        self.x
    }

    pub fn latch_debug(&self) -> bool {
        self.w
    }
}
