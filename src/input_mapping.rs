use crate::config::{ControllerBindings, KeyboardBindings};
use crate::joypad::{Joypad, JoypadButton};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct DirectionalInput {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct JoypadInputState {
    pub a: bool,
    pub b: bool,
    pub select: bool,
    pub start: bool,
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
}

impl JoypadInputState {
    pub fn to_mask(self) -> u8 {
        (self.a as u8)
            | ((self.b as u8) << 1)
            | ((self.select as u8) << 2)
            | ((self.start as u8) << 3)
            | ((self.up as u8) << 4)
            | ((self.down as u8) << 5)
            | ((self.left as u8) << 6)
            | ((self.right as u8) << 7)
    }

    pub fn from_mask(mask: u8) -> Self {
        Self {
            a: mask & 0x01 != 0,
            b: mask & 0x02 != 0,
            select: mask & 0x04 != 0,
            start: mask & 0x08 != 0,
            up: mask & 0x10 != 0,
            down: mask & 0x20 != 0,
            left: mask & 0x40 != 0,
            right: mask & 0x80 != 0,
        }
    }

    pub fn merge(&mut self, other: Self) {
        self.a |= other.a;
        self.b |= other.b;
        self.select |= other.select;
        self.start |= other.start;
        self.up |= other.up;
        self.down |= other.down;
        self.left |= other.left;
        self.right |= other.right;
    }

    pub fn apply_to_joypad(self, joypad: &mut Joypad) {
        joypad.set_button_pressed(JoypadButton::A, self.a);
        joypad.set_button_pressed(JoypadButton::B, self.b);
        joypad.set_button_pressed(JoypadButton::Select, self.select);
        joypad.set_button_pressed(JoypadButton::Start, self.start);
        joypad.set_button_pressed(JoypadButton::Up, self.up);
        joypad.set_button_pressed(JoypadButton::Down, self.down);
        joypad.set_button_pressed(JoypadButton::Left, self.left);
        joypad.set_button_pressed(JoypadButton::Right, self.right);
    }
}

pub fn keyboard_state_from_bindings<F>(
    bindings: &KeyboardBindings,
    mut pressed_key: F,
    turbo_active: bool,
) -> JoypadInputState
where
    F: FnMut(&str) -> bool,
{
    let mut state = JoypadInputState {
        a: pressed_key(&bindings.a),
        b: pressed_key(&bindings.b),
        select: pressed_key(&bindings.select),
        start: pressed_key(&bindings.start),
        up: pressed_key(&bindings.up),
        down: pressed_key(&bindings.down),
        left: pressed_key(&bindings.left),
        right: pressed_key(&bindings.right),
    };

    if turbo_active && pressed_key(&bindings.turbo_a) {
        state.a = true;
    }
    if turbo_active && pressed_key(&bindings.turbo_b) {
        state.b = true;
    }

    state
}

pub fn controller_state_from_bindings<F>(
    bindings: &ControllerBindings,
    mut pressed_button: F,
    dpad: DirectionalInput,
    left_stick: DirectionalInput,
    turbo_active: bool,
) -> JoypadInputState
where
    F: FnMut(&str) -> bool,
{
    let mut state = JoypadInputState {
        a: pressed_button(&bindings.a),
        b: pressed_button(&bindings.b),
        select: pressed_button(&bindings.select),
        start: pressed_button(&bindings.start),
        up: dpad.up || left_stick.up,
        down: dpad.down || left_stick.down,
        left: dpad.left || left_stick.left,
        right: dpad.right || left_stick.right,
    };

    if turbo_active && pressed_button(&bindings.turbo_a) {
        state.a = true;
    }
    if turbo_active && pressed_button(&bindings.turbo_b) {
        state.b = true;
    }

    state
}
