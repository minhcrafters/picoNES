pub struct ScrollRegister {
    pub fine_x: u8,
    pub coarse_x: u8,
    pub fine_y: u8,
    pub coarse_y: u8,
    pub latch: bool,
}

impl ScrollRegister {
    pub fn new() -> Self {
        ScrollRegister {
            fine_x: 0,
            coarse_x: 0,
            fine_y: 0,
            coarse_y: 0,
            latch: false,
        }
    }

    pub fn write(&mut self, data: u8) {
        if !self.latch {
            self.coarse_x = data >> 3;
            self.fine_x = data & 0x07;
        } else {
            self.coarse_y = data >> 3;
            self.fine_y = data & 0x07;
        }
        self.latch = !self.latch;
    }

    pub fn reset_latch(&mut self) {
        self.latch = false;
    }
}
