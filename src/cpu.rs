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

pub struct CPU {
    pub registers: Registers,
    extra_cycles: u8,
    cycles_wait: u8,
    halted: bool,
}

impl CPU {
    pub fn new() -> Self {
        CPU {
            registers: Registers {
                a: 0,
                x: 0,
                y: 0,
                status: StatusFlags::from_bits_truncate(0b00100100),
                pc: PRG_START,
                sp: 0xFD,
            },
            extra_cycles: 0,
            cycles_wait: 0,
            halted: false,
        }
    }

    pub fn clock<M: Memory>(&mut self, memory: &mut M) -> bool {
        if self.halted {
            return false;
        }

        if self.cycles_wait == 0 {
            let opcode = memory.read(self.registers.pc);
            self.registers.pc = self.registers.pc.wrapping_add(1);

            if let Some(opcode_info) = CPU_OPCODES.find_by_code(opcode) {
                self.extra_cycles = 0;
                self.execute_instruction(
                    memory,
                    opcode_info.bytes,
                    &opcode_info.mnemonic,
                    &opcode_info.mode,
                );
                self.cycles_wait = opcode_info.cycles + self.extra_cycles;
                self.extra_cycles = 0;
            } else {
                panic!("Unknown opcode: {opcode:#04X}");
            }
        }

        if self.cycles_wait > 0 {
            self.cycles_wait -= 1;
        }

        self.cycles_wait == 0
    }

    pub fn nmi<M: Memory>(&mut self, memory: &mut M) {
        self.interrupt(memory, interrupt::NMI);
    }

    pub fn irq<M: Memory>(&mut self, memory: &mut M) {
        if !self
            .registers
            .status
            .contains(StatusFlags::INTERRUPT_DISABLE)
        {
            self.interrupt(memory, interrupt::IRQ);
        }
    }

    fn execute_instruction<M: Memory>(
        &mut self,
        memory: &mut M,
        bytes: u8,
        mnemonic: &Mnemonic,
        mode: &AddressingMode,
    ) {
        let pc_state = self.registers.pc;

        match mnemonic {
            Mnemonic::ADC => self.adc(memory, mode),
            Mnemonic::AND => self.and(memory, mode),
            Mnemonic::ASL => self.asl(memory, mode),
            Mnemonic::BCC => self.bcc(memory, mode),
            Mnemonic::BCS => self.bcs(memory, mode),
            Mnemonic::BEQ => self.beq(memory, mode),
            Mnemonic::BIT => self.bit(memory, mode),
            Mnemonic::BMI => self.bmi(memory, mode),
            Mnemonic::BNE => self.bne(memory, mode),
            Mnemonic::BPL => self.bpl(memory, mode),
            Mnemonic::BRK => self.brk(memory, mode),
            Mnemonic::BVC => self.bvc(memory, mode),
            Mnemonic::BVS => self.bvs(memory, mode),
            Mnemonic::CLC => self.clc(),
            Mnemonic::CLD => self.cld(),
            Mnemonic::CLI => self.cli(),
            Mnemonic::CLV => self.clv(),
            Mnemonic::CMP => self.cmp(memory, mode),
            Mnemonic::CPX => self.cpx(memory, mode),
            Mnemonic::CPY => self.cpy(memory, mode),
            Mnemonic::DEC => self.dec(memory, mode),
            Mnemonic::DEX => self.dex(),
            Mnemonic::DEY => self.dey(),
            Mnemonic::EOR => self.eor(memory, mode),
            Mnemonic::INC => self.inc(memory, mode),
            Mnemonic::INX => self.inx(),
            Mnemonic::INY => self.iny(),
            Mnemonic::JMP => self.jmp(memory, mode),
            Mnemonic::JSR => self.jsr(memory, mode),
            Mnemonic::LDA => self.lda(memory, mode),
            Mnemonic::LDX => self.ldx(memory, mode),
            Mnemonic::LDY => self.ldy(memory, mode),
            Mnemonic::LSR => self.lsr(memory, mode),
            Mnemonic::NOP => self.nop(memory, mode),
            Mnemonic::ORA => self.ora(memory, mode),
            Mnemonic::PHA => self.pha(memory),
            Mnemonic::PHP => self.php(memory),
            Mnemonic::PLA => self.pla(memory),
            Mnemonic::PLP => self.plp(memory),
            Mnemonic::ROL => self.rol(memory, mode),
            Mnemonic::ROR => self.ror(memory, mode),
            Mnemonic::RTI => self.rti(memory),
            Mnemonic::RTS => self.rts(memory),
            Mnemonic::SBC => self.sbc(memory, mode),
            Mnemonic::SEC => self.sec(),
            Mnemonic::SED => self.sed(),
            Mnemonic::SEI => self.sei(),
            Mnemonic::STA => self.sta(memory, mode),
            Mnemonic::STX => self.stx(memory, mode),
            Mnemonic::STY => self.sty(memory, mode),
            Mnemonic::TAX => self.tax(),
            Mnemonic::TAY => self.tay(),
            Mnemonic::TSX => self.tsx(),
            Mnemonic::TXA => self.txa(),
            Mnemonic::TXS => self.txs(),
            Mnemonic::TYA => self.tya(),
            Mnemonic::AHX => self.ahx(memory, mode),
            Mnemonic::ALR => self.alr(memory, mode),
            Mnemonic::ANC => self.anc(memory, mode),
            Mnemonic::ARR => self.arr(memory, mode),
            Mnemonic::AXS => self.axs(memory, mode),
            Mnemonic::DCP => self.dcp(memory, mode),
            Mnemonic::ISC => self.isc(memory, mode),
            Mnemonic::LAS => self.las(memory, mode),
            Mnemonic::LAX => self.lax(memory, mode),
            Mnemonic::LXA => self.lxa(memory, mode),
            Mnemonic::RLA => self.rla(memory, mode),
            Mnemonic::RRA => self.rra(memory, mode),
            Mnemonic::SAX => self.sax(memory, mode),
            Mnemonic::SHX => self.shx(memory, mode),
            Mnemonic::SHY => self.shy(memory, mode),
            Mnemonic::SLO => self.slo(memory, mode),
            Mnemonic::SRE => self.sre(memory, mode),
            Mnemonic::STP => self.stp(),
            Mnemonic::TAS => self.tas(memory, mode),
            Mnemonic::XAA => self.xaa(memory, mode),
        }

        if pc_state == self.registers.pc {
            self.registers.pc = self
                .registers
                .pc
                .wrapping_add(bytes.saturating_sub(1) as u16);
        }
    }

    pub fn reset<M: Memory>(&mut self, memory: &mut M) {
        self.registers.a = 0;
        self.registers.x = 0;
        self.registers.y = 0;

        self.registers.status = StatusFlags::empty();
        self.registers.status.insert(StatusFlags::INTERRUPT_DISABLE);

        self.registers.sp = 0xFD;

        self.registers.pc = memory.read_u16(0xFFFC);
        self.halted = false;
    }
}

/// Instructions
impl CPU {
    fn adc<M: Memory>(&mut self, memory: &mut M, mode: &AddressingMode) {
        let (addr, page_cross) = self.get_operand_address(memory, mode);
        if page_cross {
            self.extra_cycles += 1;
        }
        let value = memory.read(addr);

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

    fn and<M: Memory>(&mut self, memory: &mut M, mode: &AddressingMode) {
        let (addr, page_cross) = self.get_operand_address(memory, mode);
        if page_cross {
            self.extra_cycles += 1;
        }
        let value = memory.read(addr);

        self.registers.a &= value;
        self.update_zero_and_negative_flags(self.registers.a);
    }

    fn asl<M: Memory>(&mut self, memory: &mut M, mode: &AddressingMode) {
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

        let (addr, _) = self.get_operand_address(memory, mode);
        let mut value = memory.read(addr);

        self.registers
            .status
            .set(StatusFlags::CARRY, value & 0b1000_0000 != 0);

        value <<= 1;
        memory.write(addr, value);
        self.update_zero_and_negative_flags(value);
    }

    fn bcc<M: Memory>(&mut self, memory: &mut M, mode: &AddressingMode) {
        let base_pc = self.registers.pc + 1;
        let (addr, _) = self.get_operand_address(memory, mode);
        if !self.registers.status.contains(StatusFlags::CARRY) {
            self.registers.pc = addr;
            self.extra_cycles += 1;
            if (base_pc & 0xFF00) != (addr & 0xFF00) {
                self.extra_cycles += 2;
            }
        }
    }

    fn bcs<M: Memory>(&mut self, memory: &mut M, mode: &AddressingMode) {
        let base_pc = self.registers.pc + 1;
        let (addr, _) = self.get_operand_address(memory, mode);
        if self.registers.status.contains(StatusFlags::CARRY) {
            self.registers.pc = addr;
            self.extra_cycles += 1;
            if (base_pc & 0xFF00) != (addr & 0xFF00) {
                self.extra_cycles += 2;
            }
        }
    }

    fn beq<M: Memory>(&mut self, memory: &mut M, mode: &AddressingMode) {
        let base_pc = self.registers.pc + 1;
        let (addr, _) = self.get_operand_address(memory, mode);
        if self.registers.status.contains(StatusFlags::ZERO) {
            self.registers.pc = addr;
            self.extra_cycles += 1;
            if (base_pc & 0xFF00) != (addr & 0xFF00) {
                self.extra_cycles += 2;
            }
        }
    }

    fn bit<M: Memory>(&mut self, memory: &mut M, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(memory, mode);
        let value = memory.read(addr);

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

    fn bmi<M: Memory>(&mut self, memory: &mut M, mode: &AddressingMode) {
        let base_pc = self.registers.pc + 1;
        let (addr, _) = self.get_operand_address(memory, mode);
        if self.registers.status.contains(StatusFlags::NEGATIVE) {
            self.registers.pc = addr;
            self.extra_cycles += 1;
            if (base_pc & 0xFF00) != (addr & 0xFF00) {
                self.extra_cycles += 2;
            }
        }
    }

    fn bne<M: Memory>(&mut self, memory: &mut M, mode: &AddressingMode) {
        let base_pc = self.registers.pc + 1;
        let (addr, _) = self.get_operand_address(memory, mode);
        if !self.registers.status.contains(StatusFlags::ZERO) {
            self.registers.pc = addr;
            self.extra_cycles += 1;
            if (base_pc & 0xFF00) != (addr & 0xFF00) {
                self.extra_cycles += 2;
            }
        }
    }

    fn bpl<M: Memory>(&mut self, memory: &mut M, mode: &AddressingMode) {
        let base_pc = self.registers.pc + 1;
        let (addr, _) = self.get_operand_address(memory, mode);
        if !self.registers.status.contains(StatusFlags::NEGATIVE) {
            self.registers.pc = addr;
            self.extra_cycles += 1;
            if (base_pc & 0xFF00) != (addr & 0xFF00) {
                self.extra_cycles += 2;
            }
        }
    }

    fn brk<M: Memory>(&mut self, memory: &mut M, _mode: &AddressingMode) {
        self.registers.pc += 1;
        if !self
            .registers
            .status
            .contains(StatusFlags::INTERRUPT_DISABLE)
        {
            self.interrupt(memory, interrupt::BRK);
        }
    }

    fn bvc<M: Memory>(&mut self, memory: &mut M, mode: &AddressingMode) {
        let base_pc = self.registers.pc + 1;
        let (addr, _) = self.get_operand_address(memory, mode);
        if !self.registers.status.contains(StatusFlags::OVERFLOW) {
            self.registers.pc = addr;
            self.extra_cycles += 1;
            if (base_pc & 0xFF00) != (addr & 0xFF00) {
                self.extra_cycles += 2;
            }
        }
    }

    fn bvs<M: Memory>(&mut self, memory: &mut M, mode: &AddressingMode) {
        let base_pc = self.registers.pc + 1;
        let (addr, _) = self.get_operand_address(memory, mode);
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

    fn cmp<M: Memory>(&mut self, memory: &mut M, mode: &AddressingMode) {
        let (addr, page_cross) = self.get_operand_address(memory, mode);
        if page_cross {
            self.extra_cycles += 1;
        }
        let value = memory.read(addr);

        let result = self.registers.a.wrapping_sub(value);

        if self.registers.a >= value {
            self.registers.status.insert(StatusFlags::CARRY); // Set carry flag
        } else {
            self.registers.status.remove(StatusFlags::CARRY); // Clear carry flag
        }

        self.update_zero_and_negative_flags(result);
    }

    fn cpx<M: Memory>(&mut self, memory: &mut M, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(memory, mode);
        let value = memory.read(addr);

        let result = self.registers.x.wrapping_sub(value);

        if self.registers.x >= value {
            self.registers.status.insert(StatusFlags::CARRY); // Set carry flag
        } else {
            self.registers.status.remove(StatusFlags::CARRY); // Clear carry flag
        }

        self.update_zero_and_negative_flags(result);
    }

    fn cpy<M: Memory>(&mut self, memory: &mut M, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(memory, mode);
        let value = memory.read(addr);

        let result = self.registers.y.wrapping_sub(value);

        if self.registers.y >= value {
            self.registers.status.insert(StatusFlags::CARRY); // Set carry flag
        } else {
            self.registers.status.remove(StatusFlags::CARRY); // Clear carry flag
        }

        self.update_zero_and_negative_flags(result);
    }

    fn dec<M: Memory>(&mut self, memory: &mut M, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(memory, mode);
        let mut value = memory.read(addr);

        value = value.wrapping_sub(1);
        memory.write(addr, value);
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

    fn eor<M: Memory>(&mut self, memory: &mut M, mode: &AddressingMode) {
        let (addr, page_cross) = self.get_operand_address(memory, mode);
        if page_cross {
            self.extra_cycles += 1;
        }
        let value = memory.read(addr);

        self.registers.a ^= value;
        self.update_zero_and_negative_flags(self.registers.a);
    }

    fn inc<M: Memory>(&mut self, memory: &mut M, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(memory, mode);
        let mut value = memory.read(addr);

        value = value.wrapping_add(1);
        memory.write(addr, value);
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

    fn jmp<M: Memory>(&mut self, memory: &mut M, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(memory, mode);
        self.registers.pc = addr;
        // println!("JMP to {:04X}", self.registers.pc);
    }

    fn jsr<M: Memory>(&mut self, memory: &mut M, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(memory, mode);

        let return_addr = self.registers.pc.wrapping_add(2).wrapping_sub(1);

        self.push_stack_u16(memory, return_addr);

        self.registers.pc = addr;
        // println!(
        //     "JSR to {:04X}, stack: {:02X}",
        //     self.registers.pc, self.registers.sp
        // );
    }

    fn lda<M: Memory>(&mut self, memory: &mut M, mode: &AddressingMode) {
        let (addr, page_cross) = self.get_operand_address(memory, mode);
        if page_cross {
            self.extra_cycles += 1;
        }
        let value = memory.read(addr);

        self.registers.a = value;
        self.update_zero_and_negative_flags(self.registers.a);
    }

    fn ldx<M: Memory>(&mut self, memory: &mut M, mode: &AddressingMode) {
        let (addr, page_cross) = self.get_operand_address(memory, mode);
        if page_cross {
            self.extra_cycles += 1;
        }
        let value = memory.read(addr);

        self.registers.x = value;
        self.update_zero_and_negative_flags(self.registers.x);
    }

    fn ldy<M: Memory>(&mut self, memory: &mut M, mode: &AddressingMode) {
        let (addr, page_cross) = self.get_operand_address(memory, mode);
        if page_cross {
            self.extra_cycles += 1;
        }
        let value = memory.read(addr);

        self.registers.y = value;
        self.update_zero_and_negative_flags(self.registers.y);
    }

    fn lsr<M: Memory>(&mut self, memory: &mut M, mode: &AddressingMode) {
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

        let (addr, _) = self.get_operand_address(memory, mode);
        let mut value = memory.read(addr);

        if value & 0b0000_0001 != 0 {
            self.registers.status.insert(StatusFlags::CARRY); // Set carry flag
        } else {
            self.registers.status.remove(StatusFlags::CARRY); // Clear carry flag
        }

        value >>= 1;
        memory.write(addr, value);
        self.update_zero_and_negative_flags(value);
    }

    fn nop<M: Memory>(&mut self, memory: &mut M, mode: &AddressingMode) {
        if matches!(mode, AddressingMode::None | AddressingMode::Accumulator) {
            return;
        }
        let (addr, page_cross) = self.get_operand_address(memory, mode);
        memory.read(addr);
        if page_cross {
            self.extra_cycles += 1;
        }
    }

    fn ora<M: Memory>(&mut self, memory: &mut M, mode: &AddressingMode) {
        let (addr, page_cross) = self.get_operand_address(memory, mode);
        if page_cross {
            self.extra_cycles += 1;
        }
        let value = memory.read(addr);

        self.registers.a |= value;
        self.update_zero_and_negative_flags(self.registers.a);
    }

    fn pha<M: Memory>(&mut self, memory: &mut M) {
        let sp_addr = self.stack_addr();
        memory.write(sp_addr, self.registers.a);
        self.registers.sp = self.registers.sp.wrapping_sub(1);
    }

    fn php<M: Memory>(&mut self, memory: &mut M) {
        let mut flags = StatusFlags::from_bits_truncate(self.registers.status.bits());
        flags.insert(StatusFlags::BREAK_COMMAND);
        flags.insert(StatusFlags::UNUSED);
        self.push_stack(memory, flags.bits());
    }

    fn pla<M: Memory>(&mut self, memory: &mut M) {
        self.registers.a = self.pull_stack(memory);
        self.update_zero_and_negative_flags(self.registers.a);
    }

    fn plp<M: Memory>(&mut self, memory: &mut M) {
        self.registers.sp = self.registers.sp.wrapping_add(1);
        let sp_addr = self.stack_addr();
        self.registers.status = StatusFlags::from_bits_truncate(memory.read(sp_addr));
        self.registers.status.remove(StatusFlags::BREAK_COMMAND);
        self.registers.status.insert(StatusFlags::UNUSED);
    }

    fn rol<M: Memory>(&mut self, memory: &mut M, mode: &AddressingMode) {
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

        let (addr, _) = self.get_operand_address(memory, mode);
        let mut value = memory.read(addr);
        let carry_in = if self.registers.status.contains(StatusFlags::CARRY) {
            1
        } else {
            0
        };

        self.registers
            .status
            .set(StatusFlags::CARRY, value & 0b1000_0000 != 0);

        value = (value << 1) | carry_in;
        memory.write(addr, value);
        self.update_zero_and_negative_flags(value);
    }

    fn ror<M: Memory>(&mut self, memory: &mut M, mode: &AddressingMode) {
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

        let (addr, _) = self.get_operand_address(memory, mode);
        let mut value = memory.read(addr);
        let carry_in = if self.registers.status.contains(StatusFlags::CARRY) {
            0b1000_0000
        } else {
            0
        };

        self.registers
            .status
            .set(StatusFlags::CARRY, value & 0b0000_0001 != 0);

        value = (value >> 1) | carry_in;
        memory.write(addr, value);
        self.update_zero_and_negative_flags(value);
    }

    fn rti<M: Memory>(&mut self, memory: &mut M) {
        let status = self.pull_stack(memory);
        self.registers.status = StatusFlags::from_bits_truncate(status);
        self.registers.status.remove(StatusFlags::BREAK_COMMAND);
        self.registers.status.insert(StatusFlags::UNUSED);

        self.registers.pc = self.pull_stack_u16(memory);
    }

    fn rts<M: Memory>(&mut self, memory: &mut M) {
        let addr = self.pull_stack_u16(memory);
        self.registers.pc = addr.wrapping_add(1);
    }

    fn sbc<M: Memory>(&mut self, memory: &mut M, mode: &AddressingMode) {
        let (addr, page_cross) = self.get_operand_address(memory, mode);
        if page_cross {
            self.extra_cycles += 1;
        }
        let value = memory.read(addr);

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

    fn sta<M: Memory>(&mut self, memory: &mut M, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(memory, mode);
        memory.write(addr, self.registers.a);
    }

    fn stx<M: Memory>(&mut self, memory: &mut M, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(memory, mode);
        memory.write(addr, self.registers.x);
    }

    fn sty<M: Memory>(&mut self, memory: &mut M, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(memory, mode);
        memory.write(addr, self.registers.y);
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
    fn anc<M: Memory>(&mut self, memory: &mut M, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(memory, mode);
        let value = memory.read(addr);
        self.registers.a &= value;
        self.update_zero_and_negative_flags(self.registers.a);
        self.registers
            .status
            .set(StatusFlags::CARRY, (self.registers.a & 0x80) != 0);
    }

    fn alr<M: Memory>(&mut self, memory: &mut M, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(memory, mode);
        let value = memory.read(addr);
        let mut result = self.registers.a & value;
        self.registers
            .status
            .set(StatusFlags::CARRY, (result & 0x01) != 0);
        result >>= 1;
        self.registers.a = result;
        self.update_zero_and_negative_flags(result);
    }

    fn arr<M: Memory>(&mut self, memory: &mut M, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(memory, mode);
        let value = memory.read(addr);
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

    fn axs<M: Memory>(&mut self, memory: &mut M, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(memory, mode);
        let value = memory.read(addr);
        let masked = self.registers.a & self.registers.x;
        let result = masked.wrapping_sub(value);
        self.registers
            .status
            .set(StatusFlags::CARRY, masked >= value);
        self.registers.x = result;
        self.update_zero_and_negative_flags(result);
    }

    fn slo<M: Memory>(&mut self, memory: &mut M, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(memory, mode);
        let mut value = memory.read(addr);
        self.registers
            .status
            .set(StatusFlags::CARRY, (value & 0x80) != 0);
        value <<= 1;
        memory.write(addr, value);
        self.registers.a |= value;
        self.update_zero_and_negative_flags(self.registers.a);
    }

    fn rla<M: Memory>(&mut self, memory: &mut M, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(memory, mode);
        let mut value = memory.read(addr);
        let carry_in = if self.registers.status.contains(StatusFlags::CARRY) {
            1
        } else {
            0
        };
        self.registers
            .status
            .set(StatusFlags::CARRY, (value & 0x80) != 0);
        value = (value << 1) | carry_in;
        memory.write(addr, value);
        self.registers.a &= value;
        self.update_zero_and_negative_flags(self.registers.a);
    }

    fn sre<M: Memory>(&mut self, memory: &mut M, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(memory, mode);
        let mut value = memory.read(addr);
        self.registers
            .status
            .set(StatusFlags::CARRY, (value & 0x01) != 0);
        value >>= 1;
        memory.write(addr, value);
        self.registers.a ^= value;
        self.update_zero_and_negative_flags(self.registers.a);
    }

    fn rra<M: Memory>(&mut self, memory: &mut M, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(memory, mode);
        let mut value = memory.read(addr);
        let carry_in = if self.registers.status.contains(StatusFlags::CARRY) {
            0x80
        } else {
            0
        };
        self.registers
            .status
            .set(StatusFlags::CARRY, (value & 0x01) != 0);
        value = (value >> 1) | carry_in;
        memory.write(addr, value);
        self.adc_value(value);
    }

    fn dcp<M: Memory>(&mut self, memory: &mut M, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(memory, mode);
        let value = memory.read(addr).wrapping_sub(1);
        memory.write(addr, value);
        self.registers
            .status
            .set(StatusFlags::CARRY, self.registers.a >= value);
        let result = self.registers.a.wrapping_sub(value);
        self.update_zero_and_negative_flags(result);
    }

    fn isc<M: Memory>(&mut self, memory: &mut M, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(memory, mode);
        let value = memory.read(addr).wrapping_add(1);
        memory.write(addr, value);
        self.sbc_value(value);
    }

    fn sax<M: Memory>(&mut self, memory: &mut M, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(memory, mode);
        let value = self.registers.a & self.registers.x;
        memory.write(addr, value);
    }

    fn lax<M: Memory>(&mut self, memory: &mut M, mode: &AddressingMode) {
        let (addr, page_cross) = self.get_operand_address(memory, mode);
        if page_cross {
            self.extra_cycles += 1;
        }
        let value = memory.read(addr);
        self.registers.a = value;
        self.registers.x = value;
        self.update_zero_and_negative_flags(value);
    }

    fn lxa<M: Memory>(&mut self, memory: &mut M, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(memory, mode);
        let value = memory.read(addr);
        let result = (self.registers.a | 0xEE) & value;
        self.registers.a = result;
        self.registers.x = result;
        self.update_zero_and_negative_flags(result);
    }

    fn las<M: Memory>(&mut self, memory: &mut M, mode: &AddressingMode) {
        let (addr, page_cross) = self.get_operand_address(memory, mode);
        if page_cross {
            self.extra_cycles += 1;
        }
        let value = memory.read(addr) & self.registers.sp;
        self.registers.sp = value;
        self.registers.a = value;
        self.registers.x = value;
        self.update_zero_and_negative_flags(value);
    }

    fn ahx<M: Memory>(&mut self, memory: &mut M, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(memory, mode);
        let high = ((addr >> 8) as u8).wrapping_add(1);
        let value = self.registers.a & self.registers.x & high;
        memory.write(addr, value);
    }

    fn shy<M: Memory>(&mut self, memory: &mut M, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(memory, mode);
        let high = ((addr >> 8) as u8).wrapping_add(1);
        let value = self.registers.y & high;
        memory.write(addr, value);
    }

    fn shx<M: Memory>(&mut self, memory: &mut M, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(memory, mode);
        let high = ((addr >> 8) as u8).wrapping_add(1);
        let value = self.registers.x & high;
        memory.write(addr, value);
    }

    fn tas<M: Memory>(&mut self, memory: &mut M, mode: &AddressingMode) {
        let mut masked = self.registers.a & self.registers.x;
        self.registers.sp = masked;
        let (addr, _) = self.get_operand_address(memory, mode);
        let high = ((addr >> 8) as u8).wrapping_add(1);
        masked &= high;
        memory.write(addr, masked);
    }

    fn xaa<M: Memory>(&mut self, memory: &mut M, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(memory, mode);
        let value = memory.read(addr);
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

    pub fn get_operand_address<M: Memory>(
        &mut self,
        memory: &mut M,
        mode: &AddressingMode,
    ) -> (u16, bool) {
        match mode {
            AddressingMode::Immediate => (self.registers.pc, false),

            AddressingMode::ZeroPage => (memory.read(self.registers.pc) as u16, false),

            AddressingMode::Absolute => (memory.read_u16(self.registers.pc), false),

            AddressingMode::ZeroPageX => {
                let pos = memory.read(self.registers.pc);
                let addr = pos.wrapping_add(self.registers.x) as u16;
                (addr, false)
            }
            AddressingMode::ZeroPageY => {
                let pos = memory.read(self.registers.pc);
                let addr = pos.wrapping_add(self.registers.y) as u16;
                (addr, false)
            }

            AddressingMode::Relative => {
                let offset = memory.read(self.registers.pc) as i8;
                let next = self.registers.pc.wrapping_add(1) as i32;
                ((next + offset as i32) as u16, false)
            }

            AddressingMode::AbsoluteX => {
                let base = memory.read_u16(self.registers.pc);
                let addr = base.wrapping_add(self.registers.x as u16);
                let page_cross = (base & 0xFF00) != (addr & 0xFF00);
                (addr, page_cross)
            }
            AddressingMode::AbsoluteY => {
                let base = memory.read_u16(self.registers.pc);
                let addr = base.wrapping_add(self.registers.y as u16);
                let page_cross = (base & 0xFF00) != (addr & 0xFF00);
                (addr, page_cross)
            }

            AddressingMode::Indirect => {
                let addr = memory.read_u16(self.registers.pc);

                let indirect_ref = if addr & 0x00FF == 0x00FF {
                    let lo = memory.read(addr);
                    let hi = memory.read(addr & 0xFF00);
                    (hi as u16) << 8 | (lo as u16)
                } else {
                    memory.read_u16(addr)
                };
                (indirect_ref, false)
            }
            AddressingMode::IndirectX => {
                let base = memory.read(self.registers.pc);

                let ptr: u8 = base.wrapping_add(self.registers.x);
                let lo = memory.read(ptr as u16);
                let hi = memory.read(ptr.wrapping_add(1) as u16);
                ((hi as u16) << 8 | (lo as u16), false)
            }
            AddressingMode::IndirectY => {
                let base = memory.read(self.registers.pc);

                let lo = memory.read(base as u16);
                let hi = memory.read(base.wrapping_add(1) as u16);
                let deref_base = (hi as u16) << 8 | (lo as u16);
                let deref = deref_base.wrapping_add(self.registers.y as u16);
                let page_cross = (deref_base & 0xFF00) != (deref & 0xFF00);
                (deref, page_cross)
            }

            AddressingMode::None | AddressingMode::Accumulator => {
                // dummy read
                (memory.read(self.registers.pc + 1) as u16, false)
            }
        }
    }
}

/// Helpers
impl CPU {
    fn stack_addr(&self) -> u16 {
        STACK_START + self.registers.sp as u16
    }

    fn push_stack<M: Memory>(&mut self, memory: &mut M, v: u8) {
        let addr = self.stack_addr();
        memory.write(addr, v);
        self.registers.sp = self.registers.sp.wrapping_sub(1);
    }

    fn pull_stack<M: Memory>(&mut self, memory: &mut M) -> u8 {
        self.registers.sp = self.registers.sp.wrapping_add(1);
        let addr = self.stack_addr();
        memory.read(addr)
    }

    fn push_stack_u16<M: Memory>(&mut self, memory: &mut M, v: u16) {
        // push high then low
        self.push_stack(memory, (v >> 8) as u8);
        self.push_stack(memory, (v & 0xFF) as u8);
    }

    fn pull_stack_u16<M: Memory>(&mut self, memory: &mut M) -> u16 {
        let lo = self.pull_stack(memory) as u16;
        let hi = self.pull_stack(memory) as u16;
        (hi << 8) | lo
    }

    fn interrupt<M: Memory>(&mut self, memory: &mut M, interrupt: interrupt::Interrupt) {
        self.push_stack_u16(memory, self.registers.pc);
        let mut flag = StatusFlags::from_bits_truncate(self.registers.status.bits());
        flag.remove(StatusFlags::BREAK_COMMAND);
        flag.insert(StatusFlags::UNUSED);

        self.push_stack(memory, flag.bits());
        self.registers.status.insert(StatusFlags::INTERRUPT_DISABLE);

        self.cycles_wait = self.cycles_wait.wrapping_add(interrupt.cpu_cycles);
        self.registers.pc = memory.read_u16(interrupt.vector_addr);
    }
}
