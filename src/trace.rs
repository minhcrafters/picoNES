use crate::cpu::AddressingMode;
use crate::cpu::CPU;
use crate::opcodes::CPU_OPCODES;

pub fn trace(cpu: &mut CPU) -> String {
    let code = cpu.memory.read(cpu.registers.pc);
    let ops = CPU_OPCODES.find_by_code(code).unwrap();

    let begin = cpu.registers.pc;
    let mut hex_dump = vec![];
    hex_dump.push(code);

    let (mem_addr, stored_value) = match ops.addressing_mode {
        AddressingMode::Immediate | AddressingMode::None => (0, 0),
        _ => {
            let (addr, _) = cpu.get_operand_address(&ops.addressing_mode);
            (addr, cpu.memory.read(addr))
        }
    };

    let tmp = match ops.bytes {
        1 => match ops.code {
            0x0a | 0x4a | 0x2a | 0x6a => format!("A "),
            _ => String::from(""),
        },
        2 => {
            let address: u8 = cpu.memory.read(begin + 1);
            // let value = cpu.bus.read(address));
            hex_dump.push(address);

            match ops.addressing_mode {
                AddressingMode::Immediate => format!("#${:02x}", address),
                AddressingMode::ZeroPage => format!("${:02x} = {:02x}", mem_addr, stored_value),
                AddressingMode::ZeroPageX => format!(
                    "${:02x},X @ {:02x} = {:02x}",
                    address, mem_addr, stored_value
                ),
                AddressingMode::ZeroPageY => format!(
                    "${:02x},Y @ {:02x} = {:02x}",
                    address, mem_addr, stored_value
                ),
                AddressingMode::IndirectX => format!(
                    "(${:02x},X) @ {:02x} = {:04x} = {:02x}",
                    address,
                    (address.wrapping_add(cpu.registers.x)),
                    mem_addr,
                    stored_value
                ),
                AddressingMode::IndirectY => format!(
                    "(${:02x}),Y = {:04x} @ {:04x} = {:02x}",
                    address,
                    (mem_addr.wrapping_sub(cpu.registers.y as u16)),
                    mem_addr,
                    stored_value
                ),
                AddressingMode::None => {
                    // assuming local jumps: BNE, BVS, etc....
                    let address: usize =
                        (begin as usize + 2).wrapping_add((address as i8) as usize);
                    format!("${:04x}", address)
                }

                _ => String::new(),
            }
        }
        3 => {
            let address_lo = cpu.memory.read(begin + 1);
            let address_hi = cpu.memory.read(begin + 2);
            hex_dump.push(address_lo);
            hex_dump.push(address_hi);

            let address = cpu.memory.read_u16(begin + 1);

            match ops.addressing_mode {
                AddressingMode::None => {
                    if ops.code == 0x6c {
                        //jmp indirect
                        let jmp_addr = if address & 0x00FF == 0x00FF {
                            let lo = cpu.memory.read(address);
                            let hi = cpu.memory.read(address & 0xFF00);
                            (hi as u16) << 8 | (lo as u16)
                        } else {
                            cpu.memory.read_u16(address)
                        };

                        // let jmp_addr = cpu.bus.read_u16(address);
                        format!("(${:04x}) = {:04x}", address, jmp_addr)
                    } else {
                        format!("${:04x}", address)
                    }
                }
                AddressingMode::Absolute => format!("${:04x} = {:02x}", mem_addr, stored_value),
                AddressingMode::AbsoluteX => format!(
                    "${:04x},X @ {:04x} = {:02x}",
                    address, mem_addr, stored_value
                ),
                AddressingMode::AbsoluteY => format!(
                    "${:04x},Y @ {:04x} = {:02x}",
                    address, mem_addr, stored_value
                ),
                _ => panic!(
                    "unexpected addressing mode {:?} has ops-len 3. code {:02x}",
                    ops.addressing_mode, ops.code
                ),
            }
        }
        _ => String::from(""),
    };

    let hex_str = hex_dump
        .iter()
        .map(|z| format!("{:02x}", z))
        .collect::<Vec<String>>()
        .join(" ");
    let asm_str = format!("{:04x}  {:8} {: >4} {}", begin, hex_str, ops.mnemonic, tmp)
        .trim()
        .to_string();

    format!(
        "{:47} A:{:02x} X:{:02x} Y:{:02x} P:{:02x} SP:{:02x}",
        asm_str,
        cpu.registers.a,
        cpu.registers.x,
        cpu.registers.y,
        cpu.registers.status,
        cpu.registers.sp,
    )
    .to_ascii_uppercase()
}
