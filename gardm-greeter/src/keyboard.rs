//! Keyboard input handling for the greeter
//!
//! Converts X11 keycodes to characters with basic shift support.

use x11rb::protocol::xproto::KeyButMask;

/// Common X11 keycodes (evdev-based, typical for modern Linux)
pub mod keycodes {
    pub const ESCAPE: u8 = 9;
    pub const BACKSPACE: u8 = 22;
    pub const TAB: u8 = 23;
    pub const RETURN: u8 = 36;
    pub const SHIFT_L: u8 = 50;
    pub const SHIFT_R: u8 = 62;
    pub const CAPS_LOCK: u8 = 66;
    pub const LEFT: u8 = 113;
    pub const UP: u8 = 111;
    pub const RIGHT: u8 = 114;
    pub const DOWN: u8 = 116;
    pub const HOME: u8 = 110;
    pub const END: u8 = 115;
    pub const DELETE: u8 = 119;
}

/// Convert a keycode to a character, considering shift and caps lock state
pub fn keycode_to_char(keycode: u8, state: KeyButMask) -> Option<char> {
    let shift_pressed = state.contains(KeyButMask::SHIFT);
    let caps_lock_on = state.contains(KeyButMask::LOCK);

    // Main alphanumeric keys (evdev keycodes)
    let base = match keycode {
        // Number row
        10 => '1',
        11 => '2',
        12 => '3',
        13 => '4',
        14 => '5',
        15 => '6',
        16 => '7',
        17 => '8',
        18 => '9',
        19 => '0',
        20 => '-',
        21 => '=',

        // Top row (QWERTY)
        24 => 'q',
        25 => 'w',
        26 => 'e',
        27 => 'r',
        28 => 't',
        29 => 'y',
        30 => 'u',
        31 => 'i',
        32 => 'o',
        33 => 'p',
        34 => '[',
        35 => ']',

        // Home row (ASDF)
        38 => 'a',
        39 => 's',
        40 => 'd',
        41 => 'f',
        42 => 'g',
        43 => 'h',
        44 => 'j',
        45 => 'k',
        46 => 'l',
        47 => ';',
        48 => '\'',
        51 => '\\',

        // Bottom row (ZXCV)
        52 => 'z',
        53 => 'x',
        54 => 'c',
        55 => 'v',
        56 => 'b',
        57 => 'n',
        58 => 'm',
        59 => ',',
        60 => '.',
        61 => '/',

        // Space
        65 => ' ',

        // Grave/tilde
        49 => '`',

        _ => return None,
    };

    // For letters: uppercase if shift XOR caps_lock (one but not both)
    // For symbols: only shift matters
    let c = if base.is_ascii_lowercase() {
        // Letters: shift XOR caps_lock determines case
        if shift_pressed != caps_lock_on {
            base.to_ascii_uppercase()
        } else {
            base
        }
    } else if shift_pressed {
        // Non-letters: only shift affects them
        match base {
            // Numbers to symbols
            '1' => '!',
            '2' => '@',
            '3' => '#',
            '4' => '$',
            '5' => '%',
            '6' => '^',
            '7' => '&',
            '8' => '*',
            '9' => '(',
            '0' => ')',
            '-' => '_',
            '=' => '+',
            '[' => '{',
            ']' => '}',
            ';' => ':',
            '\'' => '"',
            '\\' => '|',
            ',' => '<',
            '.' => '>',
            '/' => '?',
            '`' => '~',
            c => c,
        }
    } else {
        base
    };

    Some(c)
}
