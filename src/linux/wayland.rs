use std::collections::{HashMap, VecDeque};
use std::convert::TryInto;
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::fd::AsFd;
use std::time::Instant;

use tempfile::tempfile;

// use wayland_client::protocol::wl_output;
use wayland_client::{
    protocol::{wl_pointer, wl_registry, wl_seat},
    Connection, Dispatch, EventQueue, QueueHandle,
};
use wayland_protocols_misc::zwp_input_method_v2::client::{
    zwp_input_method_manager_v2, zwp_input_method_v2,
};
use wayland_protocols_misc::zwp_virtual_keyboard_v1::client::{
    zwp_virtual_keyboard_manager_v1, zwp_virtual_keyboard_v1,
};
use wayland_protocols_plasma::fake_input::client::org_kde_kwin_fake_input;
use wayland_protocols_wlr::virtual_pointer::v1::client::{
    zwlr_virtual_pointer_manager_v1, zwlr_virtual_pointer_v1,
};

use xkbcommon::xkb::{keysym_from_name, keysym_get_name, KEYSYM_NO_FLAGS};

use super::{ConnectionError, Keysym, KEYMAP_BEGINNING, KEYMAP_END};
use crate::{Key, KeyboardControllable, MouseButton, MouseControllable};

pub type Keycode = u32;

struct KeyMap {
    keymap: HashMap<Keysym, Keycode>, // UTF-8 -> (keysym, keycode, refcount)
    unused_keycodes: VecDeque<Keycode>, // Used to keep track of unused keycodes
    needs_regeneration: bool,
    file: Option<std::fs::File>, // Memory mapped temporary file that contains the keymap
    modifiers: u32,
    held: Vec<Key>,
}

impl KeyMap {
    /// Create a new `KeyMap`
    pub fn new() -> Self {
        // Only keycodes from 8 to 255 can be used
        let keymap = HashMap::with_capacity(255 - 7);
        let mut unused_keycodes = VecDeque::with_capacity(255 - 7); // All keycodes are unused when initialized
        for n in 8..=255 {
            unused_keycodes.push_back(n);
        }
        let file = None;
        let needs_regeneration = true;
        let modifiers = 0;
        let held = Vec::with_capacity(255 - 7);
        Self {
            keymap,
            unused_keycodes,
            needs_regeneration,
            file,
            modifiers,
            held,
        }
    }

    fn get_keycode(&mut self, keysym: Keysym) -> Result<Keycode, ConnectionError> {
        if let Some(keycode) = self.keymap.get(&keysym) {
            // The keysym is already mapped and cached in the keymap
            Ok(*keycode)
        } else {
            // The keysym needs to get mapped to an unused keycode
            self.map(keysym) // Always map the keycode if it has not yet
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

    fn map(&mut self, keysym: Keysym) -> Result<Keycode, ConnectionError> {
        match self.unused_keycodes.pop_front() {
            // A keycode is unused so a mapping is possible
            Some(unused_keycode) => {
                self.needs_regeneration = true;
                self.keymap.insert(keysym, unused_keycode);
                Ok(unused_keycode)
            }
            // All keycodes are being used. A mapping is not possible
            None => Err(ConnectionError::MappingFailed(keysym)),
        }
    }

    // Map the the given keycode to the NoSymbol keysym so it can get reused
    fn unmap(&mut self, keysym: Keysym) {
        if let Some(&keycode) = self.keymap.get(&keysym) {
            self.needs_regeneration = true;
            self.unused_keycodes.push_back(keycode);
            self.keymap.remove(&keysym);
        }
    }

    /// Check if there are still unused keycodes available. If there aren't,
    /// make some room by freeing the mapped keycodes Returns true, if keys
    /// were unmapped and the keymap needs to be regenerated
    fn make_room(&mut self) -> bool {
        // Unmap all keys, if all keycodes are already being used
        // TODO: Don't unmap the keycodes if they will be needed next
        // TODO: Don't unmap held keys!
        if self.unused_keycodes.is_empty() {
            let mapped_keys = self.keymap.clone();
            for &sym in mapped_keys.keys() {
                self.unmap(sym);
            }
            return true;
        }
        false
    }

    /// Regenerate the keymap if there were any changes
    /// and write the new keymap to a memory mapped file
    ///
    /// If there was the need to regenerate the keymap, a file descriptor and
    /// the size of the keymap are returned
    pub fn regenerate(&mut self) -> Option<u32> {
        // Don't do anything if there were no changes
        if !self.needs_regeneration {
            return None;
        }

        // Create a file to store the layout
        if self.file.is_none() {
            let mut temp_file = tempfile().expect("Unable to create tempfile");
            temp_file.write_all(KEYMAP_BEGINNING).unwrap();
            self.file = Some(temp_file);
        }

        let keymap_file = self
            .file
            .as_mut()
            .expect("There was no file to write to. This should not be possible!");
        // Move the virtual cursor of the file to the end of the part of the keymap that
        // is always the same so we only overwrite the parts that can change.
        keymap_file
            .seek(SeekFrom::Start(KEYMAP_BEGINNING.len() as u64))
            .unwrap();
        for (&keysym, &keycode) in &self.keymap {
            write!(
                keymap_file,
                "
            key <I{}> {{ [ {} ] }}; // \\n",
                keycode,
                keysym_get_name(keysym)
            )
            .unwrap();
        }
        keymap_file.write_all(KEYMAP_END).unwrap();
        // Truncate the file at the current cursor position in order to cut off any old
        // data in case the keymap was smaller than the old one
        let keymap_len = keymap_file
            .stream_position()
            .expect("Unable to find the position of the cursor in the file");
        keymap_file
            .set_len(keymap_len)
            .expect("Unable to trim the file");
        self.needs_regeneration = false;

        // DEBUG OUTPUT
        // TODO: Delete this once it is confirmed the keymap file is correct
        let mut contents = String::new();
        keymap_file
            .read_to_string(&mut contents)
            .expect("Unable to read the file");
        println!("Content of the keymap file:");
        println!("{contents}");

        Some(keymap_len.try_into().unwrap())
    }

    fn press_modifier(&mut self, pressed_modifier: u32) -> u32 {
        self.modifiers |= pressed_modifier;
        self.modifiers
    }

    fn release_modifier(&mut self, released_modifier: u32) -> u32 {
        self.modifiers &= !released_modifier;
        self.modifiers
    }

    fn pressed(&mut self, key: Key) {
        self.held.push(key);
    }

    fn released(&mut self, key: Key) {
        self.held.retain(|&k| k != key);
    }

    fn held(&mut self) -> Vec<Key> {
        self.held.clone()
    }
}

pub struct Con {
    keymap: KeyMap,
    event_queue: EventQueue<WaylandState>,
    state: WaylandState,
    virtual_keyboard: Option<zwp_virtual_keyboard_v1::ZwpVirtualKeyboardV1>,
    input_method: Option<zwp_input_method_v2::ZwpInputMethodV2>,
    virtual_pointer: Option<zwlr_virtual_pointer_v1::ZwlrVirtualPointerV1>,
    serial: u32,
    base_time: std::time::Instant,
}

impl Con {
    /// Tries to establish a new Wayland connection
    ///
    /// # Errors
    /// TODO
    pub fn new() -> Result<Self, ConnectionError> {
        // Setup Wayland Connection
        let connection = Connection::connect_to_env();
        let connection = match connection {
            Ok(connection) => connection,
            Err(e) => {
                println!(
                    "Failed to connect to Wayland. Try setting 'export WAYLAND_DISPLAY=wayland-0'"
                );
                return Err(ConnectionError::Connection(e.to_string()));
            }
        };

        // Check to see if there was an error trying to connect
        if let Some(err) = connection.protocol_error() {
            //  error!(
            //     "Unknown Wayland initialization failure: {} {} {} {}",
            //      err.code, err.object_id, err.object_interface, err.message
            // );
            return Err(ConnectionError::General(err.to_string()));
        }

        // Create the event queue
        let mut event_queue = connection.new_event_queue();
        // Get queue handle
        let qh = event_queue.handle();

        // Start registry
        let display = connection.display();
        display.get_registry(&qh, ());

        // Setup VKState and dispatch events
        let mut state = WaylandState::new();
        if event_queue.roundtrip(&mut state).is_err() {
            return Err(ConnectionError::General(
                "Roundtrip not possible".to_string(),
            ));
        };

        // Setup virtual keyboard
        let virtual_keyboard = if let Some(seat) = state.seat.as_ref() {
            state
                .keyboard_manager
                .as_ref()
                .map(|vk_mgr| vk_mgr.create_virtual_keyboard(seat, &qh, ()))
        } else {
            None
        };

        // Setup input method
        let input_method = if let Some(seat) = state.seat.as_ref() {
            state
                .im_manager
                .as_ref()
                .map(|im_mgr| im_mgr.get_input_method(seat, &qh, ()))
        } else {
            None
        };

        // Setup virtual pointer
        let virtual_pointer = state
            .pointer_manager
            .as_ref()
            .map(|vp_mgr| vp_mgr.create_virtual_pointer(state.seat.as_ref(), &qh, ()));

        // Try to authenticate for the KDE Fake Input protocol
        if let Some(kde_input) = &state.kde_input {
            let application = "enigo".to_string();
            let reason = "enter keycodes or move the mouse".to_string();
            kde_input.authenticate(application, reason);
        }

        let serial = 0;
        let base_time = Instant::now();

        let keymap = KeyMap::new();

        Ok(Self {
            keymap,
            event_queue,
            state,
            virtual_keyboard,
            input_method,
            virtual_pointer,
            serial,
            base_time,
        })
    }

    /// Used to apply ms timestamps for Wayland key events
    fn get_time(&self) -> u32 {
        let duration = self.base_time.elapsed();
        let time = duration.as_millis();
        time.try_into().unwrap()
    }

    /// Apply XKB layout to virtual keyboard
    /// The layout is an XKB layout as a string (see `generate_keymap_string`())
    /// NOTE: This function does not flush any messages to Wayland, you have to
    /// do that afterwards
    ///
    /// # Errors
    /// TODO
    fn apply_layout(&mut self) {
        if let Some(vk) = &self.virtual_keyboard {
            // Only send an updated keymap if we had to regenerate it
            if let Some(keymap_size) = self.keymap.regenerate() {
                vk.keymap(1, self.keymap.file.as_ref().unwrap().as_fd(), keymap_size);
            }
        }
    }

    /// Press/Release a specific UTF-8 symbol
    /// NOTE: This function does not synchronize the event queue, should be done
    /// immediately after calling (unless you're trying to optimize
    /// scheduling).
    ///
    /// # Errors
    /// TODO
    fn send_key_event(&mut self, keycode: Keycode, press: bool) {
        if let Some(vk) = &self.virtual_keyboard {
            let time = self.get_time();
            let state = u32::from(press);
            let keycode = keycode - 8; // Adjust by 8 due to the xkb/xwayland requirements

            // debug!("time:{} keycode:{}:{} state:{}", time, c, keycode, state);

            // Send key event message
            vk.key(time, keycode, state);
        }
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

        if self.keymap.make_room() {
            self.event_queue.roundtrip(&mut self.state).unwrap();
        }

        let keycode = if let Key::Raw(kc) = key {
            kc.try_into().unwrap()
        } else {
            let sym = KeyMap::key_to_keysym(key);
            self.keymap.get_keycode(sym).unwrap()
        };

        // Apply the new keymap if there were any changes
        self.apply_layout();

        let modifier = Self::is_modifier(key);

        match press {
            None => {
                if let Some(m) = modifier {
                    let modifiers = self.keymap.press_modifier(m);
                    self.send_modifier_event(modifiers);
                    let modifiers = self.keymap.release_modifier(m);
                    self.send_modifier_event(modifiers);
                } else {
                    self.send_key_event(keycode, true);
                    self.send_key_event(keycode, false);
                }
            }
            Some(true) => {
                if let Some(m) = modifier {
                    let modifiers = self.keymap.press_modifier(m);
                    self.send_modifier_event(modifiers);
                } else {
                    self.send_key_event(keycode, true);
                }
                self.keymap.pressed(key);
            }
            Some(false) => {
                if let Some(m) = modifier {
                    let modifiers = self.keymap.release_modifier(m);
                    self.send_modifier_event(modifiers);
                } else {
                    self.send_key_event(keycode, false);
                }
                self.keymap.released(key);
            }
        }
    }

    fn is_modifier(key: Key) -> Option<u32> {
        match key {
            Key::Shift | Key::LShift | Key::RShift => Some(Modifier::Shift as u32),
            Key::CapsLock => Some(Modifier::Lock as u32),
            Key::Control | Key::LControl | Key::RControl => Some(Modifier::Control as u32),
            Key::Alt | Key::LAlt | Key::RAlt | Key::Option => Some(Modifier::Mod1 as u32),
            Key::Numlock => Some(Modifier::Mod2 as u32),
            // Key:: => Some(Modifier::Mod3 as u32),
            Key::Command | Key::Super | Key::Windows | Key::Meta => Some(Modifier::Mod4 as u32),
            Key::ModeChange => Some(Modifier::Mod5 as u32),
            _ => None,
        }
    }

    fn send_modifier_event(&mut self, modifiers: u32) {
        if let Some(vk) = &self.virtual_keyboard {
            vk.modifiers(modifiers, 0, 0, 0);
        }
    }
}

enum Modifier {
    Shift = 0x1,
    Lock = 0x2,
    Control = 0x4,
    Mod1 = 0x8,
    Mod2 = 0x10,
    Mod3 = 0x20,
    Mod4 = 0x40,
    Mod5 = 0x80,
}

impl Drop for Con {
    // Release the held keys before the connection is dropped
    fn drop(&mut self) {
        for &k in &self.keymap.held() {
            self.press_key(k, Some(false));
        }

        if let Some(vk) = &self.virtual_keyboard {
            vk.destroy();
        }
        if let Some(im) = &self.input_method {
            im.destroy();
        }
        if let Some(vp) = &self.virtual_pointer {
            vp.destroy();
        }
    }
}

impl KeyboardControllable for Con {
    fn key_sequence(&mut self, string: &str) {
        // Use the much faster and less error prone input_method protocol if it is
        // available
        if let Some(im) = &self.input_method {
            im.commit_string(string.to_string());
            im.commit(self.serial);
            self.serial = self.serial.wrapping_add(1);
        }
        // otherwise fall back to using the virtual_keyboard method
        else {
            for c in string.chars() {
                self.press_key(Key::Layout(c), None);
            }
        }
        self.event_queue.roundtrip(&mut self.state).unwrap();
    }

    fn key_down(&mut self, key: crate::Key) {
        self.press_key(key, Some(true));
        self.event_queue.roundtrip(&mut self.state).unwrap();
    }

    fn key_up(&mut self, key: crate::Key) {
        self.press_key(key, Some(false));
        self.event_queue.roundtrip(&mut self.state).unwrap();
    }

    fn key_click(&mut self, key: crate::Key) {
        self.press_key(key, Some(true));
        self.press_key(key, Some(false));
        self.event_queue.roundtrip(&mut self.state).unwrap();
    }
}

impl MouseControllable for Con {
    fn mouse_move_to(&mut self, x: i32, y: i32) {
        if let Some(vp) = &self.virtual_pointer {
            let time = self.get_time();
            vp.motion_absolute(
                time,
                x.try_into().unwrap(),
                y.try_into().unwrap(),
                u32::MAX, // TODO: Check what would be the correct value here
                u32::MAX, // TODO: Check what would be the correct value here
            );
            vp.frame(); // TODO: Check if this is needed
        }

        self.event_queue.roundtrip(&mut self.state).unwrap();
    }
    fn mouse_move_relative(&mut self, x: i32, y: i32) {
        if let Some(vp) = &self.virtual_pointer {
            let time = self.get_time();
            vp.motion(time, x as f64, y as f64);
            vp.frame(); // TODO: Check if this is needed
        }
        self.event_queue.roundtrip(&mut self.state).unwrap();
    }
    fn mouse_down(&mut self, button: MouseButton) {
        if let Some(vp) = &self.virtual_pointer {
            let time = self.get_time();
            let button = match button {
                // Taken from /linux/input-event-codes.h
                MouseButton::Left => 0x110,
                MouseButton::Middle => 0x112,
                MouseButton::Right => 0x111,
                MouseButton::ScrollUp => return self.mouse_scroll_y(-1),
                MouseButton::ScrollDown => return self.mouse_scroll_y(1),
                MouseButton::ScrollLeft => return self.mouse_scroll_x(-1),
                MouseButton::ScrollRight => return self.mouse_scroll_x(1),
                MouseButton::Back => 0x116,
                MouseButton::Forward => 0x115,
            };

            vp.button(time, button, wl_pointer::ButtonState::Pressed);
            vp.frame(); // TODO: Check if this is needed
        }
        self.event_queue.roundtrip(&mut self.state).unwrap();
    }
    fn mouse_up(&mut self, button: MouseButton) {
        if let Some(vp) = &self.virtual_pointer {
            let time = self.get_time();
            let button = match button {
                // Taken from /linux/input-event-codes.h
                MouseButton::Left => 0x110,
                MouseButton::Middle => 0x112,
                MouseButton::Right => 0x111,
                // Releasing one of the scroll mouse buttons has no effect
                MouseButton::ScrollUp
                | MouseButton::ScrollDown
                | MouseButton::ScrollLeft
                | MouseButton::ScrollRight => return,
                MouseButton::Back => 0x116,
                MouseButton::Forward => 0x115,
            };
            vp.button(time, button, wl_pointer::ButtonState::Released);
            vp.frame(); // TODO: Check if this is needed
        }
        self.event_queue.roundtrip(&mut self.state).unwrap();
    }
    fn mouse_click(&mut self, button: MouseButton) {
        if let Some(vp) = &self.virtual_pointer {
            let time = self.get_time();
            let button = match button {
                // Taken from /linux/input-event-codes.h
                MouseButton::Left => 0x110,
                MouseButton::Middle => 0x112,
                MouseButton::Right => 0x111,
                MouseButton::ScrollUp => return self.mouse_scroll_y(-1),
                MouseButton::ScrollDown => return self.mouse_scroll_y(1),
                MouseButton::ScrollLeft => return self.mouse_scroll_x(-1),
                MouseButton::ScrollRight => return self.mouse_scroll_x(1),
                MouseButton::Back => 0x116,
                MouseButton::Forward => 0x115,
            };
            vp.button(time, button, wl_pointer::ButtonState::Pressed);
            vp.frame(); // TODO: Check if this is needed
            vp.button(time + 50, button, wl_pointer::ButtonState::Released);
            vp.frame(); // TODO: Check if this is needed
        }
        self.event_queue.roundtrip(&mut self.state).unwrap();
    }
    fn mouse_scroll_x(&mut self, length: i32) {
        if let Some(vp) = &self.virtual_pointer {
            // TODO: Check what the value of length should be
            // TODO: Check if it would be better to use .axis_discrete here
            let time = self.get_time();
            vp.axis(time, wl_pointer::Axis::HorizontalScroll, length.into());
            vp.frame(); // TODO: Check if this is needed
        }
        self.event_queue.roundtrip(&mut self.state).unwrap();
    }
    fn mouse_scroll_y(&mut self, length: i32) {
        if let Some(vp) = &self.virtual_pointer {
            // TODO: Check what the value of length should be
            // TODO: Check if it would be better to use .axis_discrete here
            let time = self.get_time();
            vp.axis(time, wl_pointer::Axis::VerticalScroll, length.into());
            vp.frame(); // TODO: Check if this is needed
        }
        self.event_queue.roundtrip(&mut self.state).unwrap();
    }
    fn main_display_size(&self) -> (i32, i32) {
        //(self.state.width, self.state.height)
        (0, 0)
    }
    fn mouse_location(&self) -> (i32, i32) {
        println!("You tried to get the mouse location. I don't know how this is possible under Wayland. Let me know if there is a new protocol");
        (0, 0)
    }
}

struct WaylandState {
    keyboard_manager: Option<zwp_virtual_keyboard_manager_v1::ZwpVirtualKeyboardManagerV1>,
    im_manager: Option<zwp_input_method_manager_v2::ZwpInputMethodManagerV2>,
    pointer_manager: Option<zwlr_virtual_pointer_manager_v1::ZwlrVirtualPointerManagerV1>,
    kde_input: Option<org_kde_kwin_fake_input::OrgKdeKwinFakeInput>,
    seat: Option<wl_seat::WlSeat>,
    /*  output: Option<wl_output::WlOutput>,
    width: i32,
    height: i32,*/
}

impl WaylandState {
    fn new() -> Self {
        Self {
            keyboard_manager: None,
            im_manager: None,
            pointer_manager: None,
            kde_input: None,
            seat: None,
            /*  output: None,
            width: 0,
            height: 0,*/
        }
    }
}

impl Dispatch<wl_registry::WlRegistry, ()> for WaylandState {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        // When receiving events from the wl_registry, we are only interested in the
        // `global` event, which signals a new available global.
        if let wl_registry::Event::Global {
            name,
            interface,
            version: _,
        } = event
        {
            match &interface[..] {
                "wl_seat" => {
                    let seat = registry.bind::<wl_seat::WlSeat, _, _>(name, 1, qh, ());
                    state.seat = Some(seat);
                }
                /*"wl_output" => {
                    let output = registry.bind::<wl_output::WlOutput, _, _>(name, 1, qh, ());
                    state.output = Some(output);
                }*/
                "zwp_input_method_manager_v2" => {
                    let manager = registry
                        .bind::<zwp_input_method_manager_v2::ZwpInputMethodManagerV2, _, _>(
                            name,
                            1, // TODO: should this be 2?
                            qh,
                            (),
                        );
                    state.im_manager = Some(manager);
                }
                "zwp_virtual_keyboard_manager_v1" => {
                    let manager = registry
                        .bind::<zwp_virtual_keyboard_manager_v1::ZwpVirtualKeyboardManagerV1, _, _>(
                        name,
                        1,
                        qh,
                        (),
                    );
                    state.keyboard_manager = Some(manager);
                }
                "zwlr_virtual_pointer_manager_v1" => {
                    let manager = registry
                        .bind::<zwlr_virtual_pointer_manager_v1::ZwlrVirtualPointerManagerV1, _, _>(
                        name,
                        1,
                        qh,
                        (),
                    );
                    state.pointer_manager = Some(manager);
                }
                "org_kde_kwin_fake_input" => {
                    println!("FAKE_INPUT AVAILABLE!");
                    let kde_input = registry
                        .bind::<org_kde_kwin_fake_input::OrgKdeKwinFakeInput, _, _>(
                            name,
                            1,
                            qh,
                            (),
                        );
                    state.kde_input = Some(kde_input);
                }
                s => {
                    println!("i: {s}");
                }
            }
        }
    }
}

impl Dispatch<zwp_virtual_keyboard_manager_v1::ZwpVirtualKeyboardManagerV1, ()> for WaylandState {
    fn event(
        _state: &mut Self,
        _manager: &zwp_virtual_keyboard_manager_v1::ZwpVirtualKeyboardManagerV1,
        _event: zwp_virtual_keyboard_manager_v1::Event,
        _: &(),
        _: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // println!("Received a virtual keyboard manager event {event:?}");
    }
}

impl Dispatch<zwp_virtual_keyboard_v1::ZwpVirtualKeyboardV1, ()> for WaylandState {
    fn event(
        _state: &mut Self,
        _vk: &zwp_virtual_keyboard_v1::ZwpVirtualKeyboardV1,
        _event: zwp_virtual_keyboard_v1::Event,
        _: &(),
        _: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // println!("Got a virtual keyboard event {event:?}");
    }
}

impl Dispatch<zwp_input_method_manager_v2::ZwpInputMethodManagerV2, ()> for WaylandState {
    fn event(
        _state: &mut Self,
        _manager: &zwp_input_method_manager_v2::ZwpInputMethodManagerV2,
        _event: zwp_input_method_manager_v2::Event,
        _: &(),
        _: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // println!("Received an input method manager event {event:?}");
    }
}
impl Dispatch<zwp_input_method_v2::ZwpInputMethodV2, ()> for WaylandState {
    fn event(
        _state: &mut Self,
        _vk: &zwp_input_method_v2::ZwpInputMethodV2,
        _event: zwp_input_method_v2::Event,
        _: &(),
        _: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // println!("Got a virtual keyboard event {event:?}");
    }
}
impl Dispatch<org_kde_kwin_fake_input::OrgKdeKwinFakeInput, ()> for WaylandState {
    fn event(
        _state: &mut Self,
        _vk: &org_kde_kwin_fake_input::OrgKdeKwinFakeInput,
        _event: org_kde_kwin_fake_input::Event,
        _: &(),
        _: &Connection,
        _qh: &QueueHandle<Self>,
    ) { // This should never happen, as there are no events specified for this
         // in the protocol
         // println!("Got a plasma fake input event {event:?}");
    }
}

impl Dispatch<wl_seat::WlSeat, ()> for WaylandState {
    fn event(
        _state: &mut Self,
        _seat: &wl_seat::WlSeat,
        _event: wl_seat::Event,
        _: &(),
        _: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // println!("Got a seat event {event:?}");
    }
}

/*
impl Dispatch<wl_output::WlOutput, ()> for WaylandState {
    fn event(
        state: &mut Self,
        _output: &wl_output::WlOutput,
        event: wl_output::Event,
        _: &(),
        _: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            wl_output::Event::Geometry {
                x,
                y,
                physical_width,
                physical_height,
                subpixel,
                make,
                model,
                transform,
            } => {
                state.width = x;
                state.height = y;
                println!("x: {x}, y: {y}, physical_width: {physical_width}, physical_height: {physical_height}, make: {make}, model: {model}");
            }
            wl_output::Event::Mode {
                flags,
                width,
                height,
                refresh,
            } => {
                println!("width: {width}, height: {height}");
            }
            _ => {}
        };
    }
}*/

impl Dispatch<zwlr_virtual_pointer_manager_v1::ZwlrVirtualPointerManagerV1, ()> for WaylandState {
    fn event(
        _state: &mut Self,
        _manager: &zwlr_virtual_pointer_manager_v1::ZwlrVirtualPointerManagerV1,
        _event: zwlr_virtual_pointer_manager_v1::Event,
        _: &(),
        _: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // println!("Received a virtual keyboard manager event {event:?}");
    }
}

impl Dispatch<zwlr_virtual_pointer_v1::ZwlrVirtualPointerV1, ()> for WaylandState {
    fn event(
        _state: &mut Self,
        _vk: &zwlr_virtual_pointer_v1::ZwlrVirtualPointerV1,
        _event: zwlr_virtual_pointer_v1::Event,
        _: &(),
        _: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // println!("Got a virtual keyboard event {event:?}");
    }
}

impl Drop for WaylandState {
    fn drop(&mut self) {
        if let Some(im_mgr) = self.im_manager.as_ref() {
            im_mgr.destroy();
        }
        if let Some(pointer_mgr) = self.pointer_manager.as_ref() {
            pointer_mgr.destroy();
        }
    }
}
