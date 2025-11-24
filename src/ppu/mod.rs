pub mod framebuffer;
pub mod palette;
pub mod registers;
pub mod render;

use crate::cart::Mirroring;
use crate::mapper::{ChrSource, Mapper};
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
    pub screen_origin: usize,
}

pub struct PPU {
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
    pub cycle: i16,
    pub scanline: i16,
    pub frame_count: u64,

    internal_data_buf: u8,
    scroll_segments: Vec<ScrollSegment>,
    pending_scroll_descriptor: Option<(usize, usize, usize, usize)>,
}

impl PPU {
    pub fn empty() -> Self {
        PPU::new()
    }

    pub fn new() -> Self {
        let mut ppu = PPU {
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
            cycle: 0,
            scanline: 0,
            frame_count: 0,
            internal_data_buf: 0,
            scroll_segments: Vec::new(),
            pending_scroll_descriptor: None,
        };

        ppu.reset_scroll_segments_for_new_frame();
        ppu
    }

    pub fn mirror_vram_addr(&self, mapper: &dyn Mapper, addr: u16) -> u16 {
        let mirrored_vram = addr & 0b10111111111111;
        let vram_index = mirrored_vram - 0x2000;
        let name_table = vram_index / 0x400;
        let mirroring = mapper.mirroring();
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

    fn mirror_palette_addr(addr: u16) -> usize {
        let mut palette_index = (addr - 0x3f00) & 0x1f;
        if palette_index >= 0x10 && (palette_index & 0x03) == 0 {
            palette_index -= 0x10;
        }
        palette_index as usize
    }

    pub fn peek_nametable_byte(&self, mapper: &dyn Mapper, addr: u16) -> u8 {
        mapper
            .peek_nametable(addr, &self.vram)
            .unwrap_or_else(|| self.vram[self.mirror_vram_addr(mapper, addr) as usize])
    }

    fn nametable_base_addr(table_index: usize) -> u16 {
        0x2000 + ((table_index & 0x03) * 0x400) as u16
    }

    pub fn read_nametable_entry(
        &self,
        mapper: &dyn Mapper,
        table_index: usize,
        tile_column: usize,
        tile_row: usize,
    ) -> u8 {
        let base = Self::nametable_base_addr(table_index);
        let offset = (tile_row * 32 + tile_column) as u16;
        self.peek_nametable_byte(mapper, base + offset)
    }

    pub fn read_attribute_entry(
        &self,
        mapper: &dyn Mapper,
        table_index: usize,
        tile_column: usize,
        tile_row: usize,
    ) -> u8 {
        let base = Self::nametable_base_addr(table_index);
        let attr = 0x3C0 + (tile_row / 4 * 8 + tile_column / 4) as u16;
        self.peek_nametable_byte(mapper, base + attr)
    }

    fn increment_vram_addr(&mut self) {
        let step = self.ctrl.vram_addr_increment();
        self.addr.increment(step);
        self.scroll.increment(step);
    }

    pub fn scroll_segments(&self) -> &[ScrollSegment] {
        &self.scroll_segments
    }

    fn current_scroll_descriptor(&self) -> (usize, usize, usize) {
        (
            self.scroll.scroll_x(),
            self.scroll.scroll_y(),
            self.scroll.base_nametable(),
        )
    }

    fn visible_scanline(&self) -> Option<usize> {
        if (self.scanline as usize) < 240 {
            Some(self.scanline as usize)
        } else {
            None
        }
    }

    fn push_scroll_segment(
        &mut self,
        descriptor: (usize, usize, usize),
        scanline: usize,
        screen_origin: usize,
    ) {
        let (scroll_x, scroll_y, base_nametable) = descriptor;
        if let Some(last) = self.scroll_segments.last_mut() {
            if last.start_scanline == scanline {
                *last = ScrollSegment {
                    start_scanline: scanline,
                    scroll_x,
                    scroll_y,
                    base_nametable,
                    screen_origin,
                };
                return;
            }

            if last.scroll_x == scroll_x
                && last.scroll_y == scroll_y
                && last.base_nametable == base_nametable
                && last.screen_origin == screen_origin
            {
                return;
            }
        }

        self.scroll_segments.push(ScrollSegment {
            start_scanline: scanline,
            scroll_x,
            scroll_y,
            base_nametable,
            screen_origin,
        });
    }

    fn queue_scroll_state_change(&mut self, reset_origin: bool) {
        let descriptor = self.current_scroll_descriptor();
        if let Some(scanline) = self.visible_scanline() {
            let scanline = scanline.min(239);
            let screen_origin = if reset_origin {
                scanline
            } else {
                self.scroll_segments
                    .last()
                    .map(|segment| segment.screen_origin)
                    .unwrap_or(0)
            };
            self.push_scroll_segment(descriptor, scanline, screen_origin);
        } else {
            let screen_origin = if reset_origin {
                0
            } else {
                self.pending_scroll_descriptor
                    .map(|(_, _, _, origin)| origin)
                    .or_else(|| {
                        self.scroll_segments
                            .last()
                            .map(|segment| segment.screen_origin)
                    })
                    .unwrap_or(0)
            };
            self.pending_scroll_descriptor =
                Some((descriptor.0, descriptor.1, descriptor.2, screen_origin));
        }
    }

    pub fn reset_scroll_segments_for_new_frame(&mut self) {
        let descriptor = self.pending_scroll_descriptor.take().unwrap_or_else(|| {
            let (scroll_x, scroll_y, base_nametable) = self.current_scroll_descriptor();
            (scroll_x, scroll_y, base_nametable, 0)
        });
        self.scroll_segments.clear();
        self.scroll_segments.push(ScrollSegment {
            start_scanline: 0,
            scroll_x: descriptor.0,
            scroll_y: descriptor.1,
            base_nametable: descriptor.2,
            screen_origin: descriptor.3,
        });
    }
}

impl PPU {
    pub fn write_to_ctrl(&mut self, value: u8) {
        let before_nmi_status = self.ctrl.generate_vblank_nmi();
        let previous_base_nametable = self.scroll.base_nametable();
        self.ctrl.update(value);
        self.scroll.update_ctrl(value);
        let base_changed = previous_base_nametable != self.scroll.base_nametable();
        if !before_nmi_status && self.ctrl.generate_vblank_nmi() && self.status.is_in_vblank() {
            self.nmi_interrupt = Some(1);
        }
        self.queue_scroll_state_change(base_changed);
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
            self.queue_scroll_state_change(true);
        }
    }

    pub fn write_to_ppu_addr(&mut self, value: u8) {
        self.addr.update(value);
        let completed_sequence = self.scroll.write_ppu_addr(value);

        if completed_sequence {
            self.queue_scroll_state_change(true);
        }
    }

    pub fn write_to_data(&mut self, mapper: &mut dyn Mapper, value: u8) {
        let addr = self.scroll.addr();
        match addr {
            0..=0x1fff => mapper.write_chr(addr, value),
            0x2000..=0x3eff => {
                if !mapper.ppu_write_nametable(addr, value, &mut self.vram) {
                    self.vram[self.mirror_vram_addr(mapper, addr) as usize] = value;
                }
            }
            0x3f00..=0x3fff => {
                let palette_index = PPU::mirror_palette_addr(addr);
                self.palette_table[palette_index] = value & 0x3f;
            }
            _ => panic!("unexpected access to mirrored space {}", addr),
        }
        self.increment_vram_addr();
    }

    pub fn read_data(&mut self, mapper: &mut dyn Mapper) -> u8 {
        let addr = self.scroll.addr();

        self.increment_vram_addr();

        match addr {
            0..=0x1fff => {
                let result = self.internal_data_buf;
                self.internal_data_buf = mapper.read_chr(addr, ChrSource::Cpu);
                result
            }
            0x2000..=0x3eff => {
                let result = self.internal_data_buf;
                let value = mapper
                    .ppu_read_nametable(addr, &self.vram)
                    .unwrap_or_else(|| self.vram[self.mirror_vram_addr(mapper, addr) as usize]);
                self.internal_data_buf = value;
                result
            }
            0x3f00..=0x3fff => {
                let palette_index = PPU::mirror_palette_addr(addr);
                let mirrored_vram_addr = addr - 0x1000;
                self.internal_data_buf =
                    self.vram[self.mirror_vram_addr(mapper, mirrored_vram_addr) as usize];
                self.palette_table[palette_index]
            }
            _ => panic!("unexpected access to mirrored space {}", addr),
        }
    }

    pub fn write_oam_dma(&mut self, data: &[u8; 256]) {
        for x in data.iter() {
            self.oam_data[self.oam_addr as usize] = *x;
            self.oam_addr = self.oam_addr.wrapping_add(1);
        }
    }

    pub fn clock(&mut self, mapper: &mut dyn Mapper) -> bool {
        self.cycle += 1;

        if self.cycle >= 341 {
            if self.is_sprite_zero_hit(self.cycle as usize) {
                self.status.set_sprite_zero_hit(true);
            }

            self.cycle -= 341;

            if self.scanline < 240 {
                let rendering_enabled = self.mask.show_background() || self.mask.show_sprites();
                mapper.handle_scanline(rendering_enabled);
            }

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
                self.cycle = 0;
                self.nmi_interrupt = None;
                self.status.set_sprite_zero_hit(false);
                self.status.reset_vblank_status();
                self.frame_count = self.frame_count.wrapping_add(1);
                return true;
            }
        }
        false
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
        let mut ppu = PPU::empty();
        ppu.write_to_ppu_addr(0x23);
        ppu.write_to_ppu_addr(0x05);
        ppu.write_to_data(&mut mapper, 0x66);

        assert_eq!(ppu.vram[0x0305], 0x66);
    }

    #[test]
    fn test_ppu_vram_reads() {
        let mut mapper = NromMapper::new(vec![], vec![], Mirroring::Horizontal);
        let mut ppu = PPU::empty();
        ppu.write_to_ctrl(0);
        ppu.vram[0x0305] = 0x66;

        ppu.write_to_ppu_addr(0x23);
        ppu.write_to_ppu_addr(0x05);

        ppu.read_data(&mut mapper);
        assert_eq!(ppu.addr.get(), 0x2306);
        assert_eq!(ppu.read_data(&mut mapper), 0x66);
    }

    #[test]
    fn test_ppu_vram_reads_cross_page() {
        let mut mapper = NromMapper::new(vec![], vec![], Mirroring::Horizontal);
        let mut ppu = PPU::empty();
        ppu.write_to_ctrl(0);
        ppu.vram[0x01ff] = 0x66;
        ppu.vram[0x0200] = 0x77;

        ppu.write_to_ppu_addr(0x21);
        ppu.write_to_ppu_addr(0xff);

        ppu.read_data(&mut mapper);
        assert_eq!(ppu.read_data(&mut mapper), 0x66);
        assert_eq!(ppu.read_data(&mut mapper), 0x77);
    }

    #[test]
    fn test_ppu_vram_reads_step_32() {
        let mut mapper = NromMapper::new(vec![], vec![], Mirroring::Horizontal);
        let mut ppu = PPU::empty();
        ppu.write_to_ctrl(0b100);
        ppu.vram[0x01ff] = 0x66;
        ppu.vram[0x01ff + 32] = 0x77;
        ppu.vram[0x01ff + 64] = 0x88;

        ppu.write_to_ppu_addr(0x21);
        ppu.write_to_ppu_addr(0xff);

        ppu.read_data(&mut mapper);
        assert_eq!(ppu.read_data(&mut mapper), 0x66);
        assert_eq!(ppu.read_data(&mut mapper), 0x77);
        assert_eq!(ppu.read_data(&mut mapper), 0x88);
    }

    #[test]
    fn test_vram_horizontal_mirror() {
        let mut mapper = NromMapper::new(vec![], vec![], Mirroring::Horizontal);
        let mut ppu = PPU::empty();
        ppu.write_to_ppu_addr(0x24);
        ppu.write_to_ppu_addr(0x05);

        ppu.write_to_data(&mut mapper, 0x66);

        ppu.write_to_ppu_addr(0x28);
        ppu.write_to_ppu_addr(0x05);

        ppu.write_to_data(&mut mapper, 0x77);

        ppu.write_to_ppu_addr(0x20);
        ppu.write_to_ppu_addr(0x05);

        ppu.read_data(&mut mapper);
        assert_eq!(ppu.read_data(&mut mapper), 0x66);

        ppu.write_to_ppu_addr(0x2C);
        ppu.write_to_ppu_addr(0x05);

        ppu.read_data(&mut mapper);
        assert_eq!(ppu.read_data(&mut mapper), 0x77);
    }

    #[test]
    fn test_vram_vertical_mirror() {
        let mut mapper = NromMapper::new(vec![], vec![0; 2048], Mirroring::Vertical);
        let mut ppu = PPU::empty();

        ppu.write_to_ppu_addr(0x20);
        ppu.write_to_ppu_addr(0x05);

        ppu.write_to_data(&mut mapper, 0x66);

        ppu.write_to_ppu_addr(0x2C);
        ppu.write_to_ppu_addr(0x05);

        ppu.write_to_data(&mut mapper, 0x77);

        ppu.write_to_ppu_addr(0x28);
        ppu.write_to_ppu_addr(0x05);

        ppu.read_data(&mut mapper);
        assert_eq!(ppu.read_data(&mut mapper), 0x66);

        ppu.write_to_ppu_addr(0x24);
        ppu.write_to_ppu_addr(0x05);

        ppu.read_data(&mut mapper);
        assert_eq!(ppu.read_data(&mut mapper), 0x77);
    }

    #[test]
    fn test_vram_single_screen_lower_mirror() {
        let mapper = NromMapper::new(vec![], vec![0; 2048], Mirroring::SingleScreenLower);
        let ppu = PPU::empty();

        assert_eq!(ppu.mirror_vram_addr(&mapper, 0x2000), 0);
        assert_eq!(ppu.mirror_vram_addr(&mapper, 0x23ff), 0x03ff);
        assert_eq!(ppu.mirror_vram_addr(&mapper, 0x2400), 0);
        assert_eq!(ppu.mirror_vram_addr(&mapper, 0x27ff), 0x03ff);
        assert_eq!(ppu.mirror_vram_addr(&mapper, 0x2c00), 0);
        assert_eq!(ppu.mirror_vram_addr(&mapper, 0x2fff), 0x03ff);
    }

    #[test]
    fn test_vram_single_screen_upper_mirror() {
        let mapper = NromMapper::new(vec![], vec![0; 2048], Mirroring::SingleScreenUpper);
        let ppu = PPU::empty();

        assert_eq!(ppu.mirror_vram_addr(&mapper, 0x2000), 0x0400);
        assert_eq!(ppu.mirror_vram_addr(&mapper, 0x23ff), 0x07ff);
        assert_eq!(ppu.mirror_vram_addr(&mapper, 0x2400), 0x0400);
        assert_eq!(ppu.mirror_vram_addr(&mapper, 0x27ff), 0x07ff);
        assert_eq!(ppu.mirror_vram_addr(&mapper, 0x2c00), 0x0400);
        assert_eq!(ppu.mirror_vram_addr(&mapper, 0x2fff), 0x07ff);
    }

    #[test]
    fn test_scroll_segments_capture_mid_frame_changes() {
        let mut ppu = PPU::empty();

        assert_eq!(ppu.scroll_segments().len(), 1);
        ppu.scanline = 100;
        ppu.write_to_scroll(0x14);
        ppu.write_to_scroll(0x08);

        assert_eq!(ppu.scroll_segments().len(), 2);
        let segment = &ppu.scroll_segments()[1];
        assert_eq!(segment.start_scanline, 100);
        assert_eq!(segment.scroll_x, 20);
        assert_eq!(segment.scroll_y, 8);
    }

    #[test]
    fn test_scroll_writes_during_vblank_apply_next_frame() {
        let mut ppu = PPU::empty();

        assert_eq!(ppu.scroll_segments()[0].scroll_y, 0);
        ppu.scanline = 241;
        ppu.write_to_scroll(0x00);
        ppu.write_to_scroll(0x10);

        ppu.reset_scroll_segments_for_new_frame();
        assert_eq!(ppu.scroll_segments()[0].scroll_y, 16);
    }

    #[test]
    fn test_read_status_resets_latch() {
        let mut mapper = NromMapper::new(vec![], vec![], Mirroring::Horizontal);
        let mut ppu = PPU::empty();
        ppu.vram[0x0305] = 0x66;

        ppu.write_to_ppu_addr(0x21);
        ppu.write_to_ppu_addr(0x23);
        ppu.write_to_ppu_addr(0x05);

        ppu.read_data(&mut mapper);
        assert_ne!(ppu.read_data(&mut mapper), 0x66);

        ppu.read_status();

        ppu.write_to_ppu_addr(0x23);
        ppu.write_to_ppu_addr(0x05);

        ppu.read_data(&mut mapper);
        assert_eq!(ppu.read_data(&mut mapper), 0x66);
    }

    #[test]
    fn test_ppu_vram_mirroring() {
        let mut mapper = NromMapper::new(vec![], vec![], Mirroring::Horizontal);
        let mut ppu = PPU::empty();
        ppu.write_to_ctrl(0);
        ppu.vram[0x0305] = 0x66;

        ppu.write_to_ppu_addr(0x63);
        ppu.write_to_ppu_addr(0x05);

        ppu.read_data(&mut mapper);
        assert_eq!(ppu.read_data(&mut mapper), 0x66);
    }

    #[test]
    fn test_palette_address_mirroring() {
        let mut ppu = PPU::empty();

        fn write_palette(ppu: &mut PPU, addr: u16, value: u8) {
            let mut mapper = NromMapper::new(vec![], vec![], Mirroring::Horizontal);

            ppu.write_to_ppu_addr((addr >> 8) as u8);
            ppu.write_to_ppu_addr((addr & 0xff) as u8);
            ppu.write_to_data(&mut mapper, value);
        }

        write_palette(&mut ppu, 0x3f00, 0x11);
        assert_eq!(ppu.palette_table[0x00], 0x11);

        write_palette(&mut ppu, 0x3f10, 0x22);
        assert_eq!(ppu.palette_table[0x00], 0x22);

        write_palette(&mut ppu, 0x3f20, 0x33);
        assert_eq!(ppu.palette_table[0x00], 0x33);

        write_palette(&mut ppu, 0x3f1c, 0x3d);
        assert_eq!(ppu.palette_table[0x0c], 0x3d & 0x3f);
    }

    #[test]
    fn test_palette_reads_update_buffer() {
        let mut mapper = NromMapper::new(vec![], vec![0; 2048], Mirroring::Horizontal);
        let mut ppu = PPU::empty();

        let mirrored_vram = ppu.mirror_vram_addr(&mapper, 0x2f00) as usize;
        ppu.vram[mirrored_vram] = 0xaa;

        ppu.write_to_ppu_addr(0x3f);
        ppu.write_to_ppu_addr(0x00);
        ppu.write_to_data(&mut mapper, 0x2f);

        ppu.write_to_ppu_addr(0x3f);
        ppu.write_to_ppu_addr(0x00);
        ppu.internal_data_buf = 0;
        let data = ppu.read_data(&mut mapper);
        assert_eq!(data, 0x2f);
        assert_eq!(ppu.internal_data_buf, 0xaa);
    }

    #[test]
    fn test_read_status_resets_vblank() {
        let mut ppu = PPU::empty();
        ppu.status.set_vblank_status(true);

        let status = ppu.read_status();

        assert_eq!(status >> 7, 1);
        assert_eq!(ppu.status.snapshot() >> 7, 0);
    }

    #[test]
    fn test_oam_read_write() {
        let mut ppu = PPU::empty();
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
        let mut ppu = PPU::empty();

        let mut data = [0x66; 256];
        data[0] = 0x77;
        data[255] = 0x88;

        ppu.write_to_oam_addr(0x10);
        ppu.write_oam_dma(&data);

        ppu.write_to_oam_addr(0xf);
        assert_eq!(ppu.read_oam_data(), 0x88);

        ppu.write_to_oam_addr(0x10);
        assert_eq!(ppu.read_oam_data(), 0x77);

        ppu.write_to_oam_addr(0x11);
        assert_eq!(ppu.read_oam_data(), 0x66);
    }
}
