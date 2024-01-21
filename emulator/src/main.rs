use chip8::Chip8;
use softbuffer::Surface;
use std::num::NonZeroU32;
use winit::dpi::LogicalSize;
use std::rc::Rc;
use std::{env, fs};
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{WindowBuilder, Window};

const SCALE: usize = 15; // 15x scale to the display
const SCREEN_WIDTH: usize = 64 * SCALE;
const SCREEN_HEIGHT: usize = 32 * SCALE;

fn draw_screen(surface: &mut Surface<Rc<Window>, Rc<Window>>, display: &[bool]) {
    let mut buffer = surface.buffer_mut().unwrap();

    for (i, pixel) in display.iter().enumerate() {
        let value = if *pixel { 0 } else { u32::MAX };
        let index = i * SCALE;
        let x = index % SCREEN_WIDTH;
        let y = index / SCREEN_WIDTH;
        for i in x..(x+SCALE) {
            for j in y..(y+SCALE) {
                buffer[j * SCREEN_WIDTH + i] = value;
            }
        }
    }
    buffer.present().unwrap();
}

fn main() {
    // let args: Vec<String> = env::args().collect();
    // if args.len() < 2 {
    //     println!("Usage: cargo run [game/path]");
    //     return;
    // }
    let path = "../roms/test_opcode.ch8";
    let program = fs::read(path).expect("Unable to open file");

    let mut emulator = Chip8::new();
    emulator.load(&program);
    let event_loop = EventLoop::new().unwrap();
    let window_size = LogicalSize::new(SCREEN_WIDTH as u32, SCREEN_HEIGHT as u32);
    let window = Rc::new(
        WindowBuilder::new()
            .with_resizable(false)
            .with_inner_size(window_size)
            .build(&event_loop)
            .unwrap()
    );
    let context = softbuffer::Context::new(window.clone()).unwrap();
    let mut surface = softbuffer::Surface::new(&context, window.clone()).unwrap();
    surface.resize(NonZeroU32::new(SCREEN_WIDTH as u32).unwrap(), NonZeroU32::new(SCREEN_HEIGHT as u32).unwrap()).unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    event_loop.run(move |event, elwt| {
        match event {
            Event::WindowEvent { window_id: _, event: WindowEvent::CloseRequested } => {
                elwt.exit();
            },
            Event::WindowEvent { window_id: _, event: WindowEvent::RedrawRequested } => {
                println!("Drawing Screen");
                draw_screen(&mut surface, emulator.get_display());
            }
            _ => ()
        }
        for _ in 0..10 {
            emulator.step();
        }
        
        emulator.tick_timers();
    }).unwrap();
}
