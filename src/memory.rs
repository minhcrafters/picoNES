pub struct Memory {
    data: [u8; 0xFFFF],
}

impl Memory {
    pub fn new() -> Self {
        Memory { data: [0; 0xFFFF] }
    }

    pub fn read(&self, addr: u16) -> u8 {
        self.data[addr as usize]
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        self.data[addr as usize] = value;
    }

    pub fn read_u16(&mut self, addr: u16) -> u16 {
        let lo = self.read(addr) as u16;
        let hi = self.read(addr + 1) as u16;
        (hi << 8) | lo
    }

    pub fn write_u16(&mut self, addr: u16, value: u16) {
        let lo = (value & 0xFF) as u8;
        let hi = (value >> 8) as u8;
        self.write(addr, lo);
        self.write(addr + 1, hi);
    }

    pub fn load(&mut self, start_addr: u16, data: &[u8]) {
        let start = start_addr as usize;
        let end = start + data.len();
        self.data[start..end].copy_from_slice(&data[..]);
    }

    pub fn clear(&mut self) {
        self.data = [0; 0xFFFF];
    }
}
