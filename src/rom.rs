pub enum Mirroring {
    Horizontal,
    Vertical,
    FourScreen,
}

const NES_MAGIC: [u8; 4] = [0x4e, 0x45, 0x53, 0x1a];

pub struct Rom {
   pub prg_rom: Vec<u8>,
   pub chr_rom: Vec<u8>,
   pub mapper: u8,
   pub screen_mirroring: Mirroring,
}

impl Rom {
    pub fn new(raw: &Vec<u8>) -> Result<Self, String> {
        if raw[0..4] != NES_MAGIC {
            return Err("Invalid NES magic number".to_owned())
        }

        let mapper = (raw[6] & 0xf0) | (raw[7] >> 4);
        let ines_version = raw[7] & 0x0f;
        if ines_version != 0 {
            return Err("Only iNES version 0 is supported".to_owned())
        }

        let four_screen = raw[6] & 0x08 != 0;
        let vertical_mirroring = raw[6] & 0x01 != 0;
        let screen_mirroring = match (four_screen, vertical_mirroring) {
            (true, _) => Mirroring::FourScreen,
            (false, true) => Mirroring::Vertical,
            (false, false) => Mirroring::Horizontal,
        };

        let prg_rom_size = raw[4] as usize * 0x4000;
        let chr_rom_size = raw[5] as usize * 0x2000;

        let sikp_trainer = raw[6] & 0x04 != 0;
        let prg_rom_start = 16 + if sikp_trainer { 512 } else { 0 };
        let prg_rom_end = prg_rom_start + prg_rom_size;

        let chr_rom_start = prg_rom_end;
        let chr_rom_end = chr_rom_start + chr_rom_size;

        Ok(Self {
            prg_rom: raw[prg_rom_start..prg_rom_end].to_vec(),
            chr_rom: raw[chr_rom_start..chr_rom_end].to_vec(),
            mapper,
            screen_mirroring,
        })
    }
}
