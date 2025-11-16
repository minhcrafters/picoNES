pub struct Framebuffer {
    pub data: Vec<u8>,
}

impl Framebuffer {
    pub const WIDTH: usize = 256;
    pub const HEIGHT: usize = 240;

    pub fn new() -> Self {
        Framebuffer {
            data: vec![0; (Framebuffer::WIDTH) * (Framebuffer::HEIGHT) * 3],
        }
    }

    pub fn set_pixel(&mut self, x: usize, y: usize, rgb: (u8, u8, u8)) {
        let base = y * 3 * Framebuffer::WIDTH + x * 3;
        if base + 2 < self.data.len() {
            self.data[base] = rgb.0;
            self.data[base + 1] = rgb.1;
            self.data[base + 2] = rgb.2;
        }
    }
}
