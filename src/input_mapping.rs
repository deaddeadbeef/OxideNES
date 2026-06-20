use crate::config::{ControllerBindings, InputBindings, KeyboardBindings};
use crate::joypad::{Joypad, JoypadButton};
use gilrs::Button as GilrsButton;
use minifb::Key as MinifbKey;

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

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct JoypadInputPair {
    pub p1: JoypadInputState,
    pub p2: JoypadInputState,
}

impl JoypadInputPair {
    pub fn apply_to_joypads(self, joypad1: &mut Joypad, joypad2: &mut Joypad) {
        self.p1.apply_to_joypad(joypad1);
        self.p2.apply_to_joypad(joypad2);
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ControllerInputSnapshot<'a> {
    pub pressed_buttons: &'a [&'a str],
    pub dpad: DirectionalInput,
    pub left_stick: DirectionalInput,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct HostInputSnapshot<'a> {
    pub pressed_keys: &'a [&'a str],
    pub controller_p1: Option<ControllerInputSnapshot<'a>>,
    pub controller_p2: Option<ControllerInputSnapshot<'a>>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct GilrsControllerInputSnapshot<'a> {
    pub pressed_buttons: &'a [GilrsButton],
    pub dpad: DirectionalInput,
    pub left_stick: DirectionalInput,
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct OsHostInputSnapshot<'a> {
    pub pressed_keys: &'a [MinifbKey],
    pub controller_p1: Option<GilrsControllerInputSnapshot<'a>>,
    pub controller_p2: Option<GilrsControllerInputSnapshot<'a>>,
}

pub fn minifb_key_from_binding_name(name: &str) -> Option<MinifbKey> {
    match name {
        "A" => Some(MinifbKey::A),
        "B" => Some(MinifbKey::B),
        "C" => Some(MinifbKey::C),
        "D" => Some(MinifbKey::D),
        "E" => Some(MinifbKey::E),
        "F" => Some(MinifbKey::F),
        "G" => Some(MinifbKey::G),
        "H" => Some(MinifbKey::H),
        "I" => Some(MinifbKey::I),
        "J" => Some(MinifbKey::J),
        "K" => Some(MinifbKey::K),
        "L" => Some(MinifbKey::L),
        "M" => Some(MinifbKey::M),
        "N" => Some(MinifbKey::N),
        "O" => Some(MinifbKey::O),
        "P" => Some(MinifbKey::P),
        "Q" => Some(MinifbKey::Q),
        "R" => Some(MinifbKey::R),
        "S" => Some(MinifbKey::S),
        "T" => Some(MinifbKey::T),
        "U" => Some(MinifbKey::U),
        "V" => Some(MinifbKey::V),
        "W" => Some(MinifbKey::W),
        "X" => Some(MinifbKey::X),
        "Y" => Some(MinifbKey::Y),
        "Z" => Some(MinifbKey::Z),
        "Up" => Some(MinifbKey::Up),
        "Down" => Some(MinifbKey::Down),
        "Left" => Some(MinifbKey::Left),
        "Right" => Some(MinifbKey::Right),
        "Enter" => Some(MinifbKey::Enter),
        "Space" => Some(MinifbKey::Space),
        "LeftShift" => Some(MinifbKey::LeftShift),
        "RightShift" => Some(MinifbKey::RightShift),
        "LeftCtrl" => Some(MinifbKey::LeftCtrl),
        "RightCtrl" => Some(MinifbKey::RightCtrl),
        "Comma" => Some(MinifbKey::Comma),
        "Period" => Some(MinifbKey::Period),
        "Slash" => Some(MinifbKey::Slash),
        "Semicolon" => Some(MinifbKey::Semicolon),
        "Apostrophe" => Some(MinifbKey::Apostrophe),
        "1" => Some(MinifbKey::Key1),
        "2" => Some(MinifbKey::Key2),
        "3" => Some(MinifbKey::Key3),
        "4" => Some(MinifbKey::Key4),
        "5" => Some(MinifbKey::Key5),
        "6" => Some(MinifbKey::Key6),
        "7" => Some(MinifbKey::Key7),
        "8" => Some(MinifbKey::Key8),
        "9" => Some(MinifbKey::Key9),
        "0" => Some(MinifbKey::Key0),
        "Escape" => Some(MinifbKey::Escape),
        "Tab" => Some(MinifbKey::Tab),
        "Backspace" => Some(MinifbKey::Backspace),
        "Delete" => Some(MinifbKey::Delete),
        "Insert" => Some(MinifbKey::Insert),
        "Home" => Some(MinifbKey::Home),
        "End" => Some(MinifbKey::End),
        "PageUp" => Some(MinifbKey::PageUp),
        "PageDown" => Some(MinifbKey::PageDown),
        "Pause" => Some(MinifbKey::Pause),
        "Menu" => Some(MinifbKey::Menu),
        "F1" => Some(MinifbKey::F1),
        "F2" => Some(MinifbKey::F2),
        "F3" => Some(MinifbKey::F3),
        "F4" => Some(MinifbKey::F4),
        "F5" => Some(MinifbKey::F5),
        "F6" => Some(MinifbKey::F6),
        "F7" => Some(MinifbKey::F7),
        "F8" => Some(MinifbKey::F8),
        "F9" => Some(MinifbKey::F9),
        "F10" => Some(MinifbKey::F10),
        "F11" => Some(MinifbKey::F11),
        "F12" => Some(MinifbKey::F12),
        "F13" => Some(MinifbKey::F13),
        "F14" => Some(MinifbKey::F14),
        "F15" => Some(MinifbKey::F15),
        "CapsLock" => Some(MinifbKey::CapsLock),
        "NumLock" => Some(MinifbKey::NumLock),
        "ScrollLock" => Some(MinifbKey::ScrollLock),
        "NumPad0" => Some(MinifbKey::NumPad0),
        "NumPad1" => Some(MinifbKey::NumPad1),
        "NumPad2" => Some(MinifbKey::NumPad2),
        "NumPad3" => Some(MinifbKey::NumPad3),
        "NumPad4" => Some(MinifbKey::NumPad4),
        "NumPad5" => Some(MinifbKey::NumPad5),
        "NumPad6" => Some(MinifbKey::NumPad6),
        "NumPad7" => Some(MinifbKey::NumPad7),
        "NumPad8" => Some(MinifbKey::NumPad8),
        "NumPad9" => Some(MinifbKey::NumPad9),
        "NumPadDot" => Some(MinifbKey::NumPadDot),
        "NumPadSlash" => Some(MinifbKey::NumPadSlash),
        "NumPadAsterisk" => Some(MinifbKey::NumPadAsterisk),
        "NumPadMinus" => Some(MinifbKey::NumPadMinus),
        "NumPadPlus" => Some(MinifbKey::NumPadPlus),
        "NumPadEnter" => Some(MinifbKey::NumPadEnter),
        "LeftAlt" => Some(MinifbKey::LeftAlt),
        "RightAlt" => Some(MinifbKey::RightAlt),
        "LeftSuper" => Some(MinifbKey::LeftSuper),
        "RightSuper" => Some(MinifbKey::RightSuper),
        "Backquote" => Some(MinifbKey::Backquote),
        "Backslash" => Some(MinifbKey::Backslash),
        "Equal" => Some(MinifbKey::Equal),
        "Minus" => Some(MinifbKey::Minus),
        "LeftBracket" => Some(MinifbKey::LeftBracket),
        "RightBracket" => Some(MinifbKey::RightBracket),
        _ => None,
    }
}

pub fn gilrs_button_from_binding_name(name: &str) -> Option<GilrsButton> {
    match name {
        "South" => Some(GilrsButton::South),
        "East" => Some(GilrsButton::East),
        "North" => Some(GilrsButton::North),
        "West" => Some(GilrsButton::West),
        "Start" => Some(GilrsButton::Start),
        "Select" => Some(GilrsButton::Select),
        "Mode" => Some(GilrsButton::Mode),
        "LeftTrigger" => Some(GilrsButton::LeftTrigger),
        "RightTrigger" => Some(GilrsButton::RightTrigger),
        "LeftTrigger2" => Some(GilrsButton::LeftTrigger2),
        "RightTrigger2" => Some(GilrsButton::RightTrigger2),
        "LeftThumb" => Some(GilrsButton::LeftThumb),
        "RightThumb" => Some(GilrsButton::RightThumb),
        "DPadUp" => Some(GilrsButton::DPadUp),
        "DPadDown" => Some(GilrsButton::DPadDown),
        "DPadLeft" => Some(GilrsButton::DPadLeft),
        "DPadRight" => Some(GilrsButton::DPadRight),
        _ => None,
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

pub fn keyboard_pair_from_bindings<F>(
    bindings: &InputBindings,
    mut pressed_key: F,
    turbo_active: bool,
) -> JoypadInputPair
where
    F: FnMut(&str) -> bool,
{
    JoypadInputPair {
        p1: keyboard_state_from_bindings(
            &bindings.keyboard_p1,
            |name| pressed_key(name),
            turbo_active,
        ),
        p2: keyboard_state_from_bindings(
            &bindings.keyboard_p2,
            |name| pressed_key(name),
            turbo_active,
        ),
    }
}

pub fn keyboard_pair_from_minifb_keys<F>(
    bindings: &InputBindings,
    mut is_key_pressed: F,
    turbo_active: bool,
) -> JoypadInputPair
where
    F: FnMut(MinifbKey) -> bool,
{
    keyboard_pair_from_bindings(
        bindings,
        |name| minifb_key_from_binding_name(name).is_some_and(&mut is_key_pressed),
        turbo_active,
    )
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

pub fn controller_state_from_gilrs_buttons<F>(
    bindings: &ControllerBindings,
    mut is_button_pressed: F,
    dpad: DirectionalInput,
    left_stick: DirectionalInput,
    turbo_active: bool,
) -> JoypadInputState
where
    F: FnMut(GilrsButton) -> bool,
{
    controller_state_from_bindings(
        bindings,
        |name| gilrs_button_from_binding_name(name).is_some_and(&mut is_button_pressed),
        dpad,
        left_stick,
        turbo_active,
    )
}

pub fn host_input_pair_from_snapshot(
    bindings: &InputBindings,
    snapshot: HostInputSnapshot<'_>,
    turbo_active: bool,
) -> JoypadInputPair {
    let mut pair = keyboard_pair_from_bindings(
        bindings,
        |key| snapshot.pressed_keys.contains(&key),
        turbo_active,
    );

    if let Some(controller) = snapshot.controller_p1 {
        pair.p1.merge(controller_state_from_bindings(
            &bindings.controller_p1,
            |button| controller.pressed_buttons.contains(&button),
            controller.dpad,
            controller.left_stick,
            turbo_active,
        ));
    }

    if let Some(controller) = snapshot.controller_p2 {
        pair.p2.merge(controller_state_from_bindings(
            &bindings.controller_p2,
            |button| controller.pressed_buttons.contains(&button),
            controller.dpad,
            controller.left_stick,
            turbo_active,
        ));
    }

    pair
}

pub fn host_input_pair_from_os_snapshot(
    bindings: &InputBindings,
    snapshot: OsHostInputSnapshot<'_>,
    turbo_active: bool,
) -> JoypadInputPair {
    let mut pair = keyboard_pair_from_minifb_keys(
        bindings,
        |key| snapshot.pressed_keys.contains(&key),
        turbo_active,
    );

    if let Some(controller) = snapshot.controller_p1 {
        pair.p1.merge(controller_state_from_gilrs_buttons(
            &bindings.controller_p1,
            |button| controller.pressed_buttons.contains(&button),
            controller.dpad,
            controller.left_stick,
            turbo_active,
        ));
    }

    if let Some(controller) = snapshot.controller_p2 {
        pair.p2.merge(controller_state_from_gilrs_buttons(
            &bindings.controller_p2,
            |button| controller.pressed_buttons.contains(&button),
            controller.dpad,
            controller.left_stick,
            turbo_active,
        ));
    }

    pair
}
