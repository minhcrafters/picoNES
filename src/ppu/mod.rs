pub mod framebuffer;
pub mod palette;
pub mod registers;
pub mod render;

use crate::cart::Mirroring;
use crate::mapper::Mapper;
use registers::addr::AddrRegister;
use registers::control::ControlRegister;
use registers::mask::MaskRegister;
use registers::scroll::ScrollRegister;
use registers::status::StatusRegister;

#[derive(Clone, Debug)]
pub struct ScrollSegment {
    pub start_scanline: usize,
    pub scroll_x: usize,
    pub scroll_y: usize,
    pub base_nametable: usize,
}

pub struct PPU<'a> {
    pub mapper: &'a mut dyn Mapper,
    pub ctrl: ControlRegister,
    pub mask: MaskRegister,
    pub status: StatusRegister,
    pub scroll: ScrollRegister,
    pub addr: AddrRegister,
    pub vram: [u8; 2048],

    pub oam_addr: u8,
    pub oam_data: [u8; 256],
    pub palette_table: [u8; 32],

    pub nmi_interrupt: Option<u8>,

    scanline: u16,
    cycles: usize,

    internal_data_buf: u8,
    scroll_segments: Vec<ScrollSegment>,
    pending_scroll_descriptor: Option<(usize, usize, usize)>,
}

impl<'a> PPU<'a> {
    pub fn empty(mapper: &'a mut dyn Mapper) -> Self {
        PPU::new(mapper)
    }

    pub fn new(mapper: &'a mut dyn Mapper) -> Self {
        let mut ppu = PPU {
            mapper,
            ctrl: ControlRegister::new(),
            mask: MaskRegister::new(),
            status: StatusRegister::new(),
            oam_addr: 0,
            scroll: ScrollRegister::new(),
            addr: AddrRegister::new(),
            vram: [0; 2048],
            oam_data: [0; 64 * 4],
            palette_table: [0; 32],
            nmi_interrupt: None,
            scanline: 0,
            cycles: 0,
            internal_data_buf: 0,
            scroll_segments: Vec::new(),
            pending_scroll_descriptor: None,
        };

        ppu.reset_scroll_segments_for_new_frame();
        ppu
    }

    // Horizontal:
    //   [ A ] [ a ]
    //   [ B ] [ b ]

    // Vertical:
    //   [ A ] [ B ]
    //   [ a ] [ b ]
    pub fn mirror_vram_addr(&self, addr: u16) -> u16 {
        let mirrored_vram = addr & 0b10111111111111; // mirror down 0x3000-0x3eff to 0x2000 - 0x2eff
        let vram_index = mirrored_vram - 0x2000; // to vram vector
        let name_table = vram_index / 0x400;
        let mirroring = self.mapper.mirroring();
        match (mirroring, name_table) {
            (Mirroring::Vertical, 2) | (Mirroring::Vertical, 3) => vram_index - 0x800,
            (Mirroring::Horizontal, 2) => vram_index - 0x400,
            (Mirroring::Horizontal, 1) => vram_index - 0x400,
            (Mirroring::Horizontal, 3) => vram_index - 0x800,
            (Mirroring::SingleScreenLower, _) => vram_index & 0x03FF,
            (Mirroring::SingleScreenUpper, _) => (vram_index & 0x03FF) + 0x400,
            _ => vram_index,
        }
    }

    fn increment_vram_addr(&mut self) {
        self.addr.increment(self.ctrl.vram_addr_increment());
    }

    pub fn scroll_segments(&self) -> &[ScrollSegment] {
        &self.scroll_segments
    }

    fn current_scroll_descriptor(&self) -> (usize, usize, usize) {
        let scroll_x = self.scroll.scroll_x();
        let scroll_y = self.scroll.scroll_y();
        let base_nametable = self.scroll.base_nametable();
        (scroll_x, scroll_y, base_nametable)
    }

    fn visible_scanline(&self) -> Option<usize> {
        if (self.scanline as usize) < 240 {
            Some(self.scanline as usize)
        } else {
            None
        }
    }

    fn push_scroll_segment(&mut self, descriptor: (usize, usize, usize), scanline: usize) {
        let (scroll_x, scroll_y, base_nametable) = descriptor;
        if let Some(last) = self.scroll_segments.last_mut() {
            if last.start_scanline == scanline {
                *last = ScrollSegment {
                    start_scanline: scanline,
                    scroll_x,
                    scroll_y,
                    base_nametable,
                };
                return;
            }

            if last.scroll_x == scroll_x
                && last.scroll_y == scroll_y
                && last.base_nametable == base_nametable
            {
                return;
            }
        }

        self.scroll_segments.push(ScrollSegment {
            start_scanline: scanline,
            scroll_x,
            scroll_y,
            base_nametable,
        });
    }

    fn queue_scroll_state_change(&mut self) {
        let descriptor = self.current_scroll_descriptor();
        if let Some(scanline) = self.visible_scanline() {
            self.push_scroll_segment(descriptor, scanline.min(239));
        } else {
            self.pending_scroll_descriptor = Some(descriptor);
        }
    }

    fn reset_scroll_segments_for_new_frame(&mut self) {
        let descriptor = self
            .pending_scroll_descriptor
            .take()
            .unwrap_or_else(|| self.current_scroll_descriptor());
        self.scroll_segments.clear();
        self.scroll_segments.push(ScrollSegment {
            start_scanline: 0,
            scroll_x: descriptor.0,
            scroll_y: descriptor.1,
            base_nametable: descriptor.2,
        });
    }
}

impl<'a> PPU<'a> {
    pub fn write_to_ctrl(&mut self, value: u8) {
        let before_nmi_status = self.ctrl.generate_vblank_nmi();
        self.ctrl.update(value);
        self.scroll.update_ctrl(value);
        if !before_nmi_status && self.ctrl.generate_vblank_nmi() && self.status.is_in_vblank() {
            self.nmi_interrupt = Some(1);
        }
        self.queue_scroll_state_change();
    }

    pub fn write_to_mask(&mut self, value: u8) {
        self.mask.update(value);
    }

    pub fn read_status(&mut self) -> u8 {
        let data = self.status.snapshot();
        self.status.reset_vblank_status();
        self.addr.reset_latch();
        self.scroll.reset_latch();
        data
    }

    pub fn write_to_oam_addr(&mut self, value: u8) {
        self.oam_addr = value;
    }

    pub fn write_to_oam_data(&mut self, value: u8) {
        self.oam_data[self.oam_addr as usize] = value;
        self.oam_addr = self.oam_addr.wrapping_add(1);
    }

    pub fn read_oam_data(&self) -> u8 {
        self.oam_data[self.oam_addr as usize]
    }

    pub fn write_to_scroll(&mut self, value: u8) {
        let completed_sequence = self.scroll.write(value);
        if completed_sequence {
            self.queue_scroll_state_change();
        }
    }

    pub fn write_to_ppu_addr(&mut self, value: u8) {
        self.addr.update(value);
        let completed_sequence = self.scroll.write_ppu_addr(value);
        if completed_sequence {
            self.queue_scroll_state_change();
        }
    }

    pub fn write_to_data(&mut self, value: u8) {
        let addr = self.addr.get();
        match addr {
            0..=0x1fff => self.mapper.write_chr(addr, value),
            0x2000..=0x2fff => {
                self.vram[self.mirror_vram_addr(addr) as usize] = value;
            }
            0x3000..=0x3eff => unimplemented!("addr {} shouldn't be used in reallity", addr),

            //Addresses $3F10/$3F14/$3F18/$3F1C are mirrors of $3F00/$3F04/$3F08/$3F0C
            0x3f10 | 0x3f14 | 0x3f18 | 0x3f1c => {
                let add_mirror = addr - 0x10;
                self.palette_table[(add_mirror - 0x3f00) as usize] = value;
            }
            0x3f00..=0x3fff => {
                self.palette_table[(addr - 0x3f00) as usize] = value;
            }
            _ => panic!("unexpected access to mirrored space {}", addr),
        }
        self.increment_vram_addr();
    }

    pub fn read_data(&mut self) -> u8 {
        let addr = self.addr.get();

        self.increment_vram_addr();

        match addr {
            0..=0x1fff => {
                let result = self.internal_data_buf;
                self.internal_data_buf = self.mapper.read_chr(addr);
                result
            }
            0x2000..=0x2fff => {
                let result = self.internal_data_buf;
                self.internal_data_buf = self.vram[self.mirror_vram_addr(addr) as usize];
                result
            }
            0x3000..=0x3eff => unimplemented!("addr {} shouldn't be used really", addr),

            //Addresses $3F10/$3F14/$3F18/$3F1C are mirrors of $3F00/$3F04/$3F08/$3F0C
            0x3f10 | 0x3f14 | 0x3f18 | 0x3f1c => {
                let add_mirror = addr - 0x10;
                self.palette_table[(add_mirror - 0x3f00) as usize]
            }

            0x3f00..=0x3fff => self.palette_table[(addr - 0x3f00) as usize],
            _ => panic!("unexpected access to mirrored space {}", addr),
        }
    }

    pub fn write_oam_dma(&mut self, data: &[u8; 256]) {
        for x in data.iter() {
            self.oam_data[self.oam_addr as usize] = *x;
            self.oam_addr = self.oam_addr.wrapping_add(1);
        }
    }

    pub fn tick(&mut self, cycles: u8) -> bool {
        self.cycles += cycles as usize;
        if self.cycles >= 341 {
            if self.is_sprite_zero_hit(self.cycles) {
                self.status.set_sprite_zero_hit(true);
            }

            self.cycles = self.cycles - 341;
            self.scanline += 1;

            if self.scanline == 241 {
                self.status.set_vblank_status(true);
                self.status.set_sprite_zero_hit(false);
                if self.ctrl.generate_vblank_nmi() {
                    self.nmi_interrupt = Some(1);
                }
            }

            if self.scanline >= 262 {
                self.scanline = 0;
                self.nmi_interrupt = None;
                self.status.set_sprite_zero_hit(false);
                self.status.reset_vblank_status();
                self.reset_scroll_segments_for_new_frame();
                return true;
            }
        }
        return false;
    }

    pub fn poll_nmi_interrupt(&mut self) -> Option<u8> {
        self.nmi_interrupt.take()
    }

    fn is_sprite_zero_hit(&self, cycle: usize) -> bool {
        let y = self.oam_data[0] as usize;
        let x = self.oam_data[3] as usize;
        (y == self.scanline as usize) && x <= cycle && self.mask.show_sprites()
    }
}

#[cfg(test)]
pub mod test {
    use crate::mapper::nrom::NromMapper;

    use super::*;

    #[test]
    fn test_ppu_vram_writes() {
        let mut mapper = NromMapper::new(vec![], vec![], Mirroring::Horizontal);
        let mut ppu = PPU::empty(&mut mapper);
        ppu.write_to_ppu_addr(0x23);
        ppu.write_to_ppu_addr(0x05);
        ppu.write_to_data(0x66);

        assert_eq!(ppu.vram[0x0305], 0x66);
    }

    #[test]
    fn test_ppu_vram_reads() {
        let mut mapper = NromMapper::new(vec![], vec![], Mirroring::Horizontal);
        let mut ppu = PPU::empty(&mut mapper);
        ppu.write_to_ctrl(0);
        ppu.vram[0x0305] = 0x66;

        ppu.write_to_ppu_addr(0x23);
        ppu.write_to_ppu_addr(0x05);

        ppu.read_data(); //load_into_buffer
        assert_eq!(ppu.addr.get(), 0x2306);
        assert_eq!(ppu.read_data(), 0x66);
    }

    #[test]
    fn test_ppu_vram_reads_cross_page() {
        let mut mapper = NromMapper::new(vec![], vec![], Mirroring::Horizontal);
        let mut ppu = PPU::empty(&mut mapper);
        ppu.write_to_ctrl(0);
        ppu.vram[0x01ff] = 0x66;
        ppu.vram[0x0200] = 0x77;

        ppu.write_to_ppu_addr(0x21);
        ppu.write_to_ppu_addr(0xff);

        ppu.read_data(); //load_into_buffer
        assert_eq!(ppu.read_data(), 0x66);
        assert_eq!(ppu.read_data(), 0x77);
    }

    #[test]
    fn test_ppu_vram_reads_step_32() {
        let mut mapper = NromMapper::new(vec![], vec![], Mirroring::Horizontal);
        let mut ppu = PPU::empty(&mut mapper);
        ppu.write_to_ctrl(0b100);
        ppu.vram[0x01ff] = 0x66;
        ppu.vram[0x01ff + 32] = 0x77;
        ppu.vram[0x01ff + 64] = 0x88;

        ppu.write_to_ppu_addr(0x21);
        ppu.write_to_ppu_addr(0xff);

        ppu.read_data(); //load_into_buffer
        assert_eq!(ppu.read_data(), 0x66);
        assert_eq!(ppu.read_data(), 0x77);
        assert_eq!(ppu.read_data(), 0x88);
    }

    // Horizontal: https://wiki.nesdev.com/w/index.php/Mirroring
    //   [0x2000 A ] [0x2400 a ]
    //   [0x2800 B ] [0x2C00 b ]
    #[test]
    fn test_vram_horizontal_mirror() {
        let mut mapper = NromMapper::new(vec![], vec![], Mirroring::Horizontal);
        let mut ppu = PPU::empty(&mut mapper);
        ppu.write_to_ppu_addr(0x24);
        ppu.write_to_ppu_addr(0x05);

        ppu.write_to_data(0x66); //write to a

        ppu.write_to_ppu_addr(0x28);
        ppu.write_to_ppu_addr(0x05);

        ppu.write_to_data(0x77); //write to B

        ppu.write_to_ppu_addr(0x20);
        ppu.write_to_ppu_addr(0x05);

        ppu.read_data(); //load into buffer
        assert_eq!(ppu.read_data(), 0x66); //read from A

        ppu.write_to_ppu_addr(0x2C);
        ppu.write_to_ppu_addr(0x05);

        ppu.read_data(); //load into buffer
        assert_eq!(ppu.read_data(), 0x77); //read from b
    }

    // Vertical: https://wiki.nesdev.com/w/index.php/Mirroring
    //   [0x2000 A ] [0x2400 B ]
    //   [0x2800 a ] [0x2C00 b ]
    #[test]
    fn test_vram_vertical_mirror() {
        let mut mapper = NromMapper::new(vec![], vec![0; 2048], Mirroring::Vertical);
        let mut ppu = PPU::empty(&mut mapper);

        ppu.write_to_ppu_addr(0x20);
        ppu.write_to_ppu_addr(0x05);

        ppu.write_to_data(0x66); //write to A

        ppu.write_to_ppu_addr(0x2C);
        ppu.write_to_ppu_addr(0x05);

        ppu.write_to_data(0x77); //write to b

        ppu.write_to_ppu_addr(0x28);
        ppu.write_to_ppu_addr(0x05);

        ppu.read_data(); //load into buffer
        assert_eq!(ppu.read_data(), 0x66); //read from a

        ppu.write_to_ppu_addr(0x24);
        ppu.write_to_ppu_addr(0x05);

        ppu.read_data(); //load into buffer
        assert_eq!(ppu.read_data(), 0x77); //read from B
    }

    #[test]
    fn test_vram_single_screen_lower_mirror() {
        let mut mapper = NromMapper::new(vec![], vec![0; 2048], Mirroring::SingleScreenLower);
        let ppu = PPU::empty(&mut mapper);

        assert_eq!(ppu.mirror_vram_addr(0x2000), 0);
        assert_eq!(ppu.mirror_vram_addr(0x23ff), 0x03ff);
        assert_eq!(ppu.mirror_vram_addr(0x2400), 0);
        assert_eq!(ppu.mirror_vram_addr(0x27ff), 0x03ff);
        assert_eq!(ppu.mirror_vram_addr(0x2c00), 0);
        assert_eq!(ppu.mirror_vram_addr(0x2fff), 0x03ff);
    }

    #[test]
    fn test_vram_single_screen_upper_mirror() {
        let mut mapper = NromMapper::new(vec![], vec![0; 2048], Mirroring::SingleScreenUpper);
        let ppu = PPU::empty(&mut mapper);

        assert_eq!(ppu.mirror_vram_addr(0x2000), 0x0400);
        assert_eq!(ppu.mirror_vram_addr(0x23ff), 0x07ff);
        assert_eq!(ppu.mirror_vram_addr(0x2400), 0x0400);
        assert_eq!(ppu.mirror_vram_addr(0x27ff), 0x07ff);
        assert_eq!(ppu.mirror_vram_addr(0x2c00), 0x0400);
        assert_eq!(ppu.mirror_vram_addr(0x2fff), 0x07ff);
    }

    #[test]
    fn test_scroll_segments_capture_mid_frame_changes() {
        let mut mapper = NromMapper::new(vec![], vec![0; 2048], Mirroring::Horizontal);
        let mut ppu = PPU::empty(&mut mapper);

        assert_eq!(ppu.scroll_segments().len(), 1);
        ppu.scanline = 100;
        ppu.write_to_scroll(0x14); // coarse X = 0x02 -> scroll_x = 16
        ppu.write_to_scroll(0x08); // coarse Y = 1 -> scroll_y = 8

        assert_eq!(ppu.scroll_segments().len(), 2);
        let segment = &ppu.scroll_segments()[1];
        assert_eq!(segment.start_scanline, 100);
        assert_eq!(segment.scroll_x, 16);
        assert_eq!(segment.scroll_y, 8);
    }

    #[test]
    fn test_scroll_writes_during_vblank_apply_next_frame() {
        let mut mapper = NromMapper::new(vec![], vec![0; 2048], Mirroring::Horizontal);
        let mut ppu = PPU::empty(&mut mapper);

        assert_eq!(ppu.scroll_segments()[0].scroll_y, 0);
        ppu.scanline = 241; // vblank
        ppu.write_to_scroll(0x00);
        ppu.write_to_scroll(0x10); // coarse Y = 2 => scroll_y = 16

        // simulate start of next frame
        ppu.reset_scroll_segments_for_new_frame();
        assert_eq!(ppu.scroll_segments()[0].scroll_y, 16);
    }

    #[test]
    fn test_read_status_resets_latch() {
        let mut mapper = NromMapper::new(vec![], vec![], Mirroring::Horizontal);
        let mut ppu = PPU::empty(&mut mapper);
        ppu.vram[0x0305] = 0x66;

        ppu.write_to_ppu_addr(0x21);
        ppu.write_to_ppu_addr(0x23);
        ppu.write_to_ppu_addr(0x05);

        ppu.read_data(); //load_into_buffer
        assert_ne!(ppu.read_data(), 0x66);

        ppu.read_status();

        ppu.write_to_ppu_addr(0x23);
        ppu.write_to_ppu_addr(0x05);

        ppu.read_data(); //load_into_buffer
        assert_eq!(ppu.read_data(), 0x66);
    }

    #[test]
    fn test_ppu_vram_mirroring() {
        let mut mapper = NromMapper::new(vec![], vec![], Mirroring::Horizontal);
        let mut ppu = PPU::empty(&mut mapper);
        ppu.write_to_ctrl(0);
        ppu.vram[0x0305] = 0x66;

        ppu.write_to_ppu_addr(0x63); //0x6305 -> 0x2305
        ppu.write_to_ppu_addr(0x05);

        ppu.read_data(); //load into_buffer
        assert_eq!(ppu.read_data(), 0x66);
        // assert_eq!(ppu.addr.read(), 0x0306)
    }

    #[test]
    fn test_read_status_resets_vblank() {
        let mut mapper = NromMapper::new(vec![], vec![], Mirroring::Horizontal);
        let mut ppu = PPU::empty(&mut mapper);
        ppu.status.set_vblank_status(true);

        let status = ppu.read_status();

        assert_eq!(status >> 7, 1);
        assert_eq!(ppu.status.snapshot() >> 7, 0);
    }

    #[test]
    fn test_oam_read_write() {
        let mut mapper = NromMapper::new(vec![], vec![], Mirroring::Horizontal);
        let mut ppu = PPU::empty(&mut mapper);
        ppu.write_to_oam_addr(0x10);
        ppu.write_to_oam_data(0x66);
        ppu.write_to_oam_data(0x77);

        ppu.write_to_oam_addr(0x10);
        assert_eq!(ppu.read_oam_data(), 0x66);

        ppu.write_to_oam_addr(0x11);
        assert_eq!(ppu.read_oam_data(), 0x77);
    }

    #[test]
    fn test_oam_dma() {
        let mut mapper = NromMapper::new(vec![], vec![], Mirroring::Horizontal);
        let mut ppu = PPU::empty(&mut mapper);

        let mut data = [0x66; 256];
        data[0] = 0x77;
        data[255] = 0x88;

        ppu.write_to_oam_addr(0x10);
        ppu.write_oam_dma(&data);

        ppu.write_to_oam_addr(0xf); //wrap around
        assert_eq!(ppu.read_oam_data(), 0x88);

        ppu.write_to_oam_addr(0x10);
        assert_eq!(ppu.read_oam_data(), 0x77);

        ppu.write_to_oam_addr(0x11);
        assert_eq!(ppu.read_oam_data(), 0x66);
    }
}
