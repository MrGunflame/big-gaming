use std::fmt::Write;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use ahash::HashMap;
use game_common::record::RecordReference;
use game_input::hotkeys::TriggerKind;
use game_input::keyboard::{KeyCode, ScanCode};

#[derive(Clone, Debug)]
pub struct Inputs {
    pub inputs: HashMap<RecordReference, Input>,
}

impl Inputs {
    pub fn encode(&self) -> String {
        let mut buf = String::new();

        for (id, input) in &self.inputs {
            write!(buf, "{}=", id);
        }

        buf
    }

    pub fn from_file(path: impl AsRef<Path>) -> Self {
        let mut file = File::open(path).unwrap();
        let mut string = String::new();
        file.read_to_string(&mut string).unwrap();

        let mut inputs = HashMap::default();
        for line in string.split('\n') {
            let Some((record, rem)) = line.split_once("=") else {
                continue;
            };

            let id = record.parse().unwrap();
            let input = Input::decode(rem);
            inputs.insert(id, input);
        }

        Self { inputs }
    }
}

#[derive(Clone, Debug)]
pub struct Input {
    pub trigger: TriggerKind,
    pub input_keys: Vec<InputKey>,
}

impl Input {
    pub fn encode(&self) -> String {
        let mut buf = String::new();
        if self.trigger.just_pressed() {
            buf.push_str("JUST_PRESSED");
        }
        if self.trigger.just_released() {
            buf.push_str("JUST_RELEASED");
        }
        if self.trigger.pressed() {
            buf.push_str("PRESSED");
        }

        for input in &self.input_keys {
            buf.push_str(&input.encode());
            buf.push_str(",");
        }
        buf
    }

    fn decode(mut buf: &str) -> Self {
        let mut trigger = TriggerKind::NONE;
        if let Some(s) = buf.strip_prefix("JUST_PRESSED") {
            trigger |= TriggerKind::JUST_PRESSED;
            buf = s;
        }
        if let Some(s) = buf.strip_prefix("JUST_RELEASED") {
            trigger |= TriggerKind::JUST_RELEASED;
            buf = s;
        }
        if let Some(s) = buf.strip_prefix("PRESSED") {
            trigger |= TriggerKind::PRESSED;
            buf = s;
        }

        let mut input_keys = vec![];
        for key in buf.split(",") {
            input_keys.push(InputKey::decode(key));
        }

        Self {
            trigger,
            input_keys,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum InputKey {
    KeyCode(KeyCode),
    ScanCode(ScanCode),
}

impl InputKey {
    pub fn encode(&self) -> String {
        match self {
            Self::KeyCode(code) => format!("kc={}", code.as_str()),
            Self::ScanCode(code) => format!("sc={}", code.0),
        }
    }

    fn decode(buf: &str) -> Self {
        if let Some(buf) = buf.strip_prefix("kc=") {
            let key = buf.parse().unwrap();
            Self::KeyCode(key)
        } else if let Some(buf) = buf.strip_prefix("sc=") {
            let key = buf.parse().unwrap();
            Self::ScanCode(ScanCode(key))
        } else {
            todo!()
        }
    }
}
