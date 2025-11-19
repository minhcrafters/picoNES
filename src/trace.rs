use crate::bus::Bus;
use crate::cpu::CPU;
use crate::opcodes::{AddressingMode, CPU_OPCODES};

pub fn trace(cpu: &CPU, bus: &Bus) -> String {
    let pc = cpu.registers.pc;
    let opcode = bus.peek(pc);
    let ops = CPU_OPCODES.find_by_code(opcode).unwrap();

    let mut hex_dump = vec![opcode];
    let (mem_addr, stored_value) = match ops.mode {
        AddressingMode::Immediate | AddressingMode::None | AddressingMode::Accumulator => (0, 0),
        _ => {
            let (addr, value) = operand(bus, cpu, &ops.mode);
            (addr, value)
        }
    };

    let operand_str = match ops.bytes {
        1 => match ops.code {
            0x0a | 0x4a | 0x2a | 0x6a => "A ".to_string(),
            _ => String::new(),
        },
        2 => {
            let value = bus.peek(pc.wrapping_add(1));
            hex_dump.push(value);
            match ops.mode {
                AddressingMode::Immediate => format!("#${:02x}", value),
                AddressingMode::ZeroPage => format!("${:02x} = {:02x}", mem_addr, stored_value),
                AddressingMode::ZeroPageX => {
                    format!("${:02x},X @ {:02x} = {:02x}", value, mem_addr, stored_value)
                }
                AddressingMode::ZeroPageY => {
                    format!("${:02x},Y @ {:02x} = {:02x}", value, mem_addr, stored_value)
                }
                AddressingMode::IndirectX => format!(
                    "(${:02x},X) @ {:02x} = {:04x} = {:02x}",
                    value,
                    value.wrapping_add(cpu.registers.x),
                    mem_addr,
                    stored_value
                ),
                AddressingMode::IndirectY => format!(
                    "(${:02x}),Y = {:04x} @ {:04x} = {:02x}",
                    value,
                    mem_addr.wrapping_sub(cpu.registers.y as u16),
                    mem_addr,
                    stored_value
                ),
                AddressingMode::None => {
                    let offset = value as i8;
                    let target = (pc as i32 + 2 + offset as i32) as u16;
                    format!("${:04x}", target)
                }
                _ => String::new(),
            }
        }
        3 => {
            let lo = bus.peek(pc.wrapping_add(1));
            let hi = bus.peek(pc.wrapping_add(2));
            hex_dump.push(lo);
            hex_dump.push(hi);
            let absolute = ((hi as u16) << 8) | lo as u16;
            match ops.mode {
                AddressingMode::None => {
                    if ops.code == 0x6c {
                        let addr = if absolute & 0x00ff == 0x00ff {
                            let lo = bus.peek(absolute);
                            let hi = bus.peek(absolute & 0xff00);
                            (hi as u16) << 8 | lo as u16
                        } else {
                            read_u16(bus, absolute)
                        };
                        format!("(${:04x}) = {:04x}", absolute, addr)
                    } else {
                        format!("${:04x}", absolute)
                    }
                }
                AddressingMode::Absolute => format!("${:04x} = {:02x}", mem_addr, stored_value),
                AddressingMode::AbsoluteX => format!(
                    "${:04x},X @ {:04x} = {:02x}",
                    absolute, mem_addr, stored_value
                ),
                AddressingMode::AbsoluteY => format!(
                    "${:04x},Y @ {:04x} = {:02x}",
                    absolute, mem_addr, stored_value
                ),
                _ => String::new(),
            }
        }
        _ => String::new(),
    };

    let hex_str = hex_dump
        .iter()
        .map(|byte| format!("{:02x}", byte))
        .collect::<Vec<_>>()
        .join(" ");

    let asm_str = format!(
        "{:04x}  {:8} {: >4} {}",
        pc, hex_str, ops.mnemonic, operand_str
    )
    .trim()
    .to_string();

    format!(
        "{:47} A:{:02x} X:{:02x} Y:{:02x} P:{:02x} SP:{:02x}",
        asm_str,
        cpu.registers.a,
        cpu.registers.x,
        cpu.registers.y,
        cpu.registers.status,
        cpu.registers.sp
    )
    .to_ascii_uppercase()
}

fn operand(bus: &Bus, cpu: &CPU, mode: &AddressingMode) -> (u16, u8) {
    let pc = cpu.registers.pc;
    match mode {
        AddressingMode::ZeroPage => {
            let addr = bus.peek(pc.wrapping_add(1)) as u16;
            (addr, bus.peek(addr))
        }
        AddressingMode::ZeroPageX => {
            let base = bus.peek(pc.wrapping_add(1));
            let addr = base.wrapping_add(cpu.registers.x) as u16;
            (addr, bus.peek(addr))
        }
        AddressingMode::ZeroPageY => {
            let base = bus.peek(pc.wrapping_add(1));
            let addr = base.wrapping_add(cpu.registers.y) as u16;
            (addr, bus.peek(addr))
        }
        AddressingMode::Absolute => {
            let addr = read_u16(bus, pc.wrapping_add(1));
            (addr, bus.peek(addr))
        }
        AddressingMode::AbsoluteX => {
            let base = read_u16(bus, pc.wrapping_add(1));
            let addr = base.wrapping_add(cpu.registers.x as u16);
            (addr, bus.peek(addr))
        }
        AddressingMode::AbsoluteY => {
            let base = read_u16(bus, pc.wrapping_add(1));
            let addr = base.wrapping_add(cpu.registers.y as u16);
            (addr, bus.peek(addr))
        }
        AddressingMode::Indirect => {
            let base = read_u16(bus, pc.wrapping_add(1));
            let addr = if base & 0x00ff == 0x00ff {
                let lo = bus.peek(base);
                let hi = bus.peek(base & 0xff00);
                (hi as u16) << 8 | lo as u16
            } else {
                read_u16(bus, base)
            };
            (addr, bus.peek(addr))
        }
        AddressingMode::IndirectX => {
            let base = bus.peek(pc.wrapping_add(1));
            let ptr = base.wrapping_add(cpu.registers.x);
            let lo = bus.peek(ptr as u16);
            let hi = bus.peek(ptr.wrapping_add(1) as u16);
            let addr = (hi as u16) << 8 | lo as u16;
            (addr, bus.peek(addr))
        }
        AddressingMode::IndirectY => {
            let base = bus.peek(pc.wrapping_add(1));
            let lo = bus.peek(base as u16);
            let hi = bus.peek(base.wrapping_add(1) as u16);
            let deref = ((hi as u16) << 8 | lo as u16).wrapping_add(cpu.registers.y as u16);
            (deref, bus.peek(deref))
        }
        AddressingMode::Relative => {
            let offset = bus.peek(pc.wrapping_add(1)) as i8;
            let base = pc.wrapping_add(2);
            let target = (base as i32 + offset as i32) as u16;
            (target, bus.peek(target))
        }
        AddressingMode::Immediate | AddressingMode::None | AddressingMode::Accumulator => (0, 0),
    }
}

fn read_u16(bus: &Bus, addr: u16) -> u16 {
    let lo = bus.peek(addr) as u16;
    let hi = bus.peek(addr.wrapping_add(1)) as u16;
    (hi << 8) | lo
}
