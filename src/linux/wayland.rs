use std::collections::{HashMap, VecDeque};
use std::convert::TryInto;
use std::io::{Seek, SeekFrom, Write};
use std::os::unix::io::IntoRawFd;
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

use xkbcommon::xkb::{keysym_from_name, keysym_get_name, keysyms, KEYSYM_NO_FLAGS};

use super::{ConnectionError, Keysym};
use crate::{Key, KeyboardControllable, MouseButton, MouseControllable};

pub type Keycode = u32;

type CompositorConnection = Connection;

pub struct Con {
    connection: Connection,
    keymap: HashMap<Keysym, Keycode>, // UTF-8 -> (keysym, keycode, refcount)
    unused_keycodes: VecDeque<Keycode>, // Used to keep track of unused keycodes
    event_queue: EventQueue<WaylandState>,
    state: WaylandState,
    virtual_keyboard: Option<zwp_virtual_keyboard_v1::ZwpVirtualKeyboardV1>,
    input_method: Option<zwp_input_method_v2::ZwpInputMethodV2>,
    serial: u32,
    virtual_pointer: Option<zwlr_virtual_pointer_v1::ZwlrVirtualPointerV1>,
    needs_regeneration: bool,
    modifiers: u32,
    held: Vec<Key>,
    base_time: std::time::Instant,
}

impl Con {
    /// Tries to establish a new Wayland connection
    ///
    /// # Errors
    /// TODO
    pub fn new() -> Result<Con, ConnectionError> {
        println!("Try wayland connection");
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
            if let Some(vk_mgr) = state.keyboard_manager.as_ref() {
                println!("Created vk mgr");
                Some(vk_mgr.create_virtual_keyboard(seat, &qh, ()))
            } else {
                None
            }
        } else {
            None
        };

        // Setup input method
        let input_method = if let Some(seat) = state.seat.as_ref() {
            if let Some(im_mgr) = state.im_manager.as_ref() {
                println!("Created im mgr");
                Some(im_mgr.get_input_method(seat, &qh, ()))
            } else {
                None
            }
        } else {
            None
        };
        let serial = 0;

        // Setup virtual pointer
        let virtual_pointer = if let Some(vp_mgr) = state.pointer_manager.as_ref() {
            Some(vp_mgr.create_virtual_pointer(state.seat.as_ref(), &qh, ()))
        } else {
            None
        };

        // Try to authenticate for the KDE Fake Input protocol
        if let Some(kde_input) = &state.kde_input {
            let application = "enigo".to_string();
            let reason = "enter keycodes or move the mouse".to_string();
            kde_input.authenticate(application, reason);
        }

        // Only keycodes from 8 to 255 can be used
        let keymap = HashMap::with_capacity(255 - 7);
        let mut unused_keycodes = VecDeque::with_capacity(255 - 7); // All keycodes are unused when initialized
        for n in 8..=255 {
            unused_keycodes.push_back(n);
        }
        let needs_regeneration = true;
        let modifiers = 0x0;
        let held = Vec::with_capacity(255 - 7);
        let base_time = Instant::now();
        Ok(Con {
            connection,
            keymap,
            unused_keycodes,
            held,
            event_queue,
            state,
            needs_regeneration,
            modifiers,
            base_time,
            virtual_keyboard,
            input_method,
            serial,
            virtual_pointer,
        })
    }

    /// Generates a single-level keymap.
    ///
    /// # Errors
    /// The only way this can throw an error is if the generated String is not
    /// valid UTF8
    fn generate_keymap_string(&mut self) -> Result<String, ConnectionError> {
        let mut buf: Vec<u8> = Vec::new();
        writeln!(
            buf,
            "xkb_keymap {{
        xkb_keycodes {{
            minimum = 8;
            maximum = 255;
            
            <I8> = 8;
            <I9> = 9;
            <I10> = 10;
            <I11> = 11;
            <I12> = 12;
            <I13> = 13;
            <I14> = 14;
            <I15> = 15;
            <I16> = 16;
            <I17> = 17;
            <I18> = 18;
            <I19> = 19;
            <I20> = 20;
            <I21> = 21;
            <I22> = 22;
            <I23> = 23;
            <I24> = 24;
            <I25> = 25;
            <I26> = 26;
            <I27> = 27;
            <I28> = 28;
            <I29> = 29;
            <I30> = 30;
            <I31> = 31;
            <I32> = 32;
            <I33> = 33;
            <I34> = 34;
            <I35> = 35;
            <I36> = 36;
            <I37> = 37;
            <I38> = 38;
            <I39> = 39;
            <I40> = 40;
            <I41> = 41;
            <I42> = 42;
            <I43> = 43;
            <I44> = 44;
            <I45> = 45;
            <I46> = 46;
            <I47> = 47;
            <I48> = 48;
            <I49> = 49;
            <I50> = 50;
            <I51> = 51;
            <I52> = 52;
            <I53> = 53;
            <I54> = 54;
            <I55> = 55;
            <I56> = 56;
            <I57> = 57;
            <I58> = 58;
            <I59> = 59;
            <I60> = 60;
            <I61> = 61;
            <I62> = 62;
            <I63> = 63;
            <I64> = 64;
            <I65> = 65;
            <I66> = 66;
            <I67> = 67;
            <I68> = 68;
            <I69> = 69;
            <I70> = 70;
            <I71> = 71;
            <I72> = 72;
            <I73> = 73;
            <I74> = 74;
            <I75> = 75;
            <I76> = 76;
            <I77> = 77;
            <I78> = 78;
            <I79> = 79;
            <I80> = 80;
            <I81> = 81;
            <I82> = 82;
            <I83> = 83;
            <I84> = 84;
            <I85> = 85;
            <I86> = 86;
            <I87> = 87;
            <I88> = 88;
            <I89> = 89;
            <I90> = 90;
            <I91> = 91;
            <I92> = 92;
            <I93> = 93;
            <I94> = 94;
            <I95> = 95;
            <I96> = 96;
            <I97> = 97;
            <I98> = 98;
            <I99> = 99;
            <I100> = 100;
            <I101> = 101;
            <I102> = 102;
            <I103> = 103;
            <I104> = 104;
            <I105> = 105;
            <I106> = 106;
            <I107> = 107;
            <I108> = 108;
            <I109> = 109;
            <I110> = 110;
            <I111> = 111;
            <I112> = 112;
            <I113> = 113;
            <I114> = 114;
            <I115> = 115;
            <I116> = 116;
            <I117> = 117;
            <I118> = 118;
            <I119> = 119;
            <I120> = 120;
            <I121> = 121;
            <I122> = 122;
            <I123> = 123;
            <I124> = 124;
            <I125> = 125;
            <I126> = 126;
            <I127> = 127;
            <I128> = 128;
            <I129> = 129;
            <I130> = 130;
            <I131> = 131;
            <I132> = 132;
            <I133> = 133;
            <I134> = 134;
            <I135> = 135;
            <I136> = 136;
            <I137> = 137;
            <I138> = 138;
            <I139> = 139;
            <I140> = 140;
            <I141> = 141;
            <I142> = 142;
            <I143> = 143;
            <I144> = 144;
            <I145> = 145;
            <I146> = 146;
            <I147> = 147;
            <I148> = 148;
            <I149> = 149;
            <I150> = 150;
            <I151> = 151;
            <I152> = 152;
            <I153> = 153;
            <I154> = 154;
            <I155> = 155;
            <I156> = 156;
            <I157> = 157;
            <I158> = 158;
            <I159> = 159;
            <I160> = 160;
            <I161> = 161;
            <I162> = 162;
            <I163> = 163;
            <I164> = 164;
            <I165> = 165;
            <I166> = 166;
            <I167> = 167;
            <I168> = 168;
            <I169> = 169;
            <I170> = 170;
            <I171> = 171;
            <I172> = 172;
            <I173> = 173;
            <I174> = 174;
            <I175> = 175;
            <I176> = 176;
            <I177> = 177;
            <I178> = 178;
            <I179> = 179;
            <I180> = 180;
            <I181> = 181;
            <I182> = 182;
            <I183> = 183;
            <I184> = 184;
            <I185> = 185;
            <I186> = 186;
            <I187> = 187;
            <I188> = 188;
            <I189> = 189;
            <I190> = 190;
            <I191> = 191;
            <I192> = 192;
            <I193> = 193;
            <I194> = 194;
            <I195> = 195;
            <I196> = 196;
            <I197> = 197;
            <I198> = 198;
            <I199> = 199;
            <I200> = 200;
            <I201> = 201;
            <I202> = 202;
            <I203> = 203;
            <I204> = 204;
            <I205> = 205;
            <I206> = 206;
            <I207> = 207;
            <I208> = 208;
            <I209> = 209;
            <I210> = 210;
            <I211> = 211;
            <I212> = 212;
            <I213> = 213;
            <I214> = 214;
            <I215> = 215;
            <I216> = 216;
            <I217> = 217;
            <I218> = 218;
            <I219> = 219;
            <I220> = 220;
            <I221> = 221;
            <I222> = 222;
            <I223> = 223;
            <I224> = 224;
            <I225> = 225;
            <I226> = 226;
            <I227> = 227;
            <I228> = 228;
            <I229> = 229;
            <I230> = 230;
            <I231> = 231;
            <I232> = 232;
            <I233> = 233;
            <I234> = 234;
            <I235> = 235;
            <I236> = 236;
            <I237> = 237;
            <I238> = 238;
            <I239> = 239;
            <I240> = 240;
            <I241> = 241;
            <I242> = 242;
            <I243> = 243;
            <I244> = 244;
            <I245> = 245;
            <I246> = 246;
            <I247> = 247;
            <I248> = 248;
            <I249> = 249;
            <I250> = 250;
            <I251> = 251;
            <I252> = 252;
            <I253> = 253;
            <I254> = 254;
            <I255> = 255;
            
            indicator 1 = \"Caps Lock\"; // Needed for Xwayland
        }};
        xkb_types {{
            // Do NOT change this part. It is required by Xorg/Xwayland.
            virtual_modifiers OSK;
            type \"ONE_LEVEL\" {{
                modifiers= none;
                level_name[Level1]= \"Any\";
            }};
            type \"TWO_LEVEL\" {{
                level_name[Level1]= \"Base\";
            }};
            type \"ALPHABETIC\" {{
                level_name[Level1]= \"Base\";
            }};
            type \"KEYPAD\" {{
                level_name[Level1]= \"Base\";
            }};
            type \"SHIFT+ALT\" {{
                level_name[Level1]= \"Base\";
            }};
        }};
        xkb_compatibility {{
            // Do NOT change this part. It is required by Xorg/Xwayland.
            interpret Any+AnyOf(all) {{
                action= SetMods(modifiers=modMapMods,clearLocks);
            }};
        }};
        xkb_symbols {{"
        )?;
        for (&keysym, &keycode) in &self.keymap {
            write!(
                buf,
                "
            key <I{}> {{ [ {} ] }}; // \\n",
                keycode,
                keysym_get_name(keysym)
            )?;
        }
        writeln!(
            buf,
            "
        }};
        
    }};"
        )?;

        String::from_utf8(buf).map_err(ConnectionError::Utf)
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
                '\n' => keysyms::KEY_Return,
                '\t' => keysyms::KEY_Tab,
                _ => {
                    let hex: u32 = c.into();
                    let name = format!("U{hex:x}");
                    keysym_from_name(&name, KEYSYM_NO_FLAGS)
                }
            },
            Key::Raw(k) => {
                // Raw keycodes cannot be converted to keysyms
                panic!("Attempted to convert raw keycode {k} to keysym");
            }
            Key::Alt | Key::LAlt | Key::Option => keysyms::KEY_Alt_L,
            Key::Backspace => keysyms::KEY_BackSpace,
            Key::Begin => keysyms::KEY_Begin,
            Key::Break => keysyms::KEY_Break,
            Key::Cancel => keysyms::KEY_Cancel,
            Key::CapsLock => keysyms::KEY_Caps_Lock,
            Key::Clear => keysyms::KEY_Clear,
            Key::Control | Key::LControl => keysyms::KEY_Control_L,
            Key::Delete => keysyms::KEY_Delete,
            Key::DownArrow => keysyms::KEY_Down,
            Key::End => keysyms::KEY_End,
            Key::Escape => keysyms::KEY_Escape,
            Key::Execute => keysyms::KEY_Execute,
            Key::F1 => keysyms::KEY_F1,
            Key::F2 => keysyms::KEY_F2,
            Key::F3 => keysyms::KEY_F3,
            Key::F4 => keysyms::KEY_F4,
            Key::F5 => keysyms::KEY_F5,
            Key::F6 => keysyms::KEY_F6,
            Key::F7 => keysyms::KEY_F7,
            Key::F8 => keysyms::KEY_F8,
            Key::F9 => keysyms::KEY_F9,
            Key::F10 => keysyms::KEY_F10,
            Key::F11 => keysyms::KEY_F11,
            Key::F12 => keysyms::KEY_F12,
            Key::F13 => keysyms::KEY_F13,
            Key::F14 => keysyms::KEY_F14,
            Key::F15 => keysyms::KEY_F15,
            Key::F16 => keysyms::KEY_F16,
            Key::F17 => keysyms::KEY_F17,
            Key::F18 => keysyms::KEY_F18,
            Key::F19 => keysyms::KEY_F19,
            Key::F20 => keysyms::KEY_F20,
            Key::F21 => keysyms::KEY_F21,
            Key::F22 => keysyms::KEY_F22,
            Key::F23 => keysyms::KEY_F23,
            Key::F24 => keysyms::KEY_F24,
            Key::F25 => keysyms::KEY_F25,
            Key::F26 => keysyms::KEY_F26,
            Key::F27 => keysyms::KEY_F27,
            Key::F28 => keysyms::KEY_F28,
            Key::F29 => keysyms::KEY_F29,
            Key::F30 => keysyms::KEY_F30,
            Key::F31 => keysyms::KEY_F31,
            Key::F32 => keysyms::KEY_F32,
            Key::F33 => keysyms::KEY_F33,
            Key::F34 => keysyms::KEY_F34,
            Key::F35 => keysyms::KEY_F35,
            Key::Find => keysyms::KEY_Find,
            Key::Hangul => keysyms::KEY_Hangul,
            Key::Hanja => keysyms::KEY_Hangul_Hanja,
            Key::Help => keysyms::KEY_Help,
            Key::Home => keysyms::KEY_Home,
            Key::Insert => keysyms::KEY_Insert,
            Key::Kanji => keysyms::KEY_Kanji,
            Key::LeftArrow => keysyms::KEY_Left,
            Key::Linefeed => keysyms::KEY_Linefeed,
            Key::LMenu => keysyms::KEY_Menu,
            Key::ModeChange => keysyms::KEY_Mode_switch,
            Key::Numlock => keysyms::KEY_Num_Lock,
            Key::PageDown => keysyms::KEY_Page_Down,
            Key::PageUp => keysyms::KEY_Page_Up,
            Key::Pause => keysyms::KEY_Pause,
            Key::Print => keysyms::KEY_Print,
            Key::RAlt => keysyms::KEY_Alt_R,
            Key::RControl => keysyms::KEY_Control_R,
            Key::Redo => keysyms::KEY_Redo,
            Key::Return => keysyms::KEY_Return,
            Key::RightArrow => keysyms::KEY_Right,
            Key::RShift => keysyms::KEY_Shift_R,
            Key::ScrollLock => keysyms::KEY_Scroll_Lock,
            Key::Select => keysyms::KEY_Select,
            Key::ScriptSwitch => keysyms::KEY_script_switch,
            Key::Shift | Key::LShift => keysyms::KEY_Shift_L,
            Key::ShiftLock => keysyms::KEY_Shift_Lock,
            Key::Space => keysyms::KEY_space,
            Key::SysReq => keysyms::KEY_Sys_Req,
            Key::Tab => keysyms::KEY_Tab,
            Key::Undo => keysyms::KEY_Undo,
            Key::UpArrow => keysyms::KEY_Up,
            Key::Command | Key::Super | Key::Windows | Key::Meta => keysyms::KEY_Super_L,
        }
    }

    fn map_sym(&mut self, keysym: Keysym) -> Result<Keycode, ConnectionError> {
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
    fn unmap_sym(&mut self, keysym: Keysym) {
        if let Some(&keycode) = self.keymap.get(&keysym) {
            self.needs_regeneration = true;
            self.unused_keycodes.push_back(keycode);
            self.keymap.remove(&keysym);
        }
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
    fn apply_layout(&mut self, layout: &str) {
        if let Some(vk) = &self.virtual_keyboard {
            // We need to build a file with a fd in order to pass the layout file to Wayland
            // for processing
            let keymap_size = layout.len();
            let keymap_size_u32: u32 = keymap_size.try_into().unwrap(); // Convert it from usize to u32, panics if it is not possible
            let keymap_size_u64: u64 = keymap_size.try_into().unwrap(); // Convert it from usize to u64, panics if it is not possible
            let mut keymap_file = tempfile().expect("Unable to create tempfile");

            // Allocate space in the file first
            keymap_file.seek(SeekFrom::Start(keymap_size_u64)).unwrap();
            keymap_file.write_all(&[0]).unwrap();
            keymap_file.rewind().unwrap();
            let mut data = unsafe {
                memmap2::MmapOptions::new()
                    .map_mut(&keymap_file)
                    .expect("Could not access data from memory mapped file")
            };
            data[..layout.len()].copy_from_slice(layout.as_bytes());

            // Get fd to pass to Wayland
            let keymap_raw_fd = keymap_file.into_raw_fd();
            vk.keymap(1, keymap_raw_fd, keymap_size_u32);
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

        // Unmap all keys, if all keycodes are already being used
        // TODO: Don't unmap the keycodes if they will be needed next
        // TODO: Don't unmap held keys!
        if self.unused_keycodes.is_empty() {
            let mapped_keys = self.keymap.clone();
            for &sym in mapped_keys.keys() {
                self.unmap_sym(sym);
            }
            self.event_queue.roundtrip(&mut self.state).unwrap();
        }

        let keycode = if let Key::Raw(kc) = key {
            kc.try_into().unwrap()
        } else {
            let sym = Self::key_to_keysym(key);
            let keycode = self.get_keycode(sym).unwrap();
            keycode
        };

        if self.needs_regeneration {
            let keymap = self.generate_keymap_string().unwrap();
            self.apply_layout(&keymap);
            self.needs_regeneration = false;
        }
        let modifier = self.is_modifier(key);

        match press {
            None => {
                if let Some(m) = modifier {
                    self.send_modifier_event(self.modifiers | m);
                    self.send_modifier_event(self.modifiers);
                } else {
                    self.send_key_event(keycode, true);
                    self.send_key_event(keycode, false);
                }
            }
            Some(true) => {
                if let Some(m) = modifier {
                    self.modifiers |= m;
                    self.send_modifier_event(self.modifiers);
                } else {
                    self.send_key_event(keycode, true);
                }
                self.held.push(key);
            }
            Some(false) => {
                if let Some(m) = modifier {
                    self.modifiers &= !m;
                    self.send_modifier_event(self.modifiers);
                } else {
                    self.send_key_event(keycode, false);
                }
                self.held.retain(|&k| k != key);
            }
        }
    }

    fn is_modifier(&self, key: Key) -> Option<u32> {
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
            vk.modifiers(modifiers, 0, 0, 0)
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
        for &k in &self.held.clone() {
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
        for c in string.chars() {
            self.press_key(Key::Layout(c), None);
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
        println!("key_click");
        // self.press_key(key, Some(true));
        //  self.press_key(key, Some(false));
        if let Some(im) = &self.input_method {
            im.commit_string("Hello World! here is a lot of text  ❤️💣💩🔥".to_string());
            im.commit(self.serial);
            self.serial = self.serial.wrapping_add(1);
        }
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
            im_mgr.destroy()
        }
        if let Some(pointer_mgr) = self.pointer_manager.as_ref() {
            pointer_mgr.destroy()
        }
    }
}
