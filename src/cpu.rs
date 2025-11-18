use std::fmt::Debug;

use bitflags::bitflags;

use crate::memory::Memory;
use crate::opcodes::{AddressingMode, CPU_OPCODES, Mnemonic};

pub const STACK_START: u16 = 0x0100;
pub const PRG_START: u16 = 0x8000;

bitflags! {
    pub struct StatusFlags: u8 {
        const CARRY = 0b0000_0001;
        const ZERO = 0b0000_0010;
        const INTERRUPT_DISABLE = 0b0000_0100;
        const DECIMAL_MODE = 0b0000_1000;
        const BREAK_COMMAND = 0b0001_0000;
        const UNUSED = 0b0010_0000;
        const OVERFLOW = 0b0100_0000;
        const NEGATIVE = 0b1000_0000;
    }
}

pub struct Registers {
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub status: StatusFlags,
    pub pc: u16,
    pub sp: u8,
}

impl Debug for Registers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "A: {:02X}, X: {:02X}, Y: {:02X}, STATUS: {:08b}, PC: {:04X}",
            self.a, self.x, self.y, self.status, self.pc
        )
    }
}

mod interrupt {
    #[derive(PartialEq, Eq)]
    pub enum InterruptType {
        NMI,
        IRQ,
        BRK,
    }

    #[derive(PartialEq, Eq)]
    pub(super) struct Interrupt {
        pub(super) itype: InterruptType,
        pub(super) vector_addr: u16,
        pub(super) b_flag_mask: u8,
        pub(super) cpu_cycles: u8,
    }

    pub(super) const NMI: Interrupt = Interrupt {
        itype: InterruptType::NMI,
        vector_addr: 0xFFFA,
        b_flag_mask: 0b00100000,
        cpu_cycles: 2,
    };

    pub(super) const IRQ: Interrupt = Interrupt {
        itype: InterruptType::IRQ,
        vector_addr: 0xFFFE,
        b_flag_mask: 0b00100000,
        cpu_cycles: 2,
    };

    pub(super) const BRK: Interrupt = Interrupt {
        itype: InterruptType::BRK,
        vector_addr: 0xFFFE,
        b_flag_mask: 0b00110000,
        cpu_cycles: 1,
    };
}

pub struct CPU<M: Memory> {
    pub registers: Registers,
    pub memory: M,
    extra_cycles: u8,
    halted: bool,
}

impl<M: Memory> CPU<M> {
    pub fn new(memory: M) -> Self {
        CPU {
            registers: Registers {
                a: 0,
                x: 0,
                y: 0,
                status: StatusFlags::from_bits_truncate(0b00100100),
                pc: PRG_START,
                sp: 0xFD,
            },
            memory,
            extra_cycles: 0,
            halted: false,
        }
    }

    pub fn run(&mut self) {
        self.run_with_callback(|_| {});
    }

    pub fn run_with_callback<F>(&mut self, mut callback: F)
    where
        F: FnMut(&mut CPU<M>),
    {
        loop {
            if self.halted {
                break;
            }

            callback(self);

            // Check for NMI first (has higher priority)
            if let Some(_nmi) = self.memory.poll_nmi_status() {
                self.interrupt(interrupt::NMI);
            }

            // Check for IRQ (maskable interrupt) if not disabled
            if let Some(_irq) = self.memory.poll_irq_status()
                && !self
                    .registers
                    .status
                    .contains(StatusFlags::INTERRUPT_DISABLE)
            {
                self.interrupt(interrupt::IRQ);
            }

            let opcode = self.memory.read(self.registers.pc);
            // println!("{:?}", self.registers);

            self.registers.pc = self.registers.pc.wrapping_add(1);

            let pc_state = self.registers.pc;

            if let Some(opcode_info) = CPU_OPCODES.find_by_code(opcode) {
                match opcode_info.mnemonic {
                    Mnemonic::ADC => {
                        self.adc(&opcode_info.mode);
                    }
                    Mnemonic::AND => {
                        self.and(&opcode_info.mode);
                    }
                    Mnemonic::ASL => {
                        self.asl(&opcode_info.mode);
                    }
                    Mnemonic::BCC => {
                        self.bcc(&opcode_info.mode);
                    }
                    Mnemonic::BCS => {
                        self.bcs(&opcode_info.mode);
                    }
                    Mnemonic::BEQ => {
                        self.beq(&opcode_info.mode);
                    }
                    Mnemonic::BIT => {
                        self.bit(&opcode_info.mode);
                    }
                    Mnemonic::BMI => {
                        self.bmi(&opcode_info.mode);
                    }
                    Mnemonic::BNE => {
                        self.bne(&opcode_info.mode);
                    }
                    Mnemonic::BPL => {
                        self.bpl(&opcode_info.mode);
                    }
                    Mnemonic::BRK => {
                        self.brk(&opcode_info.mode);
                        // println!("BRK encountered. Halting execution.");
                        // break;
                    }
                    Mnemonic::BVC => {
                        self.bvc(&opcode_info.mode);
                    }
                    Mnemonic::BVS => {
                        self.bvs(&opcode_info.mode);
                    }
                    Mnemonic::CLC => {
                        self.clc();
                    }
                    Mnemonic::CLD => {
                        self.cld();
                    }
                    Mnemonic::CLI => {
                        self.cli();
                    }
                    Mnemonic::CLV => {
                        self.clv();
                    }
                    Mnemonic::CMP => {
                        self.cmp(&opcode_info.mode);
                    }
                    Mnemonic::CPX => {
                        self.cpx(&opcode_info.mode);
                    }
                    Mnemonic::CPY => {
                        self.cpy(&opcode_info.mode);
                    }
                    Mnemonic::DEC => {
                        self.dec(&opcode_info.mode);
                    }
                    Mnemonic::DEX => {
                        self.dex();
                    }
                    Mnemonic::DEY => {
                        self.dey();
                    }
                    Mnemonic::EOR => {
                        self.eor(&opcode_info.mode);
                    }
                    Mnemonic::INC => {
                        self.inc(&opcode_info.mode);
                    }
                    Mnemonic::INX => {
                        self.inx();
                    }
                    Mnemonic::INY => {
                        self.iny();
                    }
                    Mnemonic::JMP => {
                        self.jmp(&opcode_info.mode);
                    }
                    Mnemonic::JSR => {
                        self.jsr(&opcode_info.mode);
                    }
                    Mnemonic::LDA => {
                        self.lda(&opcode_info.mode);
                    }
                    Mnemonic::LDX => {
                        self.ldx(&opcode_info.mode);
                    }
                    Mnemonic::LDY => {
                        self.ldy(&opcode_info.mode);
                    }
                    Mnemonic::LSR => {
                        self.lsr(&opcode_info.mode);
                    }
                    Mnemonic::NOP => {
                        self.nop(&opcode_info.mode);
                    }
                    Mnemonic::ORA => {
                        self.ora(&opcode_info.mode);
                    }
                    Mnemonic::PHA => {
                        self.pha();
                    }
                    Mnemonic::PHP => {
                        self.php();
                    }
                    Mnemonic::PLA => {
                        self.pla();
                    }
                    Mnemonic::PLP => {
                        self.plp();
                    }
                    Mnemonic::ROL => {
                        self.rol(&opcode_info.mode);
                    }
                    Mnemonic::ROR => {
                        self.ror(&opcode_info.mode);
                    }
                    Mnemonic::RTI => {
                        self.rti();
                    }
                    Mnemonic::RTS => {
                        self.rts();
                    }
                    Mnemonic::SBC => {
                        self.sbc(&opcode_info.mode);
                    }
                    Mnemonic::SEC => {
                        self.sec();
                    }
                    Mnemonic::SED => {
                        self.sed();
                    }
                    Mnemonic::SEI => {
                        self.sei();
                    }
                    Mnemonic::STA => {
                        self.sta(&opcode_info.mode);
                    }
                    Mnemonic::STX => {
                        self.stx(&opcode_info.mode);
                    }
                    Mnemonic::STY => {
                        self.sty(&opcode_info.mode);
                    }
                    Mnemonic::TAX => {
                        self.tax();
                    }
                    Mnemonic::TAY => {
                        self.tay();
                    }
                    Mnemonic::TSX => {
                        self.tsx();
                    }
                    Mnemonic::TXA => {
                        self.txa();
                    }
                    Mnemonic::TXS => {
                        self.txs();
                    }
                    Mnemonic::TYA => {
                        self.tya();
                    }
                    Mnemonic::AHX => {
                        self.ahx(&opcode_info.mode);
                    }
                    Mnemonic::ALR => {
                        self.alr(&opcode_info.mode);
                    }
                    Mnemonic::ANC => {
                        self.anc(&opcode_info.mode);
                    }
                    Mnemonic::ARR => {
                        self.arr(&opcode_info.mode);
                    }
                    Mnemonic::AXS => {
                        self.axs(&opcode_info.mode);
                    }
                    Mnemonic::DCP => {
                        self.dcp(&opcode_info.mode);
                    }
                    Mnemonic::ISC => {
                        self.isc(&opcode_info.mode);
                    }
                    Mnemonic::LAS => {
                        self.las(&opcode_info.mode);
                    }
                    Mnemonic::LAX => {
                        self.lax(&opcode_info.mode);
                    }
                    Mnemonic::LXA => {
                        self.lxa(&opcode_info.mode);
                    }
                    Mnemonic::RLA => {
                        self.rla(&opcode_info.mode);
                    }
                    Mnemonic::RRA => {
                        self.rra(&opcode_info.mode);
                    }
                    Mnemonic::SAX => {
                        self.sax(&opcode_info.mode);
                    }
                    Mnemonic::SHX => {
                        self.shx(&opcode_info.mode);
                    }
                    Mnemonic::SHY => {
                        self.shy(&opcode_info.mode);
                    }
                    Mnemonic::SLO => {
                        self.slo(&opcode_info.mode);
                    }
                    Mnemonic::SRE => {
                        self.sre(&opcode_info.mode);
                    }
                    Mnemonic::STP => {
                        self.stp();
                    }
                    Mnemonic::TAS => {
                        self.tas(&opcode_info.mode);
                    }
                    Mnemonic::XAA => {
                        self.xaa(&opcode_info.mode);
                    }
                }

                self.memory.tick(opcode_info.cycles + self.extra_cycles);
                self.extra_cycles = 0;

                if pc_state == self.registers.pc {
                    self.registers.pc += opcode_info.bytes as u16 - 1;
                }
            } else {
                println!("Unknown opcode: {:02X}", opcode);
                break;
            }
        }
    }

    pub fn reset(&mut self) {
        self.registers.a = 0;
        self.registers.x = 0;
        self.registers.y = 0;

        self.registers.status = StatusFlags::empty();
        self.registers.status.insert(StatusFlags::INTERRUPT_DISABLE);

        self.registers.sp = 0xFD;

        self.registers.pc = self.memory.read_u16(0xFFFC);
        self.halted = false;
    }

    pub fn load(&mut self, program: Vec<u8>, start_addr: Option<u16>) {
        let load_addr = start_addr.unwrap_or(PRG_START);
        self.memory.load(load_addr, &program);
    }

    #[allow(dead_code)]
    pub fn load_and_run(&mut self, program: Vec<u8>, start_addr: Option<u16>) {
        self.load(program, start_addr);
        self.reset();
        self.run();
    }
}

/// Instructions
impl<M: Memory> CPU<M> {
    fn adc(&mut self, mode: &AddressingMode) {
        let (addr, page_cross) = self.get_operand_address(mode);
        if page_cross {
            self.extra_cycles += 1;
        }
        let value = self.memory.read(addr);

        self.adc_value(value);
    }

    fn adc_value(&mut self, value: u8) {
        let sum = self.registers.a as u16
            + value as u16
            + if self.registers.status.contains(StatusFlags::CARRY) {
                1
            } else {
                0
            };

        self.registers.status.set(StatusFlags::CARRY, sum > 0xFF);

        let result = sum as u8;

        self.registers.status.set(
            StatusFlags::OVERFLOW,
            ((self.registers.a ^ result) & (value ^ result) & 0x80) != 0,
        );

        self.registers.a = result;
        self.update_zero_and_negative_flags(self.registers.a);
    }

    fn and(&mut self, mode: &AddressingMode) {
        let (addr, page_cross) = self.get_operand_address(mode);
        if page_cross {
            self.extra_cycles += 1;
        }
        let value = self.memory.read(addr);

        self.registers.a &= value;
        self.update_zero_and_negative_flags(self.registers.a);
    }

    fn asl(&mut self, mode: &AddressingMode) {
        if *mode == AddressingMode::Accumulator {
            let mut value = self.registers.a;

            self.registers
                .status
                .set(StatusFlags::CARRY, value & 0b1000_0000 != 0);

            value <<= 1;
            self.registers.a = value;
            self.update_zero_and_negative_flags(self.registers.a);
            return;
        }

        let (addr, _) = self.get_operand_address(mode);
        let mut value = self.memory.read(addr);

        self.registers
            .status
            .set(StatusFlags::CARRY, value & 0b1000_0000 != 0);

        value <<= 1;
        self.memory.write(addr, value);
        self.update_zero_and_negative_flags(value);
    }

    fn bcc(&mut self, mode: &AddressingMode) {
        let base_pc = self.registers.pc + 1;
        let (addr, _) = self.get_operand_address(mode);
        if !self.registers.status.contains(StatusFlags::CARRY) {
            self.registers.pc = addr;
            self.extra_cycles += 1;
            if (base_pc & 0xFF00) != (addr & 0xFF00) {
                self.extra_cycles += 2;
            }
        }
    }

    fn bcs(&mut self, mode: &AddressingMode) {
        let base_pc = self.registers.pc + 1;
        let (addr, _) = self.get_operand_address(mode);
        if self.registers.status.contains(StatusFlags::CARRY) {
            self.registers.pc = addr;
            self.extra_cycles += 1;
            if (base_pc & 0xFF00) != (addr & 0xFF00) {
                self.extra_cycles += 2;
            }
        }
    }

    fn beq(&mut self, mode: &AddressingMode) {
        let base_pc = self.registers.pc + 1;
        let (addr, _) = self.get_operand_address(mode);
        if self.registers.status.contains(StatusFlags::ZERO) {
            self.registers.pc = addr;
            self.extra_cycles += 1;
            if (base_pc & 0xFF00) != (addr & 0xFF00) {
                self.extra_cycles += 2;
            }
        }
    }

    fn bit(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(mode);
        let value = self.memory.read(addr);

        let result = self.registers.a & value;

        if result == 0 {
            self.registers.status.insert(StatusFlags::ZERO); // Set zero flag
        } else {
            self.registers.status.remove(StatusFlags::ZERO); // Clear zero flag
        }

        if value & 0b0100_0000 != 0 {
            self.registers.status.insert(StatusFlags::OVERFLOW); // Set overflow flag
        } else {
            self.registers.status.remove(StatusFlags::OVERFLOW); // Clear overflow flag
        }

        if value & 0b1000_0000 != 0 {
            self.registers.status.insert(StatusFlags::NEGATIVE); // Set negative flag
        } else {
            self.registers.status.remove(StatusFlags::NEGATIVE); // Clear negative flag
        }
    }

    fn bmi(&mut self, mode: &AddressingMode) {
        let base_pc = self.registers.pc + 1;
        let (addr, _) = self.get_operand_address(mode);
        if self.registers.status.contains(StatusFlags::NEGATIVE) {
            self.registers.pc = addr;
            self.extra_cycles += 1;
            if (base_pc & 0xFF00) != (addr & 0xFF00) {
                self.extra_cycles += 2;
            }
        }
    }

    fn bne(&mut self, mode: &AddressingMode) {
        let base_pc = self.registers.pc + 1;
        let (addr, _) = self.get_operand_address(mode);
        if !self.registers.status.contains(StatusFlags::ZERO) {
            self.registers.pc = addr;
            self.extra_cycles += 1;
            if (base_pc & 0xFF00) != (addr & 0xFF00) {
                self.extra_cycles += 2;
            }
        }
    }

    fn bpl(&mut self, mode: &AddressingMode) {
        let base_pc = self.registers.pc + 1;
        let (addr, _) = self.get_operand_address(mode);
        if !self.registers.status.contains(StatusFlags::NEGATIVE) {
            self.registers.pc = addr;
            self.extra_cycles += 1;
            if (base_pc & 0xFF00) != (addr & 0xFF00) {
                self.extra_cycles += 2;
            }
        }
    }

    fn brk(&mut self, _mode: &AddressingMode) {
        self.registers.pc += 1;
        if !self
            .registers
            .status
            .contains(StatusFlags::INTERRUPT_DISABLE)
        {
            self.interrupt(interrupt::BRK);
        }
    }

    fn bvc(&mut self, mode: &AddressingMode) {
        let base_pc = self.registers.pc + 1;
        let (addr, _) = self.get_operand_address(mode);
        if !self.registers.status.contains(StatusFlags::OVERFLOW) {
            self.registers.pc = addr;
            self.extra_cycles += 1;
            if (base_pc & 0xFF00) != (addr & 0xFF00) {
                self.extra_cycles += 2;
            }
        }
    }

    fn bvs(&mut self, mode: &AddressingMode) {
        let base_pc = self.registers.pc + 1;
        let (addr, _) = self.get_operand_address(mode);
        if self.registers.status.contains(StatusFlags::OVERFLOW) {
            self.registers.pc = addr;
            self.extra_cycles += 1;
            if (base_pc & 0xFF00) != (addr & 0xFF00) {
                self.extra_cycles += 2;
            }
        }
    }

    fn clc(&mut self) {
        self.registers.status.remove(StatusFlags::CARRY); // Clear carry flag
    }

    fn cld(&mut self) {
        self.registers.status.remove(StatusFlags::DECIMAL_MODE); // Clear decimal flag
    }

    fn cli(&mut self) {
        self.registers.status.remove(StatusFlags::INTERRUPT_DISABLE); // Clear interrupt disable flag
    }

    fn clv(&mut self) {
        self.registers.status.remove(StatusFlags::OVERFLOW); // Clear overflow flag
    }

    fn cmp(&mut self, mode: &AddressingMode) {
        let (addr, page_cross) = self.get_operand_address(mode);
        if page_cross {
            self.extra_cycles += 1;
        }
        let value = self.memory.read(addr);

        let result = self.registers.a.wrapping_sub(value);

        if self.registers.a >= value {
            self.registers.status.insert(StatusFlags::CARRY); // Set carry flag
        } else {
            self.registers.status.remove(StatusFlags::CARRY); // Clear carry flag
        }

        self.update_zero_and_negative_flags(result);
    }

    fn cpx(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(mode);
        let value = self.memory.read(addr);

        let result = self.registers.x.wrapping_sub(value);

        if self.registers.x >= value {
            self.registers.status.insert(StatusFlags::CARRY); // Set carry flag
        } else {
            self.registers.status.remove(StatusFlags::CARRY); // Clear carry flag
        }

        self.update_zero_and_negative_flags(result);
    }

    fn cpy(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(mode);
        let value = self.memory.read(addr);

        let result = self.registers.y.wrapping_sub(value);

        if self.registers.y >= value {
            self.registers.status.insert(StatusFlags::CARRY); // Set carry flag
        } else {
            self.registers.status.remove(StatusFlags::CARRY); // Clear carry flag
        }

        self.update_zero_and_negative_flags(result);
    }

    fn dec(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(mode);
        let mut value = self.memory.read(addr);

        value = value.wrapping_sub(1);
        self.memory.write(addr, value);
        self.update_zero_and_negative_flags(value);
    }

    fn dex(&mut self) {
        self.registers.x = self.registers.x.wrapping_sub(1);
        self.update_zero_and_negative_flags(self.registers.x);
    }

    fn dey(&mut self) {
        self.registers.y = self.registers.y.wrapping_sub(1);
        self.update_zero_and_negative_flags(self.registers.y);
    }

    fn eor(&mut self, mode: &AddressingMode) {
        let (addr, page_cross) = self.get_operand_address(mode);
        if page_cross {
            self.extra_cycles += 1;
        }
        let value = self.memory.read(addr);

        self.registers.a ^= value;
        self.update_zero_and_negative_flags(self.registers.a);
    }

    fn inc(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(mode);
        let mut value = self.memory.read(addr);

        value = value.wrapping_add(1);
        self.memory.write(addr, value);
        self.update_zero_and_negative_flags(value);
    }

    fn inx(&mut self) {
        self.registers.x = self.registers.x.wrapping_add(1);
        self.update_zero_and_negative_flags(self.registers.x);
    }

    fn iny(&mut self) {
        self.registers.y = self.registers.y.wrapping_add(1);
        self.update_zero_and_negative_flags(self.registers.y);
    }

    fn jmp(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(mode);
        self.registers.pc = addr;
        // println!("JMP to {:04X}", self.registers.pc);
    }

    fn jsr(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(mode);

        let return_addr = self.registers.pc.wrapping_add(2).wrapping_sub(1);

        self.push_stack_u16(return_addr);

        self.registers.pc = addr;
        // println!(
        //     "JSR to {:04X}, stack: {:02X}",
        //     self.registers.pc, self.registers.sp
        // );
    }

    fn lda(&mut self, mode: &AddressingMode) {
        let (addr, page_cross) = self.get_operand_address(mode);
        if page_cross {
            self.extra_cycles += 1;
        }
        let value = self.memory.read(addr);

        self.registers.a = value;
        self.update_zero_and_negative_flags(self.registers.a);
    }

    fn ldx(&mut self, mode: &AddressingMode) {
        let (addr, page_cross) = self.get_operand_address(mode);
        if page_cross {
            self.extra_cycles += 1;
        }
        let value = self.memory.read(addr);

        self.registers.x = value;
        self.update_zero_and_negative_flags(self.registers.x);
    }

    fn ldy(&mut self, mode: &AddressingMode) {
        let (addr, page_cross) = self.get_operand_address(mode);
        if page_cross {
            self.extra_cycles += 1;
        }
        let value = self.memory.read(addr);

        self.registers.y = value;
        self.update_zero_and_negative_flags(self.registers.y);
    }

    fn lsr(&mut self, mode: &AddressingMode) {
        if *mode == AddressingMode::Accumulator {
            let mut value = self.registers.a;

            if value & 0b0000_0001 != 0 {
                self.registers.status.insert(StatusFlags::CARRY); // Set carry flag
            } else {
                self.registers.status.remove(StatusFlags::CARRY); // Clear carry flag
            }

            value >>= 1;
            self.registers.a = value;
            self.update_zero_and_negative_flags(self.registers.a);
            return;
        }

        let (addr, _) = self.get_operand_address(mode);
        let mut value = self.memory.read(addr);

        if value & 0b0000_0001 != 0 {
            self.registers.status.insert(StatusFlags::CARRY); // Set carry flag
        } else {
            self.registers.status.remove(StatusFlags::CARRY); // Clear carry flag
        }

        value >>= 1;
        self.memory.write(addr, value);
        self.update_zero_and_negative_flags(value);
    }

    fn nop(&mut self, mode: &AddressingMode) {
        if matches!(mode, AddressingMode::None | AddressingMode::Accumulator) {
            return;
        }
        let (addr, page_cross) = self.get_operand_address(mode);
        self.memory.read(addr);
        if page_cross {
            self.extra_cycles += 1;
        }
    }

    fn ora(&mut self, mode: &AddressingMode) {
        let (addr, page_cross) = self.get_operand_address(mode);
        if page_cross {
            self.extra_cycles += 1;
        }
        let value = self.memory.read(addr);

        self.registers.a |= value;
        self.update_zero_and_negative_flags(self.registers.a);
    }

    fn pha(&mut self) {
        let sp_addr = self.stack_addr();
        self.memory.write(sp_addr, self.registers.a);
        self.registers.sp = self.registers.sp.wrapping_sub(1);
    }

    fn php(&mut self) {
        let mut flags = StatusFlags::from_bits_truncate(self.registers.status.bits());
        flags.insert(StatusFlags::BREAK_COMMAND);
        flags.insert(StatusFlags::UNUSED);
        self.push_stack(flags.bits());
    }

    fn pla(&mut self) {
        self.registers.a = self.pull_stack();
        self.update_zero_and_negative_flags(self.registers.a);
    }

    fn plp(&mut self) {
        self.registers.sp = self.registers.sp.wrapping_add(1);
        let sp_addr = self.stack_addr();
        self.registers.status = StatusFlags::from_bits_truncate(self.memory.read(sp_addr));
        self.registers.status.remove(StatusFlags::BREAK_COMMAND);
        self.registers.status.insert(StatusFlags::UNUSED);
    }

    fn rol(&mut self, mode: &AddressingMode) {
        if *mode == AddressingMode::Accumulator {
            let mut value = self.registers.a;
            let carry_in = if self.registers.status.contains(StatusFlags::CARRY) {
                1
            } else {
                0
            };

            self.registers
                .status
                .set(StatusFlags::CARRY, value & 0b1000_0000 != 0);

            value = (value << 1) | carry_in;
            self.registers.a = value;
            self.update_zero_and_negative_flags(self.registers.a);
            return;
        }

        let (addr, _) = self.get_operand_address(mode);
        let mut value = self.memory.read(addr);
        let carry_in = if self.registers.status.contains(StatusFlags::CARRY) {
            1
        } else {
            0
        };

        self.registers
            .status
            .set(StatusFlags::CARRY, value & 0b1000_0000 != 0);

        value = (value << 1) | carry_in;
        self.memory.write(addr, value);
        self.update_zero_and_negative_flags(value);
    }

    fn ror(&mut self, mode: &AddressingMode) {
        if *mode == AddressingMode::Accumulator {
            let mut value = self.registers.a;
            let carry_in = if self.registers.status.contains(StatusFlags::CARRY) {
                0b1000_0000
            } else {
                0
            };

            self.registers
                .status
                .set(StatusFlags::CARRY, value & 0b0000_0001 != 0);

            value = (value >> 1) | carry_in;
            self.registers.a = value;
            self.update_zero_and_negative_flags(self.registers.a);
            return;
        }

        let (addr, _) = self.get_operand_address(mode);
        let mut value = self.memory.read(addr);
        let carry_in = if self.registers.status.contains(StatusFlags::CARRY) {
            0b1000_0000
        } else {
            0
        };

        self.registers
            .status
            .set(StatusFlags::CARRY, value & 0b0000_0001 != 0);

        value = (value >> 1) | carry_in;
        self.memory.write(addr, value);
        self.update_zero_and_negative_flags(value);
    }

    fn rti(&mut self) {
        let status = self.pull_stack();
        self.registers.status = StatusFlags::from_bits_truncate(status);
        self.registers.status.remove(StatusFlags::BREAK_COMMAND);
        self.registers.status.insert(StatusFlags::UNUSED);

        self.registers.pc = self.pull_stack_u16();
    }

    fn rts(&mut self) {
        let addr = self.pull_stack_u16();
        self.registers.pc = addr.wrapping_add(1);
    }

    fn sbc(&mut self, mode: &AddressingMode) {
        let (addr, page_cross) = self.get_operand_address(mode);
        if page_cross {
            self.extra_cycles += 1;
        }
        let value = self.memory.read(addr);

        self.sbc_value(value);
    }

    fn sbc_value(&mut self, value: u8) {
        let carry = if self.registers.status.contains(StatusFlags::CARRY) {
            0
        } else {
            1
        };

        let diff = self.registers.a as i16 - value as i16 - carry as i16;

        self.registers.status.set(StatusFlags::CARRY, diff >= 0);

        let result = diff as u8;

        // Set overflow flag
        if ((self.registers.a ^ result) & (!(value) ^ result) & 0x80) != 0 {
            self.registers.status.insert(StatusFlags::OVERFLOW); // Set overflow flag
        } else {
            self.registers.status.remove(StatusFlags::OVERFLOW); // Clear overflow flag
        }

        self.registers.a = result;
        self.update_zero_and_negative_flags(self.registers.a);
    }

    fn sec(&mut self) {
        self.registers.status.insert(StatusFlags::CARRY); // Set carry flag
    }

    fn sed(&mut self) {
        self.registers.status.insert(StatusFlags::DECIMAL_MODE); // Set decimal flag
    }

    fn sei(&mut self) {
        self.registers.status.insert(StatusFlags::INTERRUPT_DISABLE); // Set interrupt disable flag
    }

    fn sta(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(mode);
        self.memory.write(addr, self.registers.a);
    }

    fn stx(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(mode);
        self.memory.write(addr, self.registers.x);
    }

    fn sty(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(mode);
        self.memory.write(addr, self.registers.y);
    }

    fn tax(&mut self) {
        self.registers.x = self.registers.a;
        self.update_zero_and_negative_flags(self.registers.x);
    }

    fn tay(&mut self) {
        self.registers.y = self.registers.a;
        self.update_zero_and_negative_flags(self.registers.y);
    }

    fn tsx(&mut self) {
        self.registers.x = self.registers.sp;
        self.update_zero_and_negative_flags(self.registers.x);
    }

    fn txa(&mut self) {
        self.registers.a = self.registers.x;
        self.update_zero_and_negative_flags(self.registers.a);
    }

    fn txs(&mut self) {
        self.registers.sp = self.registers.x;
    }

    fn tya(&mut self) {
        self.registers.a = self.registers.y;
        self.update_zero_and_negative_flags(self.registers.a);
    }

    // Unofficial instructions
    fn anc(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(mode);
        let value = self.memory.read(addr);
        self.registers.a &= value;
        self.update_zero_and_negative_flags(self.registers.a);
        self.registers
            .status
            .set(StatusFlags::CARRY, (self.registers.a & 0x80) != 0);
    }

    fn alr(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(mode);
        let value = self.memory.read(addr);
        let mut result = self.registers.a & value;
        self.registers
            .status
            .set(StatusFlags::CARRY, (result & 0x01) != 0);
        result >>= 1;
        self.registers.a = result;
        self.update_zero_and_negative_flags(result);
    }

    fn arr(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(mode);
        let value = self.memory.read(addr);
        let mut result = self.registers.a & value;
        let carry_in = if self.registers.status.contains(StatusFlags::CARRY) {
            0x80
        } else {
            0
        };
        result = (result >> 1) | carry_in;
        self.registers.a = result;
        self.update_zero_and_negative_flags(result);
        self.registers
            .status
            .set(StatusFlags::CARRY, (result & 0x40) != 0);
        let bit6 = (result >> 6) & 1;
        let bit5 = (result >> 5) & 1;
        self.registers
            .status
            .set(StatusFlags::OVERFLOW, (bit6 ^ bit5) != 0);
    }

    fn axs(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(mode);
        let value = self.memory.read(addr);
        let masked = self.registers.a & self.registers.x;
        let result = masked.wrapping_sub(value);
        self.registers
            .status
            .set(StatusFlags::CARRY, masked >= value);
        self.registers.x = result;
        self.update_zero_and_negative_flags(result);
    }

    fn slo(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(mode);
        let mut value = self.memory.read(addr);
        self.registers
            .status
            .set(StatusFlags::CARRY, (value & 0x80) != 0);
        value <<= 1;
        self.memory.write(addr, value);
        self.registers.a |= value;
        self.update_zero_and_negative_flags(self.registers.a);
    }

    fn rla(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(mode);
        let mut value = self.memory.read(addr);
        let carry_in = if self.registers.status.contains(StatusFlags::CARRY) {
            1
        } else {
            0
        };
        self.registers
            .status
            .set(StatusFlags::CARRY, (value & 0x80) != 0);
        value = (value << 1) | carry_in;
        self.memory.write(addr, value);
        self.registers.a &= value;
        self.update_zero_and_negative_flags(self.registers.a);
    }

    fn sre(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(mode);
        let mut value = self.memory.read(addr);
        self.registers
            .status
            .set(StatusFlags::CARRY, (value & 0x01) != 0);
        value >>= 1;
        self.memory.write(addr, value);
        self.registers.a ^= value;
        self.update_zero_and_negative_flags(self.registers.a);
    }

    fn rra(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(mode);
        let mut value = self.memory.read(addr);
        let carry_in = if self.registers.status.contains(StatusFlags::CARRY) {
            0x80
        } else {
            0
        };
        self.registers
            .status
            .set(StatusFlags::CARRY, (value & 0x01) != 0);
        value = (value >> 1) | carry_in;
        self.memory.write(addr, value);
        self.adc_value(value);
    }

    fn dcp(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(mode);
        let value = self.memory.read(addr).wrapping_sub(1);
        self.memory.write(addr, value);
        self.registers
            .status
            .set(StatusFlags::CARRY, self.registers.a >= value);
        let result = self.registers.a.wrapping_sub(value);
        self.update_zero_and_negative_flags(result);
    }

    fn isc(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(mode);
        let value = self.memory.read(addr).wrapping_add(1);
        self.memory.write(addr, value);
        self.sbc_value(value);
    }

    fn sax(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(mode);
        let value = self.registers.a & self.registers.x;
        self.memory.write(addr, value);
    }

    fn lax(&mut self, mode: &AddressingMode) {
        let (addr, page_cross) = self.get_operand_address(mode);
        if page_cross {
            self.extra_cycles += 1;
        }
        let value = self.memory.read(addr);
        self.registers.a = value;
        self.registers.x = value;
        self.update_zero_and_negative_flags(value);
    }

    fn lxa(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(mode);
        let value = self.memory.read(addr);
        let result = (self.registers.a | 0xEE) & value;
        self.registers.a = result;
        self.registers.x = result;
        self.update_zero_and_negative_flags(result);
    }

    fn las(&mut self, mode: &AddressingMode) {
        let (addr, page_cross) = self.get_operand_address(mode);
        if page_cross {
            self.extra_cycles += 1;
        }
        let value = self.memory.read(addr) & self.registers.sp;
        self.registers.sp = value;
        self.registers.a = value;
        self.registers.x = value;
        self.update_zero_and_negative_flags(value);
    }

    fn ahx(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(mode);
        let high = ((addr >> 8) as u8).wrapping_add(1);
        let value = self.registers.a & self.registers.x & high;
        self.memory.write(addr, value);
    }

    fn shy(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(mode);
        let high = ((addr >> 8) as u8).wrapping_add(1);
        let value = self.registers.y & high;
        self.memory.write(addr, value);
    }

    fn shx(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(mode);
        let high = ((addr >> 8) as u8).wrapping_add(1);
        let value = self.registers.x & high;
        self.memory.write(addr, value);
    }

    fn tas(&mut self, mode: &AddressingMode) {
        let mut masked = self.registers.a & self.registers.x;
        self.registers.sp = masked;
        let (addr, _) = self.get_operand_address(mode);
        let high = ((addr >> 8) as u8).wrapping_add(1);
        masked &= high;
        self.memory.write(addr, masked);
    }

    fn xaa(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(mode);
        let value = self.memory.read(addr);
        let result = (self.registers.x & (self.registers.a | 0xEE)) & value;
        self.registers.a = result;
        self.registers.x = result;
        self.update_zero_and_negative_flags(result);
    }

    fn stp(&mut self) {
        self.halted = true;
    }

    fn update_zero_and_negative_flags(&mut self, value: u8) {
        if value == 0 {
            self.registers.status.insert(StatusFlags::ZERO); // Set zero flag
        } else {
            self.registers.status.remove(StatusFlags::ZERO); // Clear zero flag
        }

        if value & 0b1000_0000 != 0 {
            self.registers.status.insert(StatusFlags::NEGATIVE); // Set negative flag
        } else {
            self.registers.status.remove(StatusFlags::NEGATIVE); // Clear negative flag
        }
    }

    pub fn get_operand_address(&mut self, mode: &AddressingMode) -> (u16, bool) {
        match mode {
            AddressingMode::Immediate => (self.registers.pc, false),

            AddressingMode::ZeroPage => (self.memory.read(self.registers.pc) as u16, false),

            AddressingMode::Absolute => (self.memory.read_u16(self.registers.pc), false),

            AddressingMode::ZeroPageX => {
                let pos = self.memory.read(self.registers.pc);
                let addr = pos.wrapping_add(self.registers.x) as u16;
                (addr, false)
            }
            AddressingMode::ZeroPageY => {
                let pos = self.memory.read(self.registers.pc);
                let addr = pos.wrapping_add(self.registers.y) as u16;
                (addr, false)
            }

            AddressingMode::Relative => {
                let offset = self.memory.read(self.registers.pc) as i8;
                let next = self.registers.pc.wrapping_add(1) as i32;
                ((next + offset as i32) as u16, false)
            }

            AddressingMode::AbsoluteX => {
                let base = self.memory.read_u16(self.registers.pc);
                let addr = base.wrapping_add(self.registers.x as u16);
                let page_cross = (base & 0xFF00) != (addr & 0xFF00);
                (addr, page_cross)
            }
            AddressingMode::AbsoluteY => {
                let base = self.memory.read_u16(self.registers.pc);
                let addr = base.wrapping_add(self.registers.y as u16);
                let page_cross = (base & 0xFF00) != (addr & 0xFF00);
                (addr, page_cross)
            }

            AddressingMode::Indirect => {
                let addr = self.memory.read_u16(self.registers.pc);

                let indirect_ref = if addr & 0x00FF == 0x00FF {
                    let lo = self.memory.read(addr);
                    let hi = self.memory.read(addr & 0xFF00);
                    (hi as u16) << 8 | (lo as u16)
                } else {
                    self.memory.read_u16(addr)
                };
                (indirect_ref, false)
            }
            AddressingMode::IndirectX => {
                let base = self.memory.read(self.registers.pc);

                let ptr: u8 = base.wrapping_add(self.registers.x);
                let lo = self.memory.read(ptr as u16);
                let hi = self.memory.read(ptr.wrapping_add(1) as u16);
                ((hi as u16) << 8 | (lo as u16), false)
            }
            AddressingMode::IndirectY => {
                let base = self.memory.read(self.registers.pc);

                let lo = self.memory.read(base as u16);
                let hi = self.memory.read(base.wrapping_add(1) as u16);
                let deref_base = (hi as u16) << 8 | (lo as u16);
                let deref = deref_base.wrapping_add(self.registers.y as u16);
                let page_cross = (deref_base & 0xFF00) != (deref & 0xFF00);
                (deref, page_cross)
            }

            AddressingMode::None | AddressingMode::Accumulator => {
                // dummy read
                (self.memory.read(self.registers.pc + 1) as u16, false)
            }
        }
    }
}

/// Helpers
impl<M: Memory> CPU<M> {
    fn stack_addr(&self) -> u16 {
        STACK_START + self.registers.sp as u16
    }

    fn push_stack(&mut self, v: u8) {
        let addr = self.stack_addr();
        self.memory.write(addr, v);
        self.registers.sp = self.registers.sp.wrapping_sub(1);
    }

    fn pull_stack(&mut self) -> u8 {
        self.registers.sp = self.registers.sp.wrapping_add(1);
        let addr = self.stack_addr();
        self.memory.read(addr)
    }

    fn push_stack_u16(&mut self, v: u16) {
        // push high then low
        self.push_stack((v >> 8) as u8);
        self.push_stack((v & 0xFF) as u8);
    }

    fn pull_stack_u16(&mut self) -> u16 {
        let lo = self.pull_stack() as u16;
        let hi = self.pull_stack() as u16;
        (hi << 8) | lo
    }

    fn interrupt(&mut self, interrupt: interrupt::Interrupt) {
        self.push_stack_u16(self.registers.pc);
        let mut flag = StatusFlags::from_bits_truncate(self.registers.status.bits());
        flag.remove(StatusFlags::BREAK_COMMAND);
        flag.insert(StatusFlags::UNUSED);

        self.push_stack(flag.bits());
        self.registers.status.insert(StatusFlags::INTERRUPT_DISABLE);

        self.memory.tick(interrupt.cpu_cycles);
        self.registers.pc = self.memory.read_u16(interrupt.vector_addr);
    }
}
