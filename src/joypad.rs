#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JoypadButton {
    A,
    B,
    Select,
    Start,
    Up,
    Down,
    Left,
    Right,
}

pub struct Joypad {
    strobe: bool,
    button_index: u8,
    button_status: u8,
}

impl Joypad {
    pub fn new() -> Self {
        Joypad {
            strobe: false,
            button_index: 0,
            button_status: 0,
        }
    }

    pub fn write(&mut self, data: u8) {
        self.strobe = data & 1 == 1;
        if self.strobe {
            self.button_index = 0;
        }
    }

    pub fn read(&mut self) -> u8 {
        if self.button_index > 7 {
            return 1;
        }
        let response = (self.button_status & (1 << self.button_index)) >> self.button_index;
        if !self.strobe {
            self.button_index += 1;
        }
        response
    }

    pub fn set_button_pressed(&mut self, button: JoypadButton, pressed: bool) {
        let bit = match button {
            JoypadButton::A => 0,
            JoypadButton::B => 1,
            JoypadButton::Select => 2,
            JoypadButton::Start => 3,
            JoypadButton::Up => 4,
            JoypadButton::Down => 5,
            JoypadButton::Left => 6,
            JoypadButton::Right => 7,
        };
        if pressed {
            self.button_status |= 1 << bit;
        } else {
            self.button_status &= !(1 << bit);
        }
    }
}
