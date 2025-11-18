pub mod cnrom;
pub mod nsf;
pub mod mmc1;
pub mod mmc3;
pub mod mmc5;
pub mod nrom;
pub mod uxrom;

#[derive(Clone, Copy, Debug)]
pub enum ChrSource {
    Background,
    Sprite,
    Cpu,
}

pub trait Mapper {
    fn read_prg(&self, addr: u16) -> u8;
    fn write_prg(&mut self, addr: u16, data: u8);
    fn read_chr(&self, addr: u16, source: ChrSource) -> u8;
    fn write_chr(&mut self, addr: u16, data: u8);
    fn mirroring(&self) -> crate::cart::Mirroring;
    fn handle_scanline(&mut self, _rendering_enabled: bool) {}
    fn poll_irq(&self) -> Option<u8> {
        None // Default implementation - no IRQ support
    }
    fn ppu_read_nametable(&self, _addr: u16, _vram: &[u8]) -> Option<u8> {
        None
    }
    fn ppu_write_nametable(&mut self, _addr: u16, _value: u8, _vram: &mut [u8]) -> bool {
        false
    }
    fn peek_nametable(&self, addr: u16, vram: &[u8]) -> Option<u8> {
        self.ppu_read_nametable(addr, vram)
    }
    fn background_tile_override(
        &self,
        _table_index: usize,
        _tile_column: usize,
        _tile_row: usize,
        _tile_index: u8,
        _pattern_addr: u16,
    ) -> Option<[u8; 16]> {
        None
    }
    fn background_palette_override(
        &self,
        _table_index: usize,
        _tile_column: usize,
        _tile_row: usize,
    ) -> Option<u8> {
        None
    }
}
