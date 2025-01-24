use std::collections::HashMap;
use crate::opcode;

bitflags! {
    pub struct CpuFlags: u8 {
        const CARRY             = 0b0000_0001;
        const ZERO              = 0b0000_0010;
        const INTERRUPT_DISABLE = 0b0000_0100;
        const DECIMAL_MODE      = 0b0000_1000;
        const BREAK             = 0b0001_0000;
        const BREAK2            = 0b0010_0000;
        const OVERFLOW          = 0b0100_0000;
        const NEGATIV           = 0b1000_0000;
    }
}
const STACK: u16 = 0x0100;
const STACK_RESET: u8 = 0xfd;

pub struct CPU {
    pub register_a : u8,
    pub register_x : u8,
    pub register_y : u8,
    pub status : CpuFlags,
    pub program_count : u16,
    pub stack_pointer: u8,
    mem: [u8; 0xFFFF]
}

#[derive(Debug)]
#[allow(non_camel_case_types)]
pub enum AddressingMode {
   Immediate,
   ZeroPage,
   ZeroPage_X,
   ZeroPage_Y,
   Absolute,
   Absolute_X,
   Absolute_Y,
   Indirect_X,
   Indirect_Y,
   NoneAddressing,
}


pub trait Mem {
    fn mem_read(&self, addr: u16) -> u8;

    fn mem_write(&mut self, addr: u16, data: u8);

    fn mem_read_u16(&self, pos: u16) -> u16 {
        let lo = self.mem_read(pos) as u16;
        let hi = self.mem_read(pos + 1) as u16;
        (hi << 8) | (lo as u16)
    }

    fn mem_write_u16(&mut self, pos: u16, data: u16) {
        let hi = (data >> 8) as u8;
        let lo = (data & 0xff) as u8;
        self.mem_write(pos, lo);
        self.mem_write(pos + 1, hi);
    }
}

impl Mem for CPU {
    fn mem_read(&self, addr: u16) -> u8 {
        self.mem[addr as usize]
    }

    fn mem_write(&mut self, addr: u16, data: u8) {
        self.mem[addr as usize] = data;
    }
}


impl CPU {
    pub fn new () -> Self {
        CPU {
            register_a : 0,
            register_x : 0,
            register_y : 0,
            stack_pointer:STACK_RESET,
            status : CpuFlags::from_bits_truncate(0b10_0100),
            program_count : 0,
            mem: [0; 0xFFFF]
        }
    }

    pub fn reset (&mut self) {
        self.register_a = 0;
        self.register_x = 0;
        self.register_y = 0;
        self.status = CpuFlags::from_bits_truncate(0b10_0100);
        self.program_count = self.mem_read_u16(0xFFFC);
    }

    pub fn load(&mut self, program: Vec<u8>) {
        self.mem[0x0600..(0x0600 + program.len())].copy_from_slice(&program[..]);
        self.mem_write_u16(0xFFFC, 0x0600);
    }

    fn get_operand_address(&mut self, mode: &AddressingMode) -> u16 {
        match mode {
            AddressingMode::Immediate => self.program_count,
            AddressingMode::ZeroPage => self.mem_read(self.program_count) as u16,
            AddressingMode::Absolute => self.mem_read_u16(self.program_count),

            AddressingMode::ZeroPage_X => {
                let addr = self.mem_read(self.program_count) as u16;
                addr.wrapping_add(self.register_x as u16)
            }
            AddressingMode::ZeroPage_Y => {
                let addr = self.mem_read(self.program_count) as u16;
                addr.wrapping_add(self.register_y as u16)
            }

            AddressingMode::Absolute_X => {
                let addr = self.mem_read_u16(self.program_count);
                addr.wrapping_add(self.register_x as u16)
            }

            AddressingMode::Absolute_Y => {
                let addr = self.mem_read_u16(self.program_count);
                addr.wrapping_add(self.register_y as u16)
            }

            AddressingMode::Indirect_X => {
                let base = self.mem_read(self.program_count);
                let ptr = (base as u8).wrapping_add(self.register_x);
                let lo = self.mem_read(ptr as u16);
                let hi = self.mem_read(ptr.wrapping_add(1) as u16);
                ((hi as u16) << 8) | (lo as u16)
            }

            AddressingMode::Indirect_Y => {
                let base = self.mem_read(self.program_count);
                let lo = self.mem_read(base as u16);
                let hi = self.mem_read(base.wrapping_add(1) as u16);
                let addr = ((hi as u16) << 8) | (lo as u16);
                addr.wrapping_add(self.register_y as u16)
            }

            AddressingMode::NoneAddressing => panic!("Invalid Addressing Mode")
        }
    }

    pub fn load_and_run (&mut self, program: Vec<u8>) {
        self.load(program);
        self.reset();
        self.run();
    }

    fn ldy(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let value = self.mem_read(addr);

        self.register_y = value;
        self.update_zero_and_negative_flags(self.register_y);
    }

    fn ldx(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let value = self.mem_read(addr);

        self.register_x = value;
        self.update_zero_and_negative_flags(self.register_x);
    }

    fn lda(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let value = self.mem_read(addr);

        self.set_register_a(value);
    }

    fn sta (&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        self.mem_write(addr, self.register_a);
    }

    fn set_register_a(&mut self, value: u8) {
        self.register_a = value;
        self.update_zero_and_negative_flags(self.register_a);
    }

    fn and(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let value = self.mem_read(addr);

        self.set_register_a(self.register_a & value);
    }

    fn eor(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let value = self.mem_read(addr);

        self.set_register_a(self.register_a ^ value);
    }

    fn ora(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let value = self.mem_read(addr);

        self.set_register_a(self.register_a | value);
    }

    fn tax(&mut self) {
        self.register_x = self.register_a;
        self.update_zero_and_negative_flags(self.register_x);
    }

    fn update_zero_and_negative_flags (&mut self, result: u8) {
        if result == 0 {
            self.status.insert(CpuFlags::ZERO);
        } else {
            self.status.remove(CpuFlags::ZERO);
        }

        if result >> 7 == 1 {
            self.status.insert(CpuFlags::NEGATIV);
        } else {
            self.status.remove(CpuFlags::NEGATIV);
        }
    }

    fn update_negative_flag (&mut self, result: u8) {
        if result >> 7 == 1 {
            self.status.insert(CpuFlags::NEGATIV);
        } else {
            self.status.remove(CpuFlags::NEGATIV);
        }
    }

    fn inx(&mut self) {
        self.register_x = self.register_x.wrapping_add(1);
        self.update_zero_and_negative_flags(self.register_x);
    }

    fn iny(&mut self) {
        self.register_y = self.register_y.wrapping_add(1);
        self.update_zero_and_negative_flags(self.register_y);
    }

    fn set_carry_flag(&mut self) {
        self.status.insert(CpuFlags::CARRY);
    }

    fn clear_carry_flag(&mut self) {
        self.status.remove(CpuFlags::CARRY)
    }

    fn add_to_register_a(&mut self, data: u8) {
        let sum = self.register_a as u16 + data as u16 + (if self.status.contains(CpuFlags::CARRY) { 1 } else { 0 }) as u16;

        let carry = sum > 0xff;

        if carry {
            self.set_carry_flag()
        }
        else {

            self.clear_carry_flag()
        }

        let result = sum as u8;

        if (data ^ result) & (result ^ self.register_a) & 0x80 != 0 {
            self.status.insert(CpuFlags::OVERFLOW);
        } else {
            self.status.remove(CpuFlags::OVERFLOW);
        }

        self.set_register_a(result);
    }

    fn sbc(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let value = self.mem_read(addr);
        self.add_to_register_a(((value as i8).wrapping_neg().wrapping_sub(1)) as u8);
    }

    fn adc(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let value = self.mem_read(addr);
        self.add_to_register_a(value);
    }

    fn stack_pop (&mut self) -> u8 {
        self.stack_pointer = self.stack_pointer.wrapping_add(1);
        self.mem_read((STACK as u16) + self.stack_pointer as u16)
    }

    fn stack_push (&mut self, value: u8) {
        self.mem_write((STACK as u16) + self.stack_pointer as u16, value);
        self.stack_pointer = self.stack_pointer.wrapping_sub(1)
    }

    fn stack_push_u16 (&mut self, value: u16) {
        let hi = (value >> 8) as u8;
        let lo = (value & 0xff) as u8;

        self.stack_push(hi);
        self.stack_push(lo);
    }

    fn stack_pop_u16 (&mut self) -> u16 {
        let lo = self.stack_pop() as u16;
        let hi = self.stack_pop() as u16;

        hi << 8 | lo
    }

    fn asl_accumulator(&mut self) {
        let mut data = self.register_a;

        if data >> 7 == 1 {
            self.set_carry_flag();
        } else {
            self.clear_carry_flag();
        }

        data = data << 1;
        self.set_register_a(data)
    }

    fn asl(&mut self, mode: &AddressingMode) -> u8 {
        let addr = self.get_operand_address(mode);
        let mut value = self.mem_read(addr);

        if value >> 7 == 1 {
            self.set_carry_flag();
        } else {
            self.clear_carry_flag();
        }

        value = value << 1;
        self.mem_write(addr, value);
        self.update_zero_and_negative_flags(value);
        value
    }

    fn lsr_accumulator(&mut self) {
        let mut data = self.register_a;

        if data & 1 == 1 {
            self.set_carry_flag();
        } else {
            self.clear_carry_flag();
        }

        data = data >> 1;
        self.set_register_a(data)
    }

    fn lsr(&mut self, mode: &AddressingMode) -> u8 {
        let addr = self.get_operand_address(mode);
        let mut value = self.mem_read(addr);

        if value & 1 == 1 {
            self.set_carry_flag();
        } else {
            self.clear_carry_flag();
        }

        value = value >> 1;
        self.mem_write(addr, value);
        self.update_zero_and_negative_flags(value);
        value
    }

    fn rol_accumulator(&mut self) {
        let mut data = self.register_a;
        let old_carry = self.status.contains(CpuFlags::CARRY);

        if data >> 7 == 1 {
            self.set_carry_flag();
        }
        else {
            self.clear_carry_flag();
        }

        data = data << 1;
        if old_carry {
            data = data | 1;
        }

        self.set_register_a(data);
    }

    fn rol(&mut self, mode: &AddressingMode) -> u8 {
        let addr = self.get_operand_address(mode);
        let mut value = self.mem_read(addr);
        let old_carry = self.status.contains(CpuFlags::CARRY);

        if value >> 7 == 1 {
            self.set_carry_flag();
        } else {
            self.clear_carry_flag();
        }

        value = value << 1;
        if old_carry {
            value = value | 1;
        }

        self.update_negative_flag(value);
        self.mem_write(addr, value);
        value
    }

    fn ror_accumulator(&mut self) {
        let mut data = self.register_a;
        let old_carry = self.status.contains(CpuFlags::CARRY);

        if data & 1 == 1 {
            self.set_carry_flag();
        }
        else {
            self.clear_carry_flag();
        }

        data = data >> 1;
        if old_carry {
            data = data | 0b1000_0000;
        }

        self.set_register_a(data);
    }

    fn ror(&mut self, mode: &AddressingMode) -> u8 {
        let addr = self.get_operand_address(mode);
        let mut value = self.mem_read(addr);
        let old_carry = self.status.contains(CpuFlags::CARRY);

        if value & 1 == 1 {
            self.set_carry_flag();
        } else {
            self.clear_carry_flag();
        }

        value = value >> 1;
        if old_carry {
            value = value | 0b1000_0000;
        }

        self.mem_write(addr, value);
        self.update_zero_and_negative_flags(value);
        value
    }

    fn inc(&mut self, mode: &AddressingMode) -> u8 {
        let addr = self.get_operand_address(mode);
        let mut value = self.mem_read(addr);

        value = value.wrapping_add(1);
        self.mem_write(addr, value);
        self.update_zero_and_negative_flags(value);
        value
    }

    fn dey(&mut self) {
        self.register_y = self.register_y.wrapping_sub(1);
        self.update_zero_and_negative_flags(self.register_y);
    }

    fn dex(&mut self) {
        self.register_x = self.register_x.wrapping_sub(1);
        self.update_zero_and_negative_flags(self.register_x);
    }

    fn dec(&mut self, mode: &AddressingMode) -> u8 {
        let addr = self.get_operand_address(mode);
        let mut value = self.mem_read(addr);

        value = value.wrapping_sub(1);
        self.mem_write(addr, value);
        self.update_zero_and_negative_flags(value);
        value
    }

    fn pla (&mut self) {
        let value = self.stack_pop();
        self.register_a = value;
        self.update_zero_and_negative_flags(self.register_a);
    }

    fn plp (&mut self) {
        self.status.bits = self.stack_pop();
        self.status.remove(CpuFlags::BREAK);
        self.status.insert(CpuFlags::BREAK2);
    }

    fn php (&mut self) {
        self.stack_push(self.status.bits);
        self.status.remove(CpuFlags::BREAK);
        self.status.insert(CpuFlags::BREAK);
    }

    fn bit (&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let value = self.mem_read(addr);
        let and = self.register_a & value;

        if and == 0 {
            self.status.insert(CpuFlags::ZERO);
        } else {
            self.status.remove(CpuFlags::ZERO);
        }

        self.status.set(CpuFlags::NEGATIV, value & 0b1000_0000 != 0);
        self.status.set(CpuFlags::OVERFLOW, value & 0b0100_0000 != 0);
    }

    fn compare(&mut self, mode: &AddressingMode, compare_with: u8) {
        let addr = self.get_operand_address(mode);
        let value = self.mem_read(addr);
        if value <= compare_with {
            self.set_carry_flag();
        } else {
            self.clear_carry_flag();
        }

        self.update_zero_and_negative_flags(compare_with.wrapping_sub(value));
    }

    fn branch(&mut self, condition: bool) {
        if condition {
            let jump: i8 = self.mem_read(self.program_count) as i8;
            let jump_addr = self.program_count.wrapping_add(jump as u16).wrapping_add(1);
            self.program_count = jump_addr;
        }
    }

    pub fn run(&mut self) {
        self.run_with_callback(|_| {});
    }

    pub fn run_with_callback<F>(&mut self, mut callback: F) 
    where 
        F: FnMut(&mut CPU),
    {
        let ref opcode: HashMap<u8, &'static opcode::OpCode> = *opcode::OPCODES_MAP;

        loop {
            let code = self.mem_read(self.program_count);
            self.program_count += 1;
            let program_count_state = self.program_count;

            let opcode = opcode.get(&code).expect(&format!("Code {:x} is not recognized", code));

            println!("opcode: {:x?}", opcode);
            println!("program_count: {:x?}", self.program_count);
            println!("stack_pointer: {:x?}", self.stack_pointer);
            println!("register_a: {:x?}", self.register_a);
            println!("register_x: {:x?}", self.register_x);
            println!("register_y: {:x?}", self.register_y);
            println!("status: {:x?}", self.status);
            match code {

                /* ADC */
                0x69 | 0x65 | 0x75 | 0x6d | 0x7d | 0x79 | 0x61 | 0x71 => {
                    self.adc(&opcode.mode)
                }

                /* AND */
                0x29 | 0x25 | 0x35 | 0x2d | 0x3d | 0x39 | 0x21 | 0x31 => {
                    self.and(&opcode.mode);
                }

                /* ASL */
                0x0a => self.asl_accumulator(),
                0x06 | 0x16 | 0x0e | 0x1e => {
                    self.asl(&opcode.mode);
                }

                /* BCC */
                0x90 => self.branch(!self.status.contains(CpuFlags::CARRY)),

                /* BCS */
                0xb0 => self.branch(self.status.contains(CpuFlags::CARRY)),

                /* BEQ */
                0xf0 => self.branch(self.status.contains(CpuFlags::ZERO)),

                /* BIT */
                0x24 | 0x2c => self.bit(&opcode.mode),

                /* BMI */
                0x30 => self.branch(self.status.contains(CpuFlags::NEGATIV)),

                /* BNE */
                0xd0 => self.branch(!self.status.contains(CpuFlags::ZERO)),

                /* BPL */
                0x10 => self.branch(!self.status.contains(CpuFlags::NEGATIV)),

                /* BRK */
                0x00 => {
                    println!("BRK");
                    return
                }

                /* BVC */
                0x50 => self.branch(!self.status.contains(CpuFlags::OVERFLOW)),

                /* BVS */
                0x70 => self.branch(self.status.contains(CpuFlags::OVERFLOW)),

                /* CLC */
                0x18 => self.status.remove(CpuFlags::CARRY),

                /* CLD */
                0xd8 => self.status.remove(CpuFlags::DECIMAL_MODE),

                /* CLI */
                0x58 => self.status.remove(CpuFlags::INTERRUPT_DISABLE),

                /* CLV */
                0xb8 => self.status.remove(CpuFlags::OVERFLOW),

                /* CMP */
                0xc9 | 0xc5 | 0xd5 | 0xcd | 0xdd | 0xd9 | 0xc1 | 0xd1 => {
                    self.compare(&opcode.mode, self.register_a);
                }

                /* CPX */
                0xe0 | 0xe4 | 0xec => {
                    self.compare(&opcode.mode, self.register_x);
                }

                /* CPY */
                0xc0 | 0xc4 | 0xcc => {
                    self.compare(&opcode.mode, self.register_y);
                }

                /* DEC */
                0xc6 | 0xd6 | 0xce | 0xde => {
                    self.dec(&opcode.mode);
                }

                /* DEX */
                0xca => self.dex(),

                /* DEY */
                0x88 => self.dey(),

                /* EOR */
                0x49 | 0x45 | 0x55 | 0x4d | 0x5d | 0x59 | 0x41 | 0x51 => {
                    self.eor(&opcode.mode);
                }

                /* INC */
                0xe6 | 0xf6 | 0xee | 0xef => {
                    self.inc(&opcode.mode);
                }

                /* INX */
                0xe8 => self.inx(),

                /* INY */
                0xc8 => self.iny(),

                /* JMP Absolute */
                0x4c => {
                    let mem_address = self.mem_read_u16(self.program_count);
                    self.program_count = mem_address;
                }

                /* JMP Indirect */
                0x6c => {
                   let mem_address = self.mem_read_u16(self.program_count);
                    // let indirect_ref = self.mem_read_u16(mem_address);
                    //6502 bug mode with with page boundary:
                    //  if address $3000 contains $40, $30FF contains $80, and $3100 contains $50,
                    // the result of JMP ($30FF) will be a transfer of control to $4080 rather than $5080 as you intended
                    // i.e. the 6502 took the low byte of the address from $30FF and the high byte from $3000

                    let indirect_ref = if mem_address & 0x00FF == 0x00FF {
                        let lo = self.mem_read(mem_address);
                        let hi = self.mem_read(mem_address & 0xFF00);
                        (hi as u16) << 8 | (lo as u16)
                    } else {
                        self.mem_read_u16(mem_address)
                    };

                    self.program_count = indirect_ref;
                }

                /* JSR */
                0x20 => {
                    self.stack_push_u16(self.program_count + 2 - 1);
                    let target_address = self.mem_read_u16(self.program_count);
                    self.program_count = target_address;
                }

                /* LDA */
                0xa9 | 0xa5 | 0xb5 | 0xad | 0xbd | 0xb9 | 0xa1 | 0xb1 => {
                    self.lda(&opcode.mode);
                }

                /* LDX */
                0xa2 | 0xa6 | 0xb6 | 0xae | 0xbe => {
                    self.ldx(&opcode.mode)
                }

                /* LDY */
                0xa0 | 0xa4 | 0xb4 | 0xac | 0xbc => {
                    self.ldy(&opcode.mode)
                }

                /* LSR */
                0x4a => self.lsr_accumulator(),
                0x46 | 0x56 | 0x4e | 0x5e => {
                    self.lsr(&opcode.mode);
                }

                /* NOP */
                0xea => {
                }

                /* ORA */
                0x09 | 0x05 | 0x15 | 0x0d | 0x1d | 0x19 | 0x01 | 0x11 => {
                    self.ora(&opcode.mode);
                }

                /*PHA */
                0x48 => self.stack_push(self.register_a),

                /* PHP */
                0x08 => self.php(),

                /* PLA */
                0x68 => self.pla(),

                /* PLP */
                0x28 => self.plp(),

                /* ROL */
                0x2a => self.rol_accumulator(),
                0x26 | 0x36 | 0x2e | 0x3e => {
                    self.rol(&opcode.mode);
                }

                /* ROR */
                0x6a => self.ror_accumulator(),
                0x66 | 0x76 | 0x6e | 0x7e => {
                    self.ror(&opcode.mode);
                }


                /* RTI */
                0x40 => {
                    self.plp();
                    self.program_count = self.stack_pop_u16();
                }

                /* RTS */
                0x60 => {
                    self.program_count = self.stack_pop_u16() + 1;
                }

                /* SBC */
                0xe9 | 0xe5 | 0xf5 | 0xed | 0xfd | 0xf9 | 0xe1 | 0xf1 => {
                    self.sbc(&opcode.mode)
                }

                /* SEC */
                0x38 => self.status.insert(CpuFlags::CARRY),

                /* SED */
                0xf8 => self.status.insert(CpuFlags::DECIMAL_MODE),

                /* SEI */
                0x78 => self.status.insert(CpuFlags::INTERRUPT_DISABLE),

                /* STA */
                0x85 | 0x95 | 0x8d | 0x9d | 0x99 | 0x81 | 0x91 => {
                    self.sta(&opcode.mode)
                }

                /* STX */
                0x86 | 0x96 | 0x8e => {
                    let addr = self.get_operand_address(&opcode.mode);
                    self.mem_write(addr, self.register_x);
                }

                /* STY */
                0x84 | 0x94 | 0x8c => {
                    let addr = self.get_operand_address(&opcode.mode);
                    self.mem_write(addr, self.register_y);
                }

                /* TAX */
                0xaa => self.tax(),

                /* TAY */
                0xa8 => {
                    self.register_y = self.register_a;
                    self.update_zero_and_negative_flags(self.register_y);
                }

                /* TSX */
                0xba => {
                    self.register_x = self.stack_pointer;
                    self.update_zero_and_negative_flags(self.register_x);
                }

                /* TXA */
                0x8a => {
                    self.register_a = self.register_x;
                    self.update_zero_and_negative_flags(self.register_a);
                }

                /* TXS */
                0x9a => {
                    self.stack_pointer = self.register_x;
                }

                /* TYA */
                0x98 => {
                    self.register_a = self.register_y;
                    self.update_zero_and_negative_flags(self.register_a);
                }

                _ => todo!()
            }

            if program_count_state == self.program_count {
                self.program_count += (opcode.len - 1) as u16;
            }

            println!("cpu status: {:x}", self.program_count);
            println!("");
            callback(self);
        }
    }
}


#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_0xa9_lda_immidiate_load_data() {
        let mut cpu = CPU::new();
        cpu.load_and_run(vec![0xa9, 0x05, 0x00]);
        assert_eq!(cpu.register_a, 0x05);
        assert!(!cpu.status.contains(CpuFlags::ZERO));
        assert!(!cpu.status.contains(CpuFlags::NEGATIV));
    }

    #[test]
    fn test_0xa9_lda_zero_flag() {
        let mut cpu = CPU::new();
        cpu.load_and_run(vec![0xa9, 0x00, 0x00]);
        assert_eq!(cpu.register_a, 0x00);
        assert!(cpu.status.contains(CpuFlags::ZERO));
    }

    #[test]
    fn test_0xaa_tax() {
        let mut cpu = CPU::new();
        cpu.load_and_run(vec![0xa9,0x05,0xaa, 0x00]);
        assert_eq!(cpu.register_x, 0x05);
        assert!(!cpu.status.contains(CpuFlags::ZERO));
        assert!(!cpu.status.contains(CpuFlags::NEGATIV));
    }

    #[test]
    fn test_5_ops_working_together() {
        let mut cpu = CPU::new();
        cpu.load_and_run(vec![0xa9, 0x05, 0xaa, 0xa9, 0x08, 0x00]);
        assert_eq!(cpu.register_a, 0x08);
        assert_eq!(cpu.register_x, 0x05);
    }

    #[test]
    fn test_inx_overflow() {
        let mut cpu = CPU::new();
        cpu.load_and_run(vec![0xa9, 0xff, 0xaa, 0xe8, 0x00]);

        assert_eq!(cpu.register_x, 0)
    }

    #[test]
    fn test_lda_from_memory() {
        let mut cpu = CPU::new();
        cpu.mem_write(0x10, 0x55);
        cpu.load_and_run(vec![0xa5, 0x10, 0x00]);

        assert_eq!(cpu.register_a, 0x55)
    }

    #[test]
    fn test_0x29_and() {
        let mut cpu = CPU::new();
        cpu.load_and_run(vec![0xa9, 0x01, 0x29, 0x2]);
        assert_eq!(cpu.register_a, 0x0);
        cpu.reset();
        cpu.load_and_run(vec![0xa9, 0x01, 0x29, 0x1]);
        assert_eq!(cpu.register_a, 0x1);
    }

    #[test]
    fn test_0x49_eor() {
        let mut cpu = CPU::new();
        cpu.load_and_run(vec![0xa9, 0x01, 0x49, 0x2]);
        assert_eq!(cpu.register_a, 0x3);
        cpu.reset();
        cpu.load_and_run(vec![0xa9, 0xf0, 0x49, 0x0f]);
        assert_eq!(cpu.register_a, 0xff);
    }

    #[test]
    fn test_0x09_eor() {
        let mut cpu = CPU::new();
        cpu.load_and_run(vec![0xa9, 0x01, 0x09, 0x00]);
        assert_eq!(cpu.register_a, 0x1);
        cpu.reset();
        cpu.load_and_run(vec![0xa9, 0xff, 0x09, 0x00]);
        assert_eq!(cpu.register_a, 0xff);
    }

    #[test]
    fn test_0xe9_sbc() {
        let mut cpu = CPU::new();
        cpu.load_and_run(vec![0xa9, 0x05, 0xe9, 0x03, 0x00]);
        assert_eq!(cpu.register_a, 0x1);
        cpu.reset();
        cpu.load_and_run(vec![0xa9, 0x00, 0xe9, 0x01, 0x00]);
        assert_eq!(cpu.register_a, 0xfe);
    }

    #[test]
    fn test_0x69_adc() {
        let mut cpu = CPU::new();
        cpu.load_and_run(vec![0xa9, 0x0f, 0x69, 0x01, 0x00]);
        assert_eq!(cpu.register_a, 0x0f + 0x1);
        cpu.reset();
        cpu.load_and_run(vec![0xa9, 0xef, 0x69, 0x01, 0x00]);
        assert_eq!(cpu.register_a, 0xef + 0x01);
    }

    #[test]
    fn test_0x0a_asl() {
        let mut cpu = CPU::new();
        cpu.load_and_run(vec![0xa9, 0x0f, 0x0a, 0x00]);
        assert_eq!(cpu.register_a, 0x0f << 1);
    }

    #[test]
    fn test_0x4a_lsr() {
        let mut cpu = CPU::new();
        cpu.load_and_run(vec![0xa9, 0x0f, 0x4a, 0x00]);
        assert_eq!(cpu.register_a, 0x0f >> 1);
        assert!(cpu.status.contains(CpuFlags::CARRY));
    }

    #[test]
    fn test_0x2a_rol() {
        let mut cpu = CPU::new();
        cpu.load_and_run(vec![0xa9, 0x0f, 0x2a, 0x00]);
        assert_eq!(cpu.register_a, 0x0f << 1);
    }

    #[test]
    fn test_0x6a_rol() {
        let mut cpu = CPU::new();
        cpu.load_and_run(vec![0xa9, 0x0f, 0x6a, 0x00]);
        assert_eq!(cpu.register_a, 0x0f >> 1);
    }
}

