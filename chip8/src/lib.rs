use std::io;
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
    /// Whether the display needs to be redrawn.
    needs_redraw: bool,
    /// Holds index for program.
    index_register: u16,
    /// Decremented 60 times/second, used for timing.
    delay_timer: u8,
    /// Plays a tone as long as the value is not zero, decremented 60 times/second.
    sound_timer: u8,
    /// Stores the information of each pixel on the screen.
    display: [bool; SCREEN_WIDTH * SCREEN_HEIGHT],
    /// Stores the information on the keys that is being pressed.
    keyboard: [bool; 16],
    /// Program stack, used for recursion and generally has a max length of 16 
    stack: Vec<u16> 
}

impl Chip8 {
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
            needs_redraw: false,
            index_register: 0,
            delay_timer: 60, // 60hz 
            sound_timer: 60,
            display: [false; SCREEN_WIDTH * SCREEN_HEIGHT],
            keyboard: [false; 16],
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

    /// Returns the display.
    pub fn get_display(&self) -> &[bool] {
        return &self.display;
    }

    /// Resets the execution
    pub fn reset(&mut self) {
        self.program_counter = 0x200;
        self.display = [false; SCREEN_HEIGHT * SCREEN_WIDTH];
        let mut memory = [0; MEMORY_SIZE];
        Self::initialize_font(&mut memory);
        self.memory = memory;
        self.registers = [0; 16];
        self.needs_redraw = false;
        self.index_register = 0;
        self.delay_timer = 60; // 60hz 
        self.sound_timer = 60;
        self.keyboard = [false; 16];
        self.stack = Vec::new(); // Unbounded stack for convenience 
        self.needs_redraw = true;
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
            (0x0, 0x0, 0xE, 0x0) => { // 00E0
                self.clear_screen();
                self.needs_redraw = true;
            },
            (0x1, nib1, nib2, nib3) => { // 1NNN = Unconditional jump
                self.program_counter = Self::combine_nibbles(nib1, nib2, nib3);
            }, 
            (0x2, nib1, nib2, nib3) => { // 2NNN = Enter a subroutine
                self.stack.push(self.program_counter);
                self.program_counter = Self::combine_nibbles(nib1, nib2, nib3);
            },
            (0x0, 0x0, 0xE, 0xE) => { // 00EE = Return from subroutine
                self.program_counter = self.stack.pop().expect("Attempted to return from subroutine on empty stack.");
            }, 
            (0x3, reg, _, _) => { // 3XNN = Skip inst. if reg == byte2 
                if self.registers[reg as usize] == byte2 {
                    self.program_counter += 2;
                }
            },
            (0x4, reg, _, _) => { // 4XNN = Skip isnt. if reg != byte2 
                if self.registers[reg as usize] != byte2 {
                    self.program_counter += 2;
                }
            },
            (0x5, reg1, reg2, 0x0) => { // 5XY0 = Skip inst. if reg1 == reg2
                if self.registers[reg1 as usize] == self.registers[reg2 as usize] {
                    self.program_counter += 2;
                }
            },
            (0x9, reg1, reg2, 0x0) => { // 9XY0 = Skip inst. if reg1 != reg2 
                if self.registers[reg1 as usize] != self.registers[reg2 as usize] {
                    self.program_counter += 2;
                }
            },
            (0x6, reg, _, _) => { // 6XNN = Set reg to byte2
                self.registers[reg as usize] = byte2;
            },
            (0x7, reg, _, _) => { // 7XNN = Add byte2 to reg 
                self.registers[reg as usize] = self.registers[reg as usize].wrapping_add(byte2);
            },
            (0x8, reg1, reg2, 0x0) => { // 8XY0 = Set reg1 to reg2 
                self.registers[reg1 as usize] = self.registers[reg2 as usize];
            },
            (0x8, reg1, reg2, 0x1) => { // 8XY1 = reg1 = reg1 | reg2
                self.registers[reg1 as usize] |= self.registers[reg2 as usize];
            },
            (0x8, reg1, reg2, 0x2) => { // 8XY2 = reg1 = reg1 & reg2
                self.registers[reg1 as usize] &= self.registers[reg2 as usize];
            },
            (0x8, reg1, reg2, 0x3) => { // 8XY3 = reg1 = reg1 ^ reg2
                self.registers[reg1 as usize] ^= self.registers[reg2 as usize];
            },
            (0x8, reg1, reg2, 0x4) => { // 8XY4 = reg1 = reg1 + reg2
                let val1 = self.registers[reg1 as usize];
                let val2 = self.registers[reg2 as usize];
                let (value, did_overflow) = val1.overflowing_add(val2);
                if did_overflow {
                    self.registers[0xf] = 1;
                } else {
                    self.registers[0xf] = 0;
                }
                self.registers[reg1 as usize] = value;
            },
            (0x8, reg1, reg2, 0x5) => { // 8XY5 = reg1 = reg1 - reg2, VF = reg1 > reg2
                let val1 = self.registers[reg1 as usize];
                let val2 = self.registers[reg2 as usize];
                let (value, did_underflow) = val1.overflowing_sub(val2);

                if did_underflow {
                    self.registers[0xf] = 1;
                } else {
                    self.registers[0xf] = 0;
                }
                self.registers[reg1 as usize] = value;
            },
            (0x8, reg1, _, 0x6) => { // 8XY6 = reg1 = reg1 >> 1, VF = reg1 & 1
                // TODO: Add option to set reg1 to reg2
                self.registers[0xf] = self.registers[reg1 as usize] & 1;
                self.registers[reg1 as usize] >>= 1;
            },
            (0x8, reg1, reg2, 0x7) => { // 8XY7 = reg1 = reg2 - reg1, VF = reg2 > reg1
                 // 8XY5 = reg1 = reg1 - reg2, VF = reg1 > reg2
                let val1 = self.registers[reg1 as usize];
                let val2 = self.registers[reg2 as usize];
                let (value, did_underflow) = val2.overflowing_sub(val1);

                if did_underflow {
                    self.registers[0xf] = 1;
                } else {
                    self.registers[0xf] = 0;
                }
                self.registers[reg1 as usize] = value;
            },
            (0x8, reg1, _, 0xe) => { // 8XYE = reg1 = reg1 << 1, VF = reg1 & (1 << 8)
                // TODO: Add option to set reg1 to reg2
                self.registers[0xf] = self.registers[reg1 as usize] & (1 << 7);
                self.registers[reg1 as usize] <<= 1;
            },
            (0xa, nib1, nib2, nib3) => { //  ANNN = IndexRegister = NNN
                self.index_register = Self::combine_nibbles(nib1, nib2, nib3);
            },
            (0xb, nib1, nib2, nib3) => { // BNNN =  Jump to NNN + V0
                // TODO: Add option to allow BXNN (maybe)
                self.program_counter = Self::combine_nibbles(nib1, nib2, nib3) + self.registers[0] as u16;
            },
            (0xc, reg, _, _) => { // reg = rand & byte2
                let rand_value: u8 = rand::random::<u8>();
                self.registers[reg as usize] = rand_value & byte2;
            },
            (0xd, reg1, reg2, num_bytes) => { // DXYN = Changes the display
                self.needs_redraw = true;
                let x_pos: u8 = self.registers[reg1 as usize] % (SCREEN_WIDTH as u8);
                let y_pos: u8 = self.registers[reg2 as usize] % (SCREEN_HEIGHT as u8);
                let mut flipped = false; // Check if any pixel was flipped

                for row_num in 0..num_bytes {
                    let pixels = self.memory[(self.index_register + row_num as u16) as usize];
                    // stop writing when reaching bottom of screen
                    if y_pos >= SCREEN_HEIGHT as u8 {
                        break;
                    }
                    for sprite_pos in 0..8 {
                        // stop writing when reaching edge of screen
                        if x_pos >= SCREEN_WIDTH as u8 {
                            break;
                        }
                        let sprite_pixel = (pixels & (0b10000000 >> sprite_pos)) != 0;
                        let index = ((x_pos + sprite_pos) as usize) + ((y_pos + row_num) as usize) * SCREEN_WIDTH;
                        flipped |= self.display[index as usize] != sprite_pixel;
                        self.display[index as usize] ^= sprite_pixel;
                    }
                }
                if flipped {
                    self.registers[0xf] = 1;
                } else {
                    self.registers[0xf] = 0;
                }
            }, 
            (0xe, reg, 0x9, 0xe) => { // EX9E = Skip if key in reg is pressed 
                if self.keyboard[self.registers[reg as usize] as usize] {
                    self.program_counter += 2;
                }
            }, 
            (0xe, reg, 0xa, 0x1) => { // EXA1 = Skip is key in reg is not pressed
                if !self.keyboard[self.registers[reg as usize] as usize] {
                    self.program_counter += 2;
                }
            },
            (0xf, reg, 0x0, 0x7) => { // FX07 = Sets the reg to delay timer
                self.registers[reg as usize] = self.delay_timer;
            },
            (0xf, reg, 0x1, 0x5) => { // FX15
                self.delay_timer = self.registers[reg as usize];
            },
            (0xf, reg, 0x1, 0x8) => { // FX18
                self.sound_timer = self.registers[reg as usize];
            },
            (0xf, reg, 0x1, 0xe) => { // FX1E
                self.index_register = self.index_register.wrapping_add(self.registers[reg as usize] as u16);
            },
            (0xf, reg, 0x0, 0xa) => { // FX0A
                let mut any_pressed = false;
                for (i, key) in self.keyboard.iter().enumerate() {
                    if *key {
                        self.registers[reg as usize] = i as u8;
                        any_pressed = true;
                    }
                }
                if !any_pressed { // loop until key is pressed
                    self.program_counter -= 2;
                }
           },
            (0xf, reg, 0x2, 0x9) => { // Fx29 = Sets I reg to the font in vx
                let x = reg as usize;
                let c = self.registers[x] as u16;
                self.index_register = c * 5;
            },
            (0xf, reg, 0x3, 0x3) => { // FX33 = Stores the digits of num in reg at the address in I
                let num = self.registers[reg as usize];
                self.memory[self.index_register as usize] = num / 100;
                self.memory[(self.index_register + 1) as usize] = (num / 10) % 10;
                self.memory[(self.index_register + 2) as usize] = num % 10;
            },
            (0xf, reg, 0x5, 0x5) => { // Fx55 = Load into memory from reg at address I
                // TODO: Add option for older behavior potentially.
                let i_reg_value = self.index_register as usize;
                let x = reg as usize;
                for i in 0..=x {
                    self.memory[i_reg_value + i] = self.registers[i];
                }
            },
            (0xf, reg, 0x6, 0x5) => { // FX65 = Load into reg from memory at address I
                let i_reg_value = self.index_register as usize;
                let x = reg as usize;
                for i in 0..=x {
                    self.registers[i] = self.memory[i_reg_value + i];
                }
            }

            (0x0, _, _, _) => {}, // Do nothing, for compatibility.
            (_, _, _, _) => unimplemented!("ERROR: Instruction {:?} not implemented.", instruction),
        }
    }

    /// Decrements both the delay and the sound timers. Does not reset after they reach 0, that is
    /// the responsibility of the program. 
    pub fn tick_timers(&mut self) {
        if self.sound_timer > 0 {
            self.sound_timer -= 1;
        }
        if self.delay_timer > 0 {
            self.delay_timer -= 1;
        }
    }

    /// Sets all the display pixels to 0. 
    fn clear_screen(&mut self) {
        for i in 0..self.display.len() {
            self.display[i] = false;
        }
    }

    /// A number 0-15 that marks the position on the control grid. Allows the frontend to choose the key mappings.
    pub fn press_key(&mut self, key_num: u8) {
        if key_num > 0xf { // Invalid key entered, ignore
            return; 
        }
        self.keyboard[key_num as usize] = true;
    }

    /// Unpresses the specified key.
    pub fn unpress_key(&mut self, key_num: u8) {
        if key_num > 0xf {
            return;
        }
        self.keyboard[key_num as usize] = false;
    }

    /// Sets the needs_redraw flag to false.
    pub fn was_redrawn(&mut self) {
        self.needs_redraw = false;
    }
    
    pub fn needs_redraw(&self) -> bool {
        return self.needs_redraw;
    }

    /// Combines 3 nibbles into one u16, top 4 bits empty.
    fn combine_nibbles(nib1: u8, nib2: u8, nib3: u8) -> u16 {
        let mut res: u16 = 0;
        res |= ((nib1 & 0xf) as u16) << 8;
        res |= ((nib2 & 0xf) as u16) << 4;
        res |= (nib3 & 0xf) as u16;
        res
    }
}

fn pause() {
    io::stdin().read_line(&mut String::new()).unwrap();
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
        emu.display = [true; SCREEN_HEIGHT * SCREEN_WIDTH];
        emu.clear_screen();
        assert_eq!(emu.display, [false; SCREEN_HEIGHT * SCREEN_WIDTH]);
    }

    #[test]
    fn jump() {
        let mut emu = Chip8::new();
        let data = vec![0x11, 0x11]; // Jump to 111
        emu.load(&data);
        emu.step();
        assert_eq!(emu.program_counter, 0x111);
    }

    #[test]
    fn draw_sprite() {
        unimplemented!();
    }

    #[test]
    fn load_from_memory() {
        unimplemented!();
    }

    #[test]
    fn load_to_memory() {
        unimplemented!();
    }

    // TODO: Write tests for the rest of the instructions
}
