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
use sdl2::audio::{AudioCallback, AudioSpecDesired};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::PixelFormatEnum;

struct ApuAudioCallback {
    buffer: Arc<Mutex<VecDeque<f32>>>,
}

impl AudioCallback for ApuAudioCallback {
    type Channel = f32;

    fn callback(&mut self, out: &mut [f32]) {
        if let Ok(mut buf) = self.buffer.lock() {
            for sample in out.iter_mut() {
                if let Some(value) = buf.pop_front() {
                    *sample = value;
                } else {
                    *sample = 0.0;
                }
            }
        } else {
            out.fill(0.0);
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

    let bytes = std::fs::read(&args.rom_file).expect("failed to read ROM");
    let cart = Cart::new(&bytes).expect("failed to parse cartridge");

    let sdl = sdl2::init().expect("failed to init SDL");
    let video = sdl.video().expect("failed to init video");
    let audio = sdl.audio().expect("failed to init audio");

    let window = video
        .window("pico", (256 * 3) as u32, (240 * 3) as u32)
        .position_centered()
        .build()
        .expect("failed to create window");
    let mut canvas = window.into_canvas().present_vsync().build().unwrap();
    canvas.set_scale(3.0, 3.0).unwrap();

    let creator = canvas.texture_creator();
    let mut texture = creator
        .create_texture_streaming(PixelFormatEnum::RGB24, 256, 240)
        .unwrap();

    let mut event_pump = sdl.event_pump().unwrap();

    let audio_buffer = Arc::new(Mutex::new(VecDeque::with_capacity(96_000)));
    let desired_spec = AudioSpecDesired {
        freq: Some(48_000),
        channels: Some(1),
        samples: Some(1024),
    };

    let buffer_for_device = audio_buffer.clone();
    let audio_device = audio
        .open_playback(None, &desired_spec, move |_spec| ApuAudioCallback {
            buffer: buffer_for_device,
        })
        .unwrap();
    audio_device.resume();
    let sample_rate = audio_device.spec().freq.max(1) as u32;

    let mut key_map = HashMap::new();
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

    let apu = APU::new(sample_rate, audio_buffer.clone());
    let mut nes = Nes::new(cart, apu);
    nes.reset();

    let mut movie = args
        .movie_file
        .and_then(|path| FM2Movie::load_from_file(path).ok());

    let mut frame_count: usize = 0;
    let mut framebuffer = Framebuffer::new();

    'emulation: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'emulation,
                Event::KeyDown {
                    keycode: Some(Keycode::R),
                    ..
                } => {
                    nes.reset();
                    frame_count = 0;
                }
                Event::KeyDown {
                    keycode: Some(kc), ..
                } => {
                    if let Some(btn) = key_map.get(&kc) {
                        button_states.insert(*btn, true);
                    }
                }
                Event::KeyUp {
                    keycode: Some(kc), ..
                } => {
                    if let Some(btn) = key_map.get(&kc) {
                        button_states.insert(*btn, false);
                    }
                }
                _ => {}
            }
        }

        apply_inputs(&mut nes, &mut movie, frame_count, &button_states);

        run_frame(&mut nes, args.debug);
        
        frame_count = frame_count.wrapping_add(1);

        framebuffer.data.fill(0);
        nes.bus.render_frame(&mut framebuffer);
        texture
            .update(None, &framebuffer.data, 256 * 3)
            .expect("texture upload failed");
        canvas.clear();
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
            println!("{}", trace(&nes.cpu, &nes.bus));
        }

        if frame_complete {
            break;
        }
    }
}
