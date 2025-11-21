use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

use clap::Parser;
use pico::apu::APU;
use pico::cart::Cart;
use pico::joypad::JoypadButton;
use pico::movie::FM2Movie;
use pico::nes::{ClockResult, Nes};
use pico::ppu::framebuffer::Framebuffer;
use pico::trace::trace;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::PixelFormatEnum;

const WIDTH: u32 = 256;
const HEIGHT: u32 = 240;
const SCALE: u32 = 2;

struct AudioCallbackImpl {
    audio_buffer: Arc<Mutex<VecDeque<f32>>>,
}

impl sdl2::audio::AudioCallback for AudioCallbackImpl {
    type Channel = f32;

    fn callback(&mut self, out: &mut [f32]) {
        let mut buffer = self.audio_buffer.lock().unwrap();
        for sample in out.iter_mut() {
            *sample = buffer.pop_front().unwrap_or(0.0);
        }
    }
}

#[derive(Parser)]
struct CliArgs {
    rom_file: String,
    movie_file: Option<String>,

    #[arg(short, long)]
    debug: bool,
}

fn main() {
    env_logger::init();
    let args = CliArgs::parse();

    let sdl_ctx = sdl2::init().unwrap();
    let video_subsystem = sdl_ctx.video().unwrap();
    let audio_subsystem = sdl_ctx.audio().unwrap();

    let bytes = std::fs::read(&args.rom_file).expect("failed to read ROM");
    let cart = Cart::new(&bytes).expect("failed to parse cartridge");

    let window = video_subsystem
        .window("pico", WIDTH * SCALE, HEIGHT * SCALE)
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().present_vsync().build().unwrap();
    canvas.set_draw_color(sdl2::pixels::Color::BLACK);
    canvas.clear();
    canvas.present();

    let texture_creator = canvas.texture_creator();
    let mut texture = texture_creator
        .create_texture_target(PixelFormatEnum::RGB24, WIDTH, HEIGHT)
        .unwrap();

    // Initialize emulator
    let sample_rate = 48000;
    let audio_buffer = Arc::new(Mutex::new(VecDeque::with_capacity(
        sample_rate as usize * 2,
    )));

    let apu = APU::new(sample_rate, audio_buffer.clone());

    let audio_device = audio_subsystem
        .open_playback(
            None,
            &sdl2::audio::AudioSpecDesired {
                freq: Some(sample_rate as i32),
                channels: Some(1),
                samples: None,
            },
            |spec| {
                assert_eq!(spec.freq, sample_rate as i32);
                assert_eq!(spec.channels, 1);
                AudioCallbackImpl {
                    audio_buffer: audio_buffer.clone(),
                }
            },
        )
        .unwrap();

    audio_device.resume();

    let mut nes = Nes::new(cart, apu);
    nes.reset();

    // Setup input mapping
    let mut key_map: HashMap<Keycode, JoypadButton> = HashMap::new();
    key_map.insert(Keycode::Down, JoypadButton::DOWN);
    key_map.insert(Keycode::Up, JoypadButton::UP);
    key_map.insert(Keycode::Right, JoypadButton::RIGHT);
    key_map.insert(Keycode::Left, JoypadButton::LEFT);
    key_map.insert(Keycode::Space, JoypadButton::SELECT);
    key_map.insert(Keycode::Return, JoypadButton::START);
    key_map.insert(Keycode::X, JoypadButton::BUTTON_A);
    key_map.insert(Keycode::Z, JoypadButton::BUTTON_B);

    let mut button_states: HashMap<JoypadButton, bool> =
        key_map.values().copied().map(|btn| (btn, false)).collect();

    let mut movie = args
        .movie_file
        .and_then(|path| FM2Movie::load_from_file(path).ok());

    let mut frame_count: usize = 0;
    let mut framebuffer = Framebuffer::new();

    let mut event_pump = sdl_ctx.event_pump().unwrap();
    let mut running = true;

    while running {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => {
                    running = false;
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => {
                    running = false;
                }
                Event::KeyDown {
                    keycode: Some(Keycode::R),
                    ..
                } => {
                    nes.reset();
                    frame_count = 0;
                }
                _ => {}
            }
        }

        let keys: Vec<Keycode> = event_pump
            .keyboard_state()
            .pressed_scancodes()
            .filter_map(|sc| Keycode::from_scancode(sc))
            .collect();

        for (key, btn) in &key_map {
            button_states.insert(*btn, keys.contains(key));
        }

        apply_inputs(&mut nes, &mut movie, frame_count, &button_states);
        run_frame(&mut nes, args.debug);
        frame_count = frame_count.wrapping_add(1);

        framebuffer.data.fill(0);
        nes.bus.render_frame(&mut framebuffer);

        texture
            .update(None, &framebuffer.data, (WIDTH * 3) as usize)
            .unwrap();
        canvas.copy(&texture, None, None).unwrap();
        canvas.present();
    }
}

fn apply_inputs(
    nes: &mut Nes,
    movie: &mut Option<FM2Movie>,
    frame_count: usize,
    buttons: &HashMap<JoypadButton, bool>,
) {
    if let Some(movie) = movie {
        if frame_count < movie.frame_count() {
            let (joypad1, joypad2) = nes.joypads_mut();
            let _ = movie.apply_frame_input(frame_count, joypad1, joypad2);
            return;
        }
    }

    if let Some(joypad) = nes.joypad_mut(0) {
        for (btn, state) in buttons {
            joypad.set_button_pressed_status(*btn, *state);
        }
    }
}

fn run_frame(nes: &mut Nes, debug_trace: bool) {
    loop {
        let ClockResult {
            frame_complete,
            instruction_complete,
        } = nes.clock();

        if debug_trace && instruction_complete {
            println!("{}", trace(&nes.bus.cpu, &nes.bus));
        }

        if frame_complete {
            break;
        }
    }
}
