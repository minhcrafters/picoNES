use crate::cpu::AddressingMode;
use core::fmt;
use std::sync::LazyLock;

#[derive(Debug, PartialEq)]
pub enum Mnemonic {
    ADC,
    AND,
    ASL,
    BCC,
    BCS,
    BEQ,
    BIT,
    BMI,
    BNE,
    BPL,
    BRK,
    BVC,
    BVS,
    CLC,
    CLD,
    CLI,
    CLV,
    CMP,
    CPX,
    CPY,
    DEC,
    DEX,
    DEY,
    EOR,
    INC,
    INX,
    INY,
    JMP,
    JSR,
    LDA,
    LDX,
    LDY,
    LSR,
    NOP,
    ORA,
    PHA,
    PHP,
    PLA,
    PLP,
    ROL,
    ROR,
    RTI,
    RTS,
    SBC,
    SEC,
    SED,
    SEI,
    STA,
    STX,
    STY,
    TAX,
    TAY,
    TSX,
    TXA,
    TXS,
    TYA,
}

impl fmt::Display for Mnemonic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug)]
pub struct Opcode {
    pub code: u8,
    pub mnemonic: Mnemonic,
    pub bytes: u8,
    #[allow(dead_code)] // not yet cycle-accurate
    pub cycles: u8,
    pub additional_cycles: u8,
    pub addressing_mode: AddressingMode,
}

impl Opcode {
    pub const fn new(
        code: u8,
        mnemonic: Mnemonic,
        bytes: u8,
        cycles: u8,
        additional_cycles: u8,
        addressing_mode: AddressingMode,
    ) -> Self {
        Opcode {
            code,
            mnemonic,
            bytes,
            cycles,
            additional_cycles,
            addressing_mode,
        }
    }
}

pub struct OpcodeMap {
    opcodes: Vec<Opcode>,
}

impl OpcodeMap {
    pub fn new() -> Self {
        OpcodeMap {
            opcodes: vec![
                // ADC
                Opcode::new(0x69, Mnemonic::ADC, 2, 2, 0, AddressingMode::Immediate),
                Opcode::new(0x65, Mnemonic::ADC, 2, 3, 0, AddressingMode::ZeroPage),
                Opcode::new(0x75, Mnemonic::ADC, 2, 4, 0, AddressingMode::ZeroPageX),
                Opcode::new(0x6D, Mnemonic::ADC, 3, 4, 0, AddressingMode::Absolute),
                Opcode::new(0x7D, Mnemonic::ADC, 3, 4, 1, AddressingMode::AbsoluteX),
                Opcode::new(0x79, Mnemonic::ADC, 3, 4, 1, AddressingMode::AbsoluteY),
                Opcode::new(0x61, Mnemonic::ADC, 2, 6, 0, AddressingMode::IndirectX),
                Opcode::new(0x71, Mnemonic::ADC, 2, 5, 1, AddressingMode::IndirectY),
                // AND
                Opcode::new(0x29, Mnemonic::AND, 2, 2, 0, AddressingMode::Immediate),
                Opcode::new(0x25, Mnemonic::AND, 2, 3, 0, AddressingMode::ZeroPage),
                Opcode::new(0x35, Mnemonic::AND, 2, 4, 0, AddressingMode::ZeroPageX),
                Opcode::new(0x2D, Mnemonic::AND, 3, 4, 0, AddressingMode::Absolute),
                Opcode::new(0x3D, Mnemonic::AND, 3, 4, 1, AddressingMode::AbsoluteX),
                Opcode::new(0x39, Mnemonic::AND, 3, 4, 1, AddressingMode::AbsoluteY),
                Opcode::new(0x21, Mnemonic::AND, 2, 6, 0, AddressingMode::IndirectX),
                Opcode::new(0x31, Mnemonic::AND, 2, 5, 1, AddressingMode::IndirectY),
                // ASL
                Opcode::new(0x0A, Mnemonic::ASL, 1, 2, 0, AddressingMode::Accumulator),
                Opcode::new(0x06, Mnemonic::ASL, 2, 5, 0, AddressingMode::ZeroPage),
                Opcode::new(0x16, Mnemonic::ASL, 2, 6, 0, AddressingMode::ZeroPageX),
                Opcode::new(0x0E, Mnemonic::ASL, 3, 6, 0, AddressingMode::Absolute),
                Opcode::new(0x1E, Mnemonic::ASL, 3, 7, 0, AddressingMode::AbsoluteX),
                // BCC
                Opcode::new(0x90, Mnemonic::BCC, 2, 2, 1, AddressingMode::Relative),
                // BCS
                Opcode::new(0xB0, Mnemonic::BCS, 2, 2, 1, AddressingMode::Relative),
                // BEQ
                Opcode::new(0xF0, Mnemonic::BEQ, 2, 2, 1, AddressingMode::Relative),
                // BIT
                Opcode::new(0x24, Mnemonic::BIT, 2, 3, 0, AddressingMode::ZeroPage),
                Opcode::new(0x2C, Mnemonic::BIT, 3, 4, 0, AddressingMode::Absolute),
                // BMI
                Opcode::new(0x30, Mnemonic::BMI, 2, 2, 1, AddressingMode::Relative),
                // BNE
                Opcode::new(0xD0, Mnemonic::BNE, 2, 2, 1, AddressingMode::Relative),
                // BPL
                Opcode::new(0x10, Mnemonic::BPL, 2, 2, 1, AddressingMode::Relative),
                // BRK
                Opcode::new(0x00, Mnemonic::BRK, 1, 7, 0, AddressingMode::None),
                // BVC
                Opcode::new(0x50, Mnemonic::BVC, 2, 2, 1, AddressingMode::Relative),
                // BVS
                Opcode::new(0x70, Mnemonic::BVS, 2, 2, 1, AddressingMode::Relative),
                // CLC
                Opcode::new(0x18, Mnemonic::CLC, 1, 2, 0, AddressingMode::None),
                // CLD
                Opcode::new(0xD8, Mnemonic::CLD, 1, 2, 0, AddressingMode::None),
                // CLI
                Opcode::new(0x58, Mnemonic::CLI, 1, 2, 0, AddressingMode::None),
                // CLV
                Opcode::new(0xB8, Mnemonic::CLV, 1, 2, 0, AddressingMode::None),
                // CMP
                Opcode::new(0xC9, Mnemonic::CMP, 2, 2, 0, AddressingMode::Immediate),
                Opcode::new(0xC5, Mnemonic::CMP, 2, 3, 0, AddressingMode::ZeroPage),
                Opcode::new(0xD5, Mnemonic::CMP, 2, 4, 0, AddressingMode::ZeroPageX),
                Opcode::new(0xCD, Mnemonic::CMP, 3, 4, 0, AddressingMode::Absolute),
                Opcode::new(0xDD, Mnemonic::CMP, 3, 4, 1, AddressingMode::AbsoluteX),
                Opcode::new(0xD9, Mnemonic::CMP, 3, 4, 1, AddressingMode::AbsoluteY),
                Opcode::new(0xC1, Mnemonic::CMP, 2, 6, 0, AddressingMode::IndirectX),
                Opcode::new(0xD1, Mnemonic::CMP, 2, 5, 1, AddressingMode::IndirectY),
                // CPX
                Opcode::new(0xE0, Mnemonic::CPX, 2, 2, 0, AddressingMode::Immediate),
                Opcode::new(0xE4, Mnemonic::CPX, 2, 3, 0, AddressingMode::ZeroPage),
                Opcode::new(0xEC, Mnemonic::CPX, 3, 4, 0, AddressingMode::Absolute),
                // CPY
                Opcode::new(0xC0, Mnemonic::CPY, 2, 2, 0, AddressingMode::Immediate),
                Opcode::new(0xC4, Mnemonic::CPY, 2, 3, 0, AddressingMode::ZeroPage),
                Opcode::new(0xCC, Mnemonic::CPY, 3, 4, 0, AddressingMode::Absolute),
                // DEC
                Opcode::new(0xC6, Mnemonic::DEC, 2, 5, 0, AddressingMode::ZeroPage),
                Opcode::new(0xD6, Mnemonic::DEC, 2, 6, 0, AddressingMode::ZeroPageX),
                Opcode::new(0xCE, Mnemonic::DEC, 3, 6, 0, AddressingMode::Absolute),
                Opcode::new(0xDE, Mnemonic::DEC, 3, 7, 0, AddressingMode::AbsoluteX),
                // DEX
                Opcode::new(0xCA, Mnemonic::DEX, 1, 2, 0, AddressingMode::None),
                // DEY
                Opcode::new(0x88, Mnemonic::DEY, 1, 2, 0, AddressingMode::None),
                // EOR
                Opcode::new(0x49, Mnemonic::EOR, 2, 2, 0, AddressingMode::Immediate),
                Opcode::new(0x45, Mnemonic::EOR, 2, 3, 0, AddressingMode::ZeroPage),
                Opcode::new(0x55, Mnemonic::EOR, 2, 4, 0, AddressingMode::ZeroPageX),
                Opcode::new(0x4D, Mnemonic::EOR, 3, 4, 0, AddressingMode::Absolute),
                Opcode::new(0x5D, Mnemonic::EOR, 3, 4, 1, AddressingMode::AbsoluteX),
                Opcode::new(0x59, Mnemonic::EOR, 3, 4, 1, AddressingMode::AbsoluteY),
                Opcode::new(0x41, Mnemonic::EOR, 2, 6, 0, AddressingMode::IndirectX),
                Opcode::new(0x51, Mnemonic::EOR, 2, 5, 1, AddressingMode::IndirectY),
                // INC
                Opcode::new(0xE6, Mnemonic::INC, 2, 5, 0, AddressingMode::ZeroPage),
                Opcode::new(0xF6, Mnemonic::INC, 2, 6, 0, AddressingMode::ZeroPageX),
                Opcode::new(0xEE, Mnemonic::INC, 3, 6, 0, AddressingMode::Absolute),
                Opcode::new(0xFE, Mnemonic::INC, 3, 7, 0, AddressingMode::AbsoluteX),
                // INX
                Opcode::new(0xE8, Mnemonic::INX, 1, 2, 0, AddressingMode::None),
                // INY
                Opcode::new(0xC8, Mnemonic::INY, 1, 2, 0, AddressingMode::None),
                // JMP
                Opcode::new(0x4C, Mnemonic::JMP, 3, 3, 0, AddressingMode::Absolute),
                Opcode::new(0x6C, Mnemonic::JMP, 3, 5, 0, AddressingMode::Indirect),
                // JSR
                Opcode::new(0x20, Mnemonic::JSR, 3, 6, 0, AddressingMode::Absolute),
                // LDA
                Opcode::new(0xA9, Mnemonic::LDA, 2, 2, 0, AddressingMode::Immediate),
                Opcode::new(0xA5, Mnemonic::LDA, 2, 3, 0, AddressingMode::ZeroPage),
                Opcode::new(0xB5, Mnemonic::LDA, 2, 4, 0, AddressingMode::ZeroPageX),
                Opcode::new(0xAD, Mnemonic::LDA, 3, 4, 0, AddressingMode::Absolute),
                Opcode::new(0xBD, Mnemonic::LDA, 3, 4, 1, AddressingMode::AbsoluteX),
                Opcode::new(0xB9, Mnemonic::LDA, 3, 4, 1, AddressingMode::AbsoluteY),
                Opcode::new(0xA1, Mnemonic::LDA, 2, 6, 0, AddressingMode::IndirectX),
                Opcode::new(0xB1, Mnemonic::LDA, 2, 5, 1, AddressingMode::IndirectY),
                // LDX
                Opcode::new(0xA2, Mnemonic::LDX, 2, 2, 0, AddressingMode::Immediate),
                Opcode::new(0xA6, Mnemonic::LDX, 2, 3, 0, AddressingMode::ZeroPage),
                Opcode::new(0xB6, Mnemonic::LDX, 2, 4, 0, AddressingMode::ZeroPageY),
                Opcode::new(0xAE, Mnemonic::LDX, 3, 4, 0, AddressingMode::Absolute),
                Opcode::new(0xBE, Mnemonic::LDX, 3, 4, 1, AddressingMode::AbsoluteY),
                // LDY
                Opcode::new(0xA0, Mnemonic::LDY, 2, 2, 0, AddressingMode::Immediate),
                Opcode::new(0xA4, Mnemonic::LDY, 2, 3, 0, AddressingMode::ZeroPage),
                Opcode::new(0xB4, Mnemonic::LDY, 2, 4, 0, AddressingMode::ZeroPageX),
                Opcode::new(0xAC, Mnemonic::LDY, 3, 4, 0, AddressingMode::Absolute),
                Opcode::new(0xBC, Mnemonic::LDY, 3, 4, 1, AddressingMode::AbsoluteX),
                // LSR
                Opcode::new(0x4A, Mnemonic::LSR, 1, 2, 0, AddressingMode::Accumulator),
                Opcode::new(0x46, Mnemonic::LSR, 2, 5, 0, AddressingMode::ZeroPage),
                Opcode::new(0x56, Mnemonic::LSR, 2, 6, 0, AddressingMode::ZeroPageX),
                Opcode::new(0x4E, Mnemonic::LSR, 3, 6, 0, AddressingMode::Absolute),
                Opcode::new(0x5E, Mnemonic::LSR, 3, 7, 0, AddressingMode::AbsoluteX),
                // NOP
                Opcode::new(0xEA, Mnemonic::NOP, 1, 2, 0, AddressingMode::None),
                // ORA
                Opcode::new(0x09, Mnemonic::ORA, 2, 2, 0, AddressingMode::Immediate),
                Opcode::new(0x05, Mnemonic::ORA, 2, 3, 0, AddressingMode::ZeroPage),
                Opcode::new(0x15, Mnemonic::ORA, 2, 4, 0, AddressingMode::ZeroPageX),
                Opcode::new(0x0D, Mnemonic::ORA, 3, 4, 0, AddressingMode::Absolute),
                Opcode::new(0x1D, Mnemonic::ORA, 3, 4, 1, AddressingMode::AbsoluteX),
                Opcode::new(0x19, Mnemonic::ORA, 3, 4, 1, AddressingMode::AbsoluteY),
                Opcode::new(0x01, Mnemonic::ORA, 2, 6, 0, AddressingMode::IndirectX),
                Opcode::new(0x11, Mnemonic::ORA, 2, 5, 1, AddressingMode::IndirectY),
                // PHA
                Opcode::new(0x48, Mnemonic::PHA, 1, 3, 0, AddressingMode::None),
                // PHP
                Opcode::new(0x08, Mnemonic::PHP, 1, 3, 0, AddressingMode::None),
                // PLA
                Opcode::new(0x68, Mnemonic::PLA, 1, 4, 0, AddressingMode::None),
                // PLP
                Opcode::new(0x28, Mnemonic::PLP, 1, 4, 0, AddressingMode::None),
                // ROL
                Opcode::new(0x2A, Mnemonic::ROL, 1, 2, 0, AddressingMode::Accumulator),
                Opcode::new(0x26, Mnemonic::ROL, 2, 5, 0, AddressingMode::ZeroPage),
                Opcode::new(0x36, Mnemonic::ROL, 2, 6, 0, AddressingMode::ZeroPageX),
                Opcode::new(0x2E, Mnemonic::ROL, 3, 6, 0, AddressingMode::Absolute),
                Opcode::new(0x3E, Mnemonic::ROL, 3, 7, 0, AddressingMode::AbsoluteX),
                // ROR
                Opcode::new(0x6A, Mnemonic::ROR, 1, 2, 0, AddressingMode::Accumulator),
                Opcode::new(0x66, Mnemonic::ROR, 2, 5, 0, AddressingMode::ZeroPage),
                Opcode::new(0x76, Mnemonic::ROR, 2, 6, 0, AddressingMode::ZeroPageX),
                Opcode::new(0x6E, Mnemonic::ROR, 3, 6, 0, AddressingMode::Absolute),
                Opcode::new(0x7E, Mnemonic::ROR, 3, 7, 0, AddressingMode::AbsoluteX),
                // RTI
                Opcode::new(0x40, Mnemonic::RTI, 1, 6, 0, AddressingMode::None),
                // RTS
                Opcode::new(0x60, Mnemonic::RTS, 1, 6, 0, AddressingMode::None),
                // SBC
                Opcode::new(0xE9, Mnemonic::SBC, 2, 2, 0, AddressingMode::Immediate),
                Opcode::new(0xE5, Mnemonic::SBC, 2, 3, 0, AddressingMode::ZeroPage),
                Opcode::new(0xF5, Mnemonic::SBC, 2, 4, 0, AddressingMode::ZeroPageX),
                Opcode::new(0xED, Mnemonic::SBC, 3, 4, 0, AddressingMode::Absolute),
                Opcode::new(0xFD, Mnemonic::SBC, 3, 4, 1, AddressingMode::AbsoluteX),
                Opcode::new(0xF9, Mnemonic::SBC, 3, 4, 1, AddressingMode::AbsoluteY),
                Opcode::new(0xE1, Mnemonic::SBC, 2, 6, 0, AddressingMode::IndirectX),
                Opcode::new(0xF1, Mnemonic::SBC, 2, 5, 1, AddressingMode::IndirectY),
                // SEC
                Opcode::new(0x38, Mnemonic::SEC, 1, 2, 0, AddressingMode::None),
                // SED
                Opcode::new(0xF8, Mnemonic::SED, 1, 2, 0, AddressingMode::None),
                // SEI
                Opcode::new(0x78, Mnemonic::SEI, 1, 2, 0, AddressingMode::None),
                // STA
                Opcode::new(0x85, Mnemonic::STA, 2, 3, 0, AddressingMode::ZeroPage),
                Opcode::new(0x95, Mnemonic::STA, 2, 4, 0, AddressingMode::ZeroPageX),
                Opcode::new(0x8D, Mnemonic::STA, 3, 4, 0, AddressingMode::Absolute),
                Opcode::new(0x9D, Mnemonic::STA, 3, 5, 0, AddressingMode::AbsoluteX),
                Opcode::new(0x99, Mnemonic::STA, 3, 5, 0, AddressingMode::AbsoluteY),
                Opcode::new(0x81, Mnemonic::STA, 2, 6, 0, AddressingMode::IndirectX),
                Opcode::new(0x91, Mnemonic::STA, 2, 6, 0, AddressingMode::IndirectY),
                // STX
                Opcode::new(0x86, Mnemonic::STX, 2, 3, 0, AddressingMode::ZeroPage),
                Opcode::new(0x96, Mnemonic::STX, 2, 4, 0, AddressingMode::ZeroPageY),
                Opcode::new(0x8E, Mnemonic::STX, 3, 4, 0, AddressingMode::Absolute),
                // STY
                Opcode::new(0x84, Mnemonic::STY, 2, 3, 0, AddressingMode::ZeroPage),
                Opcode::new(0x94, Mnemonic::STY, 2, 4, 0, AddressingMode::ZeroPageX),
                Opcode::new(0x8C, Mnemonic::STY, 3, 4, 0, AddressingMode::Absolute),
                // TAX
                Opcode::new(0xAA, Mnemonic::TAX, 1, 2, 0, AddressingMode::None),
                // TAY
                Opcode::new(0xA8, Mnemonic::TAY, 1, 2, 0, AddressingMode::None),
                // TSX
                Opcode::new(0xBA, Mnemonic::TSX, 1, 2, 0, AddressingMode::None),
                // TXA
                Opcode::new(0x8A, Mnemonic::TXA, 1, 2, 0, AddressingMode::None),
                // TXS
                Opcode::new(0x9A, Mnemonic::TXS, 1, 2, 0, AddressingMode::None),
                // TYA
                Opcode::new(0x98, Mnemonic::TYA, 1, 2, 0, AddressingMode::None),
            ],
        }
    }

    pub fn find_by_code(&self, code: u8) -> Option<&Opcode> {
        self.opcodes.iter().find(|opcode| opcode.code == code)
    }

    #[allow(dead_code)]
    pub fn get_opcodes(&self) -> &[Opcode] {
        &self.opcodes
    }
}

pub static CPU_OPCODES: LazyLock<OpcodeMap> = LazyLock::new(|| OpcodeMap::new());
