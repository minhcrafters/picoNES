use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

use crate::joypad::JoypadButton;

#[derive(Debug, Clone)]
pub struct MovieHeader {
    pub version: i32,
    pub emu_version: String,
    pub rerecord_count: Option<i32>,
    pub pal_flag: bool,
    pub new_ppu: bool,
    pub fds: bool,
    pub fourscore: bool,
    pub port0: InputDevice,
    pub port1: InputDevice,
    pub port2: FamicomExpPort,
    pub binary: bool,
    pub length: Option<usize>,
    pub rom_filename: String,
    pub comment: Option<String>,
    pub subtitles: Option<Vec<Subtitle>>,
    pub guid: String,
    pub rom_checksum: String,
    pub savestate: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputDevice {
    None = 0,
    Gamepad = 1,
    Zapper = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FamicomExpPort {
    None = 0,
}

#[derive(Debug, Clone)]
pub struct InputRecord {
    pub commands: u8,
    pub port0_input: Option<GamepadInput>,
    pub port1_input: Option<GamepadInput>,
    pub port2_input: Option<()>,
}

#[derive(Debug, Clone)]
pub struct GamepadInput {
    pub right: bool,
    pub left: bool,
    pub down: bool,
    pub up: bool,
    pub start: bool,
    pub select: bool,
    pub b: bool,
    pub a: bool,
}

#[derive(Debug, Clone)]
pub struct Subtitle {
    pub frame: u32,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct FM2Movie {
    pub header: MovieHeader,
    pub input_log: Vec<InputRecord>,
}

impl FM2Movie {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let file = File::open(path).map_err(|e| format!("Failed to open file: {}", e))?;
        let reader = BufReader::new(file);
        Self::parse(reader)
    }

    pub fn parse<R: Read>(mut reader: R) -> Result<Self, String> {
        let mut buffer = Vec::new();
        reader
            .read_to_end(&mut buffer)
            .map_err(|e| format!("Failed to read file: {}", e))?;

        // Handle UTF-8 encoding issues gracefully
        let contents = String::from_utf8(buffer.clone())
            .unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned());

        // Split into header and input log
        let mut lines = contents.lines();
        let mut header = String::new();

        // Read header section
        for line in &mut lines {
            if line.trim().is_empty() {
                continue;
            }

            if line.starts_with('|') {
                // We've reached the input log section
                break;
            }

            header.push_str(line);
            header.push('\n');
        }

        let movie_header = parse_header(&header)?;

        // Parse input log and subtitles from the remaining lines
        let input_log = parse_input_log(lines.clone(), &movie_header)?;

        Ok(FM2Movie {
            header: movie_header,
            input_log,
        })
    }

    pub fn frame_count(&self) -> usize {
        self.header.length.unwrap_or(self.input_log.len())
    }

    pub fn get_frame_input(&self, frame: usize) -> Option<&InputRecord> {
        self.input_log.get(frame)
    }

    pub fn apply_frame_input(
        &self,
        frame: usize,
        joypad1: &mut crate::joypad::Joypad,
        joypad2: &mut crate::joypad::Joypad,
    ) -> Result<(), String> {
        let input = self
            .get_frame_input(frame)
            .ok_or_else(|| format!("Frame {} out of range", frame))?;

        // Apply port0 input
        if let Some(gamepad_input) = &input.port0_input {
            let mut buttons = JoypadButton::empty();

            if gamepad_input.right {
                buttons |= JoypadButton::RIGHT;
            }
            if gamepad_input.left {
                buttons |= JoypadButton::LEFT;
            }
            if gamepad_input.down {
                buttons |= JoypadButton::DOWN;
            }
            if gamepad_input.up {
                buttons |= JoypadButton::UP;
            }
            if gamepad_input.start {
                buttons |= JoypadButton::START;
            }
            if gamepad_input.select {
                buttons |= JoypadButton::SELECT;
            }
            if gamepad_input.b {
                buttons |= JoypadButton::BUTTON_B;
            }
            if gamepad_input.a {
                buttons |= JoypadButton::BUTTON_A;
            }

            joypad1.button_status = buttons;

            // println!("{:?}", joypad1.button_status);
        }

        // Apply port1 input
        if let Some(gamepad_input) = &input.port1_input {
            let mut buttons = JoypadButton::empty();

            if gamepad_input.right {
                buttons |= JoypadButton::RIGHT;
            }
            if gamepad_input.left {
                buttons |= JoypadButton::LEFT;
            }
            if gamepad_input.down {
                buttons |= JoypadButton::DOWN;
            }
            if gamepad_input.up {
                buttons |= JoypadButton::UP;
            }
            if gamepad_input.start {
                buttons |= JoypadButton::START;
            }
            if gamepad_input.select {
                buttons |= JoypadButton::SELECT;
            }
            if gamepad_input.b {
                buttons |= JoypadButton::BUTTON_B;
            }
            if gamepad_input.a {
                buttons |= JoypadButton::BUTTON_A;
            }

            joypad2.button_status = buttons;
        }

        Ok(())
    }
}

fn parse_header(header_text: &str) -> Result<MovieHeader, String> {
    let mut pairs = HashMap::new();

    let mut subtitles = Vec::new();

    for line in header_text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.splitn(2, ' ').collect();
        if parts.len() != 2 {
            continue;
        }

        if line.starts_with("subtitle") {
            subtitles.push(parse_subtitle_line(line)?);
        }

        let key = parts[0];
        let value = parts[1];
        pairs.insert(key, value);
    }

    let version = pairs
        .get("version")
        .ok_or("Missing version field")?
        .parse::<i32>()
        .map_err(|_| "Invalid version format")?;

    let emu_version = pairs
        .get("emuVersion")
        .ok_or("Missing emuVersion field")?
        .to_string();

    let rerecord_count = pairs
        .get("rerecordCount")
        .and_then(|v| v.parse::<i32>().ok());

    let pal_flag = pairs.get("palFlag").map(|v| *v == "1").unwrap_or(false);

    let new_ppu = pairs.get("NewPPU").map(|v| *v == "1").unwrap_or(false);

    let fds = pairs.get("FDS").map(|v| *v == "1").unwrap_or(false);

    let fourscore = pairs.get("fourscore").map(|v| *v == "1").unwrap_or(false);

    let port0 = match pairs.get("port0").and_then(|v| v.parse::<i32>().ok()) {
        Some(0) => InputDevice::None,
        Some(1) => InputDevice::Gamepad,
        Some(2) => InputDevice::Zapper,
        Some(v) => return Err(format!("Invalid port0 value: {}", v)),
        None => InputDevice::Gamepad,
    };

    let port1 = match pairs.get("port1").and_then(|v| v.parse::<i32>().ok()) {
        Some(0) => InputDevice::None,
        Some(1) => InputDevice::Gamepad,
        Some(2) => InputDevice::Zapper,
        Some(v) => return Err(format!("Invalid port1 value: {}", v)),
        None => InputDevice::Gamepad,
    };

    let port2 = match pairs.get("port2").and_then(|v| v.parse::<i32>().ok()) {
        Some(0) => FamicomExpPort::None,
        Some(v) => return Err(format!("Invalid port2 value: {}", v)),
        None => FamicomExpPort::None,
    };

    let binary = pairs.get("binary").map(|v| *v == "1").unwrap_or(false);

    let length = pairs.get("length").and_then(|v| v.parse::<usize>().ok());

    let rom_filename = pairs
        .get("romFilename")
        .ok_or("Missing romFilename field")?
        .to_string();

    let comment = pairs.get("comment").map(|v| v.to_string());

    let guid = pairs.get("guid").ok_or("Missing guid field")?.to_string();

    let rom_checksum = pairs
        .get("romChecksum")
        .ok_or("Missing romChecksum field")?
        .to_string();

    Ok(MovieHeader {
        version,
        emu_version,
        rerecord_count,
        pal_flag,
        new_ppu,
        fds,
        fourscore,
        port0,
        port1,
        port2,
        binary,
        length,
        rom_filename,
        comment,
        subtitles: Some(subtitles),
        guid,
        rom_checksum,
        savestate: None, // TODO: Implement savestate parsing
    })
}

fn parse_input_log(
    lines: std::str::Lines,
    header: &MovieHeader,
) -> Result<Vec<InputRecord>, String> {
    let mut input_log = Vec::new();

    if header.binary {
        return Err("Binary format not supported with line-based parsing".to_string());
    }

    // Parse text format and subtitles from remaining lines
    for line in lines {
        let trimmed_line = line.trim();
        if trimmed_line.is_empty() {
            continue;
        }

        // Check for input log lines
        if !trimmed_line.starts_with('|') || !trimmed_line.ends_with('|') {
            continue;
        }

        let record = parse_text_record(&trimmed_line, header)?;
        input_log.push(record);

        // Respect length limit if specified
        if let Some(length) = header.length {
            if input_log.len() >= length {
                break;
            }
        }
    }

    Ok(input_log)
}

fn parse_text_record(line: &str, header: &MovieHeader) -> Result<InputRecord, String> {
    // Remove leading and trailing pipes
    let content = &line[1..line.len() - 1];
    let fields: Vec<&str> = content.split('|').collect();

    if fields.len() < 3 {
        return Err("Invalid record format".to_string());
    }

    let commands = fields[0]
        .trim()
        .parse::<u8>()
        .map_err(|_| "Invalid commands field")?;

    let port0_input = if header.port0 == InputDevice::Gamepad {
        Some(parse_gamepad_input(fields[1].trim())?)
    } else {
        None
    };

    let port1_input = if header.port1 == InputDevice::Gamepad {
        Some(parse_gamepad_input(fields[2].trim())?)
    } else {
        None
    };

    Ok(InputRecord {
        commands,
        port0_input,
        port1_input,
        port2_input: None,
    })
}

fn parse_gamepad_input(input: &str) -> Result<GamepadInput, String> {
    let input = input.trim();

    // Handle empty or minimal input
    if input.is_empty() {
        return Ok(GamepadInput {
            right: false,
            left: false,
            down: false,
            up: false,
            start: false,
            select: false,
            b: false,
            a: false,
        });
    }

    let chars: Vec<char> = input.chars().collect();

    // Ensure we have at least 8 characters, pad with dots if necessary
    let mut padded_chars = vec!['.'; 8];
    for (i, &ch) in chars.iter().take(8).enumerate() {
        padded_chars[i] = ch;
    }

    Ok(GamepadInput {
        right: padded_chars[0] != ' ' && padded_chars[0] != '.',
        left: padded_chars[1] != ' ' && padded_chars[1] != '.',
        down: padded_chars[2] != ' ' && padded_chars[2] != '.',
        up: padded_chars[3] != ' ' && padded_chars[3] != '.',
        start: padded_chars[4] != ' ' && padded_chars[4] != '.',
        select: padded_chars[5] != ' ' && padded_chars[5] != '.',
        b: padded_chars[6] != ' ' && padded_chars[6] != '.',
        a: padded_chars[7] != ' ' && padded_chars[7] != '.',
    })
}

fn parse_subtitle_line(line: &str) -> Result<Subtitle, String> {
    let parts: Vec<&str> = line.splitn(3, char::is_whitespace).collect();
    if parts.len() < 3 {
        return Err("Invalid subtitle format".to_string());
    }

    let frame = parts[1]
        .parse::<u32>()
        .map_err(|e| format!("Invalid frame number: {}", e))?;
    let text = parts[2].to_string();

    Ok(Subtitle { frame, text })
}
