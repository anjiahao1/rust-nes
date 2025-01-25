use crate::cpu::Mem;
use crate::rom::Rom;

const RAM: u16 = 0x0000;
const RAM_MIRRORS_END: u16 = 0x1FFF;
const PPU_REGISTERS: u16 = 0x2000;
const PPU_REGISTERS_MIRRORS_END: u16 = 0x3FFF;

pub struct Bus {
    cpu_vram: [u8; 2048],
    rom: Rom,
}

impl Bus {
    pub fn new() -> Self {
        Self {
            cpu_vram: [0; 2048],
            rom: Rom::new(&vec![]).unwrap(),
        }
    }

    fn read_prg_rom(&self, mut addr: u16) -> u8 {
        addr -= 0x8000;
        if self.rom.prg_rom.len() == 0x4000 && addr >= 0x4000 {
            addr = addr % 0x4000;
        }

        self.rom.prg_rom[addr as usize]
    }
}

impl Mem for Bus {
    fn mem_read(&self, addr: u16) -> u8 {
        match addr {
            RAM ..= RAM_MIRRORS_END => {
                let mirror_donw_addr = addr & 0b0000_0111_1111_1111;
                self.cpu_vram[mirror_donw_addr as usize]
            },
            PPU_REGISTERS ..= PPU_REGISTERS_MIRRORS_END => {
                println!("PPU registers not implemented yet");
                0
            },
            0x8000..=0xFFFF => {
                self.read_prg_rom(addr)
            }
            _ => {
                println!("Address not implemented yet");
                0
            },
        }
    }
    fn mem_write(&mut self, addr: u16, data: u8) {
        match addr {
            RAM ..= RAM_MIRRORS_END => {
                let mirror_donw_addr = addr & 0b0000_0111_1111_1111;
                self.cpu_vram[mirror_donw_addr as usize] = data;
            },
            PPU_REGISTERS ..= PPU_REGISTERS_MIRRORS_END => {
                println!("PPU registers not implemented yet");
            },
            _ => {
                println!("Address not implemented yet");
            },
        }
    }
}
