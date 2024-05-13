use chip8::Chip8;
use softbuffer::Surface;
use std::num::NonZeroU32;
use std::rc::Rc;
use std::{env, fs};
use std::time::{Duration, Instant};
use std::thread::sleep;
use winit::dpi::LogicalSize;
use winit::event::{Event, WindowEvent, ElementState};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{Window, WindowBuilder};
use winit::keyboard::{KeyCode, PhysicalKey};

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
fn handle_key(state: ElementState, key: PhysicalKey, emulator: &mut Chip8, rom: &Vec<u8>) -> bool {
    match state {
        ElementState::Pressed => {
            match key {
                PhysicalKey::Code(keycode) => {
                    match keycode {
                        KeyCode::Escape => {
                            return true;
                        },
                        KeyCode::F5 => {
                            emulator.reset();
                            emulator.load(rom);
                        },
                        KeyCode::Digit1 => emulator.press_key(0x1),
                        KeyCode::Digit2 => emulator.press_key(0x2),
                        KeyCode::Digit3 => emulator.press_key(0x3),
                        KeyCode::Digit4 => emulator.press_key(0xc),
                        KeyCode::KeyQ => emulator.press_key(0x4),
                        KeyCode::KeyW => emulator.press_key(0x5),
                        KeyCode::KeyE => emulator.press_key(0x6),
                        KeyCode::KeyR => emulator.press_key(0xd),
                        KeyCode::KeyA => emulator.press_key(0x7),
                        KeyCode::KeyS => emulator.press_key(0x8),
                        KeyCode::KeyD => emulator.press_key(0x9),
                        KeyCode::KeyF => emulator.press_key(0xe),
                        KeyCode::KeyZ => emulator.press_key(0xa),
                        KeyCode::KeyX => emulator.press_key(0x0),
                        KeyCode::KeyC => emulator.press_key(0xb),
                        KeyCode::KeyV => emulator.press_key(0xf),
                        _ => ()
                    }
                },
                _ => ()
            }
        },
        ElementState::Released => {
            match key {
                PhysicalKey::Code(keycode) => {
                    match keycode {
                        KeyCode::Digit1 => emulator.unpress_key(0x1),
                        KeyCode::Digit2 => emulator.unpress_key(0x2),
                        KeyCode::Digit3 => emulator.unpress_key(0x3),
                        KeyCode::Digit4 => emulator.unpress_key(0xc),
                        KeyCode::KeyQ => emulator.unpress_key(0x4),
                        KeyCode::KeyW => emulator.unpress_key(0x5),
                        KeyCode::KeyE => emulator.unpress_key(0x6),
                        KeyCode::KeyR => emulator.unpress_key(0xd),
                        KeyCode::KeyA => emulator.unpress_key(0x7),
                        KeyCode::KeyS => emulator.unpress_key(0x8),
                        KeyCode::KeyD => emulator.unpress_key(0x9),
                        KeyCode::KeyF => emulator.unpress_key(0xe),
                        KeyCode::KeyZ => emulator.unpress_key(0xa),
                        KeyCode::KeyX => emulator.unpress_key(0x0),
                        KeyCode::KeyC => emulator.unpress_key(0xb),
                        KeyCode::KeyV => emulator.unpress_key(0xf),
                        _ => ()
                    }
                },
                _ => ()
            }
        }
    }
    false
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
                    let should_exit = handle_key(event.state, event.physical_key, &mut emulator, &program);
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
