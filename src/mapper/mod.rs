pub mod cnrom;
pub mod mmc1;
pub mod mmc3;
pub mod nrom;
pub mod uxrom;

pub trait Mapper {
    fn read_prg(&self, addr: u16) -> u8;
    fn write_prg(&self, addr: u16, data: u8);
    fn read_chr(&self, addr: u16) -> u8;
    fn write_chr(&self, addr: u16, data: u8);
    fn mirroring(&self) -> crate::cart::Mirroring;
}
