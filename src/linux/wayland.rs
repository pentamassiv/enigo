use std::collections::{HashMap, VecDeque};
use std::convert::TryInto;
use std::io::{Seek, SeekFrom, Write};
use std::os::unix::io::IntoRawFd;
use std::time::Instant;

use tempfile::tempfile;

use wayland_client::{
    protocol::{wl_registry, wl_seat},
    Connection, Dispatch, EventQueue, QueueHandle,
};
use wayland_protocols_misc::zwp_virtual_keyboard_v1::client::{
    zwp_virtual_keyboard_manager_v1, zwp_virtual_keyboard_v1,
};

use crate::KeyboardControllable;

#[derive(Debug)]
pub enum DisplayOutputError {
    AllocationFailed(char),
    Connection(String),
    Format(std::io::Error),
    General(String),
    LostConnection,
    NoKeycode,
    SetLayoutFailed(String),
    Unimplemented,
    Utf(std::string::FromUtf8Error),
}

impl std::fmt::Display for DisplayOutputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DisplayOutputError::AllocationFailed(e) => write!(f, "Allocation failed: {e}"),
            DisplayOutputError::Connection(e) => write!(f, "Connection: {e}"),
            DisplayOutputError::Format(e) => write!(f, "Format: {e}"),
            DisplayOutputError::General(e) => write!(f, "General: {e}"),
            DisplayOutputError::LostConnection => write!(f, "Lost connection"),
            DisplayOutputError::NoKeycode => write!(f, "No keycode mapped"),
            DisplayOutputError::SetLayoutFailed(e) => write!(f, "set_layout() failed: {e}"),
            DisplayOutputError::Unimplemented => write!(f, "Unimplemented"),
            DisplayOutputError::Utf(e) => write!(f, "UTF: {e}"),
        }
    }
}

impl From<std::io::Error> for DisplayOutputError {
    fn from(e: std::io::Error) -> Self {
        DisplayOutputError::Format(e)
    }
}

pub struct KeyMap {
    pub keysym: xkbcommon::xkb::Keysym,
    pub keycode: u32,
    pub refcount: u32,
}

impl KeyMap {
    #[must_use]
    /// Lookup keysym for a UTF-8 symbol
    /// \n and \t are special symbols for Return and Tab respectively
    pub fn new(c: char, keycode: u32) -> Option<Self> {
        // Special character lookup, otherwise normal lookup
        let keysym = match c {
            '\n' => xkbcommon::xkb::keysyms::KEY_Return,
            '\t' => xkbcommon::xkb::keysyms::KEY_Tab,
            _ => {
                // Convert UTF-8 to a code point first to do the keysym lookup
                let codepoint = format!("U{:X}", c as u32);
                xkbcommon::xkb::keysym_from_name(&codepoint, xkbcommon::xkb::KEYSYM_NO_FLAGS)
            }
        };
        // trace!("{} {:04X} -> U{:04X}", c, c as u32, keysym);

        // Make sure the keysym is valid
        if keysym == xkbcommon::xkb::keysyms::KEY_NoSymbol {
            None
        } else {
            Some(KeyMap {
                keysym,
                keycode,
                refcount: 1,
            })
        }
    }
}

impl std::fmt::Debug for KeyMap {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "keysym:{} keycode:{} refcount:{}",
            self.keysym, self.keycode, self.refcount
        )
    }
}

pub struct Keymap {
    keysym_lookup: HashMap<char, KeyMap>, // UTF-8 -> (keysym, keycode, refcount)
    unused_keycodes: VecDeque<u32>,       // Used to keep track of unused keycodes
}

impl Keymap {
    #[must_use]
    pub fn new() -> Keymap {
        // Only keycodes from 8 to 255 can be used
        let keysym_lookup = HashMap::with_capacity(255 - 7);
        let mut unused_keycodes: VecDeque<u32> = VecDeque::with_capacity(255 - 7); // All keycodes are unused when initialized
        for n in 8..=255 {
            unused_keycodes.push_back(n);
        }

        Keymap {
            keysym_lookup,
            unused_keycodes,
        }
    }

    /// Generates a single-level keymap.
    ///
    /// # Errors
    /// The only way this can throw an error is if the generated String is not
    /// valid UTF8
    pub fn generate_keymap_string(&mut self) -> Result<String, DisplayOutputError> {
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

        for (key, val) in &self.keysym_lookup {
            match key {
                '\n' => {
                    write!(
                        buf,
                        "
            key <I{}> {{ [ Return ] }}; // \\n",
                        val.keycode,
                    )?;
                }
                '\t' => {
                    write!(
                        buf,
                        "
            key <I{}> {{ [ Tab ] }}; // \\t",
                        val.keycode,
                    )?;
                }
                _ => {
                    write!(
                        buf,
                        "
            key <I{}> {{ [ U{:X} ] }}; // {}",
                        val.keycode,
                        val.keysym & 0x1F_FFFF, /* XXX (HaaTa): I suspect there's a UTF-8 ->
                                                 * Keysym incompatibility for higher orders */
                        //              this mask seems allow mappings to work
                        //              correctly but I don't think it's correct.
                        // Might be related to: https://docs.rs/xkbcommon/0.4.0/xkbcommon/xkb/type.Keysym.html
                        key,
                    )?;
                }
            }
        }

        writeln!(
            buf,
            "
        }};
        
    }};"
        )?;

        String::from_utf8(buf).map_err(DisplayOutputError::Utf)
    }

    /// Adds UTF-8 symbols to be added to the virtual keyboard.
    /// If the keymap needs to be regenerated, it returns the new string. If any
    /// of the symbols could not be mapped, none of the symbols will be mapped.
    /// Increments a reference counter if the symbol has already been
    /// added.
    ///
    /// # Errors
    /// TODO
    pub fn add(&mut self, chars: std::str::Chars) -> Result<Option<String>, DisplayOutputError> {
        let mut regenerate = false;
        // trace!("add({:?})", chars);

        // Increment the reference counters and allocate keycodes
        for c in chars {
            if let Some(val) = self.keysym_lookup.get_mut(&c) {
                val.refcount += 1;
                continue;
            }
            // Allocate keycode
            let Some(keycode) = self.unused_keycodes.pop_front() else { return Err(DisplayOutputError::AllocationFailed(c)); };
            // Insert keysym and keycode for lookup
            let Some(key) = KeyMap::new(c,keycode) else { return Err(DisplayOutputError::AllocationFailed(c))};
            self.keysym_lookup.insert(c, key);

            // Trigger a regen of the layout
            regenerate = true;
        }

        // Return the new keymap
        if regenerate {
            // trace!("add({:?}) regenerate {}", chars, layout);
            Ok(Some(self.generate_keymap_string()?))
        } else {
            Ok(None)
        }
    }

    /// Removes UTF-8 symbols from the virtual keyboard.
    /// Decrements the reference counter
    /// If the reference count has reached zero, the keycode is freed and
    /// removed from the keymap
    ///
    /// # Errors
    /// TODO
    pub fn remove(&mut self, chars: std::str::Chars) -> Result<(), DisplayOutputError> {
        // trace!("remove({:?})", chars);

        // Lookup each of the keysyms, decrementing the reference counters
        for c in chars {
            if let Some(key) = self.keysym_lookup.get_mut(&c) {
                key.refcount -= 1;
                if key.refcount == 0 {
                    // Add the keycode back to the queue and remove the entry
                    self.unused_keycodes.push_back(key.keycode);
                    self.keysym_lookup.remove(&c);
                }
            }
        }

        // No need to regenerate the keymap when something is removed (skip for
        // performance increase)

        Ok(())
    }
}

pub struct WaylandConnection {
    _conn: Connection,
    event_queue: EventQueue<VirtKbdState>,
    state: VirtKbdState,
    virtual_keyboard: zwp_virtual_keyboard_v1::ZwpVirtualKeyboardV1,
    held: Vec<char>,
    keymap: Keymap,
    base_time: std::time::Instant,
}

impl WaylandConnection {
    /// Tries to establish a new Wayland connection
    ///
    /// # Errors
    /// TODO
    pub fn new() -> Result<WaylandConnection, DisplayOutputError> {
        let held = Vec::with_capacity(255 - 7);

        // Setup Wayland Connection
        let conn = Connection::connect_to_env();

        // Make sure we made a connection
        let conn = match conn {
            Ok(conn) => conn,
            Err(e) => {
                // error!("Failed to connect to Wayland");
                return Err(DisplayOutputError::Connection(e.to_string()));
            }
        };

        // Check to see if there was an error trying to connect
        if let Some(err) = conn.protocol_error() {
            //  error!(
            //     "Unknown Wayland initialization failure: {} {} {} {}",
            //      err.code, err.object_id, err.object_interface, err.message
            // );
            return Err(DisplayOutputError::General(err.to_string()));
        }

        // Create the event queue
        let mut event_queue = conn.new_event_queue();
        // Get queue handle
        let qh = event_queue.handle();

        // Start registry
        let display = conn.display();
        display.get_registry(&qh, ());

        // Dispatch events so we can setup a virtual keyboard
        // This requires a wl_seat and zwp_virtual_keyboard_manager_v1
        let mut state = VirtKbdState::new();
        event_queue.roundtrip(&mut state).unwrap();

        // Setup Virtual Keyboard
        let seat = state.seat.as_ref().unwrap();
        let vk_mgr = state.keyboard_manager.as_ref().unwrap();
        let virtual_keyboard = vk_mgr.create_virtual_keyboard(seat, &qh, ());

        // Setup Keymap
        let keymap = Keymap::new();

        let base_time = Instant::now();
        Ok(WaylandConnection {
            _conn: conn,
            event_queue,
            state,
            held,
            keymap,
            base_time,
            virtual_keyboard,
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
    /// NOTE: This function does not flush any messages to Wayland, you'll need
    /// to schedule afterwards
    ///
    /// # Errors
    /// TODO
    pub fn apply_layout(&mut self, layout: &str) -> Result<(), DisplayOutputError> {
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
        self.virtual_keyboard
            .keymap(1, keymap_raw_fd, keymap_size_u32);
        Ok(())
    }

    /// Press/Release a given UTF-8 symbol
    /// NOTE: This function does not synchronize the event queue, should be done
    /// immediately after calling (unless you're trying to optimize
    /// scheduling).
    fn press_symbol(&mut self, c: char, press: bool) -> Result<(), DisplayOutputError> {
        // Nothing to do
        if c == '\0' {
            return Ok(());
        }

        if press {
            if let Ok(Some(layout)) = self.keymap.add(c.to_string().chars()) {
                let _ = self.apply_layout(&layout);
            }
            self.press_key(c, true)?;
            self.held.push(c);
        } else {
            self.press_key(c, false)?;
            self.held
                .iter()
                .position(|&x| x == c)
                .map(|e| self.held.remove(e));
            self.keymap.remove(c.to_string().chars())?;
        }

        Ok(())
    }

    /// Press/Release a specific UTF-8 symbol
    /// NOTE: This function does not synchronize the event queue, should be done
    /// immediately after calling (unless you're trying to optimize
    /// scheduling).
    ///
    /// # Errors
    /// TODO
    pub fn press_key(&mut self, c: char, press: bool) -> Result<(), DisplayOutputError> {
        let time = self.get_time();
        let state = u32::from(press);
        let  Some(&KeyMap{mut keycode,..}) = self.keymap.keysym_lookup.get(&c) else { return Err(DisplayOutputError::NoKeycode); };
        keycode -= 8; // Adjust by 8 due to the xkb/xwayland requirements

        // debug!("time:{} keycode:{}:{} state:{}", time, c, keycode, state);

        // Send key event message
        self.virtual_keyboard.key(time, keycode, state);
        Ok(())
    }

    /// Press then release a specific UTF-8 symbol
    /// Faster than individual calls to `press_key` as you don't need a delay
    /// (or sync) between press and release of the same keycode.
    /// NOTE: This function does not synchronize the event queue, should be done
    /// immediately after calling (unless you're trying to optimize
    /// scheduling).
    ///
    /// # Errors
    /// TODO
    pub fn click_key(&mut self, c: char) -> Result<(), DisplayOutputError> {
        let time = self.get_time();
        let  Some(&KeyMap{mut keycode,..}) = self.keymap.keysym_lookup.get(&c) else { return Err(DisplayOutputError::NoKeycode); };
        keycode -= 8; // Adjust by 8 due to the xkb/xwayland requirements

        // debug!("time:{} keycode:{}:{}", time, c, keycode);

        // Send key event message
        self.virtual_keyboard.key(time, keycode, 1);
        self.virtual_keyboard.key(time, keycode, 0);
        Ok(())
    }
}

impl Drop for WaylandConnection {
    fn drop(&mut self) {
        // warn!("Releasing and unbinding all keys");
        let held_keys = self.held.clone();
        for c in &held_keys {
            self.press_key(*c, false).unwrap();
            self.keymap.remove(c.to_string().chars()).unwrap();
        }
    }
}

impl KeyboardControllable for WaylandConnection {
    /// Type the given UTF-8 string using the virtual keyboard
    /// Should behave nicely even if keys were previously held (those keys will
    /// continue to be held after sequence is complete, though there may be
    /// some issues with this case due to the layout switching)
    fn key_sequence(&mut self, string: &str) {
        // Allocate keysyms to virtual keyboard layout
        if let Ok(Some(layout)) = self.keymap.add(string.chars()) {
            let _ = self.apply_layout(&layout);
        }

        for c in string.chars() {
            self.click_key(c).unwrap();
        }

        // Pump event queue
        self.event_queue.roundtrip(&mut self.state).unwrap();

        // Deallocate keysyms in virtual keyboard layout
        self.keymap.remove(string.chars()).unwrap();
    }

    fn key_down(&mut self, key: crate::Key) {
        if let crate::Key::Layout(c) = key {
            let _ = self.press_symbol(c, true); // Pump event queue
            self.event_queue.roundtrip(&mut self.state).unwrap();
        }
    }
    fn key_up(&mut self, key: crate::Key) {
        if let crate::Key::Layout(c) = key {
            let _ = self.press_symbol(c, false); // Pump event queue
            self.event_queue.roundtrip(&mut self.state).unwrap();
        }
    }

    fn key_click(&mut self, key: crate::Key) {
        if let crate::Key::Layout(c) = key {
            let _ = self.click_key(c); // Pump event queue
            self.event_queue.roundtrip(&mut self.state).unwrap();
        }
    }
}

struct VirtKbdState {
    keyboard_manager: Option<zwp_virtual_keyboard_manager_v1::ZwpVirtualKeyboardManagerV1>,
    seat: Option<wl_seat::WlSeat>,
}

impl VirtKbdState {
    fn new() -> Self {
        Self {
            keyboard_manager: None,
            seat: None,
        }
    }
}

impl Dispatch<wl_registry::WlRegistry, ()> for VirtKbdState {
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
        // When receiving this event, we just print its characteristics in this example.
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        {
            // trace!("[{}] {} (v{})", name, interface, version);

            match &interface[..] {
                "wl_seat" => {
                    // Setup seat for keyboard
                    let seat = registry.bind::<wl_seat::WlSeat, _, _>(name, 1, qh, ());
                    state.seat = Some(seat);
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
                _ => {}
            }
        }
    }
}

impl Dispatch<zwp_virtual_keyboard_manager_v1::ZwpVirtualKeyboardManagerV1, ()> for VirtKbdState {
    fn event(
        _state: &mut Self,
        _manager: &zwp_virtual_keyboard_manager_v1::ZwpVirtualKeyboardManagerV1,
        event: zwp_virtual_keyboard_manager_v1::Event,
        _: &(),
        _: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // info!("Got a virtual keyboard manager event {:?}", event);
    }
}

impl Dispatch<zwp_virtual_keyboard_v1::ZwpVirtualKeyboardV1, ()> for VirtKbdState {
    fn event(
        _state: &mut Self,
        _manager: &zwp_virtual_keyboard_v1::ZwpVirtualKeyboardV1,
        event: zwp_virtual_keyboard_v1::Event,
        _: &(),
        _: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // info!("Got a virtual keyboard event {:?}", event);
    }
}

impl Dispatch<wl_seat::WlSeat, ()> for VirtKbdState {
    fn event(
        _: &mut Self,
        _: &wl_seat::WlSeat,
        event: wl_seat::Event,
        _: &(),
        _: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // info!("Got a seat event {:?}", event);
    }
}
