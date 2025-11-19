use error_iter::ErrorIter as _;
use log::error;
use pixels::wgpu::{PowerPreference, RequestAdapterOptions};
use pixels::{Error, PixelsBuilder, SurfaceTexture};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use winit::dpi::LogicalSize;
use winit::event::{Event, WindowEvent};
use winit::event_loop::EventLoop;
use winit::keyboard::KeyCode;
use winit::window::WindowBuilder;
use winit_input_helper::WinitInputHelper;

use clap::Parser;
use pico::apu::APU;
use pico::cart::Cart;
use pico::joypad::JoypadButton;
use pico::movie::FM2Movie;
use pico::nes::{ClockResult, Nes};
use pico::ppu::framebuffer::Framebuffer;
use pico::trace::trace;

const WIDTH: u32 = 256;

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
const HEIGHT: u32 = 240;
const SCALE: u32 = 3;

#[derive(Parser)]
struct CliArgs {
    rom_file: String,
    movie_file: Option<String>,

    #[arg(short, long)]
    debug: bool,
}

fn main() -> Result<(), Error> {
    env_logger::init();
    let args = CliArgs::parse();

    let sdl_ctx = sdl2::init().unwrap();
    let audio_subsystem = sdl_ctx.audio().unwrap();

    let bytes = std::fs::read(&args.rom_file).expect("failed to read ROM");
    let cart = Cart::new(&bytes).expect("failed to parse cartridge");

    let event_loop = EventLoop::new().unwrap();
    let mut input = WinitInputHelper::new();
    let window = {
        let size = LogicalSize::new((WIDTH * SCALE) as f64, (HEIGHT * SCALE) as f64);
        WindowBuilder::new()
            .with_title("pico")
            .with_inner_size(size)
            .with_min_inner_size(size)
            .build(&event_loop)
            .unwrap()
    };

    let mut pixels = {
        let window_size = window.inner_size();
        let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
        PixelsBuilder::new(WIDTH, HEIGHT, surface_texture)
            .request_adapter_options(RequestAdapterOptions {
                power_preference: PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: None,
            })
            .build()?
    };

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
    let mut key_map = HashMap::new();
    key_map.insert(KeyCode::ArrowDown, JoypadButton::DOWN);
    key_map.insert(KeyCode::ArrowUp, JoypadButton::UP);
    key_map.insert(KeyCode::ArrowRight, JoypadButton::RIGHT);
    key_map.insert(KeyCode::ArrowLeft, JoypadButton::LEFT);
    key_map.insert(KeyCode::Space, JoypadButton::SELECT);
    key_map.insert(KeyCode::Enter, JoypadButton::START);
    key_map.insert(KeyCode::KeyX, JoypadButton::BUTTON_A);
    key_map.insert(KeyCode::KeyZ, JoypadButton::BUTTON_B);

    let mut button_states: HashMap<JoypadButton, bool> =
        key_map.values().copied().map(|btn| (btn, false)).collect();

    let mut movie = args
        .movie_file
        .and_then(|path| FM2Movie::load_from_file(path).ok());

    let mut frame_count: usize = 0;
    let mut framebuffer = Framebuffer::new();

    let res = event_loop.run(|event, elwt| {
        // Draw the current frame
        if let Event::WindowEvent {
            event: WindowEvent::RedrawRequested,
            ..
        } = event
        {
            apply_inputs(&mut nes, &mut movie, frame_count, &button_states);
            run_frame(&mut nes, args.debug);
            frame_count = frame_count.wrapping_add(1);

            framebuffer.data.fill(0);
            nes.bus.render_frame(&mut framebuffer);

            // Convert RGB24 framebuffer to RGBA8 for pixels
            let frame = pixels.frame_mut();
            for (i, chunk) in framebuffer.data.chunks(3).enumerate() {
                if i * 4 + 3 < frame.len() {
                    frame[i * 4] = chunk[0];
                    frame[i * 4 + 1] = chunk[1];
                    frame[i * 4 + 2] = chunk[2];
                    frame[i * 4 + 3] = 0xff;
                }
            }

            if let Err(err) = pixels.render() {
                log_error("pixels.render", err);
                elwt.exit();
                return;
            }
        }

        // Handle input events
        if input.update(&event) {
            // Close events
            if input.key_pressed(KeyCode::Escape) || input.close_requested() {
                elwt.exit();
                return;
            }

            // Reset emulator
            if input.key_pressed(KeyCode::KeyR) {
                nes.reset();
                frame_count = 0;
            }

            // Handle key presses for button mapping
            for (key, btn) in &key_map {
                if input.key_pressed(*key) {
                    button_states.insert(*btn, true);
                }
                if input.key_released(*key) {
                    button_states.insert(*btn, false);
                }
            }

            // Resize the window
            if let Some(size) = input.window_resized() {
                if let Err(err) = pixels.resize_surface(size.width, size.height) {
                    log_error("pixels.resize_surface", err);
                    elwt.exit();
                    return;
                }
            }

            // Request a redraw
            window.request_redraw();
        }
    });

    res.map_err(|e| Error::UserDefined(Box::new(e)))
}

fn log_error<E: std::error::Error + 'static>(method_name: &str, err: E) {
    error!("{method_name}() failed: {err}");
    for source in err.sources().skip(1) {
        error!("  Caused by: {source}");
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
