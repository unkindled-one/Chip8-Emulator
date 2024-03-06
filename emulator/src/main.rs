use chip8::Chip8;
use softbuffer::Surface;
use std::num::NonZeroU32;
use std::rc::Rc;
use std::{env, fs};
use std::time::{Duration, Instant};
use std::thread::sleep;
use winit::dpi::LogicalSize;
use winit::event::{Event, WindowEvent, ElementState};
use winit::event_loop::{ControlFlow, EventLoop, EventLoopWindowTarget};
use winit::window::{Window, WindowBuilder};
use winit::keyboard::{Key, SmolStr};
use winit::keyboard::NamedKey;

const SCALE: usize = 15; // 15x scale to the display
                         // const SCALE: usize = 1; // 15x scale to the display
const SCREEN_WIDTH: usize = 64;
const SCREEN_HEIGHT: usize = 32;
const SCALED_WIDTH: usize = 64 * SCALE;
const SCALED_HEIGHT: usize = 32 * SCALE;
const TICKS_PER_FRAME: u8 = 10;

fn draw_screen(surface: &mut Surface<Rc<Window>, Rc<Window>>, emulator: &mut Chip8) {
    let mut buffer = surface.buffer_mut().unwrap();
    let display = emulator.get_display();
    let dark_gray = 0x3a3b3c;
    let light_gray = 0xb0b3b8;

    for (index, pixel) in display.iter().enumerate() {
        let x = index % SCREEN_WIDTH;
        let y = index / SCREEN_WIDTH;

        let value = if *pixel { dark_gray } else { light_gray };

        for sy in 0..SCALE {
            for sx in 0..SCALE {
                let scaled_y = y * SCALE + sy;
                let scaled_x = x * SCALE + sx;

                let index = scaled_y * SCALED_WIDTH + scaled_x;
                buffer[index] = value;
            }
        }
    }

    emulator.was_redrawn();
    buffer.present().unwrap();
}

/// Handles a keypress, returns whether the application should exit.
fn handle_key(state: ElementState, key: Key, emulator: &mut Chip8, rom: &Vec<u8>) -> bool {
    match state {
        ElementState::Pressed => {
            match key {
                Key::Named(named_key) => {
                    match named_key {
                        NamedKey::Escape => {
                            return true;
                        },
                        NamedKey::F5 => {
                            emulator.reset();
                            emulator.load(rom);
                        }
                        _ => {}
                    }
                },
                Key::Character(character) => {
                    match character.as_str() {
                        "1" => emulator.press_key(0),
                        "2" => emulator.press_key(1),
                        "3" => emulator.press_key(2),
                        "4" => emulator.press_key(3),
                        "q" | "Q" => emulator.press_key(4),
                        "w" | "W" => emulator.press_key(5),
                        "e" | "E" => emulator.press_key(6),
                        "r" | "R" => emulator.press_key(7),
                        "a" | "A" => emulator.press_key(8),
                        "s" | "S" => emulator.press_key(9),
                        "d" | "D" => emulator.press_key(10),
                        "f" | "F" => emulator.press_key(11),
                        "z" | "Z" => emulator.press_key(12),
                        "x" | "X" => emulator.press_key(13),
                        "c" | "C" => emulator.press_key(14),
                        "v" | "V" => emulator.press_key(15),
                        _ => ()
                    }

                },
                _ => {}
            }
        },
        ElementState::Released => {
            match key {
                Key::Character(character) => {
                    match character.as_str() {
                        "1" => emulator.unpress_key(0),
                        "2" => emulator.unpress_key(1),
                        "3" => emulator.unpress_key(2),
                        "4" => emulator.unpress_key(3),
                        "q" | "Q" => emulator.unpress_key(4),
                        "w" | "W" => emulator.unpress_key(5),
                        "e" | "E" => emulator.unpress_key(6),
                        "r" | "R" => emulator.unpress_key(7),
                        "a" | "A" => emulator.unpress_key(8),
                        "s" | "S" => emulator.unpress_key(9),
                        "d" | "D" => emulator.unpress_key(10),
                        "f" | "F" => emulator.unpress_key(11),
                        "z" | "Z" => emulator.unpress_key(12),
                        "x" | "X" => emulator.unpress_key(13),
                        "c" | "C" => emulator.unpress_key(14),
                        _ => ()
                    }
                },
                _ => ()
            }
        }
    }
    return false;
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Usage: cargo run [game/path]");
        return;
    }
    let program = fs::read(&args[1]).expect("Unable to open file");

    let mut emulator = Chip8::new();
    emulator.load(&program);
    let event_loop = EventLoop::new().unwrap();
    let window_size = LogicalSize::new(SCALED_WIDTH as u32, SCALED_HEIGHT as u32);
    let window = Rc::new(
        WindowBuilder::new()
            .with_resizable(false)
            .with_inner_size(window_size)
            .build(&event_loop)
            .unwrap(),
    );
    let context = softbuffer::Context::new(window.clone()).unwrap();
    let mut surface = softbuffer::Surface::new(&context, window.clone()).unwrap();
    surface
        .resize(
            NonZeroU32::new(SCALED_WIDTH as u32).unwrap(),
            NonZeroU32::new(SCALED_HEIGHT as u32).unwrap(),
        )
        .unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    // Time controls for the frame rate
    let mut last_frame_time = Instant::now();
    let target_frame_rate = 60.0;
    let time_per_frame: u64 = ((1.0 / target_frame_rate) * 1_000.0) as u64;

    event_loop
        .run(move |event, elwt| {
            match event {
                Event::WindowEvent {
                    window_id: _,
                    event: WindowEvent::CloseRequested,
                } => {
                    elwt.exit();
                }
                Event::AboutToWait => {
                    for _ in 0..TICKS_PER_FRAME {
                        emulator.step();
                        emulator.tick_timers();
                    }
                    if emulator.needs_redraw() {
                        window.request_redraw();
                    }
                    // Limits the frame rate to 60 fps, avoids running too fast 
                    let time_elapsed: u64 = last_frame_time.elapsed().as_millis().try_into().unwrap_or_default();
                    last_frame_time = Instant::now();
                    if time_elapsed < time_per_frame {
                        sleep(Duration::from_millis(time_per_frame - time_elapsed))
                    }
                }
                Event::WindowEvent { window_id: _, event: WindowEvent::KeyboardInput { event, .. }} => {
                    let should_exit = handle_key(event.state, event.logical_key, &mut emulator, &program);
                    if should_exit {
                        elwt.exit();
                    }
                }
                Event::WindowEvent {
                    window_id: _,
                    event: WindowEvent::RedrawRequested,
                } => {
                    draw_screen(&mut surface, &mut emulator);
                }
                _ => (),
            }
        })
        .unwrap();
}
