const COARSE_X_MASK: u16 = 0x001f;
const COARSE_Y_MASK: u16 = 0x03e0;
const NAMETABLE_MASK: u16 = 0x0c00;
const FINE_Y_MASK: u16 = 0x7000;

/// Implements the loopy scroll registers described on https://www.nesdev.org/wiki/PPU_scrolling.
/// The structure mirrors the v/t/x/w behavior so writes to $2000/$2005/$2006 update the
/// internal scroll state the same way they do on hardware.
pub struct ScrollRegister {
    v: u16,
    t: u16,
    fine_x: u8,
    latch: bool,
}

impl Default for ScrollRegister {
    fn default() -> Self {
        Self::new()
    }
}

impl ScrollRegister {
    pub fn new() -> Self {
        ScrollRegister {
            v: 0,
            t: 0,
            fine_x: 0,
            latch: false,
        }
    }

    /// Handles writes to PPUSCROLL ($2005). Returns true after the two-write sequence finishes.
    pub fn write(&mut self, data: u8) -> bool {
        if !self.latch {
            let coarse_x = (data >> 3) as u16 & 0x1f;
            self.t = (self.t & !COARSE_X_MASK) | coarse_x;
            self.fine_x = data & 0x07;
            self.latch = true;
            false
        } else {
            let coarse_y = (data >> 3) as u16 & 0x1f;
            let fine_y = (data & 0x07) as u16;
            self.t = (self.t & !COARSE_Y_MASK) | (coarse_y << 5);
            self.t = (self.t & !FINE_Y_MASK) | (fine_y << 12);
            self.v = self.t & 0x7fff;
            self.latch = false;
            true
        }
    }

    /// Handles writes to PPUADDR ($2006). Returns true once both bytes have been written.
    pub fn write_ppu_addr(&mut self, data: u8) -> bool {
        if !self.latch {
            self.t = (self.t & 0x00ff) | (((data & 0x3f) as u16) << 8);
            self.latch = true;
            false
        } else {
            self.t = (self.t & 0x7f00) | data as u16;
            self.v = self.t & 0x7fff;
            self.latch = false;
            true
        }
    }

    /// Updates the nametable selection bits using the PPUCTRL value.
    pub fn update_ctrl(&mut self, data: u8) {
        let nametable_bits = (data & 0x03) as u16;
        let nametable_masked = nametable_bits << 10;
        self.t = (self.t & !NAMETABLE_MASK) | nametable_masked;
        self.v = (self.v & !NAMETABLE_MASK) | nametable_masked;
    }

    pub fn scroll_x(&self) -> usize {
        ((self.coarse_x() as usize) * 8 + self.fine_x as usize) % 256
    }

    pub fn scroll_y(&self) -> usize {
        (((self.coarse_y() as usize) % 30) * 8 + self.fine_y() as usize) % 240
    }

    pub fn base_nametable(&self) -> usize {
        ((self.v >> 10) & 0x03) as usize
    }

    pub fn coarse_x(&self) -> u8 {
        (self.v & 0x1f) as u8
    }

    pub fn coarse_y(&self) -> u8 {
        ((self.v >> 5) & 0x1f) as u8
    }

    pub fn fine_y(&self) -> u8 {
        ((self.v >> 12) & 0x07) as u8
    }

    pub fn fine_x(&self) -> u8 {
        self.fine_x
    }

    pub fn reset_latch(&mut self) {
        self.latch = false;
    }
}
