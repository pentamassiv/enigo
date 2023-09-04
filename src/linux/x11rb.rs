use std::collections::{HashMap, VecDeque};
use std::convert::TryInto;

use x11rb::protocol::{
    randr::ConnectionExt as _,
    xinput::DeviceUse,
    xproto::{ConnectionExt as _, GetKeyboardMappingReply, Screen},
    xtest::ConnectionExt as _,
};
use x11rb::rust_connection::{DefaultStream, RustConnection};
use x11rb::{connection::Connection, wrapper::ConnectionExt as _};

use xkbcommon::xkb::{keysym_from_name, KEYSYM_NO_FLAGS};

use super::{ConnectionError, Keysym, NO_SYMBOL};
use crate::{Key, KeyboardControllable, MouseButton, MouseControllable};

pub type Keycode = u8;

type CompositorConnection = RustConnection<DefaultStream>;

/// Default delay between chunks of keys that are sent to the X11 server in
/// milliseconds
const DEFAULT_DELAY: u32 = 12;

#[allow(clippy::module_name_repetitions)]
pub struct Con {
    connection: CompositorConnection,
    keymap: HashMap<Keysym, Keycode>,
    unused_keycodes: VecDeque<Keycode>,
    delay: u32, // milliseconds
    screen: Screen,
    held: Vec<(Key, Keysym)>,                     // Currently held keys
    last_keys: Vec<Keycode>,                      // Last pressed keycodes
    last_event_before_delays: std::time::Instant, // Time of the last event
    pending_delays: u32,
}

impl Default for Con {
    fn default() -> Self {
        Self::new(DEFAULT_DELAY)
    }
}

impl Con {
    /// Tries to establish a new X11 connection
    ///
    /// delay: Minimum delay in milliseconds between keypresses in order to
    /// properly enter all chars
    ///
    /// # Errors
    /// TODO
    pub fn new(delay: u32) -> Con {
        let (connection, screen_idx) = x11rb::connect(None).unwrap();
        let setup = connection.setup();
        let screen = setup.roots[screen_idx].clone();
        let min_keycode = setup.min_keycode;
        let max_keycode = setup.max_keycode;
        let keymap = HashMap::with_capacity((max_keycode - min_keycode) as usize);
        let unused_keycodes = Self::find_unused_keycodes(&connection, min_keycode, max_keycode);
        // Check if a mapping is possible
        assert!(
            !(unused_keycodes.is_empty()),
            "There was no space to map any keycodes"
        );
        let held = vec![];
        let last_keys = vec![];
        let last_event_before_delays = std::time::Instant::now();
        let pending_delays = 0;
        Con {
            connection,
            keymap,
            unused_keycodes,
            held,
            delay,
            screen,
            last_keys,
            last_event_before_delays,
            pending_delays,
        }
    }

    /// Get the delay per keypress in milliseconds.
    /// Default value is 12 ms.
    /// This is Linux-specific.
    #[must_use]
    pub fn delay(&self) -> u32 {
        self.delay
    }
    /// Set the delay in milliseconds per keypress.
    /// This is Linux-specific.
    pub fn set_delay(&mut self, delay: u32) {
        self.delay = delay;
    }

    fn find_unused_keycodes(
        connection: &CompositorConnection,
        keycode_min: Keycode,
        keycode_max: Keycode,
    ) -> VecDeque<Keycode> {
        let mut unused_keycodes: VecDeque<Keycode> =
            VecDeque::with_capacity((keycode_max - keycode_min) as usize);

        let GetKeyboardMappingReply {
            keysyms_per_keycode,
            keysyms,
            ..
        } = connection
            .get_keyboard_mapping(keycode_min, keycode_max - keycode_min)
            .unwrap()
            .reply()
            .unwrap();

        print_keymap(keysyms.clone());

        // Split the mapping into the chunks of keysyms that are mapped to each keycode
        let keysyms = keysyms.chunks(keysyms_per_keycode as usize);
        for (syms, kc) in keysyms.zip(keycode_min..=keycode_max) {
            // Check if the keycode is unused
            if syms.iter().all(|&s| s == NO_SYMBOL.raw()) {
                unused_keycodes.push_back(kc);
            }
        }
        unused_keycodes
    }

    fn get_keycode(&mut self, keysym: Keysym) -> Result<Keycode, ConnectionError> {
        if let Some(keycode) = self.keymap.get(&keysym) {
            // The keysym is already mapped and cached in the keymap
            Ok(*keycode)
        } else {
            // The keysym needs to get mapped to an unused keycode
            self.map_sym(keysym) // Always map the keycode if it has not yet
                                 // been mapped, so it is layer agnostic
        }
    }

    fn key_to_keysym(key: Key) -> Keysym {
        match key {
            Key::Layout(c) => match c {
                '\n' => Keysym::Return,
                '\t' => Keysym::Tab,
                _ => {
                    // TODO: Replace with Keysym.from_char(ch: char)
                    let hex: u32 = c.into();
                    let name = format!("U{hex:x}");
                    keysym_from_name(&name, KEYSYM_NO_FLAGS)
                }
            },
            Key::Raw(k) => {
                // Raw keycodes cannot be converted to keysyms
                panic!("Attempted to convert raw keycode {k} to keysym");
            }
            Key::Alt | Key::LAlt | Key::Option => Keysym::Alt_L,
            Key::Backspace => Keysym::BackSpace,
            Key::Begin => Keysym::Begin,
            Key::Break => Keysym::Break,
            Key::Cancel => Keysym::Cancel,
            Key::CapsLock => Keysym::Caps_Lock,
            Key::Clear => Keysym::Clear,
            Key::Control | Key::LControl => Keysym::Control_L,
            Key::Delete => Keysym::Delete,
            Key::DownArrow => Keysym::Down,
            Key::End => Keysym::End,
            Key::Escape => Keysym::Escape,
            Key::Execute => Keysym::Execute,
            Key::F1 => Keysym::F1,
            Key::F2 => Keysym::F2,
            Key::F3 => Keysym::F3,
            Key::F4 => Keysym::F4,
            Key::F5 => Keysym::F5,
            Key::F6 => Keysym::F6,
            Key::F7 => Keysym::F7,
            Key::F8 => Keysym::F8,
            Key::F9 => Keysym::F9,
            Key::F10 => Keysym::F10,
            Key::F11 => Keysym::F11,
            Key::F12 => Keysym::F12,
            Key::F13 => Keysym::F13,
            Key::F14 => Keysym::F14,
            Key::F15 => Keysym::F15,
            Key::F16 => Keysym::F16,
            Key::F17 => Keysym::F17,
            Key::F18 => Keysym::F18,
            Key::F19 => Keysym::F19,
            Key::F20 => Keysym::F20,
            Key::F21 => Keysym::F21,
            Key::F22 => Keysym::F22,
            Key::F23 => Keysym::F23,
            Key::F24 => Keysym::F24,
            Key::F25 => Keysym::F25,
            Key::F26 => Keysym::F26,
            Key::F27 => Keysym::F27,
            Key::F28 => Keysym::F28,
            Key::F29 => Keysym::F29,
            Key::F30 => Keysym::F30,
            Key::F31 => Keysym::F31,
            Key::F32 => Keysym::F32,
            Key::F33 => Keysym::F33,
            Key::F34 => Keysym::F34,
            Key::F35 => Keysym::F35,
            Key::Find => Keysym::Find,
            Key::Hangul => Keysym::Hangul,
            Key::Hanja => Keysym::Hangul_Hanja,
            Key::Help => Keysym::Help,
            Key::Home => Keysym::Home,
            Key::Insert => Keysym::Insert,
            Key::Kanji => Keysym::Kanji,
            Key::LeftArrow => Keysym::Left,
            Key::Linefeed => Keysym::Linefeed,
            Key::LMenu => Keysym::Menu,
            Key::ModeChange => Keysym::Mode_switch,
            Key::Numlock => Keysym::Num_Lock,
            Key::PageDown => Keysym::Page_Down,
            Key::PageUp => Keysym::Page_Up,
            Key::Pause => Keysym::Pause,
            Key::Print => Keysym::Print,
            Key::RAlt => Keysym::Alt_R,
            Key::RControl => Keysym::Control_R,
            Key::Redo => Keysym::Redo,
            Key::Return => Keysym::Return,
            Key::RightArrow => Keysym::Right,
            Key::RShift => Keysym::Shift_R,
            Key::ScrollLock => Keysym::Scroll_Lock,
            Key::Select => Keysym::Select,
            Key::ScriptSwitch => Keysym::script_switch,
            Key::Shift | Key::LShift => Keysym::Shift_L,
            Key::ShiftLock => Keysym::Shift_Lock,
            Key::Space => Keysym::space,
            Key::SysReq => Keysym::Sys_Req,
            Key::Tab => Keysym::Tab,
            Key::Undo => Keysym::Undo,
            Key::UpArrow => Keysym::Up,
            Key::Command | Key::Super | Key::Windows | Key::Meta => Keysym::Super_L,
        }
    }

    fn map_sym(&mut self, keysym: Keysym) -> Result<Keycode, ConnectionError> {
        match self.unused_keycodes.pop_front() {
            // A keycode is unused so a mapping is possible
            Some(unused_keycode) => {
                self.bind_key(unused_keycode, keysym);
                self.keymap.insert(keysym, unused_keycode);
                println!("keymap insert: {:?}", self.keymap);
                println!();

                let GetKeyboardMappingReply { keysyms, .. } = self
                    .connection
                    .get_keyboard_mapping(8, 255 - 8)
                    .unwrap()
                    .reply()
                    .unwrap();
                print_keymap(keysyms);
                println!("---------------");
                Ok(unused_keycode)
            }
            // All keycodes are being used. A mapping is not possible
            None => Err(ConnectionError::MappingFailed(keysym)),
        }
    }

    // Map the the given keycode to the NoSymbol keysym so it can get reused
    fn unmap_sym(&mut self, keysym: Keysym) {
        if let Some(&keycode) = self.keymap.get(&keysym) {
            self.bind_key(keycode, NO_SYMBOL);
            self.unused_keycodes.push_back(keycode);
            self.keymap.remove(&keysym);
            println!("keymap remove: {:?}", self.keymap);
            println!();

            let GetKeyboardMappingReply { keysyms, .. } = self
                .connection
                .get_keyboard_mapping(8, 255 - 8)
                .unwrap()
                .reply()
                .unwrap();
            print_keymap(keysyms);
            println!("---------------");
        }
    }

    // Map the keysym to the given keycode
    // Only use keycodes that are not used, otherwise the existing mapping is
    // overwritten
    // If the keycode is mapped to the NoSymbol keysym, the key is unbinded and can
    // get used again later
    fn bind_key(&self, keycode: Keycode, keysym: Keysym) {
        // A list of two keycodes has to be mapped, otherwise the map is not what would
        // be expected If we would try to map only one keysym, we would get a
        // map that is tolower(keysym), toupper(keysym), tolower(keysym),
        // toupper(keysym), tolower(keysym), toupper(keysym), 0, 0, 0, 0, ...
        // https://stackoverflow.com/a/44334103
        self.connection
            .change_keyboard_mapping(1, keycode, 2, &[keysym.raw(), keysym.raw()])
            .unwrap();
        self.connection.sync().unwrap();
    }

    // Update the delay
    // TODO: A delay of 1 ms in all cases seems to work on my machine. Maybe this is
    // not needed?
    fn update_delays(&mut self, keycode: Keycode) {
        // Check if a delay is needed
        // A delay is required, if one of the keycodes was recently entered and there
        // was no delay between it

        // e.g. A quick rabbit
        // Chunk 1: 'A quick' # Add a delay before the second space
        // Chunk 2: ' rab'     # Add a delay before the second 'b'
        // Chunk 3: 'bit'     # Enter the remaining chars

        if self.last_keys.contains(&keycode) {
            let elapsed_ms = self
                .last_event_before_delays
                .elapsed()
                .as_millis()
                .try_into()
                .unwrap();
            self.pending_delays = self.delay.saturating_sub(elapsed_ms);
            self.last_keys.clear();
        } else {
            self.pending_delays = 1;
        }
        self.last_keys.push(keycode);
    }

    /// Sends a key event to the X11 server via XTest extension
    fn send_key_event(&mut self, keycode: Keycode, press: bool) {
        let type_ = if press {
            x11rb::protocol::xproto::KEY_PRESS_EVENT
        } else {
            x11rb::protocol::xproto::KEY_RELEASE_EVENT
        };
        let detail = keycode;
        let time = self.pending_delays;
        let root = self.screen.root;
        let root_x = 0;
        let root_y = 0;
        let deviceid = x11rb::protocol::xinput::list_input_devices(&self.connection)
            .unwrap()
            .reply()
            .unwrap()
            .devices
            .iter()
            .find(|d| d.device_use == DeviceUse::IS_X_KEYBOARD)
            .map(|d| d.device_id)
            .unwrap();

        self.connection
            .xtest_fake_input(type_, detail, time, root, root_x, root_y, deviceid)
            .unwrap();
        self.connection.sync().unwrap();
        self.last_event_before_delays = std::time::Instant::now();
    }

    // Try to enter the key
    // If press is None, it is assumed that the key is pressed and released
    // If press is true, the key is pressed
    // Otherwise the key is released
    fn press_key(&mut self, key: Key, press: Option<bool>) {
        // Nothing to do
        if key == Key::Layout('\0') {
            return;
        }

        // Unmap all keys, if all keycodes are already being used
        // TODO: Don't unmap the keycodes if they will be needed next
        if self.unused_keycodes.is_empty() {
            let mapped_keys = self.keymap.clone();
            for &sym in mapped_keys.keys() {
                if !self.held.iter().any(|(_, s)| *s == sym) {
                    self.unmap_sym(sym);
                }
            }
        }

        let (sym, keycode) = if let Key::Raw(kc) = key {
            (None, kc.try_into().unwrap())
        } else {
            let sym = Self::key_to_keysym(key);
            let keycode = self.get_keycode(sym).unwrap();
            (Some(sym), keycode)
        };

        match press {
            None => {
                self.update_delays(keycode);
                self.send_key_event(keycode, true);
                self.send_key_event(keycode, false);
            }
            Some(true) => {
                self.update_delays(keycode);
                self.send_key_event(keycode, true);
                if let Some(sym) = sym {
                    self.held.push((key, sym));
                }
            }
            Some(false) => {
                // self.update_delays(keycode); TODO: Check if releases really don't need a
                // delay
                self.send_key_event(keycode, false);
                // if let Some(s) = sym {
                //    self.unmap_sym(s);
                // }
                self.held.retain(|&(k, _)| k != key);
            }
        }
    }

    // Sends a button event to the X11 server via XTest extension
    fn send_mouse_button_event(&self, button: MouseButton, press: bool, delay: u32) {
        let type_ = if press {
            x11rb::protocol::xproto::BUTTON_PRESS_EVENT
        } else {
            x11rb::protocol::xproto::BUTTON_RELEASE_EVENT
        };
        let detail = match button {
            MouseButton::Left => 1,
            MouseButton::Middle => 2,
            MouseButton::Right => 3,
            MouseButton::ScrollUp => 4,
            MouseButton::ScrollDown => 5,
            MouseButton::ScrollLeft => 6,
            MouseButton::ScrollRight => 7,
            MouseButton::Back => 8,
            MouseButton::Forward => 9,
        };
        let time = delay;
        let root = self.screen.root;
        let root_x = 0;
        let root_y = 0;
        let deviceid = x11rb::protocol::xinput::list_input_devices(&self.connection)
            .unwrap()
            .reply()
            .unwrap()
            .devices
            .iter()
            .find(|d| d.device_use == DeviceUse::IS_X_POINTER)
            .map(|d| d.device_id)
            .unwrap();
        self.connection
            .xtest_fake_input(type_, detail, time, root, root_x, root_y, deviceid)
            .unwrap();

        self.connection.sync().unwrap();
    }

    // Sends a motion notify event to the X11 server via XTest extension
    // TODO: Check if using x11rb::protocol::xproto::warp_pointer would be better
    fn send_motion_notify_event(&self, x: i32, y: i32, relative: bool) {
        let type_ = x11rb::protocol::xproto::MOTION_NOTIFY_EVENT;
        // TRUE -> relative coordinates
        // FALSE -> absolute coordinates
        let detail = u8::from(relative);
        let time = x11rb::CURRENT_TIME;
        let root = x11rb::NONE; //  the root window of the screen the pointer is currently on
        let root_x = x.try_into().unwrap();
        let root_y = y.try_into().unwrap();
        let deviceid = x11rb::protocol::xinput::list_input_devices(&self.connection)
            .unwrap()
            .reply()
            .unwrap()
            .devices
            .iter()
            .find(|d| d.device_use == DeviceUse::IS_X_POINTER)
            .map(|d| d.device_id)
            .unwrap();
        self.connection
            .xtest_fake_input(type_, detail, time, root, root_x, root_y, deviceid)
            .unwrap();
        self.connection.sync().unwrap();
    }
}

impl Drop for Con {
    // Release the held keys before the connection is dropped
    fn drop(&mut self) {
        for &(k, _) in &self.held.clone() {
            self.press_key(k, Some(false));
        }

        // This is not needed on wayland with the virtual keyboard protocol,
        // because we create a new keymap just for this protocol so we don't
        // care about it's state as soon as we no longer use it
        for &keycode in self.keymap.values() {
            // Map the the given keycode
            // to the NoSymbol keysym so
            // it can get reused
            self.bind_key(keycode, NO_SYMBOL);
        }
        println!();

        let GetKeyboardMappingReply { keysyms, .. } = self
            .connection
            .get_keyboard_mapping(8, 255 - 8)
            .unwrap()
            .reply()
            .unwrap();

        print_keymap(keysyms);
        println!("---------------");
    }
}

impl KeyboardControllable for Con {
    fn key_sequence(&mut self, string: &str) {
        for c in string.chars() {
            self.press_key(Key::Layout(c), None);
        }
    }

    fn key_down(&mut self, key: crate::Key) {
        self.press_key(key, Some(true));
    }

    fn key_up(&mut self, key: crate::Key) {
        self.press_key(key, Some(false));
    }

    fn key_click(&mut self, key: crate::Key) {
        self.press_key(key, Some(true));
        self.press_key(key, Some(false));
    }
}

impl MouseControllable for Con {
    fn mouse_move_to(&mut self, x: i32, y: i32) {
        self.send_motion_notify_event(x, y, false);
    }

    fn mouse_move_relative(&mut self, x: i32, y: i32) {
        self.send_motion_notify_event(x, y, true);
    }

    fn mouse_down(&mut self, button: MouseButton) {
        self.send_mouse_button_event(button, true, 1);
    }

    fn mouse_up(&mut self, button: MouseButton) {
        self.send_mouse_button_event(button, false, 1);
    }

    fn mouse_click(&mut self, button: MouseButton) {
        self.send_mouse_button_event(button, true, 1);
        self.send_mouse_button_event(button, false, DEFAULT_DELAY);
    }

    fn mouse_scroll_x(&mut self, length: i32) {
        let mut length = length;
        let button = if length < 0 {
            MouseButton::ScrollLeft
        } else {
            MouseButton::ScrollRight
        };

        if length < 0 {
            length = -length;
        }

        for _ in 0..length {
            self.mouse_click(button);
        }
    }
    fn mouse_scroll_y(&mut self, length: i32) {
        let mut length = length;
        let button = if length < 0 {
            MouseButton::ScrollUp
        } else {
            MouseButton::ScrollDown
        };

        if length < 0 {
            length = -length;
        }

        for _ in 0..length {
            self.mouse_click(button);
        }
    }

    fn main_display_size(&self) -> (i32, i32) {
        let main_display = self
            .connection
            .randr_get_screen_resources(self.screen.root)
            .unwrap()
            .reply()
            .unwrap()
            .modes[0];

        (main_display.width as i32, main_display.height as i32)
    }

    fn mouse_location(&self) -> (i32, i32) {
        let reply = self
            .connection
            .query_pointer(self.screen.root)
            .unwrap()
            .reply()
            .unwrap();
        (reply.root_x as i32, reply.root_y as i32)
    }
}

fn print_keymap(keysyms: Vec<u32>) {
    let keysyms = keysyms.chunks(4);
    for (syms, kc) in keysyms.zip(8..=255) {
        println!("{} {:?}", kc, syms);
    }
}
