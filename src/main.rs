pub struct CPU {
    pub register_a : u8,
    pub register_x : u8,
    pub status : u8,
    pub program_count : u16,
    mem: [u8; 0xFFFF]
}

impl CPU {
    pub fn new () -> Self {
        CPU {
            register_a : 0,
            register_x : 0,
            status : 0,
            program_count : 0,
            mem: [0; 0xFFFF]
        }
    }

    fn mem_read(&self, addr: u16) -> u8{
        self.mem[addr as usize]
    }

    fn mem_write(&mut self, addr: u16, value: u8) {
        self.mem[addr as usize] = value;
    }

    fn mem_read_u16 (&self, pos: u16) -> u16 {
        let lo = self.mem_read(pos) as u16;
        let hi = self.mem_read(pos + 1) as u16;
        (hi << 8) | lo
    }

    fn mem_write_u16 (&mut self, pos: u16, value: u16) {
        let lo = value as u8;
        let hi = (value >> 8) as u8;
        self.mem_write(pos, lo);
        self.mem_write(pos + 1, hi);
    }

    pub fn reset (&mut self) {
        self.register_a = 0;
        self.register_x = 0;
        self.status = 0;
        self.program_count = self.mem_read_u16(0xFFFC);
    }

    pub fn load(&mut self, program: Vec<u8>) {
        self.mem[0x8000 .. (0x8000 + program.len())].copy_from_slice(&program[..]);
        self.mem_write_u16(0xFFFC, 0x8000)
    }

    pub fn run (&mut self) {
        loop {
            let opscode = self.mem_read(self.program_count);
            self.program_count += 1;
            match opscode {
                0xa9 => {
                    let param = self.mem_read(self.program_count);
                    self.program_count += 1;
                    self.lda(param)
                }
                0xaa => self.tax(),
                0x00 => break,
                _ => todo!()
            }
        }
    }

    pub fn load_and_run (&mut self, program: Vec<u8>) {
        self.load(program);
        self.reset();
        self.run();
    }

    fn lda(&mut self, value : u8) {
        self.register_a = value;
        self.update_zero_and_negative_flags(self.register_a);
    }

    fn tax(&mut self) {
        self.register_x = self.register_a;
        self.update_zero_and_negative_flags(self.register_x);
    }

    fn update_zero_and_negative_flags (&mut self, result: u8) {
        if result == 0 {
            self.status = self.status | 0b0000_0010;
        } else {
            self.status = self.status & 0b1111_1101;
        }

        if result & 0b1000_0000 != 0 {
            self.status = self.status | 0b1000_0000;
        } else {
            self.status = self.status & 0b0111_1111;
        }
    }

    pub fn interpret (&mut self, program: Vec<u8>) {
        self.program_count = 0;
        loop {
            let opscode = program[self.program_count as usize];
            self.program_count += 1;

            match opscode {
                0xa9 => {
                    let param = program[self.program_count as usize];
                    self.program_count += 1;
                    self.lda(param)
                }

                0xaa => self.tax(),
                0x00 => break,
                _ => todo!()
            }
        }
    }
}


#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_0xa9_lda_immidiate_load_data() {
        let mut cpu = CPU::new();
        cpu.interpret(vec![0xa9, 0x05, 0x00]);
        assert_eq!(cpu.register_a, 0x05);
        assert!(cpu.status & 0b0000_0010 == 0b00);
        assert!(cpu.status & 0b1000_0000 == 0);
    }

    #[test]
    fn test_0xa9_lda_zero_flag() {
        let mut cpu = CPU::new();
        cpu.interpret(vec![0xa9, 0x00, 0x00]);
        assert_eq!(cpu.register_a, 0x00);
        assert!(cpu.status & 0b0000_0010 == 0b10);
    }

    #[test]
    fn test_0xaa_tax() {
        let mut cpu = CPU::new();
        cpu.register_a = 0x05;
        cpu.interpret(vec![0xaa, 0x00]);
        assert_eq!(cpu.register_x, 0x05);
        assert!(cpu.status & 0b0000_0010 == 0b00);
        assert!(cpu.status & 0b1000_0000 == 0);
    }

    #[test]
    fn test_5_ops_working_together() {
        let mut cpu = CPU::new();
        cpu.interpret(vec![0xa9, 0x05, 0xaa, 0xa9, 0x08, 0x00]);
        assert_eq!(cpu.register_a, 0x08);
        assert_eq!(cpu.register_x, 0x05);
    }
}

fn main() {
    println!("Hello, world!");
}
