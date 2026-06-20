use oxidenes::config::{ControllerBindings, KeyboardBindings};
use oxidenes::input_mapping::{
    controller_state_from_bindings, keyboard_state_from_bindings, DirectionalInput,
    JoypadInputState,
};
use oxidenes::joypad::{Joypad, JoypadButton};
use std::collections::BTreeSet;

fn custom_keyboard_bindings() -> KeyboardBindings {
    KeyboardBindings {
        up: "I".to_string(),
        down: "K".to_string(),
        left: "J".to_string(),
        right: "L".to_string(),
        a: "F".to_string(),
        b: "G".to_string(),
        start: "H".to_string(),
        select: "U".to_string(),
        turbo_a: "T".to_string(),
        turbo_b: "Y".to_string(),
    }
}

fn serial_bits(mut joypad: Joypad) -> Vec<u8> {
    joypad.write(1);
    joypad.write(0);
    (0..8).map(|_| joypad.read()).collect()
}

#[test]
fn custom_keyboard_bindings_serialize_to_joypad_bits() {
    let bindings = custom_keyboard_bindings();
    let pressed = BTreeSet::from(["F", "H", "I", "L"]);

    let state = keyboard_state_from_bindings(&bindings, |key| pressed.contains(key), false);

    assert_eq!(state.to_mask(), 0x99);
    assert_eq!(
        state,
        JoypadInputState {
            a: true,
            start: true,
            up: true,
            right: true,
            ..JoypadInputState::default()
        }
    );

    let mut joypad = Joypad::new();
    state.apply_to_joypad(&mut joypad);
    assert!(joypad.get_button(JoypadButton::A));
    assert!(joypad.get_button(JoypadButton::Start));
    assert!(joypad.get_button(JoypadButton::Up));
    assert!(joypad.get_button(JoypadButton::Right));
    assert_eq!(serial_bits(joypad), vec![1, 0, 0, 1, 1, 0, 0, 1]);
}

#[test]
fn keyboard_turbo_bindings_only_press_buttons_when_gate_is_active() {
    let bindings = custom_keyboard_bindings();
    let pressed = BTreeSet::from(["T", "Y"]);

    let inactive = keyboard_state_from_bindings(&bindings, |key| pressed.contains(key), false);
    assert_eq!(inactive.to_mask(), 0x00);

    let active = keyboard_state_from_bindings(&bindings, |key| pressed.contains(key), true);
    assert_eq!(active.to_mask(), 0x03);
}

#[test]
fn controller_bindings_merge_remapped_buttons_dpad_and_stick() {
    let bindings = ControllerBindings {
        a: "East".to_string(),
        b: "North".to_string(),
        turbo_a: "RightTrigger".to_string(),
        turbo_b: "LeftTrigger".to_string(),
        start: "Mode".to_string(),
        select: "LeftThumb".to_string(),
        deadzone: 0.35,
    };
    let pressed = BTreeSet::from(["East", "LeftThumb"]);

    let state = controller_state_from_bindings(
        &bindings,
        |button| pressed.contains(button),
        DirectionalInput {
            down: true,
            ..DirectionalInput::default()
        },
        DirectionalInput {
            right: true,
            ..DirectionalInput::default()
        },
        false,
    );

    assert_eq!(state.to_mask(), 0xA5);
    assert_eq!(
        state,
        JoypadInputState {
            a: true,
            select: true,
            down: true,
            right: true,
            ..JoypadInputState::default()
        }
    );
}

#[test]
fn merged_keyboard_and_controller_states_match_joypad_mask_order() {
    let keyboard = JoypadInputState::from_mask(0x14);
    let controller = JoypadInputState::from_mask(0x83);
    let mut merged = keyboard;

    merged.merge(controller);

    assert_eq!(merged.to_mask(), 0x97);
}
