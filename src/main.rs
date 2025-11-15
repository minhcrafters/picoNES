use crate::cpu::CPU;

mod cpu;
mod memory;
mod opcodes;

fn main() {
    let mut cpu = CPU::new();

    cpu.load_and_run(vec![0xAD, 0x34, 0x12, 0x00], None);

    println!("{:?}", cpu.registers);
}
