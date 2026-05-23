use oxidenes::joypad::{Joypad, JoypadButton};

#[test]
fn new_joypad_all_buttons_released() {
    let joypad = Joypad::new();
    assert!(!joypad.get_button(JoypadButton::A));
    assert!(!joypad.get_button(JoypadButton::B));
    assert!(!joypad.get_button(JoypadButton::Select));
    assert!(!joypad.get_button(JoypadButton::Start));
    assert!(!joypad.get_button(JoypadButton::Up));
    assert!(!joypad.get_button(JoypadButton::Down));
    assert!(!joypad.get_button(JoypadButton::Left));
    assert!(!joypad.get_button(JoypadButton::Right));
}

#[test]
fn set_and_get_button() {
    let mut joypad = Joypad::new();
    let buttons = [
        JoypadButton::A,
        JoypadButton::B,
        JoypadButton::Select,
        JoypadButton::Start,
        JoypadButton::Up,
        JoypadButton::Down,
        JoypadButton::Left,
        JoypadButton::Right,
    ];
    for &button in &buttons {
        joypad.set_button_pressed(button, true);
        assert!(joypad.get_button(button), "{:?} should be pressed", button);
        joypad.set_button_pressed(button, false);
    }
}

#[test]
fn strobe_resets_read_index() {
    let mut joypad = Joypad::new();
    // Press all buttons so every read returns 1
    let buttons = [
        JoypadButton::A,
        JoypadButton::B,
        JoypadButton::Select,
        JoypadButton::Start,
        JoypadButton::Up,
        JoypadButton::Down,
        JoypadButton::Left,
        JoypadButton::Right,
    ];
    for &button in &buttons {
        joypad.set_button_pressed(button, true);
    }

    // Strobe: write(1) then write(0) resets index to 0
    joypad.write(1);
    joypad.write(0);

    // Read all 8 buttons in order: A(0), B(1), Select(2), Start(3), Up(4), Down(5), Left(6), Right(7)
    for &button in &buttons {
        assert_eq!(joypad.read(), 1, "Expected 1 for {:?}", button);
    }
}

#[test]
fn strobe_high_always_returns_first_button() {
    let mut joypad = Joypad::new();
    joypad.set_button_pressed(JoypadButton::A, true);

    // Set strobe high
    joypad.write(1);

    // While strobe is 1, read() always returns the A button state
    for _ in 0..5 {
        assert_eq!(
            joypad.read(),
            1,
            "Strobe high should always return A button"
        );
    }

    // Verify with A released
    joypad.set_button_pressed(JoypadButton::A, false);
    for _ in 0..5 {
        assert_eq!(
            joypad.read(),
            0,
            "Strobe high with A released should return 0"
        );
    }
}

#[test]
fn read_after_8_buttons_returns_1() {
    let mut joypad = Joypad::new();
    // All buttons released — reads should be 0 for first 8, then 1

    // Reset read index
    joypad.write(1);
    joypad.write(0);

    // Read the 8 button states (all 0)
    for _ in 0..8 {
        joypad.read();
    }

    // After 8 reads, subsequent reads return 1
    for _ in 0..5 {
        assert_eq!(joypad.read(), 1, "Reads past 8 buttons should return 1");
    }
}

#[test]
fn multiple_buttons_pressed() {
    let mut joypad = Joypad::new();
    // Press A (index 0), Start (index 3), Right (index 7)
    joypad.set_button_pressed(JoypadButton::A, true);
    joypad.set_button_pressed(JoypadButton::Start, true);
    joypad.set_button_pressed(JoypadButton::Right, true);

    // Reset read index
    joypad.write(1);
    joypad.write(0);

    let expected = [1, 0, 0, 1, 0, 0, 0, 1]; // A=1, B=0, Sel=0, Start=1, U=0, D=0, L=0, R=1
    for (i, &exp) in expected.iter().enumerate() {
        assert_eq!(joypad.read(), exp, "Mismatch at button index {}", i);
    }
}

#[test]
fn release_button() {
    let mut joypad = Joypad::new();

    joypad.set_button_pressed(JoypadButton::Start, true);
    assert!(
        joypad.get_button(JoypadButton::Start),
        "Start should be pressed"
    );

    joypad.set_button_pressed(JoypadButton::Start, false);
    assert!(
        !joypad.get_button(JoypadButton::Start),
        "Start should be released"
    );
}

#[test]
fn strobe_write_only_uses_bit_0() {
    let mut joypad = Joypad::new();
    joypad.set_button_pressed(JoypadButton::A, true);

    // Writing 0xFF has bit 0 set → strobe active, read always returns A
    joypad.write(0xFF);
    assert_eq!(joypad.read(), 1);
    assert_eq!(joypad.read(), 1); // still returns A (index not advancing)

    // Writing 0xFE has bit 0 clear → strobe off, index advances
    joypad.write(0xFE);
    let first = joypad.read(); // A (index 0)
    let second = joypad.read(); // B (index 1)
    assert_eq!(first, 1, "A is pressed");
    assert_eq!(second, 0, "B is not pressed — index advanced");
}
