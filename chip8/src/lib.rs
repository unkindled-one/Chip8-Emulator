use rand::random;

const SCREEN_WIDTH: usize = 64;
const SCREEN_HEIGHT: usize = 32;
const MEMORY_SIZE: usize = 4096;


pub struct Chip8 {
    // Can loop in here or in emulator
    program_counter: u16,
    /// The memory of the program. The actual program starts at 0x200.
    memory: [u8; MEMORY_SIZE],
    /// The general purpose registers
    registers: [u8; 16],

    index_register: u16,
    /// Decremented 60 times/second, used for timing.
    delay_timer: u8,
    /// Plays a tone as long as the value is not zero, decremented 60 times/second.
    sound_timer: u8,
    /// Stores the information of each pixel on the screen.
    pub display: [u8; SCREEN_WIDTH * SCREEN_HEIGHT],
    /// Program stack, used for recursion and generally has a max length of 16 
    stack: Vec<u16> 
}

impl Chip8 {
    // TODO: put font in memory when interpreter is started
    /// Load the font into memory starting at byte 0x50 (by convention).
    fn initialize_font(memory: &mut [u8; MEMORY_SIZE]) {
        // Source: https://tobiasvl.github.io/blog/write-a-chip-8-emulator/#display
        let font: [u8; 80] = [0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
            0x20, 0x60, 0x20, 0x20, 0x70, // 1
            0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
            0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
            0x90, 0x90, 0xF0, 0x10, 0x10, // 4
            0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
            0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
            0xF0, 0x10, 0x20, 0x40, 0x40, // 7
            0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
            0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
            0xF0, 0x90, 0xF0, 0x90, 0x90, // A
            0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
            0xF0, 0x80, 0x80, 0x80, 0xF0, // C
            0xE0, 0x90, 0x90, 0x90, 0xE0, // D
            0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
            0xF0, 0x80, 0xF0, 0x80, 0x80  // F
        ];
        for (i, byte) in font.iter().enumerate() {
            memory[0x50 + i] = *byte;
        }
    }

    /// Initializes the Chip8 Interpreter.
    pub fn new() -> Self {
        let mut memory: [u8; MEMORY_SIZE] = [0; MEMORY_SIZE];
        Self::initialize_font(&mut memory);
        Chip8 {
            program_counter: 0x200, // start of the program
            memory,
            registers: [0; 16],
            index_register: 0,
            delay_timer: 60, // 60hz 
            sound_timer: 60,
            display: [0; SCREEN_WIDTH * SCREEN_HEIGHT],
            stack: Vec::new() // Unbounded stack for convenience 
        }
    }
    
    /// Loads a chip8 program into memory.
    pub fn load(&mut self, data: &Vec<u8>) {
        if data.len() > (MEMORY_SIZE - 0x200) {
            panic!("Program too large to fit into memory.");
        }
        for (i, byte) in data.iter().enumerate() {
            self.memory[0x200 + i] = *byte;
        }
    }

    /// Goes through the fetch, decode, execute cycle once.
    pub fn step(&mut self) {
        let byte1 = self.memory[self.program_counter as usize];
        let byte2 = self.memory[(self.program_counter as usize) + 1];
        self.program_counter += 2;

        let instruction = (
            byte1 >> 4,
            byte1 & 0xf,
            byte2 >> 4,
            byte2 & 0xf
        );

        match instruction {
            (0x0, 0x0, 0xE, 0x0) => { 
                self.clear_screen();
            },
            (0x1, nib1, nib2, nib3) => { // Unconditional jump
                self.program_counter = Self::combine_nibbles(nib1, nib2, nib3);
            }, 
            (0x2, nib1, nib2, nib3) => { // Enter a subroutine
                self.stack.push(self.program_counter);
                self.program_counter = Self::combine_nibbles(nib1, nib2, nib3);
            },
            (0x0, 0x0, 0xE, 0xE) => { // Return from subroutine
                self.program_counter = self.stack.pop().expect("Attempted to return from subroutine on empty stack.");
            }, 
            (0x3, reg, _, _) => { // Skip inst. if reg == byte2 
                if self.registers[reg as usize] == byte2 {
                    self.program_counter += 2;
                }
            },
            (0x4, reg, _, _) => { // Skip isnt. if reg != byte2 
                if self.registers[reg as usize] != byte2 {
                    self.program_counter += 2;
                }
            },
            (0x5, reg1, reg2, _) => { // Skip inst. if reg1 == reg2
                if self.registers[reg1 as usize] == self.registers[reg2 as usize] {
                    self.program_counter += 2;
                }
            },
            (0x9, reg1, reg2, _) => { // Skip inst. if reg1 != reg2 
                if self.registers[reg1 as usize] != self.registers[reg2 as usize] {
                    self.program_counter += 2;
                }
            },
            (0x6, reg, _, _) => { // Set reg to byte2
                self.registers[reg as usize] = byte2;
            },
            (0x7, reg, _, _) => { // Add byte2 to reg 
                self.registers[reg as usize] += byte2;
            },
            (0x8, reg1, reg2, 0x0) => { // Set reg1 to reg2 
                self.registers[reg1 as usize] = self.registers[reg2 as usize];
            },
            (0x8, reg1, reg2, 0x1) => { // reg1 = reg1 | reg2
                self.registers[reg1 as usize] |= self.registers[reg2 as usize];
            },
            (0x8, reg1, reg2, 0x2) => { // reg1 = reg1 & reg2
                self.registers[reg1 as usize] &= self.registers[reg2 as usize];
            },
            (0x8, reg1, reg2, 0x3) => { // reg1 = reg1 ^ reg2
                self.registers[reg1 as usize] ^= self.registers[reg2 as usize];
            },
            (0x8, reg1, reg2, 0x4) => { // reg1 = reg1 + reg2
                let val1 = self.registers[reg1 as usize];
                let val2 = self.registers[reg2 as usize];
                self.registers[reg1 as usize] = val1 + val2;
                match val1.checked_add(val2) {
                    Some(_) => self.registers[0xf] = 0,
                    None =>  self.registers[0xf] = 1
                }
            },
            (0x8, reg1, reg2, 0x5) => { // reg1 = reg1 - reg2, VF = reg1 > reg2
                let val1 = self.registers[reg1 as usize];
                let val2 = self.registers[reg2 as usize];
                if val1 > val2 {
                    self.registers[0xf] = 1;
                } else {
                    self.registers[0xf] = 0;
                }
                self.registers[reg1 as usize] = val1 - val2;
            },
            (0x8, reg1, _, 0x6) => { // reg1 = reg1 >> 1, VF = reg1 & 1
                // TODO: Add option to set reg1 to reg2
                self.registers[0xf] = self.registers[reg1 as usize] & 1;
                self.registers[reg1 as usize] >>= 1;
            },
            (0x8, reg1, reg2, 0x7) => { // reg1 = reg2 - reg1, VF = reg2 > reg1
                let val1 = self.registers[reg1 as usize];
                let val2 = self.registers[reg2 as usize];
                if val2 > val1 {
                    self.registers[0xf] = 1;
                } else {
                    self.registers[0xf] = 0;
                }
                self.registers[reg1 as usize] = val2 - val1;
            },
            (0x8, reg1, _, 0xe) => { // reg1 = reg1 << 1, VF = reg1 & (1 << 8)
                // TODO: Add option to set reg1 to reg2
                self.registers[0xf] = self.registers[reg1 as usize] & (1 << 8);
                self.registers[reg1 as usize] <<= 1;
            },
            (0xa, nib1, nib2, nib3) => { // IndexRegister = NNN
                self.index_register = Self::combine_nibbles(nib1, nib2, nib3);
            },
            (0xb, nib1, nib2, nib3) => { // Jump to NNN + V0
                // TODO: Add option to allow BXNN (maybe)
                self.program_counter = Self::combine_nibbles(nib1, nib2, nib3) + self.registers[0] as u16;
            },
            (0xc, reg, _, _) => { // reg = rand & byte2
                let rand_value: u8 = rand::random::<u8>();
                self.registers[reg as usize] = rand_value & byte2;
            }
            (0x0, _, _, _) => {}, // Do nothing, for compatibility.
            (_, _, _, _) => panic!("ERROR: Instruction {:?} not implemented.", instruction),
        }
    }

    /// Sets all the display pixels to 0.
    fn clear_screen(&mut self) {
        for i in 0..self.display.len() {
            self.display[i] = 0;
        }
    }

    /// Combines 3 nibbles into one u16, top 4 bytes empty.
    fn combine_nibbles(nib1: u8, nib2: u8, nib3: u8) -> u16 {
        let mut res: u16 = 0;
        res |= (nib1 as u16) << 8;
        res |= (nib2 as u16) << 4;
        res |= nib3 as u16;
        res
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_font_init() {
        let emu = Chip8::new();
        // Source: https://tobiasvl.github.io/blog/write-a-chip-8-emulator/#display
        let font: [u8; 80] = [0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
            0x20, 0x60, 0x20, 0x20, 0x70, // 1
            0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
            0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
            0x90, 0x90, 0xF0, 0x10, 0x10, // 4
            0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
            0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
            0xF0, 0x10, 0x20, 0x40, 0x40, // 7
            0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
            0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
            0xF0, 0x90, 0xF0, 0x90, 0x90, // A
            0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
            0xF0, 0x80, 0x80, 0x80, 0xF0, // C
            0xE0, 0x90, 0x90, 0x90, 0xE0, // D
            0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
            0xF0, 0x80, 0xF0, 0x80, 0x80  // F
        ];
        assert_eq!(emu.memory[0x50..=0x9f], font);
    }

    #[test]
    fn load_program() {
        let mut emu = Chip8::new();
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
        emu.load(&data);
        assert_eq!(emu.memory[0x200..=0x200+data.len()-1], data);
    }

    #[test]
    #[should_panic]
    fn too_large_program() {
        let mut emu = Chip8::new();
        let data = vec![0; 10000];
        emu.load(&data);
    }

    #[test]
    fn clear_screen() {
        let mut emu = Chip8::new();
        emu.display = [1; SCREEN_HEIGHT * SCREEN_WIDTH];
        emu.clear_screen();
        assert_eq!(emu.display, [0; SCREEN_HEIGHT * SCREEN_WIDTH]);
    }
}
