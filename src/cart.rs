use crate::mapper::{
    Mapper,
    cnrom::CnromMapper,
    mmc1::Mmc1Mapper,
    mmc3::Mmc3Mapper,
    nrom::NromMapper,
    uxrom::UxromMapper,
};

const NES_TAG: [u8; 4] = [0x4E, 0x45, 0x53, 0x1A];
const PRG_ROM_PAGE_SIZE: usize = 16384;
const CHR_ROM_PAGE_SIZE: usize = 8192;

#[derive(Debug, PartialEq, Clone)]
pub enum Mirroring {
    Vertical,
    Horizontal,
    FourScreen,
    SingleScreenLower,
    SingleScreenUpper,
}

#[derive(Debug, PartialEq, Clone)]
pub enum RomFormat {
    INes,
    Nes2,
}

#[derive(Debug, Clone)]
pub struct Nes2Data {
    pub submapper: u8,
    pub console_type: u8,
    pub timing: u8,
    pub prg_ram_size: usize,
    pub chr_ram_size: usize,
    pub misc_rom_count: u8,
    pub default_expansion_device: u8,
}

fn calculate_nes2_prg_size(lsb: u8, msb: u8) -> usize {
    let msb_nibble = (msb >> 4) & 0x0F;
    if msb_nibble == 0x0F {
        // Exponent-multiplier notation
        let multiplier = ((msb & 0x03) * 2) + 1;
        let exponent = (msb >> 2) & 0x3F;
        (2u64.pow(exponent as u32) * multiplier as u64) as usize
    } else {
        // Simple notation: (MSB << 8) | LSB in 16 KiB units
        (((msb_nibble as usize) << 8) | (lsb as usize)) * PRG_ROM_PAGE_SIZE
    }
}

fn calculate_nes2_chr_size(lsb: u8, msb: u8) -> usize {
    let msb_nibble = msb & 0x0F;
    if msb_nibble == 0x0F {
        // Exponent-multiplier notation
        let multiplier = ((lsb & 0x03) * 2) + 1;
        let exponent = (lsb >> 2) & 0x3F;
        (2u64.pow(exponent as u32) * multiplier as u64) as usize
    } else {
        // Simple notation: (MSB << 8) | LSB in 8 KiB units
        (((msb_nibble as usize) << 8) | (lsb as usize)) * CHR_ROM_PAGE_SIZE
    }
}

fn calculate_ram_size(shift_count: u8) -> usize {
    if shift_count == 0 {
        0
    } else {
        64 << shift_count
    }
}

pub struct Cart {
    pub mapper: Box<dyn Mapper>,
    pub screen_mirroring: Mirroring,
    pub format: RomFormat,
    pub nes2_data: Option<Nes2Data>,
}

impl Cart {
    pub fn new(raw: &Vec<u8>) -> Result<Cart, String> {
        if raw[0..4] != NES_TAG {
            return Err("File is not in iNES file format".to_string());
        }

        // Check for NES 2.0 format: header[7] bits 2 and 3 set to 1 and 0 respectively
        let format = if (raw[7] & 0x0C) == 0x08 {
            RomFormat::Nes2
        } else {
            RomFormat::INes
        };

        let mapper = (raw[7] & 0b1111_0000) | (raw[6] >> 4);

        // For iNES, ensure version is 0
        if let RomFormat::INes = format {
            let ines_ver = (raw[7] >> 2) & 0b11;
            if ines_ver != 0 {
                return Err("Invalid iNES format version".to_string());
            }
        }

        let four_screen = raw[6] & 0b1000 != 0;
        let vertical_mirroring = raw[6] & 0b1 != 0;
        let screen_mirroring = match (four_screen, vertical_mirroring) {
            (true, _) => Mirroring::FourScreen,
            (false, true) => Mirroring::Vertical,
            (false, false) => Mirroring::Horizontal,
        };

        let (prg_rom_size, chr_rom_size) = match format {
            RomFormat::INes => (
                raw[4] as usize * PRG_ROM_PAGE_SIZE,
                raw[5] as usize * CHR_ROM_PAGE_SIZE,
            ),
            RomFormat::Nes2 => (
                calculate_nes2_prg_size(raw[4], raw[9]),
                calculate_nes2_chr_size(raw[5], raw[9]),
            ),
        };

        let skip_trainer = raw[6] & 0b100 != 0;

        let prg_rom_start = 16 + if skip_trainer { 512 } else { 0 };
        let chr_rom_start = prg_rom_start + prg_rom_size;

        let prg_rom = raw[prg_rom_start..(prg_rom_start + prg_rom_size)].to_vec();
        let chr_rom = raw[chr_rom_start..(chr_rom_start + chr_rom_size)].to_vec();

        let nes2_data = if let RomFormat::Nes2 = format {
            Some(Nes2Data {
                submapper: raw[8] >> 4,
                console_type: raw[7] & 0x03,
                timing: raw[12],
                prg_ram_size: calculate_ram_size(raw[10] & 0x0F) + calculate_ram_size(raw[10] >> 4),
                chr_ram_size: calculate_ram_size(raw[11] & 0x0F) + calculate_ram_size(raw[11] >> 4),
                misc_rom_count: raw[14] & 0x03,
                default_expansion_device: raw[15],
            })
        } else {
            None
        };

        println!("Mapper: {mapper}");

        let mapper: Box<dyn Mapper> = match mapper {
            0 => Box::new(NromMapper::new(prg_rom, chr_rom, screen_mirroring.clone())),
            1 => Box::new(Mmc1Mapper::new(prg_rom, chr_rom, screen_mirroring.clone())),
            2 => Box::new(UxromMapper::new(prg_rom, chr_rom, screen_mirroring.clone())),
            3 => Box::new(CnromMapper::new(prg_rom, chr_rom, screen_mirroring.clone())),
            4 => Box::new(Mmc3Mapper::new(prg_rom, chr_rom, screen_mirroring.clone())),
            _ => return Err(format!("Mapper {} not supported", mapper)),
        };

        Ok(Cart {
            mapper,
            screen_mirroring,
            format,
            nes2_data,
        })
    }

    pub fn empty() -> Cart {
        Cart {
            mapper: Box::new(NromMapper::new(vec![], vec![], Mirroring::Vertical)),
            screen_mirroring: Mirroring::Vertical,
            format: RomFormat::INes,
            nes2_data: None,
        }
    }
}

pub mod test {

    use super::*;

    struct TestRom {
        header: Vec<u8>,
        trainer: Option<Vec<u8>>,
        pgp_rom: Vec<u8>,
        chr_rom: Vec<u8>,
    }

    fn create_rom(rom: TestRom) -> Vec<u8> {
        let mut result = Vec::with_capacity(
            rom.header.len()
                + rom.trainer.as_ref().map_or(0, |t| t.len())
                + rom.pgp_rom.len()
                + rom.chr_rom.len(),
        );

        result.extend(&rom.header);
        if let Some(t) = rom.trainer {
            result.extend(t);
        }
        result.extend(&rom.pgp_rom);
        result.extend(&rom.chr_rom);

        result
    }

    pub fn test_rom(program: Vec<u8>) -> Cart {
        let mut pgp_rom_contents = program;
        pgp_rom_contents.resize(2 * PRG_ROM_PAGE_SIZE, 0);

        let test_rom = create_rom(TestRom {
            header: vec![
                0x4E, 0x45, 0x53, 0x1A, 0x02, 0x01, 0x31, 00, 00, 00, 00, 00, 00, 00, 00, 00,
            ],
            trainer: None,
            pgp_rom: pgp_rom_contents,
            chr_rom: vec![2; CHR_ROM_PAGE_SIZE],
        });

        Cart::new(&test_rom).unwrap()
    }

    #[test]
    fn test() {
        let test_rom = create_rom(TestRom {
            header: vec![
                0x4E, 0x45, 0x53, 0x1A, 0x02, 0x01, 0x31, 00, 00, 00, 00, 00, 00, 00, 00, 00,
            ],
            trainer: None,
            pgp_rom: vec![1; 2 * PRG_ROM_PAGE_SIZE],
            chr_rom: vec![2; 1 * CHR_ROM_PAGE_SIZE],
        });

        let rom: Cart = Cart::new(&test_rom).unwrap();

        // assert_eq!(rom.chr_rom, vec!(2; 1 * CHR_ROM_PAGE_SIZE));
        // assert_eq!(rom.prg_rom, vec!(1; 2 * PRG_ROM_PAGE_SIZE));
        // assert_eq!(rom.mapper, 3);
        assert_eq!(rom.screen_mirroring, Mirroring::Vertical);
    }

    #[test]
    fn test_with_trainer() {
        let test_rom = create_rom(TestRom {
            header: vec![
                0x4E,
                0x45,
                0x53,
                0x1A,
                0x02,
                0x01,
                0x31 | 0b100,
                00,
                00,
                00,
                00,
                00,
                00,
                00,
                00,
                00,
            ],
            trainer: Some(vec![0; 512]),
            pgp_rom: vec![1; 2 * PRG_ROM_PAGE_SIZE],
            chr_rom: vec![2; 1 * CHR_ROM_PAGE_SIZE],
        });

        let rom: Cart = Cart::new(&test_rom).unwrap();

        // assert_eq!(rom.chr_rom, vec!(2; 1 * CHR_ROM_PAGE_SIZE));
        // assert_eq!(rom.prg_rom, vec!(1; 2 * PRG_ROM_PAGE_SIZE));
        // assert_eq!(rom.mapper, 3);
        assert_eq!(rom.screen_mirroring, Mirroring::Vertical);
    }

    #[test]
    fn test_nes2_is_supported() {
        let test_rom = create_rom(TestRom {
            header: vec![
                0x4E, 0x45, 0x53, 0x1A, 0x01, 0x01, 0x01, 0x8, 00, 00, 00, 00, 00, 00, 00, 00,
            ],
            trainer: None,
            pgp_rom: vec![1; 1 * PRG_ROM_PAGE_SIZE],
            chr_rom: vec![2; 1 * CHR_ROM_PAGE_SIZE],
        });
        let rom = Cart::new(&test_rom);
        match rom {
            Result::Ok(cart) => {
                assert_eq!(cart.format, RomFormat::Nes2);
                assert!(cart.nes2_data.is_some());
            }
            Result::Err(_) => assert!(false, "should load NES 2.0 rom"),
        }
    }
}
