use std::fs::File;
use std::io::Write;

pub struct Memory {
    data: [u8; 0x10000],
}

impl Memory {
    pub fn new() -> Self {
        Memory { data: [0; 0x10000] }
    }

    pub fn read(&mut self, addr: u16) -> u8 {
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
        self.data[start..end].copy_from_slice(data);
    }

    pub fn clear(&mut self) {
        self.data = [0; 0x10000];
    }

    pub fn dump_to_file(&self, path: &str) -> std::io::Result<()> {
        let mut file = File::create(path)?;
        let bytes_per_row = 16usize;

        for (row_idx, chunk) in self.data.chunks(bytes_per_row).enumerate() {
            let offset = row_idx * bytes_per_row;
            write!(file, "{:08x}: ", offset)?;

            for j in 0..bytes_per_row {
                if j < chunk.len() {
                    write!(file, "{:02x} ", chunk[j])?;
                } else {
                    write!(file, "   ")?;
                }
                if j == 7 {
                    write!(file, " ")?;
                }
            }

            write!(file, " |")?;
            for &b in chunk {
                let ch = if (0x20..=0x7e).contains(&b) {
                    b as char
                } else {
                    '.'
                };
                write!(file, "{}", ch)?;
            }
            writeln!(file, "|")?;
        }

        Ok(())
    }
}
