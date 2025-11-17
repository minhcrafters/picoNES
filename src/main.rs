use std::collections::HashMap;
use std::sync::{Arc, Mutex, mpsc::channel};

use clap::Parser;
use pico::bus::Bus;
use pico::cart::Cart;
use pico::cpu::CPU;
use pico::joypad;
use pico::movie::FM2Movie;
use pico::ppu::framebuffer::Framebuffer;
use pico::trace::trace;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::PixelFormatEnum;

#[derive(Parser)]
struct CliArgs {
    rom_file: String,
    movie_file: Option<String>,

    #[arg(short, long)]
    debug: bool,
}

fn main() {
    let args = CliArgs::parse();

    let bytes: Vec<u8> = std::fs::read(args.rom_file).unwrap();
    let mut rom = Cart::new(&bytes).unwrap();

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem
        .window("pico", (256.0 * 3.0) as u32, (240.0 * 3.0) as u32)
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().present_vsync().build().unwrap();
    canvas.set_scale(3.0, 3.0).unwrap();
    let creator = canvas.texture_creator();
    let mut texture = creator
        .create_texture_target(PixelFormatEnum::RGB24, 256, 240)
        .unwrap();

    let mut event_pump = sdl_context.event_pump().unwrap();

    let mut key_map = HashMap::new();
    key_map.insert(Keycode::Down, joypad::JoypadButton::DOWN);
    key_map.insert(Keycode::Up, joypad::JoypadButton::UP);
    key_map.insert(Keycode::Right, joypad::JoypadButton::RIGHT);
    key_map.insert(Keycode::Left, joypad::JoypadButton::LEFT);
    key_map.insert(Keycode::Space, joypad::JoypadButton::SELECT);
    key_map.insert(Keycode::Return, joypad::JoypadButton::START);
    key_map.insert(Keycode::X, joypad::JoypadButton::BUTTON_A);
    key_map.insert(Keycode::Z, joypad::JoypadButton::BUTTON_B);

    let (frame_tx, frame_rx) = channel::<Vec<u8>>();
    let shared_buttons: Arc<Mutex<HashMap<joypad::JoypadButton, bool>>> =
        Arc::new(Mutex::new(HashMap::new()));

    {
        let mut sb = shared_buttons.lock().unwrap();
        for b in [
            joypad::JoypadButton::DOWN,
            joypad::JoypadButton::UP,
            joypad::JoypadButton::RIGHT,
            joypad::JoypadButton::LEFT,
            joypad::JoypadButton::SELECT,
            joypad::JoypadButton::START,
            joypad::JoypadButton::BUTTON_A,
            joypad::JoypadButton::BUTTON_B,
        ] {
            sb.insert(b, false);
        }
    }

    let mut movie1: Option<FM2Movie> = None;

    if let Some(movie_path) = args.movie_file {
        match FM2Movie::load_from_file(movie_path) {
            Ok(movie) => {
                movie1 = Some(movie);
            }
            Err(e) => {
                eprintln!("Error: {e}");
            }
        }
    }

    let mut frame_count: usize = 0;

    let frame_tx_clone = frame_tx.clone();
    let shared_buttons_for_bus = shared_buttons.clone();

    let bus = Bus::new(&mut rom, move |ppu, joypad1, joypad2| {
        if movie1.is_some() {
            let movie = movie1.as_mut().unwrap();

            if frame_count < movie.frame_count() {
                let _ = movie.apply_frame_input(frame_count, joypad1, joypad2);
            }
        } else {
            if let Ok(sb) = shared_buttons_for_bus.lock() {
                for (btn, pressed) in sb.iter() {
                    joypad1.set_button_pressed_status(*btn, *pressed);
                }
            }
        }

        let mut fb = Framebuffer::new();
        pico::ppu::render::render(ppu, &mut fb);
        let _ = frame_tx_clone.send(fb.data);

        frame_count += 1;
    });

    let mut cpu = CPU::new(bus);

    cpu.reset();

    cpu.run_with_callback(move |cpu| {
        if args.debug {
            println!("{}", trace(cpu));
        }

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => {
                    std::process::exit(0);
                }
                Event::KeyDown {
                    keycode: Some(Keycode::R),
                    ..
                } => {
                    cpu.reset();
                }
                Event::KeyDown { keycode, .. } => {
                    if let Some(kc) = keycode {
                        if let Some(btn) = key_map.get(&kc) {
                            let mut sb = shared_buttons.lock().unwrap();
                            sb.insert(*btn, true);
                        }
                    }
                }
                Event::KeyUp { keycode, .. } => {
                    if let Some(kc) = keycode {
                        if let Some(btn) = key_map.get(&kc) {
                            let mut sb = shared_buttons.lock().unwrap();
                            sb.insert(*btn, false);
                        }
                    }
                }
                _ => {}
            }
        }

        if let Ok(frame_bytes) = frame_rx.try_recv() {
            texture.update(None, &frame_bytes, 256 * 3).unwrap();
            canvas.copy(&texture, None, None).unwrap();
            canvas.present();
        }
    });
}
