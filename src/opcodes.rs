use core::fmt;
use std::sync::LazyLock;

#[derive(Debug, PartialEq)]
pub enum Mnemonic {
    ADC,
    AND,
    ASL,
    AHX,
    ALR,
    ANC,
    ARR,
    AXS,
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
    DCP,
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
    ISC,
    INC,
    INX,
    INY,
    JMP,
    JSR,
    LDA,
    LAS,
    LDX,
    LAX,
    LXA,
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
    RLA,
    RRA,
    RTI,
    RTS,
    SAX,
    SBC,
    SLO,
    SRE,
    STP,
    SEC,
    SED,
    SHX,
    SHY,
    SEI,
    TAS,
    STA,
    STX,
    STY,
    XAA,
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

#[derive(Debug, PartialEq)]
pub enum AddressingMode {
    None,
    Accumulator,
    Immediate,
    ZeroPage,
    ZeroPageX,
    ZeroPageY,
    Relative,
    Absolute,
    AbsoluteX,
    AbsoluteY,
    Indirect,
    IndirectX,
    IndirectY,
}

#[derive(Debug)]
pub struct Opcode {
    pub code: u8,
    pub mnemonic: Mnemonic,
    pub bytes: u8,
    pub cycles: u8,
    pub mode: AddressingMode,
}

impl Opcode {
    pub const fn new(
        code: u8,
        mnemonic: Mnemonic,
        bytes: u8,
        cycles: u8,
        mode: AddressingMode,
    ) -> Self {
        Opcode {
            code,
            mnemonic,
            bytes,
            cycles,
            mode,
        }
    }
}

pub struct OpcodeMap {
    opcodes: Vec<Opcode>,
}

impl Default for OpcodeMap {
    fn default() -> Self {
        Self::new()
    }
}

impl OpcodeMap {
    pub fn new() -> Self {
        OpcodeMap {
            opcodes: vec![
                // ADC
                Opcode::new(0x69, Mnemonic::ADC, 2, 2, AddressingMode::Immediate),
                Opcode::new(0x65, Mnemonic::ADC, 2, 3, AddressingMode::ZeroPage),
                Opcode::new(0x75, Mnemonic::ADC, 2, 4, AddressingMode::ZeroPageX),
                Opcode::new(0x6D, Mnemonic::ADC, 3, 4, AddressingMode::Absolute),
                Opcode::new(0x7D, Mnemonic::ADC, 3, 4, AddressingMode::AbsoluteX),
                Opcode::new(0x79, Mnemonic::ADC, 3, 4, AddressingMode::AbsoluteY),
                Opcode::new(0x61, Mnemonic::ADC, 2, 6, AddressingMode::IndirectX),
                Opcode::new(0x71, Mnemonic::ADC, 2, 5, AddressingMode::IndirectY),
                // AND
                Opcode::new(0x29, Mnemonic::AND, 2, 2, AddressingMode::Immediate),
                Opcode::new(0x25, Mnemonic::AND, 2, 3, AddressingMode::ZeroPage),
                Opcode::new(0x35, Mnemonic::AND, 2, 4, AddressingMode::ZeroPageX),
                Opcode::new(0x2D, Mnemonic::AND, 3, 4, AddressingMode::Absolute),
                Opcode::new(0x3D, Mnemonic::AND, 3, 4, AddressingMode::AbsoluteX),
                Opcode::new(0x39, Mnemonic::AND, 3, 4, AddressingMode::AbsoluteY),
                Opcode::new(0x21, Mnemonic::AND, 2, 6, AddressingMode::IndirectX),
                Opcode::new(0x31, Mnemonic::AND, 2, 5, AddressingMode::IndirectY),
                // ASL
                Opcode::new(0x0A, Mnemonic::ASL, 1, 2, AddressingMode::Accumulator),
                Opcode::new(0x06, Mnemonic::ASL, 2, 5, AddressingMode::ZeroPage),
                Opcode::new(0x16, Mnemonic::ASL, 2, 6, AddressingMode::ZeroPageX),
                Opcode::new(0x0E, Mnemonic::ASL, 3, 6, AddressingMode::Absolute),
                Opcode::new(0x1E, Mnemonic::ASL, 3, 7, AddressingMode::AbsoluteX),
                // BCC
                Opcode::new(0x90, Mnemonic::BCC, 2, 2, AddressingMode::Relative),
                // BCS
                Opcode::new(0xB0, Mnemonic::BCS, 2, 2, AddressingMode::Relative),
                // BEQ
                Opcode::new(0xF0, Mnemonic::BEQ, 2, 2, AddressingMode::Relative),
                // BIT
                Opcode::new(0x24, Mnemonic::BIT, 2, 3, AddressingMode::ZeroPage),
                Opcode::new(0x2C, Mnemonic::BIT, 3, 4, AddressingMode::Absolute),
                // BMI
                Opcode::new(0x30, Mnemonic::BMI, 2, 2, AddressingMode::Relative),
                // BNE
                Opcode::new(0xD0, Mnemonic::BNE, 2, 2, AddressingMode::Relative),
                // BPL
                Opcode::new(0x10, Mnemonic::BPL, 2, 2, AddressingMode::Relative),
                // BRK
                Opcode::new(0x00, Mnemonic::BRK, 1, 7, AddressingMode::None),
                // BVC
                Opcode::new(0x50, Mnemonic::BVC, 2, 2, AddressingMode::Relative),
                // BVS
                Opcode::new(0x70, Mnemonic::BVS, 2, 2, AddressingMode::Relative),
                // CLC
                Opcode::new(0x18, Mnemonic::CLC, 1, 2, AddressingMode::None),
                // CLD
                Opcode::new(0xD8, Mnemonic::CLD, 1, 2, AddressingMode::None),
                // CLI
                Opcode::new(0x58, Mnemonic::CLI, 1, 2, AddressingMode::None),
                // CLV
                Opcode::new(0xB8, Mnemonic::CLV, 1, 2, AddressingMode::None),
                // CMP
                Opcode::new(0xC9, Mnemonic::CMP, 2, 2, AddressingMode::Immediate),
                Opcode::new(0xC5, Mnemonic::CMP, 2, 3, AddressingMode::ZeroPage),
                Opcode::new(0xD5, Mnemonic::CMP, 2, 4, AddressingMode::ZeroPageX),
                Opcode::new(0xCD, Mnemonic::CMP, 3, 4, AddressingMode::Absolute),
                Opcode::new(0xDD, Mnemonic::CMP, 3, 4, AddressingMode::AbsoluteX),
                Opcode::new(0xD9, Mnemonic::CMP, 3, 4, AddressingMode::AbsoluteY),
                Opcode::new(0xC1, Mnemonic::CMP, 2, 6, AddressingMode::IndirectX),
                Opcode::new(0xD1, Mnemonic::CMP, 2, 5, AddressingMode::IndirectY),
                // CPX
                Opcode::new(0xE0, Mnemonic::CPX, 2, 2, AddressingMode::Immediate),
                Opcode::new(0xE4, Mnemonic::CPX, 2, 3, AddressingMode::ZeroPage),
                Opcode::new(0xEC, Mnemonic::CPX, 3, 4, AddressingMode::Absolute),
                // CPY
                Opcode::new(0xC0, Mnemonic::CPY, 2, 2, AddressingMode::Immediate),
                Opcode::new(0xC4, Mnemonic::CPY, 2, 3, AddressingMode::ZeroPage),
                Opcode::new(0xCC, Mnemonic::CPY, 3, 4, AddressingMode::Absolute),
                // DEC
                Opcode::new(0xC6, Mnemonic::DEC, 2, 5, AddressingMode::ZeroPage),
                Opcode::new(0xD6, Mnemonic::DEC, 2, 6, AddressingMode::ZeroPageX),
                Opcode::new(0xCE, Mnemonic::DEC, 3, 6, AddressingMode::Absolute),
                Opcode::new(0xDE, Mnemonic::DEC, 3, 7, AddressingMode::AbsoluteX),
                // DEX
                Opcode::new(0xCA, Mnemonic::DEX, 1, 2, AddressingMode::None),
                // DEY
                Opcode::new(0x88, Mnemonic::DEY, 1, 2, AddressingMode::None),
                // EOR
                Opcode::new(0x49, Mnemonic::EOR, 2, 2, AddressingMode::Immediate),
                Opcode::new(0x45, Mnemonic::EOR, 2, 3, AddressingMode::ZeroPage),
                Opcode::new(0x55, Mnemonic::EOR, 2, 4, AddressingMode::ZeroPageX),
                Opcode::new(0x4D, Mnemonic::EOR, 3, 4, AddressingMode::Absolute),
                Opcode::new(0x5D, Mnemonic::EOR, 3, 4, AddressingMode::AbsoluteX),
                Opcode::new(0x59, Mnemonic::EOR, 3, 4, AddressingMode::AbsoluteY),
                Opcode::new(0x41, Mnemonic::EOR, 2, 6, AddressingMode::IndirectX),
                Opcode::new(0x51, Mnemonic::EOR, 2, 5, AddressingMode::IndirectY),
                // INC
                Opcode::new(0xE6, Mnemonic::INC, 2, 5, AddressingMode::ZeroPage),
                Opcode::new(0xF6, Mnemonic::INC, 2, 6, AddressingMode::ZeroPageX),
                Opcode::new(0xEE, Mnemonic::INC, 3, 6, AddressingMode::Absolute),
                Opcode::new(0xFE, Mnemonic::INC, 3, 7, AddressingMode::AbsoluteX),
                // INX
                Opcode::new(0xE8, Mnemonic::INX, 1, 2, AddressingMode::None),
                // INY
                Opcode::new(0xC8, Mnemonic::INY, 1, 2, AddressingMode::None),
                // JMP
                Opcode::new(0x4C, Mnemonic::JMP, 3, 3, AddressingMode::Absolute),
                Opcode::new(0x6C, Mnemonic::JMP, 3, 5, AddressingMode::Indirect),
                // JSR
                Opcode::new(0x20, Mnemonic::JSR, 3, 6, AddressingMode::Absolute),
                // LDA
                Opcode::new(0xA9, Mnemonic::LDA, 2, 2, AddressingMode::Immediate),
                Opcode::new(0xA5, Mnemonic::LDA, 2, 3, AddressingMode::ZeroPage),
                Opcode::new(0xB5, Mnemonic::LDA, 2, 4, AddressingMode::ZeroPageX),
                Opcode::new(0xAD, Mnemonic::LDA, 3, 4, AddressingMode::Absolute),
                Opcode::new(0xBD, Mnemonic::LDA, 3, 4, AddressingMode::AbsoluteX),
                Opcode::new(0xB9, Mnemonic::LDA, 3, 4, AddressingMode::AbsoluteY),
                Opcode::new(0xA1, Mnemonic::LDA, 2, 6, AddressingMode::IndirectX),
                Opcode::new(0xB1, Mnemonic::LDA, 2, 5, AddressingMode::IndirectY),
                // LDX
                Opcode::new(0xA2, Mnemonic::LDX, 2, 2, AddressingMode::Immediate),
                Opcode::new(0xA6, Mnemonic::LDX, 2, 3, AddressingMode::ZeroPage),
                Opcode::new(0xB6, Mnemonic::LDX, 2, 4, AddressingMode::ZeroPageY),
                Opcode::new(0xAE, Mnemonic::LDX, 3, 4, AddressingMode::Absolute),
                Opcode::new(0xBE, Mnemonic::LDX, 3, 4, AddressingMode::AbsoluteY),
                // LDY
                Opcode::new(0xA0, Mnemonic::LDY, 2, 2, AddressingMode::Immediate),
                Opcode::new(0xA4, Mnemonic::LDY, 2, 3, AddressingMode::ZeroPage),
                Opcode::new(0xB4, Mnemonic::LDY, 2, 4, AddressingMode::ZeroPageX),
                Opcode::new(0xAC, Mnemonic::LDY, 3, 4, AddressingMode::Absolute),
                Opcode::new(0xBC, Mnemonic::LDY, 3, 4, AddressingMode::AbsoluteX),
                // LSR
                Opcode::new(0x4A, Mnemonic::LSR, 1, 2, AddressingMode::Accumulator),
                Opcode::new(0x46, Mnemonic::LSR, 2, 5, AddressingMode::ZeroPage),
                Opcode::new(0x56, Mnemonic::LSR, 2, 6, AddressingMode::ZeroPageX),
                Opcode::new(0x4E, Mnemonic::LSR, 3, 6, AddressingMode::Absolute),
                Opcode::new(0x5E, Mnemonic::LSR, 3, 7, AddressingMode::AbsoluteX),
                // NOP
                Opcode::new(0xEA, Mnemonic::NOP, 1, 2, AddressingMode::None),
                // ORA
                Opcode::new(0x09, Mnemonic::ORA, 2, 2, AddressingMode::Immediate),
                Opcode::new(0x05, Mnemonic::ORA, 2, 3, AddressingMode::ZeroPage),
                Opcode::new(0x15, Mnemonic::ORA, 2, 4, AddressingMode::ZeroPageX),
                Opcode::new(0x0D, Mnemonic::ORA, 3, 4, AddressingMode::Absolute),
                Opcode::new(0x1D, Mnemonic::ORA, 3, 4, AddressingMode::AbsoluteX),
                Opcode::new(0x19, Mnemonic::ORA, 3, 4, AddressingMode::AbsoluteY),
                Opcode::new(0x01, Mnemonic::ORA, 2, 6, AddressingMode::IndirectX),
                Opcode::new(0x11, Mnemonic::ORA, 2, 5, AddressingMode::IndirectY),
                // PHA
                Opcode::new(0x48, Mnemonic::PHA, 1, 3, AddressingMode::None),
                // PHP
                Opcode::new(0x08, Mnemonic::PHP, 1, 3, AddressingMode::None),
                // PLA
                Opcode::new(0x68, Mnemonic::PLA, 1, 4, AddressingMode::None),
                // PLP
                Opcode::new(0x28, Mnemonic::PLP, 1, 4, AddressingMode::None),
                // ROL
                Opcode::new(0x2A, Mnemonic::ROL, 1, 2, AddressingMode::Accumulator),
                Opcode::new(0x26, Mnemonic::ROL, 2, 5, AddressingMode::ZeroPage),
                Opcode::new(0x36, Mnemonic::ROL, 2, 6, AddressingMode::ZeroPageX),
                Opcode::new(0x2E, Mnemonic::ROL, 3, 6, AddressingMode::Absolute),
                Opcode::new(0x3E, Mnemonic::ROL, 3, 7, AddressingMode::AbsoluteX),
                // ROR
                Opcode::new(0x6A, Mnemonic::ROR, 1, 2, AddressingMode::Accumulator),
                Opcode::new(0x66, Mnemonic::ROR, 2, 5, AddressingMode::ZeroPage),
                Opcode::new(0x76, Mnemonic::ROR, 2, 6, AddressingMode::ZeroPageX),
                Opcode::new(0x6E, Mnemonic::ROR, 3, 6, AddressingMode::Absolute),
                Opcode::new(0x7E, Mnemonic::ROR, 3, 7, AddressingMode::AbsoluteX),
                // RTI
                Opcode::new(0x40, Mnemonic::RTI, 1, 6, AddressingMode::None),
                // RTS
                Opcode::new(0x60, Mnemonic::RTS, 1, 6, AddressingMode::None),
                // SBC
                Opcode::new(0xE9, Mnemonic::SBC, 2, 2, AddressingMode::Immediate),
                Opcode::new(0xE5, Mnemonic::SBC, 2, 3, AddressingMode::ZeroPage),
                Opcode::new(0xF5, Mnemonic::SBC, 2, 4, AddressingMode::ZeroPageX),
                Opcode::new(0xED, Mnemonic::SBC, 3, 4, AddressingMode::Absolute),
                Opcode::new(0xFD, Mnemonic::SBC, 3, 4, AddressingMode::AbsoluteX),
                Opcode::new(0xF9, Mnemonic::SBC, 3, 4, AddressingMode::AbsoluteY),
                Opcode::new(0xE1, Mnemonic::SBC, 2, 6, AddressingMode::IndirectX),
                Opcode::new(0xF1, Mnemonic::SBC, 2, 5, AddressingMode::IndirectY),
                // SEC
                Opcode::new(0x38, Mnemonic::SEC, 1, 2, AddressingMode::None),
                // SED
                Opcode::new(0xF8, Mnemonic::SED, 1, 2, AddressingMode::None),
                // SEI
                Opcode::new(0x78, Mnemonic::SEI, 1, 2, AddressingMode::None),
                // STA
                Opcode::new(0x85, Mnemonic::STA, 2, 3, AddressingMode::ZeroPage),
                Opcode::new(0x95, Mnemonic::STA, 2, 4, AddressingMode::ZeroPageX),
                Opcode::new(0x8D, Mnemonic::STA, 3, 4, AddressingMode::Absolute),
                Opcode::new(0x9D, Mnemonic::STA, 3, 5, AddressingMode::AbsoluteX),
                Opcode::new(0x99, Mnemonic::STA, 3, 5, AddressingMode::AbsoluteY),
                Opcode::new(0x81, Mnemonic::STA, 2, 6, AddressingMode::IndirectX),
                Opcode::new(0x91, Mnemonic::STA, 2, 6, AddressingMode::IndirectY),
                // STX
                Opcode::new(0x86, Mnemonic::STX, 2, 3, AddressingMode::ZeroPage),
                Opcode::new(0x96, Mnemonic::STX, 2, 4, AddressingMode::ZeroPageY),
                Opcode::new(0x8E, Mnemonic::STX, 3, 4, AddressingMode::Absolute),
                // STY
                Opcode::new(0x84, Mnemonic::STY, 2, 3, AddressingMode::ZeroPage),
                Opcode::new(0x94, Mnemonic::STY, 2, 4, AddressingMode::ZeroPageX),
                Opcode::new(0x8C, Mnemonic::STY, 3, 4, AddressingMode::Absolute),
                // TAX
                Opcode::new(0xAA, Mnemonic::TAX, 1, 2, AddressingMode::None),
                // TAY
                Opcode::new(0xA8, Mnemonic::TAY, 1, 2, AddressingMode::None),
                // TSX
                Opcode::new(0xBA, Mnemonic::TSX, 1, 2, AddressingMode::None),
                // TXA
                Opcode::new(0x8A, Mnemonic::TXA, 1, 2, AddressingMode::None),
                // TXS
                Opcode::new(0x9A, Mnemonic::TXS, 1, 2, AddressingMode::None),
                // TYA
                Opcode::new(0x98, Mnemonic::TYA, 1, 2, AddressingMode::None),
                // Unofficial instructions (see NESdev CPU unofficial opcodes)
                // ANC
                Opcode::new(0x0B, Mnemonic::ANC, 2, 2, AddressingMode::Immediate),
                Opcode::new(0x2B, Mnemonic::ANC, 2, 2, AddressingMode::Immediate),
                // ALR
                Opcode::new(0x4B, Mnemonic::ALR, 2, 2, AddressingMode::Immediate),
                // ARR
                Opcode::new(0x6B, Mnemonic::ARR, 2, 2, AddressingMode::Immediate),
                // XAA
                Opcode::new(0x8B, Mnemonic::XAA, 2, 2, AddressingMode::Immediate),
                // LXA (a.k.a. LAX #imm)
                Opcode::new(0xAB, Mnemonic::LXA, 2, 2, AddressingMode::Immediate),
                // AXS
                Opcode::new(0xCB, Mnemonic::AXS, 2, 2, AddressingMode::Immediate),
                // Extra SBC immediate
                Opcode::new(0xEB, Mnemonic::SBC, 2, 2, AddressingMode::Immediate),
                // SLO
                Opcode::new(0x03, Mnemonic::SLO, 2, 8, AddressingMode::IndirectX),
                Opcode::new(0x07, Mnemonic::SLO, 2, 5, AddressingMode::ZeroPage),
                Opcode::new(0x0F, Mnemonic::SLO, 3, 6, AddressingMode::Absolute),
                Opcode::new(0x13, Mnemonic::SLO, 2, 8, AddressingMode::IndirectY),
                Opcode::new(0x17, Mnemonic::SLO, 2, 6, AddressingMode::ZeroPageX),
                Opcode::new(0x1B, Mnemonic::SLO, 3, 7, AddressingMode::AbsoluteY),
                Opcode::new(0x1F, Mnemonic::SLO, 3, 7, AddressingMode::AbsoluteX),
                // RLA
                Opcode::new(0x23, Mnemonic::RLA, 2, 8, AddressingMode::IndirectX),
                Opcode::new(0x27, Mnemonic::RLA, 2, 5, AddressingMode::ZeroPage),
                Opcode::new(0x2F, Mnemonic::RLA, 3, 6, AddressingMode::Absolute),
                Opcode::new(0x33, Mnemonic::RLA, 2, 8, AddressingMode::IndirectY),
                Opcode::new(0x37, Mnemonic::RLA, 2, 6, AddressingMode::ZeroPageX),
                Opcode::new(0x3B, Mnemonic::RLA, 3, 7, AddressingMode::AbsoluteY),
                Opcode::new(0x3F, Mnemonic::RLA, 3, 7, AddressingMode::AbsoluteX),
                // SRE
                Opcode::new(0x43, Mnemonic::SRE, 2, 8, AddressingMode::IndirectX),
                Opcode::new(0x47, Mnemonic::SRE, 2, 5, AddressingMode::ZeroPage),
                Opcode::new(0x4F, Mnemonic::SRE, 3, 6, AddressingMode::Absolute),
                Opcode::new(0x53, Mnemonic::SRE, 2, 8, AddressingMode::IndirectY),
                Opcode::new(0x57, Mnemonic::SRE, 2, 6, AddressingMode::ZeroPageX),
                Opcode::new(0x5B, Mnemonic::SRE, 3, 7, AddressingMode::AbsoluteY),
                Opcode::new(0x5F, Mnemonic::SRE, 3, 7, AddressingMode::AbsoluteX),
                // RRA
                Opcode::new(0x63, Mnemonic::RRA, 2, 8, AddressingMode::IndirectX),
                Opcode::new(0x67, Mnemonic::RRA, 2, 5, AddressingMode::ZeroPage),
                Opcode::new(0x6F, Mnemonic::RRA, 3, 6, AddressingMode::Absolute),
                Opcode::new(0x73, Mnemonic::RRA, 2, 8, AddressingMode::IndirectY),
                Opcode::new(0x77, Mnemonic::RRA, 2, 6, AddressingMode::ZeroPageX),
                Opcode::new(0x7B, Mnemonic::RRA, 3, 7, AddressingMode::AbsoluteY),
                Opcode::new(0x7F, Mnemonic::RRA, 3, 7, AddressingMode::AbsoluteX),
                // SAX
                Opcode::new(0x83, Mnemonic::SAX, 2, 6, AddressingMode::IndirectX),
                Opcode::new(0x87, Mnemonic::SAX, 2, 3, AddressingMode::ZeroPage),
                Opcode::new(0x8F, Mnemonic::SAX, 3, 4, AddressingMode::Absolute),
                Opcode::new(0x97, Mnemonic::SAX, 2, 4, AddressingMode::ZeroPageY),
                // AHX
                Opcode::new(0x93, Mnemonic::AHX, 2, 6, AddressingMode::IndirectY),
                Opcode::new(0x9F, Mnemonic::AHX, 3, 5, AddressingMode::AbsoluteY),
                // SHY
                Opcode::new(0x9C, Mnemonic::SHY, 3, 5, AddressingMode::AbsoluteX),
                // TAS
                Opcode::new(0x9B, Mnemonic::TAS, 3, 5, AddressingMode::AbsoluteY),
                // SHX
                Opcode::new(0x9E, Mnemonic::SHX, 3, 5, AddressingMode::AbsoluteY),
                // LAX
                Opcode::new(0xA3, Mnemonic::LAX, 2, 6, AddressingMode::IndirectX),
                Opcode::new(0xA7, Mnemonic::LAX, 2, 3, AddressingMode::ZeroPage),
                Opcode::new(0xAF, Mnemonic::LAX, 3, 4, AddressingMode::Absolute),
                Opcode::new(0xB3, Mnemonic::LAX, 2, 5, AddressingMode::IndirectY),
                Opcode::new(0xB7, Mnemonic::LAX, 2, 4, AddressingMode::ZeroPageY),
                Opcode::new(0xBF, Mnemonic::LAX, 3, 4, AddressingMode::AbsoluteY),
                // LAS
                Opcode::new(0xBB, Mnemonic::LAS, 3, 4, AddressingMode::AbsoluteY),
                // DCP
                Opcode::new(0xC3, Mnemonic::DCP, 2, 8, AddressingMode::IndirectX),
                Opcode::new(0xC7, Mnemonic::DCP, 2, 5, AddressingMode::ZeroPage),
                Opcode::new(0xCF, Mnemonic::DCP, 3, 6, AddressingMode::Absolute),
                Opcode::new(0xD3, Mnemonic::DCP, 2, 8, AddressingMode::IndirectY),
                Opcode::new(0xD7, Mnemonic::DCP, 2, 6, AddressingMode::ZeroPageX),
                Opcode::new(0xDB, Mnemonic::DCP, 3, 7, AddressingMode::AbsoluteY),
                Opcode::new(0xDF, Mnemonic::DCP, 3, 7, AddressingMode::AbsoluteX),
                // ISC
                Opcode::new(0xE3, Mnemonic::ISC, 2, 8, AddressingMode::IndirectX),
                Opcode::new(0xE7, Mnemonic::ISC, 2, 5, AddressingMode::ZeroPage),
                Opcode::new(0xEF, Mnemonic::ISC, 3, 6, AddressingMode::Absolute),
                Opcode::new(0xF3, Mnemonic::ISC, 2, 8, AddressingMode::IndirectY),
                Opcode::new(0xF7, Mnemonic::ISC, 2, 6, AddressingMode::ZeroPageX),
                Opcode::new(0xFB, Mnemonic::ISC, 3, 7, AddressingMode::AbsoluteY),
                Opcode::new(0xFF, Mnemonic::ISC, 3, 7, AddressingMode::AbsoluteX),
                // STP / JAM (halts CPU)
                Opcode::new(0x02, Mnemonic::STP, 1, 0, AddressingMode::None),
                Opcode::new(0x12, Mnemonic::STP, 1, 0, AddressingMode::None),
                Opcode::new(0x22, Mnemonic::STP, 1, 0, AddressingMode::None),
                Opcode::new(0x32, Mnemonic::STP, 1, 0, AddressingMode::None),
                Opcode::new(0x42, Mnemonic::STP, 1, 0, AddressingMode::None),
                Opcode::new(0x52, Mnemonic::STP, 1, 0, AddressingMode::None),
                Opcode::new(0x62, Mnemonic::STP, 1, 0, AddressingMode::None),
                Opcode::new(0x72, Mnemonic::STP, 1, 0, AddressingMode::None),
                Opcode::new(0x92, Mnemonic::STP, 1, 0, AddressingMode::None),
                Opcode::new(0xB2, Mnemonic::STP, 1, 0, AddressingMode::None),
                Opcode::new(0xD2, Mnemonic::STP, 1, 0, AddressingMode::None),
                Opcode::new(0xF2, Mnemonic::STP, 1, 0, AddressingMode::None),
                // Unofficial NOPs (various addressing modes)
                Opcode::new(0x1A, Mnemonic::NOP, 1, 2, AddressingMode::None),
                Opcode::new(0x3A, Mnemonic::NOP, 1, 2, AddressingMode::None),
                Opcode::new(0x5A, Mnemonic::NOP, 1, 2, AddressingMode::None),
                Opcode::new(0x7A, Mnemonic::NOP, 1, 2, AddressingMode::None),
                Opcode::new(0xDA, Mnemonic::NOP, 1, 2, AddressingMode::None),
                Opcode::new(0xFA, Mnemonic::NOP, 1, 2, AddressingMode::None),
                Opcode::new(0x80, Mnemonic::NOP, 2, 2, AddressingMode::Immediate),
                Opcode::new(0x82, Mnemonic::NOP, 2, 2, AddressingMode::Immediate),
                Opcode::new(0x89, Mnemonic::NOP, 2, 2, AddressingMode::Immediate),
                Opcode::new(0xC2, Mnemonic::NOP, 2, 2, AddressingMode::Immediate),
                Opcode::new(0xE2, Mnemonic::NOP, 2, 2, AddressingMode::Immediate),
                Opcode::new(0x04, Mnemonic::NOP, 2, 3, AddressingMode::ZeroPage),
                Opcode::new(0x44, Mnemonic::NOP, 2, 3, AddressingMode::ZeroPage),
                Opcode::new(0x64, Mnemonic::NOP, 2, 3, AddressingMode::ZeroPage),
                Opcode::new(0x14, Mnemonic::NOP, 2, 4, AddressingMode::ZeroPageX),
                Opcode::new(0x34, Mnemonic::NOP, 2, 4, AddressingMode::ZeroPageX),
                Opcode::new(0x54, Mnemonic::NOP, 2, 4, AddressingMode::ZeroPageX),
                Opcode::new(0x74, Mnemonic::NOP, 2, 4, AddressingMode::ZeroPageX),
                Opcode::new(0xD4, Mnemonic::NOP, 2, 4, AddressingMode::ZeroPageX),
                Opcode::new(0xF4, Mnemonic::NOP, 2, 4, AddressingMode::ZeroPageX),
                Opcode::new(0x0C, Mnemonic::NOP, 3, 4, AddressingMode::Absolute),
                Opcode::new(0x1C, Mnemonic::NOP, 3, 4, AddressingMode::AbsoluteX),
                Opcode::new(0x3C, Mnemonic::NOP, 3, 4, AddressingMode::AbsoluteX),
                Opcode::new(0x5C, Mnemonic::NOP, 3, 4, AddressingMode::AbsoluteX),
                Opcode::new(0x7C, Mnemonic::NOP, 3, 4, AddressingMode::AbsoluteX),
                Opcode::new(0xDC, Mnemonic::NOP, 3, 4, AddressingMode::AbsoluteX),
                Opcode::new(0xFC, Mnemonic::NOP, 3, 4, AddressingMode::AbsoluteX),
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

pub static CPU_OPCODES: LazyLock<OpcodeMap> = LazyLock::new(OpcodeMap::new);
