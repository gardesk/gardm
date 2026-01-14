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
}

/// Convert a keycode to a character, considering shift state
pub fn keycode_to_char(keycode: u8, state: KeyButMask) -> Option<char> {
    let shifted = state.contains(KeyButMask::SHIFT);

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

    // Apply shift transformations
    let c = if shifted {
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
            // Letters to uppercase
            c if c.is_ascii_lowercase() => c.to_ascii_uppercase(),
            c => c,
        }
    } else {
        base
    };

    Some(c)
}
